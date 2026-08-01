#!/usr/bin/env bash
set -euo pipefail
mkdir -p .cache/qa-browser
nohup env VITE_CUTRIGHT_QA=1 pnpm exec vite --host 127.0.0.1 --port 4173 \
  </dev/null >.cache/qa-browser/server.log 2>&1 &
echo $! >.cache/qa-browser/pid.txt
echo 'http://127.0.0.1:4173/?qa=1' >.cache/qa-browser/url.txt
