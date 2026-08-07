"""Build CutRight's Kokoro local TTS and phonemizer pack.

This script does NOT download the model. It validates the manifest
contract and the voice file ledger. The actual fetching is an
offline-only step.

The pull contract:

    model_sha256: 496dba118d1a58f5f3db2efc88dbdc216e0483fc89fe6e47ee1f2c53f18ad1e4
    licence: Apache-2.0
    runtime: native ONNX (no Python, no espeak system dependency)

The script asserts:

    voice_pack.files = [model, tokenizer_or_config, phonemizer_data, audited_voice_files]
    all(file.license_resolved for file in voice_pack.files)

If any voice file is unresolved, the script aborts.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

WORKSPACE_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MANIFEST = WORKSPACE_ROOT / "runtime/manifests/voice.model.json"

EXPECTED_MODEL_HASH = "496dba118d1a58f5f3db2efc88dbdc216e0483fc89fe6e47ee1f2c53f18ad1e4"


def _check_contract(manifest: dict) -> list[str]:
    missing: list[str] = []
    if manifest.get("model", {}).get("sha256") != EXPECTED_MODEL_HASH:
        missing.append(f"model.sha256 must equal {EXPECTED_MODEL_HASH[:16]}...")
    if manifest.get("runtime") != "native-onnx":
        missing.append("runtime must be 'native-onnx'")
    if manifest.get("dependency_python") is not False:
        missing.append("dependency_python must be false")
    if manifest.get("dependency_espeak") is not False:
        missing.append("dependency_espeak must be false")
    files = manifest.get("files", [])
    if not files:
        missing.append("files")
    for index, file in enumerate(files):
        if file.get("license") in (None, "unresolved", ""):
            missing.append(f"files[{index}].license")
    return missing


def main() -> int:
    parser = argparse.ArgumentParser(description="Build the Kokoro TTS pack")
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

    print(f"kokoro voice manifest OK for target {args.target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
