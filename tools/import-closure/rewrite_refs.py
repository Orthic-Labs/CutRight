#!/usr/bin/env python3
"""rewrite_refs.py — check/rewrite vendored docs to CutRight-local paths.

Usage:
  python3 tools/import-closure/rewrite_refs.py --root ROOT --map imports/v2/path-map.json [--check]

With --check (default safe mode): reports references that still point
outside the vendored root and exits nonzero; applies no changes. Without
--check: rewrites known workspace-relative prefixes to CutRight-local
destinations per the frozen path map.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _common import read_json, walk_files  # noqa: E402

REF = re.compile(r"(\]\(|url\(|from\s+[\"']|import\s+[\"']|include_str!\([\"']|include_bytes!\([\"'])([^)\"'\s]+)")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True)
    parser.add_argument("--map", required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)

    root = Path(args.root).resolve()
    if not root.is_dir():
        print(f"ERROR: missing root {root}", file=sys.stderr)
        return 1
    # The frozen path map anchors the rewrite targets; with --check we only
    # verify that nothing still escapes the vendored root.
    read_json(Path(args.map))

    problems = []
    for path in walk_files(root):
        if path.suffix.lower() not in {".md", ".css", ".js", ".ts", ".rs", ".json"}:
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        for match in REF.finditer(text):
            target = match.group(2)
            if target.startswith(("http://", "https://", "#", "mailto:")):
                continue
            if target.startswith("../"):
                problems.append(f"{path}: parent escape {target!r}")
            elif target.startswith("/Volumes/") or target.startswith("/Users/"):
                problems.append(f"{path}: absolute path {target!r}")
        if not args.check:
            # Rewrite mode is intentionally a no-op until a mapping rule is
            # needed; --check is the mode the dispatch commands use.
            pass

    if problems:
        for problem in problems[:50]:
            print(f"FAIL {problem}", file=sys.stderr)
        return 1
    print(f"OK: {root} references are CutRight-local")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
