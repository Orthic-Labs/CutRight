# Arcs, pacing, and coverage — adapted director reference

Adapted from the MIT-licensed Vox Director beat-layer concepts
(`imports/provenance/vox-director/references/beat-layer.md`). Tokens are
schema-bound by `schemas/shot-plan.schema.json`; this document is the
reasoning layer, not a looser vocabulary.

## 1. Narrative arc library

| Arc | When to use | Beat shape |
| --- | --- | --- |
| `hook_payoff` | default for any single idea; safest | Hook → Context → Build → Payoff → Button |
| `pas` | pain-aware ads, urgency | Problem → Agitate → Solve → Proof → CTA |
| `bab` | when the "after" sells better than the pain | Before → After → Bridge → CTA |
| `aida` | cold-audience paid ads | Attention → Interest → Desire → Action |
| `storybrand` | brand/service, customer-as-hero | Hero wants → Problem → Guide → Plan → CTA → stakes |
| `how_it_works` | product/process/system explainer | Hook → What it is → 2–3 shown steps → Benefit → CTA |
| `timeline` | history, "evolution of", journeys | Start → events → turning point → present → takeaway |
| `man_in_hole` | case study, comeback, transformation | OK → fall → deepen → climb out → better than before |
| `story_spine` | mission/brand/founder tales | Once… → Every day… → Until one day… → Because… → Until finally… → Ever since… |
| `origin` | founder / "why we exist" | World → spark → leap → struggle → breakthrough → today |
| `myth_buster` | correct a misconception | FACT first → the myth → expose fallacy → what to believe → CTA |
| `listicle` | tips/tools/rankings | Promise → items → #1 → recap/CTA |
| `three_act` | any narrative 60s piece | Setup → Confrontation (rising) → Resolution |
| `story_circle` | character-driven stories | You → Need → Go → Search → Find → Take → Return → Change |

Topic → arc heuristic: product/service ad → `pas`/`bab`/`aida`/`storybrand`;
concept/system → `how_it_works`/`hook_payoff`; historical → `timeline`;
transformation/case study → `man_in_hole`; brand/mission →
`story_spine`/`origin`; correcting a belief → `myth_buster`; rankings/tips →
`listicle`.

## 2. Hook, pacing, beat counts (firm presets)

- **Hook in ≤3s.** Beat 1 carries the payoff promise (bold claim /
  provocative question / surprising stat / "you're doing X wrong"). Never
  spend beat 1 on setup.
- **Hook patterns:** `mistake_callout · pain_point · surprising_stat ·
  direct_question · urgent_warning · secret_reveal · experiment_story ·
  pattern_interrupt · outcome_tease`.
- **Beat counts:** 30s → 6–8 beats @ ~4–5s (~70–80 VO words); 60s → 10–12
  beats @ ~5–6s (~130–150 VO words).
- **Proportions:** hook 1–3s → body 70–80% → payoff 10–20% → end/CTA 0–2s.
- **Change something visually every 3–5s; never hold a static frame >8s** —
  this is why beats split into short shots.
- **Endings:** `hard_cut` (default; drives rewatches) · `quick_cta` (≤2s,
  one action + benefit, 3–5 words) · `loop_close` (last line mirrors first).

## 3. Shot sizes and coverage

`EST_WIDE` (whole scene/system) · `WIDE` (subject in environment) ·
`MEDIUM` (one subject centered; workhorse) · `CLOSE` (one detail fills
frame) · `DETAIL` (single texture/word/number; punch beat).

Coverage plays out ACROSS beats: move-in `EST_WIDE → MEDIUM → CLOSE`
(builds intensity) · move-out `CLOSE → … → WIDE` (reveals context; good
ending) · establishing-wide → detail cut-in is the strongest two-shot beat.

## 4. Camera move — hard-constrained flat-safe vocabulary

Rule: **uniform translate + uniform scale = safe** (text moves as one
piece); anything that warps perspective, blurs, or rotates text off-axis is
banned in strict mode.

| Safe token | Realization | Job |
| --- | --- | --- |
| `static` | no transform, tiny element float | let a stat/quote land |
| `push_in` | uniform scale-up | tension/focus |
| `pull_out` | uniform scale-down | reveal / big picture |
| `pan` | translate across an over-wide frame | read a list/timeline |
| `tilt` | vertical translate | reveal scale / countdown |
| `parallax` | fg/mid/bg layers at different speeds | the living-paper signature |
| `element` | one cut-out slides/hinges in, others still | introduce/emphasize one item |

**Banned in strict mode** (verified upstream to tend to warp flat art /
smear text): `orbit · arc · crane · boom · pedestal · dolly_zoom · roll ·
whip_pan · handheld · fast_zoom`. They remain available under
`constraints: loose` as an explicit, reviewed style risk; a clean
dolly-in feel is better served by a flat `push_in`.

## 5. Element motion — the energy engine (separate axis from camera)

Rich multi-element motion is SAFE when the rigid-paper rules hold:

- Encouraged: multiple elements move at once · scatter/burst ·
  pop/slide/flap/hinge in · drift · sway · flutter · pulse.
- A traveling "hero" element is an OCCASIONAL punch on a key beat, not a
  per-shot formula.
- Hard limits: rigid paper only (no organic morph/warp), text stays stable,
  flat 2D.
- Camera and element motion are independent: one camera move + as much
  element motion as the beat earns. Shot size gates density (WIDE → several
  elements; CLOSE → the one subject animates strongly).

## 6. Anti-monotony (biggest quality lever)

- No two adjacent beats use the same camera move; alternate families
  (scale ↔ translate ↔ static).
- Reserve `static` for the payoff/quote beat so the motion drop signals
  "this is the point."
- Move-rhythm presets per arc:
  - `hook_payoff` (8): push_in → pan → parallax → static → push_in → tilt → pull_out → static
  - `pas`/`bab` (6): push_in → static → pan → parallax → push_in → static
  - `listicle`: same move on every item but flip direction / parallax on #1; static on recap
  - `timeline`: pan same direction beat-to-beat → push_in on turning point → pull_out on takeaway

## 7. Style bake-off

Plan 2–4 `StyleDecision` candidates (theme preset + per-video palette +
texture language) and compare them as typed artifacts; one decision is
recorded in the plan. Style is a decision, not a runtime lookup — no
provider model selection happens here.

## 8. Bounded job semantics

Every planned generation/render step in a `DirectorPlan` is a bounded job:
declared inputs, declared outputs, bounded retry budget, typed
degraded/failed states surfaced to the monitor. No unbounded queues, no
background daemons, no provider polling.
