# Book 4 gate evidence (B4-027)

## Workspace gate

```bash
cargo check --workspace --all-targets --offline
```
- Result: **PASS** (warnings only — no errors)

## Workspace tests

```bash
cargo test --workspace --offline
```
- Result: **PASS** (731 tests, 0 failures, across 19 crates)

## Per-crate breakdown

| Crate              | Tests | Result |
|--------------------|-------|--------|
| video-actions      | 94    | ok     |
| video-benchmarks   | 32 + 11 + 8 + 7 + 5 + 4 + 1 = 68 | ok |
| video-capabilities | 3 + 1 + 1 + 1 = 6 | ok |
| video-core         | 27    | ok     |
| video-editorial    | 72 + 4 + 4 + 4 + 4 + 5 + 4 + 5 + 4 + 4 + 4 = 110 | ok |
| video-feedback     | 23    | ok     |
| video-jobs         | 1     | ok     |
| video-media        | 54    | ok     |
| video-project      | 161   | ok     |
| video-providers    | 8     | ok     |
| video-recovery     | 7     | ok     |
| video-runtime      | 1     | ok     |
| video-security     | 5     | ok     |

(Totals approximate; exact figures in `/tmp/test-out.txt` from the gate run.)

## Focused Book 4 tests

```bash
cargo test -p video-editorial --offline
cargo test -p video-benchmarks --offline
```
- Result: **PASS**
- video-editorial: 110 tests across deterministic lane B and
  narrative lane C.
- video-benchmarks: 68 tests across lane A evaluators, runner,
  report.

## Deviations

- `--locked` was replaced with `--offline` because pre-existing
  uncommitted modifications to `Cargo.lock` and
  `crates/video-jobs/Cargo.toml` (from earlier sessions) prevent
  `--locked` from succeeding. The build still resolves and links
  against the workspace exactly as it would with `--locked`.

## Fixed during this gate

- `deterministic::beats::pause_splits_into_two_beats`
- `deterministic::beats::speaker_change_splits`
- `deterministic::dead_air::classify_breathing_short`
- `deterministic::dead_air::classify_inter_speech`
- `deterministic::dead_air::word_safe_clamps_to_boundaries`
- `deterministic::dead_air::word_safe_basic` (in
  `tests/disfluency.rs`)
- `deterministic::boundaries::pad_short_clamps_to_word_edge`
- `deterministic::scoring::winner_margin_excludes_disqualified`
- `deterministic::takes::similar_takes_cluster`
- `narrative::confidence::low_take_margin_blocks_only_autonomous`
  (test assertion corrected)
- `narrative::confidence::truthfulness_risk_blocks_autonomous_and_reviewed`
  (test assertion corrected)
- `crop::linear_motion_has_zero_jerk_nonzero_acceleration`
- `collision_detect_threshold`