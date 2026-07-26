#!/usr/bin/env python3
"""Pass B core (CORRECTED METHOD 2026-06-13) — cut on WHISPER WORD BOUNDARIES, never mid-word.

Amplitude margins (auto-editor) clip whole words off ("This...", "...ASAP"). This builder
instead snaps every cut to word.start / word.end from word-level whisper, so essential text
is NEVER lost. It also removes long internal pauses by splitting AT word gaps (word-safe).

Usage:
  python3 build_wordcut.py WORDS.json beats.txt MEDIA_POOL_ITEM_ID [SRC_FPS] [GAP] > clip_infos.json
    WORDS.json  faster-whisper output WITH words (transcribe.py)
    beats.txt   editorial beats, one per line: "APPROX_START APPROX_END label"
                (approx — the script snaps to the actual words inside the window)
    GAP         internal pause (s) above which a beat is split into sub-clips (default 0.55)

Pads: keeps 0.10s before the first word and 0.18s after the last word of each sub-clip,
clamped so a cut never reaches into a neighbouring word. Word-completeness > tightness.
"""
import json, sys

WORDS, BEATS, MPID = sys.argv[1], sys.argv[2], sys.argv[3]
FPS = float(sys.argv[4]) if len(sys.argv) > 4 else 24000 / 1001
GAP = float(sys.argv[5]) if len(sys.argv) > 5 else 0.55
PAD_IN, PAD_OUT = 0.10, 0.18

d = json.load(open(WORDS))
W = [(w["s"], w["e"], w["w"]) for s in d for w in s.get("words", [])]

def words_in(a, b):
    return [(s, e, w) for (s, e, w) in W if (s + e) / 2 >= a and (s + e) / 2 <= b]

spans = []  # (start_sec, end_sec, label)
for line in open(BEATS):
    line = line.strip()
    if not line or line.startswith("#"):
        continue
    a, b, *rest = line.split()
    a, b = float(a), float(b)
    lab = rest[0] if rest else ""
    ws = words_in(a, b)
    if not ws:
        continue
    # split into sub-clips at internal gaps > GAP (cut between words, never inside)
    sub = [[ws[0]]]
    for i in range(1, len(ws)):
        if ws[i][0] - ws[i - 1][1] > GAP:
            sub.append([])
        sub[-1].append(ws[i])
    for k, grp in enumerate(sub):
        fw, lw = grp[0], grp[-1]
        start = fw[0] - PAD_IN
        end = lw[1] + PAD_OUT
        spans.append((max(0, start), end, lab + (f"~{k}" if len(sub) > 1 else "")))

infos, rec = [], 0
for s, e, lab in spans:
    sf, ef = round(s * FPS), round(e * FPS)
    infos.append({"media_pool_item_id": MPID, "start_frame": sf, "end_frame": ef, "record_frame": rec})
    rec += ef - sf

print(json.dumps(infos))
print(f"# {len(infos)} sub-clips, total {rec} frames = {rec / FPS:.1f}s", file=sys.stderr)
for s, e, lab in spans:
    print(f"#   {lab:20} {s:7.2f}-{e:7.2f}  ({e-s:.2f}s)", file=sys.stderr)
