#!/usr/bin/env python3
"""flow_cut.py — a short-form rough-cut builder (documented dead end; build_wx.py is the live one).
take the words → remove EVERY silence/um/noise between them → stitch the kept takes into one FLOW.
Whole words only. No silent clips. No 1-frame glitch cuts. Fast & punchy.

KEY: silences are found by AUTO-EDITOR's ADAPTIVE loudness (threshold relative to the clip), NOT a
fixed-dB silencedetect and NOT whisper word gaps. Fixed-dB fails when mic distance varies (close mic =
room tone above threshold → pause missed); whisper mistimes words around pauses. auto-editor's relative
threshold is what an early version relied on. Whisper words are used only to keep cut edges whole.

Pipeline: auto-editor v1 (adaptive loud/silent) → editorial beats (one clean take each) → keep only the
LOUD spans inside each beat → snap each to whole words → drop wordless/um regions → merge contiguous
clips (no glitch cuts).

Prep the v1 first:
  ./.venv/bin/auto-editor audio/CLIP.mp3 --edit audio:threshold=4% --margin 0.05s --export v1 -o work/CLIP.v1
Usage: flow_cut.py CLIP.v1 WORDS.json beats.txt MEDIA_POOL_ITEM_ID [SRC_FPS] > clip_infos.json
"""
import json, sys

V1, WORDS, BEATS, MPID = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
FPS = float(sys.argv[5]) if len(sys.argv) > 5 else 24000 / 1001
LEAD, TAIL = 0.06, 0.10
MERGE = 0.12        # seconds; clips closer than this in the source are the same shot → merge
UMS = {"uh", "um", "umm", "uhh", "hmm", "mm", "err", "eh"}

# 1) adaptive loud spans from auto-editor v1 (chunks are [start_f, end_f, speed] at timebase 30)
vd = json.load(open(V1)); TB = 30.0
loud = [(c[0]/TB, c[1]/TB) for c in vd["chunks"] if c[2] == 1]

def minus_sil(a, b):
    """keep only the LOUD (speech) spans inside [a,b]"""
    return [(max(a, ls), min(b, le)) for ls, le in loud if le > a and ls < b and min(b, le) - max(a, ls) > 0.04]

# 2) words (for whole-word edges only)
d = json.load(open(WORDS))
W = [(w["s"], w["e"], w["w"].strip()) for s in d for w in s.get("words", [])]

# 3) editorial beats → speech regions → word-snapped clips
clips = []
for line in open(BEATS):
    line = line.strip()
    if not line or line.startswith("#"): continue
    a, b, *r = line.split(); a, b = float(a), float(b)
    for rs, re_ in minus_sil(a, b):
        win = [(ws, we, ww) for ws, we, ww in W if we > rs and ws < re_]
        win = [w for w in win if w[2].lower().strip(".,!?") not in UMS]
        if not win: continue                         # silence / um-only → drop
        clips.append([max(a, win[0][0] - LEAD), min(b, win[-1][1] + TAIL)])

# 4) merge clips contiguous in source (kills glitch cuts); real silences leave a gap → stay split
merged = []
for c in sorted(clips):
    if merged and c[0] - merged[-1][1] < MERGE:
        merged[-1][1] = max(merged[-1][1], c[1])
    else:
        merged.append(c)

infos, rec = [], 0
for s, e in merged:
    sf, ef = round(max(0, s) * FPS), round(e * FPS)
    infos.append({"media_pool_item_id": MPID, "start_frame": sf, "end_frame": ef, "record_frame": rec})
    rec += ef - sf

print(json.dumps(infos))
print(f"# {len(infos)} clips, {rec} frames = {rec/FPS:.1f}s (adaptive auto-editor)", file=sys.stderr)
for s, e in merged:
    txt = " ".join(w[2] for w in W if s <= (w[0]+w[1])/2 <= e)
    print(f"#  {s:7.2f}-{e:7.2f} ({e-s:4.1f}s)  «{txt[:58]}»", file=sys.stderr)
