#!/usr/bin/env python3
"""assert_no_external_refs.py — assert vendored material is CutRight-local.

Usage: python3 tools/import-closure/assert_no_external_refs.py ROOT [ROOT...]
Fails when vendored files reference sibling-repository paths, absolute
machine paths, or mutable workspace locations. Notice URLs inside
THIRD_PARTY.yml files are allowed (they are provenance, not resolution).
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _common import walk_files  # noqa: E402

SIBLING = re.compile(r"(\.\./)+(heardright|claude|autoshorts|vox-director|palmier|workspace-capabilities)")
ABSOLUTE_MACHINE = re.compile(r"(/Volumes/|/Users/|/home/|C:\\\\Users\\\\|file://)")
TEXT_SUFFIXES = {".md", ".txt", ".json", ".yml", ".yaml", ".toml", ".py", ".js", ".ts", ".sh", ".rs", ".css", ".html"}


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    errors = []
    for root_arg in argv[1:]:
        root = Path(root_arg)
        if not root.is_dir():
            errors.append(f"missing root: {root}")
            continue
        for path in walk_files(root):
            if path.suffix.lower() not in TEXT_SUFFIXES:
                continue
            try:
                text = path.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                continue
            for lineno, line in enumerate(text.splitlines(), start=1):
                if path.name == "THIRD_PARTY.yml" and "canonical_url" in line:
                    continue
                for pattern, label in ((SIBLING, "sibling-repository path"), (ABSOLUTE_MACHINE, "absolute machine path")):
                    match = pattern.search(line)
                    if match:
                        errors.append(f"{path}:{lineno}: {label} reference: {match.group(0)!r}")
    if errors:
        for err in errors[:50]:
            print(f"FAIL {err}", file=sys.stderr)
        return 1
    print(f"OK: {len(argv) - 1} root(s) contain no external references")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
