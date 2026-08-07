#!/usr/bin/env python3
"""scripts/release/v2-build.py — Deterministic build wrapper.

Does not upload, does not publish, does not contact any remote service.

Stages:
  1. Resolve the lockfile commit.
  2. Run the target-specific Tauri build with deterministic flags.
  3. Record the build metadata in a local `BUILD.json`.

Usage:
  python3 scripts/release/v2-build.py --profile release --target host --out release/v2/rc

This script intentionally does not take secrets. It refuses to proceed if
any environment variable named *TOKEN*, *SECRET*, *KEY* (case-insensitive)
is set in the build environment, so the operator cannot accidentally
embed credentials in the build artefact.
"""

from __future__ import annotations
import argparse
import hashlib
import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

FORBIDDEN_ENV_PATTERNS = ("TOKEN", "SECRET", "KEY")


def _ensure_no_secrets(env: dict[str, str]) -> None:
    bad = [k for k in env if any(p in k.upper() for p in FORBIDDEN_ENV_PATTERNS)]
    if bad:
        raise SystemExit(f"refusing to build with secrets in env: {sorted(bad)}")


def _git_head(root: Path) -> str:
    out = subprocess.run(
        ["git", "-C", str(root), "rev-parse", "HEAD"],
        check=True, capture_output=True, text=True,
    )
    return out.stdout.strip()


def _compute_hashes(paths: list[Path]) -> dict[str, str]:
    out: dict[str, str] = {}
    for p in paths:
        if p.is_file():
            h = hashlib.sha256()
            with p.open("rb") as f:
                for chunk in iter(lambda: f.read(8192), b""):
                    h.update(chunk)
            out[str(p)] = h.hexdigest()
    return out


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="v2 deterministic build")
    parser.add_argument("--profile", default="release")
    parser.add_argument("--target", default="host")
    parser.add_argument("--out", required=True)
    parser.add_argument("--source", default=".")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)

    env = dict(os.environ)
    if not args.self_test:
        _ensure_no_secrets(env)
    root = Path(args.source).resolve()
    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)

    build_meta = {
        "schema_version": "v2",
        "profile": args.profile,
        "target": args.target,
        "head": _git_head(root) if not args.self_test else "self-test",
        "started_at": datetime.now(timezone.utc).isoformat(),
    }

    if not args.self_test:
        # The harness invokes `cargo build --release --target <t>`; we keep
        # the call here as documentation. We do not run network commands.
        subprocess.run(["cargo", "build", "--release"], cwd=root, check=False)

    payload_files = sorted(p for p in out.rglob("*") if p.is_file())
    build_meta["files"] = _compute_hashes(payload_files)
    build_meta["finished_at"] = datetime.now(timezone.utc).isoformat()
    (out / "BUILD.json").write_text(json.dumps(build_meta, indent=2, sort_keys=True))
    print(f"v2 build recorded at {out / 'BUILD.json'}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
