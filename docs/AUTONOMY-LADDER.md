# Autonomy Ladder

**Spec for the implementation agent.** Written 2026-07-31 against the `a8d4584` tree + REV2.
Design authority: the Video Editor skill; Adrian sets the policy and his acceptance is the only thing
that advances a format. "Hands-off" is *earned per format*, never granted globally.

## 0. The contract (lead)

CutRight starts fully supervised and becomes hands-off one **format** at a time, as measured acceptance
proves the engine matches Adrian's taste. Three modes — `reviewed → review-light → autonomous` — gate
how many intermediate approvals a run needs. Advancement is driven by the decision ledger
(`feedback/decisions.jsonl`) and learned preferences (`feedback/preferences.json`); a format that stops
meeting its thresholds is demoted. The visual taste gate is **never** delegated: even `autonomous`
produces *ready* outputs for Adrian's eyes, not self-published ones.

A **format** is the unit of autonomy: `content_type × platform × variant` — e.g.
`youtube-talking-head-natural`, `reels-short-tight`. Each format carries its own mode, confidence, and
sample count. A new format always starts at `reviewed`.

## 1. The three modes

From the vision plan §17.3, made concrete:

| Mode | Intermediate approvals | What the run does alone | What still needs a human |
|---|---|---|---|
| `reviewed` (default) | approve rough cut + finish strategy + final | ingest, transcribe, candidate gen, render variants, evidence, QA | every gate |
| `review-light` | approve rough cut only | finish in saved style, reframe, QA, export + package + handoffs | rough cut; final received *with* QA report |
| `autonomous` | none | the whole pipeline, unattended, preserving all variants | Adrian's visual sign-off before **publish**; escalations |

Invariants in **every** mode:
- preserve all variants (never delete the unselected one);
- never overwrite the last approved output (write a new dated artifact);
- sources immutable; cloud off unless consented + budgeted;
- a JSON `status:"error"` is a failure, not a success.

## 2. Per-format state

Stored in `feedback/autonomy.json` (new; derived from the ledger + preferences):

```json
{
  "schema_version": 1,
  "formats": {
    "youtube-talking-head-natural": {
      "mode": "review-light",
      "sample_count": 7,
      "consecutive_accepted": 5,
      "confidence": 0.81,
      "take_acceptance_rate": 0.9,
      "boundary_correction_rate": 0.08,
      "graphic_acceptance_rate": 0.85,
      "escalation_rate": 0.05,
      "last_advanced_at": "2026-07-30T12:00:00Z",
      "demoted": false
    }
  }
}
```

The metrics are computed from `feedback/decisions.jsonl` (verdicts, segment flags) and
`edit/editorial-plan.json` (take picks, escalations) across that format's projects, and from
[qa](../skills/content-video-editor/workflows/qa.md) reports.

## 3. Advancement + demotion thresholds

Numbers are **starting candidates**, calibrated by Adrian's actual acceptances — not validated truths.
Advancement requires the human-acceptance gate to have passed on the counted projects.

**`reviewed → review-light`** when, over the format's last projects:
- `sample_count ≥ 5` (the first five real projects run `reviewed`, per the vision plan);
- `consecutive_accepted ≥ 5` (no rejected final in the streak);
- `take_acceptance_rate ≥ 0.85` (Adrian keeps the brain's best-take picks);
- `boundary_correction_rate ≤ 0.15` (few boundaries manually moved);
- `escalation_rate ≤ 0.15`.

**`review-light → autonomous`** when, additionally:
- `sample_count ≥ 15` in `review-light` with `consecutive_accepted ≥ 10`;
- `take_acceptance_rate ≥ 0.92` and `boundary_correction_rate ≤ 0.08`;
- `graphic_acceptance_rate ≥ 0.85` and `escalation_rate ≤ 0.05`;
- preferences for this format are **stable** (filler policy, pause length, caption style, effect density
  converged — low variance over the last N decisions).

**Demotion:** any rejected final, any unresolved escalation, or a benchmark regression drops
`consecutive_accepted` to 0; a format that falls below its current mode's floors is demoted one step and
`demoted:true` is recorded until it re-earns advancement. Demotion is automatic; advancement is not (it
also requires Adrian to not have reserved the gate).

## 4. Escalation triggers (force a human)

Any of these escalates the current project and, for that run, drops the effective mode one step toward
`reviewed` (graceful degradation — never a silent guess):

- take-score margin < 0.05 or conflicting technical signals ([EDITORIAL-BRAIN.md](EDITORIAL-BRAIN.md) §6);
- benchmark `unresolved` (destructive cuts not permitted);
- a suggest-only removal while the format is `reviewed`;
- weak/generic hook with no strong opening in the footage;
- a reorder that risks false chronology;
- overall cut confidence below the format floor;
- any QA machine-gate failure that the engine cannot attribute and route;
- a new format on its first run (always `reviewed`).

Escalations are recorded in `editorial-plan.json.escalations` and surfaced in the run digest (§6).

## 5. What `autonomous` may do unsupervised

Apply the learned filler policy; select the best take per beat; order the arc; build natural + tight;
finish in the saved style (captions/audio/colour/motion from preferences); reframe vertical; run QA;
export + package; emit the Writing/Designer/Social handoffs; write the digest.

It may **not**: approve the visual taste for publication (that stays Adrian's); overwrite the last
approved output; delete the unselected variant; enable cloud without consent; touch sources; suppress an
escalation to keep a run "clean"; or advance its own format (advancement is Adrian's, computed from his
acceptances).

## 6. Unattended-run digest

Every unattended (or partially unattended) run writes `feedback/digests/<run_id>.json` and a rendered
`feedback/digests/<run_id>.md` summary. The headline is the requested shape — **N processed, M need
review, K ready**:

```json
{
  "schema_version": 1,
  "run_id": "run-2026-07-31-01",
  "started_at": "2026-07-31T02:00:00Z",
  "ended_at": "2026-07-31T03:12:00Z",
  "totals": {"processed": 6, "ready": 4, "needs_review": 1, "failed": 1},
  "cost_usd": 0.0,
  "cache_hit_rate": 0.73,
  "projects": [
    {
      "project_id": "myvideo-9f2c1a2b",
      "format": "youtube-talking-head-natural",
      "mode": "autonomous",
      "status": "ready",
      "confidence": 0.86,
      "qa_status": "pass",
      "selected_variant": "natural",
      "finals": ["render/finals/youtube.mp4", "render/finals/reels.mp4"],
      "handoffs": ["writing-copy", "designer-thumbnail", "social-package"],
      "escalations": []
    },
    {
      "project_id": "podcast-3b7d0c11",
      "format": "youtube-interview-natural",
      "mode": "review-light",
      "status": "needs_review",
      "confidence": 0.58,
      "qa_status": "pass",
      "escalations": [{"kind": "take_margin", "detail": "beat-003 margin 0.03"}]
    }
  ]
}
```

`status` ∈ `ready` (passed QA, awaiting Adrian's visual sign-off to publish) | `needs_review`
(escalation or low confidence; blocked at the named gate) | `failed` (a stage errored; see §7). The
digest is what Adrian reads; it never claims a project is publishable — `ready` means "ready for your
eyes."

## 7. Graceful-degradation guarantees

A mid-run failure must land every project in a **reviewable** state. Guarantees:

1. **Atomic, receipted stages.** Every stage writes its artifact atomically with a receipt binding
   inputs, parameters, tool versions, and output hashes (REV2 §10.4). A crash between stages leaves the
   last good stage intact; nothing is half-written.
2. **Resumable, content-addressed jobs.** Restarting a run does not lose progress or rerun completed
   work (REV2 §10.5). A resumed project continues from its last good stage.
3. **Failure is labelled, not hidden.** A failed project is `status:"failed"` in the digest with the
   failing stage + structured error; it is never silently skipped or marked ready.
4. **The reviewable core always survives.** The selected variant, `feedback/decisions.jsonl`, and
   `edit/editorial-plan.json` are recoverable for any project that reached rough cut, so a human can
   always open it in Studio and finish by hand.
5. **Escalation downgrades the mode, not the integrity.** An unresolved escalation drops the run toward
   `reviewed`; it does not corrupt output or force a guess.
6. **Sources untouched, always.** No failure path modifies a source; manifest hashes are re-checked at
   QA.

## 8. Preference-learning loop

`feedback/decisions.jsonl` (verdicts + segment flags, content-bound per REV2 §5.4) and
`edit/editorial-plan.json` (take scores, reorder log, review flags) feed `feedback/preferences.json`,
per format:

```json
{
  "schema_version": 1,
  "format": "youtube-talking-head-natural",
  "preferred_pause_ms": {"natural": 400, "tight": 220},
  "filler_policy": {"isolated_filler": "automatic", "tangent": "suggest_only", "joke": "preserve"},
  "effect_density": "restrained",
  "caption_style": "youtube-clean",
  "broll_frequency": "low",
  "shot_duration_ms": [2500, 6000],
  "preferred_anchors": ["top-left", "bottom-center"],
  "music_sfx": {"music": "off", "sfx": "functional-only"},
  "accepted_hook_structures": ["cold-open-payoff", "promise-then-proof"],
  "preferred_final_length_ms": [480000, 720000]
}
```

Learning is what moves the ladder: stable, high-acceptance preferences are the precondition for
`autonomous` (§3). After publication, Social imports platform analytics and links retention changes to
output-timeline moments — but a single video is never causal proof, and TRIBE is never used. Preferences
cite the actual prior decisions that produced them; a recommendation that cannot cite its evidence is not
shown.

## Engine gaps to know

- The gate **modes** are designed (vision §17.3) but not implemented as engine state; today every
  project effectively runs `reviewed` because the Studio decision contract (REV2 P0-A) and variant
  selection (P0-B) are still being repaired. `feedback/autonomy.json` and `feedback/digests/` are
  additions to the package layout.
- **Resumability is partial.** Transcription has a cache envelope, but the whole pipeline is not yet
  content-addressed and resumable (REV2 §2.2, §10.5). Guarantee §7.2 holds fully only once that lands;
  until then a mid-run crash may require re-running a stage, and the skill must say so in the digest.
- **Preference learning is append-only-by-design but not yet generated/consumed** (REV2 §2.2; Phase 9).
  The brain logs `editorial-plan.json` now so the data exists when the learner lands; the engine does not
  yet read `preferences.json` to drive decisions.
- The decision ledger must first be made durable + hash-bound (REV2 P0-A) before it can be trusted as
  the ladder's evidence source. Do not compute advancement off a ledger that can silently drop records.
