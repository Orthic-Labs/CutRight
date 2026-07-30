# Finish (captions, audio, colour, motion)

Finish only a **locked, selected** rough cut. Finishing layers captions, audio, colour, and editorial
motion onto the approved base — it never re-cuts. Editorial motion (punch-ins, jump-cut masking,
motivated transitions, B-roll at payoff) follows [docs/MOTION-LANGUAGE.md](../../../docs/MOTION-LANGUAGE.md);
that is distinct from the crop-tracking reframe in [reframe](reframe.md).

## Inputs

- A selected variant from [review](review.md), rendered last so the shared aliases point at it.
- `edit/output-transcript-<variant>.json` (for caption timing) and `analysis/evidence/manifest.json`
  (for safe placement).
- `brief/VIDEO-BRAND.md` / `brief/motion-plan.md` if the project carries brand + motion direction; load
  `/brand <venture>` for brand work.

## Author the finish plan

Write `finish/finish-plan.json` (validated by the engine). Shape per the canonical schema:

```json
{
  "schema_version": 1,
  "base_timeline": "edit/timeline.json",
  "slots": [
    {
      "id": "slot-001",
      "kind": "caption",
      "renderer": "ass",
      "effect_id": "caption.vertical-phrase.v1",
      "output_start_ms": 0,
      "output_end_ms": 5918,
      "anchor": "bottom-center",
      "collision_policy": "avoid-subject-and-platform-ui",
      "props": {"profile": "vertical-phrase"}
    },
    {
      "id": "slot-002",
      "kind": "cutaway",
      "renderer": "remotion",
      "effect_id": "stat.counter.clean-v1",
      "output_start_ms": 8100,
      "output_end_ms": 11200,
      "props": {"amount": 600, "currency": "USD"}
    }
  ]
}
```

Every proposed visual must answer: what does it clarify or emphasise, why is this timing right, why this
renderer, and does it avoid subject/captions/platform-UI? If real footage exists for the point, prefer a
cutaway over an invented graphic. Apply the motion grammar's density rules; when in doubt, add less.

## Commands (in order)

```bash
# 1. Validate the finish plan (slots reference the locked base timeline, props schema-check).
videoctl finish validate <project>

# 2. Render each slot. Today only renderer "render.final" (a delivery preset) is wired;
#    Remotion/HyperFrames/ASS effect slots land in Phase 5 (see gaps below).
videoctl slot render <project> slot-001
videoctl slot render <project> slot-002
```

## Captions, audio, colour (plans to author)

- **Captions** (`edit/captions.json` / per-variant `.srt`): map output-timeline words → punctuation +
  chunking → platform safe-zone → renderer (ASS for fast fixed karaoke/phrase; Remotion for branded
  kinetic; HyperFrames for bespoke type). Pick a profile (`youtube-clean`, `vertical-karaoke`,
  `vertical-phrase`, `quote-emphasis`, `multispeaker`) defining chunking, line count, chars/line,
  reading speed, active-word behaviour, margins, safe areas, collision fallback.
- **Audio** (`finish/audio-plan.json`): denoise only if needed → high-pass → cautious resonance fix →
  dialogue compression → de-ess if required → true-peak limit → loudness normalise → room-tone at cuts →
  music/SFX ducked under speech. Keep raw and processed dialogue as separate cached assets. SFX are
  functional (one sound per meaningful event), never "whoosh on every transition."
- **Colour** (`finish/colour-plan.json`): input/log transform → exposure + WB correction → shot matching
  → optional creative look → output transform. For mixed iPhone HDR/SDR, tone-map into a defined
  working/output space before the look. Do not apply one LUT blindly to every input.

## Evidence to read before deciding

- `analysis/evidence/manifest.json` + `spatial`/saliency signals → candidate graphic placements over the
  whole slot interval (not one frame); choose the lowest-cost stable anchor.
- The selected rough cut + its output transcript → caption timing and emphasis words.
- `feedback/preferences.json` → saved caption style, effect density, anchor, SFX/music taste. Honour it.

## Gate

- `finish validate` passes; every slot renders without error.
- Captions are timed to the output transcript, within reading speed, and inside platform safe zones; no
  caption hidden under an overlay; no subject collision.
- Audio hits the integrated-loudness target and true-peak ceiling; no cut pops; dialogue primary.
- Colour is matched across shots; HDR footage delivered in the declared output space.
- In `reviewed` mode, Adrian approves the finish strategy (and the look). Adrian's eyes are the visual
  authority.

## Handoff outputs

- `finish/finish-plan.json`, `finish/audio-plan.json`, `finish/colour-plan.json`,
  `finish/effects-used.json` → [qa](qa.md) and [export](export.md).
- **→ Motion** ([docs/HANDOFF-CONTRACTS.md](../../../docs/HANDOFF-CONTRACTS.md)): a motion-language
  request when a slot needs a *new* signature motion, not a stock effect.
- **→ Designer**: a styleframe/thumbnail request for the finished frame (see [export](export.md)).

## Engine gaps to know

- **Only `render.final` slots are wired.** `slot render` rejects any other renderer; the Remotion /
  HyperFrames / ASS effect library (the 15 starter effects, caption components, alpha overlay
  composition) is Phase 5. Until then, captions ride the burned-caption card path at final render and
  graphics are limited — author the full finish plan anyway so it is ready when the renderers land, and
  flag unrenderable slots rather than faking them.
- **Caption profiles, the full audio chain, loudness/true-peak gates, music ducking, and the profile
  colour pipeline are Phase 4**, not built. Current captions are basic SRT grouping; current colour is
  HDR-to-Rec.709 only. Specify the plans now; the deterministic media engine owns the final dialogue
  processing and loudness gate (not Remotion).
- `render final` consumes the **generic** `edit/captions.srt`, not the selected variant's
  `captions-<variant>.srt` (REV2 §3.13). Until preset-specific captions land, ensure the generic
  `edit/captions.srt` reflects the selected variant (re-render selected last) or the burned captions
  will be from the wrong variant.
