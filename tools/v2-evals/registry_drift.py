#!/usr/bin/env python3
"""Drift detector driver for the canonical capability registry (CR-V2-B2-016).

Invokes the Rust crate `video-capabilities` to:
- load `docs/dispatch/v2/source/capability-registry.json`
- scan the in-tree source set for `capability_id = "..."` literals
- compare them against the canonical set
- regenerate `docs/dispatch/v2/book-2/capabilities.md`

Exit codes:
  0 — clean (no unknown references, all generated artifacts in sync)
  1 — drift detected
  2 — loader / IO failure
  3 — docs render failure
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DRIFT_BIN = "video-capabilities-drift"
DOCS_BIN = "video-capabilities-docs"


def run(cmd: list[str], **kwargs) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=REPO_ROOT, capture_output=True, text=True, **kwargs)


def detect_drift() -> dict:
    proc = run(["cargo", "run", "--locked", "-p", "video-capabilities", "--bin", DRIFT_BIN])
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr)
        raise SystemExit(2)
    return json.loads(proc.stdout)


def render_docs() -> str:
    proc = run(["cargo", "run", "--locked", "-p", "video-capabilities", "--bin", DOCS_BIN])
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr)
        raise SystemExit(3)
    return proc.stdout


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--report",
        action="store_true",
        help="Print the JSON drift report to stdout (default behaviour).",
    )
    parser.add_argument(
        "--render-docs",
        action="store_true",
        help="Regenerate docs/dispatch/v2/book-2/capabilities.md from the registry.",
    )
    parser.add_argument(
        "--allow-unreferenced",
        action="store_true",
        help="Don't fail on capabilities declared but never referenced.",
    )
    args = parser.parse_args(argv)

    report = detect_drift()
    print(json.dumps(report, indent=2, sort_keys=True))

    if args.render_docs:
        render_docs()

    if report.get("unknown_references"):
        return 1
    if report.get("generated_artifacts_drift"):
        return 1
    if not args.allow_unreferenced and report.get("unreferenced_capabilities"):
        # Soft failure — return non-zero so the gate fails closed but
        # callers that explicitly opt out (e.g. while the catalogue is
        # still being wired up) can still pass.
        return 4
    return 0


if __name__ == "__main__":
    sys.exit(main())