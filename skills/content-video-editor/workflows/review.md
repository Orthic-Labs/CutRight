# Review + variant selection

Compare the two rough cuts, judge them against the evidence, and select **exactly one** variant to
drive the back half. Approval and selection are different actions: approving a cut says "this is good";
selecting it says "build the final from these exact bytes." Both are recorded in the decision ledger.

The review surface is **CutRight Studio** (the Tauri app in `apps/studio/`). Studio reads artifacts and
writes verdicts; it never runs `videoctl` and never edits the timeline. The agent drives the engine;
Adrian judges the result. `videoctl review <project>` is a frozen-contract stub that only echoes the
artifact list — it does not open the UI.

## Inputs

- `render/rough-cuts/{natural,tight}.mp4`, `edit/output-transcript-{natural,tight}.json`,
  `edit/captions-{natural,tight}.srt`, `analysis/evidence/manifest.json` from [rough-cut](rough-cut.md).
- A resolved benchmark `decision`.

## Path A — Studio (human review, default in `reviewed` mode)

1. Open the project in CutRight Studio (Sources / Compare / Finals / QA modes).
2. In **Compare**, Adrian watches natural vs tight, compared by source-word identity (the compound
   `source_word_id` joins the same spoken word across variants).
3. Record a **variant verdict** (approve/reject + a target-specific reason: `pacing`, `word_edges`,
   `energy`, `length`, `other`) and any **segment flags** at the cursor (`clipped_word`, `too_tight`,
   `too_loose`, `bad_boundary`, `wrong_take`).
4. Make the explicit **"Use for final"** selection on the approved variant.
5. Every action appends a content-bound record to `feedback/decisions.jsonl`. UI state is never the
   source of truth; reload must replay the same decisions.

Studio sends a minimal **intent**; Rust constructs the authoritative record (REV2 P0-A). Do not
hand-build decision records in the UI or in the skill — that bypasses hash-binding and validation.

## Path B — headless / agent-assisted (when no human is at the UI)

The agent cannot fabricate a binding approval. In headless mode:

1. Read the evidence yourself: both output transcripts, the boundary filmstrips, both rough cuts.
2. Make the editorial call per [docs/EDITORIAL-BRAIN.md](../../../docs/EDITORIAL-BRAIN.md) and record
   the reasoning (which variant, why, what was flagged) — this is agent reasoning, not a binding
   verdict.
3. Express the selection concretely by **re-rendering the chosen variant last** so the shared aliases
   point at it:

   ```bash
   videoctl edit render <project> --variant <selected>   # re-run last to win the shared alias
   ```

4. Leave the human-approval gate unsatisfied until Studio records it, **unless** the format is
   calibrated for `review-light`/`autonomous` (see [docs/AUTONOMY-LADDER.md](../../../docs/AUTONOMY-LADDER.md)).
   In `reviewed` mode, a final render without an approved, hash-bound selection is a defect.

## The selection record (what "selected" means)

A variant is *selected* only when all hold (REV2 P0-B `variant_selection`):

```json
{
  "schema_version": 1,
  "selected_variant": "natural",
  "rough_cut_blake3": "...",
  "timeline_blake3": "...",
  "selected_by_decision_id": "...",
  "selected_at": "..."
}
```

- the variant verdict is **approved** for the same rough-cut artifact hash;
- the benchmark policy permits destructive word-edge cuts (resolved, not `unresolved`);
- source hashes still match the manifest;
- the variant's artifact receipt is current.

## Commands

```bash
# Frozen-contract stub: echoes the reviewable artifact list. Not the UI.
videoctl review <project>

# (planned, REV2 P0-B) Bind the selected variant for the back half.
videoctl review select <project> --variant <selected>
```

## Evidence to read before deciding

- `feedback/decisions.jsonl` → the verdicts and segment flags already recorded (and their
  `subject_blake3` — a decision is valid only for the artifact it reviewed).
- `analysis/evidence/filmstrips/*.png` → both sides of each cut.
- The two output transcripts → confirm the selected variant dropped only what was intended.

## Gate

- Exactly one variant is approved **and** selected, bound to the current rough-cut bytes.
- Benchmark resolved; source hashes unchanged.
- All segment flags are either resolved (re-cut) or explicitly accepted with a recorded reason.
- The selected variant was rendered last, so shared aliases (`edit/timeline.json`, `edit/cut-plan.json`,
  `edit/captions.srt`) reflect it.

## Handoff outputs

- The selected variant + its hash-bound selection → [finish](finish.md), [reframe](reframe.md),
  [export](export.md).
- `feedback/decisions.jsonl` (grown) → preference learning and the autonomy ladder.

## Engine gaps to know

- `videoctl review select` and the `variant_selection` projection are **planned** (REV2 P0-B), not
  shipped. Until they land, "selection" is enforced by rendering the chosen variant last + the Studio
  verdict, and downstream commands read the shared alias. Treat any back-half command run against a
  non-selected variant as a bug.
- `render final` is currently hard-coded to `render/rough-cuts/natural.mp4` + generic `edit/captions.srt`
  (REV2 §3.8). If the selected variant is **tight**, the final will still consume natural until
  variant-aware final lands — so either select natural, or block final and escalate. Do not ship a
  final built from the unselected variant.
- Decision replay can mark a once-valid record `stale`/`missing` if its artifact was renamed or
  re-rendered (REV2 §5.6). Stale is not corrupt; surface the count, do not hide it.
