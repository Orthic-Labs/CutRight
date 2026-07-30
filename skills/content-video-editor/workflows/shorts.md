# Shorts (parallel extraction branch)

Extract several **materially different** standalone vertical clips from a long recording. Runs in
parallel with the main finish/export flow once a rough cut exists. This skill chooses source cut points;
Social owns hook, CTA, and packaging ([docs/HANDOFF-CONTRACTS.md](../../../docs/HANDOFF-CONTRACTS.md)).

## Inputs

- `edit/candidates.json` (the curated candidate set from [rough-cut](rough-cut.md)).
- The word transcript + visual evidence, to judge whether a candidate stands alone.
- Platform constraints from Social (target duration, aspect, hook expectation) via
  `brief/platform-brief.json`.

## Commands (in order)

```bash
# 1. Propose candidate clips (default 4). Writes edit/shorts.json.
videoctl shorts propose <project> --count 4

# 2. EDITORIAL PASS (agent-side, see below): score each candidate, enforce diversity, drop clones,
#    record the rationale in feedback/decisions.jsonl.

# 3. Per accepted clip: build its cut plan, render vertical, reframe, caption — reusing the
#    rough-cut → reframe → finish machinery scoped to that clip. (Phase 6; see gaps.)
```

## The editorial pass (what `shorts propose` does not yet do)

`shorts propose` ranks candidates by duration + take rank — a heuristic, not story extraction. Apply the
real selection yourself:

- **Score each candidate** on: standalone completeness, hook specificity, payoff strength, proof/example
  presence, novelty vs other candidates, emotional/practical value, visual-support availability, length
  fit, brand relevance, platform fit.
- **Enforce diversity (mandatory).** Four clips that paraphrase the same idea are **one** clip. Spread
  across distinct ideas/angles; reject near-duplicates.
- **Reorder only when truthful.** A clip may reorder source segments only if meaning stays true, no
  false chronology is implied, transitions stay coherent, and the reorder is recorded.
- **Length fit:** trim to the platform target (typically 15–180 s) without clipping a word or a payoff.

## Evidence to read before deciding

- `edit/shorts.json` → the proposed clips and their source ranges.
- `analysis/transcript-packed.md` + per-candidate filmstrips → does each clip open with a hook and land
  a payoff on its own?
- `brief/platform-brief.json` → duration/aspect/hook targets from Social.

## Gate

- N accepted clips are **materially different** (distinct ideas, not rewordings).
- Each clip stands alone (hook + payoff, no dependence on the long video).
- No false chronology; every reorder recorded.
- Each clip's captions and vertical crop pass QA (run [qa](qa.md) per clip).

## Handoff outputs

- `edit/shorts.json` (accepted clips + source ranges) → per-clip vertical renders.
- **→ Social**: the platform-package handoff per clip (aspect, duration, caption burn, hook goal, CTA).
  Social does not move cut points.
- **→ Writing**: per-clip title/description/hook text.

## Engine gaps to know

- `shorts propose` is a **duration/take-rank heuristic**, not semantic standalone-story extraction
  (REV2 §2.2; Phase 6). The scoring + diversity enforcement above is agent-side until semantic
  segmentation lands. Do not claim the engine scored a clip "standalone" — read the transcript and say
  what you observed.
- There is **no dedicated per-short render command** yet. Producing a finished vertical clip per
  candidate reuses the cut-plan → render → reframe → caption path scoped to that clip; wiring it as a
  batch is Phase 6. Until then, treat each accepted short as a small project through the same machinery,
  and flag the manual sequencing.
