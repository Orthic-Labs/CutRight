#!/usr/bin/env python3
"""scan_assets.py — scan a vendored tree for model/dictionary/runtime assets.

Usage: python3 tools/import-closure/scan_assets.py DIR --out FILE.json
Emits an asset ledger: every redistributable asset candidate with sha256.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _common import sha256_file, walk_files, write_json  # noqa: E402

ASSET_SUFFIXES = {
    ".onnx", ".gguf", ".bin", ".pt", ".tflite", ".safetensors",
    ".dict", ".vocab", ".txt.gz", ".mmproj",
    ".wav", ".mp3", ".flac", ".ogg",
    ".ttf", ".otf", ".woff", ".woff2",
    ".lut", ".cube", ".png", ".jpg", ".jpeg", ".webp", ".exr",
}
ASSET_DIR_HINTS = {"models", "assets", "weights", "voices", "dictionaries", "fonts", "luts"}


def classify(rel: Path) -> str:
    parts = {p.lower() for p in rel.parts[:-1]}
    suffix = rel.suffix.lower()
    if parts & {"models", "weights"} or suffix in {".onnx", ".gguf", ".safetensors", ".pt", ".tflite", ".mmproj"}:
        return "model_weights"
    if parts & {"voices"} or suffix in {".wav", ".mp3", ".flac", ".ogg"}:
        return "voices"
    if parts & {"fonts"} or suffix in {".ttf", ".otf", ".woff", ".woff2"}:
        return "fonts"
    if parts & {"luts"} or suffix in {".lut", ".cube"}:
        return "luts"
    if parts & {"dictionaries"} or suffix in {".dict", ".vocab"}:
        return "dataset"
    if suffix in {".png", ".jpg", ".jpeg", ".webp", ".exr"}:
        return "textures"
    return "sample_media"


def main(argv: list[str]) -> int:
    if len(argv) != 4 or argv[2] != "--out":
        print(__doc__, file=sys.stderr)
        return 2
    root = Path(argv[1]).resolve()
    if not root.is_dir():
        print(f"ERROR: not a directory: {root}", file=sys.stderr)
        return 1
    assets = []
    for path in walk_files(root):
        rel = path.relative_to(root)
        parts_lower = {p.lower() for p in rel.parts}
        if rel.suffix.lower() in ASSET_SUFFIXES or parts_lower & ASSET_DIR_HINTS:
            assets.append(
                {
                    "path": rel.as_posix(),
                    "asset_class": classify(rel),
                    "sha256": sha256_file(path),
                    "bytes": path.stat().st_size,
                    "licence_status": "pending_explicit_row",
                }
            )
    doc = {
        "schema_version": 1,
        "root": str(root),
        "asset_count": len(assets),
        "policy": "Every asset needs an explicit licence row in imports/v2/dispositions.json before entering a signed pack; assets never inherit a repository licence.",
        "assets": assets,
    }
    write_json(Path(argv[3]), doc)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
