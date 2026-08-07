# CR-V2-B3-027 — Book 3 gate evidence

This file is the single-point summary of the Book 3 local gate. It is
created alongside `final-gate.md` and `final-manifest.json`.

## Book

- **book**: 3
- **commit**: see `git log --oneline -1` after the B3-027 commit
- **command**: `CR-V2-B3-027`

## Required invariants

- `network_attempts: 0`
- `path_fallbacks: 0`
- `ci: forbidden`

## Outcome

| Check | Status |
|---|---|
| `python3 scripts/gates/v2-runtime-boundary.py --check` | pass |
| `python3 scripts/legal/validate-v2-ledger.py --scope book-3` | pass |
| `bash scripts/qa/v2-clean-runtime.sh` | pass |
| `bash scripts/gate.sh --with-qa` | pass |

## Evidence pointers

- `docs/dispatch/v2/book-3/final-gate.md` — narrative gate closure.
- `docs/dispatch/v2/book-3/final-manifest.json` — machine-readable manifest.
- `docs/dispatch/v2/book-3/merge-receipt.md` — merge receipt and façade.
- `docs/dispatch/v2/book-3/clean-runtime.md` — clean-path smoke test.
- `docs/dispatch/v2/book-3/focused-tests.md` — focused pack/evidence/job tests.
- `crates/video-jobs/src/{dag,store,runner}.rs` — content-addressed job DAG.
- `crates/video-services/src/{lib,runtime,evidence,jobs}.rs` — service façade.
- `crates/video-runtime/src/{doctor,repair}.rs` — pack doctor and repair.
- `crates/video-media/src/toolchain.rs` — `PackResourceResolver`.
- `crates/video-providers/src/lib.rs` — provider re-exports.
- `scripts/gates/v2-runtime-boundary.py` — runtime boundary gate.
- `scripts/qa/v2-clean-runtime.sh` — clean-path smoke harness.
- `tests/v2/clean_runtime.rs` — clean-path smoke test.

## Constraints

- No CI. All evidence is local.
- No release publication. Pack locks are recorded but no signed public
  release is produced.
- Network attempts must be zero on the clean-path runtime.
- Path fallbacks (system PATH, env-var override) must be zero in
  release builds.
