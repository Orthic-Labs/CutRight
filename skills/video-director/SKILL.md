---
name: video-director
description: >
  CutRight-local director that turns a topic or evidence set into a bounded,
  typed shot plan: narrative arc, beat map, per-shot size/camera/element-motion
  vocabulary, style bake-off, A/B/C-roll modes, anti-monotony rhythm, and
  bounded job semantics. Plans only — never renders, never calls a cloud
  provider, never holds credentials. Adapted from the MIT-licensed Vox
  Director concepts (see THIRD_PARTY.yml and docs/legal/notices/vox-director.txt);
  provenance snapshot at imports/provenance/vox-director/.
---

# VIDEO DIRECTOR — typed shot planning, fully local

> **THE JOB:** produce a schema-bound `DirectorPlan` (arc → beats → shots)
> that downstream CutRight stages can execute. The director NEVER generates
> media itself, never touches a network, and never stores credentials.

## Hard boundary (read first)

- **Outputs are typed plans only:** `DirectorPlan`, `BeatPlan`, `ShotPlan`,
  `StyleDecision`. Media requests go through `AssetRequest` /
  `AssetDelivery` contracts; renders go through
  `cutright://skill/content` render-sample contracts.
- **No cloud providers.** Upstream hosted image/video/TTS/music providers
  are unsupported optional capabilities and are NOT referenced here by
  name. See `CUTRIGHT-ADAPTATION.md` for what was dropped.
- **No mutation.** The director emits plans; it never writes timelines,
  media files, or account state.
- Capability references use CutRight names only, e.g.
  `cutright://skill/content`, `cutright://skill/brand`.

## Workflow (two human gates, bounded jobs)

1. **Arc selection (gate 0 — confirm).** Recommend one arc from
   `references/arcs-and-coverage.md` by topic heuristic; user confirms.
2. **Beat map draft (gate 1 — mandatory approval).** Draft per-beat
   headline + narration intent; user approves/edits. This is the only
   mandatory gate.
3. **Shot planning (rule-derived).** Derive shot size + camera move from
   arc position + anti-monotony rules using the HARD-CONSTRAINED vocab in
   `schemas/shot-plan.schema.json`. Never free-form camera language.
4. **Style bake-off.** Produce 2–4 `StyleDecision` candidates (theme,
   palette, texture language) as typed comparisons; user picks one.
5. **Bounded job semantics.** Every planned generation step is a bounded
   job: declared inputs, declared outputs, bounded retries, and a typed
   degraded/failed state. No unbounded queues, no background daemons.

## Input modalities (A/B/C-roll)

- **B-roll mode (topic):** every beat gets planned evidence slots filled
  from supplied/generated evidence references.
- **A-roll mode (talking-head source):** plan cutaway/overlay shots around
  an anchored talk track; cut points come from the rough-cut stage, never
  from the director.
- **C-roll mode (single anchor image):** plan element-motion shots that
  keep one supplied subject anchored across beats.

## Typed vocabulary (schema-bound)

```rust
pub struct ShotPlan {
    pub shot_id: ShotId,
    pub beat_id: BeatId,
    pub size: ShotSize,            // EST_WIDE | WIDE | MEDIUM | CLOSE | DETAIL
    pub camera_move: CameraMove,   // static | push_in | pull_out | pan | tilt | parallax | element
    pub element_motion: Vec<ElementMotion>, // independent axis; rich is safe, rigid-paper rule applies
    pub evidence_refs: Vec<EvidenceRef>,
}
```

`ShotSize`, `CameraMove`, `Arc`, `HookPattern`, and `Ending` are enums
validated by `schemas/shot-plan.schema.json`. Banned camera tokens
(`orbit`, `arc`, `crane`, `boom`, `pedestal`, `dolly_zoom`, `roll`,
`whip_pan`, `handheld`, `fast_zoom`) fail validation in `strict` mode
because they warp flat art; a plan may declare `constraints: loose` to
carry them as an explicit, reviewed style risk.

Rules of record (details in `references/arcs-and-coverage.md`):

- Hook lands in ≤3s; beat counts are firm presets (30s → 6–8 beats;
  60s → 10–12 beats).
- Anti-monotony: no two adjacent beats share a camera move; alternate
  scale ↔ translate ↔ static families; reserve `static` for the payoff.
- Element motion is the energy engine: multiple rigid-paper elements may
  move; text stays stable; flat 2D only.
- Every `ShotPlan.evidence_refs` entry must resolve to CutRight evidence;
  dangling references fail plan validation.

## Scope

**DOES:** arc selection heuristics · beat-map drafting with approval gate ·
schema-bound shot/size/move planning · element-motion direction · style
bake-offs · A/B/C-roll modes · anti-monotony rhythm · bounded job plans.
**DOES NOT:** render media · call any provider · hold credentials · move
cut points (rough cut owns cuts) · post, schedule, or mutate accounts.
