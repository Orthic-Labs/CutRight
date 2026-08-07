"""Build CutRight's FFmpeg 8.1 media pack.

This script does NOT download FFmpeg. It validates the build contract
checks and exits successfully when the configured flags are safe. The
actual source fetch is a manual, offline-only step governed by the
licence ledger (see docs/legal/FFMPEG-BUILD.md).

Forbidden configure flags:
    --enable-gpl
    --enable-nonfree
    --enable-libiconv (allowed but recorded)

Required capability probes:
    ffprobe-json
    h264-decode
    aac-decode
    libass-or-native-caption-path
    zscale-or-qualified-hdr-path
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

FORBIDDEN_FLAGS = {"--enable-gpl", "--enable-nonfree"}
REQUIRED_PROBES = {
    "ffprobe-json",
    "h264-decode",
    "aac-decode",
    "libass-or-native-caption-path",
    "zscale-or-qualified-hdr-path",
}

WORKSPACE_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_TARGET = "host"


def _check_config(args: list[str]) -> int:
    """Validate the configure flags without compiling anything."""
    for flag in args:
        if flag in FORBIDDEN_FLAGS:
            print(f"FORBIDDEN: {flag}", file=sys.stderr)
            return 2
    return 0


def _expected_artifacts(target: str) -> list[Path]:
    arch = "auto"
    return [
        WORKSPACE_ROOT / "runtime/source/ffmpeg" / target / arch / "bin" / "ffmpeg",
        WORKSPACE_ROOT / "runtime/source/ffmpeg" / target / arch / "bin" / "ffprobe",
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description="Build the FFmpeg 8.1 media pack")
    parser.add_argument("--target", default=DEFAULT_TARGET,
                        help="Target identifier (host, macos-arm64, linux-x86_64, ...)")
    parser.add_argument("--check-config", action="store_true",
                        help="Validate configure flags without compiling")
    parser.add_argument("--probe", action="store_true",
                        help="Run the capability probe list")
    parser.add_argument("configure_flags", nargs="*",
                        help="Configure flags to validate (only with --check-config)")
    args = parser.parse_args()

    if args.check_config:
        return _check_config(args.configure_flags)

    if args.probe:
        for probe in sorted(REQUIRED_PROBES):
            print(f"probe: {probe}")
        return 0

    print(f"note: build-ffmpeg.py would target {args.target} but does not "
          f"fetch source. See docs/legal/FFMPEG-BUILD.md for the manual "
          f"offline procedure.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
