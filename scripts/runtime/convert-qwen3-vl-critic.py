"""Convert Qwen3-VL-4B to the CutRight critic pack.

This script does NOT download the model. It validates the manifest
contract and prints the critic configuration. The actual fetching is
an offline-only step.

The pull contract:

    upstream commit: ebb281ec70b05090aa6165b016eac8ec08e71b17
    licence: Apache-2.0
    role: critic (read-only)

The critic is run in a separate process from the Director; prompts and
seeds are independent. The critic has no mutation capability — every
verdict is recorded as evidence with explicit IDs and time/frame
ranges.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

WORKSPACE_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MANIFEST = WORKSPACE_ROOT / "runtime/manifests/vision-critic.model.json"


def _check_contract(manifest: dict) -> list[str]:
    missing: list[str] = []
    if manifest.get("source", {}).get("commit") in (None, "unresolved", ""):
        missing.append("source.commit")
    if manifest.get("role") != "critic":
        missing.append("role must equal 'critic'")
    if manifest.get("mutation_capable") is not False:
        missing.append("mutation_capable must be false")
    if not manifest.get("image_sampling"):
        missing.append("image_sampling")
    if not manifest.get("video_sampling"):
        missing.append("video_sampling")
    return missing


def main() -> int:
    parser = argparse.ArgumentParser(description="Convert Qwen3-VL-4B to critic pack")
    parser.add_argument("--source-revision", required=True,
                        help="Pinned Qwen3-VL source commit")
    parser.add_argument("--target", default="host")
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    args = parser.parse_args()

    if args.source_revision != "ebb281ec70b05090aa6165b016eac8ec08e71b17":
        print(f"unexpected source revision: {args.source_revision}", file=sys.stderr)
        return 2

    if not args.manifest.exists():
        print(f"missing manifest: {args.manifest}", file=sys.stderr)
        return 2

    manifest = json.loads(args.manifest.read_text())
    missing = _check_contract(manifest)
    if missing:
        print("UNRESOLVED:", file=sys.stderr)
        for f in missing:
            print(f"  {f}", file=sys.stderr)
        return 3

    print(f"critic manifest OK for target {args.target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
