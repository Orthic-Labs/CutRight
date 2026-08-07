# CR-V2-B2-027 — Book 2 gate evidence

This file is the single-point summary of the Book 2 local gate. It is
created alongside `final-gate.md` and `final-manifest.json` so any
reviewer can audit the closure of Book 2 without re-running the gate.

## Book

- **book**: 2
- **commit**: see `git log --oneline -1` after the B2-027 commit
- **command**: `CR-V2-B2-027`

## Required invariant

- `all mutation surfaces use video-executor`

## Outcome

| Check | Status |
|---|---|
| `bash scripts/gates/v2-capability-drift.sh` | pass |
| `python3 scripts/architecture/check_crate_dag.py docs/architecture/V2-CRATE-DAG.md` | pass |
| `bash scripts/gates/v2-repository-shape.sh` | pass |
| `bash scripts/gate.sh --with-qa` | pass |

## Evidence pointers

- `docs/dispatch/v2/book-2/final-gate.md` — narrative gate closure.
- `docs/dispatch/v2/book-2/final-manifest.json` — machine-readable manifest.
- `docs/dispatch/v2/book-2/focused-tests.md` — focused cross-surface tests.
- `crates/video-agent/src/mcp.rs` — loopback-only MCP adapter.
- `crates/video-agent/src/tools.rs` — generated MCP tool registry.
- `crates/video-agent/tests/mcp.rs` — loopback guard, frontmost-project guard, mutation routing.
- `tests/v2/action_surfaces.rs` — Rust cross-surface parity suite.
- `apps/studio/src/action-contract.test.ts` — Studio Tauri parity suite.

## Constraints

- CI is forbidden. All evidence is local.
- No publication, signed release or external upload occurs in this book.
- The shared `video-executor` is the only mutation path; every surface
  routes through it.
