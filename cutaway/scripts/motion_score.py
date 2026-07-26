#!/usr/bin/env python3
"""Pass B helper — CAMERA-MOVE / motion detector (deterministic, no AI).
Flags spans where the camera is moving or the shot is unstable, so the
editor drops them or trims to a calm sub-window. Example run: cleanly
isolated a CTA camera move (max motion 44, 6 spikes) from 7 calm beats (<14).

Usage:
  python3 motion_score.py SRC.MP4 keeplist.txt        # score each editorial beat
  python3 motion_score.py SRC.MP4 START END           # dense per-frame scan of one span (find calm sub-window)

Motion = mean abs difference between consecutive 160x90 grayscale frames at 6fps.
Rule of thumb: meanMotion <6 = calm talking head; spikes >15 = camera move / reframe.
"""
import sys, subprocess, tempfile, glob, os
import numpy as np
from PIL import Image

def frames(src, start, dur, fps=6):
    d = tempfile.mkdtemp()
    subprocess.run(["ffmpeg","-y","-nostdin","-ss",str(start),"-t",str(dur),"-i",src,
                    "-vf",f"fps={fps},scale=160:90,format=gray",f"{d}/f%04d.png"],
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    fs = sorted(glob.glob(f"{d}/*.png"))
    arr = [np.asarray(Image.open(f), dtype=np.float32) for f in fs]
    for f in fs: os.remove(f)
    os.rmdir(d)
    return arr

def mads(arr):
    return np.array([np.mean(np.abs(arr[i+1]-arr[i])) for i in range(len(arr)-1)])

src = sys.argv[1]
if len(sys.argv) == 4:  # dense scan of one span
    start, end = float(sys.argv[2]), float(sys.argv[3])
    m = mads(frames(src, start, end-start))
    print(f"span {start}-{end}s  meanMotion={m.mean():.1f} maxMotion={m.max():.1f}")
    for i, v in enumerate(m):
        t = start + i/6
        flag = "  <-- CAMERA MOVE" if v > 15 else ""
        print(f"  {t:7.2f} {v:5.1f} {'#'*int(v)}{flag}")
else:  # score each beat in the keeplist
    print(f"{'beat':24} {'mean':>5} {'max':>5} {'spikes':>6}  verdict")
    for line in open(sys.argv[2]):
        line = line.strip()
        if not line or line.startswith("#"): continue
        s, e, *rest = line.split()
        name = rest[0] if rest else ""
        s, e = float(s), float(e)
        m = mads(frames(src, s, e-s))
        if len(m) == 0: continue
        sp = int((m > 15).sum())
        verdict = "DROP/TRIM (camera move)" if sp >= 2 or m.max() > 20 else ("watch" if m.max() > 14 else "calm ✓")
        print(f"{name:24} {m.mean():5.1f} {m.max():5.1f} {sp:6}  {verdict}")
