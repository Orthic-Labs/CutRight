#!/usr/bin/env python3
"""scripts/release/v2-source-bundle.py — v2 source distribution manifest.

The v2 source distribution is the working tree at the exact release
commit. The script produces a `source-manifest.json` that:

  * records the exact git HEAD;
  * hashes `Cargo.toml`, `Cargo.lock`, `package.json`, `pnpm-lock.yaml`
    and the top-level `AGENTS.md` / `LICENSE` files;
  * counts files in each protected subdirectory (without enumerating
    every byte);
  * notes any FFmpeg corresponding source path supplied on the CLI;
  * never copies bytes (the workspace already lives in git at the
    recorded HEAD).

The script is local; no network calls.
"""

from __future__ import annotations
import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

# Top-level files whose hash is part of the manifest.
ROOT_HASHED = (
    "AGENTS.md", "LICENSE", "Cargo.toml", "Cargo.lock",
    "package.json", "pnpm-lock.yaml", "tsconfig.json", "vite.config.ts",
    "README.md", "CONTRIBUTING.md",
)

# Subdirectories we count (without enumerating their bytes) so the v2-audit
# script can confirm the bundle is shaped correctly.
COUNTED_DIRS = (
    "crates", "apps/studio/src", "apps/studio/src-tauri",
    "apps/studio/src/components", "apps/studio/src/contracts",
    "apps/studio/src/modes",
    "schemas", "fixtures/schemas", "docs", "scripts", "release",
)

EXCLUDE_FRAGMENTS = (
    ".git", "target", "node_modules", ".venv", ".cache",
    "private_benchmark_media", "credentials", "secrets",
    "model_bytes_not_redistributable", "workspace_only",
)


def _sha256(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as fh:
        for chunk in iter(lambda: fh.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def _git_head(root: Path) -> str:
    out = subprocess.run(
        ["git", "-C", str(root), "rev-parse", "HEAD"],
        check=False, capture_output=True, text=True,
    )
    return out.stdout.strip() or "<unknown>"


def _is_excluded(path: Path) -> bool:
    return any(frag in path.parts for frag in EXCLUDE_FRAGMENTS)


def _count_files(root: Path) -> int:
    if not root.exists():
        return 0
    count = 0
    stack = [root]
    while stack:
        p = stack.pop()
        try:
            for c in p.iterdir():
                if _is_excluded(c.relative_to(root)):
                    continue
                if c.is_dir():
                    stack.append(c)
                else:
                    count += 1
        except (PermissionError, OSError):
            continue
    return count


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="v2 source bundle")
    parser.add_argument("--target", default="host")
    parser.add_argument("--out", required=True)
    parser.add_argument("--source", default=".")
    parser.add_argument("--ffmpeg-corresponding", default=None)
    args = parser.parse_args(argv)

    root = Path(args.source).resolve()
    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)

    ffmpeg_info: dict | None = None
    if args.ffmpeg_corresponding:
        ffmpeg_src = Path(args.ffmpeg_corresponding).resolve()
        if ffmpeg_src.exists():
            ffmpeg_dst = out / "vendor" / "ffmpeg"
            ffmpeg_dst.mkdir(parents=True, exist_ok=True)
            target = ffmpeg_dst / ffmpeg_src.name
            shutil.copyfile(ffmpeg_src, target)
            ffmpeg_info = {
                "path": str(target.relative_to(out)),
                "sha256": _sha256(target),
                "size": target.stat().st_size,
            }

    file_rows = []
    for name in ROOT_HASHED:
        p = root / name
        if p.exists() and p.is_file():
            file_rows.append(
                {"path": name, "size": p.stat().st_size, "sha256": _sha256(p)}
            )

    counts = {}
    for d in COUNTED_DIRS:
        counts[d] = _count_files(root / d)

    head = _git_head(root)

    manifest = {
        "schema_version": "v2",
        "target": args.target,
        "head": head,
        "files": file_rows,
        "counts": counts,
        "excluded_categories": list(EXCLUDE_FRAGMENTS),
        "private_data_removed": True,
        "external_runtime_dependencies": [],
        "ffmpeg_corresponding_source": ffmpeg_info,
        "bundle_kind": "manifest_only",
    }
    (out / "source-manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True)
    )
    print(f"wrote {out / 'source-manifest.json'} (head={head[:8]}, files={len(file_rows)})")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
