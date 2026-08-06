---
name: content-transcription
description: Transcribe Instagram reels, YouTube videos, TikToks, or local media through the ScrapeRight-backed ASR lane. Parakeet TDT is the primary ASR path; execution happens only inside the signed CutRight content runtime pack.
---

# Transcribe — ScrapeRight lane

Use the ScrapeRight dogfood lane for agent-run transcription. Do not use the old Python Whisper
pipeline unless the operator explicitly asks for legacy behavior.

## CutRight v2 execution

The upstream invocation was a host-scoped pipeline: a ScrapeRight checkout, a host ffmpeg binary,
an on-disk TDT model directory, and an onnxruntime dynamic library, all driven by host environment
variables on a Windows machine. None of that is vendored. In CutRight v2 the lane runs as the typed
capability

    cutright://capability/content.transcribe {"media": "<url or asset reference>", "formats": ["txt", "srt"]}

inside the signed content runtime pack. The pack supplies the ASR engine, ffmpeg, and model
weights; the base content skill requires nothing on PATH. Output is delivered as AssetDelivery
records — there is no direct mutation of host directories.

## Post-run checklist (substance preserved, retargeted to AssetDelivery)

1. Read the run report from the delivery manifest (upstream: `dogfood-results.json`).
2. Confirm `asr backend: parakeet-tdt-v3`.
3. Collect the useful `.txt` transcripts from the AssetDelivery set (upstream copied these into a
   shared transcripts directory; in CutRight they stay on the delivery record).
4. Maintain the per-run media index as part of the delivery manifest (upstream: `url_index.json`
   plus a tagging script).
5. Summaries that upstream wrote into a dated research bucket become delivery notes on the
   AssetDelivery record.

## Notes

- TDT may emit text, JSON, SRT, and VTT depending on the pack's current ASR support.
- Platform empty-media/auth failures are download-layer failures, not ASR failures.
- For batches, run one item per capability invocation; the lane currently accepts one effective
  media input per call.
