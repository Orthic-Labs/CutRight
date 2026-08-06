#!/usr/bin/env python3
"""verify_exclusions.py — assert excluded paths did not enter the import.

Usage: python3 tools/import-closure/verify_exclusions.py GRAPH.json EXCLUSIONS.json
Exclusions format: {"schema_version": 1, "selection_id": "...", "excluded_paths": ["glob", ...]}
Exit 0 when no graph entry matches any exclusion glob.
"""

from __future__ import annotations

import fnmatch
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _common import read_json  # noqa: E402


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    graph = read_json(Path(argv[1]))
    exclusions = read_json(Path(argv[2]))
    patterns = exclusions.get("excluded_paths", [])
    hits = []
    for entry in graph.get("files", []):
        rel = entry["path"]
        for pattern in patterns:
            if fnmatch.fnmatch(rel, pattern) or fnmatch.fnmatch("/" + rel, "/" + pattern):
                hits.append(f"{rel} matches exclusion {pattern}")
    if hits:
        for hit in hits[:50]:
            print(f"FAIL {hit}", file=sys.stderr)
        return 1
    print(f"OK: {graph.get('file_count', 0)} files honour {len(patterns)} exclusion pattern(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
