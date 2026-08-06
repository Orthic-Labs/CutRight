#!/usr/bin/env bash
# scripts/gates/v2-repository-shape.sh — CutRight v2 repository shape guard.
#
# Fails when hosted CI, git submodules, skill symlinks, sibling-repository
# paths, or release environment overrides appear in the tree. Part of the
# standalone boundary frozen by CR-V2-B1-005.
#
# Usage:
#   bash scripts/gates/v2-repository-shape.sh [ROOT]
#   bash scripts/gates/v2-repository-shape.sh --self-test
#
# Exit 0 when the tree passes, 1 on any violation, 2 on self-test failure.

set -u

SELF_TEST=0
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
for arg in "$@"; do
  case "$arg" in
    --self-test) SELF_TEST=1 ;;
    *) ROOT="$arg" ;;
  esac
done

violations=0

fail() {
  echo "VIOLATION: $1" >&2
  violations=$((violations + 1))
}

check_shape() {
  local root="$1"
  violations=0

  # 1. No hosted CI of any flavour (a workflow file, not an empty leftover
  # directory, is what would run CI).
  if [ -d "$root/.github/workflows" ] && [ -n "$(ls -A "$root/.github/workflows" 2>/dev/null)" ]; then
    fail "hosted CI is forbidden, found workflow files under .github/workflows/"
  fi
  for ci in .circleci .gitlab-ci.yml; do
    if [ -e "$root/$ci" ]; then
      fail "hosted CI is forbidden, found: $ci"
    fi
  done

  # 2. No git submodules.
  if [ -e "$root/.gitmodules" ]; then
    fail "git submodules are forbidden, found: .gitmodules"
  fi

  # 3. No symlinks in vendored capability or vendor trees.
  for tree in skills vendor; do
    if [ -d "$root/$tree" ]; then
      while IFS= read -r -d '' link; do
        fail "symlink is forbidden under $tree/: ${link#"$root"/}"
      done < <(find "$root/$tree" -type l -print0 2>/dev/null)
    fi
  done

  # 4. No sibling-repository path references in release code.
  if command -v rg >/dev/null 2>&1; then
    local sibling_hits
    sibling_hits=$(rg -n '\.\./(heardright|claude|autoshorts|vox-director|palmier)' \
      --glob '!target/' --glob '!node_modules/' --glob '!imports/' \
      --glob '!docs/' --glob '!scripts/' --glob '!tools/' \
      "$root" 2>/dev/null || true)
    if [ -n "$sibling_hits" ]; then
      fail "sibling-repository path reference in release code:"
      echo "$sibling_hits" >&2
    fi
  fi

  # 5. No release environment overrides in release code.
  if command -v rg >/dev/null 2>&1; then
    local override_hits
    override_hits=$(rg -n 'RELEASE_OVERRIDE|CRV2_RELEASE_OVERRIDE|CUTRIGHT_RELEASE_ENV' \
      --glob '*.rs' --glob '*.ts' --glob '*.tsx' --glob '*.toml' --glob '*.json' \
      --glob '!target/' --glob '!node_modules/' --glob '!imports/' \
      --glob '!scripts/' --glob '!tools/' \
      "$root" 2>/dev/null || true)
    if [ -n "$override_hits" ]; then
      fail "release environment override in release code:"
      echo "$override_hits" >&2
    fi
  fi

  return "$violations"
}

self_test() {
  local tmp failures=0
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT

  run_case() {
    local name="$1"
    local case_dir="$tmp/$name"
    mkdir -p "$case_dir"
    if check_shape "$case_dir" >/dev/null 2>&1; then
      echo "SELF-TEST FAIL: $name did not trip the guard" >&2
      failures=$((failures + 1))
    else
      echo "self-test ok: $name correctly rejected"
    fi
  }

  mkdir -p "$tmp/clean"
  if ! check_shape "$tmp/clean" >/dev/null 2>&1; then
    echo "SELF-TEST FAIL: clean tree should pass" >&2
    failures=$((failures + 1))
  else
    echo "self-test ok: clean tree passes"
  fi

  mkdir -p "$tmp/ci/.github/workflows"
  echo "on: push" > "$tmp/ci/.github/workflows/x.yml"
  run_case ci

  mkdir -p "$tmp/submodules"
  printf '[submodule "x"]\npath = x\n' > "$tmp/submodules/.gitmodules"
  run_case submodules

  mkdir -p "$tmp/symlink/skills"
  echo "real" > "$tmp/symlink/skills/real.md"
  ln -s real.md "$tmp/symlink/skills/link.md"
  run_case symlink

  mkdir -p "$tmp/sibling/crates/x/src"
  echo 'let p = "../heardright/engine";' > "$tmp/sibling/crates/x/src/lib.rs"
  run_case sibling

  mkdir -p "$tmp/override/crates/x/src"
  echo 'let o = env!("CUTRIGHT_RELEASE_ENV");' > "$tmp/override/crates/x/src/lib.rs"
  run_case override

  if [ "$failures" -gt 0 ]; then
    trap - EXIT
    rm -rf "$tmp"
    echo "v2-repository-shape self-test: $failures failure(s)" >&2
    return 2
  fi
  trap - EXIT
  rm -rf "$tmp"
  echo "v2-repository-shape self-test: all cases pass"
  return 0
}

if [ "$SELF_TEST" -eq 1 ]; then
  self_test
  exit $?
fi

if check_shape "$ROOT"; then
  echo "[PASS] v2 repository shape"
  exit 0
else
  echo "[FAIL] v2 repository shape: $violations violation(s)" >&2
  exit 1
fi
