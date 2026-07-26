#!/usr/bin/env python3
"""WhisperX forced alignment → EXACT per-word timestamps (the breakthrough, 2026-06-13).
faster-whisper only *guesses* word times (drifts around pauses — it put "true" inside a 2s
silence, glued "it's" to 2.2s). WhisperX transcribes with faster-whisper THEN force-aligns every
word to the waveform with a phoneme model (wav2vec2) → real start/end per word. This is what lets
us cut exactly on words: no early starts, no clipped word-ends, no clips placed in silence.

Run from your WhisperX venv (WhisperX needs Python ≤3.12 + torch — see SKILL.md SETUP):
  HF_HOME=<YOUR_HF_CACHE> OMP_NUM_THREADS=4 \
    <YOUR_VENV>/bin/python whisperx_align.py audio/CLIP.mp3 work/CLIP_wx.json
Output = JSON list of {s,e,w} (word start/end seconds + text). ~1-2 min/clip on CPU; the ~360MB
alignment model downloads once to HF_HOME.
"""
import sys, json, whisperx

dev = "cpu"
audio = whisperx.load_audio(sys.argv[1])
model = whisperx.load_model("medium", dev, compute_type="int8")
res = model.transcribe(audio, batch_size=8)
lang = res["language"]; print("lang", lang, file=sys.stderr)
ma, meta = whisperx.load_align_model(language_code=lang, device=dev)
al = whisperx.align(res["segments"], ma, meta, audio, dev, return_char_alignments=False)
words = [{"s": round(w["start"], 3), "e": round(w["end"], 3), "w": w["word"]}
         for s in al["segments"] for w in s.get("words", []) if "start" in w and "end" in w]
json.dump(words, open(sys.argv[2], "w"), ensure_ascii=False)
print(f"{len(words)} aligned words", file=sys.stderr)
