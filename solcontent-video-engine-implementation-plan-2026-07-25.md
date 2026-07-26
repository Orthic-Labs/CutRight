# Content Video Engine
## Implementation Plan for YouTube and Reels/TikTok

**Target:** macOS first, Apple Silicon preferred
**Control plane:** Rust + Tauri
**Media execution:** FFmpeg/FFprobe
**Graphics:** Remotion and HyperFrames, selected per slot
**AI agents:** Claude Code or Codex
**Cloud APIs:** optional, disabled by default
**Primary output:** finished MP4 files, not a mandatory NLE project
**Product-demo recording:** out of scope; handled by the separate demo-recording system
**Research state:** verified through 25 July 2026

---

# 1. Executive decision

Build one local project system with three layers:

```text
Claude Code / Codex skill
        ↓
Rust orchestration engine + Tauri review surface
        ↓
Deterministic media tools and optional specialist providers
```

The coding agent is the **editorial director**. It reads structured evidence, proposes edits, writes validated plans, and invokes stable tools. It must not improvise shell pipelines, inspect tens of thousands of raw frames, or treat a single multimodal API response as the source of truth.

The default workflow must work without a paid API:

```text
Raw footage
→ local ingest and metadata
→ local VAD and transcription provider
→ transcript-led edit with sampled visual evidence
→ reviewed rough cut
→ output-timeline transcript
→ captions, graphics, audio and colour
→ final render
→ technical and visual QA
→ YouTube and vertical variants
```

Optional APIs improve particular stages:

```text
Gemini
→ one-shot semantic video analysis, OCR-rich inspection and final-render audit

Twelve Labs
→ timestamped segmentation, long-video analysis and searchable B-roll libraries

Cloud transcription
→ fallback when the local/workspace transcriber is unavailable or less accurate
```

The system is not one giant `SKILL.md`. It is one installable root skill that routes into internal workflow documents and calls a local CLI.

---

# 2. Decisions on the Perplexity and Gemini proposals

## 2.1 Keep: deterministic local core plus optional cloud intelligence

The proposed separation between speech structure, visual understanding and deterministic rendering is correct. Keep the principle, but do not make any single provider mandatory.

The implementation should offer four operating modes:

| Mode | Uploads footage? | Visual intelligence | Intended use |
|---|---:|---|---|
| `offline` | No | Local sampled frames + Apple Vision + agent inspection | Private footage and zero recurring cost |
| `assisted` | Yes, opt-in | Gemini video analysis and/or final QA | Faster semantic planning |
| `library` | Yes, opt-in | Twelve Labs analysis and semantic video search | Large reusable B-roll/archive |
| `experimental` | Configurable | Research plugins such as TRIBE | Evaluation only; never the default |

## 2.2 Reject: destructively stitching VAD speech before editorial analysis

VAD must be a **signal**, not the first destructive edit.

The generated proposal does:

```text
Silero VAD → stitch talk-time → transcribe → plan
```

That has several defects:

- It can remove intentional silence, breaths and reaction beats before the editor reasons about pacing.
- It changes the timeline before visual analysis, forcing every later timestamp through another mapping.
- It can conceal false starts or ghost speech at boundaries.
- It prevents creation of both tight and natural variants from one authoritative analysis.
- It makes visual discontinuities harder to diagnose.

Correct order:

```text
Original immutable source
→ extract audio
→ VAD probabilities and candidate intervals on original timestamps
→ word-level transcription on original timestamps
→ scene and visual signals on original timestamps
→ editorial plan
→ source-to-output timeline map
```

A trimmed proxy may be rendered after the cut plan is approved, but the original timebase remains canonical.

## 2.3 Keep Parakeet as an option; reject the precision claim

NVIDIA’s Parakeet TDT model cards document character-, word- and segment-level timestamps. They do **not** establish the proposal’s universal “under 10 ms word alignment error” claim. Do not make cut padding depend on that number.

Parakeet should be one provider behind a common interface:

```text
workspace / HeardRight transcription
Parakeet TDT
Whisper / WhisperX / faster-whisper
ElevenLabs Scribe
AssemblyAI
```

For Adrian’s environment, the existing workspace transcription path should be the preferred provider because the Content skill already routes video transcription to the dedicated transcription specialist, and HeardRight already has Para TDT and Whisper infrastructure.

NVIDIA documents Apple M-series inference through PyTorch MPS with fallback enabled because not every operation is implemented on MPS. Therefore Parakeet is viable as an optional Python sidecar on Mac, but it should not be rewritten into the Rust core or assumed to be the fastest path on every Apple Silicon machine.

## 2.4 Keep Silero VAD, implemented through ONNX

Silero VAD is an appropriate local signal:

- permissive MIT licence;
- small model;
- supports 8 kHz and 16 kHz audio;
- provides timestamped speech intervals;
- can run through ONNX Runtime.

Use it from Rust through an ONNX runtime crate or through a tiny isolated sidecar. Store frame-level probabilities or intervals, not only a binary final cut.

Do not hard-code one universal pair of thresholds. Profiles should be calibrated on Adrian’s footage:

```yaml
vad_profiles:
  tight_social:
    start_probability: 0.55
    end_probability: 0.35
    speech_pad_ms: 160
  natural_youtube:
    start_probability: 0.50
    end_probability: 0.30
    speech_pad_ms: 280
```

The values above are initial configuration candidates, not validated truths.

## 2.5 Reject TRIBE as a production engagement gate

TRIBE v2 predicts fMRI responses to naturalistic video, audio and text for an average subject on a cortical mesh. Its own quickstart returns approximately 20,000 cortical-vertex predictions and applies a five-second offset for haemodynamic lag.

The proposed conversion from those outputs into:

```text
engagement_score
attention_high
emotion_high
language_dense
```

is a new, unvalidated reduction invented by the generated report. TRIBE does not ship an editing-quality or audience-retention score.

Additional blockers:

- CC BY-NC 4.0 licence;
- substantial model/runtime complexity;
- a five-second physiological lag that is mismatched to frame-level edit placement;
- no evidence that a TRIBE-derived score predicts YouTube or TikTok retention for Adrian’s audience.

Decision:

```text
TRIBE plugin:
  default: disabled
  production gate: forbidden
  commercial use: forbidden without licence clearance
  allowed use: isolated internal experiment with labelled evaluation
```

A better long-term engagement model is trained from Adrian’s actual YouTube and Instagram analytics: retention curves, swipe-away rate, average percentage viewed, rewatches, saves and shares. The Social skill already treats live analytics as the basis for optimisation rather than generic theory.

## 2.6 Use Gemini for semantics and QA, not precise cutting

Gemini’s official video-understanding documentation supports video description, segmentation, information extraction and timestamp references. It also supports schema-constrained JSON output.

However, the File API documentation states that videos are processed at one frame per second by default, with audio processed at a low bitrate. This is adequate for narrative understanding, OCR at selected moments, broad scene classification and final-render review. It is not the authoritative source for:

- word boundaries;
- fast gestures;
- frame-accurate cut positions;
- cursor motion;
- brief graphic collisions;
- lip-sync.

Use Gemini for:

```text
- classify the narrative and footage type
- identify visually important moments
- propose B-roll themes
- flag unusable shots
- inspect selected high-resolution stills
- review a final render against a structured checklist
```

Do not use Gemini to overwrite local timestamps.

The provider must use the current Interactions API conventions and schema support rather than pinning implementation to older `response_mime_type` examples that Google has since changed.

## 2.7 Use Twelve Labs when persistent indexing is valuable

Twelve Labs Pegasus 1.5 currently supports prompt-based analysis and structured, timestamped segmentation. Official documentation distinguishes synchronous analysis for shorter videos and asynchronous tasks for videos up to two hours or for segmentation.

Use Twelve Labs for:

- timestamped editorial segmentation;
- analysis of long source recordings;
- reusable semantic search over a B-roll/archive collection;
- cross-video retrieval;
- highlight candidates.

Do not pin the code to “Marengo 2.7”; Twelve Labs sunset that model version in March 2026. Code against provider capabilities and current API model identifiers.

Twelve Labs has MCP offerings, but the engine should still use a direct provider adapter for repeatable production runs. MCP is convenient for exploration; a typed HTTP adapter gives the local application stable schemas, retries, caching, quotas and test doubles.

## 2.8 Reject “MCP first”

MCP is an integration surface, not the internal architecture.

Correct order:

1. Build a deterministic Rust CLI and JSON contracts.
2. Make the Tauri UI call the Rust library directly.
3. Let Claude Code and Codex invoke the CLI through their skills.
4. Add an MCP wrapper only after the commands and schemas are stable.

This avoids coupling the media engine to one agent host and makes every operation independently testable.

## 2.9 Reject the sample FFmpeg command

The sample MCP implementation combines video and audio filters inside `-vf`:

```text
select=...,aselect=...,setpts=...,asetpts=...
```

That is not a valid unified video-filter expression. Audio and video have separate filter chains. More importantly, selecting many ranges from one input and resetting timestamps is not sufficient production logic for clean speech edits.

The engine should compile the canonical timeline into one of two tested render strategies:

### Rough-cut strategy

For each segment:

```text
trim video precisely
atrim audio precisely
reset timestamps
apply edge audio fades or room-tone bridges
normalise common codec, size, frame rate and sample rate
```

Then concatenate the homogeneous segments.

### Final strategy

Use the approved rough-cut master as one continuous base and perform one compositing/encoding pass for:

- graphics;
- captions;
- B-roll;
- colour;
- music and SFX;
- platform framing.

“Single pass” is a useful optimisation target for the final composite, not a correctness requirement for every stage.

## 2.10 Use VideoToolbox, but do not promise a universal zero-copy pipeline

Apple’s VideoToolbox exposes hardware-accelerated encoding and decoding. FFmpeg exposes H.264 and HEVC VideoToolbox encoders.

That does not mean every FFmpeg filter, LUT, subtitle renderer or browser-generated overlay stays in a zero-copy Metal/CoreVideo path. Treat hardware decoding and encoding as capabilities, not proof that the entire graph is GPU-resident.

Provide render profiles:

```yaml
render_profiles:
  proxy:
    encoder: h264_videotoolbox
    target: fast
  preview:
    encoder: h264_videotoolbox
    target: balanced
  final_fast:
    encoder: hevc_videotoolbox
    target: quality
  final_master:
    encoder: prores_ks
    profile: 3
  final_delivery:
    encoder: libx264
    crf: 18
```

Actual quality and speed must be benchmarked on the target Mac.

---

# 3. Product scope

## 3.1 Supported content

### YouTube

- single- and multi-take talking head;
- tutorials and educational commentary;
- interviews with one or more recorded speakers;
- build logs;
- commentary with screenshots, B-roll and generated graphics;
- 5–20 minute finished videos;
- chapters, captions, titles and thumbnail handoff.

### Reels, TikTok and Shorts

- extraction of several materially different ideas from a long recording;
- direct short-form recordings;
- vertical reframing;
- word-highlight captions;
- hook cards, progress cues, callouts, B-roll and SFX;
- 15–180 second variants;
- platform package handoff to Social.

## 3.2 Explicit non-goals for v1

- product-demo screen recording and cinematic auto-zoom capture;
- full DaVinci/Premiere replacement;
- professional multicam synchronisation;
- automatic generative-video B-roll by default;
- face retouching, eye-contact correction or synthetic lip sync;
- TRIBE-based engagement optimisation;
- a cloud SaaS;
- Windows support in the first implementation.

DaVinci/FCPXML/OTIO export can be added later without making it part of the default workflow.

---

# 4. System architecture

```text
┌──────────────────────────────────────────────────────────────────┐
│ Agent layer                                                      │
│ content/video-editor skill                                       │
│ Claude Code or Codex                                             │
│ - reads briefs and compact evidence                              │
│ - proposes strategy                                              │
│ - writes validated edit/finish plans                             │
│ - invokes videoctl commands                                      │
└────────────────────────────┬─────────────────────────────────────┘
                             │ CLI / JSON
┌────────────────────────────▼─────────────────────────────────────┐
│ Rust control plane                                               │
│ video-core crate + videoctl CLI + Tauri commands                 │
│ - project state and migrations                                  │
│ - provider routing                                               │
│ - job graph, cache, budgets, retries                             │
│ - timeline compiler                                              │
│ - FFmpeg invocation                                              │
│ - validation, provenance and QA                                 │
└───────────────┬──────────────────┬───────────────────┬───────────┘
                │                  │                   │
┌───────────────▼──────┐ ┌────────▼────────┐ ┌────────▼───────────┐
│ Native Mac perception│ │ Node renderers   │ │ Optional model     │
│ Swift CLI sidecar    │ │ Remotion         │ │ worker             │
│ Apple Vision         │ │ HyperFrames      │ │ Python             │
│ OCR/faces/body/hands │ │ graphics slots   │ │ Parakeet/Whisper   │
│ saliency             │ │ transparent media│ │ optional research  │
└──────────────────────┘ └─────────────────┘ └────────────────────┘
                │                  │                   │
                └──────────────────┴───────────────────┘
                                   │
                         ┌─────────▼─────────┐
                         │ FFmpeg / FFprobe  │
                         │ media execution   │
                         └───────────────────┘
```

## 4.1 Rust is the authority

Rust owns:

- project lifecycle;
- schema validation;
- source hashing;
- job orchestration;
- immutable source policy;
- timeline arithmetic;
- cache keys;
- provider selection;
- process execution;
- status events;
- secrets references;
- budget enforcement;
- final QA aggregation.

Use external processes where the mature ecosystem already exists. Do not rewrite Remotion, HyperFrames, FFmpeg or NeMo in Rust.

## 4.2 Tauri is the review surface, not the media engine

The Tauri application is a thin review and control UI:

- projects;
- transcript;
- waveform;
- filmstrip;
- rough-cut timeline;
- alternative variants;
- effect selection;
- caption placement;
- render progress;
- QA findings;
- final delivery.

The frontend may be React/Vite because Remotion components and TypeScript schemas can be shared, while all critical state and execution remain in Rust.

Tauri supports bundling external binaries as sidecars, including architecture-specific Apple Silicon executables. Use that for the Swift perception helper and, later, packaged model workers. Node remains a developer/runtime dependency in the first version rather than being embedded immediately.

## 4.3 Swift Vision sidecar

Create a small native command-line binary:

```text
vision-mac analyze --manifest frames.json --output spatial.json
```

Use Apple Vision for:

- face rectangles;
- body pose;
- hand pose;
- text recognition and bounding boxes;
- attention-based saliency;
- objectness saliency;
- image-quality and similarity signals where useful.

This is a better Mac-specific v1 than introducing MediaPipe/Python for every visual signal. The output is provider-neutral JSON, so a later cross-platform implementation can replace it.

## 4.4 Node renderer sidecars

### Remotion

Primary renderer for reusable, branded components:

- captions;
- lower thirds;
- counters and pricing cards;
- charts;
- browser/terminal/device frames;
- comparison cards;
- CTA cards;
- progress components;
- reusable transitions;
- transparent overlays.

Your current Remotion skill already defines frame-driven animation, typed parameters, captions, transitions, media, charts, maps, Lottie, transparent output and audio. Keep it as the technical authority.

### HyperFrames

Secondary renderer for:

- one-off HTML/CSS/GSAP compositions;
- kinetic typography;
- bespoke editorial interludes;
- website-like graphics;
- existing catalog blocks;
- fast isolated visual experiments.

HyperFrames is Apache 2.0, agent-oriented, supports deterministic seeking across several web animation runtimes and has a reusable block catalog. It is a strong optional backend, not a replacement for the permanent Remotion library.

### ASS/libass

Default renderer for inexpensive captions where full React graphics are unnecessary:

- clean phrase captions;
- karaoke;
- outline/shadow;
- fixed placement;
- multilingual text, subject to font shaping tests.

Use Remotion when captions need complex layout, spatial avoidance, per-word transforms, rich backgrounds or reusable brand behaviour.

---

# 5. One canonical project package

```text
MyVideo.video-project/
├── project.json
├── brief/
│   ├── platform-brief.json
│   ├── editorial-brief.md
│   ├── script.md
│   ├── script-lock.json
│   ├── VIDEO-BRAND.md
│   ├── FRAME.md
│   └── motion-plan.md
├── sources/
│   └── manifest.json
├── cache/
│   ├── audio/
│   ├── proxies/
│   ├── frames/
│   ├── waveforms/
│   └── provider-responses/
├── analysis/
│   ├── vad.json
│   ├── transcript.json
│   ├── transcript-packed.md
│   ├── scenes.json
│   ├── spatial.json
│   ├── audio-features.json
│   ├── visual-evidence.json
│   └── cloud-analysis/
├── edit/
│   ├── candidates.json
│   ├── cut-plan.json
│   ├── timeline.json
│   ├── variants/
│   ├── output-transcript.json
│   └── captions.json
├── finish/
│   ├── finish-plan.json
│   ├── slots/
│   ├── effects-used.json
│   ├── audio-plan.json
│   └── colour-plan.json
├── render/
│   ├── proxies/
│   ├── rough-cuts/
│   ├── slots/
│   ├── previews/
│   └── finals/
├── qa/
│   ├── technical.json
│   ├── editorial.json
│   ├── visual.json
│   ├── captions.json
│   ├── audio.json
│   └── report.md
├── feedback/
│   ├── decisions.jsonl
│   └── preferences.json
└── exports/
    ├── youtube/
    ├── vertical/
    ├── captions/
    └── interchange/
```

Raw source files can remain outside the package. `sources/manifest.json` stores canonical absolute paths, hashes and metadata. No command may modify a source file.

JSON files are the portable source of truth. A local SQLite database may index projects, jobs, cached embeddings and effect metadata, but the project must remain reconstructable from files alone.

---

# 6. Canonical schemas

## 6.1 Word-level transcript

```json
{
  "schema_version": 1,
  "provider": "workspace-heardright",
  "source_id": "cam-a-001",
  "language": "en",
  "words": [
    {
      "id": "w_000001",
      "text": "Today",
      "start_ms": 812,
      "end_ms": 1084,
      "confidence": 0.98,
      "speaker": "S0",
      "kind": "word"
    }
  ],
  "events": [
    {
      "type": "laughter",
      "start_ms": 9400,
      "end_ms": 10100,
      "confidence": 0.73
    }
  ]
}
```

Never make SRT the canonical representation. SRT and ASS are delivery formats generated from this schema.

## 6.2 VAD signal

```json
{
  "schema_version": 1,
  "source_id": "cam-a-001",
  "sample_rate": 16000,
  "provider": "silero-onnx",
  "regions": [
    {
      "start_ms": 780,
      "end_ms": 4290,
      "mean_probability": 0.93
    }
  ]
}
```

## 6.3 Visual evidence

```json
{
  "schema_version": 1,
  "source_id": "cam-a-001",
  "samples": [
    {
      "time_ms": 8400,
      "frame_path": "cache/frames/cam-a-001/000008400.jpg",
      "reason": ["scene_boundary", "phrase_payoff"],
      "faces": [{"box": [0.33, 0.12, 0.29, 0.55], "confidence": 0.99}],
      "hands": [{"wrist": [0.74, 0.57], "confidence": 0.91}],
      "text_regions": [],
      "saliency_regions": [{"box": [0.29, 0.08, 0.38, 0.68], "score": 0.87}],
      "safe_regions": [
        {"anchor": "top-left", "score": 0.78},
        {"anchor": "top-right", "score": 0.22}
      ]
    }
  ]
}
```

All boxes use normalised top-left coordinates. Record the coordinate convention in the schema and test it across rotation and aspect changes.

## 6.4 Timeline

```json
{
  "schema_version": 1,
  "timebase": {"fps_num": 30000, "fps_den": 1001},
  "tracks": [
    {
      "id": "video-main",
      "type": "video",
      "segments": [
        {
          "id": "seg-001",
          "source_id": "cam-a-001",
          "source_start_ms": 812,
          "source_end_ms": 6730,
          "output_start_ms": 0,
          "output_end_ms": 5918,
          "speed": 1.0,
          "reason": "strongest hook take"
        }
      ]
    }
  ]
}
```

Every downstream timestamp is derived from this map.

## 6.5 Finish plan

```json
{
  "schema_version": 1,
  "base_timeline": "edit/timeline.json",
  "slots": [
    {
      "id": "slot-001",
      "kind": "caption",
      "renderer": "remotion",
      "effect_id": "caption.bold-karaoke.v1",
      "output_start_ms": 0,
      "output_end_ms": 5918,
      "anchor": "bottom-center",
      "collision_policy": "avoid-subject-and-platform-ui",
      "props": {"style": "vertical-energy"}
    },
    {
      "id": "slot-002",
      "kind": "cutaway",
      "renderer": "hyperframes",
      "effect_id": "stat.cost-counter.v1",
      "output_start_ms": 8100,
      "output_end_ms": 11200,
      "props": {"amount": 600, "currency": "USD"}
    }
  ]
}
```

## 6.6 Provider response envelope

Every cloud or local model response is stored unchanged and normalised separately:

```json
{
  "provider": "gemini",
  "provider_model": "resolved-at-runtime",
  "request_hash": "blake3:...",
  "created_at": "ISO-8601",
  "cost": {"currency": "USD", "estimated": null},
  "raw_response_path": "cache/provider-responses/...",
  "normalised_output_path": "analysis/cloud-analysis/...",
  "warnings": []
}
```

This prevents vendor format changes from corrupting the canonical project state.

---

# 7. Agent and skill architecture

## 7.1 Only one installable video skill

```text
content/
└── specialists/
    └── video-editor/
        ├── SKILL.md
        ├── workflows/
        │   ├── ingest.md
        │   ├── rough-cut.md
        │   ├── shorts.md
        │   ├── finish.md
        │   ├── review.md
        │   └── export.md
        ├── references/
        └── schemas/
```

Internal workflow documents must not carry installable skill front matter. This avoids the accidental standalone skill cards seen in the previous package.

## 7.2 Ownership boundaries

| Concern | Owner |
|---|---|
| Media sources, timecodes, edit decisions, render and QA | Video Editor |
| Platform, audience, hook, target length, CTA and packaging | Social |
| New script, rewrite, narration, onscreen wording | Writing |
| Styleframes, static layouts, thumbnails and visual system | Designer |
| Cinematic motion language and new signature motion sequences | Motion |
| Correct Remotion implementation | Existing Remotion specialist |
| Correct HyperFrames implementation | HyperFrames’s installed skills |
| Product-demo recording | Separate demo-recording system |

The existing Content skill remains the top-level media-production router. Its contract already requires brand loading, guarded provider execution, smoke checks, reviewed output and Adrian’s visual approval.

## 7.3 Handoffs

Each specialist call writes a typed handoff record:

```json
{
  "from": "video-editor",
  "to": "social",
  "purpose": "Define YouTube and vertical delivery constraints",
  "may_change": ["platform", "target_duration", "hook_goal", "cta"],
  "locked": ["sources", "edit/timeline.json"],
  "expected_outputs": ["brief/platform-brief.json"]
}
```

No specialist may silently modify another specialist’s locked files.

## 7.4 What Claude Code or Codex does

The agent may:

- inspect project state;
- read packed transcripts and selected visual evidence;
- propose editorial strategy;
- select best takes;
- identify distinct short candidates;
- write `cut-plan.json`;
- write `finish-plan.json`;
- select effects from the registry;
- ask for approval at configured gates;
- interpret QA findings and revise plans.

The agent may not:

- issue arbitrary destructive FFmpeg commands;
- change source files;
- upload footage without explicit project permission;
- invent unregistered provider calls;
- bypass schema validation;
- mark visual QA as passed without evidence.

---

# 8. Command-line contract

The root skill should call a stable CLI:

```text
videoctl doctor
videoctl project init <folder>
videoctl ingest <project> <sources...>
videoctl transcribe <project> [--provider auto]
videoctl analyze local <project>
videoctl analyze cloud <project> --provider gemini|twelvelabs
videoctl evidence build <project>
videoctl edit candidates <project>
videoctl edit validate <project>
videoctl edit render <project> --variant tight|natural
videoctl review open <project>
videoctl transcript remap <project>
videoctl shorts propose <project> --count 4
videoctl finish validate <project>
videoctl slot render <project> <slot-id>
videoctl render preview <project>
videoctl qa run <project>
videoctl render final <project> --preset youtube|reel|tiktok
videoctl package social <project>
videoctl export otio <project>
```

All commands:

- accept JSON input or project paths;
- emit machine-readable JSON events to stdout;
- emit logs to stderr;
- have stable exit codes;
- are idempotent where possible;
- use content hashes for caching;
- support `--dry-run`;
- never prompt interactively when invoked by an agent.

## 8.1 Do not expose arbitrary shell through MCP

A later MCP wrapper should expose coarse tools:

```text
inspect_project
run_ingest
run_transcription
build_edit_candidates
render_variant
run_qa
```

It should not expose `execute_ffmpeg(command: string)`.

---

# 9. Local analysis pipeline

## 9.1 Ingest

For every source:

- BLAKE3 hash;
- FFprobe stream metadata;
- duration;
- rotation;
- frame-rate rational;
- variable-frame-rate detection;
- codec and pixel format;
- colour primaries, transfer and matrix;
- audio format and channels;
- start time;
- camera/device metadata where available.

Create:

- 16 kHz mono analysis WAV;
- low-resolution intraframe or H.264 proxy;
- waveform peaks;
- thumbnails;
- scene-score stream;
- no modified source.

## 9.2 Speech analysis

1. Run VAD on the analysis WAV.
2. Run the selected transcriber on the original-time audio.
3. Preserve fillers and false starts.
4. Apply per-video keyterms before transcription.
5. Normalise provider output into the word schema.
6. Generate a compact phrase view split on speaker changes and meaningful pauses.
7. Cache based on source hash, provider and provider version.

## 9.3 Audio features

Compute deterministic features:

- RMS;
- true peak;
- integrated loudness;
- silence candidates;
- spectral centroid;
- pitch contour where useful;
- clipping;
- noise-floor estimate;
- speech rate;
- pause lengths.

Use these as evidence, not an automatic “emotion score”.

## 9.4 Adaptive frame sampling

Do not sample every frame and do not rely on one frame per second.

Extract frames at:

- scene boundaries;
- start and end of transcript phrases;
- detected high-motion intervals;
- candidate cut boundaries;
- major emphasis words;
- user-marked points;
- a low-rate background cadence for coverage.

This is the same general efficiency principle as transcript-first systems: use text and signals to decide where visual evidence is worth paying for.

## 9.5 Native spatial analysis

Run Apple Vision over selected frames and denser windows only when needed:

- subject tracks;
- face tracks;
- hand tracks;
- OCR;
- attention saliency;
- objectness saliency.

Smooth detections over time and store confidence. A one-frame detection must not directly drive a crop or graphic placement.

## 9.6 Visual-quality signals

Local deterministic checks:

- blurred frame score;
- exposure extremes;
- duplicate/frozen frames;
- black frames;
- abrupt colour changes;
- camera shake proxy;
- jump distance at proposed cuts.

These are flags for the editor, not automatic deletion rules.

---

# 10. Editorial engine

## 10.1 Integrate `video-use` as an internal rough-cut engine

Do not run it as a competing top-level skill. Adapt its strongest implemented parts:

- transcript packing;
- take selection brief;
- word-boundary cuts;
- cut padding;
- cached transcripts;
- cut-boundary filmstrips and waveforms;
- output-timeline subtitle mapping;
- rendered-output evaluation;
- project memory principles.

`video-use` is MIT licensed and explicitly supports Claude Code, Codex and other shell-capable agents.

The Rust engine should own the canonical project layout and call adapted helpers or reimplement small deterministic parts. Keep upstream provenance and pin a tested commit if code is vendored.

## 10.2 Rough-cut reasoning

The agent receives:

- platform brief;
- packed transcript;
- source metadata;
- VAD gaps;
- audio features;
- selected filmstrips;
- visual-quality flags;
- user editing profile.

It produces:

```text
- selected narrative structure;
- best take per beat;
- source ranges;
- cut categories;
- confidence;
- ambiguity flags;
- tight and natural timing variants.
```

### Automatic removals

High-confidence:

- abandoned false starts with a complete replacement;
- explicit duplicate takes;
- long dead air;
- clearly isolated filler;
- slate/setup material;
- camera-start/stop handling.

### Suggest-only removals

Require review until calibrated:

- repeated ideas;
- tangents;
- “fluff”;
- jokes;
- emotional pauses;
- uncertainty;
- personal asides.

## 10.3 Boundary logic

Cuts snap to words but are expanded using:

- VAD;
- waveform decay;
- phoneme/word confidence;
- configurable head and tail margins;
- visual discontinuity.

Do not directly crossfade two speech segments if that overlaps words. Use:

- short fades to prevent clicks;
- optional room-tone beds;
- J/L cuts only when intentional;
- crossfades for ambient/audio continuity, not as a blanket rule.

## 10.4 Two default variants

### `tight`

- stronger removal of internal pauses;
- intended for Reels/TikTok/Shorts;
- faster hook;
- more visual reinforcement.

### `natural`

- preserves breaths and section endings;
- intended for YouTube;
- avoids “machine-gun” speech;
- keeps reactions and emphasis.

Render both cheaply before final approval unless the user has already selected a persistent preference.

---

# 11. Short-form extraction

Borrow the concept—not necessarily source code—from systems such as `claude-shorts`:

1. Segment the long transcript into semantic units.
2. Generate candidate standalone narratives.
3. Score each candidate.
4. Enforce diversity.
5. Render low-cost previews.
6. Let the user approve or let a configured auto-mode select.

Candidate score:

```text
standalone completeness
hook specificity
payoff strength
proof/example presence
novelty versus other candidates
emotional or practical value
visual support availability
length fit
brand relevance
platform fit
```

Diversity is mandatory. Four clips that paraphrase the same idea are one clip, not four.

A short can reorder source segments only when:

- meaning remains truthful;
- chronology is not falsely implied;
- transitions remain coherent;
- the plan records the reorder.

The Social skill supplies current platform constraints, hook expectation, CTA and packaging. It should not choose source cut points.

---

# 12. Graphics and finishing

## 12.1 Finish-plan generation

After rough cut approval and transcript remapping, the agent identifies **visual opportunities**, not mandatory effects.

Every proposed visual must answer:

- What information does it clarify?
- What emotion or emphasis does it support?
- Why is this timing correct?
- Why is this renderer appropriate?
- Does it cover the speaker, captions or platform UI?
- Is real evidence available instead of an invented graphic?

The Motion skill’s core principles—motion serves meaning, persistent objects, hierarchy, one motion language and restraint—should be adapted for cinematic use. Its web-interaction checks such as CLS, hydration and keyboard focus do not apply to rendered video.

## 12.2 Effect registry

```text
effects/
├── registry.json
├── remotion/
├── hyperframes/
├── ass/
├── lottie/
├── media/
└── previews/
```

Each effect:

```json
{
  "id": "stat.counter.clean-v1",
  "renderer": "remotion",
  "category": "stat",
  "description": "Single-value counter with label",
  "semantic_triggers": ["cost", "revenue", "time saved", "percentage"],
  "supported_aspects": ["16:9", "9:16", "1:1"],
  "safe_anchors": ["top-left", "top-right", "center"],
  "minimum_duration_ms": 1800,
  "maximum_text_chars": 42,
  "schema": "schemas/effects/stat-counter.json",
  "preview": "previews/stat-counter-clean-v1.mp4",
  "source": "original",
  "licence": "workspace-owned",
  "verified_versions": {
    "remotion": "pinned-version"
  },
  "feedback": {
    "selected": 0,
    "rejected": 0
  }
}
```

The initial implementation needs a small, good library—not fifty generic effects.

Recommended first 15:

1. clean phrase caption;
2. bold karaoke caption;
3. minimal YouTube caption;
4. name/title lower third;
5. number/stat counter;
6. price/cost breakdown;
7. quote card;
8. three-step list;
9. comparison split;
10. browser window;
11. terminal/code window;
12. image/screenshot spotlight;
13. timeline/progress marker;
14. CTA/end card;
15. clean transition/flash overlay.

## 12.3 Remotion implementation

The current Remotion knowledge base remains authoritative for:

- `useCurrentFrame()`-driven animation;
- sequences and transitions;
- Zod parameter schemas;
- caption types;
- text measurement;
- fonts;
- transparent ProRes/WebM;
- charts, maps, Lottie and 3D;
- audio and SFX.

Build one stable renderer package:

```text
renderers/remotion/
├── package.json
├── src/
│   ├── Root.tsx
│   ├── brand/
│   ├── effects/
│   ├── captions/
│   └── compositions/
├── public/
├── scripts/
│   ├── render-slot.ts
│   ├── render-still.ts
│   └── verify.ts
└── tests/
```

Every composition accepts typed props and can render a still for review before rendering the full clip.

## 12.4 HyperFrames implementation

Do not fork the entire project.

Use the installed CLI and skills from isolated slot directories:

```text
finish/slots/slot-004/
├── brief.json
├── index.html
├── assets/
├── meta.json
├── render.mp4
└── qa.json
```

Required loop:

```text
init
→ author
→ lint
→ validate/check
→ snapshot
→ draft render
→ inspect key frames
→ final render
```

Promote a successful recurring HyperFrames visual to the permanent Remotion library only when reuse justifies it.

## 12.5 Captions

Canonical flow:

```text
word transcript on source timeline
→ approved edit
→ map words to output timeline
→ punctuation and chunking
→ platform safe-zone calculation
→ renderer selection
→ rendered caption track
→ OCR/timing/collision QA
```

Default routing:

| Need | Engine |
|---|---|
| Fast fixed karaoke/phrase caption | ASS |
| Reusable branded kinetic caption | Remotion |
| Bespoke editorial typography | HyperFrames |
| Caption sidecar | SRT/VTT export |

Caption profiles:

```text
youtube-clean
youtube-minimal
vertical-karaoke
vertical-phrase
quote-emphasis
multispeaker
```

Each profile defines:

- chunking;
- line count;
- characters per line;
- reading speed;
- punctuation behaviour;
- active-word behaviour;
- font;
- margins;
- safe areas;
- collision fallback.

## 12.6 Spatial placement

For every graphic slot, calculate candidate placements over the entire slot interval—not one frame.

Cost function:

```text
subject overlap
+ face overlap
+ hand/gesture overlap
+ existing text overlap
+ caption overlap
+ platform-UI overlap
+ saliency overlap
+ excessive edge proximity
+ temporal jitter
```

Choose the lowest-cost stable anchor. If every anchor is poor:

1. use a full-screen cutaway;
2. delay the graphic;
3. reduce or remove it;
4. ask for human placement.

Do not shrink important text until it is unreadable merely to satisfy collision avoidance.

---

# 13. Audio finishing

Pipeline:

1. analyse source and rough cut;
2. denoise only when needed;
3. high-pass/low-cut where appropriate;
4. correct obvious resonances cautiously;
5. dialogue compression;
6. de-essing if required;
7. true-peak limiting;
8. loudness normalisation;
9. room-tone continuity at cuts;
10. music and SFX with sidechain/automation.

Keep raw dialogue and processed dialogue as separate cached assets.

SFX rules:

- functional, not decorative;
- one sound per meaningful visual event;
- no automatic “whoosh on every transition”;
- use an approved local library with licence/provenance;
- snap to the visual landing or spoken payoff;
- control density through the active style profile.

Music rules:

- optional;
- dialogue remains primary;
- duck under speech;
- avoid editing every cut to a beat unless the content style calls for it;
- log the music licence.

The existing Remotion SFX/audio rules can render audio layers, but the final dialogue processing and loudness gate should be deterministic in the media engine.

---

# 14. Colour pipeline

Separate technical correction from creative look:

```text
camera/log/input transform
→ exposure and white-balance correction
→ shot matching
→ optional creative look
→ output transform
```

Do not apply `cinematic.cube` blindly to every input.

Project configuration records:

- camera profile;
- input colour space;
- output colour space;
- HDR/SDR policy;
- technical LUT;
- creative LUT;
- creative strength;
- skin-tone protection preference.

For mixed iPhone HDR and SDR footage, explicitly tone-map into a defined working/output space before applying the look.

Initial implementation:

- FFprobe colour metadata;
- FFmpeg colour transforms;
- reference-frame matching heuristics;
- a small approved LUT set;
- human review frame grid.

Later:

- Core Image/Metal colour sidecar for better Mac-native processing;
- camera-profile-specific transforms.

---

# 15. Optional API adapters

## 15.1 Common Rust trait

```rust
#[async_trait]
pub trait VisualAnalyzer {
    fn id(&self) -> &'static str;
    async fn analyze(
        &self,
        request: VisualAnalysisRequest,
    ) -> anyhow::Result<ProviderEnvelope<VisualAnalysisResult>>;
}
```

Equivalent traits:

```text
TranscriptionProvider
VisualAnalyzer
VideoSegmenter
AssetSearchProvider
ImageGenerationProvider
TtsProvider
EngagementExperimentProvider
```

The business logic depends on normalised outputs, never vendor SDK types.

## 15.2 Gemini adapter

Best for:

- one video;
- structured semantic plan;
- selected-clip questions;
- OCR/layout review;
- final preview audit.

Send:

- proxy or approved upload asset;
- authoritative transcript;
- selected stills at higher resolution when OCR matters;
- strict schema;
- explicit instruction not to re-transcribe or change timestamps.

Cache the response by video hash, prompt version, schema version and model.

## 15.3 Twelve Labs adapter

Best for:

- long videos;
- timestamped segmentation;
- recurring archive;
- cross-video B-roll search;
- multiple analysis passes without repeated upload.

Keep indexing and analysis as separate capabilities. The application should not assume a specific retired Marengo version.

## 15.4 API privacy and budget

Per project:

```yaml
cloud:
  allowed: false
  providers: []
  upload_asset: proxy
  retain_remote_asset: false
  max_budget_usd: 5.00
  redact_before_upload: true
```

Requirements:

- explicit first-use consent;
- provider-specific deletion flow;
- key storage in macOS Keychain;
- no keys in project JSON;
- upload only proxy/clip when sufficient;
- record estimated and actual cost;
- stop at budget;
- no automatic retries that duplicate expensive jobs.

## 15.5 Local fallback is always defined

A provider failure must degrade to:

```text
local transcript
+ adaptive frame samples
+ Apple Vision signals
+ agent reasoning
+ human review
```

It may reduce automation quality but must not make the project unusable.

---

# 16. Review UI

Do not build a full NLE. Build the minimum direct-manipulation surface that closes agent errors.

## 16.1 Rough-cut workspace

- video preview;
- waveform;
- transcript with kept/cut styling;
- source and output timecodes;
- draggable boundaries;
- candidate cut flags;
- tight/natural comparison;
- undo/version history;
- comments.

## 16.2 Finish workspace

- timeline slots;
- caption preview;
- stills at key cue points;
- three effect alternatives where requested;
- placement controls;
- show subject/safe-zone overlays;
- accept/reject with reason;
- render changed prefix only.

## 16.3 QA workspace

- cut-boundary filmstrips;
- waveform spikes;
- caption collisions;
- text overflow;
- subject occlusion;
- audio/loudness flags;
- colour mismatch grid;
- A/V duration mismatch;
- platform-safe-zone preview.

The Tauri UI should emit structured decisions. Every adjustment updates JSON rather than becoming hidden state in the UI.

---

# 17. QA system

## 17.1 Machine gates

### Media

- file decodes;
- expected streams exist;
- duration matches plan;
- A/V durations remain within tolerance;
- constant delivery frame rate;
- expected resolution and aspect;
- no accidental HDR metadata;
- no black/frozen tail;
- no missing audio.

### Editorial

- cuts align with planned words;
- no intended word is missing;
- second-ASR comparison flags extra/ghost speech;
- no unexplained long silence;
- no segment shorter than configured minimum;
- no duplicate narrative beat unless intentional.

### Visual

- inspect both sides of every cut;
- no flash or frozen frame;
- no graphic outside frame;
- no text overflow;
- no subject/caption collision;
- no unapproved effect;
- no low-resolution asset.

### Captions

- output-timeline alignment;
- reading speed;
- line count;
- platform-safe placement;
- punctuation;
- font availability;
- no hidden captions under overlays.

### Audio

- integrated loudness target;
- true peak;
- clipped samples;
- cut pops;
- dialogue/music balance;
- abrupt noise-floor changes.

## 17.2 Model-assisted QA

Optional Gemini/Twelve analysis can flag:

- semantic mismatch between speech and visual;
- misleading B-roll;
- inappropriate or repetitive graphics;
- obvious text errors;
- unexplained jumps;
- poor opening clarity.

Model findings are suggestions. Deterministic failures and human review remain authoritative.

## 17.3 Human gate modes

```text
reviewed:
  approve rough cut
  approve finish strategy
  approve final

review-light:
  approve rough cut
  receive final with QA report

autonomous:
  no intermediate approval
  preserve all variants
  never overwrite last approved output
```

Default the first five real projects to `reviewed`. Move to lighter modes only after preference calibration.

---

# 18. Preference learning

Do not start with a generic “engagement model”.

Log actual decisions:

```json
{
  "type": "effect_rejection",
  "project_id": "p1",
  "slot_id": "slot-4",
  "effect_id": "stat.counter.loud-v1",
  "reason": "too busy for explanatory section",
  "replacement": "stat.counter.clean-v1"
}
```

Learn:

- preferred pause lengths;
- filler policy;
- effect density;
- caption style;
- B-roll frequency;
- shot duration;
- preferred anchors;
- music/SFX taste;
- accepted hook structures;
- preferred final length.

After publication, Social can import platform analytics. Link retention changes to output timeline moments, but do not claim causal certainty from one video.

TRIBE remains separate from this preference system.

---

# 19. Repository implementation target

```text
content-video-engine/
├── Cargo.toml
├── crates/
│   ├── video-core/
│   ├── video-cli/
│   ├── video-project/
│   ├── video-timeline/
│   ├── video-media/
│   ├── video-providers/
│   ├── video-render/
│   ├── video-qa/
│   └── video-mcp/              # later
├── apps/
│   └── studio/
│       ├── src-tauri/
│       └── src/
├── sidecars/
│   ├── vision-mac/
│   ├── model-worker/
│   ├── remotion-renderer/
│   └── hyperframes-runner/
├── skills/
│   └── content-video-editor/
├── schemas/
├── effects/
├── fixtures/
├── tests/
├── docs/
└── scripts/
```

## 19.1 Recommended Rust dependencies

```text
serde / serde_json
schemars + jsonschema
clap
tokio
tracing
anyhow / thiserror
blake3
uuid
chrono
reqwest
sqlx or rusqlite
notify
tempfile
camino
which
keyring
```

Use `tokio::process::Command` for FFmpeg and sidecars. Do not bind directly to libav in v1; the CLI boundary is easier to update, reproduce and debug.

## 19.2 Job graph

Every operation is a job:

```json
{
  "job_id": "job-...",
  "kind": "render_slot",
  "inputs_hash": "blake3:...",
  "status": "queued",
  "progress": 0.0,
  "attempt": 1,
  "cost_usd": 0,
  "artifacts": []
}
```

Jobs are resumable and content-addressed. Restarting the app must not lose progress or rerun completed API calls.

---

# 20. Implementation phases and acceptance gates

## Phase 0 — architecture freeze

Deliver:

- schemas;
- one root skill;
- CLI command contract;
- immutable-source policy;
- provider traits;
- test fixture format.

Gate:

- schema round-trip tests;
- timestamp arithmetic tests;
- project migrations;
- no nested installable skill front matter.

## Phase 1 — local ingest and transcription

Deliver:

- `videoctl project init`;
- FFprobe ingest;
- hashes;
- proxies and waveform;
- Silero ONNX VAD;
- workspace transcription adapter;
- Whisper/Parakeet sidecar interface;
- packed transcript.

Gate:

- two camera/iPhone formats;
- variable frame rate;
- rotation;
- HDR metadata;
- cached rerun;
- word timestamps remain on source timebase.

## Phase 2 — working rough cut

Deliver:

- adapted `video-use` reasoning workflow;
- candidate generation;
- cut-plan validator;
- tight/natural render;
- output transcript remapping;
- boundary filmstrips;
- second-ASR verification hook.

Gate:

- real 20-minute multi-take recording;
- no clipped words;
- no A/V drift accumulation;
- every cut traceable to source words;
- preview can be regenerated after a boundary edit.

This is the first genuinely useful milestone.

## Phase 3 — Tauri review surface

Deliver:

- project browser;
- transcript/timeline review;
- waveform;
- filmstrip;
- boundary drag;
- variant comparison;
- version history.

Gate:

- all edits persist into canonical JSON;
- reload restores state;
- undo does not mutate source;
- no manual JSON editing required.

## Phase 4 — captions, audio and direct final export

Deliver:

- canonical captions;
- ASS renderer;
- first Remotion caption components;
- dialogue processing;
- music/SFX plan;
- colour profiles;
- YouTube and vertical export presets;
- QA report.

Gate:

- 16:9 and 9:16;
- caption timing and safe zones;
- target loudness;
- HDR-to-SDR fixture;
- final file accepted by QuickTime and platform upload checks.

## Phase 5 — graphics backends and starter library

Deliver:

- pinned Remotion project;
- HyperFrames slot runner;
- 15 starter effects;
- effect catalog;
- still/key-frame QA;
- alpha overlay composition.

Gate:

- each effect renders in 16:9 and 9:16 where declared;
- props validated;
- fonts loaded;
- text overflow tests;
- deterministic re-render;
- provenance present.

## Phase 6 — short extraction

Deliver:

- candidate semantic segmentation;
- candidate scoring;
- diversity constraint;
- vertical reframe;
- four-preview batch;
- social handoff.

Gate:

- four clips from one long source are materially different;
- each stands alone;
- no false chronology;
- captions and crops pass QA.

## Phase 7 — local visual perception

Deliver:

- Swift Vision sidecar;
- face/body/hand/OCR/saliency;
- temporal tracks;
- safe-region scoring;
- caption/graphic collision checks;
- crop-path planner.

Gate:

- coordinate fixtures;
- multi-subject footage;
- gesture crossing frame;
- vertical crop stability;
- no anchor jitter.

## Phase 8 — optional cloud providers

Deliver:

- Gemini adapter;
- Twelve Labs adapter;
- consent and budgets;
- response cache;
- deletion;
- direct API and optional MCP documentation.

Gate:

- offline project remains functional;
- provider outage fallback;
- strict schema rejection/retry;
- budget stops;
- no duplicate upload;
- raw response retained for audit.

## Phase 9 — preference and analytics loop

Deliver:

- feedback ranking;
- per-style profile learning;
- Social analytics import;
- experiment reports.

Gate:

- recommendations cite actual prior decisions;
- analytics are not presented as causal proof;
- no TRIBE dependency.

---

# 21. Real-footage benchmark

Before calling the system production-ready, evaluate on a fixed private suite:

```text
3 × YouTube talking-head projects, 10–25 min raw
2 × multi-take videos with many false starts
3 × direct vertical recordings
2 × long recordings used to extract 4 shorts each
1 × iPhone HDR source
1 × noisy room
1 × mixed frame-rate/source project
```

Measure:

- editor acceptance of selected takes;
- boundary correction rate;
- clipped/ghost word count;
- rough-cut time;
- final-render time;
- caption timing errors;
- caption/subject collisions;
- graphic acceptance rate;
- number of manual placement fixes;
- final length;
- provider cost;
- cache hit rate;
- crash/recovery behaviour.

A feature is not “done” because it produced an MP4. It is done when the output passes the benchmark and Adrian accepts the visual result.

---

# 22. Reuse and licensing policy

## Directly integrate when licence permits

- `video-use`: MIT; adapt the rough-cut implementation and preserve notices.
- HyperFrames: Apache 2.0; use as an external pinned renderer.
- Silero VAD: MIT.
- `claude-youtube-editor`: MIT according to its repository; selectively adapt useful deterministic helpers after code review.
- Auto-Editor: repository code is public-domain/Unlicense, but recent releases may include licence-key behaviour for some capabilities; pin and audit the exact version before depending on it.

## Use as architectural references unless separately audited

- YUV effect workflow;
- `claude-shorts`;
- ButterCut;
- RoughCut;
- commercial After Effects packs;
- creator-provided Skool files.

Do not recreate or redistribute paid motion packs component-for-component without permission.

## Remotion

Remotion is appropriate for Adrian’s local creator workflow, but it uses a custom licence. Its current pricing page states that individuals and companies up to three people have a free licence, while larger teams and video-automation products have paid conditions. Record the pinned version and recheck the licence before distributing a commercial editing product.

---

# 23. Final recommended configuration

```yaml
project:
  kind: mixed_creator_content
  outputs: [youtube, reels, tiktok]
  review_mode: reviewed
  source_policy: immutable

local:
  vad: silero_onnx
  transcription: workspace
  transcription_fallback: whisper
  visual_perception: apple_vision
  media_engine: ffmpeg
  review_ui: tauri

editorial:
  engine: video_use_adapter
  variants: [natural, tight]
  preserve_fillers_in_transcript: true
  auto_remove_fluff: false

graphics:
  reusable_renderer: remotion
  bespoke_renderer: hyperframes
  simple_captions: ass
  effect_policy: library_first
  generated_media_default: false

cloud:
  allowed: false
  visual_analyzer: none
  semantic_archive: none
  max_budget_usd: 0

experimental:
  tribe_enabled: false

delivery:
  youtube:
    aspect: "16:9"
    captions: sidecar_and_optional_burned
  reels:
    aspect: "9:16"
    captions: burned
  tiktok:
    aspect: "9:16"
    captions: burned
```

When cloud assistance is enabled:

```yaml
cloud:
  allowed: true
  visual_analyzer: gemini
  semantic_archive: twelvelabs
  max_budget_usd: 5
  upload_asset: proxy
  retain_remote_asset: false
```

---

# 24. Definition of success

The implementation is successful when this prompt works from Claude Code or Codex:

```text
Edit the footage in this folder.

Create:
1. One natural-paced 8–12 minute YouTube video.
2. Four materially different vertical clips.
3. Clean captions and restrained graphics in my saved style.
4. Final MP4s and a QA report.

Keep sources untouched.
Use local processing unless I approve a cloud analysis request.
Show me the natural and tight rough cuts before finishing.
```

The agent should then:

1. initialise the project;
2. ingest and analyse locally;
3. present an editorial strategy;
4. render two rough-cut variants;
5. open the Tauri review surface;
6. finish the approved cut;
7. create distinct shorts;
8. render and inspect all outputs;
9. route title/thumbnail/platform packaging through Social, Writing and Designer;
10. deliver versioned MP4s with traceable plans and QA evidence.

That is the correct end-to-end tool: local-first, inspectable, provider-optional, agent-friendly and capable of becoming increasingly hands-off as Adrian’s preferences are learned.

---

# 25. Sources and verification notes

## Workspace sources

- Existing Content router: establishes Content as the owner of media production and routes transcription to the dedicated specialist.
- Existing Remotion guide and rule set: establishes frame-driven animation, typed composition props, captions, transitions, media, transparent output, SFX, charts, maps and other rendering mechanics.
- Existing Motion skill: supplies motion meaning, hierarchy, persistent objects, language and restraint; its interactive web/native QA rules are not copied wholesale into cinematic video.
- Existing Designer, Writing and Social skills: establish the cross-skill ownership boundaries used above.

## Current external primary sources

1. Browser Use, **video-use** repository and skill documentation — MIT, agent-portable transcript-led editing workflow.
2. HeyGen, **HyperFrames** repository and documentation — Apache 2.0, deterministic HTML video rendering, agent skills and catalog.
3. Remotion official documentation and pricing/licence pages — React video rendering, agent skills and current licensing conditions.
4. Silero Team, **silero-vad** repository — ONNX/PyTorch VAD and MIT licence.
5. NVIDIA Parakeet TDT model cards and NeMo documentation — timestamp support and Apple M-series MPS inference guidance.
6. Google AI, Gemini API **Video understanding** and **Structured outputs** documentation.
7. Twelve Labs, **Pegasus 1.5 Analyze API**, segmentation, release notes and MCP documentation.
8. Meta FAIR, **TRIBE v2** repository — brain-response model purpose, five-second lag and CC BY-NC 4.0 licence.
9. Apple Developer Documentation, **Vision**, hand-pose, OCR and saliency APIs.
10. Apple Developer Documentation, **VideoToolbox**.
11. Tauri v2 documentation, **Embedding External Binaries**.
12. Auto-Editor official repository and release notes.
13. FFmpeg source/documentation for VideoToolbox encoders and libass subtitle filters.

All volatile model identifiers, API parameters, prices and licence terms should be resolved and pinned during implementation rather than copied indefinitely from this plan.
