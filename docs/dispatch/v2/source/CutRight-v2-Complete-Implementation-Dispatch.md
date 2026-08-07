# CutRight v2 complete implementation dispatch

This file combines the v2 package documents and all seven dispatch books. Individual files remain authoritative for execution and checksums.


---

# CutRight v2 source corpus and disposition ledger

## 1. Exhaustiveness claim

This document does not claim to cover every repository, paper, tool, or model that could ever be relevant. It defines the complete reproducible corpus used for the v2 decision as of **2026-08-06**. The import compiler must prove that every reference reachable from an included skill, tool, model, asset, schema, or script has one terminal disposition:

- `ship_source`
- `ship_runtime_pack`
- `adapt_with_notice`
- `clean_room_behavior`
- `provenance_only`
- `development_only`
- `excluded_with_reason`
- `blocked_unresolved`

`blocked_unresolved` is not shippable. Missing entries, dangling relative links, symlinks outside CutRight, submodules, and undeclared binary/model files fail the Book 1 gate.

## 2. Pinned source corpus

| Source | Repository / artefact | Pinned revision | Licence posture | Disposition | Use |
| --- | --- | --- | --- | --- | --- |
| CutRight | Orthic-Labs/CutRight | 7f3e5a61c729d4d877715b9a083d13a2e5ebe277 | MIT | shipping base | All current source; current external-provider assumptions are replaced by v2. |
| Workspace capabilities | bogusyogi/claude | 6ee21f03a787e7b57dc412760a8996ea7a235302 | user-owned / per-file third-party notices | vendor selected closure | Designer plus Brand, Brand Identity, Content, Writing, Social, QA, bounded-run, eval topology, and selected local assets. |
| HeardRight | bogusyogi/heardright | b60bff947f12ffa9d25e94ad27e8ff30db006a24 | private user-owned; third-party model notices mandatory | vendor and adapt | Copy the engine/core/platform source and only redistributable model assets into first-party CutRight packs. |
| AutoShorts | JayWebtech/autoshorts | f17b04cdd97ef65c32b81b31b36bb6eb5d013d5b | no declared repository license | behavior only | Reproduce project-library, onboarding, one-click-run, candidate-card, and progress behavior without copying source. |
| Vox Director | Alisa0808/vox-director | 8b034354dc443edcde7fdb2622e0491df5142fd3 | MIT | adapt with notice | Absorb beat/shot structure, style bake-offs, A/B/C-roll, anti-monotony, and bounded async job semantics. |
| Palmier Pro | palmier-io/palmier-pro | 397b82e64093f986cbabd89f1a1c93812ff546c2 | GPL-3.0 | clean-room behavior only | Reimplement typed editing actions, stable IDs, composited inspection, undo, variants, jobs, and skill catalog without copying Swift. |
| llama.cpp | ggml-org/llama.cpp | 6a32c29a746a2e44de463de647f9f6661eb5086b | MIT | vendor runtime source | Pinned local inference runtime; CutRight builds its own platform binaries. |
| whisper.cpp | ggml-org/whisper.cpp | 306c88f4d1286aec1bf96e544632897886af5501 | MIT | vendor verifier source | Independent transcript/edge verifier; not transcript authority. |
| Silero VAD | snakers4/silero-vad | 76e3dc408eb2a5c655c34e230d2d5459b4439daa | MIT | vendor model/runtime subset | Use exact ONNX model bytes with generated SHA-256 and no network fallback. |
| MediaPipe | google-ai-edge/mediapipe | f8ef212d5c962c0e853db7e59d217056b187084b | Apache-2.0 | conditional source component | May provide face/pose tracking only after telemetry-disabled, network-blocked qualification and per-model license closure. |
| FFmpeg | FFmpeg/FFmpeg n8.1 | 9047fa1b084f76b1b4d065af2d743df1b40dfb56 | LGPL-2.1-or-later build only | vendor build + corresponding source | No GPL/nonfree flags; distribute configure line, notices, and corresponding source. |
| Qwen3-4B | Qwen/Qwen3-4B | 7c69a109fc3fa19c860be9dff46fc23299092018 | Apache-2.0 | director candidate selected | Convert official safetensors to CutRight-owned GGUF; exact output hashes frozen by pack builder. |
| Qwen3-VL-4B-Instruct | Qwen/Qwen3-VL-4B-Instruct | ebb281ec70b05090aa6165b016eac8ec08e71b17 | Apache-2.0 | independent critic selected | Convert official model and multimodal projector; ship only after all-target qualification. |
| Qwen3.5-4B | Qwen/Qwen3.5-4B | 851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a | Apache-2.0 | qualification candidate only | Not a shipping dependency until deterministic local runtime support passes the full matrix. |
| Kokoro-82M v1.0 | hexgrad/Kokoro-82M | 496dba118d1a58f5f3db2efc88dbdc216e0483fc89fe6e47ee1f2c53f18ad1e4 | Apache-2.0 weights; voices separately audited | TTS selected | Model hash is fixed; every voice file must have its own provenance and redistribution entry. |
| Attached Cutaway/Finish material | conversation attachments | materialized hash manifest generated in Book 1 | user supplied; third-party dependencies audited separately | vendor as provenance and migrate | Keep scripts as golden behavior; migrate live execution to typed CutRight stages. |

## 3. Relevant workspace skill closure

The closure is computed from the pinned `bogusyogi/claude` tree, not from a hand-written list. These are the expected roots and the required disposition:

| Root | Disposition | Included capability | Excluded material |
| --- | --- | --- | --- |
| `designer/` | ship_source + adapt | Complete Designer engine, agents, Huashu references/assets/scripts, visual critique, audio design, cinematic patterns, style and scene systems. | PPTX/deck-only branches may be excluded only by an explicit ledger row; no silent omission. |
| `brand/` | ship_source + adapt | Brand Cards, locked visual/voice restrictions, Right Suite identity rules, motion restrictions. | Brands unrelated to the project remain optional data packs, not runtime code. |
| `brand-identity/` | ship_source + adapt | Identity creation/evolution, signature mechanism, tokens, accessibility and reproduction checks. | Registry mutation outside a CutRight project. |
| `content/` | selected transitive closure | Video-editor, production routing, Remotion rules as migration evidence, transcription, motion-graphics, enhancement, avatar/anchored modes, local evals. | KDP and carousel production unless a later CutRight feature explicitly needs them. |
| `writing/` | selected transitive closure | Script, editorial, content repurposing, copywriting hooks, proof/anti-slop rules, titles/descriptions/captions. | Email, blogs, changelogs, profiles, and unrelated prose lanes. |
| `social/` | selected transitive closure | YouTube, Reels/Instagram, Shorts, platform constraints, packaging and measurement definitions. | Posting, scheduling, account mutation, and network connectors. |
| `qa/` | ship_source + adapt | Deterministic Tauri/local QA, functional assertions, visual captures, contract-test patterns and evals. | Browser downloads and network-dependent fixtures. |
| `research/` and `tools/research-core/` | development_only + optional local-source skill | Source ledger and evidence discipline for explainer projects using user-provided documents. | Open-web retrieval is not a required runtime dependency. |
| `ads/`, `marketing/`, `seo/` | excluded_with_reason | No core editing capability needed for v2. | May be revisited as optional publishing packs. |
| `architect/`, `coder/`, `commit/`, `debugger/`, `dispatch/`, `tasklist/`, `jfdi/` | development_only | Useful to build CutRight, not user-facing video-production skills. | Never bundled as product agent capabilities. |

## 4. Relevant workspace tool closure

| Tool | Disposition | Absorb | Do not absorb |
| --- | --- | --- | --- |
| `tools/bounded-run` | adapt_with_notice | Skill compiler, schemas, monitor concepts, state migration, acceptance fixtures. | Workspace-global state or external skill locations. |
| `tools/evals` | adapt_with_notice | Catalog integrity, topology validation, fixtures, judges, deterministic evaluation entry points. | Research-only assumptions and unrelated skill cases. |
| `tools/gauntlet` | adapt_with_notice | Changed-line mutation testing, changed-line coverage, test-order randomisation as an optional local hardening lane. | Hosted CI integration. |
| `tools/hyperframes` | clean_room_behavior / provenance_only | Deterministic declarative timelines, seekability, local validation concepts. | External package, external skill installer, publishing service, or runtime dependency. |
| `tools/remotion` and CutRight `apps/effects` | provenance_only then retire shipping path | Existing effect schemas, previews, fixtures, timing semantics, and visual targets. | Remotion runtime, Node, Chromium, or a commercial licence dependency in the shipped product. |
| `tools/rightkit` | development_only | Local release discipline, signing and manifest ideas. | A runtime dependency or hosted release automation. |
| `tools/mcp` | clean_room_behavior | Shared typed executor and optional loopback server pattern. | A separate tool implementation with divergent semantics. |

## 5. Research corpus and architectural consequence

| Work | Primary source | Venue/date | Supported finding | CutRight consequence |
| --- | --- | --- | --- | --- |
| VideoLLaMB | https://openaccess.thecvf.com/content/ICCV2025/html/Wang_VideoLLaMB_Long_Streaming_Video_Understanding_with_Recurrent_Memory_Bridges_ICCV_2025_paper.html | ICCV 2025 | Scene tiling and recurrent memory bridges for long streaming video. | Hierarchical scene memory, not whole-video prompt stuffing. |
| Flash-VStream | https://openaccess.thecvf.com/content/ICCV2025/html/Zhang_Flash-VStream_Efficient_Real-Time_Understanding_for_Long_Video_Streams_ICCV_2025_paper.html | ICCV 2025 | Compact context memory plus detail memory retrieved by information density. | Two-tier evidence store and selective frame retrieval. |
| ProVideLLM | https://openaccess.thecvf.com/content/ICCV2025/html/Chatterjee_Streaming_VideoLLMs_for_Real-Time_Procedural_Video_Understanding_ICCV_2025_paper.html | ICCV 2025 | Compressed long-term text with detailed short-term visual tokens. | Separate semantic summaries from high-resolution evidence. |
| LVAgent | https://openaccess.thecvf.com/content/ICCV2025/html/Chen_LVAgent_Long_Video_Understanding_by_Multi-Round_Dynamical_Collaboration_of_MLLM_ICCV_2025_paper.html | ICCV 2025 | Selection, retrieval/perception, action, and reflection in multiple rounds. | Agent loop must retrieve, act, inspect, and revise. |
| SALOVA | https://openaccess.thecvf.com/content/CVPR2025/html/Kim_SALOVA_Segment-Augmented_Long_Video_Assistant_for_Targeted_Retrieval_and_Routing_CVPR_2025_paper.html | CVPR 2025 | Segment-level retrieval and dynamic routing improve long-form contextual relevance. | Queries operate over indexed segments and retrieve bounded detail. |
| Adaptive Keyframe Sampling | https://openaccess.thecvf.com/content/CVPR2025/html/Tang_Adaptive_Keyframe_Sampling_for_Long_Video_Understanding_CVPR_2025_paper.html | CVPR 2025 | Relevant and coverage-aware keyframe selection outperforms uniform sampling. | Evidence sampling must balance query relevance and coverage. |
| LongVALE | https://openaccess.thecvf.com/content/CVPR2025/html/Geng_LongVALE_Vision-Audio-Language-Event_Benchmark_Towards_Time-Aware_Omni-Modal_Perception_of_Long_Videos_CVPR_2025_paper.html | CVPR 2025 | Fine-grained audio-visual-language events and temporal boundaries. | Evidence graph has explicit audio/visual events and boundaries. |
| VidHalluc | https://openaccess.thecvf.com/content/CVPR2025/html/Li_VidHalluc_Evaluating_Temporal_Hallucinations_in_Multimodal_Large_Language_Models_for_CVPR_2025_paper.html | CVPR 2025 | Temporal hallucinations occur in actions, sequences, and scene transitions. | Truthfulness and chronology checks are first-class. |
| AVE-Compass | https://arxiv.org/abs/2607.24821 | 2026 preprint | Checklist-based audio-visual editing evaluation and iterative evaluator feedback. | Instruction, preservation, realism, edit-intent, and critic-revision axes. |
| SpeechEditBench | https://arxiv.org/abs/2606.01804 | 2026 preprint | Target success, preservation success, and joint success. | Every edit is scored for requested change and untouched-content preservation. |
| UniEditBench | https://arxiv.org/abs/2604.15871 | 2026 preprint | Structural fidelity, background consistency, naturalness, and temporal-spatial consistency. | Independent visual critic and multi-dimensional edit scoring. |
| FiVE | https://arxiv.org/abs/2503.13684 | 2025 preprint | Fine-grained editing requires background preservation, temporal consistency, quality, and runtime metrics. | Object-level edit success is separated from non-target preservation. |
| V2V-Bench | https://arxiv.org/abs/2606.05665 | 2026 preprint | Video-to-video evaluation needs temporal alignment, structural fidelity, transformation quality, visual quality, and semantic alignment. | Release reports separate these dimensions instead of one aggregate score. |
| AVID | https://arxiv.org/abs/2604.13593 | 2026 preprint | Audio-visual inconsistency requires temporal grounding and conflict classification. | Critic tests include active-speaker, voiceover, scenic, and cross-modal conflict cases. |
| TARO | https://openaccess.thecvf.com/content/ICCV2025/html/Ton_TARO_Timestep-Adaptive_Representation_Alignment_with_Onset-Aware_Conditioning_for_Synchronized_Video-to-Audio_ICCV_2025_paper.html | ICCV 2025 | Onset-aware conditioning improves event-level audio-visual synchronization. | Transient alignment is evaluated at event/onset level. |
| MMAudio | https://openaccess.thecvf.com/content/CVPR2025/html/Cheng_MMAudio_Taming_Multimodal_Joint_Training_for_High-Quality_Video-to-Audio_Synthesis_CVPR_2025_paper.html | CVPR 2025 | Frame-level conditioning improves audio-visual synchronization. | Audio generation and SFX placement use frame/event alignment evidence. |

## 6. Licence and provenance rules

1. Every source file keeps its original notice where required.
2. Every copied subtree receives `THIRD_PARTY.yml` with source, revision, files, licence, modifications, and owner.
3. Behaviour-only sources receive a clean-room note containing observed public behaviour, implementer separation, and a no-copy attestation.
4. Every model, voice, font, LUT, texture, SFX, music file, and sample project has a separate entry; a repository-level licence never automatically covers every asset.
5. FFmpeg is built without `--enable-gpl` and without `--enable-nonfree`; the installer carries the exact configure line and corresponding source.
6. Remotion and HyperFrames are not included in a shipping runtime pack. Migration tests compare native outputs to retained visual fixtures.
7. No pack is signed while any reachable ledger row is `blocked_unresolved`.

## 7. Closure compiler algorithm

```text
seed included roots
→ parse Markdown links, script imports, package manifests, include_str!, assets and model manifests
→ canonicalise each target inside the pinned source snapshot
→ reject path escape, symlink escape, submodule and mutable branch references
→ require one disposition for every node
→ copy permitted nodes into staging
→ rewrite references to CutRight-local paths
→ run topology and licence validation
→ hash every staged byte
→ emit import receipt and immutable manifest
```

The shipping application never reads this source corpus. It reads only CutRight-owned skills, schemas, binaries, models, assets, and pack manifests produced from it.


---

# CutRight v2 product architecture

## 1. Product definition

CutRight is a self-contained, local-first desktop system that turns recordings, existing finished media, scripts, topics, people, and product assets into verified long-form and short-form deliverables. It combines a deterministic non-destructive editor with an embedded editorial and creative operating system.

The default user action is **Make versions**. CutRight ingests the sources, builds evidence, proposes an editorial story, renders natural and tight variants, creates requested platform versions, applies the selected brand/design/motion language, runs independent QA, and presents ready outputs with exact reasons and evidence. Low-confidence decisions stop at the named review surface; they do not silently guess.

## 2. Supported production lanes

1. **Recorded footage:** talking head, interview, podcast, screen recording, tutorial, multi-take, or multi-camera sources become a long-form edit and platform variants.
2. **Repurpose:** an existing programme becomes ranked, self-contained short clips with native captions, reframing, copy, and thumbnails.
3. **Explainer:** a user-provided brief, script, and local source package become narration, beats, shots, graphics, motion, and final media.
4. **Anchored creative:** a real presenter, face, product, label, logo, or source video remains identity-locked while the surrounding creative system changes.

## 3. Five systems

### 3.1 Media Kernel

Rust owns canonical project state, stable IDs, rational time, source hashes, revisions, timeline transactions, validation, undo, rendering, migrations, receipts, and package integrity. Models and skills cannot write project files directly. They submit typed actions to the kernel.

### 3.2 Evidence and Job Plane

The evidence graph is the canonical derived understanding of the project:

```text
source → scene → shot → event → frame
video event ↔ face/subject/gesture/text/saliency/motion tracks
audio → speaker turn → utterance → word/phoneme boundary
music → section → bar → beat → transient
editorial beat ↔ claims ↔ source ranges ↔ supporting assets
```

Every node carries source identity, exact time range, confidence, producer identity, parameter hash, and receipt. Compact summaries route retrieval; they never replace the underlying evidence. The agent requests windows and nodes instead of repeatedly scanning whole videos.

The same plane owns the content-addressed job DAG, stage fingerprints, resource budgets, concurrency, cancellation, retry, resume, pack acquisition from installer payload, stale-result rejection, and crash recovery.

### 3.3 Embedded Creative Operating System

The product-local skill runtime contains CutRight Director, Editorial Director, Video/Beat Director, Finish Director, Designer, Brand, Brand Identity, Content Production, Writing/Packaging, Social Constraints, Functional QA, and Visual QA. Skills are immutable versioned resources with schemas, permissions, evaluations, and dependency closure.

A local Director model plans against retrieved evidence. A separate visual critic model inspects samples and finals. Deterministic stages perform arithmetic, boundaries, asset validation, rendering, and QA. The critic may reject or escalate but cannot directly mutate a project.

### 3.4 Studio

The user-facing information architecture is:

```text
Home → Sources → Transcript → Story → Beats → Timeline
     → Design → Motion & Sound → Compare → Finals → QA & Receipts → Settings
```

Studio is not a generic Premiere clone. It exposes the corrective operations automation most often gets wrong: restore/drop a passage, select another take, move a beat, adjust boundaries, edit transcript/captions, change reframe anchors, replace/disable assets, tune graphics/motion/audio, rerun one stage, compare variants, and record a preference.

### 3.5 Shared Capability Registry

One versioned registry describes actions, skills, tools, models, runtime packs, renderers, assets, permissions, requirements, degradation, schemas, and evaluations. CLI commands, Studio bindings, embedded-agent tools, optional MCP tools, documentation tables, and contract fixtures are generated from or checked against this registry. There is one executor and one semantic action vocabulary.

## 4. Canonical project flow

```text
ProductionBrief
  ↓
Immutable Sources + Runtime Pack Lock
  ↓
Hierarchical Evidence Graph
  ↓
EditorialPlan + confidence + chronology log
  ↓
Timeline Revision (natural / tight / platform variants)
  ↓
CreativePlan + AssetRequests + FinishPlan
  ↓
Native Render Graph
  ↓
Independent Critic + deterministic QA
  ↓
Finals + package + digest + preference evidence
```

## 5. Agent execution loop

```text
plan
→ retrieve bounded evidence
→ propose typed action batch
→ schema and semantic validation
→ dry-run and semantic diff
→ atomic transaction
→ render representative samples
→ independent critic
→ revise once within budget or escalate
→ final render and deterministic QA
```

The model that made a decision is not the sole authority certifying it. The independent critic receives the user brief, action diff, evidence references, and rendered samples, but not the planner's hidden reasoning.

## 6. Transaction model

A timeline edit is a revision-bound batch:

```rust
pub struct ActionBatch {
    pub batch_id: ActionBatchId,
    pub project_id: ProjectId,
    pub timeline_id: TimelineId,
    pub expected_revision: RevisionId,
    pub actions: Vec<Action>,
    pub evidence_refs: Vec<EvidenceRef>,
    pub intent: String,
}
```

The kernel validates every action against the current revision, applies it to a staged clone, validates the resulting timeline, writes artifacts atomically, creates a new revision, emits receipts and an inverse batch, and only then changes the active pointer. A stale expected revision is rejected.

## 7. Native renderer

CutRight uses a declarative render graph owned by CutRight. The graph composes source media, crops, transforms, masks, captions, vector/text layers, images, procedural elements, colour, audio, transitions, and effects. FFmpeg/libav provides codec, resampling, filtering, muxing, and selected compositing primitives. A Rust GPU/vector layer provides deterministic branded graphics and motion where FFmpeg is insufficient.

Existing Remotion and HyperFrames implementations are visual fixtures and migration references only. The installed product does not require Node, Chromium, an external package, or a commercial renderer licence.

## 8. Runtime packs

The installer ships or contains signed first-party packs:

- Media pack: FFmpeg/FFprobe and native codec/filter configuration.
- Speech pack: Parakeet, Silero VAD, alignment/verifier runtime, and dictionaries.
- Director pack: local text planning model and inference runtime.
- Vision critic pack: multimodal critic and visual tracking components.
- Voice pack: TTS runtime, model, phonemizer data, and audited voices.
- Creative pack: fonts, templates, vector assets, textures, SFX, music, LUTs, effect schemas, and previews.

Each pack is immutable, content-addressed, signed, independently verifiable, and repairable from the offline installer payload. No component is discovered on `PATH`.

## 9. Autonomy

Autonomy is earned per `content_type × platform × variant`. New formats begin `reviewed`, progress to `review-light`, and may reach `autonomous` only after benchmark floors and the user's acceptance data are satisfied. Automatic demotion is immediate after a rejected final, unresolved escalation, benchmark regression, pack change, or critic disagreement above threshold.

`autonomous` means no intermediate approval. It does not mean self-publishing. Final visual sign-off and any account mutation remain explicit user actions.

## 10. Clean-machine acceptance

A release is standalone only when a fresh supported machine can install the complete offline bundle, disconnect networking, clear user `PATH`, and complete all four production lanes without another repository, global skill, interpreter, media utility, model server, browser download, or cloud key. Network attempts, missing-pack downloads, and silent system-tool fallback are test failures.

## 11. Deliberate non-goals for v2

- Full parity with every manual NLE feature.
- Hosted generation or cloud analysis as a required path.
- Automatic posting or account mutation.
- A marketplace for arbitrary untrusted skills.
- Training a new foundation model before product benchmarks prove a need.
- Copying GPL source into the MIT product.


---

# CutRight v2 runtime and model matrix

## 1. Pack policy

A row may enter a shipping pack only after its source revision, output byte hash, licence, notices, supported targets, peak memory, disk size, throughput, deterministic fixture result, and degradation behavior are present in `runtime/packs.lock.json`. The values measured by the pack builder are authoritative; estimates and mutable URLs are rejected.

## 2. Selected matrix

| Component | Selected source | Pinned revision | Licence | Pack | Shipping role | Release rule |
| --- | --- | --- | --- | --- | --- | --- |
| Media runtime | FFmpeg 8.1 | 9047fa1b084f76b1b4d065af2d743df1b40dfb56 | LGPL-2.1-or-later configuration | media | Probe, decode, encode, filter, resample, mux and evidence extraction. | Build with recorded configure line; forbid GPL/nonfree; ship corresponding source. |
| Local LLM runtime | llama.cpp | 6a32c29a746a2e44de463de647f9f6661eb5086b | MIT | director + vision | CutRight-owned local inference binaries. | Build per supported target; no server process required; network disabled. |
| Primary speech engine | HeardRight engine/core/platform | b60bff947f12ffa9d25e94ad27e8ff30db006a24 | User-owned source plus third-party notices | speech | Parakeet timed transcript authority and shared audio preprocessing. | Copy exact source; remove external discovery; all models resolved from signed pack. |
| Primary ASR weights | Parakeet TDT shipping model from pinned HeardRight tree | hash generated by Book 1 import | Must be resolved from HeardRight legal/model provenance | speech | Timed transcript authority. | Shipping blocked until exact file list, licence and SHA-256 are frozen. |
| VAD | Silero VAD ONNX | 76e3dc408eb2a5c655c34e230d2d5459b4439daa | MIT | speech | Independent speech probability and pause evidence. | Exact model byte hash required; 8/16 kHz parity fixtures pass. |
| Independent verifier | whisper.cpp v1.9.2 | 306c88f4d1286aec1bf96e544632897886af5501 | MIT; selected model licence separately recorded | speech-quality | Independent transcript and edge evidence; never authority by itself. | Select one multilingual model in Book 3; pack hash and licence mandatory. |
| Director model | Qwen/Qwen3-4B | 7c69a109fc3fa19c860be9dff46fc23299092018 | Apache-2.0 | director | Editorial planning, tool selection, structured plan generation. | In-house deterministic GGUF conversion; Q4_K_M is a starting candidate, final quant selected by benchmark. |
| Visual critic model | Qwen/Qwen3-VL-4B-Instruct | ebb281ec70b05090aa6165b016eac8ec08e71b17 | Apache-2.0 | vision-quality | Independent visual, temporal, layout and instruction critic over bounded evidence. | In-house GGUF/mmproj conversion; ship only if all supported targets pass quality and memory floors. |
| Director/critic upgrade candidate | Qwen/Qwen3.5-4B | 851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a | Apache-2.0 | none by default | Potential unified model after runtime maturity. | Never silently replaces selected models; requires a full pack-version and benchmark transition. |
| Face/pose tracker | MediaPipe v0.10.35 or a CutRight-owned replacement | f8ef212d5c962c0e853db7e59d217056b187084b | Apache-2.0; each model asset separately audited | vision | Face, pose, hand and subject continuity tracks. | Ship only with telemetry removed/disabled and zero outbound network in blocked-network tests; otherwise use qualified fallback. |
| TTS | Kokoro-82M v1.0 | 496dba118d1a58f5f3db2efc88dbdc216e0483fc89fe6e47ee1f2c53f18ad1e4 | Apache-2.0 model; voices separate | voice | Local narration and preview speech. | Model hash fixed; every voice/phonemizer asset must pass separate redistribution and pronunciation tests. |
| Native compositor | CutRight Rust render graph | CutRight lockfiles | MIT project plus dependency notices | media + creative | Text, vector, image, mask, transition and procedural motion rendering. | No Node/Chromium runtime; Cargo licence graph and golden-frame tests pass. |
| Skills and creative assets | Pinned workspace closure + CutRight originals | 6ee21f03a787e7b57dc412760a8996ea7a235302 | Per-file ledger | creative | Designer, brand, writing, platform constraints, templates, fonts, SFX and previews. | Unclassified transitive reference or asset blocks pack signing. |

## 3. Product tiers

### Base application

Contains the Media Kernel, Studio, capability registry, action executor, project/index storage, pack verifier, and enough built-in fixtures to open and inspect projects. It does not pretend missing model packs are installed.

### Creator offline bundle

Contains `media`, `speech`, `director`, `vision`, `voice`, and `creative`. It supports all four lanes on qualified consumer Mac and Windows targets. This is the acceptance target for “no external dependencies.”

### Quality pack

May add a larger visual critic, higher-quality verifier, or larger language model. It is optional and separately signed. Autonomous mode may require this pack for formats whose benchmark floor cannot be met by Creator.

## 4. Required lock fields

```json
{
  "pack_id": "speech",
  "pack_version": "1.0.0",
  "target": "aarch64-apple-darwin",
  "files": [
    {
      "path": "bin/cutright-speech",
      "sha256": "64 lowercase hex characters",
      "size_bytes": 0,
      "source_id": "heardright@b60bff947f12ffa9d25e94ad27e8ff30db006a24",
      "license_id": "resolved-ledger-entry"
    }
  ],
  "measured": {
    "peak_rss_bytes": 0,
    "cold_start_ms": 0,
    "fixture_throughput": 0.0
  },
  "signature": {"algorithm": "ed25519", "key_id": "cutright-pack-release"}
}
```

Zero measurements or non-hex hashes are schema-invalid in a release lock. They are shown above only to define field types; fixture manifests use a non-release schema and cannot be signed.

## 5. Platform matrix

The initial release target matrix is:

- macOS Apple Silicon: Metal/CoreML where qualified; CPU fallback must remain functional.
- macOS Intel: CPU path; no feature may be advertised without benchmark evidence.
- Windows x64: DirectML/CPU where qualified; no visible console windows for workers.
- Windows ARM64: blocked until all required packs, installers, and benchmarks pass natively.
- Linux: source-build and headless engine support may ship after the desktop product; it is not allowed to dilute Mac/Windows acceptance.

Every pack declares exact accelerators and fallbacks. Unsupported acceleration is a typed degradation, not a crash or silent cloud fallback.

## 6. Runtime resolution

```text
application resource root
→ active signed pack lock
→ target-specific relative file
→ hash and signature verification
→ capability handshake
→ use
```

Environment-variable overrides are allowed only in an explicit development build. Release builds reject them. Bare executable names and user `PATH` resolution are forbidden.

## 7. Remotion and HyperFrames disposition

Neither is a shipping runtime. Existing CutRight Remotion effects and workspace HyperFrames examples are retained in `imports/provenance/` and converted into native golden fixtures. The native renderer must match the declared timing, safe zones, reduced-motion behavior, and visual acceptance—not the implementation technology.


---

# CutRight v2 benchmark and evaluation plan

## 1. Purpose

The benchmark programme determines architecture, pack selection, autonomy, and release readiness. It is not an end-of-project demo. No format advances beyond `reviewed` because a model card looks strong or because one showcase edit looks good.

## 2. Golden corpus

Every media item has a rights manifest, source hash, consent/provenance, expected language, camera/audio conditions, and permitted distribution. The minimum corpus contains:

- 40 recorded-footage projects: single take, multi-take, interview, podcast, tutorial, screen recording, multi-camera, difficult room tone, interruptions, camera handling and mixed frame rates.
- 30 repurpose projects: podcasts, talks, tutorials and finished videos with human-ranked standalone shorts.
- 20 explainer projects: local source packages, scripts, charts, narration, product/process and historical topics.
- 20 anchored-creative projects: presenters, product labels, logos, packaging, identity-sensitive photos and A-roll restyling.
- 20 adversarial projects: false chronology risk, contradictory takes, clipped speech, silent spans, captions near UI zones, camera motion, low light, HDR, variable frame rate, corrupt media and interrupted jobs.

At least 25% of the speech corpus is non-English or code-switching before multilingual support is claimed. Projects are split by speaker, recording session and source programme so near-duplicates cannot cross train/calibration and test sets.

## 3. Evaluation axes

### 3.1 Kernel integrity

- Source mutation count: exactly zero.
- Atomic action success: 100% of injected interruption points leave either the old revision or the complete new revision.
- Undo round-trip: canonical timeline hash returns to the pre-action hash for every reversible action.
- Receipt verification: 100% pass before packaging; tampering is detected.
- Stale revision rejection: 100%.
- Cache identity: moved paths preserve cache hits; changed bytes invalidate them.

### 3.2 Speech and boundary quality

- Word/phoneme truncation: zero clipped high-confidence lexical words in the release corpus.
- Boundary onset/offset absolute error: report median, p90 and p95 against human labels.
- Boundary consensus coverage: fraction supported by primary ASR, VAD and verifier.
- Filler removal precision/recall by context class.
- False-start removal precision and replacement-presence proof.
- Transcript preservation: the canonical transcript never deletes speech because a cut drops it.

### 3.3 Audio-visual preservation

- Audio-video sync drift and local onset alignment before/after edit.
- Non-target frame preservation outside declared actions.
- Non-target audio preservation outside declared fades, cuts and mixes.
- Loudness, true peak, clipping, discontinuity, noise and channel-layout checks.
- Identity, label, logo and OCR preservation for anchored projects.

### 3.4 Editorial quality

- Beat segmentation agreement with human editors.
- Duplicate-take clustering precision/recall.
- Best-take acceptance and score margin.
- Narrative-order acceptance.
- Hook, payoff and CTA acceptance.
- Reorder truthfulness violations: zero accepted false-chronology cases.
- Manual correction time and number of corrections per minute of final video.
- Final accept/reject and reason distribution.

### 3.5 Creative quality

- Brand-token compliance.
- Subject, face, gesture, caption and platform-UI collision count: zero unresolved collisions.
- Crop stability and subject loss.
- Graphic, caption, motion, SFX and music acceptance.
- Reduced-motion equivalence.
- Text legibility and OCR match.
- Visual critic agreement with human review, including false-positive and false-negative rates.

### 3.6 Instruction and preservation quality

Each request is decomposed into a checklist. Score:

- target success;
- untouched-content preservation;
- joint success;
- intent fidelity;
- realism/naturalness;
- temporal and spatial consistency;
- evidence traceability;
- escalation correctness.

### 3.7 Reliability and resource quality

- Cold start, stage latency, throughput, peak RSS/VRAM, disk writes and pack load time by supported target.
- Cancellation latency.
- Resume success at every stage boundary.
- Retry amplification and duplicate-cost prevention.
- Clean-machine offline success.
- Zero network attempts in offline mode.

## 4. Evaluator separation

Planner and critic are separate model instances and separate prompts. Deterministic metrics run first. The critic sees the brief, evidence references, semantic action diff, before/after samples, and deterministic findings. It returns a schema-bound verdict with exact frame/time evidence. It cannot execute actions.

A critic disagreement triggers one bounded revision cycle. A second disagreement or low-confidence verdict escalates. The planner's self-assessment is logged but never counted as independent evidence.

## 5. Initial gates

These are starting release floors; the benchmark report must show confidence intervals and may tighten them, never silently loosen them.

| Gate | Reviewed | Review-light | Autonomous |
|---|---:|---:|---:|
| Consecutive human-accepted finals | 0 | 5 | 10 after review-light |
| Projects in current mode | 0 | ≥5 | ≥15 |
| Best-take acceptance | report only | ≥0.85 | ≥0.92 |
| Boundary correction rate | report only | ≤0.15 | ≤0.08 |
| Graphic acceptance | report only | ≥0.80 | ≥0.90 |
| Unresolved escalation rate | allowed, blocks run | ≤0.15 | ≤0.05 |
| False chronology | 0 accepted | 0 accepted | 0 accepted |
| Source mutation / atomicity / receipt failure | 0 | 0 | 0 |
| Unresolved caption/subject collision | 0 | 0 | 0 |
| Independent critic required | advisory | required for finish | required for every final |

A runtime-pack, model, prompt, skill, renderer, or threshold change resets or invalidates affected format evidence according to its compatibility declaration.

## 6. Benchmark artefacts

Each run writes:

```text
benchmarks/runs/<run_id>/manifest.json
benchmarks/runs/<run_id>/per-project.jsonl
benchmarks/runs/<run_id>/metrics.json
benchmarks/runs/<run_id>/confusion-matrices/
benchmarks/runs/<run_id>/samples/
benchmarks/runs/<run_id>/failures/
benchmarks/runs/<run_id>/report.md
benchmarks/runs/<run_id>/receipt.json
```

The report identifies skipped checks, missing labels, unsupported targets and unproven claims. A tool that did not run is `unproven`, never `pass`.

## 7. Human review protocol

Reviewers compare blinded variants where possible, use a fixed reason vocabulary, and annotate exact time ranges. Editorial disagreement is retained; consensus and individual preferences are separate data. The user’s acceptance controls their autonomy profile, while the shared benchmark controls minimum safety and integrity floors.


---

# CutRight v2 implementation plan

## 1. Authority and supersession

This plan supersedes the earlier CutRight standalone implementation package. The v2 source corpus is frozen to 2026-08-06; later source/model changes require a new corpus revision and compatibility decision.

## 2. End state

CutRight is one installable offline product with five systems: Media Kernel, Evidence and Job Plane, Embedded Creative Operating System, Studio, and Shared Capability Registry. The complete Creator offline bundle performs recorded-footage editing, repurposing, procedural explainers and anchored creative without a sibling repository, global skill, Python, Node, Ollama, system FFmpeg, browser download, cloud key or network connection.

## 3. Implementation sequence

| Book | Title | Tasks | Why it is before the next book |
| --- | --- | --- | --- |
| 1 | Reproducible Corpus, Licence Closure, and Standalone Boundary | 27 | Freeze every source and licence input, compute the relevant skill/tool closure, vendor the permitted material, and make unresolved or external runtime references impossible to ship. |
| 2 | Shared Capability Registry, Typed Actions, and Transactional Project State | 27 | Create one action and capability contract for Studio, the embedded agent, CLI, MCP and tests; make every mutation revision-bound, atomic, validated and undoable. |
| 3 | Signed Runtime Packs, Hierarchical Evidence Graph, and Durable Job Plane | 27 | Replace every system-tool and sibling-app dependency with signed CutRight packs, then build bounded multimedia evidence retrieval and content-addressed resumable jobs. |
| 4 | Benchmark-First Evaluation and Editorial Intelligence | 27 | Establish the golden corpus, deterministic and model-based evaluators, then implement editorial reasoning under measurable confidence, preservation, truthfulness and escalation constraints. |
| 5 | Embedded Creative Operating System and Native Finish Renderer | 27 | Turn the imported skills into a product-local creative system, implement Designer/brand/script/platform contracts, and replace external render technologies with a CutRight-owned native finish graph. |
| 6 | Full Studio Authoring Surface, Embedded Agent, and Optional MCP | 27 | Productize the engine as a coherent desktop workflow with corrective editing, bounded evidence inspection, one-click production, and one shared typed agent tool surface. |
| 7 | Measured Autonomy, Security Hardening, Offline Distribution, and Release Acceptance | 27 | Turn review evidence into bounded per-format autonomy, harden the local product, migrate existing projects, and prove signed offline installers on clean machines without CI or external dependencies. |

The order is mandatory. Book 4 benchmark and editorial work cannot begin against mutable runtime/source identities. Book 5 creative breadth cannot be trusted before the action/evidence/benchmark foundation. Studio authoring comes after the domain APIs. Autonomy and distribution come last.

## 4. Parallel model within a book

Tasks 001–006 are sequential and freeze contracts. Tasks 007–011, 012–016 and 017–021 form three disjoint lanes. Task 022 joins all lanes in fixed A→B→C order. Tasks 023–027 integrate, test and run the one full local gate.

A single agent may execute numeric order. Parallel execution requires already-authorised isolated checkouts and must preserve one commit per task. No task tells an agent to create branches or worktrees without current user authority.

## 5. Global completion invariants

- Every source, skill, model, asset, runtime and binary is pinned, classified, licensed and hash-bound.
- Project package and immutable revision/object graph are canonical; SQLite indexes are disposable.
- Studio, agent, CLI and MCP use one capability registry, one action vocabulary and one executor.
- Skills/models propose typed artefacts/actions; Rust validates and mutates atomically.
- Evidence is hierarchical, bounded and traceable; compact summaries do not replace source evidence.
- Planner and critic are independent; deterministic checks run first.
- Cutaway selection/timing and Finish styling remain separate, with a locked editorial revision.
- Native CutRight renderer is the shipping path. Remotion/HyperFrames are provenance/migration references only.
- Runtime resolution uses signed CutRight packs only; no PATH, user interpreter, sibling application or network fallback.
- No hosted CI. `scripts/gate.sh --with-qa` remains the authoritative repository gate and runs once at the end of each book.
- A clean-machine blocked-network proof is required for every claimed desktop target.

## 6. Dispatch size

- 7 books
- 27 tasks per book
- 189 tasks total
- 15 parallel-lane tasks per book (5 in each of three lanes); 12 tasks are sequential.
- Each task has exact dependencies, ownership, commands, acceptance, stop-loss ceilings and commit message.

## 7. Release boundary

The dispatch ends with a sealed local release candidate and checksum manifest. Upload, public release, tags, announcements, account mutation and spend are intentionally excluded.


---

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


---

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


---

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


---

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


---

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


---

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


---

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
