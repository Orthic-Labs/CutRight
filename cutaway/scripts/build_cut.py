#!/usr/bin/env python3
"""Pass B DEFINITIVE builder (2026-06-13) — combines the two ground-truth signals:
  • AUDIO ENERGY (ffmpeg silencedetect) decides WHERE to split — real silences only, so a clip
    is NEVER placed in dead air / a camera-move pause (the v3 hook-c bug).
  • WHISPER WORD EDGES decide the exact in/out — so a word is NEVER clipped ("This", "ASAP").
  • A speech region with NO words (pure silence) is DROPPED automatically.
Whisper word *positions* are NOT trusted to place cuts (whisper can hallucinate a word into a
multi-second silence). Words are used only to find clean edges inside verified-speech regions.

Usage: build_cut.py AUDIO.mp3 WORDS.json beats.txt MEDIA_POOL_ITEM_ID [SRC_FPS] > clip_infos.json
  beats.txt: take-level editorial spans "APPROX_START APPROX_END label" (folds/doubles already excluded)

Tunables: MINSIL=0.45 (split only on silences this long), PAD_IN=0.10, PAD_OUT=0.20.
"""
import json, sys, subprocess, re

AUDIO, WORDS, BEATS, MPID = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
FPS = float(sys.argv[5]) if len(sys.argv) > 5 else 24000 / 1001
MINSIL, PAD_IN, PAD_OUT = 0.45, 0.10, 0.20

# 1) audio-energy silences (ground truth for WHERE to cut)
out = subprocess.run(["ffmpeg","-nostdin","-i",AUDIO,"-af",f"silencedetect=noise=-30dB:d={MINSIL}","-f","null","-"],
                     capture_output=True, text=True).stderr
sil, cur = [], None
for line in out.splitlines():
    m = re.search(r"silence_start: ([\d.]+)", line)
    if m: cur = float(m.group(1))
    m = re.search(r"silence_end: ([\d.]+)", line)
    if m and cur is not None: sil.append((cur, float(m.group(1)))); cur = None

def speech_regions(a, b):
    """[a,b] minus the silences = list of verified-speech intervals."""
    regs, s = [], a
    for ss, se in sil:
        if se <= a or ss >= b: continue
        if ss > s: regs.append((s, min(ss, b)))
        s = max(s, se)
    if s < b: regs.append((s, b))
    return [(x, y) for x, y in regs if y - x > 0.05]

# 2) words (only for clean edges)
d = json.load(open(WORDS))
W = [(w["s"], w["e"], w["w"]) for s in d for w in s.get("words", [])]

infos, rec, dbg = [], 0, []
for line in open(BEATS):
    line = line.strip()
    if not line or line.startswith("#"): continue
    a, b, *rest = line.split(); a, b = float(a), float(b)
    lab = rest[0] if rest else ""
    for ri, (rs, re_) in enumerate(speech_regions(a, b)):
        # include any word that OVERLAPS the speech region (not just midpoint) so a word
        # straddling a silence boundary ("This", "works", "All") is kept whole, never clipped
        win = [(ws, we, ww) for ws, we, ww in W if we > rs and ws < re_]
        if not win:        # pure silence / camera-move region → drop
            continue
        cs = max(a, win[0][0] - PAD_IN)
        ce = min(b, win[-1][1] + PAD_OUT)
        sf, ef = round(cs * FPS), round(ce * FPS)
        infos.append({"media_pool_item_id": MPID, "start_frame": sf, "end_frame": ef, "record_frame": rec})
        rec += ef - sf
        dbg.append(f"{lab}{('~'+str(ri)) if ri else '':4} {cs:7.2f}-{ce:7.2f} ({ce-cs:.2f}s)  «{' '.join(w[2].strip() for w in win)[:48]}»")

print(json.dumps(infos))
print(f"# {len(infos)} subclips, total {rec} frames = {rec/FPS:.1f}s", file=sys.stderr)
for x in dbg: print("#  " + x, file=sys.stderr)
