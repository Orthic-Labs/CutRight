# CutRight — as-built status

Generated from the gate and release process. Update it in the same change that
alters what it describes; do not edit it from memory.

```yaml
as_of_commit: 57992cb
current_stage: p1_hardening
primary_audio_engine: HeardRight
primary_asr: Parakeet TDT v3
word_edge_verifier: WhisperX
cloud_default: disabled
ci: none                                  # scripts/gate.sh is the contract
last_full_gate: scripts/gate.sh — root cargo (fmt/clippy/test), Studio cargo
                (fmt/clippy/test), Studio frontend (typecheck/test/build),
                license/asset resolution
known_blockers:
  - per_deliverable_qa                    # one YouTube-only report, not one per final
  - stage_receipts                        # no common receipt binding inputs/params/tools/outputs
  - content_addressed_cache               # cache identity still includes machine-local paths
  - media_toolchain_pairing               # ffprobe still resolved independently of ffmpeg
  - sidecar_content_addressing            # embedded workers keyed by version, not by hash
```

## What is closed

**Review contract (§5).** Studio sends a minimal `DecisionIntent`; Rust builds
the authoritative record — canonical non-injectable subject, artifact hash,
project and benchmark provenance, app version — appends it under a cross-process
lock in one buffered write, and returns exactly what was persisted. Replay
preserves stale and missing records instead of silently dropping them, and
reports malformed lines. Reason vocabularies are target-specific; verdict
controls are mode-gated; finals are per preset; QA acknowledgement binds the
report hash.

**Variant scoping (§6).** A hash-bound selection record gates final rendering;
`render final`, `reframe plan`, `finish validate`, `qa` and `export otio`
resolve the selected variant rather than a hard-coded `natural.mp4`. Gap
compaction implements the tight/natural pause policy. Cut-plan validation
accepts reordered non-overlapping source intervals. The timebase is explicit
and rational. Legacy layouts migrate with backups.

**Benchmark (§8).** HeardRight is the transcript authority in every branch.
Status is one of `primary_accepted`, `verifier_edges_required`,
`manual_review_required`, `verifier_unavailable`; two clean providers accept
HeardRight instead of deadlocking, and only `manual_review_required` blocks
destructive word-edge automation. The zero-unmatched-words requirement is gone,
replaced by coverage, unmatched-content rate and delta distributions, with
thresholds in `schemas/benchmark-policy.v1.json`. Reports bind source,
transcript, envelope and engine identity hashes.

**Schema (§8.4/§8.5).** `source_word_id` is described, unknown fields are
rejected, and semantic validation covers ordering, uniqueness, source
references and timebase rationality, with valid and invalid fixtures per
version.

**Local audio boundary (§9).** One HeardRight session serves transcription and
VAD behind a handshake, with unique request and trace ids, hard-fail response
correlation, per-request timeouts, bounded stderr, exactly one controlled
restart, and no model-directory knowledge or network fallback. VAD provenance
persists to `analysis/vad-<source>.provenance.json`.

**Process safety (§10.1/§10.6/§10.8).** Every spawn goes through one runner:
environment allow-list, timeout, kill-tree, byte caps, cancellation, temp
cleanup, telemetry. Atomic writes use collision-proof temp names. A JSON
`status: "error"` can no longer exit zero — see the exit-code table in
`crates/video-cli/src/main.rs`.

**Diagnostics (§11).** `doctor --profile core|audio|render|studio|all`, with
`--strict` and `--write-receipt`. Probes are active: real temp-dir lifecycle,
real tiny encodes for h264_videotoolbox/libx264/AAC/zscale re-probed by the
paired ffprobe. Checks this crate cannot honestly verify report `missing` with
remediation rather than a fabricated `ok`.

**Studio integrity (§12).** Asset grants are exact files, not a recursive grant
of the project tree, and source grants require a regular file and a successful
media probe. Relink only rewrites the manifest on a BLAKE3 match and records
every attempt. `ArtifactState` distinguishes missing from corrupt from stale.
`project_revision` supplements `generated_at`. Project identity is random and
immutable — identically named projects no longer collide, and existing projects
gain an instance id on migration without losing their original id.

## What is open

Per-deliverable QA reports and preset-specific captions (§13); stage receipts,
content-addressed caching and sidecar materialization, and paired
FFmpeg/FFprobe toolchain resolution (§10.2–§10.5); behavior-preserving
decomposition of `video-project` and `video-media` (§14); product phases 4–9 —
caption/audio/color finish, effect registry, semantic shorts, temporal
reframing, optional cloud, preference learning (§15).

Candidate generation still groups words by gap with best-take scoring rather
than performing red-thread editorial selection, and reframe still anchors one
face box per segment rather than tracking temporally.
