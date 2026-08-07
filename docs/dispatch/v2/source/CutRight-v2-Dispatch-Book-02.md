# CutRight v2 Dispatch Book 2: Shared Capability Registry, Typed Actions, and Transactional Project State

**Tasks:** 27  
**Goal:** Create one action and capability contract for Studio, the embedded agent, CLI, MCP and tests; make every mutation revision-bound, atomic, validated and undoable.  
**Execution default:** A single agent runs numeric order. The labels show safe parallelism for an authorised dispatcher with isolated checkouts; this document does not itself authorise new branches or worktrees.  
**Testing cadence:** Focused checks run inside tasks. The full authoritative local gate runs only in task `CR-V2-B2-027`.  
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
CR-V2-B2-001 .. 006    sequential contract/interface freeze
CR-V2-B2-007 .. 011    parallel lane A
CR-V2-B2-012 .. 016    parallel lane B
CR-V2-B2-017 .. 021    parallel lane C
CR-V2-B2-022 .. 027    sequential merge, integration, acceptance, gate
```

## CR-V2-B2-001 [S] — Freeze stable identifiers, rational time, and revision semantics

**Depends on:** Book 1 final gate  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B2-001: freeze-stable-identifiers-rational-time-and-revision-seman`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/architecture/V2-IDENTITY-TIME-REVISION.md`
- `schemas/core/identity.schema.v1.json`
- `schemas/core/revision.schema.v1.json`

**Procedure**

1. Define opaque stable IDs for project, timeline, track, clip, word, evidence node, action batch, job and asset.
2. Define source time in integer nanoseconds or rational ticks and timeline time in rational project ticks; prohibit floating-point canonical time.
3. Define immutable revisions, parent links, active pointers and compatibility fingerprints.
4. Specify migration from existing string IDs and millisecond fields without losing source bindings.

**Required implementation shape**

```text
pub struct RationalTime { pub value: i128, pub rate_num: u64, pub rate_den: u64 }
pub struct RevisionId(Uuid);
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/core/identity.schema.v1.json fixtures/schemas/core/identity/v1/valid/basic.json
python3 scripts/schema-check.py schemas/core/revision.schema.v1.json fixtures/schemas/core/revision/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every canonical time conversion is exact or returns a typed rounding error.
- IDs are never inferred from names or indexes.
- Revisions are immutable and form an acyclic parent graph.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-001: freeze-stable-identifiers-rational-time-and-revision-seman`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-002 [S] — Freeze the capability and action schema vocabulary

**Depends on:** CR-V2-B2-001  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B2-002: freeze-the-capability-and-action-schema-vocabulary`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/architecture/V2-CAPABILITY-ACTION-CONTRACT.md`
- `schemas/capabilities/registry.schema.v1.json`
- `schemas/actions/action-batch.schema.v1.json`

**Procedure**

1. Define capability IDs, versions, requirements, permissions, inputs, outputs, degradation, eval suites and owner component.
2. Define read models separately from mutation actions.
3. Define action batch envelope, expected revision, evidence references, intent, actions and dry-run metadata.
4. Freeze snake_case JSON and reject unknown fields.

**Required implementation shape**

```text
{"batch_id":"ab_01","project_id":"prj_01","timeline_id":"tl_01","expected_revision":"rev_07","actions":[],"evidence_refs":[],"intent":"tighten opening"}
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/capabilities/registry.schema.v1.json fixtures/schemas/capabilities/v1/valid/basic.json
python3 scripts/schema-check.py schemas/actions/action-batch.schema.v1.json fixtures/schemas/actions/action-batch/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every mutation capability references one action schema.
- Every read capability declares bounded/windowed output behavior.
- Unknown action kinds and unknown fields fail.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-002: freeze-the-capability-and-action-schema-vocabulary`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-003 [S] — Freeze transactional apply, inverse action, and failure semantics

**Depends on:** CR-V2-B2-002  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B2-003: freeze-transactional-apply-inverse-action-and-failure-sema`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/architecture/V2-TRANSACTIONS-UNDO.md`
- `schemas/actions/action-result.schema.v1.json`
- `schemas/actions/inverse-batch.schema.v1.json`

**Procedure**

1. Specify staged clone application, full semantic validation, atomic artifact writes, revision commit, receipt emission and active-pointer swap.
2. Specify inverse action generation at apply time; non-reversible actions must declare why and require a preserved prior revision.
3. Specify stale revision, missing target, invalid range, permission denial, resource limit and partial-output failure codes.
4. Define interruption injection points for atomicity tests.

**Required implementation shape**

```text
stage clone → apply all actions → validate → fsync artifacts → write revision → fsync → atomic active pointer swap → emit result
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/actions/action-result.schema.v1.json fixtures/schemas/actions/action-result/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- No successful result can exist without a new revision and receipt.
- Failure never advances the active pointer.
- Undo is a normal validated action batch.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-003: freeze-transactional-apply-inverse-action-and-failure-sema`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-004 [S] — Freeze permissions, skill boundaries, and session write guards

**Depends on:** CR-V2-B2-003  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B2-004: freeze-permissions-skill-boundaries-and-session-write-guar`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/security/V2-ACTION-PERMISSIONS.md`
- `schemas/capabilities/permission-set.schema.v1.json`
- `schemas/agent/session-binding.schema.v1.json`

**Procedure**

1. Define least-privilege permissions for evidence reads, asset planning, timeline reads, timeline mutations, rendering, exports, settings and pack management.
2. Bind every external or embedded agent session to one project and active timeline revision.
3. Require frontmost-project confirmation for external MCP writes while allowing safe reads from the bound project.
4. Keep Designer, Writing, Social and QA unable to mutate cut points.

**Required implementation shape**

```text
permission timeline.cut.write is granted only to cutright-director, editorial-director, and explicit user actions
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/capabilities/permission-set.schema.v1.json fixtures/schemas/capabilities/permissions/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every skill has an explicit permission set.
- Cross-project writes fail.
- Non-editor skills cannot produce timeline mutation actions.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-004: freeze-permissions-skill-boundaries-and-session-write-guar`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-005 [S] — Freeze semantic dry-run and diff output

**Depends on:** CR-V2-B2-004  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B2-005: freeze-semantic-dry-run-and-diff-output`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/architecture/V2-SEMANTIC-DIFF.md`
- `schemas/actions/semantic-diff.schema.v1.json`
- `fixtures/actions/semantic-diff/`

**Procedure**

1. Define human-readable and machine-readable differences for cuts, restores, moves, take swaps, retimes, captions, graphics, audio, colour, exports and settings.
2. Include affected time ranges, before/after IDs, duration delta, evidence, confidence and risk flags.
3. Ensure dry-run uses the same validator and apply planner as real execution but does not write project state.
4. Define stable ordering so snapshot tests do not flap.

**Required implementation shape**

```text
{"summary":"Remove 1 pause and swap 1 take","duration_delta_ns":-820000000,"changes":[{"kind":"remove_range","range":{"start":...},"reason":"dead_air"}]}
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/actions/semantic-diff.schema.v1.json fixtures/actions/semantic-diff/valid.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every mutation action has a diff renderer.
- Diff output can be shown before execution without parsing logs.
- Dry-run and real apply produce identical planned operations for the same revision.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-005: freeze-semantic-dry-run-and-diff-output`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-006 [S] — Freeze Book 2 crate boundaries and parallel ownership

**Depends on:** CR-V2-B2-005  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B2-006: freeze-book-2-crate-boundaries-and-parallel-ownership`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-2/interface-freeze.md`
- `docs/architecture/V2-CRATE-DAG.md`

**Procedure**

1. Assign lane A `crates/video-actions/`; lane B `crates/video-capabilities/` plus generated bindings; lane C `crates/video-state/`, `crates/video-sessions/` and migration fixtures.
2. Freeze dependency direction: core ← state/actions/capabilities; project orchestrates but lower crates never depend on project/CLI/Studio.
3. Reserve root workspace manifest, executor integration, CLI, Studio and MCP wiring for serial tasks.
4. List public types that parallel lanes may use but not rename.

**Required implementation shape**

```text
video-core <- {video-state, video-actions, video-capabilities}
video-project <- executor <- {video-cli, studio, video-agent, mcp}
```

**Commands for this task**

```bash
python3 scripts/architecture/check_crate_dag.py docs/architecture/V2-CRATE-DAG.md
```

**Acceptance — inspect and run only the listed focused checks**

- No dependency cycle is permitted.
- Every parallel file root has one owner.
- Public frozen names match tasks 001–005.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-006: freeze-book-2-crate-boundaries-and-parallel-ownership`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-007 [P-A] — Implement the typed Action enum and stable target references

**Depends on:** CR-V2-B2-006  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B2-007: implement-the-typed-action-enum-and-stable-target-referenc`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-actions/Cargo.toml`
- `crates/video-actions/src/lib.rs`
- `crates/video-actions/src/action.rs`
- `crates/video-actions/src/targets.rs`
- `crates/video-actions/tests/serde.rs`

**Procedure**

1. Create exhaustive action variants for timeline structure, clips, transcript corrections, captions/text, graphics, motion, audio, colour, exports and project settings.
2. Use stable IDs only; indexes may appear in read models but never as mutation targets.
3. Use rational time types from `video-core` and strict serde tagging.
4. Add round-trip and unknown-variant tests.

**Required implementation shape**

```text
#[serde(tag="kind", rename_all="snake_case")]
pub enum Action { RemoveRange(RemoveRange), RestoreSegment(RestoreSegment), SwapTake(SwapTake), SetCaptionText(SetCaptionText), AddGraphic(AddGraphic), SetAudioMix(SetAudioMix) }
```

**Commands for this task**

```bash
cargo test -p video-actions --locked serde
```

**Acceptance — inspect and run only the listed focused checks**

- Every frozen mutation capability has one action variant.
- No action contains `f64` canonical time or filesystem path to a source asset.
- Unknown action kinds fail deserialization.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-007: implement-the-typed-action-enum-and-stable-target-referenc`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-008 [P-A] — Implement semantic action validation

**Depends on:** CR-V2-B2-007  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B2-008: implement-semantic-action-validation`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-actions/src/validate.rs`
- `crates/video-actions/src/errors.rs`
- `crates/video-actions/tests/validation.rs`

**Procedure**

1. Validate revision match, target existence, ranges, track compatibility, source bounds, linked audio, caption groups, safe zones, permissions and action ordering.
2. Return stable error codes and exact action index/path.
3. Reject overlapping destructive actions unless a frozen composition rule explicitly permits them.
4. Validate the complete batch before applying any action.

**Required implementation shape**

```text
pub struct ActionViolation { pub action_index: usize, pub code: ViolationCode, pub field: JsonPointer, pub message: String }
```

**Commands for this task**

```bash
cargo test -p video-actions --locked validation
```

**Acceptance — inspect and run only the listed focused checks**

- Every negative fixture returns the expected code and action index.
- Validation has no filesystem writes.
- A valid batch result is deterministic.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-008: implement-semantic-action-validation`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-009 [P-A] — Implement semantic dry-run and stable diff generation

**Depends on:** CR-V2-B2-008  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B2-009: implement-semantic-dry-run-and-stable-diff-generation`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-actions/src/dry_run.rs`
- `crates/video-actions/src/diff.rs`
- `crates/video-actions/tests/dry_run.rs`

**Procedure**

1. Apply actions to an in-memory staged state through the same operation planner used by execution.
2. Generate canonical `SemanticDiff` entries and duration/track/asset summaries.
3. Sort changes by timeline range then stable action index.
4. Prove dry-run leaves project bytes unchanged.

**Required implementation shape**

```text
pub fn dry_run(state: &ProjectState, batch: &ActionBatch, permissions: &PermissionSet) -> Result<PlannedTransaction, ActionError>
```

**Commands for this task**

```bash
cargo test -p video-actions --locked dry_run
```

**Acceptance — inspect and run only the listed focused checks**

- Repeated dry-runs are byte-identical.
- Before/after project tree hashes are equal.
- Each action variant has a snapshot fixture.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-009: implement-semantic-dry-run-and-stable-diff-generation`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-010 [P-A] — Implement atomic transaction apply and revision commit

**Depends on:** CR-V2-B2-009  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B2-010: implement-atomic-transaction-apply-and-revision-commit`  
**Stop-loss ceiling:** at most 8 files and 1800 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-actions/src/apply.rs`
- `crates/video-actions/src/transaction.rs`
- `crates/video-actions/tests/atomicity.rs`

**Procedure**

1. Create a staging directory on the same filesystem as the project package.
2. Apply the validated operation plan, validate all canonical artifacts, fsync files/directories, write revision and receipt, then atomically swap the active revision pointer.
3. Clean staging on failure and preserve diagnostic evidence in a bounded failure record.
4. Add interruption injection at every frozen transaction phase.

**Required implementation shape**

```text
pub fn apply_atomic(store: &ProjectStore, planned: PlannedTransaction, cancel: &CancellationToken) -> Result<ActionResult, ActionError>
```

**Commands for this task**

```bash
cargo test -p video-actions --locked atomicity
```

**Acceptance — inspect and run only the listed focused checks**

- Every injected interruption leaves either the old or complete new revision.
- No half-written canonical JSON is visible.
- A stale expected revision never enters staging.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-010: implement-atomic-transaction-apply-and-revision-commit`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-011 [P-A] — Implement inverse batches and undo/redo

**Depends on:** CR-V2-B2-010  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B2-011: implement-inverse-batches-and-undo-redo`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-actions/src/inverse.rs`
- `crates/video-actions/src/history.rs`
- `crates/video-actions/tests/undo.rs`

**Procedure**

1. Generate inverse actions from the pre-apply state and the planned operations, not by guessing from the result.
2. Store inverse batch hash in the revision metadata.
3. Implement undo as application of the inverse against the current expected revision; implement redo through the original batch retained in history.
4. For non-reversible external exports, retain project-state reversibility and mark the external side effect separately.

**Required implementation shape**

```text
pub struct RevisionHistoryEntry { pub revision: RevisionId, pub batch_hash: Hash, pub inverse_batch_hash: Hash, pub external_effects: Vec<ExternalEffect> }
```

**Commands for this task**

```bash
cargo test -p video-actions --locked undo
```

**Acceptance — inspect and run only the listed focused checks**

- Every reversible action round-trips canonical state hash.
- Undo creates a new revision; it does not delete history.
- Redo fails if the target state has diverged.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-011: implement-inverse-batches-and-undo-redo`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-012 [P-B] — Implement the capability registry model and validator

**Depends on:** CR-V2-B2-006  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B2-012: implement-the-capability-registry-model-and-validator`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-capabilities/Cargo.toml`
- `crates/video-capabilities/src/lib.rs`
- `crates/video-capabilities/src/model.rs`
- `crates/video-capabilities/src/validate.rs`
- `crates/video-capabilities/tests/registry.rs`

**Procedure**

1. Implement strict models for capabilities, requirements, permissions, inputs/outputs, degradations, owners, eval suites and runtime pack dependencies.
2. Load the tracked source registry and validate stable unique IDs, semantic versions, schema paths and dependency cycles.
3. Reject capabilities whose action/read implementation is absent.
4. Keep registry order canonical.

**Required implementation shape**

```text
pub struct Capability { pub id: CapabilityId, pub version: Version, pub kind: CapabilityKind, pub permissions: PermissionSet, pub requires: Vec<Requirement>, pub degradation: Vec<DegradationRule> }
```

**Commands for this task**

```bash
cargo test -p video-capabilities --locked registry
```

**Acceptance — inspect and run only the listed focused checks**

- Invalid cycle, duplicate ID, missing schema and unknown pack fixtures fail.
- Valid registry round-trips exactly.
- Registry validation performs no network access.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-012: implement-the-capability-registry-model-and-validator`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-013 [P-B] — Create the canonical source capability registry

**Depends on:** CR-V2-B2-012  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B2-013: create-the-canonical-source-capability-registry`  
**Stop-loss ceiling:** at most 100 files and 20000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `capabilities/registry.json`
- `capabilities/README.md`
- `schemas/capabilities/entries/*.json`

**Procedure**

1. Add every existing and planned v2 read/action/runtime/skill/render/export capability with status `implemented`, `planned`, `blocked`, or `retired`.
2. Map current CLI commands and Studio functions to capability IDs without changing behavior yet.
3. Mark external HeardRight, WhisperX Python, Remotion and HyperFrames runtime capabilities `retired` with native replacements.
4. Attach exact schema and eval suite IDs.

**Required implementation shape**

```text
{"id":"timeline.remove_range","version":"1.0.0","kind":"action","action_kind":"remove_range","permissions":["timeline.cut.write"],"status":"planned","owner":"video-actions"}
```

**Commands for this task**

```bash
cargo run -p video-capabilities --bin validate-registry -- capabilities/registry.json
```

**Acceptance — inspect and run only the listed focused checks**

- No current public command is unmapped.
- Retired capabilities cannot be selected by release profiles.
- Every planned capability has an owning future task/book.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-013: create-the-canonical-source-capability-registry`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-014 [P-B] — Generate Rust, TypeScript, CLI, and tool bindings from the registry

**Depends on:** CR-V2-B2-013  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B2-014: generate-rust-typescript-cli-and-tool-bindings-from-the-re`  
**Stop-loss ceiling:** at most 20 files and 18000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `tools/capability-codegen/**`
- `crates/video-capabilities/src/generated.rs`
- `apps/studio/src/generated/capabilities.ts`
- `crates/video-cli/src/generated_capabilities.rs`
- `crates/video-agent/src/generated_tools.rs`

**Procedure**

1. Create one deterministic generator reading the validated source registry and referenced schemas.
2. Generate IDs, descriptors, TypeScript read types, CLI metadata and agent tool definitions; do not generate execution logic.
3. Embed a source-registry hash in every output.
4. Add a `--check` mode that fails on drift.

**Required implementation shape**

```text
// @generated from capabilities/registry.json sha256:<hash>; do not edit
pub const TIMELINE_REMOVE_RANGE: CapabilityId = CapabilityId::new_static("timeline.remove_range");
```

**Commands for this task**

```bash
cargo test --manifest-path tools/capability-codegen/Cargo.toml --locked
cargo run --manifest-path tools/capability-codegen/Cargo.toml -- --check
```

**Acceptance — inspect and run only the listed focused checks**

- Generated files are byte-stable.
- Deleting or renaming a registry entry causes a focused drift failure.
- Studio, CLI and agent identifiers are identical.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-014: generate-rust-typescript-cli-and-tool-bindings-from-the-re`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-015 [P-B] — Integrate the compiled skill catalogue into the capability registry

**Depends on:** CR-V2-B2-014  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B2-015: integrate-the-compiled-skill-catalogue-into-the-capability`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-capabilities/src/skills.rs`
- `capabilities/skills.generated.json`
- `crates/video-capabilities/tests/skills.rs`

**Procedure**

1. Read `skills/catalog.lock.json` and generate one skill capability per compiled skill.
2. Verify skill permissions are a subset of declared capability permissions.
3. Bind each skill to its eval suite and resource hashes.
4. Reject an unknown skill dependency or direct mutation permission outside the frozen boundary.

**Required implementation shape**

```text
assert!(skill.permissions.is_subset_of(&capability.allowed_skill_permissions));
```

**Commands for this task**

```bash
cargo test -p video-capabilities --locked skills
cargo run --manifest-path tools/capability-codegen/Cargo.toml -- --check
```

**Acceptance — inspect and run only the listed focused checks**

- All embedded skills appear in the registry.
- Designer/Writing/Social/QA cannot receive cut permissions.
- Catalogue hash drift is detected.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-015: integrate-the-compiled-skill-catalogue-into-the-capability`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-016 [P-B] — Add capability registry drift and documentation generation

**Depends on:** CR-V2-B2-015  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B2-016: add-capability-registry-drift-and-documentation-generation`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `tools/capability-codegen/src/docs.rs`
- `docs/reference/CAPABILITIES.md`
- `docs/reference/ACTIONS.md`
- `scripts/gates/v2-capability-drift.sh`

**Procedure**

1. Generate human reference tables from the same registry and schemas.
2. Document status, permissions, inputs, outputs, packs, degradation and evals.
3. Fail when generated docs/bindings differ from source.
4. Add the drift guard to the local gate later, not in this task.

**Required implementation shape**

```text
cargo run --manifest-path tools/capability-codegen/Cargo.toml -- generate
git diff --exit-code -- crates apps docs/reference
```

**Commands for this task**

```bash
bash scripts/gates/v2-capability-drift.sh
```

**Acceptance — inspect and run only the listed focused checks**

- The docs have no manually maintained capability table.
- Retired and blocked capabilities are visibly labelled.
- The guard succeeds on a clean generation and fails after a planted edit.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-016: add-capability-registry-drift-and-documentation-generation`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-017 [P-C] — Implement immutable project revision storage

**Depends on:** CR-V2-B2-006  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B2-017: implement-immutable-project-revision-storage`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-state/Cargo.toml`
- `crates/video-state/src/lib.rs`
- `crates/video-state/src/store.rs`
- `crates/video-state/src/revision.rs`
- `crates/video-state/tests/store.rs`

**Procedure**

1. Store canonical revision metadata and content-addressed artifact references under the project package.
2. Use atomic active-pointer files and prevent revision overwrite.
3. Validate parent existence, acyclic ancestry, project identity and artifact hashes on read.
4. Support a read-only snapshot pinned to a revision.

**Required implementation shape**

```text
project/revisions/<revision_id>/revision.json
project/state/active-revision.json
project/objects/blake3/<hash>
```

**Commands for this task**

```bash
cargo test -p video-state --locked store
```

**Acceptance — inspect and run only the listed focused checks**

- A revision cannot be modified after commit.
- Corrupt parent/hash/active-pointer fixtures fail distinctly.
- Folder rename does not change project identity.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-017: implement-immutable-project-revision-storage`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-018 [P-C] — Implement project write locks and session bindings

**Depends on:** CR-V2-B2-017  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B2-018: implement-project-write-locks-and-session-bindings`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-sessions/Cargo.toml`
- `crates/video-sessions/src/lib.rs`
- `crates/video-sessions/src/binding.rs`
- `crates/video-sessions/src/write_lock.rs`
- `crates/video-sessions/tests/binding.rs`

**Procedure**

1. Create process-safe project write locks with owner, PID, session ID, start time and bounded stale-lock recovery.
2. Bind sessions to project, timeline and observed revision.
3. Require explicit refresh after an out-of-band active revision change.
4. Implement frontmost-project write guard for external MCP sessions.

**Required implementation shape**

```text
pub struct SessionBinding { pub session_id: SessionId, pub project_id: ProjectId, pub timeline_id: TimelineId, pub observed_revision: RevisionId, pub origin: SessionOrigin }
```

**Commands for this task**

```bash
cargo test -p video-sessions --locked binding
```

**Acceptance — inspect and run only the listed focused checks**

- Two writers cannot commit concurrently.
- A stale revision/session receives a typed refresh-required error.
- Read-only sessions do not block writers.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-018: implement-project-write-locks-and-session-bindings`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-019 [P-C] — Implement append-only action, decision, and audit logs

**Depends on:** CR-V2-B2-018  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B2-019: implement-append-only-action-decision-and-audit-logs`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-state/src/log.rs`
- `crates/video-state/src/audit.rs`
- `crates/video-state/tests/log.rs`

**Procedure**

1. Write hash-chained length-delimited records for action batches, results, user decisions, critic verdicts, pack transitions and security events.
2. Use cross-process locking and one buffered append.
3. Preserve malformed/truncated tail evidence and report it rather than silently dropping records.
4. Bind records to project instance, revision, artifact hashes and producer version.

**Required implementation shape**

```text
pub struct AuditRecord { pub previous_hash: Hash, pub record_hash: Hash, pub project: ProjectInstanceId, pub revision: RevisionId, pub payload: AuditPayload }
```

**Commands for this task**

```bash
cargo test -p video-state --locked log
```

**Acceptance — inspect and run only the listed focused checks**

- Concurrent appends remain parseable and ordered.
- Tampering breaks the hash chain.
- A truncated tail is reported and earlier records remain readable.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-019: implement-append-only-action-decision-and-audit-logs`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-020 [P-C] — Implement versioned project migrations with backups and dry-run

**Depends on:** CR-V2-B2-019  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B2-020: implement-versioned-project-migrations-with-backups-and-dr`  
**Stop-loss ceiling:** at most 14 files and 2400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-state/src/migrate.rs`
- `schemas/migrations/**`
- `fixtures/migrations/**`
- `crates/video-state/tests/migrate.rs`

**Procedure**

1. Define explicit migrations from current CutRight layout to v2 identity/time/revision/action history.
2. Dry-run reports every path, schema and semantic change.
3. Real migration writes a complete backup manifest before mutation and commits through atomic staging.
4. Make migrations idempotent and reject a newer unsupported schema.

**Required implementation shape**

```text
trait Migration { fn from(&self) -> SchemaVersion; fn to(&self) -> SchemaVersion; fn plan(&self, snapshot: &Snapshot) -> Result<MigrationPlan>; }
```

**Commands for this task**

```bash
cargo test -p video-state --locked migrate
```

**Acceptance — inspect and run only the listed focused checks**

- Every fixture migrates to the same canonical hash on first and second run.
- Failure restores the original active package.
- Dry-run writes nothing.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-020: implement-versioned-project-migrations-with-backups-and-dr`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-021 [P-C] — Create cross-crate contract fixtures for state, actions, and capabilities

**Depends on:** CR-V2-B2-020  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B2-021: create-cross-crate-contract-fixtures-for-state-actions-and`  
**Stop-loss ceiling:** at most 180 files and 40000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `fixtures/contracts/v2/**`
- `crates/video-state/tests/contracts.rs`
- `crates/video-actions/tests/contracts.rs`
- `crates/video-capabilities/tests/contracts.rs`

**Procedure**

1. Create valid and invalid fixtures for every frozen schema and critical semantic invariant.
2. Include stale revision, missing target, time overflow, permission denial, cyclic capability, corrupt revision, malformed log and non-reversible action cases.
3. Load identical JSON fixtures in Rust and TypeScript contract tests.
4. Name fixtures by expected error code.

**Required implementation shape**

```text
fixtures/contracts/v2/actions/invalid/stale_revision.json
expected_error.json: {"code":"stale_revision","action_index":null}
```

**Commands for this task**

```bash
cargo test -p video-state -p video-actions -p video-capabilities --locked contracts
```

**Acceptance — inspect and run only the listed focused checks**

- Every invalid fixture has one stable primary error code.
- No fixture depends on machine paths or current time.
- Rust and TypeScript decode the same valid canonical JSON.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-021: create-cross-crate-contract-fixtures-for-state-actions-and`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-022 [S] — Merge Book 2 lanes and scaffold the single ActionExecutor

**Depends on:** CR-V2-B2-011, CR-V2-B2-016, CR-V2-B2-021  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B2-022: merge-book-2-lanes-and-scaffold-the-single-actionexecutor`  
**Stop-loss ceiling:** at most 8 files and 1600 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-executor/Cargo.toml`
- `crates/video-executor/src/lib.rs`
- `crates/video-executor/src/executor.rs`
- `docs/dispatch/v2/book-2/merge-receipt.md`
- `Cargo.toml`

**Procedure**

1. Apply lane A, B and C commits in deterministic order.
2. Add the new crates to the root workspace and create `ActionExecutor` as the only mutation entry point.
3. Wire validation, dry-run, locks, staged apply, logs and capability checks in that order.
4. Do not expose Studio/CLI/MCP bindings yet.

**Required implementation shape**

```text
pub trait ActionExecutor { fn dry_run(&self, ctx: &ExecutionContext, batch: &ActionBatch) -> Result<SemanticDiff>; fn execute(&self, ctx: &ExecutionContext, batch: &ActionBatch) -> Result<ActionResult>; }
```

**Commands for this task**

```bash
cargo check -p video-executor --locked
python3 scripts/architecture/check_crate_dag.py docs/architecture/V2-CRATE-DAG.md
```

**Acceptance — inspect and run only the listed focused checks**

- The executor has one public dry-run and one public execute method.
- No UI/CLI crate bypasses it.
- The merge receipt lists every lane commit.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-022: merge-book-2-lanes-and-scaffold-the-single-actionexecutor`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-023 [S] — Expose the executor through the JSON CLI without duplicating logic

**Depends on:** CR-V2-B2-022  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B2-023: expose-the-executor-through-the-json-cli-without-duplicati`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-cli/src/cli.rs`
- `crates/video-cli/src/main.rs`
- `crates/video-cli/src/actions.rs`
- `crates/video-cli/tests/actions.rs`

**Procedure**

1. Add `actions dry-run`, `actions apply`, `actions undo`, `actions redo` and bounded read commands.
2. Parse canonical JSON from a file or stdin and return one JSON document on stdout.
3. Use generated capability metadata for help/IDs and the shared executor for all mutation.
4. Preserve stable nonzero exit codes for validation, stale revision, permission and receipt failure.

**Required implementation shape**

```text
videoctl actions dry-run PROJECT --batch batch.json
videoctl actions apply PROJECT --batch batch.json
```

**Commands for this task**

```bash
cargo test -p videoctl --locked actions
```

**Acceptance — inspect and run only the listed focused checks**

- CLI and direct executor produce identical result JSON.
- No CLI action contains mutation logic.
- Invalid JSON and domain errors use distinct stable codes.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-023: expose-the-executor-through-the-json-cli-without-duplicati`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-024 [S] — Expose the executor to the Studio backend

**Depends on:** CR-V2-B2-023  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B2-024: expose-the-executor-to-the-studio-backend`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src-tauri/src/action_commands.rs`
- `apps/studio/src-tauri/src/main.rs`
- `apps/studio/src/contracts/actions.ts`
- `apps/studio/src/lib/actions.ts`
- `apps/studio/src-tauri/src/tests/action_commands.rs`

**Procedure**

1. Add Tauri commands for registry read, dry-run, execute, undo, redo and bounded timeline/evidence reads.
2. Use the shared executor and session binding; frontend sends intent/batch only.
3. Return exact persisted result and semantic diff.
4. Keep current review commands functional through compatibility adapters.

**Required implementation shape**

```text
#[tauri::command]
fn apply_action_batch(state: State<AppState>, request: ApplyBatchRequest) -> Result<ActionResult, CommandError> { state.executor.execute(&request.context, &request.batch) }
```

**Commands for this task**

```bash
cargo test --manifest-path apps/studio/src-tauri/Cargo.toml --locked action_commands
pnpm --dir apps/studio test -- --run action
```

**Acceptance — inspect and run only the listed focused checks**

- Studio backend has no duplicate action validator.
- Cross-project and stale-session writes fail.
- Compatibility commands produce the same new audit records.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-024: expose-the-executor-to-the-studio-backend`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-025 [S] — Expose the same executor through an optional loopback MCP adapter

**Depends on:** CR-V2-B2-024  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B2-025: expose-the-same-executor-through-an-optional-loopback-mcp-`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-agent/src/mcp.rs`
- `crates/video-agent/src/tools.rs`
- `crates/video-agent/tests/mcp.rs`

**Procedure**

1. Generate MCP tool definitions from the capability registry.
2. Bind each connection to a project session and require frontmost-project guard for writes.
3. Listen only on loopback with an ephemeral token and disabled-by-default setting.
4. Map MCP calls to shared reads or ActionExecutor calls; no divergent schema.

**Required implementation shape**

```text
MCP request → generated tool lookup → session permission check → bounded read OR ActionExecutor → canonical result
```

**Commands for this task**

```bash
cargo test -p video-agent --locked mcp
```

**Acceptance — inspect and run only the listed focused checks**

- Tool IDs and schemas match generated capability bindings.
- Non-loopback bind is rejected.
- A write while another project is frontmost returns the frozen guard error.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-025: expose-the-same-executor-through-an-optional-loopback-mcp-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-026 [S] — Run cross-surface transaction and contract tests

**Depends on:** CR-V2-B2-025  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B2-026: run-cross-surface-transaction-and-contract-tests`  
**Stop-loss ceiling:** at most 4 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `tests/v2/action_surfaces.rs`
- `apps/studio/src/action-contract.test.ts`
- `docs/dispatch/v2/book-2/focused-tests.md`

**Procedure**

1. Run the same action fixtures through direct Rust, CLI JSON, Studio Tauri command and MCP adapter.
2. Compare semantic diff, resulting revision, receipt and error codes.
3. Inject interruption and stale revision cases through each surface.
4. Do not run the full repository gate in this task.

**Required implementation shape**

```text
assert_eq!(direct.canonical_json(), cli.canonical_json());
assert_eq!(direct.canonical_json(), tauri.canonical_json());
assert_eq!(direct.canonical_json(), mcp.canonical_json());
```

**Commands for this task**

```bash
cargo test --workspace --locked action_surfaces
pnpm --dir apps/studio test -- --run action-contract
```

**Acceptance — inspect and run only the listed focused checks**

- All surfaces are semantically identical.
- No surface can bypass permissions or revision checks.
- Evidence lists exact commands and test totals.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-026: run-cross-surface-transaction-and-contract-tests`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B2-027 [S] — Run the authoritative Book 2 local gate and freeze evidence

**Depends on:** CR-V2-B2-026  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B2-027: run-the-authoritative-book-2-local-gate-and-freeze-evidenc`  
**Stop-loss ceiling:** at most 2 files and 1200 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-2/final-gate.md`
- `docs/dispatch/v2/book-2/final-manifest.json`

**Procedure**

1. Run capability drift, crate DAG, repository boundary and focused cross-surface tests.
2. Run the existing authoritative local gate exactly once.
3. Record commit, commands, versions, exit codes, test totals and hashes.
4. Do not create hosted CI or publish.

**Required implementation shape**

```text
book: 2
required_invariant: all mutation surfaces use video-executor
ci: forbidden
```

**Commands for this task**

```bash
bash scripts/gates/v2-capability-drift.sh
python3 scripts/architecture/check_crate_dag.py docs/architecture/V2-CRATE-DAG.md
bash scripts/gates/v2-repository-shape.sh
bash scripts/gate.sh --with-qa
```

**Acceptance — inspect and run only the listed focused checks**

- All required checks pass.
- Generated registry bindings are clean.
- Final manifest binds the exact commit and evidence.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B2-027: run-the-authoritative-book-2-local-gate-and-freeze-evidenc`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.
