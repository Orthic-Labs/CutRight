# CR-V2-B1: — Malformed task-ID disposition note

This note documents the malformed task-ID commit and points at the existing
orchestrator-variance log entry that already explains it.

## What the dispatch expects

Task IDs follow the format `CR-V2-B<n>-<nnn>`. The validator will reject
`CR-V2-B1:` (no task number) as malformed.

## The commit in question

| SHA | Subject | Time | Files |
| --- | --- | --- | --- |
| `a80618c` | `CR-V2-B1: prepare-shared-import-tooling-for-parallel-lanes` | 2026-08-07 04:23:33 +0530 | 14 files: `tools/import-closure/{_common,assert_no_external_refs,hash_tree,import,import_selected,rewrite_refs,scan_assets,validate_asset_ledger,validate_clean_room,verify_copy,verify_exclusions,verify_notices}.py` + `tools/import-closure/src/main.rs` + `docs/dispatch/v2/book-1/orchestrator-variance.md` |

It landed between `CR-V2-B1-006` (04:10:53) and `CR-V2-B1-007` (04:38:59).
The commit built the Python helpers in `tools/import-closure/` that the
parallel lanes (A-007, B-012, C-017) invoke from their first task, and
recorded the variance in `orchestrator-variance.md`.

## Existing explanation

The variance log `docs/dispatch/v2/book-1/orchestrator-variance.md`
already records this case as **V-1**:

> *Shared import tooling built by the orchestrator, not by a lane task.*
> Lane task commands in the manifest invoke these scripts from their first
> task (A-007, B-012, C-017), but no task in any lane owns creating them.
> The Rust scanner crate is task CR-V2-B1-004 output; the Python helpers
> had no owning task.

The executor omitted the task number because the work fell between defined
task boundaries — it was preparatory tooling, not a single task.

## Audit response

The audit reading "malformed task ID — `CR-V2-B1:` missing number, violates
the format the validator will reject" was correct on the surface. The
deeper reading is that this was an orchestrator variance that fell between
task boundaries; the executor used a partial ID and then recorded the
deviation in `orchestrator-variance.md` V-1, exactly as AGENTS.md requires
for variances past ±10% or outside manifest task accounting.

The validator will reject the format; future dispatch tooling should be
taught to either (a) assign a free task slot (e.g. `CR-V2-B1-006-prep`)
or (b) use a non-task commit prefix (e.g. `v2: prepare-shared-import-...`)
when work falls between defined tasks. That is a future-process change,
not a backfill on this chain.

## Status

- Commit `a80618c`: malformed task ID, but logged in `orchestrator-variance.md` V-1
- Disposition: **no rebase, no rewrite**
- Future fix: executor should not emit malformed IDs; should either pick a
  valid slot or use a non-task prefix