# CutRight Book 7 Interface Freeze

This document freezes the lane ownership, public names, and integration
boundaries for Book 7 "Measured Autonomy, Security Hardening, Offline
Distribution, and Release Acceptance." It is the merge contract for the
three parallel lanes (A, B, C) and the serial integration tasks 022–027.

## 1. Lane ownership

```text
lane_a: feedback + autonomy
lane_b: security + recovery
lane_c: local distribution + clean-machine QA
```

Each lane owns its exclusive paths exactly as listed in the dispatch
(`docs/dispatch/v2/source/CutRight-v2-Dispatch-Book-07.md`). Parallel
lanes do not overlap:

- Lane A owns `crates/video-feedback/**`, `schemas/feedback/**`,
  `apps/studio/src/contracts/feedback.ts`, and the `FormatProfile`/
  `Autonomy` Studio panels plus `autonomous_run` integration.
- Lane B owns `crates/video-security/**`, `crates/video-recovery/**`,
  `crates/video-project/src/trust.rs`, `apps/studio/src/components/TrustPanel.tsx`,
  `apps/studio/src-tauri/src/privacy_settings.rs`, `apps/studio/src-tauri/src/pack_commands.rs`,
  and `PackManagerMode`.
- Lane C owns `scripts/release/**`, `release/v2/**`, `samples/v2/**`,
  `docs/user/v2/**`, `scripts/qa/v2-clean-machine/**`, and the source
  bundle.

Lanes may read shared files only when the read is a lockfile update
deterministically produced by the listed command. Writes outside the
exclusive list are merge conflicts resolved against this document.

## 2. Shared integration paths

The following paths are reserved for the serial integration tasks 022–027:

- `crates/video-state/src/migrations/v2.rs` (B7-022)
- `crates/video-project/src/legacy.rs` (B7-022)
- `apps/studio/src/modes/MigrationMode.tsx` (B7-022)
- `fixtures/migrations/v1-to-v2/**` (B7-022)
- `docs/dispatch/v2/book-7/merge-receipt.md` (B7-022)
- `benchmarks/runs/v2-release-candidate/**` (B7-023)
- `release/v2/acceptance/**` (B7-023)
- `docs/release/V2-FOUR-LANE-RESULTS.md` (B7-023)
- `release/v2/audit/**` (B7-024)
- `docs/release/V2-RELEASE-AUDIT.md` (B7-024)
- `release/v2/SBOM.spdx.json`, `release/v2/provenance.json`,
  `release/v2/THIRD-PARTY-NOTICES.md` (B7-025)
- `docs/release/V2-DISCLOSURE.md` (B7-025)
- `scripts/release/v2-provenance.py` (B7-025)
- `release/v2/rc/**` (B7-026)
- `release/v2/RC-MANIFEST.json` (B7-026)
- `docs/release/V2-RC-REPORT.md` (B7-026)
- `docs/dispatch/v2/book-7/final-gate.md` (B7-027)
- `docs/dispatch/v2/book-7/final-manifest.json` (B7-027)
- `release/v2/SHA256SUMS.txt` (B7-027)

Lanes do **not** write to these paths.

## 3. Frozen public names

The following public names are frozen for Book 7 and may not be renamed
inside parallel lanes:

- Crates: `video-feedback`, `video-security`, `video-recovery`.
- Schemas:
  - `schemas/feedback/decision.schema.v2.json`
  - `schemas/feedback/preferences.schema.v2.json`
  - `schemas/feedback/autonomy.schema.v2.json`
  - `schemas/feedback/format-profile.schema.v1.json`
  - `schemas/security/event.schema.v1.json`
  - `schemas/release/bundle-manifest.schema.v1.json`
  - `schemas/release/update-manifest.schema.v1.json`
  - `schemas/release/rollback.schema.v1.json`
  - `schemas/release/acceptance-result.schema.v1.json`
  - `schemas/release/clean-machine-result.schema.v1.json`
  - `schemas/migrations/project-compatibility.schema.v1.json`
  - `schemas/recovery/recovery-report.schema.v1.json`
- Commands: `scripts/release/v2-build.py`, `scripts/release/v2-sign.py`,
  `scripts/release/v2-seal.py`, `scripts/release/v2-assemble-offline.py`,
  `scripts/release/v2-source-bundle.py`, `scripts/release/v2-provenance.py`,
  `scripts/release/v2-audit.py`, `scripts/release/validate-samples.py`,
  `scripts/qa/v2-clean-machine/run.py`.
- Studio components: `FormatProfilePanel.tsx`, `AutonomyPanel.tsx`,
  `TrustPanel.tsx`, `PackManagerMode.tsx`, `MigrationMode.tsx`.

A merge conflict against a frozen name is resolved by re-reading this
document; the lane that proposes the rename loses.

## 4. Lane A — feedback + autonomy

Lane A owns the per-format autonomy loop. Outputs:

- `crates/video-feedback` (decision, learn, distributions, profile,
  autonomy)
- `crates/video-project/src/autonomous_run.rs`
- `crates/video-jobs/src/autonomous.rs`
- Studio `FormatProfilePanel`, `AutonomyPanel`, `feedback.ts` contract

Hard floor: autonomy cannot alter security/integrity floors. The advance
predicate is `thresholds_met && user_approval_present`; the demotion
predicate is `any(regression_triggers)`. No code path self-approves
advancement.

## 5. Lane B — security + recovery

Lane B owns the trust boundary, recovery, and pack repair. Outputs:

- `crates/video-security` (sandbox, media_limits, trust, privacy)
- `crates/video-recovery` (scan, repair)
- `crates/video-project/src/trust.rs`
- `apps/studio/src/components/TrustPanel.tsx`
- `apps/studio/src-tauri/src/privacy_settings.rs`,
  `apps/studio/src-tauri/src/pack_commands.rs`
- `apps/studio/src/modes/PackManagerMode.tsx`
- `docs/security/V2-PRIVACY.md`

Hard floor: no external byte crosses a process boundary without a
validator and a sandbox grant. Network is denied by release policy;
telemetry is off.

## 6. Lane C — local distribution + clean-machine QA

Lane C owns the offline bundle, source bundle, samples, and clean-machine
harness. Outputs:

- `scripts/release/v2-{build,sign,seal,assemble-offline,source-bundle,provenance,audit}.py`
- `scripts/release/validate-samples.py`
- `scripts/qa/v2-clean-machine/**`
- `release/v2/{bundle-manifest.json,layout/**,source-manifest.json,
  sample-manifest.json}`
- `samples/v2/**`
- `docs/user/v2/**`
- `docs/release/V2-{LOCAL-RELEASE,OFFLINE-BUNDLE-CONTENTS,SOURCE-BUNDLE,
  CLEAN-MACHINE-HARNESS}.md`

Hard floor: **no upload capability.** Lane C produces a local release
candidate; the upload step is outside the dispatch and is not part of
release acceptance. The clean-machine harness runs with `PATH` empty and
network denied.

## 7. Serial integration tasks 022–027

The serial integration tasks consolidate all three lanes:

- B7-022 — Merge lanes and integrate v1-to-v2 project migration.
- B7-023 — Final four-lane benchmark + Studio acceptance on supported
  targets.
- B7-024 — Final security, privacy, licence, and supply-chain release
  audit.
- B7-025 — Final SBOM, provenance graph, and release disclosure.
- B7-026 — Build and seal local release candidate.
- B7-027 — Final authoritative local gate, clean-machine proof, and
  checksum seal.

These tasks may write to any path. Lane A/B/C commits are unchanged.

## 8. Autonomous mode invariant

`autonomous` requires `critic_pass && deterministic_qa_pass &&
no_blocking_escalation`. A blocking escalation downgrades the run to
`review`. A failed stage leaves the previous good revision reviewable;
the system never overwrites the last approved final.

## 9. Network invariant

Lane C has no network publish capability. Lane A and Lane B do not
perform network publish either. The clean-machine harness uses
`network_deny` and emits a `network_attempt_total` of zero.

## 10. Forbidden

- Renaming a frozen public name inside a lane.
- Editing a shared path from a lane.
- Reading a sibling repository, global skill directory, or `PATH` from
  release code.
- Self-approving autonomy advancement.
- Uploading anything from a release script.
- Weakening a test, threshold, or sandbox to close a task.
