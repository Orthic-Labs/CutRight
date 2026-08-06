#!/usr/bin/env python3
"""validate_clean_room.py — validate a clean-room observation record.

Usage: python3 tools/import-closure/validate_clean_room.py RECORD.json
The record must match the frozen clean-room observation schema
(docs/dispatch/v2/book-1/interface-freeze.md) and the ledger's
clean_room block requirements.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _common import read_json  # noqa: E402

REQUIRED = (
    "schema_version",
    "source_id",
    "observed_at_revision",
    "observation_date",
    "observed_behavior",
    "implementer_separation",
    "no_copy_attestation",
)
FORBIDDEN_REVISION = re.compile(r"^(main|master|latest|HEAD|develop|dev)$")
HEX40 = re.compile(r"^[0-9a-f]{40}$")


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    doc = read_json(Path(argv[1]))
    errors = []
    for key in REQUIRED:
        value = doc.get(key)
        if value is None or (isinstance(value, str) and not value.strip()):
            errors.append(f"missing required field: {key}")
    if doc.get("schema_version") != 1:
        errors.append("schema_version must be 1")
    revision = doc.get("observed_at_revision", "")
    if isinstance(revision, str) and revision:
        if FORBIDDEN_REVISION.match(revision):
            errors.append(f"mutable revision: {revision!r}")
        elif not HEX40.match(revision):
            errors.append(f"observed_at_revision must be a 40-hex commit: {revision!r}")
    date = doc.get("observation_date", "")
    if isinstance(date, str) and date and not re.match(r"^\d{4}-\d{2}-\d{2}$", date):
        errors.append(f"observation_date must be ISO: {date!r}")
    notes = doc.get("observation_notes", [])
    if notes:
        for note in notes:
            if not isinstance(note, str) or not note.strip():
                errors.append("observation_notes entries must be non-empty strings")
    if errors:
        for err in errors:
            print(f"FAIL {err}", file=sys.stderr)
        return 1
    print(f"OK {argv[1]}: clean-room record for {doc.get('source_id')} is complete")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
