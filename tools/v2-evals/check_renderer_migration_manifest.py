#!/usr/bin/env python3
"""Validate the native-renderer migration manifest.

Checks (all deterministic, stdlib only):
  1. fixtures/native-renderer/manifest.json parses and carries schema_version 1.
  2. Every row names a unique family, a non-empty legacy path, a unique
     native_effect_id, a golden_fixture under fixtures/native-renderer/
     (no parent traversal, no absolute paths), shipping_runtime
     "cutright-native", and a known reduced_motion kind.
  3. The nine required visual-effect families are all present: lower-third,
     stat-counter, quote-card, cta-card, captions, hook-pullback, punch-wave,
     text-reveals, audio-sync.
  4. imports/v2/dispositions/renderers.json exists, classifies remotion and
     hyperframes as provenance_only/clean_room_behavior, and publishes a
     non-empty forbidden-runtime guard for each.
  5. Every registry effect whose renderer is "remotion" or "hyperframes" in
     schemas/effects/registry.json has a migration row whose registry_effect
     names it — no existing renderer/effect may lack a migration target.

Exit codes: 0 all checks passed, 1 errors found, 2 inputs missing.
Usage: python3 tools/v2-evals/check_renderer_migration_manifest.py \
           fixtures/native-renderer/manifest.json [--repo-root DIR]
"""

import argparse
import json
import os
import sys

EXIT_OK = 0
EXIT_ERRORS = 1
EXIT_MISSING = 2

REQUIRED_FAMILIES = [
    "audio-sync",
    "captions",
    "cta-card",
    "hook-pullback",
    "lower-third",
    "punch-wave",
    "quote-card",
    "stat-counter",
    "text-reveals",
]
REDUCED_MOTION_KINDS = {"not-meaningful", "static-fallback", "unsupported"}
FORBIDDEN_RENDERERS = {"remotion", "hyperframes"}
REQUIRED_ROW_FIELDS = [
    "family",
    "golden_fixture",
    "legacy",
    "native_effect_id",
    "reduced_motion",
    "registry_effect",
    "shipping_runtime",
]


def load_json(path, errors):
    if not os.path.isfile(path):
        return None
    try:
        with open(path, "r", encoding="utf-8") as handle:
            return json.load(handle)
    except (OSError, json.JSONDecodeError) as exc:
        errors.append("unreadable JSON at %s: %s" % (path, exc))
        return None


def check_rows(manifest, errors):
    rows = manifest.get("rows")
    if not isinstance(rows, list) or not rows:
        errors.append("manifest must carry a non-empty rows list")
        return []
    families = set()
    effect_ids = set()
    for index, row in enumerate(rows):
        label = row.get("family", "<row %d>" % index)
        for field in REQUIRED_ROW_FIELDS:
            if field not in row:
                errors.append("row %s: missing field %s" % (label, field))
        family = row.get("family")
        if not isinstance(family, str) or not family:
            errors.append("row %d: family must be a non-empty string" % index)
        elif family in families:
            errors.append("row %s: duplicate family" % family)
        else:
            families.add(family)
        effect_id = row.get("native_effect_id")
        if not isinstance(effect_id, str) or not effect_id:
            errors.append("row %s: native_effect_id must be a non-empty string" % label)
        elif effect_id in effect_ids:
            errors.append("row %s: duplicate native_effect_id %s" % (label, effect_id))
        else:
            effect_ids.add(effect_id)
        if not isinstance(row.get("legacy"), str) or not row.get("legacy"):
            errors.append("row %s: legacy must be a non-empty string" % label)
        fixture = row.get("golden_fixture")
        if (
            not isinstance(fixture, str)
            or not fixture.startswith("fixtures/native-renderer/")
            or ".." in fixture.split("/")
            or os.path.isabs(fixture)
        ):
            errors.append(
                "row %s: golden_fixture must live under fixtures/native-renderer/ "
                "without parent traversal" % label
            )
        if row.get("shipping_runtime") != "cutright-native":
            errors.append("row %s: shipping_runtime must be cutright-native" % label)
        if row.get("reduced_motion") not in REDUCED_MOTION_KINDS:
            errors.append(
                "row %s: reduced_motion must be one of %s"
                % (label, ", ".join(sorted(REDUCED_MOTION_KINDS)))
            )
        registry_effect = row.get("registry_effect")
        if registry_effect is not None and not isinstance(registry_effect, str):
            errors.append("row %s: registry_effect must be a string or null" % label)
    return rows


def check_required_families(rows, errors):
    present = {row.get("family") for row in rows if isinstance(row, dict)}
    for family in REQUIRED_FAMILIES:
        if family not in present:
            errors.append("required family missing: %s" % family)


def check_renderers_dispositions(repo_root, errors):
    path = os.path.join(repo_root, "imports/v2/dispositions/renderers.json")
    if not os.path.isfile(path):
        errors.append("missing renderer dispositions ledger: %s" % path)
        return
    data = load_json(path, errors)
    if data is None:
        return
    entries = {e.get("renderer"): e for e in data.get("entries", []) if isinstance(e, dict)}
    for renderer in sorted(FORBIDDEN_RENDERERS):
        entry = entries.get(renderer)
        if entry is None:
            errors.append("renderers.json: no entry for %s" % renderer)
            continue
        if entry.get("shipping_disposition") != "provenance_only":
            errors.append("renderers.json: %s shipping_disposition must be provenance_only" % renderer)
        if entry.get("behavior_disposition") != "clean_room_behavior":
            errors.append(
                "renderers.json: %s behavior_disposition must be clean_room_behavior" % renderer
            )
        guard = entry.get("forbidden_in_runtime_packs")
        if not isinstance(guard, dict):
            errors.append("renderers.json: %s lacks forbidden_in_runtime_packs" % renderer)
            continue
        banned = (
            list(guard.get("npm_packages", []))
            + list(guard.get("binaries", []))
            + list(guard.get("browser_engines", []))
            + list(guard.get("runtime_identifiers", []))
        )
        if not banned:
            errors.append("renderers.json: %s forbidden-runtime guard is empty" % renderer)
    shared = data.get("shared_release_guard")
    if not isinstance(shared, dict) or not shared.get("forbidden_runtime_binaries"):
        errors.append("renderers.json: shared_release_guard.forbidden_runtime_binaries missing")


def check_registry_coverage(repo_root, rows, errors):
    path = os.path.join(repo_root, "schemas/effects/registry.json")
    registry = load_json(path, errors)
    if registry is None:
        errors.append("missing effect registry: %s" % path)
        return
    migrated = {row.get("registry_effect") for row in rows if isinstance(row, dict)}
    for effect in registry.get("effects", []):
        effect_id = effect.get("effect_id")
        renderer = effect.get("renderer")
        if renderer in FORBIDDEN_RENDERERS and effect_id not in migrated:
            errors.append(
                "registry effect %s (renderer %s) has no migration row" % (effect_id, renderer)
            )


def main():
    parser = argparse.ArgumentParser(description="Validate the native-renderer migration manifest.")
    parser.add_argument("manifest", help="path to fixtures/native-renderer/manifest.json")
    parser.add_argument(
        "--repo-root",
        default=os.getcwd(),
        help="CutRight repository root (default: current working directory)",
    )
    args = parser.parse_args()

    errors = []
    notes = []
    if not os.path.isfile(args.manifest):
        print("SKIPPED (missing manifest): %s" % args.manifest)
        return EXIT_MISSING
    manifest = load_json(args.manifest, errors)
    rows = []
    if manifest is None:
        errors.append("manifest did not parse: %s" % args.manifest)
    else:
        if manifest.get("schema_version") != 1:
            errors.append("manifest schema_version must be 1")
        rows = check_rows(manifest, errors)
        check_required_families(rows, errors)
    check_renderers_dispositions(args.repo_root, errors)
    if rows:
        check_registry_coverage(args.repo_root, rows, errors)
    else:
        notes.append("registry coverage skipped: no valid rows")

    print("native-renderer migration contract")
    print("  manifest: %s" % args.manifest)
    print("  rows: %d" % len(rows))
    print("  required families: %d" % len(REQUIRED_FAMILIES))
    for note in sorted(notes):
        print("  NOTE %s" % note)
    if errors:
        for error in sorted(errors):
            print("  ERROR %s" % error)
        print("verdict: fail (%d error%s)" % (len(errors), "s" if len(errors) != 1 else ""))
        return EXIT_ERRORS
    print("verdict: pass")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
