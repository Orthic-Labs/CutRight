---
task: CR-V2-B5-006
book: 5
lane: S
status: frozen
title: Freeze Book 5 creative skill, planning, and renderer lane ownership
commit: CR-V2-B5-006: freeze-book-5-creative-OS-and-native-renderer-lane-ownership
depends_on:
  - CR-V2-B5-005
---

# Book 5 Interface Freeze

## Purpose

Freeze the boundaries between Book 5's three parallel lanes (product-local
creative skill runtime, creative planning, native finish renderer) and the
crate ownership each lane has. Frozen names must survive the merge step
(CR-V2-B5-022) without churn.

## Lane ownership

| Lane | Path | Crate | Scope |
|------|------|-------|-------|
| A    | `crates/video-sessions/**`        | `video-sessions`     | Product-local skill runtime and resolver |
| A    | `skills/brand/**`                | (skill assets)       | Brand skill surface |
| A    | `skills/brand-identity/**`       | (skill assets)       | Brand Identity skill surface |
| A    | `skills/designer/**`             | (skill assets)       | Designer skill surface |
| A    | `skills/writing/**`              | (skill assets)       | Writing/packaging skill surface |
| A    | `skills/social/**`               | (skill assets)       | Social platform constraints skill |
| B    | `crates/video-editorial/**`      | `video-editorial`    | Editorial planning and creative-plan composition |
| B    | `schemas/creative/{bakeoff,brand-card,brand-system,style-direction}.schema.v2.json` | schemas | Creative planning schemas |
| B    | `fixtures/creative/golden-fixtures.json` | fixtures | Native migration comparison fixtures |
| C    | `crates/video-core/**`           | `video-core`         | Native GPU/vector compositor, render graph, typography, motion, audio |
| C    | `crates/video-renderer/**`       | (planned; native)    | Native renderer surfaces (compositor + render-graph) |

## Frozen render-graph types

These types are reserved by the contract and may not be redefined by any lane:

```text
RenderGraph
RenderGraphNode
RenderGraphEdge
RenderGraphCompileError
LegacyRenderer(remotion | hyperframes | hyper-frames)
CreativePlan
CompiledFinishPlan
ValidatedAssetReview
```

`RenderGraphCompiler::legacy_renderers()` returns
`["remotion", "hyperframes", "hyper-frames"]`. Any node whose inputs or
`via` props resolve to a forbidden name produces
`RenderGraphCompileError::LegacyRenderer(_)` and is rejected before
compilation.

## Frozen skill execution contract (per B5-001)

- Skills expose `SkillRuntime` with `resolve(id) -> SkillDescriptor`,
  `invoke(batch) -> TypedActionBatch`.
- `SkillDescriptor` carries `id`, `lane`, `version`, `permissions`,
  `evaluation`, `dependency_closure`.
- `TypedActionBatch` round-trips through `video_core::lib.rs` re-exports
  to avoid name collisions with earlier module surfaces.

## Frozen creative critic axes (10)

1. brand_alignment
2. narrative_clarity
3. visual_composition
4. motion_grammar
5. typography_legibility
6. audio_balance
7. platform_fit
8. rights_safety
9. accessibility
10. determinism (fixed at 1.0 by deterministic QA)

## Acceptance

- All lane boundaries above are respected.
- Legacy renderer names rejected at compile.
- All four Book 5 native renderer crates compile and test cleanly.
- Skill resolver prefix-match key `lane.` resolves skill IDs of the form
  `brand.derive_typography` to the `brand` lane.