---
name: content-video-editor
description: "Route captured-footage editing through CutRight's validated local CLI and canonical project package."
---

# Video editor

Use this specialist for captured-footage post-production. Load `/brand <venture>` before brand work;
Social owns platform packaging and hooks, Writing owns scripts and onscreen wording, Designer owns
thumbnails and static layouts, and Motion owns cinematic motion language.

The Rust `videoctl` CLI is the control plane. Do not compose raw FFmpeg commands, mutate source files,
or upload footage without explicit consent. Read structured project evidence, write validated plans,
and keep every decision reproducible in the project package.

## Workflows

- [Ingest](workflows/ingest.md)
- [Rough cut](workflows/rough-cut.md)
- [Shorts](workflows/shorts.md)
- [Finish](workflows/finish.md)
- [Review](workflows/review.md)
- [Export](workflows/export.md)

## Defaults

- Sources are immutable.
- VAD is a signal on the original timebase, never a destructive pre-edit.
- Local transcription is primary; a second provider verifies word edges before rough-cut release.
- Cloud providers are off by default and budgeted when enabled.
- `reviewed` is the default human gate.
