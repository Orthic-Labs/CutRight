#!/usr/bin/env bash
# Pass B helper — extract a STORYBOARD the editor (best model's own vision, or Gemini 3.1 Pro) inspects
# to judge framing, eye contact, pointing/off-screen gestures, and which take is cleanest.
# Usage: ./storyboard.sh SRC.MP4 keeplist.txt OUTDIR
#   -> OUTDIR/board/NN_<label>.jpg  (one frame per beat)
#   -> OUTDIR/sheet.png             (single contact sheet to Read in one shot)
# Pick the frame at a point in the KEPT audio, not the editorial-span midpoint (which can land in a pause).
set -e
SRC="$1"; KEEP="$2"; OUT="${3:-storyboard}"
mkdir -p "$OUT/board"; rm -f "$OUT/board"/*.jpg
i=0
while read -r start end name; do
  [ -z "$start" ] && continue
  case "$start" in \#*) continue;; esac
  # sample ~1/3 into the span (usually inside speech, not the trailing pause)
  t=$(echo "scale=2; $start + ($end-$start)/3" | bc)
  ffmpeg -y -nostdin -ss "$t" -i "$SRC" -frames:v 1 \
    -vf "scale=360:-1,drawtext=text='${name} ${start}s':x=8:y=8:fontsize=20:fontcolor=yellow:box=1:boxcolor=black@0.6" \
    "$OUT/board/$(printf '%02d' $i)_${name}.jpg" 2>/dev/null
  i=$((i+1))
done < "$KEEP"
cd "$OUT"
cols=3; rows=$(( (i + cols - 1) / cols ))
ffmpeg -y -nostdin -pattern_type glob -i "board/*.jpg" -filter_complex "tile=${cols}x${rows}:margin=6:padding=6" sheet.png 2>/dev/null
echo "storyboard: $OUT/sheet.png ($i beats) — Read it, then decide drops/best-takes"
