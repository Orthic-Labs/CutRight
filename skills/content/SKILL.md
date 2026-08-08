---
name: content
description: "Route media production: images, illustrations, motion or video, avatars, voiceover, Seedance, Remotion, enhancement, and transcription. Use when media is the deliverable; route UI to cutright://skill/designer, prose to cutright://skill/writing, and strategy to cutright://skill/social."
---

# Content

MODE: OUTPUT_ONLY
PRIMARY_DELIVERABLE: Media artifact (delivered as AssetDelivery).
DISCOVERY_PROFILE: D1_SCOPED_SOURCE
EFFECT_PROFILES: asset_read, asset_delivery_emit
SPECIALIST_REFS_MAX: 1
CHILD_AGENTS_MAX: 0
EXTERNAL_REQUESTS_MAX: 0
MAY_ADD_TASKS: NO
MAY_CALL_SKILLS: cutright://skill/brand, cutright://skill/writing, cutright://skill/designer, cutright://skill/social
TERMINAL: Acceptance proven.

## Runtime contract (CutRight v2)

- **The base content skill requires nothing on PATH** — no Python, no Node, no FFmpeg, no cloud key.
  Routing, shot contracts, preflight reasoning, and review are agent work over this vendored guidance.
- **All execution runs through signed runtime packs.** Engine scripts under this skill are inert
  provenance; they execute only as typed capabilities `cutright://capability/content.<name>` inside
  the signed content runtime pack and answer with **AssetDelivery** records (no direct workspace
  mutation). Example capabilities: `content.enhance`, `content.transcribe`, `content.video_create`,
  `content.faceless_shorts`, `content.campaign`.
- **Hosted generation providers are UNSUPPORTED OPTIONAL capabilities.** GenRight pipelines,
  WaveSpeed/Seedance, HeyGen, and hosted TTS are never required paths; in a base CutRight image the
  route reports unavailable-offline and the deterministic local alternative (Remotion guidance) is
  offered instead.
- Visual review and capture are typed: `cutright://skill/qa {"mode":"visual_review"}` /
  `{"mode":"capture"}` (typed result: **VisualReviewResult**). Brand loading is typed:
  `cutright://skill/brand {"brand_code":"<code>"}` (typed result: **BrandCard**).

## Route by deliverable

- Image or edit: `references/routing.md`.
- Illustration: `references/article-illustrations.md`; biological-mechanical: `skills/_shared/illustrate/GUIDE.md`.
- Motion/video: `references/motion-graphics.md`; Remotion: `specialists/remotion/GUIDE.md`;
  Seedance (optional hosted lane): `specialists/seedance/GUIDE.md`; avatar: `references/avatar-video.md`.
- Transcription: `specialists/transcription/GUIDE.md` (capability `content.transcribe`, signed pack only).
- Enhancement: `specialists/image-enhancement/GUIDE.md` (capability `content.enhance`, signed pack only).
- Mixed or unfamiliar work: `references/manual.md`.

## Excluded branches (see imports/v2/exclusions/content.json)

- `specialists/kdp` — not a CutRight v2 video lane; not vendored.
- `specialists/carousel` — capture retyped to `cutright://skill/qa {"mode":"capture"}`; not vendored.
- `specialists/demo-recorder` — depended on the workspace tools/demo runtime; not vendored.

## Local editing route

Video editing routes to local `cutright://skill/content-video-editor`; Rust `videoctl` owns ingest,
word-safe cuts, finish, QA, and export. This skill never invokes a sibling checkout.

Read one branch. Freeze sources, rights, format, size, duration, text, & acceptance. Verify output
through the QA skill. UI → cutright://skill/designer; prose → cutright://skill/writing;
distribution → cutright://skill/social.
