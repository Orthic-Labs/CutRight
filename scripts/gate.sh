#!/usr/bin/env bash
#
# scripts/gate.sh — CutRight repository gate (hardening plan §7.2).
#
# This is the single authoritative local gate. CI (.github/workflows/ci.yml)
# is only an adapter around it; the repository contract is this script.
#
# It runs, in order and failing fast:
#   1. root cargo workspace ....... fmt --check, clippy -D warnings, test
#   2. Studio cargo workspace ..... fmt --check, clippy -D warnings, test
#      (Studio is intentionally a SEPARATE cargo workspace, gated by manifest
#       path so its Tauri dependency graph and lockfile stay isolated — §7.3)
#   3. Studio frontend ............ pnpm install, typecheck, test, build
#   4. license/asset resolution ... scripts/resolve-license.sh
#   5. (optional, --with-qa) ...... headless browser QA lane
#
# Usage:
#   bash scripts/gate.sh             # fast default gate
#   bash scripts/gate.sh --with-qa   # also run the headless QA lane (needs a browser)
#   bash scripts/gate.sh --help
#
# Portable across macOS and Linux. The repo root is resolved from this
# script's own location, so it can be invoked from any working directory.

set -euo pipefail

# --- resolve repo root from script location (never $PWD) ---------------------
SCRIPT_PATH="${BASH_SOURCE[0]}"
while [ -h "$SCRIPT_PATH" ]; do
  DIR="$(cd -P "$(dirname "$SCRIPT_PATH")" && pwd)"
  SCRIPT_PATH="$(readlink "$SCRIPT_PATH")"
  [ "${SCRIPT_PATH#/}" = "$SCRIPT_PATH" ] && SCRIPT_PATH="$DIR/$SCRIPT_PATH"
done
SCRIPT_DIR="$(cd -P "$(dirname "$SCRIPT_PATH")" && pwd)"
ROOT="$(cd -P "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

# --- colors (harmless escape codes in CI logs) -------------------------------
if [ -t 1 ]; then
  C_BLUE=$'\033[1;34m'; C_GREEN=$'\033[1;32m'; C_RED=$'\033[1;31m'; C_RESET=$'\033[0m'
else
  C_BLUE=''; C_GREEN=''; C_RED=''; C_RESET=''
fi

usage() {
  cat <<'EOF'
usage: bash scripts/gate.sh [--with-qa]

Runs the CutRight gate in order, failing fast:
  root cargo fmt/clippy/test -> Studio cargo fmt/clippy/test ->
  frontend pnpm install/typecheck/test/build -> license/asset resolution.

Options:
  --with-qa   Also run the headless browser QA lane (qa:browser ->
              qa:functional -> qa:browser:stop). Needs a local browser.
  -h, --help  Show this help.
EOF
}

WITH_QA=0
for arg in "$@"; do
  case "$arg" in
    --with-qa) WITH_QA=1 ;;
    -h|--help) usage; exit 0 ;;
    *)
      echo "gate.sh: unknown argument: $arg" >&2
      usage >&2
      exit 2
      ;;
  esac
done

# --- step runner + summary ---------------------------------------------------
CURRENT_STEP="<startup>"
QA_STARTED=0

log() { printf '\n%s==> %s%s\n' "$C_BLUE" "$*" "$C_RESET"; }
ok()  { printf '%s[PASS]%s %s\n' "$C_GREEN" "$C_RESET" "$*"; }

run() {
  CURRENT_STEP="$1"; shift
  log "$CURRENT_STEP"
  "$@"
  ok "$CURRENT_STEP"
}

on_exit() {
  local code=$?
  # Always tear down the QA dev server if we started it (happy path stops it
  # explicitly; this guards the failure path so no orphaned vite survives).
  if [ "$QA_STARTED" -eq 1 ]; then
    pnpm --dir apps/studio qa:browser:stop || true
    QA_STARTED=0
  fi
  echo
  if [ "$code" -ne 0 ]; then
    printf '%s==================================================%s\n' "$C_RED" "$C_RESET"
    printf '%sGATE FAIL%s (exit %s) during step: %s\n' "$C_RED" "$C_RESET" "$code" "$CURRENT_STEP"
    printf '%s==================================================%s\n' "$C_RED" "$C_RESET"
  else
    printf '%s==================================================%s\n' "$C_GREEN" "$C_RESET"
    if [ "$WITH_QA" -eq 1 ]; then
      printf '%sGATE PASS%s (with QA)\n' "$C_GREEN" "$C_RESET"
    else
      printf '%sGATE PASS%s\n' "$C_GREEN" "$C_RESET"
    fi
    printf '%s==================================================%s\n' "$C_GREEN" "$C_RESET"
  fi
}
trap on_exit EXIT

echo "gate.sh: repo root = $ROOT"
echo "gate.sh: mode = $([ "$WITH_QA" -eq 1 ] && echo 'default + QA' || echo 'default')"

# --- 1. root cargo workspace -------------------------------------------------
run "root: cargo fmt --all -- --check" \
  cargo fmt --all -- --check
run "root: cargo clippy --workspace --all-targets --locked -- -D warnings" \
  cargo clippy --workspace --all-targets --locked -- -D warnings
run "root: cargo test --workspace --locked" \
  cargo test --workspace --locked

# --- 2. Studio cargo workspace (separate lockfile, gated by manifest path) ---
STUDIO_MANIFEST="apps/studio/src-tauri/Cargo.toml"
run "studio: cargo fmt --manifest-path $STUDIO_MANIFEST -- --check" \
  cargo fmt --manifest-path "$STUDIO_MANIFEST" -- --check
run "studio: cargo clippy --manifest-path $STUDIO_MANIFEST --all-targets --locked -- -D warnings" \
  cargo clippy --manifest-path "$STUDIO_MANIFEST" --all-targets --locked -- -D warnings
run "studio: cargo test --manifest-path $STUDIO_MANIFEST --locked" \
  cargo test --manifest-path "$STUDIO_MANIFEST" --locked

# --- 3. Studio frontend ------------------------------------------------------
# Prefer corepack so the packageManager pin in apps/studio/package.json is
# honoured; fall back to a system pnpm; otherwise fail with a clear message.
if ! command -v pnpm >/dev/null 2>&1; then
  if command -v corepack >/dev/null 2>&1; then
    log "frontend: enabling corepack (honours packageManager pin)"
    corepack enable
  else
    echo "gate.sh: pnpm not found and corepack is unavailable." >&2
    echo "gate.sh: install pnpm matching the packageManager pin in apps/studio/package.json." >&2
    exit 1
  fi
fi
run "frontend: pnpm --dir apps/studio install --frozen-lockfile" \
  pnpm --dir apps/studio install --frozen-lockfile
run "frontend: pnpm --dir apps/studio typecheck" \
  pnpm --dir apps/studio typecheck
run "frontend: pnpm --dir apps/studio test" \
  pnpm --dir apps/studio test
run "frontend: pnpm --dir apps/studio build" \
  pnpm --dir apps/studio build

# --- 4. license/asset resolution --------------------------------------------
run "license/asset resolution (scripts/resolve-license.sh)" \
  bash scripts/resolve-license.sh

# --- 5. optional headless QA lane -------------------------------------------
if [ "$WITH_QA" -eq 1 ]; then
  log "QA: starting headless browser dev server (qa:browser)"
  pnpm --dir apps/studio qa:browser
  QA_STARTED=1
  run "QA: functional (qa:functional)" \
    pnpm --dir apps/studio qa:functional
  log "QA: stopping dev server (qa:browser:stop)"
  pnpm --dir apps/studio qa:browser:stop
  QA_STARTED=0
fi

exit 0
