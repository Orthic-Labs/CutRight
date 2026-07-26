#!/usr/bin/env bash
set -euo pipefail
if [ -f .cache/qa-browser/pid.txt ]; then kill "$(cat .cache/qa-browser/pid.txt)" 2>/dev/null || true; fi
if [ -f .cache/qa-browser/pid.txt ]; then
  pid="$(cat .cache/qa-browser/pid.txt)"
  for _ in 1 2 3 4 5; do
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.2
  done
  rm -f .cache/qa-browser/pid.txt
fi
