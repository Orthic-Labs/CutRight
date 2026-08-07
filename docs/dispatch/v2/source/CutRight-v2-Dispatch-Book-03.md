# CutRight v2 Dispatch Book 3: Signed Runtime Packs, Hierarchical Evidence Graph, and Durable Job Plane

**Tasks:** 27  
**Goal:** Replace every system-tool and sibling-app dependency with signed CutRight packs, then build bounded multimedia evidence retrieval and content-addressed resumable jobs.  
**Execution default:** A single agent runs numeric order. The labels show safe parallelism for an authorised dispatcher with isolated checkouts; this document does not itself authorise new branches or worktrees.  
**Testing cadence:** Focused checks run inside tasks. The full authoritative local gate runs only in task `CR-V2-B3-027`.  
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
CR-V2-B3-001 .. 006    sequential contract/interface freeze
CR-V2-B3-007 .. 011    parallel lane A
CR-V2-B3-012 .. 016    parallel lane B
CR-V2-B3-017 .. 021    parallel lane C
CR-V2-B3-022 .. 027    sequential merge, integration, acceptance, gate
```

## CR-V2-B3-001 [S] — Freeze runtime pack, lock, signature, and compatibility schemas

**Depends on:** Book 2 final gate  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B3-001: freeze-runtime-pack-lock-signature-and-compatibility-schem`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `schemas/runtime/pack-manifest.schema.v1.json`
- `schemas/runtime/pack-lock.schema.v1.json`
- `schemas/runtime/pack-signature.schema.v1.json`
- `docs/architecture/V2-RUNTIME-PACKS.md`

**Procedure**

1. Define immutable pack ID/version/target, file entries, source and licence links, requirements, capabilities, measurements, compatibility, signature and rollback metadata.
2. Require SHA-256 and BLAKE3 for each shipped file and a hash of the complete sorted manifest.
3. Separate non-release fixture manifests from signable release locks; zero/empty hashes and measurements are invalid in release locks.
4. Define pack compatibility with application, project, benchmark profile and other packs.

**Required implementation shape**

```text
pub struct PackLock { pub pack: PackId, pub version: Version, pub target: TargetTriple, pub manifest_hash: Hash, pub files: Vec<LockedFile>, pub measured: Measurements, pub signature: Signature }
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/runtime/pack-manifest.schema.v1.json fixtures/schemas/runtime/pack-manifest/v1/valid/basic.json
python3 scripts/schema-check.py schemas/runtime/pack-lock.schema.v1.json fixtures/schemas/runtime/pack-lock/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- Release locks cannot contain mutable URLs, missing measurements or unresolved licences.
- Compatibility is explicit rather than inferred from semantic version alone.
- Signature covers every file entry and compatibility declaration.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-001: freeze-runtime-pack-lock-signature-and-compatibility-schem`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-002 [S] — Freeze the hierarchical evidence graph schema

**Depends on:** CR-V2-B3-001  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B3-002: freeze-the-hierarchical-evidence-graph-schema`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `schemas/evidence/graph.schema.v1.json`
- `schemas/evidence/node.schema.v1.json`
- `schemas/evidence/edge.schema.v1.json`
- `docs/architecture/V2-EVIDENCE-GRAPH.md`

**Procedure**

1. Define node kinds for source, scene, shot, visual event, frame, face, subject, pose, gesture, text region, motion region, audio stream, speaker turn, utterance, word, speech region, music section, bar, beat, transient, editorial beat, claim and asset.
2. Require source/revision identity, rational time range, confidence, producer capability/version, parameter hash, receipt and bounded payload.
3. Define typed edges including contains, overlaps, supports, contradicts, derived_from, same_subject, same_take, spoken_by, visualises and synchronised_with.
4. Prohibit raw model prose as canonical node identity or timing.

**Required implementation shape**

```text
pub struct EvidenceNode { pub id: EvidenceId, pub kind: EvidenceKind, pub source_revision: RevisionId, pub range: Option<RationalRange>, pub confidence: f32, pub producer: ProducerIdentity, pub receipt: ReceiptRef }
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/evidence/graph.schema.v1.json fixtures/schemas/evidence/graph/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every derived node traces to immutable source bytes.
- Time-less semantic nodes still trace to timed supporting nodes.
- Graph cycles are allowed only for declared symmetric relation types.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-002: freeze-the-hierarchical-evidence-graph-schema`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-003 [S] — Freeze job DAG, fingerprint, resume, retry, and cancellation schemas

**Depends on:** CR-V2-B3-002  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B3-003: freeze-job-dag-fingerprint-resume-retry-and-cancellation-s`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `schemas/jobs/job.schema.v1.json`
- `schemas/jobs/stage.schema.v1.json`
- `schemas/jobs/fingerprint.schema.v1.json`
- `docs/architecture/V2-JOB-PLANE.md`

**Procedure**

1. Define job and stage lifecycle, dependencies, fingerprints, inputs, outputs, resources, attempts, cancellation, retry class, checkpoints and structured error.
2. Fingerprint source hashes, canonical parameters, capability version, pack locks, schemas and relevant preference state.
3. Define terminal statuses `succeeded`, `needs_review`, `failed`, `cancelled`; never infer success from process exit alone.
4. Specify resume only when every recorded input and output binding still verifies.

**Required implementation shape**

```text
fingerprint = blake3(canonical_json({source_hashes, parameters, capability, pack_locks, schemas, preference_hash}))
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/jobs/job.schema.v1.json fixtures/schemas/jobs/job/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- A changed pack, parameter or source invalidates only dependent stages.
- Cancellation and retry are explicit state transitions.
- A stage with unverifiable output cannot be a cache hit.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-003: freeze-job-dag-fingerprint-resume-retry-and-cancellation-s`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-004 [S] — Freeze resource budgets and degradation policies

**Depends on:** CR-V2-B3-003  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B3-004: freeze-resource-budgets-and-degradation-policies`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `schemas/runtime/resource-budget.schema.v1.json`
- `schemas/runtime/degradation.schema.v1.json`
- `config/runtime/default-budgets.json`
- `docs/architecture/V2-RESOURCE-POLICY.md`

**Procedure**

1. Define CPU threads, accelerator, RAM/VRAM, disk, process count, file descriptors, temp bytes, wall time, output bytes and model context budgets.
2. Define ordered degradation: lower batch/sample density, CPU fallback, smaller selected pack, review escalation or unsupported.
3. Prohibit cloud fallback, silent feature disablement and unbounded retries.
4. Set conservative defaults by target class; benchmark measurements replace them during qualification.

**Required implementation shape**

```text
pub enum Degradation { ReduceBatch, ReduceSampleDensity, CpuFallback, AlternateQualifiedPack(PackId), NeedsReview(ReasonCode), Unsupported(ReasonCode) }
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/runtime/resource-budget.schema.v1.json config/runtime/default-budgets.json
```

**Acceptance — inspect and run only the listed focused checks**

- Every expensive capability declares a resource profile.
- Degradation preserves correctness or stops with a typed reason.
- Retry counts and temp/output caps are finite.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-004: freeze-resource-budgets-and-degradation-policies`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-005 [S] — Freeze pack install, activation, repair, rollback, and offline-only resolution

**Depends on:** CR-V2-B3-004  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B3-005: freeze-pack-install-activation-repair-rollback-and-offline`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/architecture/V2-PACK-LIFECYCLE.md`
- `schemas/runtime/pack-activation.schema.v1.json`
- `schemas/runtime/repair-result.schema.v1.json`

**Procedure**

1. Define staging, signature/hash verification, atomic activation pointer, retained previous pack, repair from installer payload and explicit rollback.
2. Release builds may never repair by internet download or system package manager.
3. Define development-only source override behind a compile-time feature disabled in release.
4. Bind active pack set to project run receipts and benchmark compatibility.

**Required implementation shape**

```text
stage payload → verify manifest/signature/files → write activation record → fsync → atomic active pointer swap → retain previous version
```

**Commands for this task**

```bash
python3 scripts/schema-check.py schemas/runtime/pack-activation.schema.v1.json fixtures/schemas/runtime/pack-activation/v1/valid/basic.json
```

**Acceptance — inspect and run only the listed focused checks**

- An interrupted activation leaves the prior pack active.
- Repair source is a verified local installer payload.
- A project run records the exact active pack locks.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-005: freeze-pack-install-activation-repair-rollback-and-offline`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-006 [S] — Freeze Book 3 pack/evidence/job crate boundaries and lane ownership

**Depends on:** CR-V2-B3-005  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B3-006: freeze-book-3-pack-evidence-job-crate-boundaries-and-lane-`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-3/interface-freeze.md`
- `docs/architecture/V2-RUNTIME-EVIDENCE-JOB-DAG.md`

**Procedure**

1. Assign lane A runtime source/build roots and `crates/video-runtime`; lane B model manifests and `crates/video-inference`; lane C `crates/video-evidence` and `crates/video-jobs`.
2. Reserve workspace integration, doctor, release pack assembly and project orchestration for serial tasks.
3. Freeze capability handshakes for media, speech, director, critic, tracking and TTS components.
4. Freeze pack IDs `media`, `speech`, `speech-quality`, `director`, `vision`, `voice`, `creative`.

**Required implementation shape**

```text
lane_a: runtime/source + video-runtime
lane_b: runtime/models + video-inference
lane_c: video-evidence + video-jobs
```

**Commands for this task**

```bash
python3 scripts/architecture/check_crate_dag.py docs/architecture/V2-RUNTIME-EVIDENCE-JOB-DAG.md
```

**Acceptance — inspect and run only the listed focused checks**

- Parallel roots do not overlap.
- No pack component owns project state.
- Handshakes expose version, capabilities, model IDs and active hashes.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-006: freeze-book-3-pack-evidence-job-crate-boundaries-and-lane-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-007 [P-A] — Build the LGPL-only FFmpeg 8.1 media pack pipeline

**Depends on:** CR-V2-B3-006  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B3-007: build-the-lgpl-only-ffmpeg-8-1-media-pack-pipeline`  
**Stop-loss ceiling:** at most 60 files and 30000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `runtime/source/ffmpeg/**`
- `scripts/runtime/build-ffmpeg.py`
- `runtime/manifests/media.source.json`
- `docs/legal/FFMPEG-BUILD.md`

**Procedure**

1. Import FFmpeg commit `9047fa1b084f76b1b4d065af2d743df1b40dfb56` and verify the signed n8.1 tag provenance.
2. Define target-specific builds without `--enable-gpl` and without `--enable-nonfree`; record full configure output and dependency licences.
3. Build architecture-suffixed `ffmpeg` and `ffprobe` sidecars plus required shared libraries or framework bundle.
4. Generate corresponding-source archive, notices, file hashes and capability probes.

**Required implementation shape**

```text
forbidden = {"--enable-gpl", "--enable-nonfree"}
required_probes = {"ffprobe-json", "h264-decode", "aac-decode", "libass-or-native-caption-path", "zscale-or-qualified-hdr-path"}
```

**Commands for this task**

```bash
python3 scripts/runtime/build-ffmpeg.py --target host --check-config
python3 scripts/legal/build-corresponding-source.py --component ffmpeg --target host
```

**Acceptance — inspect and run only the listed focused checks**

- Forbidden configure flags fail before compilation.
- The pack passes tiny decode/encode/filter/mux probes.
- The source archive and build configuration bind to the binary hashes.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-007: build-the-lgpl-only-ffmpeg-8-1-media-pack-pipeline`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-008 [P-A] — Integrate vendored HeardRight as a CutRight speech component

**Depends on:** CR-V2-B3-007  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B3-008: integrate-vendored-heardright-as-a-cutright-speech-compone`  
**Stop-loss ceiling:** at most 40 files and 12000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-runtime/src/speech.rs`
- `vendor/heardright/cutright-adapter/**`
- `runtime/manifests/speech-engine.source.json`
- `vendor/heardright/PATCHES.md`

**Procedure**

1. Create a CutRight-owned adapter around the vendored engine/core/platform crates.
2. Remove engine discovery, installed-app paths, user model discovery and network fallback.
3. Resolve every model/dictionary/native library through an injected verified `PackResourceResolver`.
4. Retain supervised request correlation, timeouts, bounded stderr, cancellation and capability handshake if a sidecar boundary remains.

**Required implementation shape**

```text
pub trait PackResourceResolver { fn require(&self, pack: PackId, resource: ResourceId) -> Result<VerifiedResource>; }
```

**Commands for this task**

```bash
cargo test -p video-runtime --locked speech_adapter
python3 scripts/gates/v2-runtime-boundary.py --check
```

**Acceptance — inspect and run only the listed focused checks**

- Release code contains no HeardRight install/path environment variables.
- Transcription and VAD use one qualified component session.
- Adapter identity includes exact vendored commit and pack hashes.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-008: integrate-vendored-heardright-as-a-cutright-speech-compone`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-009 [P-A] — Build the Parakeet primary ASR model pack

**Depends on:** CR-V2-B3-008  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B3-009: build-the-parakeet-primary-asr-model-pack`  
**Stop-loss ceiling:** at most 30 files and 6000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `runtime/manifests/parakeet.model.json`
- `scripts/runtime/build-parakeet-pack.py`
- `fixtures/runtime/parakeet/**`
- `docs/legal/PARAKEET-MODEL.md`

**Procedure**

1. Resolve the exact Parakeet model, tokenizer and vocabulary rows from the Book 1 HeardRight asset ledger; abort if any licence or source is unresolved.
2. Copy or deterministically generate target artefacts into staging and hash every byte.
3. Run timed-word fixtures on supported targets and record WER, timing coverage, throughput, cold start and peak memory.
4. Emit a signable speech-pack fragment only after all required target results pass.

**Required implementation shape**

```text
required_assets = ["encoder", "decoder", "joiner", "tokenizer", "vocabulary"]
for asset in required_assets: require_resolved_license_and_sha256(asset)
```

**Commands for this task**

```bash
python3 scripts/runtime/build-parakeet-pack.py --target host --from-ledger imports/v2/heardright-assets.json
cargo test -p video-runtime --locked parakeet_fixtures
```

**Acceptance — inspect and run only the listed focused checks**

- No model byte is fetched from a mutable URL.
- The model identity is exact and reproducible.
- Timed words are native; segment-only output fails qualification.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-009: build-the-parakeet-primary-asr-model-pack`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-010 [P-A] — Build the Silero VAD model pack and parity suite

**Depends on:** CR-V2-B3-009  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B3-010: build-the-silero-vad-model-pack-and-parity-suite`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `runtime/source/silero-vad/**`
- `runtime/manifests/silero-vad.model.json`
- `scripts/runtime/build-silero-pack.py`
- `fixtures/runtime/silero-vad/**`

**Procedure**

1. Import the minimum ONNX/C++ reference subset from Silero commit `76e3dc408eb2a5c655c34e230d2d5459b4439daa` with MIT notice.
2. Select exact ONNX model bytes and freeze their hash and sample-rate contract.
3. Use CutRight audio decode/resample rather than Torch/torchaudio.
4. Run 8 kHz and 16 kHz parity, chunk-boundary, reset, silence/noise and long-file fixtures.

**Required implementation shape**

```text
VadConfig { sample_rate: 16_000, threshold: 0.5, min_speech_ms: 160, min_silence_ms: 180 }
```

**Commands for this task**

```bash
python3 scripts/runtime/build-silero-pack.py --target host
cargo test -p video-runtime --locked silero_parity
```

**Acceptance — inspect and run only the listed focused checks**

- VAD output is deterministic within the frozen tolerance.
- No Python/Torch runtime ships.
- State reset and file/stream parity are proven.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-010: build-the-silero-vad-model-pack-and-parity-suite`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-011 [P-A] — Build whisper.cpp v1.9.2 as an independent speech-quality pack

**Depends on:** CR-V2-B3-010  
**Scheduling:** Parallel lane A. Internally sequential with other lane A tasks; may overlap lanes B and C after task 006.  
**Commit:** `CR-V2-B3-011: build-whisper-cpp-v1-9-2-as-an-independent-speech-quality-`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `runtime/source/whisper.cpp/**`
- `scripts/runtime/build-whisper-pack.py`
- `runtime/manifests/whisper-verifier.model.json`
- `fixtures/runtime/whisper-verifier/**`

**Procedure**

1. Import whisper.cpp commit `306c88f4d1286aec1bf96e544632897886af5501` and its MIT notice.
2. Select one multilingual verifier model through the model licence ledger; abort if the model licence/hash is unresolved.
3. Build target-specific native libraries/binaries and expose bounded transcript/timestamp verification through `video-runtime`.
4. Never promote verifier text to canonical transcript authority; persist disagreement evidence.

**Required implementation shape**

```text
pub struct VerificationResult { pub coverage: f32, pub unmatched_content_rate: f32, pub boundary_deltas: Distribution, pub decision: VerificationDecision }
```

**Commands for this task**

```bash
python3 scripts/runtime/build-whisper-pack.py --target host
cargo test -p video-runtime --locked whisper_verifier
```

**Acceptance — inspect and run only the listed focused checks**

- No Python or WhisperX remains in the shipping verifier path.
- The selected model is content-addressed and audited.
- Provider disagreement produces evidence and policy status rather than destructive overwrite.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-011: build-whisper-cpp-v1-9-2-as-an-independent-speech-quality-`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-012 [P-B] — Vendor and build the pinned llama.cpp inference runtime

**Depends on:** CR-V2-B3-006  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B3-012: vendor-and-build-the-pinned-llama-cpp-inference-runtime`  
**Stop-loss ceiling:** at most 80 files and 24000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `runtime/source/llama.cpp/**`
- `scripts/runtime/build-llama.py`
- `crates/video-inference/Cargo.toml`
- `crates/video-inference/src/runtime.rs`
- `runtime/manifests/llama-runtime.source.json`

**Procedure**

1. Import llama.cpp commit `6a32c29a746a2e44de463de647f9f6661eb5086b` and MIT notice.
2. Build a library or supervised CutRight sidecar per target with network/server features disabled unless required for local IPC tests.
3. Expose model load, structured generation, multimodal sample evaluation, cancellation, token limits, deterministic seed and telemetry-free status.
4. Record backend/accelerator identity and exact build flags.

**Required implementation shape**

```text
pub trait LocalInferenceRuntime { fn load(&self, model: &VerifiedResource, config: LoadConfig) -> Result<ModelHandle>; fn generate_json<T: DeserializeOwned>(&self, handle: &ModelHandle, request: GenerationRequest) -> Result<T>; }
```

**Commands for this task**

```bash
python3 scripts/runtime/build-llama.py --target host
cargo test -p video-inference --locked runtime
```

**Acceptance — inspect and run only the listed focused checks**

- No HTTP server or remote model fetch is required.
- Structured output is byte-bounded and cancellable.
- Runtime handshake reports exact source and binary hashes.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-012: vendor-and-build-the-pinned-llama-cpp-inference-runtime`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-013 [P-B] — Build the Qwen3-4B Director model pack

**Depends on:** CR-V2-B3-012  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B3-013: build-the-qwen3-4b-director-model-pack`  
**Stop-loss ceiling:** at most 30 files and 7000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `runtime/models/director/qwen3-4b.source.json`
- `scripts/runtime/convert-qwen3-director.py`
- `runtime/manifests/director.model.json`
- `fixtures/runtime/director/**`

**Procedure**

1. Fetch only during the authorised pack-build step from official revision `7c69a109fc3fa19c860be9dff46fc23299092018`; the final offline bundle contains all bytes.
2. Verify source files, Apache notice and revision, then convert in-house with the pinned llama.cpp converter.
3. Benchmark candidate quantisations against editorial/tool-use fixtures and select the smallest one meeting the Book 4 floor; do not assume Q4_K_M wins.
4. Freeze tokenizer, chat template, quantisation, context limit, sampling defaults and output hashes.

**Required implementation shape**

```text
selection = min(candidate_quantisations, key=size, subject_to=[schema_validity_floor, editorial_eval_floor, tool_choice_floor, target_memory_floor])
```

**Commands for this task**

```bash
python3 scripts/runtime/convert-qwen3-director.py --source-revision 7c69a109fc3fa19c860be9dff46fc23299092018 --target host
cargo test -p video-inference --locked director_fixtures
```

**Acceptance — inspect and run only the listed focused checks**

- Conversion is reproducible from official source bytes.
- Selected quantisation meets structured-output and tool-choice floors.
- The runtime never downloads the model on end-user launch.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-013: build-the-qwen3-4b-director-model-pack`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-014 [P-B] — Build the Qwen3-VL-4B independent visual critic pack

**Depends on:** CR-V2-B3-013  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B3-014: build-the-qwen3-vl-4b-independent-visual-critic-pack`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `runtime/models/critic/qwen3-vl-4b.source.json`
- `scripts/runtime/convert-qwen3-vl-critic.py`
- `runtime/manifests/vision-critic.model.json`
- `fixtures/runtime/vision-critic/**`

**Procedure**

1. Use official revision `ebb281ec70b05090aa6165b016eac8ec08e71b17` and Apache notice.
2. Convert model and multimodal projector in-house with the pinned runtime; freeze image/video sampling and maximum evidence window.
3. Evaluate layout collision, identity/label preservation, crop stability, visual instruction and temporal-order cases.
4. Keep critic process/prompt/seed separate from the Director.

**Required implementation shape**

```text
pub struct CriticVerdict { pub verdict: Verdict, pub findings: Vec<CriticFinding>, pub confidence: f32, pub evidence_refs: Vec<EvidenceRef>, pub requires_human: bool }
```

**Commands for this task**

```bash
python3 scripts/runtime/convert-qwen3-vl-critic.py --source-revision ebb281ec70b05090aa6165b016eac8ec08e71b17 --target host
cargo test -p video-inference --locked critic_fixtures
```

**Acceptance — inspect and run only the listed focused checks**

- The critic has no mutation capability.
- Every verdict includes exact evidence IDs and time/frame ranges.
- Pack ships only on targets meeting memory and latency floors.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-014: build-the-qwen3-vl-4b-independent-visual-critic-pack`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-015 [P-B] — Qualify Qwen3.5-4B without making it a release dependency

**Depends on:** CR-V2-B3-014  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B3-015: qualify-qwen3-5-4b-without-making-it-a-release-dependency`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `runtime/candidates/qwen3.5-4b/qualification.json`
- `scripts/runtime/qualify-qwen35.py`
- `docs/benchmarks/QWEN35-QUALIFICATION.md`

**Procedure**

1. Use official source revision `851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a` and preserve its Apache notice.
2. Run the same Director and critic fixtures, all target runtime tests, memory/latency measurements and deterministic structured-output checks.
3. Compare against selected Qwen3/Qwen3-VL packs with blind result IDs.
4. Leave disposition `qualification_candidate` unless it passes every required target and a separate transition decision updates the pack matrix.

**Required implementation shape**

```text
assert qualification.mode == "no_promote"
assert active_pack_lock_unchanged()
```

**Commands for this task**

```bash
python3 scripts/runtime/qualify-qwen35.py --source-revision 851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a --target host --no-promote
```

**Acceptance — inspect and run only the listed focused checks**

- The task cannot modify active release pack manifests.
- Results include failures and unsupported runtime features.
- No automatic promotion occurs.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-015: qualify-qwen3-5-4b-without-making-it-a-release-dependency`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-016 [P-B] — Build the Kokoro local TTS and phonemizer pack

**Depends on:** CR-V2-B3-015  
**Scheduling:** Parallel lane B. Internally sequential with other lane B tasks; may overlap lanes A and C after task 006.  
**Commit:** `CR-V2-B3-016: build-the-kokoro-local-tts-and-phonemizer-pack`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `runtime/models/voice/kokoro-82m.source.json`
- `scripts/runtime/build-kokoro-pack.py`
- `runtime/manifests/voice.model.json`
- `fixtures/runtime/voice/**`
- `docs/legal/VOICE-ASSET-LEDGER.md`

**Procedure**

1. Use Kokoro v1.0 model hash `496dba118d1a58f5f3db2efc88dbdc216e0483fc89fe6e47ee1f2c53f18ad1e4` and Apache notice.
2. Select a native ONNX or equivalent runtime and bundle all required phonemizer data; no Python/espeak system dependency.
3. Audit every voice file separately and exclude unresolved voices.
4. Run pronunciation, determinism, duration, clipping, silence and cross-platform waveform checks.

**Required implementation shape**

```text
voice_pack.files = [model, tokenizer_or_config, phonemizer_data, audited_voice_files]
assert all(file.license_resolved for file in voice_pack.files)
```

**Commands for this task**

```bash
python3 scripts/runtime/build-kokoro-pack.py --target host
cargo test -p video-inference --locked tts_fixtures
```

**Acceptance — inspect and run only the listed focused checks**

- The exact model hash matches.
- No unresolved voice is copied.
- TTS works with network blocked and empty PATH.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-016: build-the-kokoro-local-tts-and-phonemizer-pack`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-017 [P-C] — Implement deterministic scene and shot segmentation evidence

**Depends on:** CR-V2-B3-006  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B3-017: implement-deterministic-scene-and-shot-segmentation-eviden`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-evidence/Cargo.toml`
- `crates/video-evidence/src/lib.rs`
- `crates/video-evidence/src/scene.rs`
- `crates/video-evidence/src/shot.rs`
- `crates/video-evidence/tests/scene_shot.rs`

**Procedure**

1. Use decoded frame statistics, histogram/feature deltas, keyframes and motion evidence to propose scene/shot boundaries.
2. Record algorithm parameters, confidence and source frame evidence.
3. Make sampling adaptive: coarse pass first, then refine only around candidate boundaries.
4. Keep semantic scene labels separate from deterministic boundaries.

**Required implementation shape**

```text
coarse candidates → local high-rate refinement → minimum-duration merge policy → EvidenceNode(Scene|Shot)
```

**Commands for this task**

```bash
cargo test -p video-evidence --locked scene_shot
```

**Acceptance — inspect and run only the listed focused checks**

- Boundary fixtures are deterministic.
- High-frame-rate fast-cut fixtures trigger denser refinement.
- Every boundary traces to source frames.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-017: implement-deterministic-scene-and-shot-segmentation-eviden`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-018 [P-C] — Implement face, pose, gesture, text, saliency, and motion tracks

**Depends on:** CR-V2-B3-017  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B3-018: implement-face-pose-gesture-text-saliency-and-motion-track`  
**Stop-loss ceiling:** at most 30 files and 6000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-evidence/src/vision.rs`
- `crates/video-evidence/src/tracks/**`
- `crates/video-evidence/tests/vision_tracks.rs`
- `runtime/manifests/vision-tracking.model.json`

**Procedure**

1. Resolve a qualified local tracker through the vision pack; if MediaPipe qualification failed, use the frozen fallback named in the registry.
2. Track stable subject IDs over time with confidence and re-identification evidence.
3. Generate face/pose/hand/gesture/text-region/saliency/global-motion/camera-motion tracks at adaptive sample rates.
4. Disable any telemetry/network path and prove blocked-network operation.

**Required implementation shape**

```text
pub struct TemporalTrack<T> { pub track_id: TrackId, pub samples: Vec<TimedSample<T>>, pub confidence: f32, pub gaps: Vec<RationalRange> }
```

**Commands for this task**

```bash
cargo test -p video-evidence --locked vision_tracks
python3 scripts/runtime/assert_no_network.py --component vision-tracking
```

**Acceptance — inspect and run only the listed focused checks**

- Tracks use source time and stable IDs.
- Subject loss and re-identification uncertainty are explicit.
- No outbound network attempt occurs.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-018: implement-face-pose-gesture-text-saliency-and-motion-track`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-019 [P-C] — Implement the hierarchical evidence graph store

**Depends on:** CR-V2-B3-018  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B3-019: implement-the-hierarchical-evidence-graph-store`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-evidence/src/graph.rs`
- `crates/video-evidence/src/store.rs`
- `crates/video-evidence/src/index.rs`
- `crates/video-evidence/tests/graph.rs`

**Procedure**

1. Persist immutable graph segments as content-addressed canonical JSON/CBOR objects plus a rebuildable local index.
2. Validate node/edge source revision, time ranges, producer identity, receipts and relation constraints.
3. Support append of new derived layers without rewriting prior evidence.
4. Make the index disposable and rebuildable from canonical graph objects.

**Required implementation shape**

```text
canonical: project/evidence/objects/<hash>.json
index: app data/evidence-index.sqlite (rebuildable)
```

**Commands for this task**

```bash
cargo test -p video-evidence --locked graph
```

**Acceptance — inspect and run only the listed focused checks**

- Canonical graph survives index deletion/rebuild.
- Tampered node/receipt fails read verification.
- No mutable database row is canonical truth.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-019: implement-the-hierarchical-evidence-graph-store`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-020 [P-C] — Implement bounded evidence retrieval and coarse-to-fine query planning

**Depends on:** CR-V2-B3-019  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B3-020: implement-bounded-evidence-retrieval-and-coarse-to-fine-qu`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-evidence/src/query.rs`
- `crates/video-evidence/src/retrieve.rs`
- `crates/video-evidence/tests/retrieval.rs`

**Procedure**

1. Expose typed queries by source, timeline, time window, node kind, subject, speaker, text, confidence and relation.
2. Return compact summaries first and exact high-resolution nodes/frames only within explicit budgets.
3. Implement planner-requested refinement around uncertain or information-dense spans.
4. Record every retrieval query and returned evidence hash in the run receipt.

**Required implementation shape**

```text
pub struct EvidenceQuery { pub scope: EvidenceScope, pub kinds: BTreeSet<EvidenceKind>, pub window: Option<RationalRange>, pub budget: RetrievalBudget, pub refine: bool }
```

**Commands for this task**

```bash
cargo test -p video-evidence --locked retrieval
```

**Acceptance — inspect and run only the listed focused checks**

- A whole-project query is bounded and paginated.
- Refinement returns only descendants/overlaps of selected spans.
- Same query and graph revision return identical ordered IDs.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-020: implement-bounded-evidence-retrieval-and-coarse-to-fine-qu`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-021 [P-C] — Implement the content-addressed job DAG, cache, retry, resume, and cancellation

**Depends on:** CR-V2-B3-020  
**Scheduling:** Parallel lane C. Internally sequential with other lane C tasks; may overlap lanes A and B after task 006.  
**Commit:** `CR-V2-B3-021: implement-the-content-addressed-job-dag-cache-retry-resume`  
**Stop-loss ceiling:** at most 14 files and 2600 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-jobs/Cargo.toml`
- `crates/video-jobs/src/lib.rs`
- `crates/video-jobs/src/dag.rs`
- `crates/video-jobs/src/store.rs`
- `crates/video-jobs/src/runner.rs`
- `crates/video-jobs/tests/recovery.rs`

**Procedure**

1. Create persistent jobs/stages using frozen fingerprints and atomic state records.
2. Schedule only dependency-ready stages within resource budgets.
3. Verify cache receipts before hit, classify retryable versus permanent errors, and persist bounded attempt history.
4. Implement cancellation propagation and resume from the last verified stage after crash.

**Required implementation shape**

```text
Ready if dependencies.succeeded && resources.available; cache hit only if fingerprint matches && receipt.verify_all();
```

**Commands for this task**

```bash
cargo test -p video-jobs --locked recovery
```

**Acceptance — inspect and run only the listed focused checks**

- Crash injection at every state transition resumes without duplicate outputs.
- Changed input invalidates downstream stages only.
- Cancellation leaves completed verified stages reusable.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-021: implement-the-content-addressed-job-dag-cache-retry-resume`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-022 [S] — Merge Book 3 lanes and create the runtime/evidence/job service façade

**Depends on:** CR-V2-B3-011, CR-V2-B3-016, CR-V2-B3-021  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B3-022: merge-book-3-lanes-and-create-the-runtime-evidence-job-ser`  
**Stop-loss ceiling:** at most 10 files and 1800 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-services/Cargo.toml`
- `crates/video-services/src/lib.rs`
- `crates/video-services/src/runtime.rs`
- `crates/video-services/src/evidence.rs`
- `crates/video-services/src/jobs.rs`
- `Cargo.toml`
- `docs/dispatch/v2/book-3/merge-receipt.md`

**Procedure**

1. Apply lane A, B and C commits in deterministic order.
2. Add crates to the root workspace and expose verified pack, evidence query and job-submission services.
3. Keep project orchestration above these services and prevent direct UI access to runtime file paths.
4. Record merge commits/conflicts.

**Required implementation shape**

```text
pub struct VideoServices { pub packs: PackService, pub evidence: EvidenceService, pub jobs: JobService, pub inference: InferenceService }
```

**Commands for this task**

```bash
cargo check -p video-services --locked
python3 scripts/architecture/check_crate_dag.py docs/architecture/V2-RUNTIME-EVIDENCE-JOB-DAG.md
```

**Acceptance — inspect and run only the listed focused checks**

- Service façade returns stable IDs/capabilities, not raw mutable handles.
- No dependency cycle exists.
- Merge receipt is complete.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-022: merge-book-3-lanes-and-create-the-runtime-evidence-job-ser`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-023 [S] — Implement pack doctor, verification, and local repair commands

**Depends on:** CR-V2-B3-022  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B3-023: implement-pack-doctor-verification-and-local-repair-comman`  
**Stop-loss ceiling:** at most 8 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-cli/src/pack_commands.rs`
- `crates/video-runtime/src/doctor.rs`
- `crates/video-runtime/src/repair.rs`
- `crates/video-runtime/tests/doctor.rs`

**Procedure**

1. Add `videoctl packs list|verify|activate|rollback|repair|doctor` using the shared pack service.
2. Repair only from a provided verified offline payload path.
3. Report missing, corrupt, incompatible, unsupported and unqualified distinctly.
4. Write a receipt when requested.

**Required implementation shape**

```text
videoctl packs repair --payload /path/to/CutRight-Offline-Payload --pack speech --target host
```

**Commands for this task**

```bash
cargo test -p video-runtime -p videoctl --locked doctor
```

**Acceptance — inspect and run only the listed focused checks**

- No command accesses network or PATH.
- Corrupt file and signature fixtures fail.
- Rollback restores the prior active lock atomically.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-023: implement-pack-doctor-verification-and-local-repair-comman`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-024 [S] — Replace every release runtime lookup with signed pack resolution

**Depends on:** CR-V2-B3-023  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B3-024: replace-every-release-runtime-lookup-with-signed-pack-reso`  
**Stop-loss ceiling:** at most 16 files and 2600 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `crates/video-media/src/toolchain.rs`
- `crates/video-providers/src/lib.rs`
- `crates/video-providers/src/heardright.rs`
- `crates/video-providers/src/whisperx.rs`
- `apps/studio/src-tauri/tauri.conf.json`
- `scripts/gates/v2-runtime-boundary.py`

**Procedure**

1. Thread `PackResourceResolver` into media, speech, inference, tracking and TTS paths.
2. Delete release discovery of system FFmpeg, HeardRight app/engine, Python/WhisperX, Node, browser or model directories.
3. Keep development override only behind `cfg(feature = "dev-runtime-override")`; release builds omit it.
4. Bundle target-specific sidecars through Tauri external binary/resources.

**Required implementation shape**

```text
let ffmpeg = resolver.require(PackId::MEDIA, ResourceId::FFMPEG)?;
ProcessSpec::new(ffmpeg.verified_path())
```

**Commands for this task**

```bash
cargo test --workspace --locked runtime_resolution
python3 scripts/gates/v2-runtime-boundary.py --check
```

**Acceptance — inspect and run only the listed focused checks**

- Release build has zero bare executable resolution.
- Empty PATH fixtures pass with installed packs.
- Development override is absent from default/release feature graph.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-024: replace-every-release-runtime-lookup-with-signed-pack-reso`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-025 [S] — Run a network-blocked clean-path runtime smoke test

**Depends on:** CR-V2-B3-024  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B3-025: run-a-network-blocked-clean-path-runtime-smoke-test`  
**Stop-loss ceiling:** at most 20 files and 2000 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `tests/v2/clean_runtime.rs`
- `scripts/qa/v2-clean-runtime.sh`
- `fixtures/runtime/clean-smoke/**`
- `docs/dispatch/v2/book-3/clean-runtime.md`

**Procedure**

1. Launch with temporary HOME, empty PATH, blocked outbound network and only staged application/packs.
2. Probe media, transcribe, run VAD, run verifier, load Director, load critic, synthesise TTS, build basic scene/face evidence and execute a tiny cached job twice.
3. Assert second run uses verified cache and no component attempts repair/download.
4. Capture process/network/file evidence.

**Required implementation shape**

```text
env -i HOME="$TMP/home" PATH="" CUTRIGHT_PACK_ROOT="$TMP/packs" ./cutright-clean-runtime-harness
```

**Commands for this task**

```bash
bash scripts/qa/v2-clean-runtime.sh
```

**Acceptance — inspect and run only the listed focused checks**

- Every required component succeeds.
- Network attempt count is zero.
- Second run shows expected cache hits and identical hashes.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-025: run-a-network-blocked-clean-path-runtime-smoke-test`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-026 [S] — Run focused pack, evidence, and job recovery tests

**Depends on:** CR-V2-B3-025  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B3-026: run-focused-pack-evidence-and-job-recovery-tests`  
**Stop-loss ceiling:** at most 1 file and 1200 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-3/focused-tests.md`

**Procedure**

1. Run pack schema/signature/repair, runtime component fixtures, evidence graph/retrieval, job fingerprint/cache/crash/cancellation and clean runtime suites.
2. Record target hardware, active packs, file hashes, peak memory and skipped unsupported accelerators.
3. Do not run the full repository gate in this task.
4. Fix required failures; unsupported targets remain explicit.

**Required implementation shape**

```text
required host status: pass
unsupported accelerator status: unsupported_with_reason
unavailable optional scanner: unproven
```

**Commands for this task**

```bash
cargo test -p video-runtime -p video-inference -p video-evidence -p video-jobs -p video-services --locked
bash scripts/qa/v2-clean-runtime.sh
```

**Acceptance — inspect and run only the listed focused checks**

- Required host suites pass.
- No unrun accelerator is reported as pass.
- Evidence binds exact pack locks and target.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-026: run-focused-pack-evidence-and-job-recovery-tests`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.

## CR-V2-B3-027 [S] — Run the authoritative Book 3 local gate and freeze pack/evidence/job evidence

**Depends on:** CR-V2-B3-026  
**Scheduling:** Sequential. Run on the Book integration line after all dependencies are committed.  
**Commit:** `CR-V2-B3-027: run-the-authoritative-book-3-local-gate-and-freeze-pack-ev`  
**Stop-loss ceiling:** at most 2 files and 1400 changed lines. Stop and report the exact excess before crossing either ceiling.

**Exclusive file ownership**

- `docs/dispatch/v2/book-3/final-gate.md`
- `docs/dispatch/v2/book-3/final-manifest.json`

**Procedure**

1. Run runtime-boundary, pack-lock, corresponding-source, clean-runtime and focused tests.
2. Run the authoritative local gate exactly once.
3. Record hashes and measurements without signing a public release pack.
4. Do not create CI or upload.

**Required implementation shape**

```text
book: 3
network_attempts: 0
path_fallbacks: 0
ci: forbidden
```

**Commands for this task**

```bash
python3 scripts/gates/v2-runtime-boundary.py --check
python3 scripts/legal/validate-v2-ledger.py --scope book-3
bash scripts/qa/v2-clean-runtime.sh
bash scripts/gate.sh --with-qa
```

**Acceptance — inspect and run only the listed focused checks**

- All required checks pass.
- No unresolved materialized runtime asset remains.
- Final manifest binds commit, pack locks and test evidence.

**Close the task**

1. Run `git diff --check`.
2. Confirm no file outside the exclusive ownership list changed, except a lockfile that the listed command deterministically updates.
3. Commit with exactly `CR-V2-B3-027: run-the-authoritative-book-3-local-gate-and-freeze-pack-ev`.
4. Do not run the book/root gate, broad PPM suite, or release build unless this is task 027. Do not create GitHub Actions, hosted checks, or upload artefacts.
