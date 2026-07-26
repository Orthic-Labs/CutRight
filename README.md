# CutRight — Agentic Video Editing Engine

The local headless pipeline is runnable end to end: Rust owns canonical project JSON, timestamp
arithmetic, typed FFprobe/FFmpeg boundaries, BLAKE3 source registration, immutable inputs, and the
JSON-only `videoctl` CLI. Local transcription routes through HeardRight's locked Parakeet TDT CoreML
sidecar and preserves its native timed words through cut planning, remapping, captions, renders, QA,
social packaging, and OTIO export.

## Local pipeline quick start

```bash
cargo test --workspace
cargo run -p videoctl -- doctor
cargo run -p videoctl -- project init /path/to/MyVideo.video-project
cargo run -p videoctl -- ingest /path/to/MyVideo.video-project /path/to/clip.mp4
cargo run -p videoctl -- transcribe /path/to/MyVideo.video-project --provider heardright
cargo run -p videoctl -- analyze local /path/to/MyVideo.video-project
cargo run -p videoctl -- edit candidates /path/to/MyVideo.video-project
cargo run -p videoctl -- edit render /path/to/MyVideo.video-project --variant natural
cargo run -p videoctl -- transcript remap /path/to/MyVideo.video-project
cargo run -p videoctl -- render final /path/to/MyVideo.video-project --preset youtube
cargo run -p videoctl -- qa /path/to/MyVideo.video-project
```

`videoctl project init` is idempotent, creates the canonical package layout, and never overwrites an
existing manifest or source file. Set `CUTRIGHT_HEARDRIGHT_ENGINE` and
`CUTRIGHT_HEARDRIGHT_MODELS_DIR` when HeardRight is not installed at its standard local paths. See
[schemas/](schemas/), [ARCHITECTURE-2026-07-26.md](ARCHITECTURE-2026-07-26.md), and
[skills/content-video-editor/SKILL.md](skills/content-video-editor/SKILL.md).

The existing `cutaway/` and `finish/` folders remain bridge-period creator skills for visual styling;
the CutRight control plane owns the reproducible media and timeline path.

The local E2E smoke fixture produced a Parakeet TDT transcript, natural and tight rough cuts, a final
MP4, captions, QA report, social package, and OTIO export under one project directory.

## Bridge-period short-form skills

Two Claude Code skills that turn a raw 9:16 talking-head clip into a finished short:

1. **`cutaway/`** — the rough cut. WhisperX forced alignment gives the exact start/end of every word,
   so the AI can cut on real word edges, remove only the silences (the no-word gaps), and arrange the
   best takes into one flowing story (hook → … → CTA). Output is a cut list + a matching MP4.
2. **`finish/`** — the styling pass. Reframe to 9:16, animated zooms / punch-ins, lens effects, an
   exponential text fade-in, the "authority stack" text look, captions, and SFX timed to motion.

They run in order: **cutaway locks the cut → finish styles it.**

## Install

Drop each folder into your Claude Code skills directory:

```bash
cp -R cutaway ~/.claude/skills/shortform-cutaway
cp -R finish  ~/.claude/skills/shortform-finish
```

Then in Claude Code just hand it a clip and say what you want — e.g. *"make the rough cut, remove the
silences"* (cutaway) or, once the cut is locked, *"finish this short, add the hook zoom"* (finish). The
skill descriptions trigger automatically.

**One-time setup for cutaway:** it needs WhisperX in a Python 3.11 venv. The skill's `## SETUP` section
walks you through it (`python3.11 -m venv ~/wx-env && ~/wx-env/bin/pip install whisperx`). That's the only
dependency; the cut scripts take all paths as arguments, so there's nothing to hardcode.

## First run — which editor do you use?

On the first run, each skill asks **which editor you edit in** and follows the matching branch:

- **DaVinci Resolve** — the cut lands as a timeline; zooms are built as Fusion comps.
- **Premiere Pro** — the cut lands as an EDL/XML; zooms/text via keyframes + Essential Graphics.
- **Remotion** — everything is code; the cut is a playlist of segments, effects are `interpolate` curves.
- **Claude-Code-only (no NLE)** — the cut renders straight to an MP4 with ffmpeg; no other software needed.

Don't have an editor? Pick **Claude-Code-only** — it produces a finished, postable MP4 on its own.

## A note on the visual style

These skills teach the **method**, not a pile of finished assets. The cutaway is fully turnkey — it
produces the cut. The finish skill gives you the *techniques* (the zoom curves, the exponential fade, the
authority-stack text look, the sound-matches-motion rules) so you recreate the look with **your own
footage, your own SFX, and your own style graphics**. Build that small library once; it's yours to keep.
