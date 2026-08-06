#!/usr/bin/env python3
"""verify_notices.py — verify THIRD_PARTY.yml notices in copied subtrees.

Usage: python3 tools/import-closure/verify_notices.py ROOT [ROOT...]
Every copied subtree root must carry a THIRD_PARTY.yml with the frozen
schema fields: schema_version, source_id, name, canonical_url, revision,
license, notice.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _common import walk_files  # noqa: E402

REQUIRED_KEYS = ("schema_version", "source_id", "name", "canonical_url", "revision", "license", "notice")
FORBIDDEN_REVISION = re.compile(r"^(main|master|latest|HEAD|develop|dev)$")


def parse_simple_yaml_keys(text: str) -> dict:
    keys = {}
    for line in text.splitlines():
        match = re.match(r"^([A-Za-z_][A-Za-z0-9_]*):\s*(.*)$", line)
        if match:
            keys[match.group(1)] = match.group(2).strip()
    return keys


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    errors = []
    checked = 0
    for root_arg in argv[1:]:
        root = Path(root_arg)
        if not root.is_dir():
            errors.append(f"missing root: {root}")
            continue
        notices = [p for p in walk_files(root) if p.name == "THIRD_PARTY.yml"]
        if not notices:
            errors.append(f"{root}: no THIRD_PARTY.yml found in copied subtree")
            continue
        for notice in notices:
            checked += 1
            keys = parse_simple_yaml_keys(notice.read_text(encoding="utf-8"))
            for key in REQUIRED_KEYS:
                if key not in keys or not keys[key] or keys[key] in {"|", ">"}:
                    if key == "notice":
                        continue  # block scalar body follows on later lines
                    errors.append(f"{notice}: missing or empty key {key}")
            revision = keys.get("revision", "")
            if FORBIDDEN_REVISION.match(revision):
                errors.append(f"{notice}: mutable revision {revision!r}")
    if errors:
        for err in errors[:50]:
            print(f"FAIL {err}", file=sys.stderr)
        return 1
    print(f"OK: {checked} THIRD_PARTY.yml notice(s) valid")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
