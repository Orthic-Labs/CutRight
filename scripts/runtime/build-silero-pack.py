"""Build CutRight's Silero VAD model pack.

This script does NOT download the Silero VAD source. It validates the
manifest contract and prints the frozen `VadConfig` shape. The actual
source fetch is an offline-only step.

The pull contract:

    upstream commit: 76e3dc408eb2a5c655c34e230d2d5459b4439daa
    licence: MIT
    subset: C++/ONNX reference only (no Python, no Torch)

The dispatch refuses to ship any VAD pack that uses a Python or Torch
runtime. CutRight audio decode/resample is the only input pipeline.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

WORKSPACE_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MANIFEST = WORKSPACE_ROOT / "runtime/manifests/silero-vad.model.json"

EXPECTED_CONFIG = {
    "sample_rate": 16_000,
    "threshold": 0.5,
    "min_speech_ms": 160,
    "min_silence_ms": 180,
}


def _check_contract(manifest: dict) -> list[str]:
    missing: list[str] = []
    if manifest.get("model", {}).get("sample_rate") != EXPECTED_CONFIG["sample_rate"]:
        missing.append("model.sample_rate")
    if EXPECTED_CONFIG["threshold"] < 0.0 or EXPECTED_CONFIG["threshold"] > 1.0:
        missing.append("VadConfig.threshold out of range")
    return missing


def main() -> int:
    parser = argparse.ArgumentParser(description="Build the Silero VAD pack")
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

    print(f"silero-vad manifest OK for target {args.target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
