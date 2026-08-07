# CutRight — Final Consolidated Product and Engineering Implementation Plan, Revision 2

**Date:** 2026-07-30
**Reviewed CutRight baseline:** `Orthic-Labs/CutRight` `main` at `a8d4584f2a01f51d07d7018707eb0aca83d97adc`
**Reviewed HeardRight baseline:** current `bogusyogi/heardright` default branch, including the engine protocol, timed file-transcription path, and Silero runtime
**Status:** implementation source of truth for the next hardening and product-completion campaign
**Supersedes:** all earlier consolidated CutRight plans from 2026-07-30

---

## 0. Source and verification note

The public CutRight branch now matches the `a8d4584` tree audited in the two supplied 2026-07-30 reports. The earlier repository-reconciliation blocker is therefore removed.

This revision uses:

- the live gate results reported by the two supplied audits for `a8d4584`;
- a fresh code-level review of the pushed commit and its current files on GitHub;
- the current HeardRight engine protocol and VAD implementation;
- independent reasoning about cross-file and cross-stage behavior that the mechanical audits did not cover.

This document does **not** claim that a new local test run was executed after the push. The first implementation PR must rerun and record the complete gate described below.

### What changed from the superseded plan

- removed the obsolete public-versus-local repository reconciliation campaign;
- treats the full Studio review workflow as present in the baseline;
- adds the broken frontend/Rust decision contract as the first hotfix;
- adds the natural/tight shared-artifact cross-contamination problem as a P0 correctness issue;
- makes review selection an execution gate for final render;
- corrects the benchmark from symmetric provider election to HeardRight-primary validation;
- records the already-visible `source_word_id` schema drift;
- adds current pushed-tree gaps such as nonportable QA scripts, stale embedded workers, first-source-only packed transcript, and zero-exit error paths;
- reorders the implementation sequence around current code rather than the pre-push repository state.

---

## 1. Executive conclusion

CutRight's core architecture is sound. It already has the right foundation:

- immutable source registration with BLAKE3;
- a Rust control plane and JSON-only CLI;
- local HeardRight Parakeet TDT transcription with native timed words;
- WhisperX as a separate alignment verifier;
- Silero VAD;
- deterministic rough-cut and final-render paths;
- vertical-render approval gates;
- a Tauri review surface;
- evidence and QA artifacts;
- offline-by-default operation.

The fresh push materially advances Phase 3, but it also reveals several correctness gaps that are more important than the governance-only findings in the audits.

The immediate work is:

1. **repair the Studio decision boundary**, because the frontend record currently does not satisfy the Rust command contract;
2. **make every rough-cut artifact variant-specific**, because the current shared `timeline.json`, `captions.srt`, reframe plan, and final-render path can mix natural and tight state;
3. **wire review decisions into execution**, because Studio approval currently does not select or gate the final render;
4. **correct the transcription benchmark policy**, so HeardRight remains primary and WhisperX verifies it rather than competing in a symmetric tournament;
5. **put every code surface under one authoritative gate**, including the standalone Studio Rust workspace and frontend;
6. **finish the internal HeardRight boundary**, removing CutRight's duplicate ownership of local audio inference;
7. **harden contracts, provenance, diagnostics, external processes, and per-deliverable QA**;
8. **then decompose the large files and complete the real editing product phases**.

No external replacement audio stack is part of this plan.

---

## 2. Current as-built state at `a8d4584`

### 2.1 What is genuinely built

| Area | Current implementation |
|---|---|
| Project package | `project.json`, immutable source manifest, canonical analysis/edit/render/QA directories |
| Ingest | canonical path, BLAKE3 registration, FFprobe metadata, source-change rejection |
| Transcription | HeardRight primary path; WhisperX alternate/verifier path; raw response and provider envelope for transcription |
| VAD | standalone CutRight Silero CoreML worker producing timestamped regions |
| Rough cuts | candidate grouping, VAD-expanded bounds, natural/tight renders |
| Transcript remap | output-timeline words, compound `source_word_id`, per-variant SRT and transcript files |
| Evidence | boundary frames, waveforms, filmstrip composites |
| Reframe | one Vision face anchor per segment, explicit approval required for vertical final |
| Final render | YouTube/vertical dimensions, burned caption cards, HDR-to-Rec.709 path |
| QA | source rehash, benchmark presence, container/stream/dimension checks, evidence presence |
| Packaging | YouTube, vertical, captions, OTIO export |
| Studio | Sources, Compare, Finals, QA modes; word-locked swap; keyboard control; decision backend; source verifier command |

### 2.2 What is still placeholder-level despite being wired

| Area | Current limitation |
|---|---|
| Candidate generation | mechanically groups words by a 900 ms gap; it does not perform the planned red-thread editorial selection |
| Tight vs natural | `gap_threshold_ms` is stored but not applied to compact pauses; variants differ mainly by boundary margins |
| Benchmark | requires exactly one provider to be perfect and treats both-perfect as unresolved |
| Review | append contract is broken; approval is not an execution gate |
| Final base | hard-coded to `natural.mp4`; shared generic artifacts can belong to the last-rendered variant |
| Reframe | a single midpoint face box per segment, not temporal tracking |
| Captions | basic SRT grouping and per-cue image rendering; no reading-speed or safe-zone profiles |
| Audio finish | no full dialogue chain, loudness normalization gate, true-peak gate, or music ducking |
| Color | basic HDR handling; no complete shot matching or profile-driven color pipeline |
| Shorts | duration/take-rank heuristic, not semantic standalone-story extraction |
| Effects | finish slots currently route only to final delivery |
| Preference learning | append-only design intended, but records are not yet safely generated, hash-bound, or consumed |
| Resumability | transcription has a cache envelope; the whole pipeline is not yet content-addressed and resumable |

This distinction matters. The repository is well beyond a prototype, but Phases 3–7 are not complete merely because command names and UI surfaces exist.

---

## 3. Immediate correctness findings from the pushed tree

These are not theoretical cleanup items. They can produce failed writes, stale reviews, or mismatched final artifacts.

### 3.1 Studio sends a different decision shape than Rust requires

The Rust backend requires a full record containing fields such as:

- `schema_version`;
- `project_id`;
- `note`;
- `word_id` and `source_word_id`;
- `playhead_ms`;
- `bench_resolved`;
- `snapshot_generated_at`;
- `app_version`.

The frontend sends only a partial object. Real Tauri invocation should fail during deserialization before `append_decision` runs.

### 3.2 Studio sends absolute artifact paths while Rust requires project-relative subjects

`ProjectSnapshot` exposes absolute MP4 paths. The frontend sends those paths as the decision `subject`. The backend only accepts project-relative paths and additionally requires exact variant rough-cut paths.

### 3.3 Reason vocabularies do not match

The UI chooses reasons by approve/reject state, not by decision target:

- rejected rough cuts offer segment-level reasons that the backend rejects for `variant_verdict`;
- final verdicts use the same rough-cut reasons, while the backend requires final-specific reasons;
- `other` collects text in the UI but the frontend does not place that text in the record's `note` field.

### 3.4 Source mode can accidentally create a rough-cut verdict

The verdict bar is shown in every mode except QA. In Sources mode, `commit()` falls through to `variant_verdict`, using the currently selected rough-cut variant even though the user is reviewing source footage.

### 3.5 The QA mock bypasses the boundary that is broken

Browser QA replaces `append_decision` with an in-memory push. It does not invoke Rust, so it cannot detect deserialization, path, reason, hash, or persistence failures. Its fixture word IDs also do not match the backend's six-digit ID rules.

### 3.6 Decision replay silently discards history that no longer matches current project state

`read_decisions` re-runs current-state validation and skips invalid lines. A once-valid decision can disappear from the UI if a variant file is renamed, removed, or migrated. Malformed and stale are conflated, and the frontend ignores the returned `skipped` count.

### 3.7 Studio implements only part of its own Phase 3 specification

The backend exposes source verification, but the current frontend does not provide the specified Verify action. The word-lock algorithm returns the count of words cut between variants, but the UI does not render the specified persistent cut marker. Segment flags, QA acknowledgement, session-open records, follow-state behavior, and source relinking are also incomplete.

### 3.8 Review decisions do not control rendering

`render final` always reads `render/rough-cuts/natural.mp4`. It does not resolve an approved Studio variant or require a decision bound to the exact rough-cut bytes.

### 3.9 Variant state can cross-contaminate

The code has variant-specific cut plans, rough MP4s, remapped transcripts, and SRTs, but still uses shared mutable artifacts:

- `edit/cut-plan.json`;
- `edit/timeline.json`;
- `edit/captions.srt`;
- `analysis/reframe-plan.json`;
- `finish/finish-plan.json`.

Rendering tight after natural overwrites the shared timeline and cut plan. Final render still consumes natural video. Reframe, captions, QA, finish validation, and OTIO can therefore refer to a different variant from the final input.

### 3.10 The natural/tight pacing contract is not implemented

`gap_threshold_ms` is written into the cut plan but never used to split or compact inter-word gaps. Natural and tight currently do not express the intended 400 ms versus 220 ms pause policy.

### 3.11 The benchmark policy is inverted for the product architecture

The current decision function returns:

- primary when only primary passes;
- verifier when only verifier passes;
- unresolved when both pass;
- unresolved when neither passes.

It also requires zero unmatched words. Two good providers therefore block the project, while normal tokenization disagreement can prevent either from qualifying. HeardRight should remain the transcript authority; WhisperX should verify timestamp safety and supply fallback alignment only when required.

### 3.12 The schema already drifted from the Rust model

Rust `Word` now includes optional `source_word_id`, but `schemas/transcript.schema.json` does not describe it. The current schema permits unspecified properties, so the drift is silent rather than rejected.

### 3.13 The final pipeline is not consistently variant-aware

`render final` uses natural video plus generic captions. `reframe plan`, `finish validate`, `validate edit`, QA, and OTIO use generic timeline/cut-plan state. A compare workflow that renders both variants can leave these aliases pointing at the wrong one.

### 3.14 `doctor` can print an error and still exit successfully

`doctor()` returns JSON inside `Ok(...)`. The CLI therefore exits zero even when the JSON status is `error`. It also treats process spawn as success without checking the command exit status or required capabilities.

### 3.15 Machine-specific QA paths remain in the repository

`qa:functional` and `qa:shot` invoke scripts through an absolute `/Volumes/D/claude/...` path. They are not portable to CI or another contributor's checkout.

### 3.16 Several embedded sidecar binaries can go stale

The Silero worker compares embedded bytes before re-materializing. The Vision anchor and caption-card workers only check whether a version-named temp file exists. Their source can change without a crate version bump, leaving an old binary active.

### 3.17 Several additional state and identity weaknesses should be corrected

- New `project_id` values are derived only from the project folder name, so same-named projects collide.
- Snapshot reads silently convert corrupt optional JSON into “missing” state.
- Final snapshots show configured dimensions rather than clearly separating expected and probed dimensions.
- Full provider provenance exists for ASR but not for VAD.
- FFmpeg can be custom/bundled while FFprobe is always resolved from `PATH`, allowing mismatched tool versions.
- JSON atomic temp names use only the process ID, so concurrent writes to the same target can collide.
- Unimplemented CLI commands return a successful process exit with `status: not_implemented`.

---

## 4. Final target architecture and invariants

```text
Agent / CutRight Studio
        |
        | typed intents and canonical project commands
        v
CutRight Rust control plane
  ├── immutable source registry
  ├── variant-scoped edit graph
  ├── artifact receipts and cache
  ├── review ledger and approved-base projection
  ├── rendering and per-deliverable QA
  └── packaging and interchange
        |
        ├── HeardRight engine
        |     ├── Parakeet TDT timed transcription
        |     ├── Silero VAD regions
        |     ├── model/runtime discovery
        |     └── health and capability protocol
        |
        ├── WhisperX verifier
        |
        ├── one resolved FFmpeg/FFprobe toolchain
        |
        └── focused sidecars
              ├── Apple Vision temporal analysis
              └── renderer-specific workers
```

The following invariants must hold:

1. A final render is produced from one explicitly selected rough-cut variant.
2. Cut plan, timeline, transcript, captions, reframe plan, finish plan, final, QA, and export all reference the same variant and artifact hashes.
3. Studio sends review **intent**; Rust constructs authoritative decision records.
4. No review record is silently erased because current project state changed.
5. HeardRight remains the local transcript authority; WhisperX is the independent alignment verifier.
6. CutRight does not reach into HeardRight's model internals.
7. Every external process has a timeout, bounded output, explicit environment, and structured failure.
8. Every canonical artifact is schema-valid and has a receipt binding inputs, parameters, tool versions, and output hashes.
9. A JSON `status: error` cannot be accompanied by process exit zero.
10. A packaged Studio build can access only explicitly granted project media and evidence.
11. Cloud use remains explicit, budgeted, and off by default.
12. Sources are never modified.

---

## 5. P0-A — Repair the Studio review contract

Do this before treating Phase 3 as complete or collecting preference data.

### 5.1 Separate frontend intent from authoritative record

Do not make React construct the persisted record. Replace the current `Decision` command input with a minimal typed intent.

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "target_kind", rename_all = "snake_case")]
pub enum ReviewTarget {
    Variant { variant: String },
    Final { preset: String },
    Segment { variant: String, segment_id: String },
    QaReport { preset: Option<String> },
}

#[derive(Serialize, Deserialize)]
pub struct DecisionIntent {
    pub schema_version: u32,
    pub client_request_id: String,
    pub target: ReviewTarget,
    pub verdict: DecisionVerdict,
    pub reason: String,
    pub note: Option<String>,
    pub playhead_ms: i64,
    pub word_id: Option<String>,
    pub source_word_id: Option<String>,
}
```

Rust resolves and adds:

- `decision_id`;
- server timestamp;
- `project_id`;
- canonical project-relative subject;
- subject artifact hash and size;
- selected variant/preset;
- benchmark report hash and decision;
- project/review revision hash;
- application version from Tauri package metadata;
- current schema and protocol versions.

The command returns the complete persisted `DecisionRecord`; the frontend adds that returned value to state.

### 5.2 Make reason vocabularies target-specific types

Use separate enums rather than one runtime string table:

- variant: `pacing`, `word_edges`, `energy`, `length`, `other`;
- final: `looks_right`, `captions`, `loudness`, `framing`, `color`, `audio`, `other`;
- segment: `clipped_word`, `too_tight`, `too_loose`, `bad_boundary`, `wrong_take`, `other`;
- QA: `reviewed`.

The UI renders reasons from the target type. `other` requires a trimmed note of 1–200 characters. The backend still validates independently.

### 5.3 Restrict review controls by mode

- Sources mode: source inspection, source verification, and relink only; no variant verdict.
- Compare mode: variant verdict, segment flags, and explicit “Use for final” selection.
- Finals mode: one verdict per preset, not always `finals[0]`.
- QA mode: acknowledge the exact QA report hash.

Keyboard commands and the command palette must obey the same availability rules.

### 5.4 Make records content-bound and retry-safe

A decision is valid only for the artifact reviewed. Persist:

```json
{
  "decision_id": "...",
  "client_request_id": "...",
  "subject": "render/rough-cuts/natural.mp4",
  "subject_blake3": "...",
  "subject_size": 12345678,
  "project_revision": "...",
  "bench_report_blake3": "...",
  "app_version": "0.1.0"
}
```

Use `client_request_id` for idempotency so a retry after an uncertain IPC result does not append twice.

### 5.5 Make append durable and concurrency-safe

Extract decision storage from `src-tauri/main.rs` into a testable module.

Required behavior:

1. serialize the complete record plus newline into one buffer;
2. acquire a cross-process file lock;
3. append in one write;
4. `sync_data`;
5. release the lock;
6. return the exact record written.

Do not perform two separately interleavable writes for JSON and newline.

### 5.6 Preserve stale history instead of discarding it

Replay result:

```rust
pub struct DecisionReplay {
    pub records: Vec<DecisionWithStatus>,
    pub malformed_lines: Vec<MalformedDecisionLine>,
}

pub enum RecordStatus {
    Current,
    StaleArtifact,
    MissingArtifact,
    Superseded,
}
```

Schema-invalid lines are malformed. A valid historical record whose artifact is no longer current remains visible as stale or missing. The UI shows counts and never silently hides corruption.

### 5.7 Add real boundary tests

The minimum acceptance suite must invoke the actual Tauri command layer or its exact extracted command module:

- valid variant approval appends and replays;
- valid final approval appends and replays;
- `other` retains its note;
- invalid reason/target pair is rejected;
- absolute and traversal subjects cannot be injected because callers do not supply subjects;
- duplicate `client_request_id` is idempotent;
- concurrent appends produce complete JSON lines;
- stale artifact remains in replay;
- malformed tail is reported;
- backend-derived `app_version`, benchmark status, and hashes are present;
- frontend contract fixture round-trips through Rust.

Fix QA fixture IDs to the real forms `ow_000000` and `source-...:w_000000`.

### 5.8 Phase 3 close-out UI items

Complete the features already specified but not currently rendered:

- Verify sources action and progress;
- source mismatch/missing banner;
- relink source by matching BLAKE3 before changing the stored path;
- “N words cut in tight/natural” marker from `swapTarget.cut_count`;
- transcript follow disengage/re-engage behavior;
- segment flag action at cursor;
- per-preset final selection and verdict;
- QA acknowledgement;
- session-open record only if it has a concrete product use; otherwise remove the kind rather than generating noise;
- separate “this session” and total decision counts;
- visible malformed/stale record status;
- visible provisional status when the timestamp benchmark is unresolved.

### 5.9 Acceptance

Phase 3 is closed only when a packaged app can:

1. open a real project;
2. compare natural and tight by source-word identity;
3. persist a valid variant verdict through Rust;
4. reopen and replay it;
5. select the exact approved rough-cut artifact for final rendering;
6. flag a segment at the current word;
7. verify or relink sources;
8. review each final preset;
9. acknowledge the hash-bound QA report;
10. expose stale/malformed records without losing history.

---

## 6. P0-B — Make the edit graph variant-scoped and approval-driven

This is the most important cross-stage correctness refactor.

### 6.1 Replace shared mutable aliases with a variant package

Use the already-created `edit/variants/` directory:

```text
edit/variants/natural/
  cut-plan.json
  timeline.json
  output-transcript.json
  captions.srt
  artifact-receipt.json

edit/variants/tight/
  cut-plan.json
  timeline.json
  output-transcript.json
  captions.srt
  artifact-receipt.json

render/rough-cuts/natural.mp4
render/rough-cuts/tight.mp4
analysis/reframe/natural/plan.json
analysis/reframe/tight/plan.json
finish/natural/finish-plan.json
finish/tight/finish-plan.json
```

Stop writing authoritative state to generic `cut-plan.json`, `timeline.json`, `captions.srt`, or `reframe-plan.json`.

If compatibility aliases are temporarily required, generate them only from the selected variant and write an alias receipt naming the source hash. Never let “last command run” decide their contents.

### 6.2 Add explicit reviewed-base selection

Approval and selection are different actions. Add a `variant_selection` record or a derived projection:

```json
{
  "schema_version": 1,
  "selected_variant": "natural",
  "rough_cut_blake3": "...",
  "timeline_blake3": "...",
  "selected_by_decision_id": "...",
  "selected_at": "..."
}
```

A selection is valid only when:

- the variant verdict is approved for the same artifact hash;
- the benchmark policy permits destructive word-edge cuts;
- source hashes still match;
- its variant receipt is current.

### 6.3 Make every downstream command variant-aware

Required command behavior:

```text
videoctl edit render <project> --variant natural
videoctl review select <project> --variant natural
videoctl reframe plan <project> --variant selected
videoctl finish validate <project> --variant selected
videoctl render final <project> --preset youtube --variant selected
videoctl qa <project> --preset youtube
videoctl export otio <project> --variant selected
```

`selected` resolves the hash-bound review selection. A direct variant name is permitted for development, but release/final commands require an explicit `--allow-unreviewed` override and record that override in provenance.

### 6.4 Implement the actual natural/tight gap policy

Transform candidate word spans into render subsegments:

1. iterate adjacent words within each accepted candidate;
2. compute the original no-word gap;
3. when the gap exceeds the variant threshold, retain only the configured residual pause;
4. create separate source subsegments around the removed interval;
5. apply short audio fades at joins;
6. insert measured room tone only when required to avoid a noise-floor discontinuity;
7. never remove or overlap a word interval;
8. record every removed gap in the cut plan with source and output mappings.

Initial policies from the existing design:

- tight: approximately 220 ms retained pause;
- natural: approximately 350–450 ms retained pause.

The exact values belong in a versioned edit profile, not scattered constants.

### 6.5 Correct cut-plan validation

Current overlap checking also enforces source chronology in output order. Instead:

- validate every source range independently;
- group intervals by source and sort by source time solely to detect unintended overlap;
- allow output reordering of non-overlapping source intervals;
- require an explicit repeat flag if the same source interval is intentionally reused;
- validate output continuity and variant profile separately.

### 6.6 Define an explicit working/output timebase

Do not inherit the whole project timeline rate from the first source. Add a project working timebase and per-output rate, for example rational 30000/1001 or 30/1. Normalize mixed-FPS sources during render while retaining original source milliseconds for provenance.

### 6.7 Migrate existing projects

Provide a schema/layout migration that:

- detects legacy variant files;
- moves/copies them into variant directories;
- determines whether generic timeline/captions can be safely attributed to a variant;
- marks ambiguous state as requiring rerender rather than guessing;
- preserves old files under `migrations/backup-<timestamp>/` until validation succeeds.

### 6.8 Acceptance

Render natural and tight in either order. Then prove:

- each package retains its own plan, timeline, transcript, captions, receipt, and MP4;
- final YouTube/vertical/OTIO outputs all cite the selected variant hash;
- reframe anchors cover that same timeline;
- QA rejects any mixed-variant artifact graph;
- tight removes more eligible pause than natural without clipping words;
- rerunning one variant cannot change the other.

---

## 7. P0-C — Complete the repository gate, licensing, and CI

### 7.1 Resolve the license inconsistency

The root workspace declares MIT but no root license file exists. Resolve intent explicitly:

- if MIT is intended, add the standard MIT text and align the copyright holder across `Cargo.toml`, README, and notices;
- if MIT is not intended, change the Cargo metadata before adding the correct license.

Do not leave public source with contradictory metadata.

### 7.2 Make one local script authoritative

Create `scripts/gate.sh`. CI is only an adapter around it.

```bash
#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings

cargo fmt --manifest-path apps/studio/src-tauri/Cargo.toml -- --check
cargo test --manifest-path apps/studio/src-tauri/Cargo.toml --locked
cargo clippy --manifest-path apps/studio/src-tauri/Cargo.toml \
  --all-targets --locked -- -D warnings

corepack enable
pnpm --dir apps/studio install --frozen-lockfile
pnpm --dir apps/studio typecheck
pnpm --dir apps/studio test
pnpm --dir apps/studio build

swiftlint lint --strict
cargo run -p videoctl -- doctor --profile core --strict
```

Add contract, CLI, and formatting checks as they land.

### 7.3 Keep Studio as a separate Cargo workspace for now

The standalone Studio workspace is defensible because it isolates Tauri's dependency graph and lockfile. Do not merge it solely to satisfy an audit heuristic. Explicitly gate it by manifest path and scan its lockfile.

Reconsider merging only if duplicate dependency maintenance becomes a measured cost.

### 7.4 Add automated CI without making it the source of truth

Required properties:

- runs the same `scripts/gate.sh` on every protected-branch change;
- uses a macOS runner for Swift and platform compile coverage;
- caches only package/compiler caches, never generated project artifacts;
- records the exact commit and toolchain versions;
- publishes the gate receipt;
- makes the required status non-skippable on `main`.

The adapter can be GitHub-hosted, self-hosted, or another CI service. The repository contract remains the local script.

### 7.5 Add scheduled supply-chain checks

The per-change gate stays deterministic. Scheduled/release checks add network-dependent work:

- root `Cargo.lock` audit;
- Studio `Cargo.lock` audit;
- pnpm production audit;
- dependency drift report;
- gitleaks;
- Semgrep;
- license/notice inventory;
- toolchain-manifest verification.

### 7.6 Fix QA portability

Replace absolute `/Volumes/D/claude/...` script paths with one of:

- a repository-local QA runner;
- a configured `CUTRIGHT_QA_TOOLS` root with a clear missing-tool error;
- a shared package resolved through the workspace.

The gate must not depend on one developer volume layout.

### 7.7 Add frontend formatting

Format `word-lock.ts` and its test normally. Add a formatter check for all Studio TS/CSS files. The current single-line implementation is behaviorally tested but unnecessarily hostile to review.

### 7.8 Gate acceptance

A clean checkout on the supported macOS runner must pass without private model files or cloud credentials. Provider-dependent product acceptance belongs in the target-machine gate, not in hermetic CI.

---

## 8. P1 — Correct the transcription benchmark and schema contract

### 8.1 Replace provider election with primary validation

The report should distinguish:

- `transcript_authority`: HeardRight;
- `timestamp_authority`: HeardRight or WhisperX alignment;
- `verifier`: WhisperX;
- `status`: `primary_accepted`, `verifier_edges_required`, `manual_review_required`, or `verifier_unavailable`.

Decision policy:

1. HeardRight clean and verifier coverage sufficient → accept HeardRight, including when WhisperX is also clean.
2. HeardRight content is acceptable but edge checks fail while WhisperX alignment is clean → keep HeardRight text and use WhisperX edge timings.
3. Both disagree materially or neither is clean → manual review; destructive cuts blocked.
4. Verifier unavailable → transcript may be viewed, but destructive word-edge automation remains unverified.

Do not ever switch the product transcript engine merely because the verifier produced one cleaner sample.

### 8.2 Stop requiring zero unmatched words

Record:

- normalized alignment coverage;
- unmatched content rate;
- start/end delta distributions;
- sampled boundary statuses;
- rendered-probe human verdicts;
- provider/model/protocol hashes.

Unmatched punctuation, contractions, token splitting, and genuine ASR disagreement must be distinguished.

Keep the existing requested padding as an explicit benchmark parameter. Put acceptance thresholds in a versioned benchmark policy file and calibrate them on the real fixture set.

### 8.3 Make the report binding

The benchmark report must bind:

- source BLAKE3 values;
- both normalized transcript hashes;
- raw response/envelope hashes;
- HeardRight engine/model/protocol identity;
- WhisperX environment/model identity;
- policy version;
- rendered probe hashes;
- final decision.

Any input change invalidates the decision.

### 8.4 Fix the transcript schema now

Add optional `source_word_id` to `transcript.schema.json`, with the expected compound-ID pattern. Add strict unknown-field handling for canonical artifacts under a fixed schema version.

Then add a full schema/Rust guard:

- representative valid fixtures per schema version;
- invalid fixtures for required constraints;
- every emitted canonical artifact validated before atomic write in debug/test and in a dedicated contract gate;
- Rust deserialize/serialize round-trip;
- migration tests from every supported prior version;
- generated TypeScript contract fixtures for Studio.

### 8.5 Add semantic validation beyond JSON Schema

Enforce in Rust:

- `end_ms > start_ms`;
- sorted/non-overlapping word timelines where required;
- unique word IDs;
- globally unique compound `source_word_id` in output transcripts;
- valid source references;
- rational timebase values;
- variant artifact consistency.

---

## 9. P1 — Make HeardRight the single local-audio service boundary

CutRight currently uses HeardRight for timed ASR but separately owns a Silero worker and knows HeardRight's internal engine/model paths. Consolidate the product boundary without importing HeardRight source or models into the public CutRight repository.

### 9.1 HeardRight protocol addition

Add an additive capability such as:

```text
file_transcription_timed_words_v1
file_vad_regions_v1
```

Add request/result payloads:

```rust
FileVadRequest {
    path: String,
    threshold: Option<f32>,
    min_speech_ms: Option<u32>,
    min_silence_ms: Option<u32>,
}

FileVadResult {
    sample_rate: u32,
    provider: String,
    model_revision: String,
    threshold: f32,
    min_speech_ms: u32,
    min_silence_ms: u32,
    regions: Vec<TimedVadRegion>,
}
```

HeardRight owns model discovery, runtime loading, and platform backend choice. CutRight supplies media and policy, not model-directory paths.

### 9.2 CutRight client behavior

One supervised HeardRight session implements both transcription and VAD provider traits.

Required protocol behavior:

- health/capability handshake before use;
- unique request and trace IDs;
- exact response correlation;
- protocol-major rejection and minor-version negotiation;
- per-request timeout;
- bounded stderr capture;
- one controlled restart after unexpected engine exit;
- no model download or network fallback;
- engine/model/protocol identity returned in provenance.

### 9.3 Discovery order

1. explicit `CUTRIGHT_HEARDRIGHT_ENGINE` development override;
2. installed HeardRight engine location appropriate to the platform;
3. `heardright-engine` on `PATH`;
4. clear unavailable result.

Remove hard-coded `/Volumes/D/claude/...` defaults and `CUTRIGHT_HEARDRIGHT_MODELS_DIR` from the normal product boundary.

### 9.4 Migrate VAD safely

1. add HeardRight file-VAD support;
2. run both implementations on the immutable benchmark clips;
3. compare regions, boundary deltas, missed speech, false speech, and cut outcomes;
4. make HeardRight the default after the acceptance gate passes;
5. retain a short rollback window;
6. delete CutRight's Swift Silero worker, build script, model-path variables, and duplicate tests;
7. retain only provider-contract fixtures in CutRight.

### 9.5 Do not combine both file requests until measured

A later combined file-analysis request may avoid duplicate decode, but first land the narrow compatible protocol. Optimize only after profiling shows decode duplication matters.

---

## 10. P1 — External-process reliability, provenance, and resumability

### 10.1 Add a shared process runner

Every external command—FFmpeg, FFprobe, HeardRight, WhisperX, Swift workers—must go through one abstraction with:

- executable identity;
- argument list kept separate from logs;
- explicit environment allow-list;
- working directory;
- timeout and kill-tree behavior;
- stdout/stderr byte caps;
- structured exit code/signal;
- cancellation support;
- temporary-file cleanup;
- duration telemetry.

No command may wait indefinitely.

### 10.2 Make embedded workers content-addressed

Materialize embedded binaries by content hash, not crate version alone:

```text
$TMPDIR/cutright-workers/<sha256>/vision-anchor
$TMPDIR/cutright-workers/<sha256>/caption-card
```

Verify bytes before execution. Apply the same rule to every embedded sidecar.

### 10.3 Unify FFmpeg and FFprobe resolution

Introduce `MediaToolchain`:

```rust
pub struct MediaToolchain {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub version: String,
    pub sha256: String,
    pub capabilities: MediaCapabilities,
}
```

Both binaries must come from the same resolved toolchain or a verified compatible pair. Store toolchain identity in every render/probe receipt.

### 10.4 Add full stage receipts

Create a common receipt format:

```json
{
  "schema_version": 1,
  "stage": "render.rough_cut",
  "implementation_version": "...",
  "inputs": [{"path":"...","blake3":"..."}],
  "parameters_blake3": "...",
  "toolchains": {"ffmpeg":"...","heardright":"..."},
  "outputs": [{"path":"...","blake3":"...","size":123}],
  "created_at": "..."
}
```

Use it for ingest metadata, ASR, VAD, candidates, cut plan, timeline, rough renders, transcript remap, evidence, reframe, finish, final, QA, and package.

### 10.5 Make caching content-addressed

Cache keys must use source content and policy, not machine-local paths. For transcription, remove absolute source path from the cache identity; retain it only as provenance. Include:

- source BLAKE3;
- decode policy and media-toolchain identity;
- provider/model/protocol identity;
- language/context policy;
- stage implementation version.

A moved but byte-identical source can reuse valid analysis after relinking.

### 10.6 Harden atomic writes

Use unique temp files in the destination directory, `create_new`, file sync, atomic replace, and parent-directory sync where supported. PID-only temp names are insufficient under concurrency.

### 10.7 Add VAD provenance

The normalized VAD artifact or its envelope must record:

- source and decoded-audio hash;
- model revision/hash;
- runtime/backend;
- threshold;
- minimum speech/silence durations;
- sample rate and decode policy;
- request hash;
- warnings.

### 10.8 CLI exit semantics

- `status: error` → nonzero exit;
- unavailable required doctor capability → nonzero exit;
- `not_implemented` → stable nonzero exit code;
- invalid command/config → distinct stable code;
- JSON remains on stdout; logs remain on stderr.

### 10.9 Add CLI and process-boundary smoke tests

Use `assert_cmd`, temporary projects, and fake executables to cover the dispatch layer that currently has no direct tests:

- `doctor` JSON shape and exit code for ready and missing-capability cases;
- `project init` idempotency through the actual binary;
- `ingest --dry-run` produces no writes;
- unsupported and not-yet-implemented commands fail nonzero;
- fake HeardRight frames test request correlation, malformed JSON, timeout, early EOF, and restart;
- fake WhisperX tests nonzero exit, oversized output, invalid response, and temp cleanup;
- stdout is always valid JSON and diagnostics remain on stderr;
- cancellation terminates child processes rather than leaving workers running.

---

## 11. P1 — Make `videoctl doctor` truthful

### 11.1 Profiles

```text
videoctl doctor --profile core
videoctl doctor --profile audio
videoctl doctor --profile render
videoctl doctor --profile studio
videoctl doctor --profile all
```

### 11.2 Core probes

- project temp-directory create/write/rename/delete;
- resolved FFmpeg and FFprobe execute successfully;
- versions and hashes match the toolchain lock;
- required schemas load;
- sidecar embedded bytes can materialize;
- source policy is immutable;
- cloud default is disabled.

### 11.3 Audio probes

- HeardRight engine discovery;
- health and capability handshake;
- timed-file-transcription capability;
- file-VAD capability after it lands;
- model/runtime ready without download;
- WhisperX interpreter/import/version;
- required aligner model already cached for offline use.

### 11.4 Render probes

- `h264_videotoolbox` listed and an actual tiny encode succeeds;
- `zscale` works on a generated HDR-like test frame;
- software delivery encoder available;
- caption renderer or libass path works;
- audio encode works;
- output can be probed by the paired FFprobe.

### 11.5 Studio probes

- frontend bundle exists;
- vendored fonts and notices exist;
- Tauri asset protocol enabled;
- packaged-app smoke fixture can load one allowed preview and reject one outside path.

### 11.6 Output and exits

Each check reports:

```json
{
  "id": "render.h264_videotoolbox.smoke",
  "status": "ok|degraded|missing|failed",
  "required": true,
  "evidence": {...},
  "remediation": "..."
}
```

Exit zero only when all required checks in the selected profile pass. `--strict` also fails degraded optional checks.

### 11.7 Receipts

`doctor --write-receipt <path>` writes a timestamped, hashable machine-readiness record used by release gates and support diagnostics.

---

## 12. P1 — Studio snapshot, asset scope, and project integrity

### 12.1 Do not silently turn corruption into absence

Replace `Option<T>` reads that swallow JSON errors with explicit state:

```rust
pub enum ArtifactState<T> {
    Missing,
    Ready(T),
    Invalid { path: PathBuf, error: String },
    Stale { path: PathBuf, reason: String },
}
```

Studio must distinguish “not generated” from “generated but corrupt.”

### 12.2 Separate expected and actual media facts

For finals and rough cuts return:

- expected width/height/fps/duration;
- probed width/height/fps/duration;
- probe status/error;
- artifact hash/receipt.

Never display configured dimensions as though they were measured output facts.

### 12.3 Replace timestamp-only snapshot identity

`generated_at` is useful for display but not for staleness. Add `project_revision`, a hash over the canonical review inputs and artifact receipts.

### 12.4 Tighten asset grants

Do not recursively grant the whole project tree when the UI only needs selected media/evidence. Grant:

- exact source media files;
- exact rough/final MP4s;
- exact poster/waveform/evidence assets;
- no arbitrary feedback, configuration, or unrelated project files.

Document the trust model for project packages. A shared/untrusted project must not be able to grant arbitrary local paths merely by editing `sources/manifest.json`.

At minimum, external source grants require:

- regular file;
- registered source ID;
- supported media probe;
- manifest hash match or an explicit unverified state before playback.

### 12.5 Packaged-app tests

Test both:

- allowed project preview loads through the asset protocol;
- sibling/outside file is denied.

Do this in a built Tauri app, not only Vite QA mode.

### 12.6 Source relinking

A missing source can be relinked only to a file whose BLAKE3 matches the registered source. Update the path atomically and preserve a relink history record. Do not create a new source implicitly.

### 12.7 Give every project a genuinely unique identity

New projects currently derive `project_id` from the folder name, so two projects with the same name collide. Generate a random UUID/ULID at creation and keep the human title separate.

For existing projects:

- retain the old ID for backward compatibility;
- add a new immutable `project_instance_id` during schema migration;
- use the instance ID for decision and receipt identity;
- never regenerate it on folder rename or relink.

### 12.8 Make source-integrity tests part of the real backend suite

Test missing files, content mismatch, successful hash verification, progress events, relink success, relink hash rejection, and a manifest that tries to reference an unsupported or unsafe external path.

---

## 13. P1 — Media pipeline and QA correctness

### 13.1 Generate evidence from actual edit boundaries

Current evidence is candidate-centered. Build final decision evidence from the selected variant timeline:

- source frame sequence immediately before/at/after every actual cut;
- output frame sequence around the join;
- source and output waveform;
- previous/next word IDs;
- removed-gap duration;
- source/output time mapping;
- render artifact hash.

### 13.2 Make QA per deliverable

Replace one YouTube-only report with:

```text
qa/youtube.report.json
qa/reels.report.json
qa/tiktok.report.json
qa/summary.json
```

Each report binds the exact final hash and checks:

- expected/probed dimensions and frame rate;
- duration against selected timeline;
- video/audio streams;
- decode-through-end;
- black/frozen tail;
- caption artifact and timing coverage;
- loudness, true peak, and clipped samples;
- reframe-plan identity for vertical;
- source and selected-variant hashes;
- benchmark decision;
- required human final verdict.

### 13.3 Use preset-specific captions

Each final references captions generated from its selected timeline and caption profile. Do not copy one generic SRT into every package by assumption.

### 13.4 Replace per-cue process spawning for baseline captions

The current caption-card path launches one Swift process per cue. Use:

- ASS/libass for cheap deterministic baseline captions;
- a persistent/batch renderer for card images;
- Remotion later for animated profiles.

### 13.5 Package with a manifest

Every export package includes:

- artifact paths;
- hashes and sizes;
- selected variant;
- preset and profile versions;
- QA report hash;
- caption artifact hash;
- creation time;
- toolchain identity.

Copying files alone is not a release package.

### 13.6 OTIO correctness

Use a proper file-URL encoder and variant-specific timeline. Add fixtures for spaces, Unicode, `#`, `%`, and non-ASCII paths.

---

## 14. P2 — Behavior-preserving decomposition

Do this only after P0 and the contract/gate safety nets are green.

### 14.1 `video-project`

Keep one crate and split by responsibility:

```text
src/
  lib.rs
  project_init.rs
  ingest.rs
  transcription.rs
  benchmark.rs
  analysis.rs
  candidates.rs
  cut_plan.rs
  timeline.rs
  remap.rs
  rough_render.rs
  reframe.rs
  evidence.rs
  finish.rs
  final_render.rs
  qa.rs
  shorts.rs
  package.rs
  export.rs
  snapshot.rs
  receipts.rs
  io.rs
```

`lib.rs` re-exports the existing public API during the pure-move phase.

### 14.2 `video-media`

```text
src/
  lib.rs
  toolchain.rs
  process.rs
  probe.rs
  audio.rs
  rough_render.rs
  final_render.rs
  captions.rs
  waveform.rs
  evidence.rs
  reframe.rs
```

### 14.3 `video-providers`

```text
src/
  lib.rs
  heardright.rs
  whisperx.rs
  protocol_session.rs
  fake.rs
```

The standalone VAD module disappears after HeardRight migration.

### 14.4 Studio frontend

```text
src/
  App.tsx
  contracts/
  hooks/
    useProject.ts
    usePlayback.ts
    useReviewLedger.ts
    useKeyboard.ts
  modes/
    SourcesMode.tsx
    CompareMode.tsx
    FinalsMode.tsx
    QaMode.tsx
  components/
  word-lock.ts
  fixtures/
```

### 14.5 Studio backend

```text
src-tauri/src/
  main.rs
  commands.rs
  project_scope.rs
  decision_contract.rs
  decision_store.rs
  source_integrity.rs
  tests/
```

### 14.6 CLI

Move doctor and dispatch helpers into modules once smoke tests exist. Keep the clap surface stable.

### 14.7 Refactor rules

- pure file moves first;
- no schema or behavior changes in decomposition commits;
- full gate before and after every move;
- compare JSON fixtures, command output, and render argument snapshots;
- avoid crate proliferation unless a real dependency/lifecycle boundary demands it.

---

## 15. Product-completion roadmap

### 15.1 Phase 3 — review workflow close-out

Completion is the P0-A/P0-B work above. The UI exists; correctness and execution integration remain.

Also replace the current first-source-only `analysis/transcript-packed.md` with a project-level transcript pack covering every source, with clear source headings, word/time references, and a content hash. The agent must never receive a silently incomplete multi-source transcript.

### 15.2 Phase 4 — captions, audio, color, and export

#### Captions

- canonical word/phrase caption model, not SRT as source of truth;
- reading-speed and line-length constraints;
- punctuation-aware phrase grouping;
- safe zones per platform;
- collision handling with subject tracks;
- sidecar SRT/VTT and optional burned profiles;
- deterministic font fallback and notice inventory;
- per-preset caption receipt.

#### Audio

- dialogue-only analysis and cached processed stem;
- configurable high-pass, gentle compression, de-ess, limiter;
- integrated loudness and true-peak measurement;
- no clipped samples;
- music ducking under speech;
- room-tone continuity across cuts;
- profile defaults such as approximately −14 LUFS / −1 dBTP, versioned rather than hard-coded globally;
- waveform/evidence for problem joins.

#### Color

- input color-space detection;
- HDR/HLG/PQ to defined SDR working space;
- Apple Log conversion path when applicable;
- exposure/white-balance correction;
- shot matching;
- optional approved creative LUT at bounded strength;
- output metadata verification;
- review contact sheet.

#### Export

- software/master delivery path in addition to hardware preview;
- preset profiles for YouTube, Reels, TikTok, archive/master;
- checksummed package manifest;
- upload acceptance smoke on representative outputs.

### 15.3 Phase 5 — effect and render library

Implement a typed effect registry:

```json
{
  "effect_id": "caption.bold-karaoke.v1",
  "renderer": "remotion",
  "schema_version": 1,
  "props_schema": "...",
  "safe_zones": ["vertical-bottom", "youtube-lower-third"],
  "motion_profile": "restrained",
  "preview_fixture": "..."
}
```

Build order:

1. caption profiles;
2. lower third;
3. stat counter;
4. quote card;
5. CTA end card;
6. remaining frequently used effects.

Every effect requires still preview, motion preview, reduced-motion behavior where relevant, collision test, and render receipt. Pin the exact renderer version and re-verify its commercial license terms before this phase is promoted.

### 15.4 Phase 6 — real shorts extraction

Replace duration ranking with:

1. semantic segmentation;
2. standalone-context validation;
3. hook/payoff/proof/value scoring;
4. duration fit;
5. visual support;
6. platform/brand fit;
7. diversity clustering so paraphrases do not occupy all slots;
8. four cheap previews;
9. explicit human selection.

Each candidate includes source/output mapping, transcript, rationale, score breakdown, and truthfulness/reordering note.

### 15.5 Phase 7 — temporal visual perception and reframing

Replace one midpoint face box with sampled temporal tracks:

- faces and active speaker evidence;
- body/hands;
- OCR boxes;
- saliency/objectness;
- shot boundaries;
- confidence and gaps;
- smoothing with bounded acceleration;
- safe-zone cost function;
- manual anchors where confidence is low.

Acceptance fixtures include one moving subject, alternating speakers, gesture crossing, no-face interval, OCR-heavy screen capture, rapid cut, and multi-subject handoff.

### 15.6 Phase 8 — optional cloud analysis

Build only after the local benchmark passes. Requirements:

- explicit per-project consent;
- hard budget limit;
- proxy-versus-source upload policy;
- cache by content hash and model capability;
- retention/deletion action;
- outage fallback;
- no duplicate uploads;
- current official API/model/license re-verification immediately before implementation.

Cloud output is advisory semantic evidence, never timestamp authority.

### 15.7 Phase 9 — preference learning

Learn only from current, hash-bound, target-specific records:

- boundary corrections;
- selected variant and pause policy;
- candidate rejection reasons;
- caption profile;
- effect density;
- subject/crop corrections;
- SFX choices;
- hook/CTA structure;
- post-publish retention mapped to exact output time.

Recommendations must cite the decisions that caused them. Autonomy advances per format after enough reviewed projects; it is never a global default.

---

## 16. Documentation and repository hygiene

### 16.1 Add an as-built status file

`STATUS.md` should contain:

```yaml
as_of_commit: a8d4584f2a01f51d07d7018707eb0aca83d97adc
current_stage: phase_3_closeout
primary_audio_engine: HeardRight
primary_asr: Parakeet TDT v3
word_edge_verifier: WhisperX
cloud_default: disabled
last_full_gate: <receipt>
known_blockers:
  - studio_decision_contract
  - variant_artifact_consistency
  - complete_ci_gate
```

Update it from the gate/release process, not manually from memory.

### 16.2 Correct stale architecture text

- remove “nothing here is built yet”;
- mark exact implemented/partial/unimplemented phases;
- replace stale ScrapeRight-as-primary-transcriber references with HeardRight;
- document that workspace-external files such as `PIPELINES.md` are external rather than missing;
- update README quick start with current commands and the selected-variant flow;
- distinguish bridge instructions from the current product path.

### 16.3 Pin developer/runtime environments

- tracked FFmpeg/FFprobe toolchain lock with version, build flags, source, hashes, required capabilities;
- tracked WhisperX environment lock based on the known working environment rather than a prose-only Python version;
- `.node-version` or equivalent and `engines` field;
- existing `packageManager` pin retained;
- Rust toolchain file if the project requires an exact version.

### 16.4 Conditional cleanup

- inspect references before moving `how-to.html`; archive under `docs/archive/` if historical;
- use per-run temporary browser profiles and guaranteed cleanup;
- record the known Semgrep Bash parser false positive rather than rewriting valid shell;
- fix only meaningful executable-bit mistakes; do not chase directory-mode cosmetics Git does not preserve.

### 16.5 Add contributor guidance

`CONTRIBUTING.md` should state:

- run `scripts/gate.sh`;
- canonical schemas and migration rule;
- no raw source mutation;
- no untyped FFmpeg invocation;
- how to add a provider capability;
- how to add a project artifact receipt;
- how to update Studio contracts;
- phase gate and evidence requirements.

---

## 17. Recommended implementation sequence

### Hotfix 1 — Studio decision path

- introduce `DecisionIntent` and authoritative `DecisionRecord`;
- derive subject, hashes, project ID, benchmark state, and app version in Rust;
- fix mode/target-specific reasons and note handling;
- add real append/replay integration tests;
- surface malformed/stale records;
- remove verdict controls from Sources mode.

**Exit:** a packaged Studio app successfully appends and reloads variant and final decisions.

### Hotfix 2 — variant artifact graph and selection

- move timeline/transcript/captions/reframe/finish state under variants;
- add approved-base selection;
- make final, QA, OTIO, and reframe resolve the same selected hash;
- implement effective gap compaction;
- migrate legacy generic files.

**Exit:** natural and tight can be rendered in either order with no cross-contamination.

### PR 3 — license and complete gate

- resolve root license;
- add `scripts/gate.sh`;
- gate both Cargo workspaces and Studio frontend;
- add automated CI status;
- remove machine-specific QA paths;
- add formatter gate.

### PR 4 — benchmark and contract correction

- change benchmark to primary-plus-verifier policy;
- add report binding and realistic alignment metrics;
- add `source_word_id` to schema;
- add schema/Rust/golden/migration tests;
- make unimplemented/error exit codes nonzero.

### HeardRight PR 1 — file VAD capability

- add file-VAD request/result and capability;
- reuse existing Silero runtime;
- add batch-region tests and protocol compatibility tests;
- expose runtime/model provenance.

### CutRight PR 5 — HeardRight client boundary

- remove model-directory knowledge and hard-coded developer paths;
- add handshake, correlation, timeout, restart, and fake-engine tests;
- implement transcription and VAD over one session;
- parity-test and remove duplicate VAD worker.

### PR 6 — receipts, process runner, and toolchain

- shared external-process runner;
- content-addressed sidecar materialization;
- unified FFmpeg/FFprobe resolver;
- toolchain lock and active doctor probes;
- stage receipts and cache identity.

### PR 7 — Studio integrity and packaged scope

- explicit artifact states;
- exact asset grants;
- packaged positive/negative tests;
- source verify/relink UI;
- finish Phase 3 cut marker/follow/segment/QA actions.

### PR 8 — per-deliverable QA and packaging

- preset-specific captions;
- QA report per final;
- audio/color/caption metrics;
- package manifests and hashes;
- proper OTIO file URLs.

### PR 9 — pure-move decomposition

- split `video-project`;
- split `video-media`;
- split Studio frontend/backend;
- preserve public behavior.

### PR 10+ — product phases

- Phase 4 finish quality;
- Phase 5 effects;
- Phase 6 semantic shorts;
- Phase 7 temporal reframing;
- optional cloud;
- preference learning.

---

## 18. Acceptance matrix

| Area | Required evidence |
|---|---|
| Baseline | exact `a8d4584...` or later SHA recorded with full gate receipt |
| Decision IPC | real frontend intent reaches Rust and complete record replays |
| Decision integrity | hash-bound, idempotent, concurrent-safe, stale-preserving ledger tests |
| Review gating | selected approved rough-cut hash is required for final render |
| Variant isolation | natural/tight plans, timelines, captions, reframe, finish, and receipts are independent |
| Gap policy | tight compacts more eligible pause; neither variant clips words |
| License | root file and metadata agree |
| Gate | root Rust, Studio Rust, Studio TS, Swift, contracts, and CLI covered |
| CI | required protected-branch status invokes the local gate |
| Benchmark | HeardRight remains transcript authority; verifier policy produces stable status |
| Schema | `source_word_id` represented; fixtures and migrations pass |
| HeardRight | no internal model paths; timed ASR and VAD capabilities negotiated |
| Process safety | timeout, output cap, environment allow-list, cleanup, restart tests |
| Toolchain | FFmpeg and FFprobe pair verified by hash and capabilities |
| Doctor | active profile probes and correct process exit |
| Snapshot | missing, invalid, stale, and ready artifacts are distinct |
| Asset scope | built app loads allowed files and denies outside files |
| Evidence | actual selected-timeline boundaries represented |
| QA | report for every final, bound to exact artifact hash |
| Captions/audio/color | profile-specific artifacts and measurable gates |
| Vertical | selected-variant temporal reframe fully approved |
| Shorts | four materially different standalone candidates |
| Packaging | checksummed manifest including QA and selected variant |
| Privacy | no implicit network, model download, or upload |
| Release | target-Mac product gate plus human review passes |

---

## 19. Non-goals for this campaign

- no replacement of HeardRight's Parakeet/Silero product stack;
- no second local Parakeet wrapper presented as independent verification;
- no cloud dependency in the default path;
- no automatic final render from an unreviewed or stale rough cut;
- no generic mutable timeline used across variants;
- no silent repair of corrupt project artifacts;
- no crate explosion during decomposition;
- no Rust edition migration mixed into correctness hotfixes;
- no deletion of historical assets without reference checks;
- no “green” gate that ignores Studio, nested lockfiles, hardware features, or provider readiness.

---

## 20. Bottom line

The pushed Studio work should be kept. It establishes the right review shape: source inspection, word-locked comparison, finals, QA, keyboard operation, and an append-only decision concept.

It is not yet safe to call the review workflow complete because the current frontend and backend decision contracts do not meet, shared variant aliases can mix state, and review results do not control final execution.

Fix those boundaries first. Then make the complete repository enforceable, turn HeardRight into the clean local-audio service CutRight already expects, add truthful diagnostics and artifact receipts, and complete the editing phases behind measurable gates.

The finished product contract is:

> Give CutRight a folder of footage. It preserves the sources, produces independently reviewable natural and tight edits, lets the user select one exact hash-bound base, finishes long-form and vertical deliverables with correct captions/audio/color, emits per-output QA and provenance, and uses HeardRight locally without hidden cloud behavior or duplicate audio ownership.

---

## 21. Source basis

This revision consolidates:

- `SYSTEM-AUDIT-2026-07-30.md`;
- `AUDIT-2026-07-30.md`;
- `Orthic-Labs/CutRight` at `a8d4584f2a01f51d07d7018707eb0aca83d97adc`;
- the current HeardRight engine protocol, timed file-transcription contract, and Silero implementation;
- direct review of the current Studio frontend/backend, variant render/remap flow, benchmark decision logic, snapshot behavior, media worker materialization, QA, and final-render wiring.

Audit-reported mechanical gate results are preserved as audit evidence. Newly identified correctness findings are code-review conclusions and must be proven by the first implementation tests.
