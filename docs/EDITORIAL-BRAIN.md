# Editorial Brain

**Spec for the implementation agent.** Written 2026-07-31 against the `a8d4584` tree + REV2.
Design authority: the Video Editor skill; Adrian's eyes approve the result and his saved preferences
calibrate it. This is the intelligence that makes CutRight "edit properly" rather than "remove silence."

## 0. The contract (lead)

The editorial brain is the reasoning layer between mechanical candidate generation and cut-plan
rendering. It turns transcripts + evidence into a **scored, ordered, annotated** `CandidateManifest`:
one best take per beat, beats ordered into a story arc, fillers triaged by confidence, a constructed
hook, and a confidence on every decision that drives escalation.

**Integration point — `crates/video-project/src/lib.rs`:**

- `build_candidates` (lib.rs:1085) today groups words by a fixed 900 ms gap and emits
  `Candidate { id, source_id, start_ms, end_ms, text, beat_label, take_rank, drop_reason }` with
  `beat_label = "hook"` for the first group / `"beat"` otherwise, `take_rank = index+1`, and
  `drop_reason = None`. **It performs no editorial selection.** The brain replaces this labelling with
  real beat detection, take scoring, and filler triage.
- `build_cut_plan` (lib.rs:1276) consumes `edit/candidates.json`, keeps candidates where
  `drop_reason.is_none()`, and renders per-variant segments via `candidate_chunks` (variant gap
  threshold: tight 220 ms / natural 400 ms) with VAD-expanded fallback bounds. **The brain does not
  change boundary arithmetic** — it decides *which* candidates survive (`drop_reason`), *their order*,
  and *their beat label*; `build_cut_plan` renders that decision. Keep this split: brain = selection +
  ordering; cut plan = timing + boundaries.

Today the brain runs **agent-side**: the agent reads the evidence and annotates `candidates.json`
(preserving the schema) plus an `edit/editorial-plan.json` (below). The engine target is to move the
deterministic parts (take dedup, filler classification, scoring) into Rust behind `build_candidates`,
leaving narrative judgement to the agent. Each section marks **Today** vs **Target**.

## 1. Evidence inputs

The brain reasons only from artifacts that exist in the package — never from raw frames en masse:

| Signal | Source | Used for |
|---|---|---|
| Packed narrative | `analysis/transcript-packed.md` | beat detection, red thread, hook |
| Word transcript | `analysis/transcripts/<source>.json` (`words[].confidence`, `events[]`) | take completeness, filler, false starts |
| VAD regions | `analysis/vad-<source>.json` | pause lengths, dead air |
| Audio features | `analysis/audio-features.json` (RMS, true peak, loudness, speech rate, pause lengths) | energy/delivery scoring |
| Visual evidence | `analysis/evidence/filmstrips/*.png`, saliency + visual-quality flags | technical quality, jump distance at cuts |
| Platform brief | `brief/platform-brief.json` | arc template, target length, hook goal |
| Preferences | `feedback/preferences.json` | filler policy, pause length, density (calibrated) |

Do not invent signals the engine did not produce. If technical visual quality is unavailable (pre-Phase
7), say so and escalate take *technical* judgement rather than guessing.

## 2. Beat segmentation + red-thread structure

A **beat** is one narrative unit (a point, a step, a story turn). Beats are ordered into an arc:

```text
hook → setup → development (beats…) → payoff → CTA
```

- **Detect beats** from the packed transcript: split on speaker change, topic shift, and meaningful
  pause; merge fragments that complete one thought. Each beat gets a `beat_label` from a fixed
  vocabulary: `hook | setup | development | payoff | cta | tangent`.
- **Arc templates** (from `brief/platform-brief.json`):
  - YouTube (`natural`): hook (≤15 s promise) → setup → ordered development beats → payoff → CTA. Keep
    breaths/reactions; preserve section endings.
  - Short (`tight`): cold-open hook (strongest claim/payoff first) → one development beat → payoff →
    CTA. 15–180 s.
- **Best take per beat first, then order:** select the take (§3) *within* each beat, then order the
  *beats*. Do not let take order dictate narrative order.
- **Reordering rule:** source segments may be reordered only when meaning stays truthful, no false
  chronology is implied (do not imply B followed A when it did not, unless that is genuinely the story),
  transitions stay coherent, and **every reorder is logged** in `editorial-plan.json.reorder_log`.
  `build_cut_plan` validates source ranges independently and permits output reordering of non-overlapping
  ranges (REV2 §6.5); an intentional reused range needs an explicit `repeat` flag.

## 3. Best-take-per-beat selection

When the same beat was recorded multiple times (duplicate takes), score each take and keep one.

**Detecting duplicate takes (Target: engine; Today: agent):** two candidate groups are takes of the same
beat when their word text is near-identical (normalised token overlap ≥ 0.85) within a time window, or
the packed transcript marks a re-take. Mark all but the winner `drop_reason:"duplicate_take"`.

**Scoring signals** (each 0..1; weight is a starting candidate, calibrated by preferences — not a
validated truth):

| Signal | Weight | Evidence |
|---|---|---|
| Delivery (energy, pace, fluff-free) | 0.35 | speech rate + pause pattern (audio features), filler count (transcript) |
| Completeness (no clipped/trailing words, full thought) | 0.25 | word confidence at edges, `events[]`, VAD tail |
| Technical (focus, exposure, framing, clean audio) | 0.25 | visual-quality flags + filmstrip; noise floor / clipping (audio) |
| Hook/payoff strength (for the hook/payoff beat only) | 0.15 | specificity of the line, saliency at the moment |

`take_score = Σ weight·signal`. Keep the highest scorer; set its `take_rank = 1`; mark the rest
`drop_reason:"duplicate_take"` with the winner recorded. A take disqualified on a hard fault (clipped
first/last word, unusable exposure, audible clip) is dropped regardless of score. Ties within 0.05
prefer the earlier, cleaner take and are flagged `ambiguous`.

## 4. Filler + false-start policy

Three tiers. The tier a class sits in is parameterised by format + autonomy (see
[AUTONOMY-LADDER.md](AUTONOMY-LADDER.md)) and learned from `feedback/preferences.json`.

**Automatic (high-confidence removal → set `drop_reason`):**
- abandoned false start **with a complete replacement** present;
- explicit duplicate take (loser of §3);
- long dead air (VAD gap beyond the variant's dead-air threshold);
- clearly isolated filler ("um", "uh") not carrying emphasis;
- slate / setup / clapper material;
- camera start/stop handling.

**Suggest-only (flag for review, do NOT drop until calibrated):**
- repeated ideas / restatements;
- tangents (`beat_label:"tangent"`);
- "fluff" / banter;
- jokes;
- emotional pauses / reactions;
- uncertainty hedging;
- personal asides.

Suggest-only items are recorded in `editorial-plan.json.review_flags` with the reason and the candidate
id, and surfaced in [review](../skills/content-video-editor/workflows/review.md). In `reviewed` mode a
suggest-only removal requires a human decision; in a calibrated `autonomous` format the learned
preference decides.

**Preserve (never auto-remove):**
- breaths and section-ending pauses in `natural`;
- reactions/laughter (`events[]`) that serve pacing;
- emphasis words, even if technically filler;
- anything the preference profile marks keep.

The transcript always **preserves** fillers (`preserve_fillers_in_transcript: true`); removal is a
cut-plan decision, never a transcript edit.

## 5. Hook + cold-open construction

- **YouTube hook:** the first ≤15 s must promise the payoff. Choose the strongest existing opening line;
  if the recording buries the payoff, the hook may be a *cold open* — a short segment from the payoff
  beat moved to the front, labelled `beat_label:"hook"`, with the reorder logged. Never fabricate a hook
  line that was not said.
- **Vertical cold open:** open on the single strongest claim or payoff moment (≤3 s to value), then the
  one development beat, then the payoff. The cold open is a reorder, logged, and must not imply false
  chronology.
- The hook candidate is scored with the hook/payoff-strength signal (§3); a weak hook (low specificity,
  generic opener) is flagged for Writing to re-voice or for human review — the brain proposes, it does
  not invent words (wording is Writing's handoff).

## 6. Confidence, ambiguity, escalation

Every brain decision carries a `confidence` (0..1) and optional `ambiguity` flags.

**Confidence sources:** take-score margin (winner vs runner-up), word confidence at boundaries, signal
agreement (audio says energetic AND visual says clean), evidence availability.

**Escalate to a human when any holds:**
- take-score margin < 0.05 (cannot pick a best take);
- technical-quality signal unavailable or conflicting (pre-Phase 7 visual quality);
- a suggest-only removal in `reviewed` mode;
- benchmark `unresolved` (destructive cuts not permitted — see
  [BENCHMARK-ACCEPTANCE.md](BENCHMARK-ACCEPTANCE.md));
- hook is weak/generic and no strong opening exists in the footage;
- reorder would change chronology meaning (truthfulness risk);
- overall cut confidence below the format's floor (per [AUTONOMY-LADDER.md](AUTONOMY-LADDER.md)).

An escalation writes a `review_flag` and, in `reviewed`/`review-light` modes, blocks the downstream gate
until resolved. In `autonomous` mode an unresolved escalation forces the project back to `review-light`
for that run (graceful degradation, never a silent guess).

## 7. Output schema

`edit/editorial-plan.json` (new; the brain's rationale, consumed by review + preference learning):

```json
{
  "schema_version": 1,
  "variant_policy": {"natural": {"retained_pause_ms": 400}, "tight": {"retained_pause_ms": 220}},
  "arc": ["hook", "setup", "development", "development", "payoff", "cta"],
  "beats": [
    {
      "beat_id": "beat-001",
      "beat_label": "hook",
      "selected_candidate": "candidate-004",
      "take_scores": [
        {"candidate_id": "candidate-004", "score": 0.82, "signals": {"delivery": 0.9, "completeness": 0.8, "technical": 0.75, "hook_strength": 0.85}},
        {"candidate_id": "candidate-001", "score": 0.61, "dropped": "duplicate_take"}
      ],
      "confidence": 0.78,
      "ambiguity": []
    }
  ],
  "reorder_log": [
    {"candidate_id": "candidate-004", "from_index": 3, "to_index": 0, "reason": "cold-open on payoff", "chronology_safe": true}
  ],
  "review_flags": [
    {"candidate_id": "candidate-009", "kind": "suggest_only", "reason": "tangent — possibly cut", "needs_human": true}
  ],
  "escalations": []
}
```

`edit/candidates.json` is annotated in place: `beat_label`, `take_rank`, and `drop_reason` set per the
plan; `drop_reason` values come from a fixed vocabulary
(`duplicate_take | false_start | dead_air | isolated_filler | slate | handling | <review-flag-id>`).
`build_cut_plan` then renders exactly the survivors, in order, per variant.

## 8. Engine gaps to know

- `build_candidates` is mechanical (lib.rs:1085); beat detection, take scoring, filler triage, hook
  construction, and confidence are **not implemented**. Until they move into Rust, the brain is the
  agent's annotation of `candidates.json` + `editorial-plan.json`. The deterministic parts (take dedup
  by token overlap, filler classification, dead-air detection from VAD) are the first candidates to
  implement; narrative ordering and hook judgement stay agent-side.
- Technical visual-quality signals (focus/exposure/framing) are pre-Phase 7; the `technical` score is
  weak until temporal perception lands. Escalate technical take choices rather than guessing.
- `editorial-plan.json` is an addition to the package layout; adopt it in the schema + migrations. It is
  the primary input to preference learning ([AUTONOMY-LADDER.md](AUTONOMY-LADDER.md)) — log it even while
  the brain is agent-side.
