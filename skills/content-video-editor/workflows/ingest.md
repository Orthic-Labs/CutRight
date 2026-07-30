# Ingest

Turn a folder of raw footage into a registered, hash-pinned project. Nothing is edited here; sources
stay exactly where they are and are never modified.

## Inputs

- A folder (or explicit list) of raw source files: `.mp4`, `.mov`, mixed camera/iPhone footage OK.
- `ffmpeg` and `ffprobe` resolvable on `PATH`.

## Commands (in order)

```bash
# 1. Probe the toolchain. Must report status:"ok" for ffmpeg + ffprobe.
videoctl doctor

# 2. Create the project package. Capture result.path as <project> (the .video-project/ dir).
videoctl project init <folder>

# 3. Register sources by absolute path + BLAKE3. Pass every file; trailing sources are variadic.
videoctl ingest <project> /abs/path/cam-a-001.mp4 /abs/path/cam-a-002.mov ...
```

If `doctor` reports `status:"error"` or any check is `missing`, stop and fix the toolchain. A
zero-exit envelope whose body says `status:"error"` is still a failure — read the JSON, not just the
exit code.

Use `--dry-run` on `ingest` first when the source list is large or unfamiliar; it reports what would be
registered without writing the manifest.

## Evidence to read before proceeding

- `result.path` from `project.init` → this is `<project>` for every later stage.
- `sources/manifest.json` inside `<project>` → one entry per source with `source_id`, absolute `path`,
  `blake3`, `duration_ms`, and FFprobe metadata (frame rate rational, rotation, codec, colour
  primaries/transfer/matrix, audio format). Confirm the count matches the files you passed.

## Gate

- Every intended source appears in `sources/manifest.json` with a non-empty `blake3`.
- No source is inside the `.video-project/` package (raw files stay outside; the manifest references
  them by absolute path).
- Re-running `videoctl ingest` with the same files is idempotent; a file whose contents changed since
  registration is rejected (`SourceChanged`) — that is correct behaviour, not a bug. Do not "fix" it by
  re-registering over a mutated source; investigate why the source changed.

## Handoff outputs

- `<project>/sources/manifest.json` → consumed by [transcribe](transcribe.md) and every later stage.
- `<project>/project.json` → project manifest; later stages read `outputs[]` (presets: id, aspect,
  width, height) from here. Confirm the project declares at least a `youtube` preset (16:9) and, for
  vertical work, a `reels`/`tiktok` preset (9:16). If a needed preset is absent, fix `project.json`
  before reaching export.

## Notes

- Ingest creates analysis caches (16 kHz mono analysis audio, proxies, waveform peaks) under `cache/`.
  These are derived, content-addressed, and safe to regenerate; sources are not.
- HDR/rotation/mixed-FPS are recorded as metadata here and handled downstream (colour in
  [finish](finish.md), reframe in [reframe](reframe.md), normalisation at render). Nothing in ingest
  rejects unusual footage — it only records it faithfully.
