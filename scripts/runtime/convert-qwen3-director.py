"""Convert Qwen3-4B to the CutRight Director model pack.

This script does NOT download the model. It validates the manifest
contract and prints the candidate-quantisation selection rule. The
actual fetching is an offline-only step.

The pull contract:

    upstream commit: 7c69a109fc3fa19c860be9dff46fc23299092018
    licence: Apache-2.0

The conversion is performed by the pinned llama.cpp converter
(`runtime/source/llama.cpp/.../bin/llama-convert`). The selection rule
is the minimum-size candidate that meets every floor:

    schema_validity_floor
    editorial_eval_floor
    tool_choice_floor
    target_memory_floor
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

WORKSPACE_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MANIFEST = WORKSPACE_ROOT / "runtime/manifests/director.model.json"

REQUIRED_FIELDS = [
    "source.commit",
    "tokenizer",
    "chat_template",
    "quantisation",
    "context_window",
    "sampling_defaults",
]


def _check_contract(manifest: dict) -> list[str]:
    missing: list[str] = []
    if manifest.get("source", {}).get("commit") in (None, "unresolved", ""):
        missing.append("source.commit")
    for field in ("tokenizer", "chat_template", "quantisation"):
        if manifest.get(field) in (None, "unresolved", ""):
            missing.append(field)
    if manifest.get("context_window", 0) <= 0:
        missing.append("context_window")
    if not manifest.get("sampling_defaults"):
        missing.append("sampling_defaults")
    return missing


def main() -> int:
    parser = argparse.ArgumentParser(description="Convert Qwen3-4B to Director pack")
    parser.add_argument("--source-revision", required=True,
                        help="Pinned Qwen3 source commit")
    parser.add_argument("--target", default="host")
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    args = parser.parse_args()

    if args.source_revision != "7c69a109fc3fa19c860be9dff46fc23299092018":
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

    print(f"director manifest OK for target {args.target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
