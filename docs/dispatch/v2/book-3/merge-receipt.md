# CR-V2-B3-022 — Merge receipt and service façade

This document freezes the merge order and the service façade invariants for
Book 3 task `CR-V2-B3-022`.

## Lanes merged

| Lane | Commit range | Crate |
|---|---|---|
| A | `CR-V2-B3-001..010` | `video-runtime` |
| B | `CR-V2-B3-011..020` | `video-evidence` |
| C | `CR-V2-B3-021` | `video-jobs` |

The merge is deterministic: A is the base, B is replayed on top, and C is
finally applied. Each lane was committed on its own branch segment so the
topology is recoverable.

## Conflicts

No merge conflicts were reported. The lanes are disjoint at the file level
as outlined in the exclusive ownership lists for each task.

## Façade

```text
pub struct VideoServices {
    pub packs: PackService,
    pub evidence: EvidenceService,
    pub jobs: JobService,
    pub inference: InferenceService,
}
```

The façade returns stable IDs and capability handles. Raw mutable handles
to the runtime file system are never exposed.

## Acceptance

- Service façade returns stable IDs/capabilities, not raw mutable handles.
- No dependency cycle exists.
- Merge receipt is complete.

## Commands

```bash
cargo check -p video-services --locked
python3 scripts/architecture/check_crate_dag.py docs/architecture/V2-RUNTIME-EVIDENCE-JOB-DAG.md
```
