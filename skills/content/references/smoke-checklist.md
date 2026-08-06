# Smoke Checklist

> **CutRight v2:** hosted generation providers named here (GenRight pipelines, `$imagegen`, HeyGen, hosted TTS) are UNSUPPORTED OPTIONAL capabilities of signed runtime packs — never required paths. In a base CutRight image, report the route as unavailable-offline and defer or use the local alternative. Venture workspace anchors (GenRight repo, pipeline guides) are not vendored; they arrive, when installed, with the corresponding signed pack.


Use this after editing the skill. These are route checks, not provider runs.

| Prompt | Expected route |
|---|---|
| `Use $contentcreation to make a single product image.` | Codex `$imagegen` if available; otherwise GenRight Image Studio/current image pipeline. |
| `Use $contentcreation to plan a kinetic promo video.` | `motion-graphics.md`, then GenRight Video Studio/current video pipeline with cheap draft defaults. |
| `Use $contentcreation to create article illustrations for this Chinese post.` | `article-illustrations.md` plus `xiaohei-illustration-style.md`, shot list first, Chinese sparse annotations, then `$imagegen` in Codex or GenRight Image Studio otherwise. |
| `Use $contentcreation for Ian/Xiaohei style illustrations.` | Keep the route inside `contentcreation`; read `xiaohei-illustration-style.md`; do not invoke or install a standalone Xiaohei skill. |
| `Use $contentcreation to make a HeyGen avatar video from avatar_id and script.` | `avatar-video.md`; use `heygen-avatar-video` only if present, otherwise current avatar alternative. |
| `Use $contentcreation to create a new HeyGen digital twin from my training video.` | Use `heygen-digital-twin-create` only if present; otherwise report missing route and offer alternatives. |
| `Use $contentcreation to generate a cloned voiceover for this script.` | GenRight Voice Studio/current TTS route; produce WAV first. |
