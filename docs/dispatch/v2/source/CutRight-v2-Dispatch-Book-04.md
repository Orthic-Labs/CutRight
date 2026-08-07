# CutRight v2 Dispatch Book 4: Benchmark-First Evaluation and Editorial Intelligence

**Tasks:** 27  
**Goal:** Establish the golden corpus, deterministic and model-based evaluators, then implement editorial reasoning under measurable confidence, preservation, truthfulness and escalation constraints.  
**Execution default:** A single agent runs numeric order. The labels show safe parallelism for an authorised dispatcher with isolated checkouts; this document does not itself authorise new branches or worktrees.  
**Testing cadence:** Focused checks run inside tasks. The full authoritative local gate runs only in task `CR-V2-B4-027`.  
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
CR-V2-B4-001 .. 006    sequential contract/interface freeze
CR-V2-B4-007 .. 011    parallel lane A
CR-V2-B4-012 .. 016    parallel lane B
CR-V2-B4-017 .. 021    parallel lane C
CR-V2-B4-022 .. 027    sequential merge, integration, acceptance, gate
```

## CR-V2-B4-001 [S] — Freeze the benchmark taxonomy, dataset split, and result schemas

**Depends on:** Book 3 final gate  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B4-001: freeze-the-benchmark-taxonomy-dataset-split-and-result-sch`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/benchmarks/V2-TAXONOMY.md`
- `schemas/benchmarks/corpus.schema.v1.json`
- `schemas/benchmarks/run.schema.v1.json`
- `schemas/benchmarks/project-result.schema.v1.json`

**Procedure**

1. Implement the axes from the v2 benchmark plan: kernel integrity, boundaries, audio-visual preservation, editorial quality, creative quality, instruction/preservation, reliability and resources.
2. Define train/calibration/test separation by speaker, recording session and source programme.
3. Require rights/provenance, expected language, conditions, labels, reviewer IDs and allowed distribution for every item.
4. Use explicit `pass`, `fail`, `skipped_with_reason`, `unsupported`, and `unproven` statuses.

**Required implementation shape**

```text
pub enum MetricStatus { Pass, Fail, SkippedWithReason, Unsupported, Unproven }
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/benchmarks/corpus.schema.v1.json fixtures/schemas/benchmarks/corpus/v1/valid/basic.json
python3 scripts/schema-check.py schemas/benchmarks/run.schema.v1.json fixtures/schemas/benchmarks/run/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- No item can enter a run without rights and split assignment.
- Near-duplicate split leakage is a validation error.
- Unrun metrics cannot be pass.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-001: freeze-the-benchmark-taxonomy-dataset-split-and-result-sch`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-002 [S] — Create the rights-bound golden project corpus manifest

**Depends on:** CR-V2-B4-001  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B4-002: create-the-rights-bound-golden-project-corpus-manifest`  
**Stop-loss ceiling:** at most 220 files and 50000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `benchmarks/corpus/manifest.json`
- `benchmarks/corpus/rights/**`
- `benchmarks/corpus/README.md`
- `scripts/benchmarks/validate-corpus.py`

**Procedure**

1. Populate the minimum project categories from the v2 benchmark plan using only media the user owns or has permission to use.
2. Hash source bytes, labels, expected outputs and consent/provenance records.
3. Create placeholders only as non-runnable `missing_fixture` rows; runnable benchmark reports must exclude and report them.
4. Block public redistribution for private fixtures while permitting local evaluation.

**Required implementation shape**

```text
{"project_id":"golden-recorded-001","lane":"recorded","split":"test","source_hashes":["blake3:..."],"rights_ref":"rights/golden-recorded-001.json","redistributable":false}
```

**Commands for this task**

```bash
python3 scripts/benchmarks/validate-corpus.py benchmarks/corpus/manifest.json
```

**Acceptance — inspect and run only the listed focused checks**

- All runnable items have local bytes and rights records.
- Private and redistributable fixtures are separated.
- Split leakage and duplicate source hashes fail.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-002: create-the-rights-bound-golden-project-corpus-manifest`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-003 [S] — Freeze metric definitions, units, aggregation, and release floors

**Depends on:** CR-V2-B4-002  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B4-003: freeze-metric-definitions-units-aggregation-and-release-fl`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `benchmarks/metrics/registry.json`
- `schemas/benchmarks/metric.schema.v1.json`
- `docs/benchmarks/V2-METRICS.md`

**Procedure**

1. Define formula, unit, direction, aggregation, missing-data policy, slice dimensions and initial floor for every metric.
2. Separate product floors from research diagnostics and user preference metrics.
3. Require p50/p90/p95 or confusion matrices where averages hide failure tails.
4. Version threshold changes and forbid retroactive rewriting of prior reports.

**Required implementation shape**

```text
{"id":"speech.word_clipping.high_confidence","unit":"count","direction":"lower_is_better","release_floor":{"max":0},"slices":["language","noise","format"]}
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/benchmarks/metric.schema.v1.json benchmarks/metrics/registry.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every result metric resolves to one registry entry.
- No floor is encoded only in test code.
- Threshold changes create a new profile version.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-003: freeze-metric-definitions-units-aggregation-and-release-fl`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-004 [S] — Freeze evaluator independence, evidence citation, and revision policy

**Depends on:** CR-V2-B4-003  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B4-004: freeze-evaluator-independence-evidence-citation-and-revisi`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/benchmarks/V2-EVALUATOR-PROTOCOL.md`
- `schemas/benchmarks/critic-verdict.schema.v1.json`
- `schemas/benchmarks/revision-cycle.schema.v1.json`

**Procedure**

1. Define deterministic evaluators first, Director self-assessment as diagnostic only, and independent critic as separate model/prompt/process.
2. Require findings to cite evidence IDs and exact source/timeline ranges.
3. Permit one bounded revision cycle; second disagreement or low confidence escalates.
4. Blind variant identity where possible and log evaluator version, seed, sampling and pack locks.

**Required implementation shape**

```text
planner → deterministic checks → critic_A → at most one revision → deterministic checks → critic_A → pass or escalate
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/benchmarks/critic-verdict.schema.v1.json fixtures/schemas/benchmarks/critic-verdict/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- A verdict without evidence cannot pass.
- Planner self-score is not counted as independent evidence.
- Revision count is finite and enforced.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-004: freeze-evaluator-independence-evidence-citation-and-revisi`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-005 [S] — Freeze the EditorialPlan, beat, take-score, reorder, and escalation schemas

**Depends on:** CR-V2-B4-004  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B4-005: freeze-the-editorialplan-beat-take-score-reorder-and-escal`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `schemas/editorial/editorial-plan.schema.v2.json`
- `schemas/editorial/beat.schema.v2.json`
- `schemas/editorial/take-score.schema.v2.json`
- `schemas/editorial/escalation.schema.v2.json`
- `docs/architecture/V2-EDITORIAL-PLAN.md`

**Procedure**

1. Define fixed beat labels, selected/alternate takes, signal scores, confidence, ambiguity, source ranges, evidence, output order, repeat flags, chronology status and review flags.
2. Separate semantic selection/order from deterministic boundary compilation.
3. Require every reorder to state truthfulness/chronology rationale and supporting evidence.
4. Preserve canonical transcript and dropped material references.

**Required implementation shape**

```text
pub struct EditorialBeat { pub beat_id: BeatId, pub label: BeatLabel, pub selected_take: CandidateId, pub alternates: Vec<TakeScore>, pub confidence: f32, pub evidence: Vec<EvidenceRef> }
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/editorial/editorial-plan.schema.v2.json fixtures/schemas/editorial/editorial-plan/v2/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- Plan cannot contain an unknown candidate or unbound range.
- Reorders without chronology status fail.
- Drop reasons use a fixed vocabulary.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-005: freeze-the-editorialplan-beat-take-score-reorder-and-escal`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-006 [S] — Freeze Book 4 benchmark/editorial lane ownership

**Depends on:** CR-V2-B4-005  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B4-006: freeze-book-4-benchmark-editorial-lane-ownership`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-4/interface-freeze.md`
- `docs/architecture/V2-BENCHMARK-EDITORIAL-DAG.md`

**Procedure**

1. Assign lane A benchmark evaluators and runner; lane B deterministic candidate/beat/take/boundary modules; lane C narrative Director/critic/confidence/shorts modules.
2. Reserve project/CLI/Studio integration and autonomy profile changes for serial tasks.
3. Freeze evaluator and editorial provider traits.
4. Prevent benchmark code from writing production project state.

**Required implementation shape**

```text
lane_a: crates/video-benchmarks/**
lane_b: crates/video-editorial/src/deterministic/**
lane_c: crates/video-editorial/src/narrative/**
```

**Commands for this task**

```bash
python3 scripts/architecture/check_crate_dag.py docs/architecture/V2-BENCHMARK-EDITORIAL-DAG.md
```

**Acceptance — inspect and run only the listed focused checks**

- Lane roots do not overlap.
- Production code depends on evaluator interfaces, not benchmark fixtures.
- Benchmark runner is read-only against completed project revisions.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-006: freeze-book-4-benchmark-editorial-lane-ownership`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-007 [P-A] — Implement word-boundary and speech-preservation evaluators

**Depends on:** CR-V2-B4-006  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B4-007: implement-word-boundary-and-speech-preservation-evaluators`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-benchmarks/Cargo.toml`
- `crates/video-benchmarks/src/lib.rs`
- `crates/video-benchmarks/src/speech.rs`
- `crates/video-benchmarks/tests/speech.rs`

**Procedure**

1. Compare output cut boundaries with human-labelled words/phonemes and source audio.
2. Compute onset/offset absolute errors, high-confidence clipped words, preserved speech coverage, boundary consensus coverage and transcript integrity.
3. Use aligned source/output mapping from receipts rather than re-transcribing the final as ground truth.
4. Emit exact failed word IDs and waveform windows.

**Required implementation shape**

```text
clipped if output mapping excludes any interval [word.start + tolerance_in, word.end - tolerance_out] for a kept high-confidence word
```

**Commands for this task**

```bash
cargo test -p video-benchmarks --locked speech
```

**Acceptance — inspect and run only the listed focused checks**

- Known clipped fixtures fail with exact word IDs.
- No-cut control fixture reports perfect preservation.
- Metrics are deterministic and sliced by language/noise.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-007: implement-word-boundary-and-speech-preservation-evaluators`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-008 [P-A] — Implement audio-video sync and audio-preservation evaluators

**Depends on:** CR-V2-B4-007  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B4-008: implement-audio-video-sync-and-audio-preservation-evaluato`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-benchmarks/src/audio_visual.rs`
- `crates/video-benchmarks/src/audio.rs`
- `crates/video-benchmarks/tests/audio_visual.rs`

**Procedure**

1. Measure global and local A/V drift, transient/onset alignment, linked clip sync, discontinuities, loudness, true peak, clipping, channels and non-target audio preservation.
2. Compare before/after mapped windows outside declared audio actions.
3. Report lip-sync proxy confidence separately from deterministic container timestamps.
4. Add intentional offset, fade, cut, duck and reverb fixtures.

**Required implementation shape**

```text
joint_sync = container_pts_delta + transient_alignment_delta + optional_lipsync_proxy; report components separately
```

**Commands for this task**

```bash
cargo test -p video-benchmarks --locked audio_visual
```

**Acceptance — inspect and run only the listed focused checks**

- Known offsets and discontinuities are detected.
- Declared fades/effects are not false preservation failures.
- Uncertain lip-sync proxy is not presented as deterministic truth.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-008: implement-audio-video-sync-and-audio-preservation-evaluato`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-009 [P-A] — Implement visual preservation, crop stability, and collision evaluators

**Depends on:** CR-V2-B4-008  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B4-009: implement-visual-preservation-crop-stability-and-collision`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-benchmarks/src/visual.rs`
- `crates/video-benchmarks/src/crop.rs`
- `crates/video-benchmarks/src/collision.rs`
- `crates/video-benchmarks/tests/visual.rs`

**Procedure**

1. Compute non-target frame similarity over mapped unchanged regions, subject/face retention, crop path jerk/acceleration, identity/OCR label preservation and overlay collision.
2. Use evidence tracks and rendered frame samples at boundaries, events and adaptive intervals.
3. Treat intentional colour/effect actions as declared target regions.
4. Require zero unresolved caption/subject/platform-UI collisions for release.

**Required implementation shape**

```text
collision = overlap(overlay_track.box(t), protected_track.box(t)) > threshold for any sampled/refined t
```

**Commands for this task**

```bash
cargo test -p video-benchmarks --locked visual
```

**Acceptance — inspect and run only the listed focused checks**

- Known subject loss, jitter, label drift and collisions fail.
- Declared target effects are scoped correctly.
- Every visual failure cites exact sampled frames and object IDs.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-009: implement-visual-preservation-crop-stability-and-collision`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-010 [P-A] — Implement action atomicity, undo, crash, cache, and offline evaluators

**Depends on:** CR-V2-B4-009  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B4-010: implement-action-atomicity-undo-crash-cache-and-offline-ev`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-benchmarks/src/reliability.rs`
- `crates/video-benchmarks/tests/reliability.rs`
- `scripts/benchmarks/run-fault-matrix.py`

**Procedure**

1. Drive interruption injection through every transaction and job transition.
2. Measure source mutation, old-or-new atomicity, undo hash round-trip, stale rejection, receipt tamper detection, cache invalidation, cancellation, resume and network-attempt count.
3. Run with temporary HOME and empty PATH.
4. Store each fault seed and replay command.

**Required implementation shape**

```text
for injection_point in ALL_POINTS: run_once(injection_point); assert project_state in {old_complete, new_complete}
```

**Commands for this task**

```bash
cargo test -p video-benchmarks --locked reliability
python3 scripts/benchmarks/run-fault-matrix.py --self-test
```

**Acceptance — inspect and run only the listed focused checks**

- All kernel integrity floors are exact zero-failure requirements.
- Every fault is replayable.
- Offline tests detect attempted network or system-tool fallback.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-010: implement-action-atomicity-undo-crash-cache-and-offline-ev`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-011 [P-A] — Implement editorial human-agreement metrics and benchmark runner

**Depends on:** CR-V2-B4-010  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B4-011: implement-editorial-human-agreement-metrics-and-benchmark-`  
**Stop-loss ceiling:** at most 12 files and 2400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-benchmarks/src/editorial.rs`
- `crates/video-benchmarks/src/runner.rs`
- `crates/video-benchmarks/src/report.rs`
- `crates/video-benchmarks/tests/runner.rs`

**Procedure**

1. Compare beat boundaries/labels, duplicate clusters, selected takes, ordering, hooks, payoffs, CTAs and drop reasons against human annotations and decisions.
2. Retain individual reviewer disagreement; compute consensus only where defined.
3. Produce per-project JSONL, metrics, slices, confusion matrices, samples, failures, report and receipt.
4. Never mutate evaluated projects.

**Required implementation shape**

```text
video-bench run --corpus benchmarks/corpus/manifest.json --profile v2-reviewed --packs runtime/packs.lock.json --out benchmarks/runs/<id>
```

**Commands for this task**

```bash
cargo test -p video-benchmarks --locked runner
```

**Acceptance — inspect and run only the listed focused checks**

- Runner output layout matches the benchmark plan.
- Private fixture paths are redacted from shareable reports.
- Two identical runs are byte-stable except declared timestamps/run IDs.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-011: implement-editorial-human-agreement-metrics-and-benchmark-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-012 [P-B] — Implement deterministic beat segmentation from transcript and evidence

**Depends on:** CR-V2-B4-006  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B4-012: implement-deterministic-beat-segmentation-from-transcript-`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-editorial/Cargo.toml`
- `crates/video-editorial/src/lib.rs`
- `crates/video-editorial/src/deterministic/beats.rs`
- `crates/video-editorial/tests/beats.rs`

**Procedure**

1. Generate candidate semantic units from speaker changes, sentence completion, topic embeddings, meaningful pauses and source recording markers.
2. Merge fragments that complete one thought and retain alternative boundaries with confidence.
3. Use local Director later for labels/order, not for arithmetic or source ranges.
4. Write deterministic features to evidence nodes.

**Required implementation shape**

```text
BeatCandidate { range, speaker_ids, normalized_tokens, pause_before, pause_after, topic_vector_ref, completeness_features, evidence_refs }
```

**Commands for this task**

```bash
cargo test -p video-editorial --locked beats
```

**Acceptance — inspect and run only the listed focused checks**

- No beat crosses a source boundary or invalid time range.
- Speaker/topic/pause fixtures produce expected candidates.
- Alternative ambiguity is retained rather than discarded.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-012: implement-deterministic-beat-segmentation-from-transcript-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-013 [P-B] — Implement duplicate-take and restatement clustering

**Depends on:** CR-V2-B4-012  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B4-013: implement-duplicate-take-and-restatement-clustering`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-editorial/src/deterministic/takes.rs`
- `crates/video-editorial/tests/takes.rs`

**Procedure**

1. Normalise tokens while preserving named entities, negation, numbers and semantic differences.
2. Combine token overlap, embedding similarity, temporal proximity and retake markers.
3. Separate exact/near duplicate takes from related restatements and contradictory takes.
4. Record cluster evidence and uncertainty.

**Required implementation shape**

```text
duplicate only if lexical_overlap >= policy.lexical_floor && semantic_similarity >= policy.semantic_floor && contradiction_score < policy.contradiction_ceiling
```

**Commands for this task**

```bash
cargo test -p video-editorial --locked takes
```

**Acceptance — inspect and run only the listed focused checks**

- Contradictory/negated takes are never merged as duplicates.
- Known duplicate clusters meet precision/recall fixtures.
- Cluster IDs and ordering are deterministic.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-013: implement-duplicate-take-and-restatement-clustering`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-014 [P-B] — Implement evidence-backed take scoring and hard-fault disqualification

**Depends on:** CR-V2-B4-013  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B4-014: implement-evidence-backed-take-scoring-and-hard-fault-disq`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-editorial/src/deterministic/scoring.rs`
- `crates/video-editorial/src/deterministic/faults.rs`
- `crates/video-editorial/tests/scoring.rs`

**Procedure**

1. Score delivery, completeness, technical quality and hook/payoff strength from declared evidence features.
2. Disqualify clipped words, source corruption, unusable exposure/audio and identity violations regardless of weighted score.
3. Return component scores, weights, missing-evidence flags and winner margin.
4. Read per-format preference weights only through a versioned policy input.

**Required implementation shape**

```text
take_score = Σ(weight_i * signal_i); if hard_faults.non_empty() { status = Disqualified }
```

**Commands for this task**

```bash
cargo test -p video-editorial --locked scoring
```

**Acceptance — inspect and run only the listed focused checks**

- Scores are reproducible and explainable.
- Missing technical evidence lowers confidence or escalates; it is not guessed.
- Hard faults override score.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-014: implement-evidence-backed-take-scoring-and-hard-fault-disq`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-015 [P-B] — Implement contextual filler, false-start, slate, handling, and dead-air decisions

**Depends on:** CR-V2-B4-014  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B4-015: implement-contextual-filler-false-start-slate-handling-and`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-editorial/src/deterministic/disfluency.rs`
- `crates/video-editorial/src/deterministic/dead_air.rs`
- `crates/video-editorial/tests/disfluency.rs`

**Procedure**

1. Classify isolated filler versus emphasis/discourse use, abandoned false starts with/without a complete replacement, slate/setup, handling and dead air.
2. Use transcript events, syntax, neighbouring words, VAD, replacement clusters and source handling evidence.
3. Return automatic, suggest-only or preserve decisions according to format policy and confidence.
4. Never delete words from the canonical transcript.

**Required implementation shape**

```text
pub enum RemovalTier { Automatic, SuggestOnly, Preserve }
```

**Commands for this task**

```bash
cargo test -p video-editorial --locked disfluency
```

**Acceptance — inspect and run only the listed focused checks**

- Emphasis fillers and laughter/reactions are preserved in fixtures.
- A false start is automatic only when a complete replacement exists.
- Decisions cite transcript/VAD evidence.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-015: implement-contextual-filler-false-start-slate-handling-and`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-016 [P-B] — Implement boundary consensus and word-safe segment compilation

**Depends on:** CR-V2-B4-015  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B4-016: implement-boundary-consensus-and-word-safe-segment-compila`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-editorial/src/deterministic/boundaries.rs`
- `crates/video-editorial/tests/boundaries.rs`
- `fixtures/editorial/cutaway-golden/**`

**Procedure**

1. Migrate the supplied Cutaway behavior: VAD/audio energy identifies speech regions; timed words/verifier evidence identifies complete lexical edges.
2. Compile natural/tight subsegments at word gaps while preserving whole words and clamping pads away from neighbouring words.
3. Record provider agreement, pad policy, ambiguity and fallback path for every boundary.
4. Fail destructive automation when policy requires manual review.

**Required implementation shape**

```text
speech regions from VAD ∩ selected editorial ranges → overlapping complete words → safe lead/tail clamp → variant gap split
```

**Commands for this task**

```bash
cargo test -p video-editorial --locked boundaries
```

**Acceptance — inspect and run only the listed focused checks**

- Golden Cutaway fixtures match native segment intent.
- No kept high-confidence word is clipped.
- Wordless camera-move/silence regions are dropped only with evidence.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-016: implement-boundary-consensus-and-word-safe-segment-compila`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-017 [P-C] — Implement narrative arc templates and schema-bound Director requests

**Depends on:** CR-V2-B4-006  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B4-017: implement-narrative-arc-templates-and-schema-bound-directo`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-editorial/src/narrative/arcs.rs`
- `crates/video-editorial/src/narrative/provider.rs`
- `crates/video-editorial/tests/arcs.rs`
- `skills/editorial-director/**`

**Procedure**

1. Implement the approved arc library for long-form, shorts, explainers, ads and stories as constrained templates.
2. Build a Director request containing bounded summaries, candidate beats/takes, user brief, format constraints and evidence references.
3. Require schema-bound beat labels, selected takes, ordering and rationale; the Director cannot emit raw timestamps.
4. Retain user-provided/recorded wording; do not fabricate spoken hooks.

**Required implementation shape**

```text
pub trait EditorialDirector { fn propose(&self, request: EditorialRequest) -> Result<EditorialProposal>; }
```

**Commands for this task**

```bash
cargo test -p video-editorial --locked arcs
python3 tools/v2-evals/run.py --suite editorial-director
```

**Acceptance — inspect and run only the listed focused checks**

- Every arc has valid minimum/maximum required roles.
- Director output can reference only supplied candidate IDs.
- Invalid JSON/order/role plans fail.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-017: implement-narrative-arc-templates-and-schema-bound-directo`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-018 [P-C] — Implement hook, payoff, CTA, and truthfulness-aware ordering

**Depends on:** CR-V2-B4-017  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B4-018: implement-hook-payoff-cta-and-truthfulness-aware-ordering`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-editorial/src/narrative/order.rs`
- `crates/video-editorial/src/narrative/hook.rs`
- `crates/video-editorial/src/narrative/truthfulness.rs`
- `crates/video-editorial/tests/order.rs`

**Procedure**

1. Rank existing hook/payoff candidates for specificity, promise, self-containment and evidence.
2. Allow cold-open reorder only when chronology/causality remains truthful and transitions are coherent.
3. Log from/to index, reason, claim dependencies and chronology status.
4. Escalate weak/generic hooks when no strong recorded line exists.

**Required implementation shape**

```text
if reorder.implies_false_sequence || reorder.breaks_claim_dependency { return Escalation::TruthfulnessRisk; }
```

**Commands for this task**

```bash
cargo test -p video-editorial --locked order
```

**Acceptance — inspect and run only the listed focused checks**

- False-chronology adversarial fixtures are rejected.
- Every accepted reorder has a log and evidence.
- No fabricated hook text enters the timeline.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-018: implement-hook-payoff-cta-and-truthfulness-aware-ordering`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-019 [P-C] — Implement semantic short-form candidate discovery and ranking

**Depends on:** CR-V2-B4-018  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B4-019: implement-semantic-short-form-candidate-discovery-and-rank`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-editorial/src/narrative/shorts.rs`
- `crates/video-editorial/tests/shorts.rs`
- `skills/shorts-director/**`

**Procedure**

1. Generate self-contained windows from semantic beats, claim/payoff structure, speaker continuity and platform duration constraints.
2. Rank hook strength, standalone context, payoff, emotional/novel value, visual support, boundary confidence and duplication.
3. Return rationale, score components, evidence, title/hook placeholders and exclusion reasons.
4. Do not let the model compute source timestamps; Rust compiles selected beat IDs.

**Required implementation shape**

```text
short score = hook + standalone_context + payoff + visual_support + boundary_confidence - duplication_penalty
```

**Commands for this task**

```bash
cargo test -p video-editorial --locked shorts
python3 tools/v2-evals/run.py --suite shorts-director
```

**Acceptance — inspect and run only the listed focused checks**

- Candidates are self-contained in labelled fixtures.
- Overlapping/redundant outputs are diversity-filtered.
- Source ranges are compiled from evidence-bound beats.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-019: implement-semantic-short-form-candidate-discovery-and-rank`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-020 [P-C] — Implement confidence, ambiguity, escalation, and one-cycle reflection

**Depends on:** CR-V2-B4-019  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B4-020: implement-confidence-ambiguity-escalation-and-one-cycle-re`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-editorial/src/narrative/confidence.rs`
- `crates/video-editorial/src/narrative/critic.rs`
- `crates/video-editorial/tests/confidence.rs`

**Procedure**

1. Combine take margin, evidence availability/agreement, boundary confidence, schema validity, critic findings and truthfulness checks.
2. Emit named ambiguity flags and escalations with blocking policy by review mode.
3. Run independent critic over the proposal and samples; permit one bounded revision request.
4. Never suppress an escalation to keep a run clean.

**Required implementation shape**

```text
effective_mode = requested_mode.degrade_one_step_if(escalations.blocking() || critic.requires_human)
```

**Commands for this task**

```bash
cargo test -p video-editorial --locked confidence
```

**Acceptance — inspect and run only the listed focused checks**

- Threshold edge fixtures are exact.
- Second critic disagreement escalates.
- Missing evidence cannot increase confidence.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-020: implement-confidence-ambiguity-escalation-and-one-cycle-re`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-021 [P-C] — Merge Book 4 lanes and implement the EditorialEngine façade

**Depends on:** CR-V2-B4-020  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B4-021: merge-book-4-lanes-and-implement-the-editorialengine-fa-ad`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-editorial/src/engine.rs`
- `crates/video-editorial/src/plan.rs`
- `Cargo.toml`
- `docs/dispatch/v2/book-4/merge-receipt.md`

**Procedure**

1. Apply lanes A, B and C in fixed order.
2. Implement `EditorialEngine` sequence: retrieve evidence, deterministic candidates/features, Director proposal, schema/semantic validation, critic, bounded revision, final plan.
3. Write no project file directly; return canonical artefacts to the job/project layer.
4. Record merge conflicts and resolved frozen interfaces.

**Required implementation shape**

```text
pub fn plan(&self, request: EditorialEngineRequest) -> Result<EditorialEngineResult>
```

**Commands for this task**

```bash
cargo check -p video-editorial -p video-benchmarks --locked
```

**Acceptance — inspect and run only the listed focused checks**

- Facade returns plan, candidates, confidence, escalations and retrieval receipt.
- No model output bypasses deterministic validation.
- Merge receipt is complete.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-021: merge-book-4-lanes-and-implement-the-editorialengine-fa-ad`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-022 [S] — Compile natural, tight, long-form, and short variants through the action kernel

**Depends on:** CR-V2-B4-011, CR-V2-B4-016, CR-V2-B4-021  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B4-022: compile-natural-tight-long-form-and-short-variants-through`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-project/src/editorial_v2.rs`
- `crates/video-project/src/cut_plan.rs`
- `crates/video-project/src/timeline.rs`
- `crates/video-project/tests/editorial_v2.rs`

**Procedure**

1. Convert a validated EditorialPlan and boundary consensus into ActionBatches and variant-specific timelines.
2. Preserve all variants and bind each to the plan, evidence graph revision, policy and pack locks.
3. Use natural/tight gap and pad policies as configuration, not duplicated algorithms.
4. Write through the shared executor and revision store.

**Required implementation shape**

```text
EditorialPlan → compile_variant(policy) → ActionBatch → ActionExecutor → RevisionId
```

**Commands for this task**

```bash
cargo test -p video-project --locked editorial_v2
```

**Acceptance — inspect and run only the listed focused checks**

- Plan survivors/order match timeline segments exactly.
- Variants cannot contaminate one another.
- Every timeline action has evidence and receipt.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-022: compile-natural-tight-long-form-and-short-variants-through`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-023 [S] — Implement benchmark profiles that keep autonomy disabled until earned

**Depends on:** CR-V2-B4-022  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B4-023: implement-benchmark-profiles-that-keep-autonomy-disabled-u`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `benchmarks/profiles/reviewed-v2.json`
- `benchmarks/profiles/review-light-v2.json`
- `benchmarks/profiles/autonomous-v2.json`
- `crates/video-benchmarks/src/profile.rs`
- `crates/video-project/src/autonomy_guard.rs`

**Procedure**

1. Encode initial floors from the benchmark plan and existing autonomy ladder.
2. Require integrity/safety floors for every mode and human acceptance history for review-light/autonomous.
3. Block autonomous execution when the exact format, model/skill/renderer pack set or profile version lacks compatible evidence.
4. Allow downgrade, never automatic upgrade.

**Required implementation shape**

```text
if !evidence.compatible_with(format, pack_set, profile) { return ReviewMode::Reviewed; }
```

**Commands for this task**

```bash
cargo test -p video-benchmarks -p video-project --locked autonomy_guard
```

**Acceptance — inspect and run only the listed focused checks**

- A new format always resolves reviewed.
- Pack/profile change invalidates affected autonomy evidence.
- Only an explicit user-approved advancement record upgrades mode.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-023: implement-benchmark-profiles-that-keep-autonomy-disabled-u`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-024 [S] — Expose benchmark and editorial evidence read models to Studio

**Depends on:** CR-V2-B4-023  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B4-024: expose-benchmark-and-editorial-evidence-read-models-to-stu`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src-tauri/src/editorial_commands.rs`
- `apps/studio/src/contracts/editorial.ts`
- `apps/studio/src/lib/editorial.ts`
- `apps/studio/src-tauri/src/tests/editorial_commands.rs`

**Procedure**

1. Add bounded reads for beats, takes, score components, alternatives, reorder logs, review flags, escalations, benchmark metrics, samples and failures.
2. Never expose raw hidden model reasoning.
3. Return stable evidence IDs and exact time ranges for seek/inspection.
4. Keep mutation through ActionExecutor only.

**Required implementation shape**

```text
get_editorial_beats(project, revision, offset, limit) -> BeatPage
get_benchmark_findings(run_id, project_id, metric_id) -> FindingPage
```

**Commands for this task**

```bash
cargo test --manifest-path apps/studio/src-tauri/Cargo.toml --locked editorial_commands
pnpm --dir apps/studio test -- --run editorial
```

**Acceptance — inspect and run only the listed focused checks**

- Large plans are windowed/paginated.
- Every displayed rationale traces to plan/evidence fields.
- Read commands cannot mutate project state.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-024: expose-benchmark-and-editorial-evidence-read-models-to-stu`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-025 [S] — Run the first full v2 benchmark acceptance suite

**Depends on:** CR-V2-B4-024  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B4-025: run-the-first-full-v2-benchmark-acceptance-suite`  
**Stop-loss ceiling:** at most 500 files and 80000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `benchmarks/runs/book-4-acceptance/**`
- `docs/dispatch/v2/book-4/acceptance-summary.md`

**Procedure**

1. Run all available golden projects with reviewed mode, exact active packs and frozen profile.
2. Produce per-project results, slices, confusion matrices, samples, failures and a report.
3. Do not advance autonomy based on synthetic fixtures; report missing real projects and confidence intervals.
4. Triage failures into kernel, evidence, editorial, critic, pack or label issues without waiving.

**Required implementation shape**

```text
release claim = only metrics with required sample count and status pass; all others remain unproven
```

**Commands for this task**

```bash
cargo run -p video-bench -- run --corpus benchmarks/corpus/manifest.json --profile benchmarks/profiles/reviewed-v2.json --out benchmarks/runs/book-4-acceptance
```

**Acceptance — inspect and run only the listed focused checks**

- Kernel integrity floors pass.
- Every missing metric/project is explicit.
- Report binds commit, packs, schemas and profile.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-025: run-the-first-full-v2-benchmark-acceptance-suite`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-026 [S] — Run focused editorial, evaluator, truthfulness, and preservation tests

**Depends on:** CR-V2-B4-025  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B4-026: run-focused-editorial-evaluator-truthfulness-and-preservat`  
**Stop-loss ceiling:** at most 1 file and 1200 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-4/focused-tests.md`

**Procedure**

1. Run video-editorial, video-benchmarks, project variant compilation, Studio read contracts and adversarial chronology/boundary fixtures.
2. Record exact packs, target, model seeds, test totals and benchmark report hash.
3. Do not run the full repository gate in this task.
4. Fix required deterministic failures; keep model variance slices visible.

**Required implementation shape**

```text
focused evidence includes: commands, exit codes, pack locks, seeds, report hash, unsupported slices
```

**Commands for this task**

```bash
cargo test -p video-editorial -p video-benchmarks -p video-project --locked
cargo test --manifest-path apps/studio/src-tauri/Cargo.toml --locked editorial_commands
```

**Acceptance — inspect and run only the listed focused checks**

- All required deterministic suites pass.
- Critic tests use frozen fixture outputs or qualified local packs.
- No skipped benchmark axis is presented as pass.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-026: run-focused-editorial-evaluator-truthfulness-and-preservat`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B4-027 [S] — Run the authoritative Book 4 local gate and freeze benchmark evidence

**Depends on:** CR-V2-B4-026  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B4-027: run-the-authoritative-book-4-local-gate-and-freeze-benchma`  
**Stop-loss ceiling:** at most 2 files and 1500 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-4/final-gate.md`
- `docs/dispatch/v2/book-4/final-manifest.json`

**Procedure**

1. Run corpus validation, benchmark acceptance, capability drift, runtime boundary and focused tests.
2. Run the authoritative local gate exactly once.
3. Record failed/unproven benchmark claims honestly; Book close requires integrity floors, not perfect editorial autonomy.
4. Do not create CI or publish.

**Required implementation shape**

```text
book: 4
benchmark_profile: reviewed-v2
autonomy_auto_advance: false
ci: forbidden
```

**Commands for this task**

```bash
python3 scripts/benchmarks/validate-corpus.py benchmarks/corpus/manifest.json
cargo run -p video-bench -- report --run benchmarks/runs/book-4-acceptance
bash scripts/gate.sh --with-qa
```

**Acceptance — inspect and run only the listed focused checks**

- Kernel integrity and release-blocking benchmark floors pass.
- Autonomy remains reviewed unless real evidence satisfies advancement.
- Final manifest binds report and commit hashes.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B4-027: run-the-authoritative-book-4-local-gate-and-freeze-benchma`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.
