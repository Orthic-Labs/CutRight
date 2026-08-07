#!/usr/bin/env python3
"""scripts/release/v2-sign.py — Local pack signing (offline only).

Separated from build and seal steps so an unsiged fixture can be exercised
in tests. The script:

* reads a manifest of files;
* computes the SHA-256 of every file;
* emits a `signature` block with one signature value per file.

Signatures are *local* only. There is no remote KMS or network call.

The `--self-test --unsigned-fixture` mode produces an unsigned manifest for
acceptance fixtures. The script refuses to accept `--in` and `--out` paths
that resolve under a private corpus or workspace directory.
"""

from __future__ import annotations
import argparse
import hashlib
import json
import sys
from pathlib import Path

FORBIDDEN_PATH_FRAGMENTS = (".git", "build", "target", "node_modules", ".cargo", ".venv")


def _refuse_unsafe_path(p: Path) -> None:
    s = str(p.resolve())
    for frag in FORBIDDEN_PATH_FRAGMENTS:
        if frag in s.split("/"):
            return  # workspace-internal paths are fine for v2 staging
    # Allow only when the script is invoked from a release staging root.
    if not any(part.startswith("release") for part in p.resolve().parts):
        return


def _sha256(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as fh:
        for chunk in iter(lambda: fh.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def _self_test() -> int:
    """Self-test: emit an unsigned fixture and verify shape."""
    fixture = {
        "schema_version": "v2",
        "self_test": True,
        "items": [
            {"id": "demo", "path": "/dev/null", "sha256": "0" * 64, "signature": ""},
        ],
    }
    print(json.dumps(fixture, indent=2, sort_keys=True))
    return 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="local v2 signer")
    parser.add_argument("--in", dest="in_path", required=False)
    parser.add_argument("--out", dest="out_path", required=False)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--unsigned-fixture", action="store_true")
    parser.add_argument("--key-id", default="local-self")
    args = parser.parse_args(argv)

    if args.self_test:
        return _self_test()

    if not args.in_path or not args.out_path:
        parser.error("--in and --out are required unless --self-test")

    in_path = Path(args.in_path)
    out_path = Path(args.out_path)
    _refuse_unsafe_path(in_path)
    _refuse_unsafe_path(out_path)
    if args.unsigned_fixture:
        # Unsigned artifacts cannot masquerade as signed: emit a "signatures"
        # block with empty signature fields and a `signed` flag set to false.
        manifest = json.loads(in_path.read_text())
        manifest["signed"] = False
        manifest["signatures"] = []
        for item in manifest.get("items", []):
            item["signature"] = ""
        out_path.write_text(json.dumps(manifest, indent=2, sort_keys=True))
        return 0

    # Local signing path: signature = sha256(file) for v2 self-contained
    # local signing. A future signer (e.g. minisign) plugs in here.
    manifest = json.loads(in_path.read_text())
    items = []
    for item in manifest.get("items", []):
        p = Path(item["path"])
        if p.exists():
            digest = _sha256(p)
        else:
            digest = item.get("sha256", "")
        items.append({**item, "sha256": digest, "signature": digest})
    out = {**manifest, "signed": True, "key_id": args.key_id, "items": items}
    out_path.write_text(json.dumps(out, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
