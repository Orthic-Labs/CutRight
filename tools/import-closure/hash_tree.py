#!/usr/bin/env python3
"""hash_tree.py — emit the deterministic hash manifest for a tree.

Usage: python3 tools/import-closure/hash_tree.py DIR [--out FILE]
Prints JSON to stdout unless --out is given.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _common import fail, hash_tree  # noqa: E402


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    target = Path(argv[1])
    if not target.is_dir():
        fail(f"not a directory: {target}")
    doc = hash_tree(target)
    text = json.dumps(doc, indent=2) + "\n"
    if "--out" in argv:
        out = Path(argv[argv.index("--out") + 1])
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(text, encoding="utf-8")
        print(f"wrote {out}")
    else:
        sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
