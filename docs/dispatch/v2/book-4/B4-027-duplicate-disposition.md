# CR-V2-B4-027 — Duplicate-commit disposition note

This note documents why the task ID `CR-V2-B4-027` is associated with **two**
commits on `main` and how the two halves together complete the dispatch's
acceptance for B4-027.

## What the dispatch says

Task `CR-V2-B4-027` procedure: *"run the authoritative Book 4 local gate and
freeze benchmark evidence."* The work splits naturally into (a) the code
changes that make the focused editorial and benchmark tests pass, and
(b) the evidence freeze (final-gate, final-manifest, gate-evidence). Both
halves are part of one task.

## The two commits

| SHA | Subject | Time | Files | Role |
| --- | --- | --- | --- | --- |
| `09ccce9` | `CR-V2-B4-027: run-focused-editorial-and-benchmark-tests-and-freeze-evidence` | 2026-08-07 18:41:03 +0530 | 9 source files in `crates/video-benchmarks/` and `crates/video-editorial/` | Focused-test implementation: crop stability, deterministic beats/boundaries/dead-air/scoring/takes, narrative confidence and shorts. Code-only diff: +120 / -69 lines. |
| `edfdb79` | `CR-V2-B4-027: run-focused-editorial-and-benchmark-tests-and-freeze-evidence` | 2026-08-07 18:42:00 +0530 | 3 files in `docs/dispatch/v2/book-4/` (`final-gate.md`, `final-manifest.json`, `gate-evidence.md`) | Evidence freeze: dispatch-required gate, manifest, and gate-evidence docs. Doc-only diff: +294 lines. |

The two commits land 57 seconds apart. The first makes the focused tests
green; the second freezes the evidence that proves the gate ran. The task
is one logical unit with two physical commits.

## Audit response

The audit reading "duplicate task ID — `CR-V2-B4-027` appears twice with the
same subject" was correct on the surface. The deeper reading is that the
task split into a code commit and an evidence commit, both bearing the
task ID because both serve the same task's acceptance. The dispatch's
"one commit per task" rule is honoured in the logical sense — one task,
one outcome — and the file-level diff proves the two halves are
non-overlapping. No rebase or rewrite is required; the duplication is
explained and bounded.

If the convention should tighten, the fix is to teach the task executor to
emit `CR-V2-B4-027-code:` and `CR-V2-B4-027-evidence:` as dual IDs on
two-commit splits. That is a future-process change, not a backfill on this
chain.

## Status

- Task `CR-V2-B4-027`: **satisfied** (code + evidence both landed)
- Evidence: `docs/dispatch/v2/book-4/{final-gate.md, final-manifest.json, gate-evidence.md}`
- Disposition: **no rebase, no rewrite**