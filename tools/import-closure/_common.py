"""Shared helpers for the CutRight v2 import tooling (stdlib only)."""

from __future__ import annotations

import hashlib
import json
import os
import sys
from pathlib import Path


def repo_root(start: Path | None = None) -> Path:
    here = (start or Path(__file__)).resolve()
    for parent in [here, *here.parents]:
        if (parent / "imports" / "v2").is_dir():
            return parent
    raise SystemExit("cannot locate CutRight repository root (imports/v2 missing)")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def walk_files(root: Path) -> list[Path]:
    out: list[Path] = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(d for d in dirnames if d != ".git")
        for name in sorted(filenames):
            out.append(Path(dirpath) / name)
    out.sort(key=lambda p: p.relative_to(root).as_posix())
    return out


def hash_tree(root: Path) -> dict:
    root = root.resolve()
    entries = []
    total_bytes = 0
    for path in walk_files(root):
        size = path.stat().st_size
        total_bytes += size
        entries.append(
            {
                "path": path.relative_to(root).as_posix(),
                "sha256": sha256_file(path),
                "bytes": size,
            }
        )
    path_list = "\n".join(e["path"] for e in entries).encode("utf-8")
    return {
        "root": str(root),
        "file_count": len(entries),
        "total_bytes": total_bytes,
        "sha256_of_sorted_path_list": hashlib.sha256(path_list).hexdigest(),
        "files": entries,
    }


def write_json(path: Path, doc: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=2)
        fh.write("\n")
    print(f"wrote {path}")


def read_json(path: Path) -> dict:
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


def fail(message: str) -> "sys.exit":
    print(f"ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)
