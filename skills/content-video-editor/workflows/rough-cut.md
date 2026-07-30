# Rough cut (candidates → natural + tight)

This is where the edit is *authored*. Generate candidates, apply editorial judgment to pick the best
take per beat and order them into a story, then render both pacing variants cheaply so a human can
compare. The decision logic lives in [docs/EDITORIAL-BRAIN.md](../../../docs/EDITORIAL-BRAIN.md); this
workflow is the procedure that carries it out.

## Inputs

- `analysis/transcripts/*.json` + `analysis/transcript-packed.md` from [transcribe](transcribe.md).
- `analysis/vad-<source>.json` for every source (required by `edit render`).
- A **resolved** benchmark `decision` (do not make destructive word-edge cuts on `unresolved`).

## Commands (in order)

```bash
# 1. Mechanical candidate pass: groups words into candidate beats (900 ms gap today).
videoctl edit candidates <project>

# 2. EDITORIAL REASONING (agent-side, see docs/EDITORIAL-BRAIN.md). Not a CLI call.
#    Read analysis/transcript-packed.md + edit/candidates.json, then annotate candidates.json:
#    set beat_label (hook/setup/payoff/...), take_rank (best take first), and drop_reason for
#    high-confidence removals. Flag suggest-only items for review instead of dropping. Record the
#    rationale in feedback/decisions.jsonl. Preserve the CandidateManifest schema.

# 3. Render the natural variant: cut-plan → timeline → render → transcript remap, in one call.
videoctl edit render <project> --variant natural

# 4. Render the tight variant (same chain).
videoctl edit render <project> --variant tight

# 5. Build decision evidence (boundary filmstrips + waveform). Requires natural.mp4 to exist,
#    so it runs AFTER step 3.
videoctl evidence build <project>

# 6. Validate the cut plan(s): ranges in-bounds, no unintended overlap, output continuity.
videoctl edit validate <project>
```

Use `--dry-run` on `edit render` to preview the segment count before committing to a render.

## Evidence to read before deciding

- `edit/candidates.json` → the candidate set you are curating (`id`, `source_id`, `start_ms`, `end_ms`,
  `text`, `beat_label`, `take_rank`, `drop_reason`).
- `analysis/transcript-packed.md` → the readable narrative; the primary input to beat selection.
- `analysis/evidence/manifest.json` → per-candidate `boundary_frame`s (before/decision/after) and
  `decision_filmstrip` composites under `analysis/evidence/filmstrips/`. **Look at both sides of every
  cut** for jump distance, flash, frozen frames, and clipped words.
- `edit/output-transcript-natural.json` / `edit/output-transcript-tight.json` → the words that survived
  each variant; diff them to confirm tight removed only silence/filler, never intended words.
- `render/rough-cuts/natural.mp4` and `render/rough-cuts/tight.mp4` → watch both.

## The editorial decision (summary — full spec in EDITORIAL-BRAIN.md)

- **Best take per beat:** when the same beat was recorded multiple times, score takes on delivery
  (energy, fluff-free), technical quality (focus/exposure/audio), and completeness; keep one, mark the
  rest `drop_reason:"duplicate_take"`.
- **Red-thread order:** order beats into hook → setup → development → payoff → CTA. Reordering source
  segments is allowed only when meaning stays truthful and no false chronology is implied; record every
  reorder.
- **Filler/false-start policy:** high-confidence removals (abandoned false start with a complete
  replacement, explicit duplicate take, long dead air, isolated filler, slate/handling) get a
  `drop_reason`. Suggest-only items (tangents, jokes, emotional pauses, asides) are **flagged for
  review, not dropped**, until the format is calibrated.
- **Two variants always:** `natural` (≈400 ms retained pause, breaths + reactions kept, for YouTube)
  and `tight` (≈220 ms retained pause, faster hook, for vertical). Render both unless a saved
  preference already picked one.

## Gate

- Both `render/rough-cuts/natural.mp4` and `render/rough-cuts/tight.mp4` exist and play.
- `edit validate` passes for the cut plan(s).
- No clipped words at boundaries (spot-check filmstrips + output transcripts); no A/V drift.
- Every kept cut is traceable to source words; every drop has a `drop_reason` or a review flag.
- Suggest-only removals are surfaced for the reviewer, not silently applied.

## Handoff outputs

- `render/rough-cuts/{natural,tight}.mp4`, `edit/cut-plan-{natural,tight}.json`,
  `edit/output-transcript-{natural,tight}.json`, `edit/captions-{natural,tight}.srt`,
  `analysis/evidence/manifest.json` → [review](review.md).
- `feedback/decisions.jsonl` → the editorial rationale (beat structure, take picks, drops, reorders),
  consumed by [docs/AUTONOMY-LADDER.md](../../../docs/AUTONOMY-LADDER.md) preference learning.

## Engine gaps to know

- `edit candidates` is **mechanical** (900 ms gap grouping; first candidate labelled `hook`, rest
  `beat`; `take_rank` = index). It does not score takes, find the red thread, or classify filler.
  EDITORIAL-BRAIN.md specifies that build target; until it lands, the editorial pass is your annotation
  of `candidates.json`. Do not fabricate scoring signals the engine did not produce — read real evidence.
- **Shared alias hazard (REV2 P0-B):** rendering tight after natural overwrites the shared
  `edit/cut-plan.json`, `edit/timeline.json`, and `edit/captions.srt` to *tight*. The variant-specific
  files above persist correctly. Downstream commands that still read the shared alias (`reframe plan`,
  `render final`, `export otio`, `qa`) will see whichever variant rendered last. Until P0-B lands,
  **re-render the selected variant last** (in [review](review.md)) before any back-half command, and
  note the alias risk in the handoff.
- `evidence build` keys boundary frames off candidates and renders the `natural.png` waveform from the
  rendered natural rough cut, so decision evidence is produced *after* the first render, not before
  take selection. Pre-take-selection visual evidence (frame quality, gesture, framing) arrives with
  Phase 7 temporal perception; until then, take *technical* quality from the source filmstrips you can
  sample and say so when you escalate.
