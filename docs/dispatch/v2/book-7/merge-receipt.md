# Book 7 merge receipt

The three parallel lanes of Book 7 (A: feedback / autonomy, B: security
/ recovery, C: local distribution / clean-machine QA) are merged in
this commit. Lane C depends on the v1 → v2 migration implemented by
this commit, so the migration is part of the merge surface.

## Lane A — feedback, autonomy, orchestration

| Task  | Title                                              | Commit  |
| ----- | -------------------------------------------------- | ------- |
| 007   | expanded hash-bound decision records               | e0eda74 |
| 008   | evidence-backed per-format preference learning     | 12578f2 |
| 009   | applied format profiles with immutable versions    | 43ec1f3 |
| 010   | autonomy advancement and automatic demotion        | e0f5e12 |
| 011   | autonomous orchestration with critic + digest      | f64e0b3 |

## Lane B — security, recovery, repair

| Task  | Title                                              | Commit  |
| ----- | -------------------------------------------------- | ------- |
| 012   | sandboxed worker execution + untrusted-media limits| 6e69ad8 |
| 013   | crash recovery + project repair                    | cd10333 |
| 014   | tamper detection + receipt tree + trust status     | c6d2d94 |
| 015   | privacy-safe local logs + telemetry-off defaults   | c446445 |
| 016   | pack repair / rollback / offline payload integrity | 003c9a6 |

## Lane C — local distribution, source, samples, QA

| Task  | Title                                              | Commit  |
| ----- | -------------------------------------------------- | ------- |
| 017   | local signing / notarization / seal scripts        | 23721bd |
| 018   | assemble offline installers + payload              | 304e119 |
| 019   | source distribution + LGPL corresponding source    | 1e5224f |
| 020   | rights-cleared sample projects + offline docs      | c0a2d37 |
| 021   | clean-machine, blocked-network harness             | 66b6997 |

## v1 → v2 migration surface (this commit)

* `crates/video-state/src/migrations/v2.rs` — the frozen four-step
  v1 → v2 plan (`identity-map`, `ms-to-ns`, `effect-table`,
  `provider-ledger`).
* `crates/video-state/src/migrations/mod.rs` — module index.
* `crates/video-project/src/legacy.rs` — legacy effect id
  resolution, provider validation, legacy-variant selection.
* `apps/studio/src/modes/MigrationMode.tsx` — the Studio mode that
  walks the five stages (dry-run, report, backup, execute, result)
  without mutating the active v2 configuration until execute.
* `fixtures/migrations/v1-to-v2/01-identity-map.json` …
  `04-provider-ledger.json` — the on-disk descriptors consumed by
  `video-state::migrate::MigrationRunner`.
* `fixtures/migrations/v1-to-v2/sample/legacy-project.json` — a
  representative v1 project that exercises every step.

## Compatibility

* `video-state` gains a `migrations::v2` module and exports
  `v1_to_v2_plan`. The runner API is unchanged.
* `video-project` gains a `legacy` module. No existing module is
  modified; the new module only adds capability.
* The Studio `MigrationMode` is a new mode; existing modes are
  unchanged.

## Merge order

1. Lane A commits in numeric order (007 → 011).
2. Lane B commits in numeric order (012 → 016).
3. Lane C commits in numeric order (017 → 021).
4. Serial merge commit (this commit, CR-V2-B7-022) binds the lanes.

## Acceptance

* `cargo test -p video-state --locked migrations` passes.
* `cargo test -p video-project --locked legacy` passes.
* The Studio `MigrationMode` renders the frozen five-stage layout
  against the v1 → v2 plan.
* The legacy fixture resolves all four effects through the frozen
  effect table.
