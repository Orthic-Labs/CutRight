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
