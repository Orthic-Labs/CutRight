# CutRight v2 Benchmark Taxonomy

This document defines the benchmark axes, dataset split policy, and result
statuses that every benchmark run must satisfy. The taxonomy is the single
source of truth for `benchmarks/corpus/`, `benchmarks/metrics/`, and
`benchmarks/runs/`. Any new metric, axis, or split rule must be added here
before it can appear in a benchmark report.

## 1. Evaluation axes

Every benchmark finding belongs to exactly one axis. An axis groups metrics
that measure the same property of the editorial pipeline.

| Axis ID | Property | Release blocking? |
|---|---|---|
| `kernel_integrity` | Source mutation, atomicity, undo, receipts, cache identity | Yes |
| `speech_boundary` | Word/phoneme clipping, segment boundaries, transcript integrity | Yes |
| `audio_visual` | A/V sync drift, transient alignment, non-target preservation | Yes |
| `editorial` | Beat segmentation, take selection, ordering, hook/payoff/CTA | No (advisory) |
| `creative` | Branding, subject/caption collisions, reduced-motion equivalence | Yes (only collisions) |
| `instruction` | Brief checklist, target success, untouched preservation | Yes |
| `reliability` | Cold start, cancellation, resume, offline, peak memory | Yes |

The benchmark run must report every metric. A metric that did not run is
`unproven`, never `pass`.

## 2. Dataset split policy

Every media item belongs to exactly one split:

- `train` — calibration, never reported as release evidence.
- `calibration` — threshold tuning, never reported as release evidence.
- `test` — only split that contributes to release claims.

Splits are assigned by:

1. `speaker_id` — every speaker tag must land in exactly one split.
2. `recording_session_id` — every session must land in exactly one split.
3. `source_program_id` — every source programme must land in exactly one split.

Near-duplicate test material (BLAKE3-hashed source bytes within a
documented similarity threshold) may NOT cross splits. Split leakage is a
validation error and the corpus manifest is rejected.

## 3. Item-level requirements

Every item in the corpus must declare:

- `rights/provenance` record with reviewer identity and consent timestamp.
- `expected_language` (BCP-47).
- `conditions` (room tone, lighting, frame rate, camera handling).
- `labels` (human annotations with reviewer IDs and split assignment).
- `allowed_distribution` (`local_only` or `redistributable`).

An item with missing rights or split assignment cannot enter a run.

## 4. Result statuses

Every metric result carries one of:

| Status | Meaning |
|---|---|
| `pass` | The metric was run and met its floor. |
| `fail` | The metric was run and missed its floor. |
| `skipped_with_reason` | Deliberately skipped; a reason string is required. |
| `unsupported` | The target environment cannot run this metric. |
| `unproven` | The metric was not run yet (no evidence either way). |

A benchmark report that marks `unproven` items as `pass` is invalid.

## 5. Required shapes

```text
pub enum MetricStatus { Pass, Fail, SkippedWithReason, Unsupported, Unproven }
```

This is the canonical Rust enum re-exported by `video-benchmarks` and used
throughout the benchmark runner. Benchmarks that introduce new states must
update this enum and the JSON enum list together.
