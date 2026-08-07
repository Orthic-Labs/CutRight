# V2 Creative Critic Semantics and Evaluation Axes

**Status:** Frozen — `CR-V2-B5-003`
**Owner:** CutRight creative lane (Book 5)
**Schema:** `schemas/creative/critic-evaluation.schema.v2.json`

## 1. Purpose

Define the **independent creative critic** semantics and the **ten evaluation axes** every FinishPlan, render-graph, asset, style direction, brand card, or package must be scored against before the studio unlocks a render.

The critic is the **decoupling point** between the agents that *produce* creative artefacts and the agents that *judge* them. The producer lanes (Brand, Designer, Writer, Social, Native Renderer, Audio) cannot satisfy themselves; the critic is authoritative.

## 2. Critic principles

1. **Independence** — The critic is bound only to typed schemas and to evidence anchors; never to producer-side state.
2. **Determinism first** — A deterministic visual QA pass is required before any subjective critic call. Both must record the same `target.id` and same `evaluator.id`.
3. **Evidence-anchored** — Every axis score must reference at least one `evidence_refs` entry. Unreferenced scores are blocked.
4. **Bounded subjectivity** — Subjective axes (`brand_alignment`, `narrative_clarity`, `visual_composition`) carry a hard cap on per-artefact variance across reruns.
5. **Finality** — A `verdict` of `pass` or `warn` is a publishable signal; `fail` and `blocked` are hard stops that abort the active job plane.

## 3. The ten evaluation axes

| Axis | Producer lane | Score range | Default weight |
|---|---|---|---|
| `brand_alignment` | Brand / Brand Identity | 0.0–1.0 | 0.12 |
| `narrative_clarity` | Editorial → Writing | 0.0–1.0 | 0.15 |
| `visual_composition` | Designer → Native Renderer | 0.0–1.0 | 0.12 |
| `motion_grammar` | Native Motion | 0.0–1.0 | 0.08 |
| `typography_legibility` | Native Typography | 0.0–1.0 | 0.10 |
| `audio_balance` | Native Audio | 0.0–1.0 | 0.10 |
| `platform_fit` | Social | 0.0–1.0 | 0.08 |
| `rights_safety` | Asset Validation | 0.0–1.0 | 0.10 |
| `accessibility` | Studio | 0.0–1.0 | 0.05 |
| `determinism` | Native Renderer | 0.0–1.0 | 0.10 |

Weights sum to 1.0 for any single artefact. Per-axis weight overrides require an explicit `evaluator.policy_ref`.

## 4. Verdict semantics

| Verdict | Meaning | Producer action |
|---|---|---|
| `pass` | All axes ≥ `pass_threshold` (default 0.75) | Publish |
| `warn` | One axis in `[warn_low, pass_threshold)` | Publish with `findings` recorded |
| `fail` | At least one axis < `warn_low` (default 0.55) | Remediate |
| `blocked` | A `severity: blocker` finding present | Abort, no render |

## 5. Deterministic visual QA contract

The deterministic visual QA is a *separate evaluator* whose `evaluator.kind` is `deterministic`. It must always run first and produce a `verdict` of `pass` or `blocked`. The independent creative critic only runs when deterministic QA passes.

Deterministic QA checks (fixed list, frozen in `schemas/creative/critic-evaluation.schema.v2.json`):

- Brand immutable tokens (locked marks, type, palette) are not overwritten.
- Render graph contains no `JavaScript`, `Html`, `Css`, `fetch`, or `executable` node.
- Cycle-free graph (acoustically feedback-free chains allowed).
- Caption safe-zone + reduced-motion gates satisfied.
- Determinism axis score = 1.0 (caller cannot override).

## 6. Evidence anchoring

Every axis score attaches `evidence_refs[]` to:

- `brand_card:<id>` for `brand_alignment`
- `evidence:<id>` for `narrative_clarity`
- `style_direction:<id>` for `visual_composition`
- `motion_clip:<id>` for `motion_grammar`
- `caption_doc:<id>` for `typography_legibility`
- `audio_profile:<id>` for `audio_balance`
- `platform_profile:<id>` for `platform_fit`
- `rights_record:<id>` for `rights_safety`
- `a11y_record:<id>` for `accessibility`
- `render_graph:<id>` for `determinism`

Critic that returns an unreferenced axis score is rejected by the gate.

## 7. Freeze scope

This freeze binds:

- The ten axis names and weights.
- The four verdict values.
- The two evaluator kinds (`deterministic`, `creative-critic`).
- The deterministic QA fixed list.

Any change to axis names, weights, verdict values, or evaluator kinds requires a new frozen `v2` schema revision and a new dispatch task.
