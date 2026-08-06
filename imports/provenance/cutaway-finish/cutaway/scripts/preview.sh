#!/usr/bin/env bash
# Step 7 — render the 9:16 preview proof from the editorial keeplist, then tighten.
# Usage: ./preview.sh /path/to/SRC.MP4   (run from your working folder)
# Reads preview/keeplist.txt ("START END label" per line). Outputs:
#   preview/CLIP_roughcut.mp4  (editorial cut, 9:16 center-crop)
#   preview/CLIP_tight.mp4     (+ mechanical silence tighten — the punchy version)
set -e
SRC="$1"
AE=./.venv/bin/auto-editor
mkdir -p preview/seg
rm -f preview/seg/*.mp4 preview/concat.txt
i=0
while read -r start end name; do
  [ -z "$start" ] && continue
  case "$start" in \#*) continue;; esac
  dur=$(echo "$end - $start" | bc)
  out="preview/seg/$(printf '%02d' $i)_$name.mp4"
  ffmpeg -y -nostdin -ss "$start" -i "$SRC" -t "$dur" \
    -vf "crop=ih*9/16:ih,scale=1080:1920,fps=30" \
    -c:v libx264 -preset veryfast -crf 20 -c:a aac -b:a 160k -ac 2 "$out" 2>/dev/null
  echo "file 'seg/$(printf '%02d' $i)_$name.mp4'" >> preview/concat.txt
  i=$((i+1))
done < preview/keeplist.txt

cd preview
ffmpeg -y -nostdin -f concat -safe 0 -i concat.txt -c copy CLIP_roughcut.mp4 2>/dev/null
cd ..
# mechanical tighten pass — the 57s "energy" version (margin 0.18, threshold 4%)
$AE preview/CLIP_roughcut.mp4 --margin 0.18s --edit audio:threshold=4% -o preview/CLIP_tight.mp4 2>/dev/null
echo "rough: $(ffprobe -v quiet -show_entries format=duration -of csv=p=0 preview/CLIP_roughcut.mp4)s"
echo "tight: $(ffprobe -v quiet -show_entries format=duration -of csv=p=0 preview/CLIP_tight.mp4)s"
echo "residual silences >0.4s in tight: $(ffmpeg -nostdin -i preview/CLIP_tight.mp4 -af 'silencedetect=noise=-30dB:d=0.4' -f null - 2>&1 | grep -c silence_start)"
