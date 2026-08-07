# CutRight v2 Dispatch Book 7: Measured Autonomy, Security Hardening, Offline Distribution, and Release Acceptance

**Tasks:** 27  
**Goal:** Turn review evidence into bounded per-format autonomy, harden the local product, migrate existing projects, and prove signed offline installers on clean machines without CI or external dependencies.  
**Execution default:** A single agent runs numeric order. The labels show safe parallelism for an authorised dispatcher with isolated checkouts; this document does not itself authorise new branches or worktrees.  
**Testing cadence:** Focused checks run inside tasks. The full authoritative local gate runs only in task `CR-V2-B7-027`.  
**CI rule:** Do not create GitHub Actions, CI YAML, hosted checks, or workflow files.

## Agent operating rules

1. Execute tasks in numeric order unless an authorised dispatcher assigns the three explicit parallel lanes after task 006.
2. `[S]` is sequential. `[P-A]`, `[P-B]`, and `[P-C]` are independent lanes; each lane is internally sequential.
3. One task equals one commit with the exact message in the task.
4. Parallel workers may edit only their exclusive paths. Shared manifests and integrations belong to tasks 022–027.
5. Use exact names, schemas, paths, model/source revisions and commands. Do not substitute a different dependency or architecture.
6. Stop when an exact required source, licence, model byte, capability, fixture, pack or credential is unavailable. Emit the named blocked/unproven state; do not invent it.
7. Preserve source immutability, receipts, revision history, compatibility, prior finals and unrelated changes.
8. Production code may not read a sibling repository, global skill directory, bare executable from `PATH`, user Python/Node environment, Ollama, cloud service or downloaded browser.
9. No task may add a Git submodule, symlinked skill, `.github/workflows/`, or hosted release automation.
10. Do not weaken a test, threshold, licence rule, sandbox or security gate to close a task.
11. A merge conflict is resolved against the Book interface-freeze document. Frozen public names do not change inside parallel lanes.
12. Finish every task with a clean commit before a dependent task starts.

## Parallelization map

```text
CR-V2-B7-001 .. 006    sequential contract/interface freeze
CR-V2-B7-007 .. 011    parallel lane A
CR-V2-B7-012 .. 016    parallel lane B
CR-V2-B7-017 .. 021    parallel lane C
CR-V2-B7-022 .. 027    sequential merge, integration, acceptance, gate
```

## CR-V2-B7-001 [S] — Freeze feedback, preference evidence, and per-format autonomy schemas

**Depends on:** Book 6 final gate  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B7-001: freeze-feedback-preference-evidence-and-per-format-autonom`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `schemas/feedback/decision.schema.v2.json`
- `schemas/feedback/preferences.schema.v2.json`
- `schemas/feedback/autonomy.schema.v2.json`
- `docs/architecture/V2-FEEDBACK-AUTONOMY.md`

**Procedure**

1. Expand reason vocabulary for take choice, boundaries, filler, pause, hook/CTA, beat order, crop, caption, graphic, effect density, B-roll, SFX, music, colour, audio, identity and final verdict.
2. Define evidence-backed preference distributions and source decision references.
3. Define format key `content_type × platform × variant`, review mode, compatible pack/profile set, sample counts, metrics, advancement and demotion.
4. Keep user-specific preference evidence separate from shared benchmark floors.

**Required implementation shape**

```text
FormatKey { content_type, platform, variant }
AutonomyState { mode, compatible_pack_set, benchmark_profile, sample_count, metrics, demoted, last_user_approval }
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/feedback/decision.schema.v2.json fixtures/schemas/feedback/decision/v2/valid/basic.json
python3 scripts/schema-check.py schemas/feedback/autonomy.schema.v2.json fixtures/schemas/feedback/autonomy/v2/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every preference cites decisions/projects.
- Unknown reason or unsupported axis is explicit.
- New format/pack/profile begins reviewed.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-001: freeze-feedback-preference-evidence-and-per-format-autonom`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-002 [S] — Freeze the security and privacy threat model

**Depends on:** CR-V2-B7-001  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B7-002: freeze-the-security-and-privacy-threat-model`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/security/V2-THREAT-MODEL.md`
- `schemas/security/event.schema.v1.json`
- `config/security/release-policy.json`

**Procedure**

1. Model malicious media, crafted project packages, untrusted skill/model/pack files, path traversal, decompression bombs, process abuse, prompt injection in transcripts/assets, MCP misuse, tampered updates and privacy leakage.
2. Define trust boundaries, data flows, secrets policy, network policy, filesystem scope, process sandbox, resource limits and audit events.
3. Default telemetry and cloud/network access off; no API-key UI required for core product.
4. Define safe recovery and user-visible degradation.

**Required implementation shape**

```text
trust levels: immutable_source | canonical_project | verified_pack | imported_untrusted | generated_untrusted | external_session
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/security/event.schema.v1.json fixtures/schemas/security/event/v1/valid/basic.json
python3 -m json.tool config/security/release-policy.json >/dev/null
```

**Acceptance — inspect and run only the listed focused checks**

- Every external byte crosses a validator/sandbox boundary.
- Core operation needs no secret.
- Network disabled is an enforced release policy, not preference only.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-002: freeze-the-security-and-privacy-threat-model`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-003 [S] — Freeze installer, bundle, pack, update, and rollback architecture

**Depends on:** CR-V2-B7-002  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B7-003: freeze-installer-bundle-pack-update-and-rollback-architect`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/release/V2-DISTRIBUTION.md`
- `schemas/release/bundle-manifest.schema.v1.json`
- `schemas/release/update-manifest.schema.v1.json`
- `schemas/release/rollback.schema.v1.json`

**Procedure**

1. Define base app plus complete offline bundle and optional separately signed quality pack.
2. Define target-specific installer contents, pack locks, notices, source archives, sample projects, repair payload and checksums.
3. Define local update/rollback verification without requiring hosted updater for acceptance.
4. Keep build, sign, package, seal and upload as separate local actions; upload is outside this dispatch.

**Required implementation shape**

```text
bundle = app + packs + licences + corresponding_source + sample_projects + repair_payload + checksums + signatures
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/release/bundle-manifest.schema.v1.json fixtures/schemas/release/bundle-manifest/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- A bundle is self-describing and verifiable offline.
- Rollback retains compatible project migrations or warns before opening.
- No upload/publish step is part of release acceptance.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-003: freeze-installer-bundle-pack-update-and-rollback-architect`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-004 [S] — Freeze migration, backup, recovery, and compatibility policy

**Depends on:** CR-V2-B7-003  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B7-004: freeze-migration-backup-recovery-and-compatibility-policy`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/architecture/V2-MIGRATION-RECOVERY.md`
- `schemas/migrations/project-compatibility.schema.v1.json`
- `schemas/recovery/recovery-report.schema.v1.json`

**Procedure**

1. Define v1 CutRight project migration, legacy skill/finish artifacts, external provider records, Remotion effect IDs and current Studio decisions.
2. Back up before migration, preserve original sources and prior finals, and map legacy variants to immutable revisions.
3. Define recovery for interrupted actions/jobs, corrupt index, missing pack, tampered receipt and partial installer repair.
4. Reject destructive downgrade when schema or pack incompatibility exists.

**Required implementation shape**

```text
legacy project → backup manifest → dry-run report → staged migration → validate/receipts → active v2 revision; original backup retained
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/recovery/recovery-report.schema.v1.json fixtures/schemas/recovery/recovery-report/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every legacy canonical artefact has a migration/disposition.
- Recovery never modifies source bytes.
- Compatibility failures name exact required pack/app version.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-004: freeze-migration-backup-recovery-and-compatibility-policy`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-005 [S] — Freeze release acceptance matrix and supported-target claims

**Depends on:** CR-V2-B7-004  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B7-005: freeze-release-acceptance-matrix-and-supported-target-clai`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/release/V2-ACCEPTANCE-MATRIX.md`
- `schemas/release/acceptance-result.schema.v1.json`
- `config/release/targets.json`

**Procedure**

1. Define required targets, packs, lanes, features, benchmark profiles, clean-machine conditions, security checks and installer types.
2. A claim is supported only for a target with passing installer, runtime pack, benchmark and workflow results.
3. Separate source-build/headless support from desktop release claims.
4. Record unsupported targets explicitly.

**Required implementation shape**

```text
target claim = installer_pass ∧ clean_machine_pass ∧ required_packs_pass ∧ four_lane_pass ∧ benchmark_floors_pass ∧ security_pass
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/release/acceptance-result.schema.v1.json fixtures/schemas/release/acceptance-result/v1/valid/basic.json
python3 -m json.tool config/release/targets.json >/dev/null
```

**Acceptance — inspect and run only the listed focused checks**

- No global “cross-platform” claim can hide target gaps.
- Every target row names exact pack locks.
- Unsupported targets are excluded from installer output.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-005: freeze-release-acceptance-matrix-and-supported-target-clai`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-006 [S] — Freeze Book 7 autonomy, security/recovery, and distribution lane ownership

**Depends on:** CR-V2-B7-005  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B7-006: freeze-book-7-autonomy-security-recovery-and-distribution-`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-7/interface-freeze.md`
- `docs/architecture/V2-RELEASE-DAG.md`

**Procedure**

1. Assign lane A feedback/preferences/autonomy/orchestration; lane B sandbox/recovery/tamper/privacy/repair; lane C signing/installers/source bundles/samples/clean-machine harness.
2. Reserve migration merge, final four-lane acceptance, release audit, RC build and authoritative gate for serial tasks.
3. Freeze release manifest and acceptance result APIs.
4. Prohibit lane C from uploading or publishing.

**Required implementation shape**

```text
lane_a: feedback + autonomy
lane_b: security + recovery
lane_c: local distribution + clean-machine QA
```

**Commands for this task**

```bash
python3 scripts/architecture/check_crate_dag.py docs/architecture/V2-RELEASE-DAG.md
```

**Acceptance — inspect and run only the listed focused checks**

- Parallel roots do not overlap.
- Autonomy cannot alter security/integrity floors.
- Distribution lane has no network publish capability.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-006: freeze-book-7-autonomy-security-recovery-and-distribution-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-007 [P-A] — Implement expanded hash-bound decision records

**Depends on:** CR-V2-B7-006  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B7-007: implement-expanded-hash-bound-decision-records`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-feedback/Cargo.toml`
- `crates/video-feedback/src/lib.rs`
- `crates/video-feedback/src/decision.rs`
- `crates/video-feedback/tests/decision.rs`
- `apps/studio/src/contracts/feedback.ts`

**Procedure**

1. Create target-specific reason enums and structured deltas for every frozen preference axis.
2. Bind decisions to project instance, revision, subject/action/asset/effect/final hashes, format, pack set, app version and user/session origin.
3. Append through the existing hash-chained log and preserve malformed records.
4. Add Studio controls for exact reasons without forcing a note.

**Required implementation shape**

```text
DecisionTarget = Segment | Beat | Take | Boundary | Caption | Graphic | Effect | Audio | Crop | Final
```

**Commands for this task**

```bash
cargo test -p video-feedback --locked decision
pnpm --dir apps/studio test -- --run feedback
```

**Acceptance — inspect and run only the listed focused checks**

- Every preference axis can be distinguished.
- A stale/mismatched subject hash is retained but excluded from learning.
- No record silently drops.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-007: implement-expanded-hash-bound-decision-records`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-008 [P-A] — Implement evidence-backed per-format preference learning

**Depends on:** CR-V2-B7-007  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B7-008: implement-evidence-backed-per-format-preference-learning`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-feedback/src/learn.rs`
- `crates/video-feedback/src/distributions.rs`
- `crates/video-feedback/tests/learn.rs`

**Procedure**

1. Aggregate only compatible, hash-valid decisions by format and pack/profile set.
2. Compute distributions, confidence, recency, variance, sample count and cited decision IDs for pause/filler/take/hook/caption/graphic/motion/audio/crop/final axes.
3. Return unsupported/insufficient rather than inventing a preference.
4. Write recommendations separately from applied profile.

**Required implementation shape**

```text
PreferenceEstimate<T> { distribution, confidence, sample_count, variance, evidence_decision_ids, compatibility_fingerprint }
```

**Commands for this task**

```bash
cargo test -p video-feedback --locked learn
```

**Acceptance — inspect and run only the listed focused checks**

- Every recommendation cites evidence.
- A single project cannot produce a stable preference.
- Conflicting decisions widen uncertainty or require review.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-008: implement-evidence-backed-per-format-preference-learning`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-009 [P-A] — Implement applied format profiles with immutable versions

**Depends on:** CR-V2-B7-008  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B7-009: implement-applied-format-profiles-with-immutable-versions`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-feedback/src/profile.rs`
- `schemas/feedback/format-profile.schema.v1.json`
- `crates/video-feedback/tests/profile.rs`
- `apps/studio/src/components/FormatProfilePanel.tsx`

**Procedure**

1. Create explicit user-approved profile versions from recommendations.
2. Bind profile to content type, platform, variant, pack set, benchmark profile and skill/render versions.
3. Expose inherited defaults and overridden values separately.
4. Never auto-apply an unapproved recommendation in reviewed mode.

**Required implementation shape**

```text
FormatProfile { format, version, compatibility, values, source_recommendation_hash, approved_by, approved_at }
```

**Commands for this task**

```bash
cargo test -p video-feedback --locked profile
pnpm --dir apps/studio test -- --run FormatProfilePanel
```

**Acceptance — inspect and run only the listed focused checks**

- Profile changes are new immutable versions.
- Compatibility mismatch blocks application.
- User can inspect source decisions for each setting.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-009: implement-applied-format-profiles-with-immutable-versions`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-010 [P-A] — Implement autonomy advancement and automatic demotion

**Depends on:** CR-V2-B7-009  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B7-010: implement-autonomy-advancement-and-automatic-demotion`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-feedback/src/autonomy.rs`
- `crates/video-feedback/tests/autonomy.rs`
- `apps/studio/src/components/AutonomyPanel.tsx`

**Procedure**

1. Compute metrics from compatible benchmark runs, editorial plans, QA and user decisions.
2. Allow advancement only after thresholds and an explicit user approval action.
3. Automatically demote on rejected final, unresolved escalation, benchmark regression, critic disagreement, integrity failure, or incompatible pack/profile change.
4. Write every transition as an audit record.

**Required implementation shape**

```text
advance = thresholds_met && user_approval_present; demote = any(regression_triggers)
```

**Commands for this task**

```bash
cargo test -p video-feedback --locked autonomy
pnpm --dir apps/studio test -- --run AutonomyPanel
```

**Acceptance — inspect and run only the listed focused checks**

- New formats start reviewed.
- No code path self-approves advancement.
- Demotion is immediate and evidence-bound.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-010: implement-autonomy-advancement-and-automatic-demotion`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-011 [P-A] — Implement autonomous orchestration with mandatory critic and digest

**Depends on:** CR-V2-B7-010  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B7-011: implement-autonomous-orchestration-with-mandatory-critic-a`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-project/src/autonomous_run.rs`
- `crates/video-jobs/src/autonomous.rs`
- `crates/video-project/tests/autonomous_run.rs`

**Procedure**

1. Resolve effective mode from format/profile/pack compatibility and current escalations.
2. In autonomous mode run complete editorial/creative/render/QA pipeline without intermediate approval, but require independent critic and all deterministic floors.
3. Write ready/needs-review/failed digest with confidence, QA, finals, escalations, cache and packs.
4. Never publish, delete alternatives or overwrite last approved final.

**Required implementation shape**

```text
effective_mode → DAG policy; autonomous requires critic_pass && deterministic_qa_pass && no_blocking_escalation
```

**Commands for this task**

```bash
cargo test -p video-project -p video-jobs --locked autonomous_run
```

**Acceptance — inspect and run only the listed focused checks**

- Blocking escalation downgrades the run.
- Ready means awaiting final visual sign-off.
- Failed stages leave reviewable last-good revision.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-011: implement-autonomous-orchestration-with-mandatory-critic-a`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-012 [P-B] — Implement sandboxed worker execution and untrusted-media limits

**Depends on:** CR-V2-B7-006  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B7-012: implement-sandboxed-worker-execution-and-untrusted-media-l`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-security/Cargo.toml`
- `crates/video-security/src/lib.rs`
- `crates/video-security/src/sandbox.rs`
- `crates/video-security/src/media_limits.rs`
- `crates/video-security/tests/sandbox.rs`

**Procedure**

1. Run media/model/helper workers with minimal environment, scoped paths, process tree control, time/output/temp/resource limits and platform sandbox primitives where available.
2. Validate container/stream dimensions, durations, counts, decompression ratios and metadata sizes before expensive decode.
3. Treat model/skill generated files as untrusted until validated.
4. Return typed unsupported when a required sandbox guarantee cannot be met on a target.

**Required implementation shape**

```text
WorkerGrant { executable_hash, readable_files, writable_dir, env_allowlist, limits, network: Denied }
```

**Commands for this task**

```bash
cargo test -p video-security --locked sandbox
```

**Acceptance — inspect and run only the listed focused checks**

- Path escape, process escape and decompression-bomb fixtures fail.
- Worker cannot read outside granted files/packs/temp.
- Unsupported sandbox targets are not claimed supported.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-012: implement-sandboxed-worker-execution-and-untrusted-media-l`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-013 [P-B] — Implement crash recovery and project repair

**Depends on:** CR-V2-B7-012  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B7-013: implement-crash-recovery-and-project-repair`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-recovery/Cargo.toml`
- `crates/video-recovery/src/lib.rs`
- `crates/video-recovery/src/scan.rs`
- `crates/video-recovery/src/repair.rs`
- `crates/video-recovery/tests/recovery.rs`

**Procedure**

1. Scan active pointer, revisions, staging dirs, job states, logs, object hashes, indexes, packs and receipts.
2. Repair only derivable/index/staging state automatically; canonical revision or source corruption requires restore/relink/user decision.
3. Produce dry-run and applied recovery reports.
4. Keep recovery idempotent.

**Required implementation shape**

```text
automatic: rebuild index, remove abandoned staging, resume job
manual: canonical object tamper, source mismatch, incompatible migration
```

**Commands for this task**

```bash
cargo test -p video-recovery --locked recovery
```

**Acceptance — inspect and run only the listed focused checks**

- Every fault fixture gets correct automatic/manual classification.
- Repair does not discard evidence.
- Second repair run makes no changes.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-013: implement-crash-recovery-and-project-repair`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-014 [P-B] — Implement tamper detection, receipt tree, and pack/project trust status

**Depends on:** CR-V2-B7-013  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B7-014: implement-tamper-detection-receipt-tree-and-pack-project-t`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-security/src/trust.rs`
- `crates/video-project/src/trust.rs`
- `apps/studio/src/components/TrustPanel.tsx`
- `crates/video-security/tests/trust.rs`

**Procedure**

1. Verify source hashes, canonical objects, revision ancestry, action/job/render/QA receipts, skill/catalog hashes and active pack signatures.
2. Compute project trust status with exact failures and remediation.
3. Prevent final/package approval when required trust bindings fail.
4. Expose read-only trust tree in Studio.

**Required implementation shape**

```text
TrustStatus { overall, sources, revisions, actions, jobs, renders, qa, skills, packs, failures }
```

**Commands for this task**

```bash
cargo test -p video-security -p video-project --locked trust
pnpm --dir apps/studio test -- --run TrustPanel
```

**Acceptance — inspect and run only the listed focused checks**

- Tampered source/object/receipt/pack fixtures fail distinctly.
- Trust status cannot be overridden by a model.
- Repairable versus non-repairable is explicit.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-014: implement-tamper-detection-receipt-tree-and-pack-project-t`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-015 [P-B] — Implement privacy-safe local logs and telemetry-off defaults

**Depends on:** CR-V2-B7-014  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B7-015: implement-privacy-safe-local-logs-and-telemetry-off-defaul`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-security/src/privacy.rs`
- `apps/studio/src-tauri/src/privacy_settings.rs`
- `docs/security/V2-PRIVACY.md`
- `crates/video-security/tests/privacy.rs`

**Procedure**

1. Keep logs local, bounded and redacted; transcripts/prompts/paths are opt-in diagnostic attachments, not default log text.
2. Disable telemetry and network by default; expose a network-attempt audit counter.
3. Implement local log export with user-reviewed file list.
4. Add retention/clear actions that never delete canonical project evidence without explicit destructive confirmation.

**Required implementation shape**

```text
log fields default: component, code, project pseudonymous id, revision, job/stage id, durations, hashes; raw content excluded
```

**Commands for this task**

```bash
cargo test -p video-security --locked privacy
```

**Acceptance — inspect and run only the listed focused checks**

- Default logs contain no raw transcript/source path/API key.
- Network-attempt audit works with blocked network.
- Clear diagnostics leaves project canonical state intact.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-015: implement-privacy-safe-local-logs-and-telemetry-off-defaul`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-016 [P-B] — Implement pack repair, rollback, and offline payload integrity in Studio

**Depends on:** CR-V2-B7-015  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B7-016: implement-pack-repair-rollback-and-offline-payload-integri`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src/modes/PackManagerMode.tsx`
- `apps/studio/src-tauri/src/pack_commands.rs`
- `apps/studio/src/PackManager.test.tsx`
- `crates/video-runtime/tests/offline_repair.rs`

**Procedure**

1. List active/available local payload packs, compatibility, signatures, size and measured target status.
2. Implement verify, repair from selected installer payload, activate and rollback through shared pack service.
3. Require app restart only when frozen pack policy says so and preserve old compatible pack until success.
4. Never offer web download in offline v2.

**Required implementation shape**

```text
PackManager actions: verify | repair_from_payload | activate | rollback; source must be local verified bundle
```

**Commands for this task**

```bash
cargo test -p video-runtime --locked offline_repair
pnpm --dir apps/studio test -- --run PackManager
```

**Acceptance — inspect and run only the listed focused checks**

- Interrupted repair/activation keeps old pack active.
- Corrupt payload is rejected.
- No network control or URL appears.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-016: implement-pack-repair-rollback-and-offline-payload-integri`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-017 [P-C] — Create local signing, notarization, and seal scripts without upload

**Depends on:** CR-V2-B7-006  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B7-017: create-local-signing-notarization-and-seal-scripts-without`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `scripts/release/v2-build.py`
- `scripts/release/v2-sign.py`
- `scripts/release/v2-seal.py`
- `docs/release/V2-LOCAL-RELEASE.md`

**Procedure**

1. Separate deterministic build, platform signing/notarization preparation, pack signing, installer assembly and final seal.
2. Read credentials only through approved local signing interfaces; never print or inspect secrets.
3. Support unsigned development acceptance and signed release acceptance distinctly.
4. Do not implement upload or hosted update publication.

**Required implementation shape**

```text
build → pack-sign → app-sign → installer-assemble → optional platform notarization step → seal; upload absent
```

**Commands for this task**

```bash
python3 scripts/release/v2-build.py --help
python3 scripts/release/v2-sign.py --self-test --unsigned-fixture
python3 scripts/release/v2-seal.py --self-test
```

**Acceptance — inspect and run only the listed focused checks**

- Scripts are local-only and rerunnable.
- Unsigned artifacts cannot masquerade as signed.
- Seal manifest lists every file hash/signature/notice/source archive.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-017: create-local-signing-notarization-and-seal-scripts-without`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-018 [P-C] — Assemble the complete offline installers and payload

**Depends on:** CR-V2-B7-017  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B7-018: assemble-the-complete-offline-installers-and-payload`  
**Stop-loss ceiling:** at most 60 files and 10000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `scripts/release/v2-assemble-offline.py`
- `release/v2/bundle-manifest.json`
- `release/v2/layout/**`
- `docs/release/V2-OFFLINE-BUNDLE-CONTENTS.md`

**Procedure**

1. Assemble target app, Creator packs, notices, corresponding source, repair payload, sample projects, checksums and signatures.
2. Use target-specific Tauri bundling and explicit resource paths.
3. Verify installed paths, executable permissions and pack activation from a staged install root.
4. Keep quality pack optional but available in the same offline distribution set if built.

**Required implementation shape**

```text
offline payload roots: app/ packs/ repair/ licences/ corresponding-source/ samples/ checksums/ signatures/
```

**Commands for this task**

```bash
python3 scripts/release/v2-assemble-offline.py --target host --staging release/v2/staging
python3 scripts/release/v2-seal.py --verify release/v2/staging
```

**Acceptance — inspect and run only the listed focused checks**

- Bundle manifest matches bytes.
- Creator bundle contains all required packs.
- No install-time internet/browser/model download is required.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-018: assemble-the-complete-offline-installers-and-payload`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-019 [P-C] — Build the source distribution and LGPL corresponding-source bundle

**Depends on:** CR-V2-B7-018  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B7-019: build-the-source-distribution-and-lgpl-corresponding-sourc`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `scripts/release/v2-source-bundle.py`
- `release/v2/source-manifest.json`
- `docs/release/V2-SOURCE-BUNDLE.md`

**Procedure**

1. Package CutRight source at the exact release commit, vendored permitted source, patches, lockfiles, build scripts and licence notices.
2. Package FFmpeg corresponding source/configuration separately and include links/hashes in installer notices.
3. Exclude private benchmark media, credentials, caches, model bytes not redistributable as source and workspace-only provenance not intended for release.
4. Verify an offline source build can use the included dependency/vendor cache policy or document the exact prebuilt pack boundary.

**Required implementation shape**

```text
source bundle != offline binary bundle; both share release manifest and exact commit/pack hashes
```

**Commands for this task**

```bash
python3 scripts/release/v2-source-bundle.py --target host --out release/v2/source
python3 scripts/release/v2-seal.py --verify release/v2/source
```

**Acceptance — inspect and run only the listed focused checks**

- Source manifest is complete and deterministic.
- Reciprocal source obligations are fulfilled.
- Private/user data is absent.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-019: build-the-source-distribution-and-lgpl-corresponding-sourc`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-020 [P-C] — Create rights-cleared sample projects and offline product documentation

**Depends on:** CR-V2-B7-019  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B7-020: create-rights-cleared-sample-projects-and-offline-product-`  
**Stop-loss ceiling:** at most 180 files and 40000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `samples/v2/**`
- `docs/user/v2/**`
- `scripts/release/validate-samples.py`
- `release/v2/sample-manifest.json`

**Procedure**

1. Create one small sample per production lane with redistributable sources, expected stages, tutorial and acceptance hashes.
2. Document first launch, packs, Make Versions, review/correction, design/motion, QA, export, recovery and privacy.
3. Do not instruct users to install Python, Node, FFmpeg, Ollama, HeardRight or skills.
4. Keep sample outputs small and reproducible.

**Required implementation shape**

```text
samples: recorded-talking-head | repurpose-podcast | procedural-explainer | anchored-product
```

**Commands for this task**

```bash
python3 scripts/release/validate-samples.py samples/v2 release/v2/sample-manifest.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every sample has rights/provenance.
- Docs match current generated capability registry.
- All samples work offline from installed product.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-020: create-rights-cleared-sample-projects-and-offline-product-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-021 [P-C] — Implement the clean-machine, blocked-network acceptance harness

**Depends on:** CR-V2-B7-020  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B7-021: implement-the-clean-machine-blocked-network-acceptance-har`  
**Stop-loss ceiling:** at most 40 files and 12000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `scripts/qa/v2-clean-machine/**`
- `schemas/release/clean-machine-result.schema.v1.json`
- `docs/release/V2-CLEAN-MACHINE-HARNESS.md`

**Procedure**

1. Provision a fresh supported-machine snapshot or isolated runner with no developer toolchain, empty user PATH and blocked outbound network.
2. Install the offline bundle, verify packs, run four samples, perform correction/undo, restart/resume, repair/rollback and uninstall preservation checks.
3. Capture process, file, network, installer and application evidence.
4. Emit target-specific canonical result.

**Required implementation shape**

```text
preconditions: fresh OS user, PATH empty, network deny, no Python/Node/FFmpeg/Ollama/HeardRight/CodeRight/workspace
postcondition: four-lane acceptance pass
```

**Commands for this task**

```bash
python3 scripts/qa/v2-clean-machine/run.py --target host --bundle release/v2/staging --result release/v2/clean-machine-host.json
```

**Acceptance — inspect and run only the listed focused checks**

- Network attempts are zero.
- No missing external executable/model/skill/browser appears.
- All four lanes reach expected ready/review state.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-021: implement-the-clean-machine-blocked-network-acceptance-har`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-022 [S] — Merge Book 7 lanes and integrate v1-to-v2 project migration

**Depends on:** CR-V2-B7-011, CR-V2-B7-016, CR-V2-B7-021  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B7-022: merge-book-7-lanes-and-integrate-v1-to-v2-project-migratio`  
**Stop-loss ceiling:** at most 40 files and 7000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-state/src/migrations/v2.rs`
- `crates/video-project/src/legacy.rs`
- `apps/studio/src/modes/MigrationMode.tsx`
- `fixtures/migrations/v1-to-v2/**`
- `docs/dispatch/v2/book-7/merge-receipt.md`

**Procedure**

1. Apply lane A, B and C commits in fixed order.
2. Implement migration of current candidates/cut plans/timelines/decisions/finish/effects/providers into v2 revisions, actions, evidence refs and native effects.
3. Preserve old project backup and prior finals; remove external runtime requirements from migrated active configuration.
4. Show dry-run/report/backup/execute/result in Studio.

**Required implementation shape**

```text
legacy effect_id → native migration table
legacy provider path → provenance record
legacy active variant → immutable v2 revision + selection record
```

**Commands for this task**

```bash
cargo test -p video-state -p video-project --locked v1_to_v2
pnpm --dir apps/studio test -- --run MigrationMode
```

**Acceptance — inspect and run only the listed focused checks**

- All representative v1 fixtures migrate idempotently.
- Old Remotion/WhisperX/HeardRight provider records remain provenance only.
- Merge receipt is complete.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-022: merge-book-7-lanes-and-integrate-v1-to-v2-project-migratio`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-023 [S] — Run final four-lane benchmark and Studio acceptance on supported targets

**Depends on:** CR-V2-B7-022  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B7-023: run-final-four-lane-benchmark-and-studio-acceptance-on-sup`  
**Stop-loss ceiling:** at most 600 files and 100000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `benchmarks/runs/v2-release-candidate/**`
- `release/v2/acceptance/**`
- `docs/release/V2-FOUR-LANE-RESULTS.md`

**Procedure**

1. Run the complete benchmark profile, Studio workflows and clean-machine samples using exact release candidate app and pack set.
2. Include reviewed mode for all formats and any separately earned review-light/autonomous evidence; never promote synthetic results.
3. Slice by target, lane, language, source condition and pack.
4. Record failures and block unsupported target claims.

**Required implementation shape**

```text
supported target only if benchmark + Studio + clean-machine results all pass for the exact RC hashes
```

**Commands for this task**

```bash
cargo run -p video-bench -- run --corpus benchmarks/corpus/manifest.json --profile benchmarks/profiles/reviewed-v2.json --packs release/v2/staging/packs.lock.json --out benchmarks/runs/v2-release-candidate
pnpm --dir apps/studio qa:v2:workflows
```

**Acceptance — inspect and run only the listed focused checks**

- Kernel/safety/release floors pass.
- Four lanes pass on every claimed target.
- Autonomy claims match real compatible evidence.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-023: run-final-four-lane-benchmark-and-studio-acceptance-on-sup`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-024 [S] — Run final security, privacy, licence, and supply-chain release audit

**Depends on:** CR-V2-B7-023  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B7-024: run-final-security-privacy-licence-and-supply-chain-releas`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `release/v2/audit/**`
- `docs/release/V2-RELEASE-AUDIT.md`

**Procedure**

1. Run threat-model cases, sandbox/resource limits, network denial, secret scan, pack/project tamper, licence ledger, corresponding source, dependency licences, forbidden renderer/runtime, skill closure and source-corpus leakage checks.
2. Run pinned scanners when available; absent optional scanners remain unproven and may block according to release policy.
3. Inspect installer contents and permissions.
4. Do not weaken policy to make the report pass.

**Required implementation shape**

```text
release audit status = pass only if all policy.required_checks == pass; skipped is never coerced to pass
```

**Commands for this task**

```bash
python3 scripts/release/v2-audit.py --bundle release/v2/staging --out release/v2/audit
python3 scripts/legal/validate-v2-ledger.py --scope release
python3 scripts/gates/v2-no-legacy-renderer.py --check
python3 scripts/gates/v2-runtime-boundary.py --check
```

**Acceptance — inspect and run only the listed focused checks**

- No release-blocking security/licence/provenance finding remains.
- Every skipped/unproven scanner is visible and policy-resolved.
- Installer contains no private corpus/workspace path.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-024: run-final-security-privacy-licence-and-supply-chain-releas`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-025 [S] — Generate the final SBOM, provenance graph, and release disclosure

**Depends on:** CR-V2-B7-024  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B7-025: generate-the-final-sbom-provenance-graph-and-release-discl`  
**Stop-loss ceiling:** at most 12 files and 5000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `release/v2/SBOM.spdx.json`
- `release/v2/provenance.json`
- `release/v2/THIRD-PARTY-NOTICES.md`
- `docs/release/V2-DISCLOSURE.md`
- `scripts/release/v2-provenance.py`

**Procedure**

1. Generate an SPDX SBOM for Rust, JavaScript build-time dependencies, vendored source, native libraries, model packs, fonts, voices, templates, SFX and sample assets.
2. Generate a provenance graph linking every release byte to source corpus row, source revision, transformation/build command, licence row, pack or installer location, and acceptance evidence.
3. Render user-facing third-party notices and a disclosure that distinguishes bundled runtime components, optional packs, model licences, unsupported targets, privacy defaults and known limitations.
4. Fail when a release byte lacks provenance, a materialized component lacks a licence disposition, or the SBOM and sealed bundle disagree.

**Required implementation shape**

```text
release byte → build output → source component@revision → licence row → corpus row → test/audit evidence
missing edge => release block
```

**Commands for this task**

```bash
python3 scripts/release/v2-provenance.py --bundle release/v2/staging --sbom release/v2/SBOM.spdx.json --provenance release/v2/provenance.json --notices release/v2/THIRD-PARTY-NOTICES.md
python3 scripts/release/v2-seal.py --verify-provenance release/v2/staging release/v2/provenance.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every bundled file is represented directly or by a deterministic generated-file relationship.
- SBOM package/file hashes match the staged release.
- Notices and disclosure contain no unresolved or misleading licence statements.
- The generated documents are reproducible from the same release inputs.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-025: generate-the-final-sbom-provenance-graph-and-release-discl`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-026 [S] — Build and seal the local release candidate

**Depends on:** CR-V2-B7-025  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B7-026: build-and-seal-the-local-release-candidate`  
**Stop-loss ceiling:** at most 120 files and 18000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `release/v2/rc/**`
- `release/v2/RC-MANIFEST.json`
- `docs/release/V2-RC-REPORT.md`

**Procedure**

1. Build from a clean checkout at the exact candidate commit using local scripts.
2. Assemble/sign or explicitly mark unsigned target installers/packs, source bundles, notices and samples.
3. Verify every hash/signature and reproduce build metadata.
4. Do not upload, publish, tag or mutate external services.

**Required implementation shape**

```text
RC status: local_release_candidate; publish_status: not_requested; upload_status: not_performed
```

**Commands for this task**

```bash
python3 scripts/release/v2-build.py --profile release --target host --out release/v2/rc
python3 scripts/release/v2-seal.py --seal release/v2/rc --manifest release/v2/RC-MANIFEST.json
python3 scripts/release/v2-seal.py --verify release/v2/rc
```

**Acceptance — inspect and run only the listed focused checks**

- RC manifest binds app, packs, source, tests, audits and acceptance.
- Unsigned/signed status is explicit.
- No external publication occurs.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-026: build-and-seal-the-local-release-candidate`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B7-027 [S] — Run the final authoritative local gate, clean-machine proof, and checksum seal

**Depends on:** CR-V2-B7-026  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B7-027: run-the-final-authoritative-local-gate-clean-machine-proof`  
**Stop-loss ceiling:** at most 3 files and 1800 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-7/final-gate.md`
- `docs/dispatch/v2/book-7/final-manifest.json`
- `release/v2/SHA256SUMS.txt`

**Procedure**

1. Run final capability/skill/runtime/renderer/licence/security/release audits and clean-machine harness for each claimed target.
2. Run the authoritative repository gate exactly once at the end.
3. Generate checksums for RC artefacts and bind them to final manifest.
4. Do not create CI, upload, publish, tag or announce.

**Required implementation shape**

```text
book: 7
product_boundary: standalone_offline
external_runtime_dependencies: 0
network_attempts_in_acceptance: 0
ci: forbidden
publish: false
```

**Commands for this task**

```bash
python3 scripts/release/v2-audit.py --bundle release/v2/rc --out release/v2/audit-final
python3 scripts/qa/v2-clean-machine/run.py --target host --bundle release/v2/rc --result release/v2/clean-machine-final-host.json
bash scripts/gate.sh --with-qa
python3 scripts/release/v2-seal.py --checksums release/v2/rc --out release/v2/SHA256SUMS.txt
```

**Acceptance — inspect and run only the listed focused checks**

- Every claimed target has passing clean-machine and acceptance result.
- All required local gates pass and checksums verify.
- The final manifest states no CI and no publication.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B7-027: run-the-final-authoritative-local-gate-clean-machine-proof`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.
