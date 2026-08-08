#!/usr/bin/env python3
"""Reject oversized binaries before they enter git history.

Added 2026-08-09 after CR-F-B5-004 downloaded a 2.5 GB qwen3-4B gguf and
committed it twice, taking .git to 6.9 GB and crashing the machine. No task
ceiling in the implementation book caught it, because ceilings count files and
changed lines — never bytes on disk.

Exit 1 if any tracked or staged file exceeds MAX_BYTES.
"""
import subprocess
import sys
from pathlib import Path

MAX_BYTES = 100 * 1024 * 1024  # 100 MB
REPO = Path(__file__).resolve().parents[2]


def tracked_and_staged() -> set[str]:
    out: set[str] = set()
    for args in (["git", "ls-files"], ["git", "diff", "--cached", "--name-only"]):
        res = subprocess.run(args, cwd=REPO, capture_output=True, text=True)
        out.update(p for p in res.stdout.split("\n") if p.strip())
    return out


def main() -> int:
    offenders = []
    for rel in sorted(tracked_and_staged()):
        path = REPO / rel
        if not path.is_file():
            continue
        size = path.stat().st_size
        if size > MAX_BYTES:
            offenders.append((rel, size))

    if not offenders:
        print(f"no-large-binaries: PASS (limit {MAX_BYTES // 1024 // 1024} MB)")
        return 0

    print(f"no-large-binaries: FAIL — {len(offenders)} file(s) over "
          f"{MAX_BYTES // 1024 // 1024} MB", file=sys.stderr)
    for rel, size in offenders:
        print(f"  {size / 1024 / 1024:>9.1f} MB  {rel}", file=sys.stderr)
    print("\nModel and runtime payloads belong in a signed pack fetched at install "
          "time, never in git. If this file is genuinely required, raise it with "
          "Adrian before committing.", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
