# CutRight v2 Dispatch Book 1: Reproducible Corpus, Licence Closure, and Standalone Boundary

**Tasks:** 27  
**Goal:** Freeze every source and licence input, compute the relevant skill/tool closure, vendor the permitted material, and make unresolved or external runtime references impossible to ship.  
**Execution default:** A single agent runs numeric order. The labels show safe parallelism for an authorised dispatcher with isolated checkouts; this document does not itself authorise new branches or worktrees.  
**Testing cadence:** Focused checks run inside tasks. The full authoritative local gate runs only in task `CR-V2-B1-027`.  
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
CR-V2-B1-001 .. 006    sequential contract/interface freeze
CR-V2-B1-007 .. 011    parallel lane A
CR-V2-B1-012 .. 016    parallel lane B
CR-V2-B1-017 .. 021    parallel lane C
CR-V2-B1-022 .. 027    sequential merge, integration, acceptance, gate
```

## CR-V2-B1-001 [S] — Freeze the v2 baseline and corpus date

**Depends on:** Pinned CutRight baseline  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B1-001: freeze-the-v2-baseline-and-corpus-date`  
**Stop-loss ceiling:** at most 1 file and 220 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-1/baseline.md`

**Procedure**

1. Abort unless `git rev-parse HEAD` equals `7f3e5a61c729d4d877715b9a083d13a2e5ebe277`.
2. Record the corpus freeze date `2026-08-06` and the exact CutRight, workspace, HeardRight, AutoShorts, Vox, Palmier, llama.cpp, whisper.cpp, Silero, MediaPipe and FFmpeg revisions from the v2 source ledger.
3. Record hashes for every current Cargo and pnpm lockfile plus the current repository-shape guard result.
4. Do not modify production code.

**Required implementation shape**

```text
corpus_date: 2026-08-06
cutright_commit: 7f3e5a61c729d4d877715b9a083d13a2e5ebe277
workspace_commit: 6ee21f03a787e7b57dc412760a8996ea7a235302
heardright_commit: b60bff947f12ffa9d25e94ad27e8ff30db006a24
```

**Commands for this task**

```bash
git rev-parse HEAD
python3 -c "import hashlib,pathlib; files=['Cargo.lock','apps/studio/pnpm-lock.yaml','apps/effects/pnpm-lock.yaml']; [print(hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest(), p) for p in files]"
git status --short
```

**Acceptance — inspect and run only the listed focused checks**

- The baseline file contains every pinned revision and lockfile hash.
- The repository remains on the original commit except for this evidence commit.
- The working tree is clean after commit.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-001: freeze-the-v2-baseline-and-corpus-date`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-002 [S] — Create the machine-readable source corpus schema and manifest

**Depends on:** CR-V2-B1-001  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B1-002: create-the-machine-readable-source-corpus-schema-and-manif`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `schemas/import/source-corpus.schema.v1.json`
- `imports/v2/source-corpus.json`
- `imports/v2/README.md`

**Procedure**

1. Define strict fields for source ID, kind, canonical URL, revision type, revision, licence status, disposition, allowed paths, excluded paths, destination, and notice requirements.
2. Populate one entry for every source in `CutRight-v2-Source-Corpus-and-Ledger.md`.
3. Use immutable commits, tags resolved to commits, model revisions, or attachment hashes only; mutable branches are invalid.
4. Declare `imports/v2/` provenance-only and forbidden to release runtime code.

**Required implementation shape**

```text
{"source_id":"palmier-pro","revision_type":"commit","revision":"397b82e64093f986cbabd89f1a1c93812ff546c2","disposition":"clean_room_behavior","copy_source":false}
```

**Commands for this task**

```bash
python3 -m json.tool imports/v2/source-corpus.json >/dev/null
python3 scripts/schema-check.py schemas/import/source-corpus.schema.v1.json imports/v2/source-corpus.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every corpus row is represented exactly once.
- No entry uses `main`, `master`, `latest`, or an unversioned download URL as its revision.
- Unknown fields and missing dispositions fail validation.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-002: create-the-machine-readable-source-corpus-schema-and-manif`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-003 [S] — Define the licence and disposition ledger contract

**Depends on:** CR-V2-B1-002  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B1-003: define-the-licence-and-disposition-ledger-contract`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `schemas/import/disposition.schema.v1.json`
- `imports/v2/dispositions.json`
- `docs/legal/V2-IMPORT-POLICY.md`

**Procedure**

1. Implement the eight terminal dispositions defined in the v2 ledger.
2. Require separate licence rows for code, model weights, voices, fonts, music, SFX, textures, LUTs, sample media and datasets.
3. Make `blocked_unresolved` and missing rows release-blocking.
4. Document clean-room separation requirements for AutoShorts and Palmier and notice preservation for Vox and workspace material.

**Required implementation shape**

```text
#[serde(rename_all = "snake_case")]
enum Disposition { ShipSource, ShipRuntimePack, AdaptWithNotice, CleanRoomBehavior, ProvenanceOnly, DevelopmentOnly, ExcludedWithReason, BlockedUnresolved }
```

**Commands for this task**

```bash
python3 -m json.tool imports/v2/dispositions.json >/dev/null
python3 scripts/schema-check.py schemas/import/disposition.schema.v1.json imports/v2/dispositions.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every source-corpus entry has a matching disposition row.
- Assets cannot inherit a repository licence without an explicit row.
- `blocked_unresolved` is accepted by the import schema but rejected by the release validator.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-003: define-the-licence-and-disposition-ledger-contract`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-004 [S] — Implement the transitive source-closure scanner

**Depends on:** CR-V2-B1-003  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B1-004: implement-the-transitive-source-closure-scanner`  
**Stop-loss ceiling:** at most 10 files and 1800 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `tools/import-closure/Cargo.toml`
- `tools/import-closure/src/main.rs`
- `tools/import-closure/src/scan.rs`
- `tools/import-closure/tests/fixtures/`

**Procedure**

1. Create a Rust CLI that scans Markdown links, relative paths, script imports, package manifests, Rust `include_str!`/`include_bytes!`, CSS URLs, asset manifests and model manifests.
2. Canonicalise every target inside the pinned snapshot root and reject path escapes, symlink escapes, submodules, device files and mutable URLs.
3. Emit a stable sorted graph with node hash, source path, references, and disposition lookup result.
4. Exit nonzero for an unclassified reachable node.

**Required implementation shape**

```text
pub struct ClosureNode { pub source_id: String, pub path: PathBuf, pub sha256: String, pub references: Vec<PathBuf>, pub disposition: Disposition }
```

**Commands for this task**

```bash
cargo test --manifest-path tools/import-closure/Cargo.toml --locked
cargo run --manifest-path tools/import-closure/Cargo.toml -- --help
```

**Acceptance — inspect and run only the listed focused checks**

- Fixtures prove each supported reference form is found.
- A dangling reference and a `../` escape both fail with a path-specific error.
- Output ordering and hashes are deterministic.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-004: implement-the-transitive-source-closure-scanner`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-005 [S] — Add hard guards for no CI, no submodules, no path lookup, and no external runtime

**Depends on:** CR-V2-B1-004  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B1-005: add-hard-guards-for-no-ci-no-submodules-no-path-lookup-and`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `scripts/gates/v2-repository-shape.sh`
- `scripts/gates/v2-runtime-boundary.py`
- `config/v2-runtime-boundary-allowlist.txt`
- `AGENTS.md`

**Procedure**

1. Fail when `.github/workflows`, `.gitmodules`, skill symlinks, sibling-repository paths, release environment overrides, or bare executable resolution appear in release code.
2. Scan Rust, TypeScript, JSON, TOML and shell sources while excluding tests, generated files and provenance paths through the explicit allowlist.
3. Add the standalone pack-only runtime rule and no-hosted-CI rule to `AGENTS.md` without weakening existing source-integrity rules.
4. Create self-tests that plant one forbidden item at a time and confirm the guard fails.

**Required implementation shape**

```text
if rg -n 'Command::new\("(ffmpeg|ffprobe|python|node|heardright-engine)' crates apps; then
  echo "release code may resolve only signed CutRight pack paths" >&2; exit 1
fi
```

**Commands for this task**

```bash
chmod +x scripts/gates/v2-repository-shape.sh
bash scripts/gates/v2-repository-shape.sh
python3 scripts/gates/v2-runtime-boundary.py --check
```

**Acceptance — inspect and run only the listed focused checks**

- The current tree passes.
- Temporary `.github/workflows/x.yml`, `.gitmodules`, a skill symlink and `Command::new("ffmpeg")` each fail independently.
- The temporary failure fixtures are removed before commit.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-005: add-hard-guards-for-no-ci-no-submodules-no-path-lookup-and`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-006 [S] — Freeze Book 1 import interfaces and lane ownership

**Depends on:** CR-V2-B1-005  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B1-006: freeze-book-1-import-interfaces-and-lane-ownership`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-1/interface-freeze.md`
- `imports/v2/path-map.json`
- `imports/v2/ownership.json`

**Procedure**

1. Freeze destination roots: `skills/`, `vendor/heardright/`, `imports/provenance/`, `third_party/`, `runtime/source/`, and `docs/legal/notices/`.
2. Freeze the import receipt, third-party notice and clean-room observation schemas before parallel lanes begin.
3. Assign lane A only `skills/`; lane B only `vendor/`, `imports/provenance/` and source snapshots; lane C only import/eval/legal tooling and generated ledgers.
4. State that a lane may not edit root workspace manifests; serial merge tasks own integration files.

**Required implementation shape**

```text
{"lane_a":["skills/**"],"lane_b":["vendor/**","imports/provenance/**","runtime/source/**"],"lane_c":["tools/import-closure/**","tools/v2-evals/**","docs/legal/**"]}
```

**Commands for this task**

```bash
python3 -m json.tool imports/v2/path-map.json >/dev/null
python3 -m json.tool imports/v2/ownership.json >/dev/null
```

**Acceptance — inspect and run only the listed focused checks**

- Every parallel output path has exactly one lane owner.
- No lane owns `Cargo.toml`, `scripts/gate.sh`, `AGENTS.md`, or release manifests.
- The frozen schemas and destination roots match the v2 architecture.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-006: freeze-book-1-import-interfaces-and-lane-ownership`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-007 [P-A] — Vendor the complete Designer closure into CutRight

**Depends on:** CR-V2-B1-006  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B1-007: vendor-the-complete-designer-closure-into-cutright`  
**Stop-loss ceiling:** at most 1200 files and 250000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `skills/designer/**`
- `skills/designer/THIRD_PARTY.yml`
- `imports/v2/receipts/designer.json`

**Procedure**

1. Use the closure scanner against workspace commit `6ee21f03a787e7b57dc412760a8996ea7a235302` and root `tools/skills/designer/`.
2. Copy every reachable Designer engine, agent, Huashu reference, script and asset as real files; do not use a symlink or submodule.
3. Preserve relative topology first; do not rewrite cross-skill references in this task.
4. Write byte hashes, source paths and copied-file count to the import receipt.

**Required implementation shape**

```text
source_id: workspace
source_revision: 6ee21f03a787e7b57dc412760a8996ea7a235302
source_root: tools/skills/designer
destination_root: skills/designer
```

**Commands for this task**

```bash
cargo run --manifest-path tools/import-closure/Cargo.toml -- scan --source workspace --root tools/skills/designer --out imports/v2/graphs/designer.json
python3 tools/import-closure/verify_copy.py imports/v2/graphs/designer.json skills/designer
```

**Acceptance — inspect and run only the listed focused checks**

- The exact `designer` root exists with its original `SKILL.md`.
- Every reachable file is copied and hash-bound.
- The receipt reports zero omitted reachable nodes.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-007: vendor-the-complete-designer-closure-into-cutright`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-008 [P-A] — Rewrite Designer to the CutRight-local skill and action model

**Depends on:** CR-V2-B1-007  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B1-008: rewrite-designer-to-the-cutright-local-skill-and-action-mo`  
**Stop-loss ceiling:** at most 1200 files and 250000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `skills/designer/SKILL.md`
- `skills/designer/CUTRIGHT-ADAPTATION.md`
- `skills/designer/engine/**/*.md`
- `skills/designer/engine/scripts/**`

**Procedure**

1. Replace `/brand`, `/audit-visual`, workspace tool paths, external agents, and sibling skill calls with `cutright://skill/<id>` references or typed CutRight action names.
2. Replace direct output mutation with `AssetRequest`, `AssetDelivery`, `RenderSampleRequest`, and `VisualReviewResult` contracts.
3. Retain Designer terminology, critique rules, style systems and assets unless a ledger row excludes them.
4. Record every changed source file and semantic change in `CUTRIGHT-ADAPTATION.md`.

**Required implementation shape**

```text
from: /brand <code>
to: cutright://skill/brand {"brand_code":"<code>"}
mutation: prohibited; emit AssetDelivery only
```

**Commands for this task**

```bash
python3 tools/import-closure/rewrite_refs.py --root skills/designer --map imports/v2/path-map.json --check
python3 tools/import-closure/assert_no_external_refs.py skills/designer
```

**Acceptance — inspect and run only the listed focused checks**

- No Designer file references an external skill location or sibling repository.
- No script executes a cloud API or system executable.
- The adaptation log maps every rewritten reference to a CutRight capability.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-008: rewrite-designer-to-the-cutright-local-skill-and-action-mo`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-009 [P-A] — Vendor and adapt Brand and Brand Identity

**Depends on:** CR-V2-B1-008  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B1-009: vendor-and-adapt-brand-and-brand-identity`  
**Stop-loss ceiling:** at most 200 files and 60000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `skills/brand/**`
- `skills/brand-identity/**`
- `imports/v2/receipts/brand.json`
- `imports/v2/receipts/brand-identity.json`

**Procedure**

1. Copy the complete reachable trees from workspace commit `6ee21f03a787e7b57dc412760a8996ea7a235302`.
2. Rewrite outputs to typed `BrandCard`, `BrandSystem`, `BrandTokenSet`, and `BrandRestrictionSet` artefacts.
3. Keep locked identity, accessibility, reproduction, signature-mechanism and brand-registry rules.
4. Move venture-specific brand data into optional signed creative data packs; keep schemas and generic logic in the base skill.

**Required implementation shape**

```text
pub struct BrandCard { pub brand_id: String, pub voice: VoiceRules, pub visual: VisualTokens, pub restrictions: Vec<Restriction>, pub provenance: Vec<SourceRef> }
```

**Commands for this task**

```bash
python3 tools/import-closure/import.py --source workspace --root tools/skills/brand --dest skills/brand
python3 tools/import-closure/import.py --source workspace --root tools/skills/brand-identity --dest skills/brand-identity
python3 tools/import-closure/assert_no_external_refs.py skills/brand skills/brand-identity
```

**Acceptance — inspect and run only the listed focused checks**

- Both skills are fully local and closure-complete.
- Brand rules cannot mutate source media or timeline cuts.
- Optional brand data is separated from executable skill logic.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-009: vendor-and-adapt-brand-and-brand-identity`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-010 [P-A] — Vendor and adapt the selected Content production closure

**Depends on:** CR-V2-B1-009  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B1-010: vendor-and-adapt-the-selected-content-production-closure`  
**Stop-loss ceiling:** at most 900 files and 180000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `skills/content/**`
- `imports/v2/receipts/content.json`
- `imports/v2/exclusions/content.json`

**Procedure**

1. Include the root skill, video-editor, production-routing, transcription, motion-graphics, Remotion rules/evals as provenance, Seedance/anchored-mode concepts, image enhancement, avatar-video and smoke/eval material reachable from the selected roots.
2. Exclude KDP and carousel branches through explicit `excluded_with_reason` rows; do not delete a reachable file without the exclusion row.
3. Rewrite runtime execution to typed CutRight actions and signed runtime-pack capabilities.
4. Mark hosted generation providers as unsupported optional capabilities rather than required paths.

**Required implementation shape**

```text
{"include_roots":["SKILL.md","references/motion-graphics.md","references/avatar-video.md","specialists/video-editor","specialists/production-routing","specialists/transcription","specialists/remotion"],"exclude_roots":{"specialists/kdp":"not a CutRight v2 video lane"}}
```

**Commands for this task**

```bash
python3 tools/import-closure/import_selected.py imports/v2/selections/content.json
python3 tools/import-closure/verify_exclusions.py imports/v2/graphs/content.json imports/v2/exclusions/content.json
python3 tools/import-closure/assert_no_external_refs.py skills/content
```

**Acceptance — inspect and run only the listed focused checks**

- Every selected root is closure-complete.
- Every omitted reachable branch has a reason.
- No Content skill requires Python, Node, FFmpeg on PATH, or a cloud key.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-010: vendor-and-adapt-the-selected-content-production-closure`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-011 [P-A] — Vendor and adapt Writing, Social, and QA closures

**Depends on:** CR-V2-B1-010  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B1-011: vendor-and-adapt-writing-social-and-qa-closures`  
**Stop-loss ceiling:** at most 700 files and 140000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `skills/writing/**`
- `skills/social/**`
- `skills/qa/**`
- `imports/v2/receipts/writing.json`
- `imports/v2/receipts/social.json`
- `imports/v2/receipts/qa.json`

**Procedure**

1. For Writing include script, editorial, content-repurposer, hook/copy craft and their evals; explicitly exclude email, blogs, profile and changelog lanes.
2. For Social include cross-platform content, YouTube and Instagram/Reels/Shorts constraints and evals; exclude posting, scheduling and account connectors.
3. For QA include deterministic Tauri/local QA, functional assertions, capture, contract tests and evals; remove browser-download assumptions.
4. Rewrite all handoffs as local typed artefacts and all execution as capability-registry actions.

**Required implementation shape**

```text
handoff outputs: ScriptPlan | PlatformConstraintSet | PackageCopy | FunctionalQaPlan | VisualQaPlan
forbidden: direct timeline JSON write, network connector, account mutation
```

**Commands for this task**

```bash
python3 tools/import-closure/import_selected.py imports/v2/selections/writing.json
python3 tools/import-closure/import_selected.py imports/v2/selections/social.json
python3 tools/import-closure/import_selected.py imports/v2/selections/qa.json
python3 tools/import-closure/assert_no_external_refs.py skills/writing skills/social skills/qa
```

**Acceptance — inspect and run only the listed focused checks**

- The three selected closures contain no mutable web-format rules in executable code; current rules are versioned data.
- No skill can post, schedule, spend, or mutate an account.
- QA runs only bundled/local tools.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-011: vendor-and-adapt-writing-social-and-qa-closures`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-012 [P-B] — Vendor the pinned HeardRight source needed by CutRight

**Depends on:** CR-V2-B1-006  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B1-012: vendor-the-pinned-heardright-source-needed-by-cutright`  
**Stop-loss ceiling:** at most 2500 files and 350000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `vendor/heardright/engine/**`
- `vendor/heardright/core/**`
- `vendor/heardright/platform/**`
- `vendor/heardright/THIRD_PARTY.yml`
- `imports/v2/receipts/heardright-source.json`

**Procedure**

1. Copy `heardright-engine`, `heardright_core`, and `heardright_platform` from HeardRight commit `b60bff947f12ffa9d25e94ad27e8ff30db006a24`.
2. Exclude the standalone HeardRight app/UI, user data, caches, generated artifacts and unrelated wake-word training material through explicit rows.
3. Preserve Cargo manifests, build scripts, legal files and source-relative resources required by the selected crates.
4. Write a copy receipt with every byte hash and excluded root.

**Required implementation shape**

```text
source_revision: b60bff947f12ffa9d25e94ad27e8ff30db006a24
include: [heardright-engine, heardright_core, heardright_platform, legal]
exclude: [src, src-tauri, public, artifacts, .cache]
```

**Commands for this task**

```bash
python3 tools/import-closure/import_selected.py imports/v2/selections/heardright-source.json
python3 tools/import-closure/verify_copy.py imports/v2/graphs/heardright-source.json vendor/heardright
```

**Acceptance — inspect and run only the listed focused checks**

- All source required to build the CutRight speech component is local.
- No path points back to the HeardRight repository.
- Excluded application/training material is documented rather than silently omitted.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-012: vendor-the-pinned-heardright-source-needed-by-cutright`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-013 [P-B] — Resolve HeardRight model, dictionary, and runtime-asset provenance

**Depends on:** CR-V2-B1-012  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B1-013: resolve-heardright-model-dictionary-and-runtime-asset-prov`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `imports/v2/heardright-assets.json`
- `docs/legal/HEARDRIGHT-ASSET-LEDGER.md`
- `runtime/source/speech/.gitkeep`

**Procedure**

1. Enumerate every model, tokenizer, vocabulary, dictionary, phonemizer, dynamic library and generated CoreML/ONNX/DirectML asset referenced by the selected HeardRight crates.
2. For each asset record source, exact byte hash, licence, redistribution, modification status, destination pack, and whether it is generated from a source model.
3. Set unresolved rows to `blocked_unresolved`; do not invent a licence from filename or repository ownership.
4. Do not copy model bytes in this task.

**Required implementation shape**

```text
{"asset_id":"parakeet-tdt-primary","sha256":"computed-from-source-byte","license_status":"blocked_unresolved","redistribution":null,"pack":"speech"}
```

**Commands for this task**

```bash
python3 tools/import-closure/scan_assets.py vendor/heardright --out imports/v2/heardright-assets.json
python3 tools/import-closure/validate_asset_ledger.py imports/v2/heardright-assets.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every referenced non-source file has one row.
- The release validator fails while any row is unresolved.
- Parakeet and Silero entries identify exact source and destination pack.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-013: resolve-heardright-model-dictionary-and-runtime-asset-prov`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-014 [P-B] — Materialize the supplied Cutaway and Finish artefacts as provenance

**Depends on:** CR-V2-B1-013  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B1-014: materialize-the-supplied-cutaway-and-finish-artefacts-as-p`  
**Stop-loss ceiling:** at most 80 files and 30000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `imports/provenance/cutaway-finish/**`
- `imports/v2/receipts/cutaway-finish.json`
- `docs/migrations/CUTAWAY-FINISH-GOLDEN-BEHAVIOR.md`

**Procedure**

1. Materialize every supplied skill/script/example file into the provenance root without editing its contents.
2. Hash each file and map its behavior to a named future Rust/native stage: transcript understanding, forced alignment, speech-region intersection, word-safe cuts, motion scoring, storyboards, pull-backs, punch waves, text, SFX and reverb throw.
3. Mark Python, Bash, Resolve, SoX, auto-editor and WhisperX execution as provenance-only dependencies.
4. Define golden inputs/outputs to be recreated by native tests.

**Required implementation shape**

```text
build_wx.py -> video-project::boundary_consensus::compile_word_safe_segments
motion_score.py -> video-evidence::motion::score_span
reverb_throw.sh -> video-media::audio::ReverbThrowNode
```

**Commands for this task**

```bash
python3 scripts/import-conversation-files.py --manifest imports/v2/conversation-files.json --dest imports/provenance/cutaway-finish
python3 tools/import-closure/hash_tree.py imports/provenance/cutaway-finish > imports/v2/receipts/cutaway-finish.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every supplied file is present and hash-bound.
- No provenance script is called by release code.
- Every live behavior has a named migration target and golden fixture.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-014: materialize-the-supplied-cutaway-and-finish-artefacts-as-p`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-015 [P-B] — Adapt the permitted Vox Director material

**Depends on:** CR-V2-B1-014  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B1-015: adapt-the-permitted-vox-director-material`  
**Stop-loss ceiling:** at most 180 files and 50000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `imports/provenance/vox-director/**`
- `skills/video-director/**`
- `docs/legal/notices/vox-director.txt`
- `imports/v2/receipts/vox.json`

**Procedure**

1. Copy only the selected MIT source/reference files from Vox commit `8b034354dc443edcde7fdb2622e0491df5142fd3` with notice.
2. Create a CutRight-local Video Director skill containing narrative arcs, beat/shot schema, style bake-offs, A/B/C-roll rules, constrained camera vocabulary, element motion, anti-monotony and bounded job semantics.
3. Remove Atlas Cloud model names, API clients, upload/download code and hosted-provider assumptions.
4. Use CutRight capability names and typed plans; never copy provider credentials or output directories.

**Required implementation shape**

```text
pub struct ShotPlan { pub shot_id: ShotId, pub beat_id: BeatId, pub size: ShotSize, pub camera_move: CameraMove, pub element_motion: Vec<ElementMotion>, pub evidence_refs: Vec<EvidenceRef> }
```

**Commands for this task**

```bash
python3 tools/import-closure/import_selected.py imports/v2/selections/vox.json
python3 tools/import-closure/assert_no_external_refs.py skills/video-director
python3 tools/import-closure/verify_notices.py imports/provenance/vox-director
```

**Acceptance — inspect and run only the listed focused checks**

- MIT notice is shipped.
- The skill can plan without a cloud provider.
- Beat/shot vocabularies are schema-bound and all unsupported original behaviors are listed.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-015: adapt-the-permitted-vox-director-material`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-016 [P-B] — Write clean-room AutoShorts behavior specifications

**Depends on:** CR-V2-B1-015  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B1-016: write-clean-room-autoshorts-behavior-specifications`  
**Stop-loss ceiling:** at most 20 files and 6000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `imports/provenance/behavior/autoshorts/*.md`
- `imports/v2/clean-room/autoshorts.json`

**Procedure**

1. Observe only public behavior at AutoShorts commit `f17b04cdd97ef65c32b81b31b36bb6eb5d013d5b`.
2. Document project library, onboarding, model/runtime readiness, one-click pipeline, candidate cards, progress, selection, recovery and export behavior without source-shaped class/function names.
3. Record rejected behavior: browser-local API keys, center crop, direct model timestamps, database as canonical truth, cloud-first defaults and monolithic UI.
4. Have an implementation reviewer attest that no AutoShorts source is copied.

**Required implementation shape**

```text
{"behavior_id":"project-card-progress","observable":"A project card shows the current pipeline stage and recovers after relaunch","implementation_constraints":["project package is canonical","index is disposable"]}
```

**Commands for this task**

```bash
python3 tools/import-closure/validate_clean_room.py imports/v2/clean-room/autoshorts.json
```

**Acceptance — inspect and run only the listed focused checks**

- The observation spec is implementation-neutral.
- Every adopted behavior has an acceptance test statement.
- The attestation records observer and implementer separation.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-016: write-clean-room-autoshorts-behavior-specifications`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-017 [P-C] — Write clean-room Palmier behavior specifications

**Depends on:** CR-V2-B1-006  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B1-017: write-clean-room-palmier-behavior-specifications`  
**Stop-loss ceiling:** at most 24 files and 8000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `imports/provenance/behavior/palmier/*.md`
- `imports/v2/clean-room/palmier.json`

**Procedure**

1. Observe only public behavior and documentation at Palmier commit `397b82e64093f986cbabd89f1a1c93812ff546c2`.
2. Specify typed project/timeline/media/clip/text/caption/effect/export tools, stable IDs, source seconds versus timeline frames, active timeline, variants, composited inspection, undo and async jobs.
3. Do not copy Swift declarations, descriptions, schemas, comments or implementation structure.
4. Record a clean-room attestation and direct future implementation to CutRight terminology and action contracts.

**Required implementation shape**

```text
behavior: composited_timeline_inspection
input: timeline_id + frame window
output: rendered samples + visible stable object IDs
implementation: CutRight action/read model, not Palmier schema
```

**Commands for this task**

```bash
python3 tools/import-closure/validate_clean_room.py imports/v2/clean-room/palmier.json
```

**Acceptance — inspect and run only the listed focused checks**

- No copied Swift or near-verbatim tool description appears.
- Every behavior maps to a future CutRight action or read model.
- GPL source remains outside shipping and development source roots.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-017: write-clean-room-palmier-behavior-specifications`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-018 [P-C] — Adapt the workspace bounded-run compiler and monitor concepts

**Depends on:** CR-V2-B1-017  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B1-018: adapt-the-workspace-bounded-run-compiler-and-monitor-conce`  
**Stop-loss ceiling:** at most 60 files and 12000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `tools/v2-skill-compiler/**`
- `tools/v2-skill-monitor/**`
- `imports/v2/receipts/bounded-run.json`

**Procedure**

1. Copy or reimplement the user-owned skill compilation, schema, monitor and migration concepts from the pinned workspace tool.
2. Make the compiler consume only `skills/`, `schemas/skills/` and `capabilities/registry.json` inside CutRight.
3. Produce a deterministic embedded resource pack plus topology report; reject external paths and mutable resources.
4. Keep monitoring local and project-scoped; no workspace-global agent state.

**Required implementation shape**

```text
pub struct CompiledSkill { pub id: SkillId, pub version: SemVer, pub content_hash: Hash, pub dependencies: Vec<SkillId>, pub permissions: PermissionSet, pub resources: Vec<ResourceRef> }
```

**Commands for this task**

```bash
cargo test --manifest-path tools/v2-skill-compiler/Cargo.toml --locked
cargo test --manifest-path tools/v2-skill-monitor/Cargo.toml --locked
```

**Acceptance — inspect and run only the listed focused checks**

- Two identical builds produce byte-identical skill packs.
- External path and dangling dependency fixtures fail.
- The monitor reports typed degraded/failed states without modifying skills.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-018: adapt-the-workspace-bounded-run-compiler-and-monitor-conce`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-019 [P-C] — Adapt skill topology, catalogue integrity, and evaluation fixtures

**Depends on:** CR-V2-B1-018  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B1-019: adapt-skill-topology-catalogue-integrity-and-evaluation-fi`  
**Stop-loss ceiling:** at most 100 files and 25000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `tools/v2-evals/**`
- `schemas/evals/**`
- `fixtures/evals/**`
- `imports/v2/receipts/workspace-evals.json`

**Procedure**

1. Adapt the workspace catalogue-integrity and skill-topology checks to CutRight roots and schema names.
2. Import relevant Designer, Content, Writing, Social, Brand and QA eval cases with source notices and rewrite them to CutRight inputs/outputs.
3. Add negative fixtures for unclassified dependencies, external paths, missing permissions, mutable model references and absent notices.
4. Do not import unrelated research, SEO, email or coding-agent eval cases.

**Required implementation shape**

```text
{"case_id":"designer-no-direct-mutation","input":{"request":"change the cut"},"expected":{"status":"refused","reason_code":"skill_boundary"}}
```

**Commands for this task**

```bash
python3 tools/v2-evals/catalog_integrity.py --root skills
python3 tools/v2-evals/validate_skill_topology.py --root skills
python3 tools/v2-evals/run.py --suite import
```

**Acceptance — inspect and run only the listed focused checks**

- Catalogue and topology reports are deterministic.
- Every included skill has at least one positive and one refusal/degradation case.
- An omitted workspace eval has an exclusion row.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-019: adapt-skill-topology-catalogue-integrity-and-evaluation-fi`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-020 [P-C] — Adapt the evidence gauntlet as an optional local hardening lane

**Depends on:** CR-V2-B1-019  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B1-020: adapt-the-evidence-gauntlet-as-an-optional-local-hardening`  
**Stop-loss ceiling:** at most 60 files and 15000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `tools/v2-gauntlet/**`
- `docs/testing/V2-GAUNTLET.md`
- `imports/v2/receipts/gauntlet.json`

**Procedure**

1. Port changed-line mutation testing, changed-line coverage and deterministic test-order randomisation to the CutRight local toolchain.
2. Support Rust and TypeScript changed files; report unsupported mutation shapes as skipped with reasons.
3. Emit a local JSON receipt and never integrate with GitHub Actions or a hosted service.
4. Keep the gauntlet optional for normal book gates and required only in the final release audit when its pinned toolchain is available.

**Required implementation shape**

```text
pub enum LayerStatus { Passed, Failed, Skipped { reason: String }, Unproven { reason: String } }
```

**Commands for this task**

```bash
cargo test --manifest-path tools/v2-gauntlet/Cargo.toml --locked
cargo run --manifest-path tools/v2-gauntlet/Cargo.toml -- --self-test
```

**Acceptance — inspect and run only the listed focused checks**

- A known weak fixture produces a surviving mutant and fails.
- Test-order seed is recorded and reproducible.
- An unavailable coverage backend is `unproven`, not pass.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-020: adapt-the-evidence-gauntlet-as-an-optional-local-hardening`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-021 [P-C] — Classify Remotion and HyperFrames and freeze the native migration contract

**Depends on:** CR-V2-B1-020  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B1-021: classify-remotion-and-hyperframes-and-freeze-the-native-mi`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/architecture/NATIVE-RENDERER-MIGRATION.md`
- `imports/v2/dispositions/renderers.json`
- `fixtures/native-renderer/manifest.json`

**Procedure**

1. Set Remotion and HyperFrames shipping disposition to `provenance_only`/`clean_room_behavior`; prohibit their binaries/packages in runtime packs.
2. Inventory every current CutRight effect, timing rule, safe zone, reduced-motion behavior, input schema and preview fixture.
3. Define native golden comparisons for lower third, stat counter, quote card, CTA card, captions, hook pull-back, punch wave, text reveals and audio-synchronised effects.
4. State deletion criteria: native implementation passes fixtures, current projects migrate, and release contains no Node/Chromium/Remotion/HyperFrames runtime.

**Required implementation shape**

```text
{"legacy":"remotion:StatCounter","native_effect_id":"stat.counter.v2","golden_fixture":"fixtures/native-renderer/stat-counter","shipping_runtime":"cutright-native"}
```

**Commands for this task**

```bash
python3 -m json.tool imports/v2/dispositions/renderers.json >/dev/null
python3 tools/v2-evals/check_renderer_migration_manifest.py fixtures/native-renderer/manifest.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every existing renderer/effect has a migration target.
- Release guards know forbidden runtime package names.
- No visual requirement is lost merely because its old technology is rejected.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-021: classify-remotion-and-hyperframes-and-freeze-the-native-mi`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-022 [S] — Create third-party notices and corresponding-source archive scaffolds

**Depends on:** CR-V2-B1-011, CR-V2-B1-016, CR-V2-B1-021  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B1-022: create-third-party-notices-and-corresponding-source-archiv`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `third_party/README.md`
- `third_party/notices/**`
- `runtime/source/README.md`
- `scripts/legal/build-corresponding-source.py`

**Procedure**

1. Create notice templates for source, binary, model, asset and clean-room entries.
2. Create deterministic corresponding-source archive generation for FFmpeg and other reciprocal obligations.
3. Require source revision, build configuration, patches, output hash and notice path in every binary-runtime row.
4. Do not fetch anything from the network; inputs are pinned local source snapshots.

**Required implementation shape**

```text
runtime-source/<component>/<version>/<target>.tar.zst
manifest: source_revision + patches + configure_args + source_sha256 + binary_sha256
```

**Commands for this task**

```bash
python3 scripts/legal/build-corresponding-source.py --self-test
python3 tools/import-closure/verify_notices.py third_party/notices
```

**Acceptance — inspect and run only the listed focused checks**

- Archive filenames and contents are deterministic.
- A binary without a source/notice row fails.
- The scaffold contains no empty legal claim presented as resolved.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-022: create-third-party-notices-and-corresponding-source-archiv`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-023 [S] — Merge the three Book 1 lanes in deterministic order

**Depends on:** CR-V2-B1-022  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B1-023: merge-the-three-book-1-lanes-in-deterministic-order`  
**Stop-loss ceiling:** at most 1 file and 400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-1/merge-receipt.md`

**Procedure**

1. Apply lane A commits in task order 007–011, lane B commits 012–016, then lane C commits 017–021.
2. Resolve conflicts only against `interface-freeze.md`; do not rename frozen destination roots.
3. Run import topology and repository-shape checks after each lane group, not the full book gate.
4. Record every applied commit and conflict resolution.

**Required implementation shape**

```text
merge_order:
  - lane_a: CR-V2-B1-007..011
  - lane_b: CR-V2-B1-012..016
  - lane_c: CR-V2-B1-017..021
```

**Commands for this task**

```bash
python3 tools/v2-evals/validate_skill_topology.py --root skills
bash scripts/gates/v2-repository-shape.sh
git status --short
```

**Acceptance — inspect and run only the listed focused checks**

- All lane commits are present once in fixed order.
- No parallel lane owns or modifies another lane root.
- The merge receipt names every conflict or states none.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-023: merge-the-three-book-1-lanes-in-deterministic-order`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-024 [S] — Compile the embedded skill catalogue and complete closure report

**Depends on:** CR-V2-B1-023  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B1-024: compile-the-embedded-skill-catalogue-and-complete-closure-`  
**Stop-loss ceiling:** at most 4 files and 12000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `skills/catalog.json`
- `skills/catalog.lock.json`
- `docs/skills/V2-CLOSURE-REPORT.md`
- `apps/studio/src/generated/skillCatalog.ts`

**Procedure**

1. Run the v2 compiler over all imported skills.
2. Generate stable IDs, versions, hashes, dependencies, permissions, eval suites and resource lists.
3. Generate the TypeScript read model from the same lock.
4. Write a report listing every included, adapted, excluded and blocked source node.

**Required implementation shape**

```text
{"skill_id":"designer","content_hash":"sha256:...","dependencies":["brand","brand-identity","visual-qa"],"permissions":["evidence:read","asset-plan:write"]}
```

**Commands for this task**

```bash
cargo run --manifest-path tools/v2-skill-compiler/Cargo.toml -- compile --root skills --out skills/catalog.lock.json
python3 tools/v2-evals/catalog_integrity.py --root skills
git diff --exit-code -- apps/studio/src/generated/skillCatalog.ts
```

**Acceptance — inspect and run only the listed focused checks**

- The lock is deterministic and has no external path.
- Every skill dependency resolves.
- The report has zero unclassified nodes.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-024: compile-the-embedded-skill-catalogue-and-complete-closure-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-025 [S] — Enforce zero unresolved licence and provenance rows for Book 1 outputs

**Depends on:** CR-V2-B1-024  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B1-025: enforce-zero-unresolved-licence-and-provenance-rows-for-bo`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `scripts/legal/validate-v2-ledger.py`
- `docs/dispatch/v2/book-1/licence-report.md`

**Procedure**

1. Validate all code and assets copied in Book 1, while allowing unresolved future model bytes that are not yet copied or signed.
2. Fail for a copied byte with no ledger row, a missing notice, a mismatched hash, an inherited asset licence, or GPL source under shipping roots.
3. Report future pack rows separately as `pending_not_materialized`, not resolved.
4. Record clean-room attestations for AutoShorts and Palmier.

**Required implementation shape**

```text
materialized + blocked_unresolved => FAIL
not_materialized + pending_pack_resolution => REPORT_ONLY
ship_root + GPL-3.0 => FAIL
```

**Commands for this task**

```bash
python3 scripts/legal/validate-v2-ledger.py --scope book-1 --report docs/dispatch/v2/book-1/licence-report.md
```

**Acceptance — inspect and run only the listed focused checks**

- All materialized Book 1 bytes are resolved.
- Pending future pack rows are not misreported as pass.
- No GPL source is inside a shipping source root.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-025: enforce-zero-unresolved-licence-and-provenance-rows-for-bo`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-026 [S] — Prove the source tree has no runtime dependency on another checkout

**Depends on:** CR-V2-B1-025  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B1-026: prove-the-source-tree-has-no-runtime-dependency-on-another`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `scripts/gates/v2-standalone-source-audit.py`
- `docs/dispatch/v2/book-1/standalone-source-audit.json`

**Procedure**

1. Scan imports, paths, build scripts, Tauri configuration, package manifests, Rust process calls, environment variables and documentation-generated defaults.
2. Reject paths containing the workspace, HeardRight checkout, AutoShorts, Vox, Palmier, user skill directories, home-relative tool directories, or Git submodules.
3. Allow source URLs and commit IDs only in provenance and legal files.
4. Emit exact findings with file, line and rule ID.

**Required implementation shape**

```text
forbidden_release_patterns = ["../heardright", "/tools/skills/", "CUTRIGHT_HEARDRIGHT_ENGINE", "PATH lookup", ".gitmodules"]
```

**Commands for this task**

```bash
python3 scripts/gates/v2-standalone-source-audit.py --root . --json docs/dispatch/v2/book-1/standalone-source-audit.json
```

**Acceptance — inspect and run only the listed focused checks**

- The report has zero release-code findings.
- A planted sibling path fixture fails.
- Provenance citations remain allowed.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-026: prove-the-source-tree-has-no-runtime-dependency-on-another`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B1-027 [S] — Run focused Book 1 validation and the authoritative local gate

**Depends on:** CR-V2-B1-026  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B1-027: run-focused-book-1-validation-and-the-authoritative-local-`  
**Stop-loss ceiling:** at most 3 files and 1800 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-1/focused-tests.md`
- `docs/dispatch/v2/book-1/final-gate.md`
- `docs/dispatch/v2/book-1/final-manifest.json`

**Procedure**

1. Run the import-closure, skill-compiler, evaluation-topology, gauntlet self-tests, legal validator and source-boundary guards first.
2. Fix any focused failure within Book 1 ownership; do not waive it and do not run the broad gate until every required focused check passes.
3. Run the existing authoritative local gate exactly once after the focused checks and v2 guards pass.
4. Record exact commit, commands, versions, exit codes, test totals, output hashes and every skipped or unproven check; do not add CI or upload artifacts.

**Required implementation shape**

```text
book: 1
focused_checks_before_gate: required
required_gate: "bash scripts/gate.sh --with-qa"
ci: forbidden
publish: false
```

**Commands for this task**

```bash
cargo test --manifest-path tools/import-closure/Cargo.toml --locked
cargo test --manifest-path tools/v2-skill-compiler/Cargo.toml --locked
python3 tools/v2-evals/run.py --suite import
python3 tools/v2-gauntlet/run.py --self-test
bash scripts/gates/v2-repository-shape.sh
python3 scripts/gates/v2-standalone-source-audit.py --root .
python3 scripts/legal/validate-v2-ledger.py --scope book-1
bash scripts/gate.sh --with-qa
```

**Acceptance — inspect and run only the listed focused checks**

- Every listed focused suite and required guard passes.
- No skipped or unrun check is recorded as pass.
- The final manifest binds the exact commit and evidence files.
- The tree contains no hosted-CI files, submodules, symlinked skills or external runtime references.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B1-027: run-focused-book-1-validation-and-the-authoritative-local-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.
