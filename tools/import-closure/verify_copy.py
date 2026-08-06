#!/usr/bin/env python3
"""verify_copy.py — verify a copied tree against its closure graph.

Usage: python3 tools/import-closure/verify_copy.py GRAPH.json DEST_DIR
Exit 0 when every graph entry exists at DEST with the recorded sha256 and
no extra files are present; exit 1 otherwise.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _common import fail, read_json, sha256_file, walk_files  # noqa: E402


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    graph = read_json(Path(argv[1]))
    dest = Path(argv[2]).resolve()
    if not dest.is_dir():
        fail(f"destination missing: {dest}")

    expected = {entry["path"]: entry["sha256"] for entry in graph["files"]}
    actual_files = [p.relative_to(dest).as_posix() for p in walk_files(dest)]
    actual = set(actual_files)

    errors = []
    for rel, sha in sorted(expected.items()):
        path = dest / rel
        if not path.is_file():
            errors.append(f"missing: {rel}")
        elif sha256_file(path) != sha:
            errors.append(f"hash mismatch: {rel}")
    for rel in sorted(actual - set(expected)):
        errors.append(f"unexpected extra file: {rel}")

    if errors:
        for err in errors[:50]:
            print(f"FAIL {err}", file=sys.stderr)
        if len(errors) > 50:
            print(f"... {len(errors) - 50} more", file=sys.stderr)
        return 1
    print(f"OK {dest}: {len(expected)} files verified against {argv[1]}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
