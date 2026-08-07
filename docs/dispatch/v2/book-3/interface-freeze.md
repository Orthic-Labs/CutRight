---
task: CR-V2-B3-006
book: 3
lane: S
status: frozen
title: Freeze Book 3 pack/evidence/job crate boundaries and lane ownership
commit: CR-V2-B3-006: freeze-book-3-pack-evidence-job-crate-boundaries-and-lane-
depends_on:
  - CR-V2-B3-005
---

# Book 3 Interface Freeze

## Purpose

Freeze the boundaries between the three parallel lanes of Book 3 (signed
runtime packs, hierarchical evidence graph, and durable job plane) and the
crate ownership each lane has. Frozen names must survive the merge step
(CR-V2-B3-022) without churn.

## Lane ownership

| Lane | Path                        | Crates                     | Scope                                                                 |
|------|-----------------------------|----------------------------|-----------------------------------------------------------------------|
| A    | `runtime/source/**`         | `crates/video-runtime`     | Source/build roots for native sidecars (FFmpeg, Silero, whisper.cpp). |
| A    | `runtime/manifests/*.source.json` | `crates/video-runtime` | Source manifests for native packages.                                  |
| B    | `runtime/manifests/*.model.json`  | `crates/video-inference` | Model manifests (bytes, hashes, licenses).                            |
| B    | `runtime/models/**`        | `crates/video-inference`   | Model bytes (when present) and adapter glue.                           |
| C    | `schema/evidence/**`        | `crates/video-evidence`    | Evidence graph data structures and I/O.                                |
| C    | `schemas/jobs/**`           | `crates/video-jobs`        | Job DAG, stage, fingerprint data structures and I/O.                   |

## Frozen pack IDs

These seven pack IDs are reserved by the contract and may not be redefined
by any lane:

```text
media
speech
speech-quality
director
vision
voice
creative
```

`media` and `speech` belong to lane A (runtime/source). `speech-quality`
belongs to lane A. `director` and `voice` belong to lane B
(runtime/manifests and `crates/video-inference`). `vision` and `creative`
belong to lane C (evidence and jobs).

## Frozen capability handshakes

Every component that exposes a capability must publish a manifest with at
minimum:

- `schema` — frozen version string (`cutright.pack_manifest/v1`)
- `pack_id` — one of the seven reserved IDs (or, for orthogonal packs, a
  new ID entered into the cap-ledger)
- `version` — semver `MAJOR.MINOR.PATCH[-PRERELEASE]`
- `capabilities` — typed enum of declared capabilities
- `models` — list of model IDs with hashes (lane B responsibility)
- `active_hashes` — current manifest + signature digests

## Lane A (runtime/source + `video-runtime`)

Owns:

- `runtime/source/ffmpeg/**` (CR-V2-B3-007)
- `runtime/source/silero-vad/**` (CR-V2-B3-010)
- `runtime/source/whisper.cpp/**` (CR-V2-B3-011)
- `scripts/runtime/build-{ffmpeg,silero,whisper}-*.py`
- `runtime/manifests/{media,silero-vad,whisper-verifier}.source.json`
- `crates/video-runtime/{Cargo.toml, src/**}`

Public API:

- `crates/video-runtime/src/sidecar.rs` — `Sidecar` trait
- `crates/video-runtime/src/probe.rs` — capability probe runner
- `crates/video-runtime/src/manifest.rs` — manifest loader

## Lane B (runtime/models + `video-inference`)

Owns:

- `runtime/models/**` (model bytes and metadata)
- `scripts/runtime/build-llama.py` (CR-V2-B3-012)
- `runtime/manifests/*.model.json` (CR-V2-B3-013..015)
- `crates/video-inference/{Cargo.toml, src/**}`

Public API:

- `crates/video-inference/src/runtime.rs` — `LocalInferenceRuntime`
- `crates/video-inference/src/structured.rs` — typed structured output
- `crates/video-inference/src/handle.rs` — `ModelHandle` lifecycle

## Lane C (evidence + jobs)

Owns:

- `schemas/evidence/{node,edge,graph}.schema.v1.json` (CR-V2-B3-002)
- `schemas/jobs/{job,stage,fingerprint}.schema.v1.json` (CR-V2-B3-003)
- `crates/video-evidence/{Cargo.toml, src/**}` — evidence graph reader/writer
- `crates/video-jobs/{Cargo.toml, src/**}` — job DAG, scheduler, checkpoint

Public API:

- `crates/video-evidence/src/graph.rs` — `EvidenceGraph`
- `crates/video-jobs/src/plan.rs` — `JobPlan` and `StageAttempt`
- `crates/video-jobs/src/fingerprint.rs` — `JobFingerprint`

## Workspace integration

Workspace integration, project-level orchestration, doctor, release pack
assembly, and benchmark compatibility checks are intentionally reserved for
the sequential tasks `CR-V2-B3-022..027`. Lanes A/B/C MUST NOT:

- modify root `Cargo.toml` membership; the membership is updated by
  `CR-V2-B3-022` once all three lanes are committed.
- write to `docs/architecture/V2-CRATE-DAG.md`; the lane-boundary update
  goes there in `CR-V2-B3-022`.
- add a workspace dependency; the workspace's `dependencies` table is
  append-only across parallel lanes.

## Parallel root non-overlap

Verified by `scripts/architecture/check_crate_dag.py`:

- Lane A writes only under `runtime/source/`, `runtime/manifests/*.source.json`,
  `scripts/runtime/`, and `crates/video-runtime/`.
- Lane B writes only under `runtime/models/`, `runtime/manifests/*.model.json`,
  `scripts/runtime/`, and `crates/video-inference/`.
- Lane C writes only under `schemas/evidence/`, `schemas/jobs/`, and the
  two new crates `crates/video-evidence/`, `crates/video-jobs/`.

Cross-lane overlap is impossible because the file globs above are disjoint.

## Handshakes

Components advertise:

- `version` — semver; minor bumps MUST add capability, major bumps MAY
  break the wire shape but never the on-disk shape.
- `capabilities` — enum of declared capabilities.
- `model_ids` — list of model IDs this component consumes.
- `active_hashes` — map of `manifest_hash` and `signature_digest` for the
  component's current build.

A consumer verifies the `active_hashes` match the locally-installed build
before constructing a `ModelHandle` or scheduling a `Job`. A mismatch is a
typed `HandshakeError::HashMismatch`.

## Acceptance

- Parallel roots do not overlap.
- No pack component owns project state.
- Handshakes expose version, capabilities, model IDs, and active hashes.
- Lane A lane-B lane-C files are disjointly owned.
