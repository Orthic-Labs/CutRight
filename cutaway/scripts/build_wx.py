#!/usr/bin/env python3
"""build_wx.py — THE short-form cut builder.
Cut on WhisperX's EXACT word edges, keep every word whole, remove only the no-word gaps.

Input = WhisperX-aligned words (whisperx_align.py) + beats.txt (the editor's red-thread: one clean
take per beat, folds/doubles/meta/ums already excluded). For each beat it keeps the words inside it,
walks them, and starts a NEW clip whenever (a) a new beat begins or (b) the gap between two words
exceeds GAP — i.e. a real "silence" (a stretch with no words, which is what "silence" means here,
NOT waveform dips). Each clip is bounded by word.start−LEAD / word.end+TAIL, so a word is never
clipped and a clip never starts in dead air.

Usage: build_wx.py CLIP_wx.json beats.txt MEDIA_POOL_ITEM_ID [GAP] [SRC_FPS] > clip_infos.json
  (MEDIA_POOL_ITEM_ID = your Resolve media-pool item id; for any other editor pass a label like "src".)
  GAP (default 0.22s) = the tightness dial. Smaller = remove more pauses (punchier, more jump-cuts);
  larger = keep more natural pauses (breathier). 0.22 is a good punchy-but-flowing default.
"""
import json, sys

WX, BEATS, MPID = sys.argv[1], sys.argv[2], sys.argv[3]
GAP = float(sys.argv[4]) if len(sys.argv) > 4 else 0.22
FPS = float(sys.argv[5]) if len(sys.argv) > 5 else 24000 / 1001
LEAD, TAIL = 0.04, 0.06
UMS = {"uh", "um", "umm", "uhh", "hmm", "mm", "err", "eh"}

W = [(w["s"], w["e"], w["w"]) for w in json.load(open(WX))]
beats = []
for ln in open(BEATS):
    ln = ln.strip()
    if ln and not ln.startswith("#"):
        a, b, *r = ln.split()
        beats.append((float(a), float(b), r[0] if r else ""))

# kept words = words whose midpoint falls in an editorial beat, minus ums
kept = []
for ws, we, ww in W:
    if ww.lower().strip(".,!?") in UMS:
        continue
    mid = (ws + we) / 2
    for bi, (a, b, lab) in enumerate(beats):
        if a <= mid <= b:
            kept.append((ws, we, ww, bi)); break

# group into clips; cut at beat changes and at word-gaps > GAP (the no-word silences)
clips, cs, ce, pbi = [], None, None, None
for ws, we, ww, bi in kept:
    if cs is None:
        cs, ce, pbi = ws, we, bi; continue
    if bi != pbi or ws - ce > GAP:
        clips.append([cs - LEAD, ce + TAIL]); cs, ce, pbi = ws, we, bi
    else:
        ce = we
if cs is not None:
    clips.append([cs - LEAD, ce + TAIL])

infos, rec = [], 0
for s, e in clips:
    sf, ef = round(max(0, s) * FPS), round(e * FPS)
    infos.append({"media_pool_item_id": MPID, "start_frame": sf, "end_frame": ef, "record_frame": rec})
    rec += ef - sf

print(json.dumps(infos))
print(f"# {len(infos)} clips, {rec/FPS:.1f}s (GAP={GAP}s)", file=sys.stderr)
for s, e in clips:
    txt = " ".join(w[2] for w in W if s <= (w[0] + w[1]) / 2 <= e)
    print(f"#  {s:7.2f}-{e:7.2f} ({e-s:4.1f}s)  «{txt[:56]}»", file=sys.stderr)
