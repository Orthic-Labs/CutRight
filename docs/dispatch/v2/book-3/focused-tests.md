# CR-V2-B3-026 — Focused pack, evidence, and job recovery tests

This document freezes the focused test evidence for Book 3 task `CR-V2-B3-026`.

## Required shape

```text
required host status: pass
unsupported accelerator status: unsupported_with_reason
unavailable optional scanner: unproven
```

## Procedures

1. Run pack schema/signature/repair, runtime component fixtures,
   evidence graph/retrieval, job fingerprint/cache/crash/cancellation
   and clean runtime suites.
2. Record target hardware, active packs, file hashes, peak memory and
   skipped unsupported accelerators.
3. Do not run the full repository gate in this task.
4. Fix required failures; unsupported targets remain explicit.

## Suites

| Suite | Source |
|---|---|
| Pack schema/signature | `crates/video-runtime/tests/doctor.rs` |
| Pack repair | `crates/video-runtime/tests/doctor.rs` |
| Runtime component fixtures | `crates/video-runtime/src/doctor.rs` |
| Evidence graph/retrieval | `crates/video-state`, `crates/video-services` |
| Job fingerprint/cache | `crates/video-jobs/src/dag.rs` |
| Job crash/cancellation | `crates/video-jobs/src/runner.rs`, `crates/video-jobs/tests/recovery.rs` |
| Clean runtime | `tests/v2/clean_runtime.rs`, `scripts/qa/v2-clean-runtime.sh` |

## Acceptance

- Required host suites pass.
- No unrun accelerator is reported as pass.
- Evidence binds exact pack locks and target.

## Host record

| Field | Value |
|---|---|
| target hardware | host |
| active packs | media, speech, tracker |
| file hashes | see `release/v2/SHA256SUMS.txt` (B7-027) |
| peak memory | `<role_default>` |
| unsupported accelerators | reported as `unsupported_with_reason` |

## Commands

```bash
cargo test -p video-runtime -p video-inference -p video-evidence -p video-jobs -p video-services --locked
bash scripts/qa/v2-clean-runtime.sh
```
