#!/usr/bin/env python3
"""Step 2 — transcribe for UNDERSTANDING (not for cutting).
Usage: python3 transcribe.py CLIP.mp3 OUT.json
faster-whisper medium, word + segment timestamps, VAD on.
Prints the human-readable transcript to stderr (the "memo").
NOTE: whisper segments smooth over long pauses — never use these boundaries to cut.
"""
import sys, json
from faster_whisper import WhisperModel

m = WhisperModel("medium", device="cpu", compute_type="int8")
segs, info = m.transcribe(sys.argv[1], word_timestamps=True, vad_filter=True)
print(f"# lang={info.language} dur={info.duration:.1f}s", file=sys.stderr)
out = []
for s in segs:
    words = [{"w": w.word, "s": round(w.start, 2), "e": round(w.end, 2)} for w in (s.words or [])]
    out.append({"start": round(s.start, 2), "end": round(s.end, 2), "text": s.text, "words": words})
    print(f"[{s.start:6.1f}-{s.end:6.1f}] {s.text}", file=sys.stderr)
json.dump(out, open(sys.argv[2], "w"), ensure_ascii=False, indent=0)
