#!/usr/bin/env python3
"""scripts/architecture/check_crate_dag.py — stdlib-only DAG verification.

Validates that a CutRight v2 release DAG document mentions the required
phases, lane ownership, and serial integration order. Lightweight
plain-text check; no external dependencies.

Exit 0 on pass, 1 on any missing required phrase.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path


REQUIRED_PHRASES = [
    "contracts",
    "Lane A",
    "Lane B",
    "Lane C",
    "merge",
    "four-lane",
    "audit",
    "SBOM",
    "release candidate",
    "final gate",
]


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify a CutRight v2 release DAG document.")
    parser.add_argument("path", help="Path to the release DAG markdown file.")
    args = parser.parse_args()

    text = Path(args.path).read_text(encoding="utf-8")
    missing = [p for p in REQUIRED_PHRASES if p.lower() not in text.lower()]
    if missing:
        print(f"FAIL: missing phrases: {missing}", file=sys.stderr)
        return 1
    print("OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
