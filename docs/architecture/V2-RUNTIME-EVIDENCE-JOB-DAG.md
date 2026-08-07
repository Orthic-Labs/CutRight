# V2 Runtime, Evidence, and Job DAG (CR-V2-B3-006)

## Purpose

Freeze the crate DAG and lane ownership for Book 3. Three parallel lanes
own disjoint code paths:

- **Lane A** — runtime source/build roots and `crates/video-runtime`
- **Lane B** — model manifests and `crates/video-inference`
- **Lane C** — `crates/video-evidence` and `crates/video-jobs`

The lane ownership table is the single source of truth for which crate
imports which other crate. Any violation is a merged-in bug.

## Crate DAG

```text
video-runtime ──depends on──▶ video-core
video-runtime ──depends on──▶ video-state (read-only)
video-inference ──depends on──▶ video-core
video-inference ──depends on──▶ video-runtime  (manifest loader only)
video-evidence ──depends on──▶ video-core
video-evidence ──depends on──▶ video-state (read-only)
video-jobs ──depends on──▶ video-core
video-jobs ──depends on──▶ video-evidence
video-jobs ──depends on──▶ video-runtime     (probe runner only)
video-actions ──depends on──▶ video-state
video-actions ──depends on──▶ video-capabilities
video-project ──depends on──▶ video-actions
video-project ──depends on──▶ video-state
video-cli ──depends on──▶ video-project
video-cli ──depends on──▶ video-jobs
```

Forbiddances:

- `video-runtime` MUST NOT depend on `video-inference` (lane A is upstream
  of lane B).
- `video-inference` MUST NOT depend on `video-actions` (lane B is invariant
  to action types).
- `video-evidence` MUST NOT depend on `video-jobs` (lane C is split into
  two crates; the job crate depends on the evidence crate, not the reverse).
- `video-jobs` MUST NOT depend on `video-capabilities` (capability is
  the lane-level concern; lane C is one lane away).
- No lane crate may depend on `video-cli` or `video-project`.

## Pack scopes

| Pack ID         | Lane | Manifest path                                       | Owner crate          |
|-----------------|------|-----------------------------------------------------|----------------------|
| `media`         | A    | `runtime/manifests/media.source.json`               | `video-runtime`      |
| `speech`        | A    | `runtime/manifests/silero-vad.source.json`          | `video-runtime`      |
| `speech-quality`| A    | `runtime/manifests/whisper-verifier.source.json`    | `video-runtime`      |
| `director`      | B    | `runtime/manifests/director.model.json`             | `video-inference`    |
| `voice`         | B    | `runtime/manifests/voice.model.json`                | `video-inference`    |
| `vision`        | C    | `schemas/evidence/` (graph + node + edge schemas)   | `video-evidence`     |
| `creative`      | C    | `crates/video-jobs/` (job DAG + fingerprint)        | `video-jobs`         |

## Workspace integration

Lanes A, B, and C do NOT modify the root `Cargo.toml`. The membership
table is updated by the sequential merge task `CR-V2-B3-022` after all
three lanes are committed. This is enforced by `scripts/architecture/check_crate_dag.py`.

The workspace `dependencies` table is append-only across all parallel
lanes. Lanes that need a new dependency declare it under their own
`Cargo.toml` and the merge task lifts it into the workspace table.

## Subcommand surface

Lanes add new subcommands through `video-cli` only in the merge task.
Lane A exposes `runtime probe <pack>`; lane B exposes `runtime model list`;
lane C exposes `jobs plan <fingerprint>` and `evidence query <graph>`. The
prefix `runtime` is shared so the user sees a unified surface.

## Acceptance

- Lanes A, B, and C own disjoint files.
- The forbidden-dependency list is enforced.
- The pack ID table is the only source of truth.
- New crate additions happen in the merge task, not the lanes.
