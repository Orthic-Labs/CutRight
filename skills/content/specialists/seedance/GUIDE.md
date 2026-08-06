---
name: content-seedance
description: >
  Seedance 2.0 AI video generation — cinematic motion shots Remotion can't produce (fluid motion,
  fabric/water, camera moves with depth). In CutRight v2 this lane is an UNSUPPORTED OPTIONAL
  capability of the signed content runtime pack (hosted WaveSpeed backend); never a required path.
---

# Seedance 2.0 Video Generation (optional hosted lane)

> **CutRight v2:** everything below describes the upstream locked WaveSpeed video pipeline. The
> hosted Seedance lane is an UNSUPPORTED OPTIONAL capability of the signed content runtime pack.
> In a base CutRight image this route reports unavailable-offline; deterministic motion routes to
> the Remotion guidance instead.

Seedance runs **through the locked AI Video Ad Pipeline only** — never as a standalone curl
(upstream rule, preserved).

## Backend lock (HARD)

Upstream locked the backend split (upstream PIPELINES.md section 2.E, locked 2026-05-03; the
upstream rules file is not vendored):

- **Seedance 2.0 = WaveSpeed.** fal is **Veo 3.1 ONLY**. There is no fal route for Seedance — that
  backend split is locked. Calling fal for Seedance is wrong and is rejected by the router.
- Backend module (upstream `tools/backends/wavespeed.js`; not vendored — ships with the signed
  pack): Kling 3.0 Pro, Seedance 2.0 Fast+Std, HappyHorse, Wan, InfiniteTalk. FLF anchor param for
  Seedance = `last_image`.
- Model keys: **`seedance-2-fast`** ({480p: $0.10, 720p: $0.20, 1080p: $0.50}/s) — budget social
  tier, or **`seedance-2-std`** ({480p: $0.12, 720p: $0.24, 1080p: $0.60}/s) — higher quality.
  Resolution-tiered models THROW if no `resolution` is passed (no silent default).
- Key required upstream: `WAVESPEED_API_KEY` — a host-provided credential, never vendored, never
  required in base CutRight.

## Mandatory gates (upstream pipeline — preserved as policy)

Every Seedance I2V call went through these; the pipeline threw or halted if any was missing:

1. **shot_contract (preflight).** No I2V call fires without a validated `shot_contract` block;
   preflight throws before any API spend. The contract schema/renderer (upstream modules
   `tools/recipes/video/prompt-contract.json` and `tools/pipelines/video/lib/prompt-contract.mjs`)
   ship with the signed pack. Seedance renders as timestamp blocks `[00:00-00:0X]`.
   `audio_lock.mode = "lipsync_to_external_vo"` is NOT valid for Seedance (HappyHorse/InfiniteTalk
   only).
2. **Cost preflight.** The runner prints line items + total before the first API call. Tiered
   models require `item.resolution` or it throws.
3. **Per-clip visual QA.** Upstream ran frame-by-frame review with two model jurors in parallel via
   a host vision tool (>= 3 fps, identical frame set); single-juror was forbidden — divergence is
   the signal. In CutRight v2 clip QA is typed as `cutright://skill/qa {"mode":"visual_review"}`.
4. **Operator taste / eyes gate.** Upstream `tools/lib/human-eyes-gate.mjs` halted the pipeline at
   every taste-critical artifact (storyboard frames, per-clip QA, final cut) until the operator
   explicitly approved. In CutRight v2 the eyes gate is the host approval step recorded in the
   evidence graph — AI suggests, reviewers score, **only the operator approves**. No agent can
   approve on their behalf.
5. **Assembly re-encodes** through the concat filter (`scale + pad + setsar=1 + fps +
   format=yuv420p`), never `-c copy`. Final cut is also QA + eyes-gated before delivery.

## Seedance vs Remotion

| Need | Tool |
|---|---|
| Text on screen, kinetic typography, UI demos, captions, app-state changes | Remotion (I2V hallucinates UI — never ask it) |
| Brand color motion that's deterministic | Remotion |
| Cinematic shots: water, fabric in wind, real body/face motion | Seedance |
| Camera moves with parallax + depth | Seedance |
| Hybrid: Seedance hero composited into Remotion | Both |

## Workflow (retyped to CutRight actions)

1. `cutright://skill/brand {"brand_code":"<DD|RH|HR|TS>"}` — load the brand visual system. (No SS —
   SS is a passion project; no commercial video.)
2. **Storyboard START + END frames** for each shot via NB2 (upstream: `create.mjs --step=1`;
   CutRight: `cutright://capability/content.video_create` with `step` 1). Single-frame I2V drifts;
   both anchors are required.
3. **Author the `shot_contract`** (inline or from a brand-pack `shot_contract_templates/`
   baseline). Compose the prompt from the vendored video recipes — never invent vocabulary.
4. **Animate via the pipeline** (Seedance on WaveSpeed, signed pack only):

       cutright://capability/content.video_create {
         "step": 2,
         "image": "out/shot1_start.png",
         "end": "out/shot1_end.png",
         "motion": "...",
         "model": "seedance-2-fast",
         "resolution": "720p",
         "duration": 4,
         "aspect": "9:16",
         "shot_contract": "out/shot1.contract.json",
         "out": "out/shot1.mp4"
       }

   The runner validates the contract, runs the cost preflight, routes to the hosted backend, and
   emits a `<clip>.prompt.json` audit sidecar on the delivery record.
5. **Faceless YT/IG Shorts** — use the fast path instead of hand-authoring shots:

       cutright://capability/content.faceless_shorts {}
       cutright://capability/content.campaign {"manifest": "<emitted manifest path>"}

   Captions are composited in Remotion, never asked of Seedance.
6. **Per-clip visual QA → operator eyes-gate → assemble → final visual QA → final eyes-gate.** All
   enforced; do not present output as deliverable until the eyes-gate records APPROVE.

> If you cannot confirm the exact flags for a given pipeline version, route to the typed capability
> and let the signed pack report its own usage — do NOT fall back to a raw curl.

## Brand prompt cheatsheet

### DD
- Lighting: low-key, single warm rim, deep shadows
- Camera: locked or slow push, never handheld
- Mood: considered, weighted, slightly dangerous
- Banned: bright, vibrant, cheerful, hi-energy

### RH
- Lighting: natural window, soft golden hour
- Camera: handheld with subtle drift
- Mood: honest, tactile, lived-in
- Subjects: fabric, hands, draped clothing

## Output
1. Prompt + shot_contract used (and the audit sidecar on the delivery record)
2. Model key + resolution + delivered clip reference
3. Cost preflight estimate + actual incurred (reconciled from the pack's delivery manifest)
4. Visual QA verdicts + eyes-gate status
5. Composite recommendation
