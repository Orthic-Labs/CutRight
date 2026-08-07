#!/bin/bash
set -euo pipefail

# HeardRight ASR bias-fixture recording script
# Records audio from the default input device into an output directory:
# the first positional argument wins, otherwise a script-local directory
# (hr-bias-clips next to this script) keeps the fixtures self-contained.
# Reads prompts from fixtures-positive.txt and fixtures-negative.txt

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="${1:-${SCRIPT_DIR}/hr-bias-clips}"
POSITIVE_FIXTURES="${SCRIPT_DIR}/fixtures-positive.txt"
NEGATIVE_FIXTURES="${SCRIPT_DIR}/fixtures-negative.txt"
SAMPLE_RATE=16000
CHANNELS=1

validate_setup() {
    if ! command -v ffmpeg &> /dev/null; then
        echo "ERROR: ffmpeg is required but not installed" >&2
        exit 1
    fi

    if ! ffmpeg -list_devices true -f avfoundation -i "" 2>&1 | grep -qi "microphone\|audio input"; then
        echo "ERROR: No audio input device found. Check system audio settings." >&2
        exit 1
    fi

    mkdir -p "${OUTPUT_DIR}/positive" "${OUTPUT_DIR}/negative"
}

print_resume_summary() {
    local positive_count=0
    local negative_count=0

    if [[ -d "${OUTPUT_DIR}/positive" ]]; then
        positive_count=$(find "${OUTPUT_DIR}/positive" -name "*.wav" 2>/dev/null | wc -l)
    fi
    if [[ -d "${OUTPUT_DIR}/negative" ]]; then
        negative_count=$(find "${OUTPUT_DIR}/negative" -name "*.wav" 2>/dev/null | wc -l)
    fi

    echo "Resume: ${positive_count} positive, ${negative_count} negative clips recorded"
}

record_fixture() {
    local index="$1"
    local prompt="$2"
    local category="$3"
    local output_file="${OUTPUT_DIR}/${category}/$(printf "%03d" "$index").wav"
    local label_file="${OUTPUT_DIR}/${category}/$(printf "%03d" "$index").txt"

    if [[ -f "${output_file}" ]]; then
        echo "  [skip - already recorded]"
        return 0
    fi

    echo ""
    echo "[${index}] ${prompt}"
    read -p "Press Enter to start recording..." -r

    echo "Recording... (press Enter to stop)"

    ffmpeg -hide_banner -loglevel error \
        -f avfoundation -i ":0" \
        -acodec pcm_s16le \
        -ar "${SAMPLE_RATE}" \
        -ac "${CHANNELS}" \
        "${output_file}" &
    local ffmpeg_pid=$!

    read -r

    kill -INT "$ffmpeg_pid" 2>/dev/null || true
    wait "$ffmpeg_pid" 2>/dev/null || true

    echo "${prompt}" > "${label_file}"

    echo "✓ Saved: $(basename "$output_file")"
}

main() {
    validate_setup
    print_resume_summary

    echo ""
    echo "=== Recording positive fixtures ==="
    local idx=0
    while IFS= read -r line || [[ -n "$line" ]]; do
        [[ -z "$line" || "$line" =~ ^# ]] && continue
        record_fixture "$idx" "$line" "positive"
        ((idx++))
    done < "$POSITIVE_FIXTURES" || {
        echo "ERROR: Cannot read ${POSITIVE_FIXTURES}" >&2
        exit 1
    }

    echo ""
    echo "=== Recording negative fixtures ==="
    idx=0
    while IFS= read -r line || [[ -n "$line" ]]; do
        [[ -z "$line" || "$line" =~ ^# ]] && continue
        record_fixture "$idx" "$line" "negative"
        ((idx++))
    done < "$NEGATIVE_FIXTURES" || {
        echo "ERROR: Cannot read ${NEGATIVE_FIXTURES}" >&2
        exit 1
    }

    echo ""
    echo "✓ Recording complete: ${OUTPUT_DIR}"
}

main "$@"
