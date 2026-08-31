#!/usr/bin/env bash
#
# scripts/gate.sh — CutRight repository gate (hardening plan §7.2).
#
# This is the single authoritative gate. RightKit-managed public CI calls this
# exact script; run it locally before every commit.
#
# Test policy (changed 2026-08-09, Adrian's direction)
# ----------------------------------------------------
# This gate no longer runs `cargo test`. Both suites — root workspace and
# Studio — were removed. They turned every gate invocation into a full
# regression run, and with many tasks calling the gate the machine spent its
# time recompiling and re-running the same suites.
#
# Rust tests now run per task, scoped to the crate that task owns:
#   cargo test -p <exact-crate>
#   cargo test --manifest-path apps/studio/src-tauri/Cargo.toml -p <exact-crate>
#
# Consequence, stated plainly: nothing runs the full Rust suite automatically
# any more. Whole-workspace regressions will only be caught if someone runs
# `cargo test --workspace` deliberately. The frontend suites below still run.
#
# It runs, in order and failing fast:
#   1. root cargo workspace ....... fmt --check, clippy -D warnings
#   2. Studio cargo workspace ..... fmt --check, clippy -D warnings
#      (Studio is intentionally a SEPARATE cargo workspace, gated by manifest
#       path so its Tauri dependency graph and lockfile stay isolated — §7.3)
#   3. Studio frontend ............ pnpm install, typecheck, test, build
#   4. Effects frontend ........... pnpm install, typecheck, test, build
#      (apps/effects: the Remotion render backend for EffectRenderer::Remotion
#       — REV2 §15.3 Phase 5; a separate pnpm project from apps/studio, same
#       pattern as the Studio cargo workspace being separate from root)
#   5. license/asset resolution ... scripts/resolve-license.sh
#   6. (optional, --with-qa) ...... headless browser QA lane
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
STUDIO_CLIPPY_PLACEHOLDERS=()

log() { printf '\n%s==> %s%s\n' "$C_BLUE" "$*" "$C_RESET"; }
ok()  { printf '%s[PASS]%s %s\n' "$C_GREEN" "$C_RESET" "$*"; }

# `have` + `skip_note` exist so an absent scanner reads as UNPROVEN rather
# than silently passing. A check whose tool never ran must never look clean.
have() { command -v "$1" >/dev/null 2>&1; }
skip_note() { printf '%s[SKIP]%s %s\n' "$C_BLUE" "$C_RESET" "$*"; }

run() {
  CURRENT_STEP="$1"; shift
  log "$CURRENT_STEP"
  "$@"
  ok "$CURRENT_STEP"
}

prepare_studio_clippy_sidecars() {
  local triple name path
  case "$(uname -s)/$(uname -m)" in
    Darwin/arm64) triple="aarch64-apple-darwin" ;;
    Darwin/x86_64) triple="x86_64-apple-darwin" ;;
    *) return ;;
  esac
  mkdir -p apps/studio/src-tauri/bin
  for name in videoctl cutright-mcp cutrightd; do
    path="apps/studio/src-tauri/bin/${name}-${triple}"
    if [ ! -e "$path" ]; then
      : > "$path"
      chmod +x "$path"
      STUDIO_CLIPPY_PLACEHOLDERS+=("$path")
    fi
  done
}

cleanup_studio_clippy_sidecars() {
  local path
  for path in "${STUDIO_CLIPPY_PLACEHOLDERS[@]}"; do
    rm -f "$path"
  done
  STUDIO_CLIPPY_PLACEHOLDERS=()
}

on_exit() {
  local code=$?
  cleanup_studio_clippy_sidecars
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

# --- 0. enforced crate dependency DAG ---------------------------------------
run "crate DAG: python3 scripts/check-crate-dag.py" \
  python3 scripts/check-crate-dag.py
run "repository shape: scripts/gates/v2-repository-shape.sh" \
  bash scripts/gates/v2-repository-shape.sh
run "standalone source audit: scripts/gates/v2-standalone-source-audit.py" \
  python3 scripts/gates/v2-standalone-source-audit.py --root .

# --- 1. root cargo workspace -------------------------------------------------
run "root: cargo fmt --all -- --check" \
  cargo fmt --all -- --check
run "root: cargo clippy --workspace --all-targets --locked -- -D warnings" \
  cargo clippy --workspace --all-targets --locked -- -D warnings
# `cargo test --workspace --locked` was removed 2026-08-09 by Adrian's direction.
# It made every gate invocation a full regression run; with many tasks calling
# the gate, the machine was compiling and running both Cargo suites repeatedly.
# Tests now run per task, scoped to the crate that task owns:
#   cargo test -p <exact-crate>
# See "Test policy" at the head of this file.

# --- 2. Studio cargo workspace (separate lockfile, gated by manifest path) ---
STUDIO_MANIFEST="apps/studio/src-tauri/Cargo.toml"
run "studio: cargo fmt --manifest-path $STUDIO_MANIFEST -- --check" \
  cargo fmt --manifest-path "$STUDIO_MANIFEST" -- --check
prepare_studio_clippy_sidecars
run "studio: cargo clippy --manifest-path $STUDIO_MANIFEST --all-targets --locked -- -D warnings" \
  cargo clippy --manifest-path "$STUDIO_MANIFEST" --all-targets --locked -- -D warnings
cleanup_studio_clippy_sidecars
# `cargo test --manifest-path "$STUDIO_MANIFEST" --locked` removed with the root
# suite on 2026-08-09. Studio Rust tests run per task:
#   cargo test --manifest-path apps/studio/src-tauri/Cargo.toml -p <exact-crate>

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

# --- 4. Effects frontend (apps/effects — Remotion render backend) -----------
run "effects: pnpm --dir apps/effects install --frozen-lockfile" \
  pnpm --dir apps/effects install --frozen-lockfile
run "effects: pnpm --dir apps/effects typecheck" \
  pnpm --dir apps/effects typecheck
run "effects: pnpm --dir apps/effects test" \
  pnpm --dir apps/effects test
run "effects: pnpm --dir apps/effects build" \
  pnpm --dir apps/effects build

# --- 5. license/asset resolution --------------------------------------------
run "license/asset resolution (scripts/resolve-license.sh)" \
  bash scripts/resolve-license.sh
run "fixture seals: native" \
  python3 scripts/gate-fixtures.py fixtures/macos-native/MANIFEST.json
run "fixture seals: Cutaway/Finish" \
  python3 scripts/gate-fixtures.py fixtures/cutaway-finish/MANIFEST.json

# --- 5b. supply chain + dead code (skipped when the tool is absent) ----------
# These were UNPROVEN for the whole hardening campaign because the tools were
# not installed. They are wired in here so they cannot silently regress. Each
# is skipped with a loud note rather than failing the gate when its tool is
# missing — an absent scanner is honestly "not run", never "clean".
if have cargo-deny; then
  run "supply chain: cargo deny check (deny.toml)" \
    cargo deny check
else
  skip_note "cargo-deny absent — advisories/licenses/bans/sources UNPROVEN (cargo install cargo-deny)"
fi

if have cargo-machete; then
  # Scan buildable first-party packages only. Dormant manifests and vendored
  # upstream workspaces have independent merge/build gates.
  run "unused deps: cargo machete" \
    cargo machete \
      crates/video-core \
      crates/video-actions \
      crates/video-benchmarks \
      crates/video-capabilities \
      crates/video-media \
      crates/video-providers \
      crates/video-project \
      crates/video-jobs \
      crates/video-runtime \
      crates/video-security \
      crates/video-recovery \
      crates/video-cli \
      crates/video-services \
      crates/video-state \
      crates/video-sessions \
      crates/video-feedback \
      crates/video-agent \
      crates/video-editorial \
      crates/video-protocol \
      crates/video-daemon \
      crates/video-driver-host \
      apps/studio/src-tauri
else
  skip_note "cargo-machete absent — unused-dependency check UNPROVEN (cargo install cargo-machete)"
fi

# --- 6. optional headless QA lane -------------------------------------------
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
