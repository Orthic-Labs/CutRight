# CUTRIGHT-ADAPTATION — Content skill (CR-V2-B1-010)

Adaptation log for the selected content-production closure. Source: workspace-capabilities
@ `6ee21f03a787e7b57dc412760a8996ea7a235302`, upstream path `tools/skills/content`
(provenance: `THIRD_PARTY.yml`; receipt: `imports/v2/receipts/content.json`;
selection: `imports/v2/selections/content.json`).

## 1. Import shape and exclusions

- Selected roots imported: `SKILL.md`, `agents/`, `evals/`, `references/`, and specialists
  `production-routing`, `transcription`, `remotion`, `seedance`, `image-enhancement` (61 files).
- Import tooling placed the `SKILL.md` include-path into a nested directory; it was flattened to
  `skills/content/SKILL.md` at import time (upstream path is a single file).
- Explicit exclusions with reasons (`imports/v2/exclusions/content.json`):
  - `specialists/kdp` — not a CutRight v2 video lane.
  - `specialists/carousel` — capture retyped to `cutright://skill/qa {"mode":"capture"}`.
  - `specialists/demo-recorder` — depended on the workspace tools/demo runtime (not vendored).

## 2. Runtime contract rewrite (SKILL.md)

- Header retyped: `EFFECT_PROFILES: asset_read, asset_delivery_emit`;
  `MAY_CALL_SKILLS: cutright://skill/brand, cutright://skill/writing, cutright://skill/designer, cutright://skill/social`.
- New runtime contract: the base skill requires nothing on PATH (no Python/Node/FFmpeg/cloud key);
  all execution runs as typed capabilities `cutright://capability/content.<name>` inside the signed
  content runtime pack, answering with AssetDelivery records only.
- Hosted generation providers (GenRight pipelines, WaveSpeed/Seedance, HeyGen, hosted TTS) are
  marked UNSUPPORTED OPTIONAL capabilities — never required paths.
- Upstream route `specialists/video-editor/SKILL.md` is absent at the pinned revision (honest
  upstream gap, recorded in the receipt notes — not fabricated).

## 3. Reference rewrites

| Upstream form | CutRight form | Where |
|---|---|---|
| Venture workspace anchor block (venture repo paths, pipeline guides) | venture-anchor provenance note; anchors not vendored, arrive with the signed pack | `references/routing.md` |
| Hosted-provider routes treated as default paths | NOTE_HOSTED annotation: unsupported optional capabilities, unavailable-offline in base image | `references/routing.md`, `references/motion-graphics.md`, `references/avatar-video.md`, `references/article-illustrations.md`, `references/smoke-checklist.md` |
| `tools/skills/_shared/*.md` paths | `skills/_shared/*.md` (vendored under CR-V2-B1-007) | `references/manual.md`, `SKILL.md` |
| `/brand <code>` style loads | `cutright://skill/brand {"brand_code":"<code>"}` (typed result BrandCard) | `references/manual.md` |
| visual audit routing | `cutright://skill/qa {"mode":"visual_review"}` | `references/manual.md` |
| kdp / carousel / demo-recorder rows | explicit excluded-branch rows pointing at the exclusions file | `references/manual.md` |

## 4. Specialist rewrites

- **transcription/GUIDE.md** — host-scoped Windows pipeline block (ScrapeRight checkout, host
  ffmpeg, on-disk model dir, host env vars) retyped to `cutright://capability/content.transcribe`;
  pack supplies engine/ffmpeg/weights; post-run checklist preserved and retargeted to AssetDelivery
  records. Upstream transcript-copy destinations and research-bucket writes become delivery notes.
- **production-routing/GUIDE.md** — provider execution and the host image-generation tool marked
  optional signed-pack capabilities; upstream validation command (workspace skill-creator script,
  not vendored) replaced with the import-closure gate tooling plus the smoke checklist; route table
  substance preserved with an optional-capability note.
- **seedance/GUIDE.md** — hosted WaveSpeed lane marked UNSUPPORTED OPTIONAL capability; backend
  module, contract schema/renderer, eyes-gate lib, and the upstream rules file marked not-vendored
  pack contents; host credential requirement marked host-provided (never vendored); pipeline
  invocations retyped to `content.video_create`, `content.faceless_shorts`, `content.campaign`;
  brand loads retyped to `cutright://skill/brand`; dual-juror QA retyped to
  `cutright://skill/qa {"mode":"visual_review"}`; eyes gate retyped to the host approval step in the
  evidence graph; brand cheatsheets and gate policy preserved verbatim.
- **image-enhancement/GUIDE.md** — script path retargeted to `skills/content/...`; the four
  interpreter invocations retyped to `cutright://capability/content.enhance` (Pillow ships in the
  signed pack); example output format preserved.
- **remotion/GUIDE.md** — provenance note inserted: the subtree is upstream Remotion guidance
  carried as inert provenance; Node/npx/FFmpeg never required by the base skill.
- **evals** — content evals rows for kdp / carousel / demo-recorder retyped to exclusion notices;
  transcription evals prompt and expected behavior scrubbed of host machine paths and of the
  not-vendored downloader skill name in prose.

## 5. Scripts and agent config

- `specialists/image-enhancement/scripts/enhance.py` — received the CR-V2-B1-010 provenance header;
  inert provenance in the base runtime; executes only as the typed capability
  `cutright://capability/content.enhance` inside the signed content runtime pack.
- `agents/openai.yaml` — upstream host-interface configuration; provenance header added, carried
  inert.

## 6. Gate evidence

- `python3 tools/import-closure/assert_no_external_refs.py skills/content` → OK (run at commit time).
- `python3 tools/import-closure/verify_exclusions.py imports/v2/exclusions/content.json imports/v2/graphs/content.json` → PASS.
- `python3 tools/import-closure/verify_copy.py imports/v2/graphs/content.json skills/content` → PASS
  on the pre-adaptation snapshot (61 files).
