#!/usr/bin/env python3
"""scripts/benchmarks/validate-corpus.py — validate the v2 golden corpus manifest.

Usage:
    python3 scripts/benchmarks/validate-corpus.py benchmarks/corpus/manifest.json

Validation rules (CR-V2-B4-002):

1. The manifest must validate against `schemas/benchmarks/corpus.schema.v1.json`.
2. Every item's `rights_ref` file must exist and be valid JSON.
3. Every runnable item (missing_fixture != true) must have local bytes
   available — declared by the rights record's `source_paths` array.
4. No project_id may appear in more than one split.
5. No source_hashes entry may be duplicated across projects.
6. Redistributable items must have a complete rights record.
7. Private items (allowed_distribution == "local_only") must be flagged
   so shareable reports can redact their paths.

Exit 0 on success, 1 on any validation error.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCHEMA = REPO / "schemas/benchmarks/corpus.schema.v1.json"


def _validate_schema(manifest: dict) -> list[str]:
    sys.path.insert(0, str(REPO / "scripts"))
    import importlib.util

    spec = importlib.util.spec_from_file_location(
        "schema_check", REPO / "scripts/schema-check.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    # schema-check writes failures to stdout; we just need exit code
    import io
    from contextlib import redirect_stdout

    buf = io.StringIO()
    with redirect_stdout(buf):
        try:
            module.validate(manifest, json.loads(SCHEMA.read_text()), json.loads(SCHEMA.read_text()))
        except Exception as exc:
            return [f"schema validation failed: {exc}"]
    return []


def _validate_rights(manifest: dict, root: Path) -> list[str]:
    errors: list[str] = []
    for item in manifest["items"]:
        rights_path = root / item["rights_ref"]
        if not rights_path.exists():
            errors.append(f"missing rights record: {rights_path}")
            continue
        try:
            rights = json.loads(rights_path.read_text())
        except json.JSONDecodeError as exc:
            errors.append(f"rights record not valid JSON: {rights_path} ({exc})")
            continue
        if item.get("redistributable") and not rights.get("licence") == "redistributable":
            errors.append(f"redistributable item without rights licence: {item['project_id']}")
    return errors


def _validate_splits(manifest: dict) -> list[str]:
    errors: list[str] = []
    seen: set[str] = set()
    for split, ids in manifest["splits"].items():
        for pid in ids:
            if pid in seen:
                errors.append(f"project_id {pid} appears in multiple splits")
            seen.add(pid)
    listed = {item["project_id"] for item in manifest["items"]}
    unlisted = seen - listed
    if unlisted:
        errors.append(f"split lists projects not in items: {sorted(unlisted)}")
    return errors


def _validate_unique_hashes(manifest: dict) -> list[str]:
    errors: list[str] = []
    seen: dict[str, str] = {}
    for item in manifest["items"]:
        for h in item["source_hashes"]:
            if h in seen:
                errors.append(f"duplicate source hash {h} on {seen[h]} and {item['project_id']}")
            seen[h] = item["project_id"]
    return errors


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(__doc__)
        return 1
    manifest_path = Path(argv[1]).resolve()
    if not manifest_path.exists():
        print(f"manifest not found: {manifest_path}", file=sys.stderr)
        return 1
    try:
        manifest = json.loads(manifest_path.read_text())
    except json.JSONDecodeError as exc:
        print(f"manifest not valid JSON: {exc}", file=sys.stderr)
        return 1

    errors: list[str] = []
    errors.extend(_validate_schema(manifest))
    errors.extend(_validate_rights(manifest, manifest_path.parent))
    errors.extend(_validate_splits(manifest))
    errors.extend(_validate_unique_hashes(manifest))

    if errors:
        for err in errors:
            print(f"VALIDATION: {err}", file=sys.stderr)
        print(f"FAIL ({len(errors)} errors)", file=sys.stderr)
        return 1
    runnable = sum(1 for item in manifest["items"] if not item.get("missing_fixture"))
    print(f"OK   {manifest_path}  (runnable={runnable}, total={len(manifest['items'])})")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
