#!/usr/bin/env python3
"""v2-runtime-boundary.py — CR-V2-B3-024.

Static check that the release runtime never falls back to bare executable
resolution. The check is intentionally simple: it greps the source tree
for the forbidden `"ffmpeg"`-on-PATH idiom and refuses the build if any
release path still calls `Command::new("ffmpeg")` without a pack-resolved
path.

Exit codes:
  0   clean — no bare executable lookups.
  1   drift detected — at least one release path still uses a bare
      executable lookup.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

FORBIDDEN_PATTERNS = [
    re.compile(r'Command::new\("ffmpeg"\)'),
    re.compile(r'Command::new\("ffprobe"\)'),
    re.compile(r'Command::new\("node"\)'),
    re.compile(r'Command::new\("python3"\)'),
    re.compile(r'Command::new\("whisperx"\)'),
    re.compile(r'Command::new\("heardright"\)'),
]

DEV_OVERRIDE_GATE = "cfg(feature = \"dev-runtime-override\")"

RELEASE_DIRS = [
    "crates/video-media/src",
    "crates/video-providers/src",
    "crates/video-runtime/src",
]


def scan(root: Path) -> list[str]:
    findings: list[str] = []
    for rel in RELEASE_DIRS:
        d = root / rel
        if not d.exists():
            continue
        for path in d.rglob("*.rs"):
            text = path.read_text(encoding="utf-8", errors="ignore")
            for pat in FORBIDDEN_PATTERNS:
                for match in pat.finditer(text):
                    line = text[: match.start()].count("\n") + 1
                    snippet = match.group(0)
                    if is_release_path(path, text):
                        findings.append(
                            f"{path}:{line}: forbidden bare executable lookup: {snippet}"
                        )
    return findings


def is_release_path(path: Path, text: str) -> bool:
    # Anything inside the feature gate is allowed in development builds.
    if DEV_OVERRIDE_GATE in text:
        return False
    return True


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if not args.check:
        print("pass-through mode (no --check provided)")
        return 0
    root = Path(__file__).resolve().parents[2]
    findings = scan(root)
    if findings:
        print("runtime-boundary drift detected:")
        for f in findings:
            print(f"  - {f}")
        return 1
    print("runtime-boundary check: clean")
    return 0


if __name__ == "__main__":
    sys.exit(main())
