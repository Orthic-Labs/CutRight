# CutRight Phase 0

Phase 0 freezes the portable project contract before media execution exists.

Implemented in this checkout:

- Rust workspace with `video-core`, `video-project`, and `videoctl`;
- canonical serde models for project, sources, transcript, VAD, timeline, finish plan, and provider envelopes;
- rational-FPS and source/output timestamp mapping tests;
- idempotent `videoctl project init` with immutable-source policy;
- JSON-only CLI events and the full planned command surface;
- provider traits for transcription and VAD;
- JSON Schema documents and fixture-format guidance;
- one installable video-editor skill with internal workflow documents.

Phase 1 now attaches the typed FFprobe boundary and immutable source registration to these contracts;
see [PHASE-1.md](PHASE-1.md).
