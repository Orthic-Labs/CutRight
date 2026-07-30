# Reframe (vertical crop-track)

Point a 9:16 window at the subject for each segment so the vertical final is framed, not just cropped.
Reframe is **spatial crop-tracking** — where the vertical window sits. It is *not* editorial motion
(punch-ins, cutaways, transitions), which lives in [finish](finish.md) /
[docs/MOTION-LANGUAGE.md](../../../docs/MOTION-LANGUAGE.md). The two compose: reframe keeps the subject
in frame; motion adds emphasis on top.

## Inputs

- The **selected** variant from [review](review.md), rendered last so the shared `edit/timeline.json`
  reflects it (`reframe plan` reads that shared alias).
- Source footage registered in `sources/manifest.json`.

## Commands (in order)

```bash
# 1. Plan vertical anchors: one Vision face anchor per timeline segment (midpoint frame today).
videoctl reframe plan <project>

# 2. REVIEW + APPROVE each anchor (agent or Studio). Edit analysis/reframe-plan.json:
#    set approved:true on each anchor you accept, adjust center_x/center_y (normalized 0..1) where the
#    subject is off-centre, then set the plan-level approved:true. Record the verdicts in
#    feedback/decisions.jsonl.

# 3. The 9:16 final render (in export) now consumes the approved plan:
videoctl render final <project> --preset reels
```

## Evidence to read before approving

- `analysis/reframe-plan.json` → `anchors[]`: `source_id`, `output_start_ms`/`output_end_ms`,
  `center_x`, `center_y`, `strategy` (`vision_face` | `manual_anchor_required`), `confidence`,
  `approved`. Plan-level `approved`, `requires_review`, `target_aspect:"9:16"`.
- The midpoint frame per segment (`cache/frames/reframe-<segment>.jpg`) → confirm the subject is
  actually where the anchor claims. A face box on the wrong subject, or a subject that moves across the
  segment, needs a manual centre or an escalated flag.
- Platform safe zones → keep the subject clear of where captions / platform UI will sit.

## Approval rules

- Approve an anchor only if the subject is genuinely framed for the segment's full interval.
- `strategy:"vision_face"` + high `confidence` + subject centred → approve.
- `strategy:"manual_anchor_required"`, low confidence, or a moving subject → set `center_x`/`center_y`
  by hand (normalized 0..1) and approve, **or** flag for human placement. Do not approve a guess.
- The plan is approvable only when the anchors **exactly cover** the output timeline: one anchor per
  segment, matching `source_id` + `output_start_ms`/`output_end_ms`, all centres in `[0,1]`. The 9:16
  final render enforces this and refuses an incomplete or unapproved plan.

## Gate

- `analysis/reframe-plan.json` has `approved:true` and every anchor `approved:true`.
- Anchors exactly cover the selected variant's output timeline (the render checks this).
- Subject is framed and clear of caption/UI safe zones for each segment; low-confidence anchors were
  manually placed or escalated, not waved through.

## Handoff outputs

- `analysis/reframe-plan.json` (approved) → [export](export.md) `render final --preset reels|tiktok`.
- Reframe verdicts in `feedback/decisions.jsonl`.

## Engine gaps to know

- **One midpoint face box per segment, not temporal tracking** (REV2 §2.2; Phase 7). A subject that
  moves within a segment gets a single static centre, which can frame poorly mid-segment. Smoothed
  temporal crop paths (face/body/hand tracks over time, no anchor jitter) are Phase 7. Until then,
  split a high-movement segment or set a manual centre, and flag the limitation.
- `reframe plan` reads the **shared** `edit/timeline.json`, so it plans against whichever variant
  rendered last. Re-render the selected variant last (in [review](review.md)) before planning, or the
  vertical anchors will map to the wrong variant's timeline.
- The Vision anchor worker is a bundled binary; REV2 §3.16 notes its staleness check is weak. A failed
  or low-confidence anchor is a real signal — escalate it, do not silently centre-crop.
