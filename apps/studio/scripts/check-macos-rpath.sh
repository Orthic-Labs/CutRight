#!/usr/bin/env bash
set -euo pipefail

binary="${1:?usage: check-macos-rpath.sh <universal-binary>}"
[[ -f "$binary" ]] || { echo "missing binary: $binary" >&2; exit 1; }

for arch in arm64 x86_64; do
  if ! otool -arch "$arch" -l "$binary" | awk '
    /cmd LC_RPATH/ { in_rpath = 1; next }
    in_rpath && /path \/usr\/lib\/swift/ { found = 1 }
    in_rpath && /^Load command/ { in_rpath = 0 }
    END { exit found ? 0 : 1 }
  '; then
    echo "missing /usr/lib/swift LC_RPATH for $arch: $binary" >&2
    exit 1
  fi
done
echo "macOS LC_RPATH verified for arm64,x86_64: $binary"
