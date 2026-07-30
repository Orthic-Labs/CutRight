# QA (the gate before packaging)

Run technical, editorial, visual, caption, and audio QA on the rendered finals. A feature is not done
because it produced an MP4 — it is done when the machine gates pass **and Adrian accepts the visual
result**. QA reads evidence and reports; it does not auto-fix. A failure routes back to the stage that
owns it.

## Inputs

- The selected variant, finished ([finish](finish.md)) and, for vertical, reframed
  ([reframe](reframe.md)).
- A **resolved** benchmark report (`qa` rejects `unresolved`).
- `edit/captions.srt` and `analysis/evidence/manifest.json` (both required by `qa`).

> Ordering note: `qa` inspects the rendered YouTube final, so that final must exist first. Render it
> here as the subject under test; [export](export.md) renders the remaining presets and packages
> **after** this gate passes.

## Commands (in order)

```bash
# 1. Render the subject under test (the YouTube final).
videoctl render final <project> --preset youtube

# 2. Run QA: validate edit, rehash sources, probe the final, check benchmark + captions + evidence.
videoctl qa <project>

# 3. (vertical) Render + QA each vertical preset once per-deliverable QA lands (see gaps).
videoctl render final <project> --preset reels
```

## Evidence to read before deciding

- `qa/report.json` → `status` + `checks[]` (each with `id`, `status`, `evidence`). Today: final
  explicit, transcript benchmark, media duration/streams/dimensions, captions + evidence present.
- `analysis/evidence/filmstrips/*.png` → both sides of every cut (flash, frozen frame, jump distance).
- `edit/output-transcript-<variant>.json` → no intended word missing; second-ASR comparison for ghost
  speech.
- `edit/captions.srt` → timing, reading speed, line count, safe-zone placement.
- Audio meters → integrated loudness, true peak, clipped samples, cut pops, noise-floor jumps.

## The full gate matrix

Machine gates (deterministic failures are authoritative):

- **Media:** decodes; expected streams; duration matches plan; A/V within tolerance; constant delivery
  frame rate; expected resolution/aspect; no accidental HDR metadata; no black/frozen tail; audio
  present.
- **Editorial:** cuts align with planned words; no intended word missing; second-ASR flags ghost speech;
  no unexplained long silence; no segment under the configured minimum; no unintended duplicate beat.
- **Visual:** both sides of every cut clean; no flash/frozen frame; no graphic outside frame; no text
  overflow; no subject/caption collision; no unapproved effect; no low-res asset.
- **Captions:** output-timeline alignment; reading speed; line count; platform-safe placement;
  punctuation; font available; nothing hidden under overlays.
- **Audio:** loudness target; true peak; no clipping; no cut pops; dialogue/music balance; no abrupt
  noise-floor change.

Model-assisted QA (optional Gemini/Twelve, suggestions only): semantic speech/visual mismatch, misleading
B-roll, repetitive graphics, obvious text errors, unexplained jumps, weak opening clarity. Never let a
model finding override a deterministic failure or the human gate.

## Gate

- `qa/report.json` `status:"pass"` for the deliverable.
- Every machine gate above passes (the ones the engine automates today + the ones you verify by reading
  evidence).
- **Adrian accepts the visual result.** This is the human gate; no agent or model approves it.

## On failure — route back, do not patch blindly

| Finding | Routes to |
|---|---|
| Clipped word / bad boundary / wrong take | [rough-cut](rough-cut.md) (re-cut the segment) |
| Caption timing / overflow / collision | [finish](finish.md) (caption plan) |
| Subject off-centre / crop jitter | [reframe](reframe.md) (anchor) |
| Loudness / true-peak / cut pop | [finish](finish.md) (audio plan) |
| Colour mismatch / HDR | [finish](finish.md) (colour plan) |
| Source hash changed | [ingest](ingest.md) (investigate; never re-register over a mutated source) |

Record the failure + disposition in `feedback/decisions.jsonl`, fix upstream, re-render, re-QA.

## Handoff outputs

- `qa/report.json` (pass) → [export](export.md) packaging gate.
- QA verdicts + failures in `feedback/decisions.jsonl`.

## Engine gaps to know

- **`qa` is YouTube-preset-only and minimal today** — it checks the final exists, the benchmark is
  resolved, media duration/streams/dimensions match the YouTube preset, and captions + evidence are
  present. The full editorial/visual/caption/audio matrix above is REV2 §13.2 (per-deliverable QA), not
  yet automated. Until it lands, **read the evidence and run those gates yourself**; do not report a
  gate as passed that you did not actually check.
- There is no per-preset QA report yet; vertical finals are not separately gated by the engine. Render
  and inspect each vertical preset manually and record the result.
- Model-assisted QA (Gemini/Twelve) requires cloud consent + budget and is off by default; its findings
  are suggestions, never the ship gate.
