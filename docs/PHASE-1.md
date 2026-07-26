# CutRight Phase 1 — local ingest

Implemented:

- `video-media` owns a fixed FFprobe invocation and parses duration, dimensions, frame rate,
  rotation, and HDR transfer metadata;
- `video-project` hashes sources with BLAKE3 in bounded chunks and writes absolute paths to the
  project source manifest;
- `videoctl ingest <project> <sources...>` emits machine-readable JSON events;
- repeated ingest is idempotent;
- a changed registered source is rejected before probing and cannot rewrite the manifest;
- dry-run remains non-mutating.

The smoke gate uses generated MP4 and WAV fixtures, checks metadata and hashes, reruns both files,
then mutates one temporary source and proves the manifest remains unchanged.

Remaining Phase 1 work is local speech analysis: audio extraction, Silero VAD, ScrapeRight/Parakeet
transcription, WhisperX verification, and packed transcript output.
