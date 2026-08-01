<img src=".github/banner.svg" alt="CutRight — Agentic video editing on a verified media path." width="100%">

**CutRight is a local headless video-editing pipeline built on a verified media path: Rust owns project state, FFmpeg calls are typed, and every source clip is content-hashed before it can be cut.**

![core: Rust](https://img.shields.io/badge/core-rust-5362d8?style=flat-square&labelColor=111318&color=5362d8) ![cli: videoctl (json-only)](https://img.shields.io/badge/cli-videoctl%20(json--only)-5362d8?style=flat-square&labelColor=111318&color=5362d8)

## What it is

Rust owns canonical project JSON, timestamp arithmetic, typed FFprobe/FFmpeg boundaries, BLAKE3 source registration, and immutable inputs, exposed through the JSON-only `videoctl` CLI. HeardRight's Parakeet TDT CoreML path supplies native timed words, Silero supplies speech probabilities, and WhisperX is available as an independent word-edge verifier. The pipeline produces candidate-driven rough cuts, burned-caption 16:9 and 9:16 MP4s, visual boundary/waveform evidence, explicit-final QA, and YouTube/vertical social packages. Cloud analysis, effect/preset libraries, proxy generation, preference learning, and the Studio's authoring surface are not part of this local pipeline.

## How it works

- **Project state** — `videoctl project init` creates a canonical package layout. It's idempotent and never overwrites an existing manifest or source file.
- **Transcription** — `transcribe --provider heardright` (Parakeet TDT CoreML) and `--provider whisperx` produce timed words independently, so the two can be compared rather than trusted blind.
- **Transcription gate** — `bench transcribe` requires at least three distinct immutable source clips and won't authorize either provider for destructive word-edge cuts without that evidence. Without a resolved HeardRight-versus-WhisperX decision, CutRight refuses to call a final render technically approved.
- **Reframe gate** — vertical delivery is blocked until `reframe plan` produces a human-reviewed plan with the top-level `approved` flag and every anchor's `approved` flag set to `true`. CutRight will not silently center-crop a 16:9 rough cut into a vertical final.
- **Evidence and QA** — `evidence build` and `qa` generate the waveform/boundary-frame evidence and explicit-final QA pass before a render counts as approved.
- The local E2E smoke fixture verifies Parakeet and WhisperX timed words, Silero VAD regions, candidate-driven rough cuts, captioned YouTube/reels MP4s, waveform plus boundary-frame evidence, explicit-final QA, and both social packages under one project directory.

## Quick start

```bash
cargo test --workspace
cargo run -p videoctl -- doctor
cargo run -p videoctl -- project init /path/to/MyVideo.video-project
cargo run -p videoctl -- ingest /path/to/MyVideo.video-project /path/to/clip.mp4
cargo run -p videoctl -- transcribe /path/to/MyVideo.video-project --provider heardright
cargo run -p videoctl -- transcribe /path/to/MyVideo.video-project --provider whisperx
cargo run -p videoctl -- analyze local /path/to/MyVideo.video-project
cargo run -p videoctl -- edit candidates /path/to/MyVideo.video-project
cargo run -p videoctl -- edit render /path/to/MyVideo.video-project --variant natural
cargo run -p videoctl -- transcript remap /path/to/MyVideo.video-project
cargo run -p videoctl -- bench transcribe /path/to/MyVideo.video-project
cargo run -p videoctl -- render final /path/to/MyVideo.video-project --preset youtube
cargo run -p videoctl -- reframe plan /path/to/MyVideo.video-project
# Review and explicitly approve every anchor in analysis/reframe-plan.json.
cargo run -p videoctl -- render final /path/to/MyVideo.video-project --preset reels
cargo run -p videoctl -- evidence build /path/to/MyVideo.video-project
cargo run -p videoctl -- qa /path/to/MyVideo.video-project
cargo run -p videoctl -- package social /path/to/MyVideo.video-project
```

Set `CUTRIGHT_HEARDRIGHT_ENGINE` and `CUTRIGHT_HEARDRIGHT_MODELS_DIR` when HeardRight is not installed at its standard local paths. Rough cuts require macOS `h264_videotoolbox`; HDR input additionally requires an FFmpeg build with `zscale`. Development uses the ignored `.cutright-tools/ffmpeg-zimg` build when present; deploys can set `CUTRIGHT_FFMPEG` to an equivalent executable. See [schemas/](schemas/) and [docs/PHASE-1-TRANSCRIPTION-BENCHMARK.md](docs/PHASE-1-TRANSCRIPTION-BENCHMARK.md).

## Bridge-period short-form skills

`cutaway/` and `finish/` are two Claude Code skills that turn a raw 9:16 talking-head clip into a finished short, ahead of the CutRight control plane covering that ground natively:

1. **`cutaway/`** — the rough cut. WhisperX forced alignment gives the exact start/end of every word, so the cut can land on real word edges, remove only the no-word gaps, and arrange the best takes into one story (hook → … → CTA). Output is a cut list plus a matching MP4.
2. **`finish/`** — the styling pass: reframe to 9:16, animated zooms/punch-ins, lens effects, an exponential text fade-in, the "authority stack" text look, captions, and SFX timed to motion.

They run in order: cutaway locks the cut, finish styles it.

### Install

```bash
cp -R cutaway ~/.claude/skills/shortform-cutaway
cp -R finish  ~/.claude/skills/shortform-finish
```

In Claude Code, hand it a clip and say what you want — e.g. "make the rough cut, remove the silences" (cutaway) or, once the cut is locked, "finish this short, add the hook zoom" (finish). The skill descriptions trigger automatically.

Cutaway needs WhisperX in a Python 3.11 venv; its `## SETUP` section walks through it (`python3.11 -m venv ~/wx-env && ~/wx-env/bin/pip install whisperx`). The cut scripts take all paths as arguments, so there's nothing else to configure.

### First run — which editor do you use?

On the first run, each skill asks which editor you edit in and follows the matching branch:

- **DaVinci Resolve** — the cut lands as a timeline; zooms are built as Fusion comps.
- **Premiere Pro** — the cut lands as an EDL/XML; zooms/text via keyframes and Essential Graphics.
- **Remotion** — everything is code; the cut is a playlist of segments, effects are `interpolate` curves.
- **Claude-Code-only (no NLE)** — the cut renders straight to an MP4 with FFmpeg; no other software needed.

No editor installed: pick Claude-Code-only. It produces a finished, postable MP4 on its own.

### A note on the visual style

These skills teach the method, not a pile of finished assets. Cutaway is fully turnkey — it produces the cut. Finish gives you the techniques (zoom curves, exponential fade, authority-stack text look, sound-matches-motion rules) so you recreate the look with your own footage, SFX, and style graphics. Build that small library once; it's yours to keep.

## Status

Bridge period: `cutaway/` and `finish/` are creator skills covering visual styling until the CutRight control plane takes over that path natively. The control plane already owns the reproducible media and timeline path — canonical project state, transcription, candidate cuts, gated reframing, evidence, and QA.

<!-- blueprint:docs:start -->
## Repository truth docs
- [Product overview](docs/product.md) — what this is and does (generated, code-grounded)
- [Architecture](docs/architecture.md) — components, flows, interfaces (generated, code-grounded)
<!-- blueprint:docs:end -->

---

<sub><b><a href="https://orthic-labs.github.io">Orthic Labs</a></b> — local-first infrastructure for AI-assisted development.<br>
<a href="https://github.com/Orthic-Labs/Membrane">Membrane</a> · <a href="https://github.com/Orthic-Labs/Cortex">Cortex</a> · <a href="https://github.com/Orthic-Labs/Sentinel">Sentinel</a> · <a href="https://github.com/Orthic-Labs/Roundtable">Roundtable</a> · <a href="https://github.com/Orthic-Labs/Morph">Morph</a> · <a href="https://github.com/Orthic-Labs/CutRight">CutRight</a> · <a href="https://github.com/Orthic-Labs/claudecodeX">claudecodeX</a></sub>
