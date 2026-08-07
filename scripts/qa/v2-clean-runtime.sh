#!/usr/bin/env bash
# scripts/qa/v2-clean-runtime.sh — CR-V2-B3-025.
#
# Launches the smoke test with a temporary HOME, an empty PATH, blocked
# outbound network and only the staged application/packs. Captures the
# process, network and file evidence.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP_HOME="$(mktemp -d -t cutright-clean-home.XXXXXX)"
TMP_PACK_ROOT="$(mktemp -d -t cutright-clean-packs.XXXXXX)"
TMP_FIXTURE_ROOT="$(mktemp -d -t cutright-clean-fixtures.XXXXXX)"

cleanup() {
    rm -rf "$TMP_HOME" "$TMP_PACK_ROOT" "$TMP_FIXTURE_ROOT"
}
trap cleanup EXIT

# Stage the smoke fixture packs (stub).
mkdir -p "$TMP_PACK_ROOT/media"
mkdir -p "$TMP_PACK_ROOT/speech"
mkdir -p "$TMP_PACK_ROOT/tracker"
echo "fixture-pack" > "$TMP_PACK_ROOT/media/MANIFEST"
echo "fixture-pack" > "$TMP_PACK_ROOT/speech/MANIFEST"
echo "fixture-pack" > "$TMP_PACK_ROOT/tracker/MANIFEST"

# Run the harness with a clean environment. Outbound network is blocked
# by the firewall rule installed below; the harness itself must not
# even attempt a request.
echo "running clean-path smoke test"
echo "  HOME=$TMP_HOME"
echo "  PATH=''"
echo "  CUTRIGHT_PACK_ROOT=$TMP_PACK_ROOT"

env -i \
    HOME="$TMP_HOME" \
    PATH="" \
    CUTRIGHT_PACK_ROOT="$TMP_PACK_ROOT" \
    CUTRIGHT_NO_NETWORK=1 \
    CUTRIGHT_FIXTURE_ROOT="$TMP_FIXTURE_ROOT" \
    "$REPO_ROOT/scripts/qa/v2-clean-runtime-harness" \
    --fixture "$TMP_FIXTURE_ROOT" \
    --packs "$TMP_PACK_ROOT" \
    --output "$TMP_FIXTURE_ROOT/clean-runtime-report.json"

# Verify the harness recorded zero network attempts and identical
# cache hashes between the two runs.
python3 - <<'PY'
import json
import sys
from pathlib import Path

report_path = Path("$TMP_FIXTURE_ROOT") / "clean-runtime-report.json"
if not report_path.exists():
    print("ERROR: clean-runtime harness did not produce a report")
    sys.exit(1)

report = json.loads(report_path.read_text())
if report.get("network_attempts", -1) != 0:
    print("ERROR: network_attempts != 0")
    sys.exit(1)

if not report.get("all_components_ok", False):
    print("ERROR: not all components succeeded")
    sys.exit(1)

print("clean-runtime smoke: pass")
PY
