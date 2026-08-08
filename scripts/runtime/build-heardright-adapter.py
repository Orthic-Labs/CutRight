#!/usr/bin/env python3
"""Build deterministic CutRight speech adapter metadata from vendored source.

No network, PATH lookup, sibling checkout, or user model directory is used.
The adapter is intentionally a metadata pack until native inference is linked.
"""
from __future__ import annotations
import argparse, hashlib, json, platform, sys
from pathlib import Path

def digest_tree(root: Path) -> tuple[str, list[dict]]:
    rows = []
    for path in sorted(p for p in root.rglob("*") if p.is_file() and ".git" not in p.parts):
        rel = path.relative_to(root).as_posix()
        data = path.read_bytes()
        rows.append({"path": rel, "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data)})
    canonical = "\n".join(f"{r['path']}:{r['sha256']}" for r in rows).encode()
    return hashlib.sha256(canonical).hexdigest(), rows

def build(root: Path, out: Path) -> dict:
    vendor = root / "vendor" / "heardright"
    if not vendor.is_dir(): raise FileNotFoundError(vendor)
    revision, files = digest_tree(vendor)
    pack = {
        "schema": "cutright.speech_adapter/v1", "pack_id": "speech", "version": "0.1.0",
        "source": {"root": "vendor/heardright", "revision": revision, "license": "MIT OR Apache-2.0",
                   "compiler": {"name": "metadata-only", "version": "1", "target": platform.machine()}},
        "build": {"offline": True, "network": False, "path_lookup": False,
                  "command": "python3 scripts/runtime/build-heardright-adapter.py --root ."},
        "files": files,
        "capabilities": ["speech.transcribe", "speech.vad", "speech.engine-dispatch"],
        "outputs": [{"path": "packs/speech/PACK.json", "kind": "manifest"}],
    }
    out.mkdir(parents=True, exist_ok=True)
    (out / "PACK.json").write_text(json.dumps(pack, indent=2, sort_keys=True) + "\n")
    return pack

def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(); ap.add_argument("--root", default="."); ap.add_argument("--out", default="runtime/packs/speech")
    args = ap.parse_args(argv); pack = build(Path(args.root).resolve(), Path(args.out).resolve())
    print(json.dumps({"revision": pack["source"]["revision"], "files": len(pack["files"]), "offline": True}, sort_keys=True))
    return 0
if __name__ == "__main__": sys.exit(main(sys.argv[1:]))
