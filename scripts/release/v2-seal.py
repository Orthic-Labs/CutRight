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
import re
import sys
from pathlib import Path


def _sha256(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as fh:
        for chunk in iter(lambda: fh.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def _build_manifest(root: Path, excluded: Path | None = None) -> dict:
    excluded = excluded.resolve() if excluded else None
    active_seal = (root / "SEAL.json").resolve()
    items = []
    for p in sorted(root.rglob("*")):
        if p.is_file() and p.resolve() not in (active_seal, excluded):
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


def _verify_items(target: Path, seal: object) -> int | None:
    items = seal.get("items") if isinstance(seal, dict) else None
    if not isinstance(items, list) or not items:
        return None
    root = target.resolve()
    if not root.is_dir():
        return None
    expected = {
        path.resolve()
        for path in root.rglob("*")
        if path.is_file() and path.resolve() != (root / "SEAL.json").resolve()
    }
    seen: set[Path] = set()
    for item in items:
        if not isinstance(item, dict):
            return None
        name, digest = item.get("path"), item.get("sha256")
        if (not isinstance(name, str) or not name or not isinstance(digest, str)
                or re.fullmatch(r"[0-9a-fA-F]{64}", digest) is None):
            return None
        relative = Path(name)
        if relative.is_absolute() or ".." in relative.parts or "\\" in name:
            return None
        path = (root / relative).resolve()
        try:
            path.relative_to(root)
        except ValueError:
            return None
        if path in seen or path not in expected or not path.is_file() or _sha256(path) != digest:
            return None
        seen.add(path)
    if seen != expected:
        return None
    return len(items)


def _read_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def _canonical_seal(root: Path, requested: str | None) -> Path | None:
    """Only bundled SEAL.json may drive verification or state transitions."""
    canonical = (root / "SEAL.json").resolve()
    if requested is not None and Path(requested).resolve() != canonical:
        return None
    return canonical


def _write_atomic(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    try:
        temporary.write_bytes(data)
        os.replace(temporary, path)
    finally:
        if temporary.exists():
            temporary.unlink()


def _verify_and_mark(root: Path, seal_path: Path, count: int) -> bool:
    """Commit verified=true only with a freshly sealed, reverified bundle."""
    bundled = root / "RC-MANIFEST.json"
    if not bundled.is_file():
        return True
    original_manifest = original_seal = original_export = None
    exported = root.parent / "RC-MANIFEST.json"
    try:
        manifest = _read_json(bundled)
        if not isinstance(manifest, dict):
            return False
        if "verified" not in manifest:
            return True
        if manifest.get("verified") is not False:
            return manifest.get("verified") is True
        original_manifest = bundled.read_bytes()
        original_seal = seal_path.read_bytes()
        original_export = exported.read_bytes() if exported.exists() else None
        manifest["verified"] = True
        marked = json.dumps(manifest, indent=2, sort_keys=True) + "\n"
        _write_atomic(bundled, marked.encode("utf-8"))
        rebuilt = _build_manifest(root, seal_path)
        rebuilt["seal_target"] = str(root)
        _write_atomic(seal_path, (json.dumps(rebuilt, indent=2, sort_keys=True) + "\n").encode("utf-8"))
        if _verify_items(root, _read_json(seal_path)) is None:
            raise ValueError("reverification failed")
        _write_atomic(exported, bundled.read_bytes())
        return True
    except (OSError, ValueError, UnicodeError, TypeError, json.JSONDecodeError):
        try:
            if original_manifest is not None:
                _write_atomic(bundled, original_manifest)
            if original_seal is not None:
                _write_atomic(seal_path, original_seal)
            if original_export is None:
                if exported.exists():
                    exported.unlink()
            else:
                _write_atomic(exported, original_export)
        except OSError:
            pass
        return False


def cmd_seal(args) -> int:
    root = Path(args.bundle).resolve()
    out = Path(args.manifest).resolve()
    if not root.is_dir():
        raise FileNotFoundError(f"bundle directory not found: {root}")
    manifest = _build_manifest(root, out)
    manifest["seal_target"] = str(root)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(manifest, indent=2, sort_keys=True))
    return 0


def cmd_legacy_seal(args) -> int:
    """Apply frozen-plan semantics: export RC metadata and write bundle seal."""
    root = Path(args.seal).resolve()
    bundled_manifest = root / "RC-MANIFEST.json"
    exported_manifest = Path(args.manifest).resolve()
    if not root.is_dir():
        raise FileNotFoundError(f"bundle directory not found: {root}")
    if not bundled_manifest.is_file():
        raise FileNotFoundError(f"bundled release candidate manifest not found: {bundled_manifest}")
    if exported_manifest != bundled_manifest.resolve():
        exported_manifest.parent.mkdir(parents=True, exist_ok=True)
        exported_manifest.write_bytes(bundled_manifest.read_bytes())
    seal_path = root / "SEAL.json"
    manifest = _build_manifest(root, seal_path)
    manifest["seal_target"] = str(root)
    seal_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    return 0


def cmd_verify(args) -> int:
    target = Path(args.bundle).resolve()
    seal_path = _canonical_seal(target, getattr(args, "seal", None))
    if seal_path is None:
        print("verification failed:")
        return 1
    try:
        count = _verify_items(target, _read_json(seal_path))
    except (OSError, ValueError, UnicodeError, TypeError):
        count = None
    if count is None:
        print("verification failed:")
        return 1
    if not _verify_and_mark(target, seal_path, count):
        print("verification failed:")
        return 1
    print(f"verified {count} files")
    return 0


def cmd_verify_provenance(args) -> int:
    bundle = Path(args.bundle).resolve()
    prov_path = Path(args.provenance).resolve()
    if not prov_path.exists():
        print(f"provenance file not found: {prov_path}")
        return 1
    try:
        prov = _read_json(prov_path)
        if not isinstance(prov, dict):
            raise ValueError("provenance must be an object")
        seal_target = Path(prov.get("seal_target") or bundle)
        seal = _read_json(Path(args.seal or seal_target / "SEAL.json"))
        count = _verify_items(bundle, seal)
    except (OSError, ValueError, UnicodeError, TypeError):
        count = None
    if count is None:
        print("verification failed:")
        return 1
    print(f"provenance-verified {count} files")
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
    # Legacy plan forms remain accepted alongside subcommands.
    if argv and argv[0] in ("--seal", "--verify", "--checksums"):
        legacy = argparse.ArgumentParser(description="v2 seal / verify")
        legacy.add_argument("--seal")
        legacy.add_argument("--verify")
        legacy.add_argument("--checksums")
        legacy.add_argument("--manifest")
        legacy.add_argument("--out")
        args = legacy.parse_args(argv)
        selected = [value for value in (args.seal, args.verify, args.checksums) if value is not None]
        if len(selected) != 1:
            legacy.error("choose exactly one of --seal, --verify, or --checksums")
        if args.seal is not None:
            if not args.manifest or args.out:
                legacy.error("--seal requires --manifest and does not accept --out")
            return cmd_legacy_seal(args)
        if args.verify is not None:
            if args.manifest or args.out:
                legacy.error("--verify does not accept --manifest or --out")
            args.bundle = args.verify
            args.seal = None
            return cmd_verify(args)
        if not args.out or args.manifest:
            legacy.error("--checksums requires --out and does not accept --manifest")
        args.bundle = args.checksums
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
