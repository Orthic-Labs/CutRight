"""Build CutRight's whisper.cpp verifier pack.

This script does NOT download whisper.cpp. It validates the manifest
contract and prints the frozen verifier configuration. The actual
source fetch is an offline-only step.

The pull contract:

    upstream commit: 306c88f4d1286aec1bf96e544632897886af5501
    licence: MIT
    role: independent verifier (NOT canonical transcript authority)

If a model licence or hash is unresolved the script aborts with the
unresolved field. CutRight NEVER promotes the verifier's output to
canonical transcript authority; the two engines coexist as evidence.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

WORKSPACE_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MANIFEST = WORKSPACE_ROOT / "runtime/manifests/whisper-verifier.model.json"


def _check_contract(manifest: dict) -> list[str]:
    missing: list[str] = []
    if manifest.get("model", {}).get("sha256") in (None, "unresolved", ""):
        missing.append("model.sha256")
    if manifest.get("model", {}).get("license") in (None, "unresolved", ""):
        missing.append("model.license")
    if manifest.get("role") != "verifier":
        missing.append("role must equal 'verifier'")
    return missing


def main() -> int:
    parser = argparse.ArgumentParser(description="Build the whisper.cpp verifier pack")
    parser.add_argument("--target", default="host")
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    args = parser.parse_args()

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

    print(f"whisper-verifier manifest OK for target {args.target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
