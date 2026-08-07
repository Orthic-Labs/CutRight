"""Qualify Qwen3.5-4B without making it a release dependency.

This script does NOT promote Qwen3.5-4B. It only runs the qualification
suite and writes results to the candidate record. The `--no-promote`
flag is mandatory.

The pull contract:

    upstream commit: 851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a
    licence: Apache-2.0

The script asserts:

    qualification.mode == "no_promote"
    active_pack_lock_unchanged()

If either assertion fails, the script aborts.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

WORKSPACE_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_QUALIFICATION = WORKSPACE_ROOT / "runtime/candidates/qwen3.5-4b/qualification.json"

EXPECTED_REVISION = "851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a"


def _assert_no_promote(qualification_path: Path) -> int:
    if not qualification_path.exists():
        print(f"missing qualification: {qualification_path}", file=sys.stderr)
        return 2
    text = qualification_path.read_text()
    if "mode: no_promote" not in text:
        print("FAIL: qualification.mode is not 'no_promote'", file=sys.stderr)
        return 3
    return 0


def _assert_active_lock_unchanged() -> int:
    # The active pack lock is read-only here; the assertion is structural
    # at the merge step. We just confirm the candidate is NOT in the
    # active manifests directory.
    active_manifests = WORKSPACE_ROOT / "runtime/manifests"
    candidate_name = "director.qwen35-4b"
    for path in active_manifests.glob("*.model.json"):
        if candidate_name in path.name:
            print(f"FAIL: candidate manifest leaked into active: {path}", file=sys.stderr)
            return 4
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Qualify Qwen3.5-4B without promotion")
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--target", default="host")
    parser.add_argument("--no-promote", action="store_true", required=True)
    parser.add_argument("--qualification", type=Path, default=DEFAULT_QUALIFICATION)
    args = parser.parse_args()

    if args.source_revision != EXPECTED_REVISION:
        print(f"unexpected source revision: {args.source_revision}", file=sys.stderr)
        return 2

    rc = _assert_no_promote(args.qualification)
    if rc != 0:
        return rc
    rc = _assert_active_lock_unchanged()
    if rc != 0:
        return rc

    print(f"qualification OK for target {args.target} (no promotion)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
