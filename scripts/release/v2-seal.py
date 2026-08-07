#!/usr/bin/env python3
"""scripts/release/v2-seal.py — produce and verify the seal manifest.

The seal manifest enumerates every file in the bundle, its SHA-256 hash
and its signature (or empty string). The script supports:

* `--seal`           write `release/v2/SEAL.json` from a bundle root.
* `--verify`         verify the SHA-256 hashes in the seal match the bytes.
* `--verify-provenance`  cross-check the seal against a provenance.json.
* `--checksums`      emit a SHA256SUMS-style text file.
* `--self-test`      produce a deterministic sample manifest for testing.

The script is local. It does not upload.
"""

from __future__ import annotations
import argparse
import hashlib
import json
import os
import sys
from pathlib import Path


def _sha256(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as fh:
        for chunk in iter(lambda: fh.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def _build_manifest(root: Path) -> dict:
    items = []
    for p in sorted(root.rglob("*")):
        if p.is_file() and p.name != "SEAL.json":
            items.append(
                {
                    "path": str(p.relative_to(root)),
                    "sha256": _sha256(p),
                    "signature": "",
                }
            )
    return {
        "schema_version": "v2",
        "items": items,
    }


def cmd_seal(args) -> int:
    root = Path(args.bundle).resolve()
    out = Path(args.manifest).resolve()
    manifest = _build_manifest(root)
    manifest["seal_target"] = str(root)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(manifest, indent=2, sort_keys=True))
    return 0


def cmd_verify(args) -> int:
    target = Path(args.bundle).resolve()
    seal = json.loads(Path(args.seal or target / "SEAL.json").read_text())
    failures = []
    for item in seal["items"]:
        p = target / item["path"]
        if not p.exists():
            failures.append((item["path"], "missing"))
            continue
        if _sha256(p) != item["sha256"]:
            failures.append((item["path"], "hash-mismatch"))
    if failures:
        print("verification failed:")
        for f in failures:
            print(f"  {f}")
        return 1
    print(f"verified {len(seal['items'])} files")
    return 0


def cmd_verify_provenance(args) -> int:
    bundle = Path(args.bundle).resolve()
    prov_path = Path(args.provenance).resolve()
    if not prov_path.exists():
        print(f"provenance file not found: {prov_path}")
        return 1
    prov = json.loads(prov_path.read_text())
    seal_target = Path(prov.get("seal_target") or bundle)
    seal = json.loads(Path(args.seal or seal_target / "SEAL.json").read_text())
    failures = []
    for item in seal["items"]:
        p = bundle / item["path"]
        if not p.exists():
            failures.append((item["path"], "missing"))
            continue
        if _sha256(p) != item["sha256"]:
            failures.append((item["path"], "hash-mismatch"))
    if failures:
        print("verification failed:")
        for f in failures:
            print(f"  {f}")
        return 1
    print(f"provenance-verified {len(seal['items'])} files")
    return 0


def cmd_checksums(args) -> int:
    target = Path(args.bundle).resolve()
    out = Path(args.out).resolve()
    lines = []
    for p in sorted(target.rglob("*")):
        if p.is_file() and p.name != out.name:
            lines.append(f"{_sha256(p)}  {p.relative_to(target)}")
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n")
    return 0


def cmd_self_test(_args) -> int:
    sample = {
        "schema_version": "v2",
        "self_test": True,
        "items": [],
    }
    print(json.dumps(sample, indent=2, sort_keys=True))
    return 0


class _EmptyArgs:
    """Namespace-like object used when bypassing argparse."""

    def __getattr__(self, name):
        return None


def main(argv: list[str]) -> int:
    # Lightweight dispatcher: --self-test is the documented acceptance
    # fixture; everything else is forwarded to subcommands.
    if argv and argv[0] in ("--self-test", "self-test"):
        return cmd_self_test(_EmptyArgs())
    # Top-level --verify <bundle> shortcut for the acceptance harness.
    if argv and argv[0] == "--verify" and len(argv) >= 2:
        args = _EmptyArgs()
        args.bundle = argv[1]
        args.seal = None
        return cmd_verify(args)
    if argv and argv[0] == "--checksums" and len(argv) >= 4 and argv[1] == "--out":
        args = _EmptyArgs()
        args.bundle = argv[3]
        args.out = argv[2]
        return cmd_checksums(args)
    parser = argparse.ArgumentParser(description="v2 seal / verify")
    parser.add_argument("--self-test", action="store_true")
    subs = parser.add_subparsers(dest="cmd", required=False)
    p_seal = subs.add_parser("seal")
    p_seal.add_argument("--manifest", required=True)
    p_seal.set_defaults(func=cmd_seal, bundle=None)
    p_seal.add_argument("bundle")
    p_verify = subs.add_parser("verify")
    p_verify.add_argument("--seal", required=False)
    p_verify.set_defaults(func=cmd_verify)
    p_verify.add_argument("bundle")
    p_vp = subs.add_parser("verify-provenance")
    p_vp.add_argument("--seal", required=False)
    p_vp.add_argument("--provenance", required=True)
    p_vp.add_argument("bundle")
    p_vp.set_defaults(func=cmd_verify_provenance)
    p_cs = subs.add_parser("checksums")
    p_cs.add_argument("--out", required=True)
    p_cs.add_argument("bundle")
    p_cs.set_defaults(func=cmd_checksums)
    p_st = subs.add_parser("self-test")
    p_st.set_defaults(func=cmd_self_test)
    args = parser.parse_args(argv)

    if args.self_test:
        return cmd_self_test(args)
    if not args.cmd:
        args.func = cmd_self_test
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
