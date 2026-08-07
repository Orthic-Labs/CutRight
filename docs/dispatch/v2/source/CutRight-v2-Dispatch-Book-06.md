# CutRight v2 Dispatch Book 6: Full Studio Authoring Surface, Embedded Agent, and Optional MCP

**Tasks:** 27  
**Goal:** Productize the engine as a coherent desktop workflow with corrective editing, bounded evidence inspection, one-click production, and one shared typed agent tool surface.  
**Execution default:** A single agent runs numeric order. The labels show safe parallelism for an authorised dispatcher with isolated checkouts; this document does not itself authorise new branches or worktrees.  
**Testing cadence:** Focused checks run inside tasks. The full authoritative local gate runs only in task `CR-V2-B6-027`.  
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
CR-V2-B6-001 .. 006    sequential contract/interface freeze
CR-V2-B6-007 .. 011    parallel lane A
CR-V2-B6-012 .. 016    parallel lane B
CR-V2-B6-017 .. 021    parallel lane C
CR-V2-B6-022 .. 027    sequential merge, integration, acceptance, gate
```

## CR-V2-B6-001 [S] — Freeze Studio information architecture, route state, and project read model

**Depends on:** Book 5 final gate  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B6-001: freeze-studio-information-architecture-route-state-and-pro`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/product/V2-STUDIO-IA.md`
- `schemas/studio/navigation.schema.v1.json`
- `schemas/studio/project-view.schema.v1.json`
- `apps/studio/src/contracts/navigation.ts`

**Procedure**

1. Freeze routes/modes: Home, Sources, Transcript, Story, Beats, Timeline, Design, Motion & Sound, Compare, Finals, QA & Receipts, Settings.
2. Define one project read model assembled from canonical revision/evidence/job/decision data and disposable index metadata.
3. Define deep links by stable project/revision/timeline/evidence IDs.
4. Keep selection/playhead/UI state separate from canonical project state.

**Required implementation shape**

```text
type StudioMode = "home"|"sources"|"transcript"|"story"|"beats"|"timeline"|"design"|"motion-sound"|"compare"|"finals"|"qa"|"settings";
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/studio/navigation.schema.v1.json fixtures/schemas/studio/navigation/v1/valid/basic.json
pnpm --dir apps/studio typecheck
```

**Acceptance — inspect and run only the listed focused checks**

- Every mode has a required capability/read model.
- Unknown or unavailable mode degrades visibly.
- UI state cannot overwrite canonical project JSON.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-001: freeze-studio-information-architecture-route-state-and-pro`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-002 [S] — Freeze project library and disposable index contracts

**Depends on:** CR-V2-B6-001  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B6-002: freeze-project-library-and-disposable-index-contracts`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `schemas/studio/project-index.schema.v1.json`
- `docs/architecture/V2-PROJECT-INDEX.md`
- `apps/studio/src/contracts/projectIndex.ts`

**Procedure**

1. Define recent projects, title, project identity, source thumbnail, lane, active revision, run/job status, ready/review/failure counts and updated time.
2. Make SQLite/search metadata a disposable projection rebuilt from project packages and app-local history.
3. Define create/open/rename/remove-from-list; source/project deletion remains separate explicit destructive action.
4. Define watch-folder import as optional and disabled by default.

**Required implementation shape**

```text
ProjectIndexRow { project_instance_id, package_path, title, lane, active_revision, run_status, ready_count, needs_review_count, failed_count, updated_at }
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/studio/project-index.schema.v1.json fixtures/schemas/studio/project-index/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- Deleting the index loses no project truth.
- Two same-title projects remain distinct.
- Project card status derives from jobs/digests, not free-form strings.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-002: freeze-project-library-and-disposable-index-contracts`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-003 [S] — Freeze Studio action binding, optimistic state, and semantic-diff UX

**Depends on:** CR-V2-B6-002  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B6-003: freeze-studio-action-binding-optimistic-state-and-semantic`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/product/V2-STUDIO-ACTIONS.md`
- `schemas/studio/action-intent.schema.v1.json`
- `apps/studio/src/contracts/actionIntent.ts`

**Procedure**

1. Define frontend intent → backend-generated/validated ActionBatch or explicit user-authored batch.
2. Show semantic diff before risky/multi-object actions and allow direct apply for low-risk reversible local actions according to policy.
3. Patch UI from persisted ActionResult, never from assumed optimistic success.
4. Define stale revision refresh and conflict UX.

**Required implementation shape**

```text
intent → backend builds batch against observed revision → dry-run → optional confirm → execute → persisted result → UI patch
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/studio/action-intent.schema.v1.json fixtures/schemas/studio/action-intent/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every UI mutation resolves to a registered action.
- Failed action leaves UI aligned to persisted state after refresh.
- Semantic diff uses shared schema.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-003: freeze-studio-action-binding-optimistic-state-and-semantic`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-004 [S] — Freeze timeline authoring UX and corrective operation scope

**Depends on:** CR-V2-B6-003  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B6-004: freeze-timeline-authoring-ux-and-corrective-operation-scop`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/product/V2-TIMELINE-UX.md`
- `schemas/studio/timeline-view.schema.v1.json`
- `apps/studio/src/contracts/timeline.ts`

**Procedure**

1. Define tracks, clips, linked audio, overlays, captions, music/SFX, stable IDs, frame/tick mapping, gaps, selection and keyframes.
2. Scope v2 corrections: trim, split, remove/ripple, restore, move, swap take/media, reorder beat, volume/fade, crop/reframe anchor, graphic/caption edit, enable/disable effect and undo/redo.
3. Define composited inspection and source inspection separately.
4. Do not promise full NLE breadth beyond registered actions.

**Required implementation shape**

```text
source positions: RationalTime in source timebase
timeline positions: RationalTime in project timebase
conversion: kernel only
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/studio/timeline-view.schema.v1.json fixtures/schemas/studio/timeline-view/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every corrective operation has an action ID and acceptance case.
- Source and timeline units are explicit.
- Linked media behavior is not inferred from track indexes.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-004: freeze-timeline-authoring-ux-and-corrective-operation-scop`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-005 [S] — Freeze embedded agent UX, session, planning, and approval policy

**Depends on:** CR-V2-B6-004  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B6-005: freeze-embedded-agent-ux-session-planning-and-approval-pol`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/product/V2-EMBEDDED-AGENT.md`
- `schemas/agent/turn.schema.v1.json`
- `schemas/agent/plan.schema.v1.json`
- `schemas/agent/tool-result.schema.v1.json`

**Procedure**

1. Define one project-bound agent session with bounded conversation, evidence retrieval, plan, tool calls, semantic diffs, action results and critic findings.
2. Require plan preview for multi-stage production and generation; reversible corrective edits may execute under current review policy.
3. Define concise outcome-first communication and exact escalation questions.
4. Keep raw chain-of-thought out of project artefacts.

**Required implementation shape**

```text
AgentPlan { goal, format, evidence_queries, proposed_tools, expected_actions, review_points, resource_budget }
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/agent/plan.schema.v1.json fixtures/schemas/agent/plan/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- Agent cannot act without a project/session binding.
- Every write tool returns an ActionResult.
- Costs/spend/network actions are absent from offline v2.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-005: freeze-embedded-agent-ux-session-planning-and-approval-pol`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-006 [S] — Freeze Book 6 workspace, authoring, and agent lane ownership

**Depends on:** CR-V2-B6-005  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B6-006: freeze-book-6-workspace-authoring-and-agent-lane-ownership`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-6/interface-freeze.md`
- `docs/architecture/V2-STUDIO-AGENT-DAG.md`

**Procedure**

1. Assign lane A Home/Sources/Transcript/Story/Beats/Run/Compare/Finals/QA; lane B Timeline/Design/Motion-Sound/assets/auditions/corrections; lane C embedded agent/tool registry/composited inspection/MCP/accessibility/performance.
2. Reserve root navigation, job integration, package build and full workflow acceptance for serial tasks.
3. Freeze shared generated contracts and state hooks.
4. Prevent lanes from editing another mode root.

**Required implementation shape**

```text
lane_a: modes/core-workflow
lane_b: modes/authoring
lane_c: agent + inspection + a11y/perf
```

**Commands for this task**

```bash
python3 scripts/architecture/check_crate_dag.py docs/architecture/V2-STUDIO-AGENT-DAG.md
```

**Acceptance — inspect and run only the listed focused checks**

- Parallel roots are disjoint.
- All modes use shared backend services and action bindings.
- Agent/MCP does not own UI state.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-006: freeze-book-6-workspace-authoring-and-agent-lane-ownership`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-007 [P-A] — Implement Home and the rebuildable project library

**Depends on:** CR-V2-B6-006  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B6-007: implement-home-and-the-rebuildable-project-library`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src/modes/HomeMode.tsx`
- `apps/studio/src/components/ProjectCard.tsx`
- `apps/studio/src/hooks/useProjectLibrary.ts`
- `apps/studio/src-tauri/src/project_index.rs`
- `apps/studio/src/HomeMode.test.tsx`

**Procedure**

1. Implement recent project cards, search/filter, create/open/rename/remove-from-list, lane badges and run status.
2. Rebuild/repair the index from project packages and recent history.
3. Add primary actions for Recorded Footage, Repurpose, Explainer and Anchored Creative.
4. Show missing/corrupt/incompatible pack status with local remediation.

**Required implementation shape**

```text
<ProjectCard status={digest.status} ready={digest.ready} review={digest.needs_review} failed={digest.failed} />
```

**Commands for this task**

```bash
cargo test --manifest-path apps/studio/src-tauri/Cargo.toml --locked project_index
pnpm --dir apps/studio test -- --run HomeMode
```

**Acceptance — inspect and run only the listed focused checks**

- Index deletion/rebuild preserves project discovery.
- Cards use stable project identity.
- No feature asks the user to install external software.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-007: implement-home-and-the-rebuildable-project-library`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-008 [P-A] — Implement Sources and Transcript authoring modes

**Depends on:** CR-V2-B6-007  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B6-008: implement-sources-and-transcript-authoring-modes`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src/modes/SourcesModeV2.tsx`
- `apps/studio/src/modes/TranscriptMode.tsx`
- `apps/studio/src/components/SourceInspector.tsx`
- `apps/studio/src/components/TranscriptEditor.tsx`
- `apps/studio/src/SourcesTranscript.test.tsx`

**Procedure**

1. Show immutable sources, hashes, probe facts, tracks, scene/shot overview, storyboard and source transcript.
2. Support relink by exact hash, transcript text correction with source-word binding, speaker label correction and evidence seek.
3. Separate transcript correction from cut/removal.
4. Window long transcript and evidence lists.

**Required implementation shape**

```text
correct transcript text action changes canonical corrected layer; it does not change source timing without a separate reviewed timing action
```

**Commands for this task**

```bash
pnpm --dir apps/studio test -- --run SourcesTranscript
pnpm --dir apps/studio typecheck
```

**Acceptance — inspect and run only the listed focused checks**

- Source bytes cannot be edited.
- Relink mismatch fails visibly.
- Transcript corrections create actions/revisions and preserve original provider output.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-008: implement-sources-and-transcript-authoring-modes`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-009 [P-A] — Implement Story and Beats modes

**Depends on:** CR-V2-B6-008  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B6-009: implement-story-and-beats-modes`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src/modes/StoryMode.tsx`
- `apps/studio/src/modes/BeatsMode.tsx`
- `apps/studio/src/components/BeatCard.tsx`
- `apps/studio/src/components/TakeComparison.tsx`
- `apps/studio/src/StoryBeats.test.tsx`

**Procedure**

1. Display arc, hook/setup/development/payoff/CTA, selected and alternate takes, score components, confidence, reorder log and review flags.
2. Support take swap, preserve/drop suggestion, beat reorder and escalation resolution through registered actions.
3. Warn/block truthfulness-risk reorder.
4. Seek source and output previews from every beat/take.

**Required implementation shape**

```text
<TakeComparison selected={beat.selected_take} alternates={beat.alternates} signals={take.signals} confidence={beat.confidence} />
```

**Commands for this task**

```bash
pnpm --dir apps/studio test -- --run StoryBeats
pnpm --dir apps/studio typecheck
```

**Acceptance — inspect and run only the listed focused checks**

- All decisions map to stable IDs and actions.
- No hidden model reasoning is displayed as fact.
- Truthfulness warnings show supporting evidence.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-009: implement-story-and-beats-modes`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-010 [P-A] — Implement Run mode and one-click Make Versions workflow

**Depends on:** CR-V2-B6-009  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B6-010: implement-run-mode-and-one-click-make-versions-workflow`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src/modes/RunMode.tsx`
- `apps/studio/src/components/RunGraph.tsx`
- `apps/studio/src/components/RunDigest.tsx`
- `apps/studio/src/hooks/useRun.ts`
- `apps/studio/src/RunMode.test.tsx`

**Procedure**

1. Implement brief/format/pack/review settings and the Make Versions button.
2. Show DAG stages, dependencies, cached/running/retry/review/failure states, resource usage, cancel/resume and exact escalations.
3. Display final digest counts and output links.
4. Never present elapsed time alone as a stuck-job determination.

**Required implementation shape**

```text
Make versions → submit ProductionRunSpec → JobId; UI subscribes/polls canonical job state and digest
```

**Commands for this task**

```bash
pnpm --dir apps/studio test -- --run RunMode
pnpm --dir apps/studio typecheck
```

**Acceptance — inspect and run only the listed focused checks**

- A relaunch reconnects to persistent jobs.
- Cancel/resume state matches backend.
- Ready means ready for review, not published.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-010: implement-run-mode-and-one-click-make-versions-workflow`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-011 [P-A] — Implement Compare, Finals, and QA & Receipts modes

**Depends on:** CR-V2-B6-010  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B6-011: implement-compare-finals-and-qa-receipts-modes`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src/modes/CompareModeV2.tsx`
- `apps/studio/src/modes/FinalsModeV2.tsx`
- `apps/studio/src/modes/QaReceiptsMode.tsx`
- `apps/studio/src/CompareFinalsQa.test.tsx`

**Procedure**

1. Support natural/tight/platform A/B sync, word-aligned swap, sample/critic findings and selected variant.
2. Show finals per preset, package assets/copy and selection history.
3. Show deterministic QA, critic verdict, receipt tree, tampering/stale state and acknowledgement.
4. Keep unselected variants and prior approved finals.

**Required implementation shape**

```text
final selection record binds preset + variant revision + final hash + QA hash + critic verdict hash
```

**Commands for this task**

```bash
pnpm --dir apps/studio test -- --run CompareFinalsQa
pnpm --dir apps/studio typecheck
```

**Acceptance — inspect and run only the listed focused checks**

- Variant swap preserves semantic position.
- QA evidence seeks exact ranges/frames.
- Stale/corrupt/missing are distinct states.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-011: implement-compare-finals-and-qa-receipts-modes`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-012 [P-B] — Implement the non-destructive timeline editor

**Depends on:** CR-V2-B6-006  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B6-012: implement-the-non-destructive-timeline-editor`  
**Stop-loss ceiling:** at most 60 files and 12000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src/modes/TimelineMode.tsx`
- `apps/studio/src/components/timeline/**`
- `apps/studio/src/hooks/useTimeline.ts`
- `apps/studio/src/TimelineMode.test.tsx`

**Procedure**

1. Render virtualised tracks/clips with stable IDs, rational-time conversion, linked audio and overlays.
2. Implement selection, seek, zoom, trim handles, split, remove/ripple, move, restore, swap, volume/fade, keyframe display and undo/redo.
3. Build ActionIntents and show semantic diff according to policy.
4. Patch from persisted ActionResult and refresh on stale revision.

**Required implementation shape**

```text
drag result → ActionIntent::MoveClip { clip_id, target_track_id, target_start } → dry-run → execute
```

**Commands for this task**

```bash
pnpm --dir apps/studio test -- --run TimelineMode
pnpm --dir apps/studio typecheck
```

**Acceptance — inspect and run only the listed focused checks**

- Each supported operation has keyboard and pointer coverage.
- Long timelines remain windowed.
- No operation mutates frontend-only canonical state.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-012: implement-the-non-destructive-timeline-editor`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-013 [P-B] — Implement Design mode

**Depends on:** CR-V2-B6-012  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B6-013: implement-design-mode`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src/modes/DesignMode.tsx`
- `apps/studio/src/components/design/**`
- `apps/studio/src/hooks/useDesign.ts`
- `apps/studio/src/DesignMode.test.tsx`

**Procedure**

1. Show BrandCard/System, style directions, bake-off previews, accepted assets, requests, protected zones and Designer findings.
2. Support select direction, edit text slots, request/regenerate supported asset, replace asset, accept/reject delivery and open evidence.
3. Display rights/provenance and local capability limitations.
4. Never allow design actions to modify editorial cuts.

**Required implementation shape**

```text
DesignMode consumes CreativePlan/AssetRequest/AssetDelivery/AssetReview and emits only registered creative actions
```

**Commands for this task**

```bash
pnpm --dir apps/studio test -- --run DesignMode
pnpm --dir apps/studio typecheck
```

**Acceptance — inspect and run only the listed focused checks**

- Accepted direction/assets are revision-bound.
- Rights or protected-region failures block acceptance.
- Unsupported local generation is explicit.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-013: implement-design-mode`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-014 [P-B] — Implement Motion & Sound mode

**Depends on:** CR-V2-B6-013  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B6-014: implement-motion-sound-mode`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src/modes/MotionSoundMode.tsx`
- `apps/studio/src/components/motion/**`
- `apps/studio/src/components/audio/**`
- `apps/studio/src/MotionSoundMode.test.tsx`

**Procedure**

1. Show motion language, effect slots, triggers, cooldown/density, reduced-motion variant, audio graph, music/SFX, transient markers, loudness and mix.
2. Support enable/disable/replace effect, tune bounded props, move trigger within allowed evidence, set music/SFX level, fades and audition.
3. Render short previews through native graph jobs.
4. Prevent content-cut changes.

**Required implementation shape**

```text
effect controls generated from effect props schema; no arbitrary JSON editor in normal UI
```

**Commands for this task**

```bash
pnpm --dir apps/studio test -- --run MotionSoundMode
pnpm --dir apps/studio typecheck
```

**Acceptance — inspect and run only the listed focused checks**

- Controls are schema-derived and bounded.
- Audition jobs are cancellable/cached.
- Collision/density/sync findings are visible before final render.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-014: implement-motion-sound-mode`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-015 [P-B] — Implement Assets and Auditions panels

**Depends on:** CR-V2-B6-014  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B6-015: implement-assets-and-auditions-panels`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src/components/AssetPanel.tsx`
- `apps/studio/src/components/AuditionPanel.tsx`
- `apps/studio/src/hooks/useAssets.ts`
- `apps/studio/src/AssetsAuditions.test.tsx`

**Procedure**

1. Provide project-wide inventory of source/generated/procedural assets, status, rights, provenance, usage and hashes.
2. Group style, take, crop, effect, audio and final auditions with blinded option where applicable.
3. Record selection/rejection reasons and exact preview hashes.
4. Do not delete used/approved assets; archive through explicit action.

**Required implementation shape**

```text
AssetUsage { asset_id, revision, timeline_refs, render_graph_refs, package_refs }
```

**Commands for this task**

```bash
pnpm --dir apps/studio test -- --run AssetsAuditions
pnpm --dir apps/studio typecheck
```

**Acceptance — inspect and run only the listed focused checks**

- Every used asset shows reverse references.
- Selection record survives relaunch.
- Stale previews are visibly invalidated.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-015: implement-assets-and-auditions-panels`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-016 [P-B] — Implement corrective operation workflows and comprehensive undo UX

**Depends on:** CR-V2-B6-015  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B6-016: implement-corrective-operation-workflows-and-comprehensive`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src/components/CorrectionBar.tsx`
- `apps/studio/src/components/HistoryPanel.tsx`
- `apps/studio/src/hooks/useHistory.ts`
- `apps/studio/src/CorrectionsHistory.test.tsx`

**Procedure**

1. Expose restore removed passage, alternate take, boundary nudge to safe word edge, reorder beat, change crop anchor, fix caption, disable graphic/effect, rerun stage and preference reason.
2. Show action history, semantic summary, producer, revision and undo/redo availability.
3. Require explicit confirmation for actions with external exports or broad downstream invalidation.
4. Use shared executor only.

**Required implementation shape**

```text
CorrectionBar action map is generated from capability registry tags: correction_common=true
```

**Commands for this task**

```bash
pnpm --dir apps/studio test -- --run CorrectionsHistory
pnpm --dir apps/studio typecheck
```

**Acceptance — inspect and run only the listed focused checks**

- Every common correction is possible without editing JSON.
- Undo creates/loads persisted revisions.
- Downstream invalidation is explained before broad actions.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-016: implement-corrective-operation-workflows-and-comprehensive`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-017 [P-C] — Implement embedded agent sessions and the generated tool registry

**Depends on:** CR-V2-B6-006  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B6-017: implement-embedded-agent-sessions-and-the-generated-tool-r`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-agent/src/session.rs`
- `crates/video-agent/src/registry.rs`
- `crates/video-agent/src/turn.rs`
- `crates/video-agent/tests/session.rs`
- `apps/studio/src/contracts/agent.ts`

**Procedure**

1. Load generated tools and skill capabilities from the shared registry.
2. Bind session to project/timeline/revision and retain bounded turn/tool/result history.
3. Patch the agent state from ActionResult deltas; refresh after out-of-band changes.
4. Require exact stable IDs from read tools.

**Required implementation shape**

```text
AgentSession { binding, observed_revision, plan, turn_log_refs, tool_state, token_budget, resource_budget }
```

**Commands for this task**

```bash
cargo test -p video-agent --locked session
```

**Acceptance — inspect and run only the listed focused checks**

- Tool schema hash matches capability registry.
- Cross-project IDs fail.
- Session compaction preserves decisions/tool evidence, not hidden reasoning.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-017: implement-embedded-agent-sessions-and-the-generated-tool-r`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-018 [P-C] — Implement embedded agent planning, evidence retrieval, diff review, and execution

**Depends on:** CR-V2-B6-017  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B6-018: implement-embedded-agent-planning-evidence-retrieval-diff-`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-agent/src/planner.rs`
- `crates/video-agent/src/execution.rs`
- `crates/video-agent/src/communication.rs`
- `crates/video-agent/tests/planner.rs`
- `apps/studio/src/components/AgentPanel.tsx`

**Procedure**

1. Use Director model plus internal skills to create schema-bound AgentPlan.
2. Retrieve evidence through bounded queries, call read tools, propose ActionBatches, show semantic diff when required, execute through shared executor and inspect results.
3. Use outcome-first concise messages and one focused escalation question.
4. Never narrate internal chain-of-thought or fabricate completed actions.

**Required implementation shape**

```text
AgentPlan → bounded reads → SkillExecutor/Director → ActionBatch → dry-run → policy → execute → inspect → communicate
```

**Commands for this task**

```bash
cargo test -p video-agent --locked planner
pnpm --dir apps/studio test -- --run AgentPanel
```

**Acceptance — inspect and run only the listed focused checks**

- Agent cannot call unregistered tools.
- Every claimed edit has an ActionResult.
- Low-confidence/blocking findings stop with exact evidence.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-018: implement-embedded-agent-planning-evidence-retrieval-diff-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-019 [P-C] — Implement composited timeline inspection and sample sheets

**Depends on:** CR-V2-B6-018  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B6-019: implement-composited-timeline-inspection-and-sample-sheets`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-project/src/inspect.rs`
- `apps/studio/src-tauri/src/inspect_commands.rs`
- `apps/studio/src/components/CompositedInspector.tsx`
- `crates/video-project/tests/inspect.rs`

**Procedure**

1. Render one frame or evenly sampled bounded range from the active timeline through the native graph.
2. Return visible clip/asset/text/caption/effect IDs top-down with frame/time labels and evidence links.
3. Generate storyboard/contact sheets for source, beats, transitions, effects and critic samples.
4. Cache by revision/graph/sample specification.

**Required implementation shape**

```text
inspect_timeline { timeline_id, revision, start, end?, max_frames<=12 } -> samples + visible_object_ids
```

**Commands for this task**

```bash
cargo test -p video-project --locked inspect
pnpm --dir apps/studio test -- --run CompositedInspector
```

**Acceptance — inspect and run only the listed focused checks**

- Rendered images map to exact stable object IDs.
- Window and sample count are bounded.
- Source inspection and composited inspection remain distinct.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-019: implement-composited-timeline-inspection-and-sample-sheets`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-020 [P-C] — Complete optional loopback MCP project navigation and write guards

**Depends on:** CR-V2-B6-019  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B6-020: complete-optional-loopback-mcp-project-navigation-and-writ`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-agent/src/mcp/navigation.rs`
- `crates/video-agent/src/mcp/server.rs`
- `crates/video-agent/tests/mcp_navigation.rs`
- `apps/studio/src-tauri/src/mcp_settings.rs`

**Procedure**

1. Implement list/open/create/close project session operations without deletion.
2. Keep each MCP connection bound to its project; reads remain bound if another window becomes active, writes pause until bound project is frontmost.
3. Rotate ephemeral local token on app restart and expose enable/disable in Settings.
4. Use the exact generated tool registry and executor.

**Required implementation shape**

```text
external session read: bound project allowed
external session write: bound project must equal frontmost project
```

**Commands for this task**

```bash
cargo test -p video-agent --locked mcp_navigation
```

**Acceptance — inspect and run only the listed focused checks**

- Server is disabled by default and loopback-only.
- Per-session project context is isolated.
- Write guard and token failures are typed/audited.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-020: complete-optional-loopback-mcp-project-navigation-and-writ`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-021 [P-C] — Implement accessibility, reduced motion, keyboard, and performance budgets

**Depends on:** CR-V2-B6-020  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B6-021: implement-accessibility-reduced-motion-keyboard-and-perfor`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src/a11y/**`
- `apps/studio/src/performance/**`
- `apps/studio/src/A11yPerformance.test.tsx`
- `docs/product/V2-ACCESSIBILITY-PERFORMANCE.md`

**Procedure**

1. Add focus management, dialog traps, semantic labels, live regions, keyboard equivalents, contrast and screen-reader state for all new modes.
2. Honor application and output reduced-motion settings.
3. Virtualise large transcripts/evidence/timelines and avoid per-frame React state commits.
4. Define and measure initial-load, interaction and memory budgets.

**Required implementation shape**

```text
playhead animation uses refs/requestAnimationFrame; commit React state only at bounded UI cadence
```

**Commands for this task**

```bash
pnpm --dir apps/studio test -- --run A11yPerformance
pnpm --dir apps/studio typecheck
```

**Acceptance — inspect and run only the listed focused checks**

- Critical workflows are keyboard-complete.
- Reduced-motion output/UI paths are tested.
- Large fixtures stay within frozen render/update budgets.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-021: implement-accessibility-reduced-motion-keyboard-and-perfor`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-022 [S] — Merge Book 6 lanes and replace the root Studio navigation

**Depends on:** CR-V2-B6-011, CR-V2-B6-016, CR-V2-B6-021  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B6-022: merge-book-6-lanes-and-replace-the-root-studio-navigation`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src/App.tsx`
- `apps/studio/src/components/ModeRail.tsx`
- `apps/studio/src/hooks/useStudioRouter.ts`
- `docs/dispatch/v2/book-6/merge-receipt.md`

**Procedure**

1. Apply lanes A, B and C in fixed order.
2. Wire all frozen modes, deep links, project library, action/session providers and unavailable-capability states.
3. Retain compatibility route to old review modes only for migrated projects until Book 7 removes/redirects it.
4. Record conflicts and route mapping.

**Required implementation shape**

```text
route state = {project_id?, revision?, timeline_id?, mode, evidence_id?, object_id?}
```

**Commands for this task**

```bash
pnpm --dir apps/studio typecheck
pnpm --dir apps/studio test -- --run router
```

**Acceptance — inspect and run only the listed focused checks**

- Every mode is reachable and guarded by capability.
- Navigation preserves project/revision/selection context.
- Merge receipt is complete.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-022: merge-book-6-lanes-and-replace-the-root-studio-navigation`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-023 [S] — Integrate persistent job progress, recovery, notifications, and digests

**Depends on:** CR-V2-B6-022  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B6-023: integrate-persistent-job-progress-recovery-notifications-a`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src/hooks/useJobs.ts`
- `apps/studio/src/components/JobCenter.tsx`
- `apps/studio/src-tauri/src/job_commands.rs`
- `apps/studio/src/JobCenter.test.tsx`

**Procedure**

1. Subscribe/poll persistent job state and reconnect after relaunch.
2. Show resource/degradation/retry/cancel/resume and exact failed stage/error.
3. Create local system notification only for completed/failed user jobs when enabled.
4. Open digest/project/gate directly from job center.

**Required implementation shape**

```text
JobCenter reads JobPage; commands are cancel(job_id), resume(job_id), open_project(project_id), open_digest(digest_id)
```

**Commands for this task**

```bash
cargo test --manifest-path apps/studio/src-tauri/Cargo.toml --locked job_commands
pnpm --dir apps/studio test -- --run JobCenter
```

**Acceptance — inspect and run only the listed focused checks**

- No job truth lives only in React state.
- Cancellation and resume match backend receipts.
- Notifications contain no sensitive transcript content.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-023: integrate-persistent-job-progress-recovery-notifications-a`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-024 [S] — Create deterministic visual QA fixtures for every Studio mode

**Depends on:** CR-V2-B6-023  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B6-024: create-deterministic-visual-qa-fixtures-for-every-studio-m`  
**Stop-loss ceiling:** at most 220 files and 45000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src/fixtures/v2/**`
- `apps/studio/scripts/qa-v2.mjs`
- `apps/studio/design/v2/**`
- `docs/dispatch/v2/book-6/visual-qa.md`

**Procedure**

1. Create fixed project/job/evidence/action/agent states for normal, empty, loading, degraded, needs-review, failure, stale and corrupt cases.
2. Capture required viewports/selectors in light/dark/reduced-motion and app-only mode.
3. Assert behavior before capture and clean up local servers.
4. Record hashes and review findings.

**Required implementation shape**

```text
fixture IDs and dates are frozen; QA starts local server, asserts state, captures selectors, stops server in finally
```

**Commands for this task**

```bash
pnpm --dir apps/studio qa:v2
```

**Acceptance — inspect and run only the listed focused checks**

- Every mode has deterministic functional and visual evidence.
- No fixture uses network/current time/random IDs.
- Capture failures cannot be marked pass.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-024: create-deterministic-visual-qa-fixtures-for-every-studio-m`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-025 [S] — Run full four-lane Studio workflow tests with the embedded agent

**Depends on:** CR-V2-B6-024  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B6-025: run-full-four-lane-studio-workflow-tests-with-the-embedded`  
**Stop-loss ceiling:** at most 80 files and 14000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `tests/v2/studio_workflows/**`
- `apps/studio/scripts/qa-v2-workflows.mjs`
- `docs/dispatch/v2/book-6/workflow-tests.md`

**Procedure**

1. Drive create/import, Make Versions, Story/Beats review, correction, Design/Motion audition, compare, QA and final selection for all four lanes.
2. Repeat one workflow through embedded agent and one through optional MCP using the same actions.
3. Restart the app during a job and during an uncommitted UI selection.
4. Confirm project truth, job recovery, undo and receipts.

**Required implementation shape**

```text
workflow assertion: final selected revision + QA hash + receipt verification + no source mutation + no external runtime/network
```

**Commands for this task**

```bash
pnpm --dir apps/studio qa:v2:workflows
```

**Acceptance — inspect and run only the listed focused checks**

- All four lanes reach ready or named needs-review state.
- Agent/UI/MCP produce equivalent persisted actions.
- Restart loses no committed work and corrupts no source.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-025: run-full-four-lane-studio-workflow-tests-with-the-embedded`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-026 [S] — Build the local development application bundle with all v2 modes

**Depends on:** CR-V2-B6-025  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B6-026: build-the-local-development-application-bundle-with-all-v2`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `apps/studio/src-tauri/tauri.conf.json`
- `apps/studio/src-tauri/capabilities/v2.json`
- `docs/dispatch/v2/book-6/dev-bundle.md`

**Procedure**

1. Bundle target-specific staged sidecars/resources and generated skill/pack fixture locks for development.
2. Restrict Tauri capabilities to exact dialogs, asset scopes, notifications and loopback MCP settings required.
3. Build app without fetching browser/runtime/model dependencies during build.
4. Open and inspect the resulting application manually through deterministic QA automation.

**Required implementation shape**

```text
Tauri resources: signed pack fixtures + embedded skill pack; externalBin: target-suffixed CutRight-owned sidecars only
```

**Commands for this task**

```bash
pnpm --dir apps/studio tauri build --debug
pnpm --dir apps/studio qa:v2:app
```

**Acceptance — inspect and run only the listed focused checks**

- Bundle contains no production-private source corpus.
- No broad filesystem/shell capability is granted.
- All modes open in the built app.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-026: build-the-local-development-application-bundle-with-all-v2`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B6-027 [S] — Run the authoritative Book 6 local gate and freeze Studio/agent evidence

**Depends on:** CR-V2-B6-026  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B6-027: run-the-authoritative-book-6-local-gate-and-freeze-studio-`  
**Stop-loss ceiling:** at most 2 files and 1600 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-6/final-gate.md`
- `docs/dispatch/v2/book-6/final-manifest.json`

**Procedure**

1. Run capability drift, Studio functional/visual/workflow QA, MCP/session tests, runtime boundary and local development bundle checks.
2. Run the authoritative local gate exactly once.
3. Record screenshots, reports, bundle hash, test totals and unproven platform slices.
4. Do not create CI or publish.

**Required implementation shape**

```text
book: 6
studio_modes: 12
shared_executor_surfaces: [studio, embedded_agent, cli, optional_mcp]
ci: forbidden
```

**Commands for this task**

```bash
bash scripts/gates/v2-capability-drift.sh
pnpm --dir apps/studio qa:v2
pnpm --dir apps/studio qa:v2:workflows
bash scripts/gate.sh --with-qa
```

**Acceptance — inspect and run only the listed focused checks**

- All required host checks pass.
- Agent/UI/MCP share executor and registry.
- Final manifest binds bundle and QA evidence.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B6-027: run-the-authoritative-book-6-local-gate-and-freeze-studio-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.
