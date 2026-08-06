# Routing

Upstream venture anchors (provenance; NOT vendored into CutRight — they belong to the
GenRight venture workspace and arrive only with its optional signed runtime pack):

- upstream `genright/` + `genright/README.md` (guarded generation pipelines)
- upstream `docs/video-pipeline-guide.md`, `docs/IMAGE-PIPELINE-GUIDE.md`
- upstream `.system/imagegen` static image tool
> **CutRight v2:** hosted generation providers named here (GenRight pipelines, `$imagegen`, HeyGen, hosted TTS) are UNSUPPORTED OPTIONAL capabilities of signed runtime packs — never required paths. In a base CutRight image, report the route as unavailable-offline and defer or use the local alternative. Venture workspace anchors (GenRight repo, pipeline guides) are not vendored; they arrive, when installed, with the corresponding signed pack.


## Core Rule

This skill chooses a path. It does not implement providers. Provider spend only happens through GenRight/current guarded pipeline preflight and approval, or through Codex `$imagegen` when that tool is available for static image generation.

## Runtime Detection

- In Codex, use `$imagegen` for simple static image generation when the image generation tool is available.
- In Claude or no-imagegen contexts, use GenRight Image Studio/current image pipeline for static images.
- For video, route through GenRight Video Studio/current video pipeline.
- For HeyGen, inspect GenRight model metadata or local docs first. Missing, unparsable, or old metadata means hide HeyGen choices and use current alternatives.

## HeyGen Model Keys

Use these names only when present:

- `heygen-digital-twin-create`
- `heygen-photo-avatar-create`
- `heygen-prompt-avatar-create`
- `heygen-avatar-video`
- `heygen-lipsync-speed`
- `heygen-lipsync-precision`

Run/ref kinds:

- `avatar_create`
- `avatar_video`
- `video_lipsync`

If a key is absent, report the missing route briefly and offer the current GenRight/current-pipeline alternative.
