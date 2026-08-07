#!/usr/bin/env python3
"""Merge the compiled skill catalogue into the canonical capability registry.

CR-V2-B2-015 (Lane P-B): for each capability in
`docs/dispatch/v2/source/capability-registry.json`, derive a derived
`eval_suites` entry from `skills/_shared/skill-catalogue.json` by collecting
every skill whose `permissions` list contains that capability_id.

After merging, re-validate the registry through
`crates/video-capabilities`'s loader by invoking the validator as a
subprocess (cargo test -p video-capabilities --test canonical_registry) so the
output is checked end-to-end rather than re-implemented in Python.

Exit codes:
  0 — registry validated and eval_suites derived
  1 — catalogue or registry missing/malformed
  2 — validation subprocess failed
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
REGISTRY_PATH = REPO_ROOT / "docs" / "dispatch" / "v2" / "source" / "capability-registry.json"
CATALOGUE_PATH = REPO_ROOT / "skills" / "_shared" / "skill-catalogue.json"
BACKUP_SUFFIX = ".pre-b2-015"


def load_json(path: Path) -> dict:
    with path.open(encoding="utf-8") as fh:
        return json.load(fh)


def derive_eval_suites(catalogue: dict, registry: dict) -> dict[str, list[str]]:
    """For every capability, collect skill ids whose `permissions` list
    contains the capability_id.

    Returns a dict `{capability_id: [suite_id, ...]}` sorted for stability.
    """
    suites_by_capability: dict[str, list[str]] = {
        cap["capability_id"]: [] for cap in registry.get("capabilities", [])
    }
    for skill in catalogue.get("skills", []):
        skill_id = skill.get("id")
        if not isinstance(skill_id, str):
            continue
        # Skill-derived eval suite ids follow the pattern
        # `skill.<id>_<suite-suffix>` so the catalogue's eval_suites list can
        # also pass through, but we always synthesise at minimum the bare
        # `skill.<id>` suite so the registry has *something* even when a
        # skill declares no eval_suites.
        for perm in skill.get("permissions", []):
            if perm in suites_by_capability:
                suites_by_capability[perm].append(f"skill.{skill_id}")
        for suite in skill.get("eval_suites", []):
            if isinstance(suite, str) and suite.startswith("skill."):
                # Skill declares an explicit suite; record it against every
                # capability the skill has permission for.
                for perm in skill.get("permissions", []):
                    if perm in suites_by_capability:
                        suites_by_capability[perm].append(suite)
    # Sort + dedupe for byte-stability
    return {
        cap_id: sorted(set(suites))
        for cap_id, suites in sorted(suites_by_capability.items())
    }


def merge_eval_suites(registry: dict, derived: dict[str, list[str]]) -> int:
    """Replace the `eval_suites` field on each capability with the derived
    value. Returns the count of capabilities touched.
    """
    touched = 0
    for cap in registry.get("capabilities", []):
        cap_id = cap["capability_id"]
        merged = sorted(set(derived.get(cap_id, []) + list(cap.get("eval_suites", []))))
        if merged != cap.get("eval_suites", []):
            cap["eval_suites"] = merged
            touched += 1
    return touched


def validate_registry() -> subprocess.CompletedProcess:
    """Run the Rust validator as a subprocess so the registry is checked by
    the canonical implementation, not by a Python re-implementation."""
    return subprocess.run(
        [
            "cargo",
            "test",
            "-p",
            "video-capabilities",
            "--test",
            "canonical_registry",
            "--locked",
        ],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--write",
        action="store_true",
        help="Persist the merged registry back to disk (default: dry-run report only).",
    )
    parser.add_argument(
        "--no-validate",
        action="store_true",
        help="Skip the Rust validator subprocess (used by the unit tests).",
    )
    args = parser.parse_args(argv)

    if not REGISTRY_PATH.is_file():
        print(f"error: registry missing at {REGISTRY_PATH}", file=sys.stderr)
        return 1
    if not CATALOGUE_PATH.is_file():
        print(f"error: catalogue missing at {CATALOGUE_PATH}", file=sys.stderr)
        return 1

    registry = load_json(REGISTRY_PATH)
    catalogue = load_json(CATALOGUE_PATH)

    derived = derive_eval_suites(catalogue, registry)
    touched = merge_eval_suites(registry, derived)

    try:
        rel_registry = str(REGISTRY_PATH.relative_to(REPO_ROOT))
    except ValueError:
        rel_registry = str(REGISTRY_PATH)
    try:
        rel_catalogue = str(CATALOGUE_PATH.relative_to(REPO_ROOT))
    except ValueError:
        rel_catalogue = str(CATALOGUE_PATH)
    report = {
        "schema": "cutright.skill_catalogue_integration_report/v1",
        "registry_path": rel_registry,
        "catalogue_path": rel_catalogue,
        "capabilities_touched": touched,
        "derived_eval_suites": derived,
    }
    print(json.dumps(report, indent=2, sort_keys=True))

    if args.write:
        backup = REGISTRY_PATH.with_suffix(REGISTRY_PATH.suffix + BACKUP_SUFFIX)
        if not backup.exists():
            backup.write_text(json.dumps(load_json(REGISTRY_PATH), indent=2) + "\n")
        REGISTRY_PATH.write_text(json.dumps(registry, indent=2) + "\n")

    if args.no_validate:
        return 0

    proc = validate_registry()
    sys.stdout.write(proc.stdout)
    sys.stderr.write(proc.stderr)
    return proc.returncode


if __name__ == "__main__":
    sys.exit(main())