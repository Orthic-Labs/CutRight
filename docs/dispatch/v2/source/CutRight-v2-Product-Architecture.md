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
