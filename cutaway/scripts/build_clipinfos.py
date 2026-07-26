#!/usr/bin/env python3
"""Step 5 — build the Resolve cut list: editorial keeplist ∩ auto-editor silence.

Usage: python3 build_clipinfos.py CLIP.v1 keeplist.txt MEDIA_POOL_ITEM_ID [SRC_FPS] [MARG] > clip_infos.json

  CLIP.v1            auto-editor v1 export (chunks at timebase 30)
  keeplist.txt       editorial spans, one per line: "START_SEC END_SEC label"
  MEDIA_POOL_ITEM_ID Resolve clip id (from media_pool probe_media_pool)
  SRC_FPS            source fps, default 23.976 (24000/1001) for the Sony cam
  MARG               seconds of breath kept around each kept span, default 0.10
                     (use 0.05 + threshold=4% in the auto-editor step for the tight 57s feel)

Prints the clip_infos array for media_pool create_timeline_from_clips.
Stderr: subclip count + total seconds.
"""
import json, sys

V1, KEEP, MPID = sys.argv[1], sys.argv[2], sys.argv[3]
SRC_FPS = float(sys.argv[4]) if len(sys.argv) > 4 else 24000 / 1001
MARG = float(sys.argv[5]) if len(sys.argv) > 5 else 0.10
GAP_MERGE = 0.12  # merge two kept spans whose gap is smaller than this
MIN_SPAN = 0.15   # drop intersection fragments shorter than this

d = json.load(open(V1)); TB = 30.0  # auto-editor's timebase when fed bare audio
loud = [[c[0] / TB, c[1] / TB] for c in d["chunks"] if c[2] == 1]

edit = []
for line in open(KEEP):
    line = line.strip()
    if not line or line.startswith("#"):
        continue
    a, b = line.split()[:2]
    edit.append((float(a), float(b)))

subs = []
for es, ee in edit:
    for ls, le in loud:
        s, e = max(es, ls), min(ee, le)
        if e - s > MIN_SPAN:
            subs.append([max(es, s - MARG), min(ee, e + MARG)])

merged = []
for s, e in subs:
    if merged and s - merged[-1][1] < GAP_MERGE:
        merged[-1][1] = e
    else:
        merged.append([s, e])

infos, rec = [], 0
for s, e in merged:
    sf, ef = round(s * SRC_FPS), round(e * SRC_FPS)
    infos.append({"media_pool_item_id": MPID, "start_frame": sf, "end_frame": ef, "record_frame": rec})
    rec += ef - sf

print(json.dumps(infos))
print(f"# {len(infos)} subclips, total {rec} frames = {rec / SRC_FPS:.1f}s", file=sys.stderr)
