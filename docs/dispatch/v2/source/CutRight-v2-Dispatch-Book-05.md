# CutRight v2 Dispatch Book 5: Embedded Creative Operating System and Native Finish Renderer

**Tasks:** 27  
**Goal:** Turn the imported skills into a product-local creative system, implement Designer/brand/script/platform contracts, and replace external render technologies with a CutRight-owned native finish graph.  
**Execution default:** A single agent runs numeric order. The labels show safe parallelism for an authorised dispatcher with isolated checkouts; this document does not itself authorise new branches or worktrees.  
**Testing cadence:** Focused checks run inside tasks. The full authoritative local gate runs only in task `CR-V2-B5-027`.  
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
CR-V2-B5-001 .. 006    sequential contract/interface freeze
CR-V2-B5-007 .. 011    parallel lane A
CR-V2-B5-012 .. 016    parallel lane B
CR-V2-B5-017 .. 021    parallel lane C
CR-V2-B5-022 .. 027    sequential merge, integration, acceptance, gate
```

## CR-V2-B5-001 [S] — Freeze the embedded creative skill execution contract

**Depends on:** Book 4 final gate  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B5-001: freeze-the-embedded-creative-skill-execution-contract`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/architecture/V2-CREATIVE-OS.md`
- `schemas/skills/skill-request.schema.v1.json`
- `schemas/skills/skill-result.schema.v1.json`
- `schemas/skills/skill-trace.schema.v1.json`

**Procedure**

1. Define request, result, trace, permissions, evidence access, resource budget, model capability and typed artefact output.
2. Prohibit direct filesystem/timeline mutation; skills may emit plans, requests, deliveries, reviews and action proposals within permission.
3. Define deterministic skill selection from capability registry and explicit degradation.
4. Log skill/model/resource versions and bounded retrieved evidence.

**Required implementation shape**

```text
pub trait SkillExecutor { fn execute(&self, request: SkillRequest, ctx: &SkillContext) -> Result<SkillResult>; }
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/skills/skill-request.schema.v1.json fixtures/schemas/skills/skill-request/v1/valid/basic.json
python3 scripts/schema-check.py schemas/skills/skill-result.schema.v1.json fixtures/schemas/skills/skill-result/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- A skill cannot request undeclared permissions or model/runtime packs.
- Every result cites input/evidence and output hashes.
- Raw hidden reasoning is not a required artefact.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-001: freeze-the-embedded-creative-skill-execution-contract`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-002 [S] — Freeze creative asset request, delivery, and acceptance schemas

**Depends on:** CR-V2-B5-001  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B5-002: freeze-creative-asset-request-delivery-and-acceptance-sche`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `schemas/creative/asset-request.schema.v2.json`
- `schemas/creative/asset-delivery.schema.v2.json`
- `schemas/creative/asset-review.schema.v2.json`
- `docs/architecture/V2-ASSET-CONTRACTS.md`

**Procedure**

1. Define asset kind, purpose, exact dimensions/aspects, alpha, duration, source/evidence links, text slots, brand refs, safe/protected zones, identity/OCR locks, allowed transformations and required variants.
2. Define delivery files, provenance, generator, prompt/config, hash, rights, semantic inspection and acceptance status.
3. Require each generated/delivered asset to remain immutable; revisions receive new IDs.
4. Separate source asset, preview, proxy and final delivery.

**Required implementation shape**

```text
pub struct AssetRequest { pub id: AssetRequestId, pub kind: AssetKind, pub purpose: String, pub outputs: Vec<OutputSpec>, pub protected: ProtectedRegions, pub brand: BrandCardRef, pub evidence: Vec<EvidenceRef> }
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/creative/asset-request.schema.v2.json fixtures/schemas/creative/asset-request/v2/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- Missing size, rights, protected zones or provenance fails.
- Timeline/cut fields are not writable by Designer.
- Accepted delivery binds exact file hashes.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-002: freeze-creative-asset-request-delivery-and-acceptance-sche`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-003 [S] — Freeze BrandCard, BrandSystem, style direction, and bake-off schemas

**Depends on:** CR-V2-B5-002  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B5-003: freeze-brandcard-brandsystem-style-direction-and-bake-off-`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `schemas/creative/brand-card.schema.v2.json`
- `schemas/creative/brand-system.schema.v2.json`
- `schemas/creative/style-direction.schema.v2.json`
- `schemas/creative/bakeoff.schema.v2.json`

**Procedure**

1. Define voice, visual tokens, typography, palette, marks, motion language, audio identity, restrictions, accessibility and provenance.
2. Define materially divergent style directions and one selected direction with user/critic acceptance.
3. Define bake-off fixtures with same content/geometry to compare style rather than content changes.
4. Require locked brand assets to remain immutable.

**Required implementation shape**

```text
StyleDirection { signature_mechanism, palette, typography, texture, composition, motion_language, audio_language, restrictions }
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/creative/brand-card.schema.v2.json fixtures/schemas/creative/brand-card/v2/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every style token traces to a brand/source or explicit exploration.
- Bake-off variants change only declared dimensions.
- Locked marks/type/palette cannot be overwritten by a style direction.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-003: freeze-brandcard-brandsystem-style-direction-and-bake-off-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-004 [S] — Freeze the native declarative render graph and effect DSL

**Depends on:** CR-V2-B5-003  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B5-004: freeze-the-native-declarative-render-graph-and-effect-dsl`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `schemas/render/render-graph.schema.v2.json`
- `schemas/render/node.schema.v2.json`
- `schemas/render/effect.schema.v2.json`
- `docs/architecture/V2-NATIVE-RENDER-GRAPH.md`

**Procedure**

1. Define source, transform, crop, mask, text, vector, image, video, transition, colour, audio, caption, effect, composite and output nodes.
2. Use rational time, stable inputs, deterministic parameters, explicit colour/alpha spaces and bounded resource estimates.
3. Define safe zones, protected tracks, reduced-motion behavior and semantic trigger links.
4. Prohibit arbitrary shell, JavaScript, HTML, CSS, network fetch and executable path in graph nodes.

**Required implementation shape**

```text
#[serde(tag="type", rename_all="snake_case")] enum RenderNode { Source, Transform, Mask, Text, Vector, Image, Video, Transition, Caption, Color, Audio, Composite, Output }
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/render/render-graph.schema.v2.json fixtures/schemas/render/render-graph/v2/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every node type has strict props and validation.
- Graph cycles are rejected except declared feedback-free audio chains if supported.
- No legacy renderer name is executable.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-004: freeze-the-native-declarative-render-graph-and-effect-dsl`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-005 [S] — Freeze creative critic, visual QA, and finish-lock semantics

**Depends on:** CR-V2-B5-004  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B5-005: freeze-creative-critic-visual-qa-and-finish-lock-semantics`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `schemas/creative/creative-verdict.schema.v1.json`
- `schemas/creative/finish-plan.schema.v2.json`
- `docs/architecture/V2-FINISH-LOCK.md`

**Procedure**

1. Declare that finish begins from an immutable editorial timeline revision and may add/modify only finish tracks, transforms and declared audio/colour treatment.
2. Any content cut/source range/order change requires a new editorial action and invalidates the finish review.
3. Define visual/creative verdict categories, evidence, confidence and revision request.
4. Require deterministic collision/legibility/rights checks before model critique.

**Required implementation shape**

```text
assert current.editorial_revision_hash == finish_plan.base_editorial_revision_hash;
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/creative/finish-plan.schema.v2.json fixtures/schemas/creative/finish-plan/v2/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- Finish plan references one locked base revision hash.
- Cut-changing actions are schema/permission invalid inside finish skill.
- Critic findings cite frame/time/object IDs.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-005: freeze-creative-critic-visual-qa-and-finish-lock-semantics`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-006 [S] — Freeze Book 5 creative skill, planning, and renderer lane ownership

**Depends on:** CR-V2-B5-005  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B5-006: freeze-book-5-creative-skill-planning-and-renderer-lane-ow`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-5/interface-freeze.md`
- `docs/architecture/V2-CREATIVE-RENDER-DAG.md`

**Procedure**

1. Assign lane A creative skill executors and brand/writing/social modules; lane B creative planning/assets/A-B-C-roll; lane C native compositor/text/motion/audio/render graph.
2. Reserve final finish integration, critic, project actions and acceptance for serial tasks.
3. Freeze public artefact and renderer traits.
4. Ensure renderer never depends on skill runtime.

**Required implementation shape**

```text
skills/planners → typed creative artefacts → graph compiler → native renderer; no reverse dependency
```

**Commands for this task**

```bash
python3 scripts/architecture/check_crate_dag.py docs/architecture/V2-CREATIVE-RENDER-DAG.md
```

**Acceptance — inspect and run only the listed focused checks**

- Parallel roots do not overlap.
- Skills depend on plans/contracts; renderer depends only on validated graph/assets.
- Frozen traits match tasks 001–005.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-006: freeze-book-5-creative-skill-planning-and-renderer-lane-ow`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-007 [P-A] — Implement the product-local skill runtime and resolver

**Depends on:** CR-V2-B5-006  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B5-007: implement-the-product-local-skill-runtime-and-resolver`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-skills/Cargo.toml`
- `crates/video-skills/src/lib.rs`
- `crates/video-skills/src/runtime.rs`
- `crates/video-skills/src/resolver.rs`
- `crates/video-skills/tests/runtime.rs`

**Procedure**

1. Load only the compiled embedded skill catalogue from signed creative pack resources.
2. Resolve dependencies, permissions, eval suite, model/runtime capability and bounded resources.
3. Execute with project-scoped evidence and output staging; prohibit arbitrary path access.
4. Emit canonical skill trace and result.

**Required implementation shape**

```text
pub struct SkillContext { pub project: ProjectScope, pub revision: RevisionId, pub evidence: EvidenceService, pub capabilities: CapabilityView, pub output_staging: StagingScope }
```

**Commands for this task**

```bash
cargo test -p video-skills --locked runtime
```

**Acceptance — inspect and run only the listed focused checks**

- Unknown/mismatched/hash-corrupt skills fail.
- The runtime cannot read outside approved project/pack paths.
- Same inputs/seed/pack produce stable structured outputs where deterministic mode is declared.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-007: implement-the-product-local-skill-runtime-and-resolver`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-008 [P-A] — Implement Brand and Brand Identity skills as typed services

**Depends on:** CR-V2-B5-007  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B5-008: implement-brand-and-brand-identity-skills-as-typed-service`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-skills/src/brand.rs`
- `crates/video-skills/src/brand_identity.rs`
- `crates/video-skills/tests/brand.rs`

**Procedure**

1. Parse/adapt the imported skills into deterministic request builders and schema-bound model outputs.
2. Load existing venture brand data only from signed creative data packs.
3. Implement creation/evolution as a new versioned BrandSystem, never overwriting locked assets.
4. Run contrast, scale, reproduction and accessibility checks.

**Required implementation shape**

```text
brand.resolve(existing_brand_id) -> BrandCard
brand_identity.propose(brief) -> Vec<StyleDirection>
brand_identity.accept(direction_id) -> BrandSystemRevision
```

**Commands for this task**

```bash
cargo test -p video-skills --locked brand
python3 tools/v2-evals/run.py --suite brand
```

**Acceptance — inspect and run only the listed focused checks**

- Existing locked brand rules are preserved.
- Exploration and approved identity are distinct.
- Invalid contrast/reproduction fixtures fail or require review.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-008: implement-brand-and-brand-identity-skills-as-typed-service`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-009 [P-A] — Implement Designer as an internal typed asset planner and reviewer

**Depends on:** CR-V2-B5-008  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B5-009: implement-designer-as-an-internal-typed-asset-planner-and-`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-skills/src/designer.rs`
- `crates/video-skills/src/designer/**`
- `crates/video-skills/tests/designer.rs`

**Procedure**

1. Execute the imported Designer doctrine against AssetRequest, BrandCard and bounded visual evidence.
2. Produce style directions, asset plans, procedural/native render proposals, source-asset selection and review findings.
3. Route supported deterministic graphics to native effect plans; generated raster/video requests require a qualified local capability or return unsupported/needs review.
4. Never mutate the editorial timeline or write arbitrary files.

**Required implementation shape**

```text
DesignerResult { directions, asset_requests, procedural_plans, reviews, unsupported_capabilities, evidence_refs }
```

**Commands for this task**

```bash
cargo test -p video-skills --locked designer
python3 tools/v2-evals/run.py --suite designer
```

**Acceptance — inspect and run only the listed focused checks**

- Designer respects brand/protected zones and rights.
- Unsupported generation does not silently substitute stock/remote assets.
- Every delivery is validated and hash-bound.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-009: implement-designer-as-an-internal-typed-asset-planner-and-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-010 [P-A] — Implement Writing and packaging copy as internal evidence-bound skills

**Depends on:** CR-V2-B5-009  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B5-010: implement-writing-and-packaging-copy-as-internal-evidence-`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-skills/src/writing.rs`
- `crates/video-skills/src/package_copy.rs`
- `crates/video-skills/tests/writing.rs`

**Procedure**

1. Use output transcript, EditorialPlan, brief and BrandCard as sources.
2. Generate script plans for explainers and titles/descriptions/captions/chapters/hooks for existing edits.
3. Require every factual claim to cite transcript/local source evidence and enforce channel length limits.
4. Remove generic openings, unsupported claims and repeated copy through imported evals.

**Required implementation shape**

```text
pub struct CopyClaim { pub text_range: Range<usize>, pub evidence_refs: Vec<EvidenceRef> }
```

**Commands for this task**

```bash
cargo test -p video-skills --locked writing
python3 tools/v2-evals/run.py --suite writing
```

**Acceptance — inspect and run only the listed focused checks**

- No unsupported quote/stat/testimonial is emitted.
- Character/word limits are computed deterministically.
- Copy cannot alter cut points or invent spoken lines.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-010: implement-writing-and-packaging-copy-as-internal-evidence-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-011 [P-A] — Implement Social platform constraints as versioned local data and skill output

**Depends on:** CR-V2-B5-010  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B5-011: implement-social-platform-constraints-as-versioned-local-d`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-skills/src/social.rs`
- `runtime/creative/platforms/*.json`
- `crates/video-skills/tests/social.rs`

**Procedure**

1. Convert selected YouTube/Instagram/Reels/Shorts rules into dated versioned platform data with provenance.
2. Produce PlatformConstraintSet covering aspect, duration, caption, safe zones, packaging and measurement definitions.
3. Keep publishing/account mutation capabilities absent.
4. When current external rules are unknown, require user-supplied or updated signed platform data rather than guessing.

**Required implementation shape**

```text
PlatformConstraintSet { platform, effective_date, aspect_options, duration, caption_mode, safe_zones, packaging, provenance }
```

**Commands for this task**

```bash
cargo test -p video-skills --locked social
python3 tools/v2-evals/run.py --suite social
```

**Acceptance — inspect and run only the listed focused checks**

- All rules have effective date and source/provenance.
- No network lookup occurs during an edit.
- Unknown/expired rules are explicit degraded state.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-011: implement-social-platform-constraints-as-versioned-local-d`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-012 [P-B] — Implement beat/shot creative planning from editorial evidence

**Depends on:** CR-V2-B5-006  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B5-012: implement-beat-shot-creative-planning-from-editorial-evide`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-creative/Cargo.toml`
- `crates/video-creative/src/lib.rs`
- `crates/video-creative/src/beat_shot.rs`
- `crates/video-creative/tests/beat_shot.rs`

**Procedure**

1. Compile EditorialPlan plus BrandCard into visual beats and one or more shots per beat.
2. Use constrained shot size/camera move vocabulary and anti-monotony rules; element motion remains scene-specific but schema-bound.
3. Attach narration/spoken ranges, visual intent, asset needs, title slots, protected zones and evidence.
4. Do not generate unsupported visuals; mark requests.

**Required implementation shape**

```text
EditorialBeat → CreativeBeat { shots: [wide_or_establishing, optional_detail], visual_intent, asset_requests, motion_intent }
```

**Commands for this task**

```bash
cargo test -p video-creative --locked beat_shot
```

**Acceptance — inspect and run only the listed focused checks**

- Adjacent camera moves obey anti-monotony unless explicitly motivated.
- Fast-paced formats meet shot-duration/change cadence rules.
- Every shot maps to editorial output ranges.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-012: implement-beat-shot-creative-planning-from-editorial-evide`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-013 [P-B] — Implement style bake-offs and acceptance records

**Depends on:** CR-V2-B5-012  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B5-013: implement-style-bake-offs-and-acceptance-records`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-creative/src/bakeoff.rs`
- `crates/video-creative/tests/bakeoff.rs`
- `fixtures/creative/bakeoff/**`

**Procedure**

1. Generate 3–4 materially divergent directions using the same content, dimensions and evidence.
2. Render low-cost native styleframes/previews and store exact inputs/hashes.
3. Present user/critic acceptance as an immutable record; selected direction becomes project BrandSystem override.
4. Prevent expensive/full generation before direction acceptance in reviewed mode.

**Required implementation shape**

```text
Bakeoff { invariant_content_hash, directions: Vec<DirectionPreview>, selected: Option<DirectionId> }
```

**Commands for this task**

```bash
cargo test -p video-creative --locked bakeoff
```

**Acceptance — inspect and run only the listed focused checks**

- Variants differ in declared visual dimensions, not story content.
- Selection record binds exact preview hashes.
- Rejected directions remain available as history.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-013: implement-style-bake-offs-and-acceptance-records`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-014 [P-B] — Implement recorded A-roll, generated B-roll, and anchored C-roll planning

**Depends on:** CR-V2-B5-013  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B5-014: implement-recorded-a-roll-generated-b-roll-and-anchored-c-`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-creative/src/lanes.rs`
- `crates/video-creative/src/aroll.rs`
- `crates/video-creative/src/broll.rs`
- `crates/video-creative/src/croll.rs`
- `crates/video-creative/tests/lanes.rs`

**Procedure**

1. A-roll preserves source performance/audio and may apply finish overlays/reframe/restyle only within qualified capabilities.
2. B-roll converts local brief/script/source evidence into narration, beats, shots and locally producible graphics/assets.
3. C-roll anchors a real person/product/logo/label with identity/OCR/wardrobe/product locks and allowed transformations.
4. Return unsupported when required photoreal generation is unavailable locally; never call a cloud service.

**Required implementation shape**

```text
pub enum CreativeLane { RecordedARoll, ProceduralBRoll, AnchoredCRoll }
```

**Commands for this task**

```bash
cargo test -p video-creative --locked lanes
```

**Acceptance — inspect and run only the listed focused checks**

- Lane selection is deterministic from brief/source types.
- A-roll never replaces source audio.
- C-roll protected identity/label changes fail validation.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-014: implement-recorded-a-roll-generated-b-roll-and-anchored-c-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-015 [P-B] — Implement asset semantic validation, rights, identity, and label locks

**Depends on:** CR-V2-B5-014  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B5-015: implement-asset-semantic-validation-rights-identity-and-la`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-creative/src/assets.rs`
- `crates/video-creative/src/identity_lock.rs`
- `crates/video-creative/src/rights.rs`
- `crates/video-creative/tests/assets.rs`

**Procedure**

1. Validate dimensions, aspect, alpha, duration, file type, provenance, rights, safe zones and requested variants.
2. Use OCR/feature/face evidence to compare protected labels, logos and identities.
3. Reject delivery when protected content drifts beyond policy or evidence is insufficient.
4. Keep generated prompt/config and source refs in provenance.

**Required implementation shape**

```text
AssetAcceptance = MechanicalChecks ∧ RightsResolved ∧ ProtectedRegionChecks ∧ SemanticIntentCheck
```

**Commands for this task**

```bash
cargo test -p video-creative --locked assets
```

**Acceptance — inspect and run only the listed focused checks**

- Known label typo/face drift fixtures fail.
- Rights-unresolved assets cannot be accepted.
- Validation result cites exact regions and comparison evidence.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-015: implement-asset-semantic-validation-rights-identity-and-la`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-016 [P-B] — Implement thumbnail, title-card, brand-kit, and package asset plans

**Depends on:** CR-V2-B5-015  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B5-016: implement-thumbnail-title-card-brand-kit-and-package-asset`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-creative/src/package_assets.rs`
- `crates/video-creative/tests/package_assets.rs`
- `skills/package-designer/**`

**Procedure**

1. Select real expressive frames/evidence moments for thumbnails and derive typed Designer requests.
2. Generate native title cards, lower thirds, end cards, OG cards and platform assets from BrandSystem and copy slots.
3. Respect platform safe zones and exact sizes.
4. Store alternates and selection evidence.

**Required implementation shape**

```text
ThumbnailRequest { source_frame: EvidenceRef, title_slot: CopyRef, output: 1280x720, protected_subject_box, brand_ref }
```

**Commands for this task**

```bash
cargo test -p video-creative --locked package_assets
python3 tools/v2-evals/run.py --suite package-designer
```

**Acceptance — inspect and run only the listed focused checks**

- No fabricated face or unsupported claim appears.
- Every file matches size/aspect and text limits.
- Selection ties to actual final/preview frame evidence.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-016: implement-thumbnail-title-card-brand-kit-and-package-asset`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-017 [P-C] — Implement the CutRight native GPU/vector compositor core

**Depends on:** CR-V2-B5-006  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B5-017: implement-the-cutright-native-gpu-vector-compositor-core`  
**Stop-loss ceiling:** at most 16 files and 3000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-render/Cargo.toml`
- `crates/video-render/src/lib.rs`
- `crates/video-render/src/compositor.rs`
- `crates/video-render/src/surface.rs`
- `crates/video-render/tests/compositor.rs`

**Procedure**

1. Implement deterministic offscreen rendering for RGBA frames with explicit colour/alpha spaces and rational frame times.
2. Use lockfile-pinned permissive Rust dependencies approved by the licence ledger.
3. Support transforms, opacity, masks, rounded/soft edges, images, video frame inputs, vector paths and composition ordering.
4. Expose CPU fallback or typed unsupported state for unqualified GPU targets.

**Required implementation shape**

```text
pub trait FrameCompositor { fn render(&self, graph: &CompiledFrameGraph, time: RationalTime, output: &mut FrameBuffer) -> Result<()>; }
```

**Commands for this task**

```bash
cargo test -p video-render --locked compositor
```

**Acceptance — inspect and run only the listed focused checks**

- Golden pixels are stable within declared backend tolerance.
- Layer ordering and alpha fixtures pass.
- No Node/Chromium/HTML runtime is invoked.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-017: implement-the-cutright-native-gpu-vector-compositor-core`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-018 [P-C] — Implement native typography, captions, and text animation

**Depends on:** CR-V2-B5-017  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B5-018: implement-native-typography-captions-and-text-animation`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-render/src/text/**`
- `crates/video-render/src/captions.rs`
- `crates/video-render/tests/text.rs`
- `runtime/creative/fonts/**`

**Procedure**

1. Bundle audited fonts and platform/native shaping/rasterisation resources through creative pack.
2. Implement line breaking, safe zones, phrase/word karaoke, highlighted words, lower thirds, authority stacks, counters, quotes and end cards.
3. Implement exponential/ease-out entrances and reduced-motion fallbacks; no flat default fade for prescribed effects.
4. Validate glyph coverage and fallback deterministically.

**Required implementation shape**

```text
TextNode { content, font_stack, shaped_runs, layout_box, safe_zone, animation: ExponentialReveal, reduced_motion: StaticVisible }
```

**Commands for this task**

```bash
cargo test -p video-render --locked text
```

**Acceptance — inspect and run only the listed focused checks**

- OCR/glyph and layout fixtures pass across targets.
- Caption collisions and missing glyphs fail before final render.
- Font licences/notices are complete.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-018: implement-native-typography-captions-and-text-animation`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-019 [P-C] — Implement native motion grammar, reframing, and temporal placement

**Depends on:** CR-V2-B5-018  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B5-019: implement-native-motion-grammar-reframing-and-temporal-pla`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-render/src/motion.rs`
- `crates/video-render/src/reframe.rs`
- `crates/video-render/src/placement.rs`
- `crates/video-render/tests/motion.rs`

**Procedure**

1. Implement hook pull-back, punch-in/out waves, parallax, hard/motivated transitions, cutaway placement and bounded keyframes.
2. Use subject/reframe tracks and interval-wide collision cost; smooth crop path under jerk/acceleration limits.
3. Enforce cooldown/density/one-language rules and motion blur only during real motion.
4. Implement reduced-motion alternatives.

**Required implementation shape**

```text
placement_cost = subject + face + gesture + text + captions + platform_ui + saliency + edge + temporal_jitter
```

**Commands for this task**

```bash
cargo test -p video-render --locked motion
```

**Acceptance — inspect and run only the listed focused checks**

- Golden motion samples match timing/scale/easing intent.
- Subject and captions remain protected throughout intervals.
- Unmotivated transition/density violations fail planning or QA.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-019: implement-native-motion-grammar-reframing-and-temporal-pla`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-020 [P-C] — Implement native audio finishing, music/SFX, transient sync, and reverb throw

**Depends on:** CR-V2-B5-019  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B5-020: implement-native-audio-finishing-music-sfx-transient-sync-`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-media/src/audio_graph.rs`
- `crates/video-media/src/audio_effects.rs`
- `crates/video-media/tests/audio_graph.rs`
- `runtime/creative/audio/**`

**Procedure**

1. Compile typed audio nodes for gain, EQ, dynamics, denoise if qualified, ducking, fades, music, SFX, transient alignment and wet-tail reverb throw.
2. Migrate the supplied SoX/FFmpeg reverb behavior to an in-process/bundled deterministic filter chain.
3. Use beat/transient evidence and functional one-event/one-sound policy.
4. Audit every music/SFX asset and preserve original speech unless declared.

**Required implementation shape**

```text
dry_full + delay(reverb(wet_only(last throw_ms))) → mix(weights) → measured peak/loudness normalization
```

**Commands for this task**

```bash
cargo test -p video-media --locked audio_graph
```

**Acceptance — inspect and run only the listed focused checks**

- No raw shell composition or system SoX is used.
- Loudness/peak/sync fixtures pass.
- Reverb throw affects only the declared tail and preserves dry body.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-020: implement-native-audio-finishing-music-sfx-transient-sync-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-021 [P-C] — Implement the native render-graph compiler and remove Remotion/HyperFrames from the active source graph

**Depends on:** CR-V2-B5-020  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B5-021: implement-the-native-render-graph-compiler-and-remove-remo`  
**Stop-loss ceiling:** at most 45 files and 14000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-render/src/graph.rs`
- `crates/video-render/src/compile.rs`
- `crates/video-render/src/execute.rs`
- `crates/video-project/src/effects.rs`
- `imports/provenance/remotion-effects/**`
- `apps/effects/**`
- `scripts/gate.sh`
- `scripts/gates/v2-no-legacy-renderer.py`

**Procedure**

1. Validate the declarative graph, resolve signed assets/packs, compile frame/audio/media operations, estimate resources and execute with cancellation and receipts.
2. Copy only CutRight-authored effect schemas, timing contracts, preview fixtures and golden outputs into imports/provenance/remotion-effects with hashes; do not copy upstream Remotion source.
3. After native parity is demonstrated, delete the active apps/effects Remotion package, its package and lock files, Node renderer, Chromium materialisation path and release wiring; update scripts/gate.sh to run native renderer fixtures instead.
4. Add a deterministic migration table from every legacy effect identifier to a native effect identifier. Make direct legacy renderer selection return retired_renderer with exact remediation.
5. Run the no-legacy-renderer gate across Cargo, pnpm, Tauri, release, scripts and runtime manifests.

**Required implementation shape**

```text
EffectRenderer::Native is the only executable renderer.
legacy effect ID → native effect ID migration
legacy renderer request → retired_renderer error
gate effects lane → cargo native-render golden tests
```

**Commands for this task**

```bash
cargo test -p video-render -p video-project --locked render_graph
python3 scripts/gates/v2-no-legacy-renderer.py --check
bash scripts/gate.sh --help
```

**Acceptance — inspect and run only the listed focused checks**

- Native fixtures reach visual and timing parity before the old executable path is removed.
- The active build and release dependency graph contains no Remotion, HyperFrames, Chromium or Node renderer.
- apps/effects is no longer an executable package; only hashed migration provenance remains.
- Every legacy project migrates deterministically or fails with retired_renderer and exact remediation.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-021: implement-the-native-render-graph-compiler-and-remove-remo`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-022 [S] — Merge Book 5 lanes and compile versioned FinishPlans

**Depends on:** CR-V2-B5-011, CR-V2-B5-016, CR-V2-B5-021  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B5-022: merge-book-5-lanes-and-compile-versioned-finishplans`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-creative/src/finish.rs`
- `crates/video-project/src/finish_v2.rs`
- `Cargo.toml`
- `docs/dispatch/v2/book-5/merge-receipt.md`

**Procedure**

1. Apply lane A, B and C commits in fixed order.
2. Build FinishPlan from locked editorial revision, BrandSystem, platform constraints, creative beats/assets, motion/audio policy and preferences.
3. Compile FinishPlan into action batches and native render graph without changing editorial cuts.
4. Record merge conflicts.

**Required implementation shape**

```text
LockedEditorialRevision + CreativePlan + AcceptedAssets + FinishPolicy → FinishPlan → Actions + RenderGraph
```

**Commands for this task**

```bash
cargo check -p video-creative -p video-render -p video-project --locked
```

**Acceptance — inspect and run only the listed focused checks**

- FinishPlan hash binds all inputs/packs/assets.
- A cut-changing finish action is rejected.
- Merge receipt is complete.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-022: merge-book-5-lanes-and-compile-versioned-finishplans`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-023 [S] — Implement independent creative critic and deterministic visual QA

**Depends on:** CR-V2-B5-022  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B5-023: implement-independent-creative-critic-and-deterministic-vi`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-creative/src/critic.rs`
- `crates/video-benchmarks/src/creative.rs`
- `crates/video-project/src/creative_qa.rs`
- `crates/video-creative/tests/critic.rs`

**Procedure**

1. Run deterministic rights/size/OCR/collision/glyph/density/sync checks first.
2. Render representative samples: hook, every transition, every graphic/effect, random intervals, final frame and high-risk evidence spans.
3. Invoke independent vision critic with brief, brand, diff and samples; require evidence-bound findings.
4. Permit one finish revision cycle, then escalate.

**Required implementation shape**

```text
deterministic findings + rendered sample manifest → VisionCritic → CreativeVerdict → pass | one_revision | needs_review
```

**Commands for this task**

```bash
cargo test -p video-creative -p video-benchmarks -p video-project --locked creative_qa
```

**Acceptance — inspect and run only the listed focused checks**

- Critic has no mutation permission.
- Known brand/collision/identity failures are detected.
- Second disagreement escalates.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-023: implement-independent-creative-critic-and-deterministic-vi`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-024 [S] — Integrate generated and procedural creative assembly with the job plane

**Depends on:** CR-V2-B5-023  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B5-024: integrate-generated-and-procedural-creative-assembly-with-`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-project/src/creative_run.rs`
- `crates/video-jobs/src/creative.rs`
- `crates/video-project/tests/creative_run.rs`

**Procedure**

1. Create jobs for skill planning, bake-off, asset generation/procedural rendering, validation, finish compilation, sample render, critic, revision, final render and QA.
2. Cache by all plan/asset/pack/preference inputs.
3. Resume after failed assets without rerunning editorial stages.
4. Return unsupported/needs review when no qualified local generation capability exists.

**Required implementation shape**

```text
creative DAG: plan → {asset jobs} → validate → finish graph → samples → critic → [revise once] → final → QA
```

**Commands for this task**

```bash
cargo test -p video-project -p video-jobs --locked creative_run
```

**Acceptance — inspect and run only the listed focused checks**

- Independent assets may run in parallel within budgets.
- Failed asset job does not corrupt accepted assets or timeline.
- No cloud fallback occurs.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-024: integrate-generated-and-procedural-creative-assembly-with-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-025 [S] — Create four-lane creative golden fixtures and native migration comparisons

**Depends on:** CR-V2-B5-024  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B5-025: create-four-lane-creative-golden-fixtures-and-native-migra`  
**Stop-loss ceiling:** at most 300 files and 70000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `fixtures/creative/four-lane/**`
- `fixtures/native-renderer/migration/**`
- `benchmarks/corpus/creative-manifest.json`

**Procedure**

1. Create rights-cleared fixtures for recorded, repurpose, procedural explainer and anchored creative lanes.
2. Include native equivalents of all existing Remotion effects and supplied Finish techniques.
3. Store expected semantic plans, protected regions, sample frames/audio metrics and acceptance findings.
4. Keep pixel tolerance backend-specific and semantic requirements backend-independent.

**Required implementation shape**

```text
golden = plan JSON + action diff + render graph + frame samples + audio metrics + deterministic QA + critic verdict
```

**Commands for this task**

```bash
cargo test -p video-render -p video-creative --locked golden
cargo run -p video-bench -- run --corpus benchmarks/corpus/creative-manifest.json --profile benchmarks/profiles/reviewed-v2.json --out benchmarks/runs/book-5-creative
```

**Acceptance — inspect and run only the listed focused checks**

- All four lanes produce reviewable outputs.
- Native renderer meets migration semantics.
- Anchored identity/label and collision floors pass.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-025: create-four-lane-creative-golden-fixtures-and-native-migra`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-026 [S] — Run focused creative skill, native renderer, audio, and critic tests

**Depends on:** CR-V2-B5-025  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B5-026: run-focused-creative-skill-native-renderer-audio-and-criti`  
**Stop-loss ceiling:** at most 1 file and 1200 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-5/focused-tests.md`

**Procedure**

1. Run video-skills, video-creative, video-render, audio graph, creative QA, legacy-renderer guard and four-lane creative benchmark.
2. Record pack locks, fonts/assets, target/backend, critic model/seed and report hashes.
3. Do not run the full repository gate here.
4. Fix required failures and preserve unsupported capability reports.

**Required implementation shape**

```text
required: no external renderer, no cut mutation in finish, zero unresolved protected-region/collision failures
```

**Commands for this task**

```bash
cargo test -p video-skills -p video-creative -p video-render -p video-media -p video-project --locked
python3 scripts/gates/v2-no-legacy-renderer.py --check
```

**Acceptance — inspect and run only the listed focused checks**

- Required native paths pass.
- No legacy shipping runtime is reachable.
- Evidence includes creative benchmark hash.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-026: run-focused-creative-skill-native-renderer-audio-and-criti`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B5-027 [S] — Run the authoritative Book 5 local gate and freeze creative evidence

**Depends on:** CR-V2-B5-026  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B5-027: run-the-authoritative-book-5-local-gate-and-freeze-creativ`  
**Stop-loss ceiling:** at most 2 files and 1500 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-5/final-gate.md`
- `docs/dispatch/v2/book-5/final-manifest.json`

**Procedure**

1. Run skill topology, creative pack licence, native renderer, runtime boundary, creative benchmark and focused tests.
2. Run the authoritative local gate exactly once.
3. Record report/fixture/pack hashes and any unproven optional generation claims.
4. Do not create CI or publish.

**Required implementation shape**

```text
book: 5
shipping_renderer: cutright-native
legacy_renderer_runtime_count: 0
ci: forbidden
```

**Commands for this task**

```bash
python3 tools/v2-evals/validate_skill_topology.py --root skills
python3 scripts/gates/v2-no-legacy-renderer.py --check
python3 scripts/legal/validate-v2-ledger.py --scope book-5
bash scripts/gate.sh --with-qa
```

**Acceptance — inspect and run only the listed focused checks**

- All required checks pass.
- Installed render path has no Node/Chromium/Remotion/HyperFrames dependency.
- Final manifest binds commit and creative evidence.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B5-027: run-the-authoritative-book-5-local-gate-and-freeze-creativ`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.
