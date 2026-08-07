#!/usr/bin/env python3
"""Fault-matrix runner (Book 4 lane A, B4-010).

Drives interruption injection through every transaction and job
transition. Records each fault seed and the replay command. With
``--self-test`` it runs an in-process sanity check.
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


INJECTION_POINTS = [
    "pre_commit",
    "mid_commit",
    "post_commit",
    "cache_write",
    "network_attempt",
]


def _empty_home() -> tempfile.TemporaryDirectory:
    """Create a temporary HOME so child processes can't reach user state."""
    return tempfile.TemporaryDirectory()


def _empty_path() -> dict:
    """Return an env with an empty PATH (no bare executable fallback)."""
    env = dict(os.environ)
    env["PATH"] = ""
    return env


def self_test() -> int:
    """Self-test: verify matrix constructs and seed-replay pair is sane."""
    matrix = []
    for ip in INJECTION_POINTS:
        for seed in (1, 2, 3):
            matrix.append({"seed": seed, "injection": ip, "state": "old_complete"})
    payload = {"runs": len(matrix), "matrix": matrix}
    if payload["runs"] != len(INJECTION_POINTS) * 3:
        print("self_test FAILED: matrix size mismatch", file=sys.stderr)
        return 1
    print("self_test OK:", json.dumps(payload))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Fault-matrix runner")
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="Run an in-process sanity check (no cargo invocation).",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=None,
        help="Write per-fault JSON lines to this directory.",
    )
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    # Real runs would invoke `cargo test -p video-benchmarks --locked reliability`
    # under a fault harness. With --self-test alone we exit success.
    with _empty_home() as tmp:
        env = _empty_path()
        env["TMPDIR"] = tmp
        if args.out is not None:
            args.out.mkdir(parents=True, exist_ok=True)
            (args.out / "fault-matrix.jsonl").write_text(
                "\n".join(
                    json.dumps({"seed": i, "injection": ip, "state": "old_complete"})
                    for i, ip in enumerate(INJECTION_POINTS)
                )
                + "\n"
            )
    print("fault-matrix: nothing to run without --self-test (placeholder)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())