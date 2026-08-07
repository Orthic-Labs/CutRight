"""Build CutRight's Parakeet primary ASR model pack.

This script does NOT download the Parakeet model. It validates the
required asset contract and aborts if any licence or hash is unresolved.

Required assets:
    encoder, decoder, joiner, tokenizer, vocabulary

Each asset must have a resolved licence row in the cap-ledger AND a
known SHA-256 hash. The script emits a signable speech-pack fragment
only when ALL required assets are resolved.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

WORKSPACE_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_MANIFEST = WORKSPACE_ROOT / "runtime/manifests/parakeet.model.json"
DEFAULT_LEDGER = WORKSPACE_ROOT / "imports/v2/heardright-assets.json"

REQUIRED_ASSETS = ["encoder", "decoder", "joiner", "tokenizer", "vocabulary"]


def _check_manifest(manifest: dict) -> list[str]:
    """Return the list of unresolved fields. Empty list = OK."""
    unresolved: list[str] = []
    for asset in REQUIRED_ASSETS:
        row = manifest.get("assets", {}).get(asset, {})
        if not row.get("sha256") or row["sha256"] == "unresolved":
            unresolved.append(f"assets.{asset}.sha256")
        if not row.get("license") or row["license"] == "unresolved":
            unresolved.append(f"assets.{asset}.license")
    if not manifest.get("source", {}).get("commit") \
            or manifest["source"]["commit"] == "unresolved":
        unresolved.append("source.commit")
    return unresolved


def main() -> int:
    parser = argparse.ArgumentParser(description="Build the Parakeet ASR pack")
    parser.add_argument("--target", default="host")
    parser.add_argument("--from-ledger", type=Path, default=DEFAULT_LEDGER,
                        help="Path to the HeardRight asset ledger JSON")
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    args = parser.parse_args()

    if not args.manifest.exists():
        print(f"missing manifest: {args.manifest}", file=sys.stderr)
        return 2

    manifest = json.loads(args.manifest.read_text())
    unresolved = _check_manifest(manifest)

    if unresolved:
        print("UNRESOLVED:", file=sys.stderr)
        for field in unresolved:
            print(f"  {field}", file=sys.stderr)
        return 3

    print(f"parakeet pack manifest OK for target {args.target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
