"""Build CutRight's llama.cpp inference runtime.

This script does NOT download llama.cpp. It validates the manifest contract
and prints the build flags. The actual source fetch is an offline-only
step.

The pull contract:

    upstream commit: 6a32c29a746a2e44de463de647f9f6661eb5086b
    licence: MIT
    mode: library or supervised CutRight sidecar

Network/server features are disabled by default; the only allowed
exception is local IPC for development tests, behind a debug-only flag.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

WORKSPACE_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MANIFEST = WORKSPACE_ROOT / "runtime/manifests/llama-runtime.source.json"

FORBIDDEN_FLAGS = {"--enable-http-server", "--enable-remote-fetch"}
REQUIRED_FEATURES = {"structured", "cancel", "seed", "token_limit", "telemetry_free"}


def _check_contract(manifest: dict) -> list[str]:
    missing: list[str] = []
    for feat in REQUIRED_FEATURES:
        if feat not in manifest.get("features", []):
            missing.append(f"features.{feat}")
    if manifest.get("source", {}).get("commit") in (None, "unresolved", ""):
        missing.append("source.commit")
    return missing


def main() -> int:
    parser = argparse.ArgumentParser(description="Build the llama.cpp inference runtime")
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

    print(f"llama runtime manifest OK for target {args.target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
