#!/usr/bin/env python3
"""scripts/release/v2-assemble-offline.py — assemble the v2 offline bundle.

Stages a self-describing bundle under the given `--staging` directory:

    app/                     target-specific Tauri artefact
    packs/                   runtime payload packs (lock + bytes)
    repair/                  repair payload for offline re-installs
    licences/                third-party licence notices
    corresponding-source/    LGPL / reciprocal-source bundles
    samples/                 rights-cleared sample projects
    checksums/               SHA256SUMS for every shipped file
    signatures/              local signature manifest per group

This script does not upload, publish or contact any remote service.
"""

from __future__ import annotations
import argparse
import hashlib
import json
import sys
from pathlib import Path

LAYOUT_DIRS = ("app", "packs", "repair", "licences",
               "corresponding-source", "samples", "checksums", "signatures")


def _sha256(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as fh:
        for chunk in iter(lambda: fh.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def _emit_placeholder(path: Path, body: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if not path.exists():
        path.write_text(body)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="assemble v2 offline bundle")
    parser.add_argument("--target", default="host")
    parser.add_argument("--staging", required=True)
    parser.add_argument("--manifest-out", default=None,
                        help="Optional explicit manifest path; defaults to <staging>/../bundle-manifest.json")
    args = parser.parse_args(argv)

    staging = Path(args.staging).resolve()
    staging.mkdir(parents=True, exist_ok=True)

    # Lay out the roots.
    for d in LAYOUT_DIRS:
        (staging / d).mkdir(parents=True, exist_ok=True)

    # Placeholder license notice. A real assemble pulls the cumulative
    # notices into this directory.
    _emit_placeholder(
        staging / "licences" / "README.md",
        "Third-party licence notices are accumulated at v2 build time.\n",
    )
    _emit_placeholder(
        staging / "checksums" / "SHA256SUMS.template",
        "<sha256>  <relative-path>\n",
    )
    _emit_placeholder(
        staging / "signatures" / "README.md",
        "Per-file signatures live alongside this README.\n",
    )
    _emit_placeholder(
        staging / "samples" / "README.md",
        "Rights-cleared sample projects live under this directory.\n",
    )
    _emit_placeholder(
        staging / "corresponding-source" / "README.md",
        "Reciprocal source (LGPL) bundle lives here.\n",
    )

    # Manifest lives at the staging root if a sibling path is not given.
    manifest_path = Path(args.manifest_out).resolve() if args.manifest_out else \
        staging.parent / "bundle-manifest.json"

    files = []
    for p in sorted(staging.rglob("*")):
        if p.is_file() and p != manifest_path:
            files.append(
                {
                    "path": str(p.relative_to(staging)),
                    "size": p.stat().st_size,
                    "sha256": _sha256(p),
                }
            )

    manifest = {
        "schema_version": "v2",
        "target": args.target,
        "staging": ".",
        "layout": list(LAYOUT_DIRS),
        "files": files,
        "external_runtime_dependencies": [],
    }
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True))
    print(f"wrote manifest {manifest_path} with {len(files)} files")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
