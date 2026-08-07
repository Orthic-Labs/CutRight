# V2 Book 5 Creative OS and Native Renderer Lane Ownership

**Status:** Frozen — `CR-V2-B5-006`
**Owner:** CutRight creative OS (Book 5)
**Schema:** `schemas/creative/book5-lane-ownership.schema.v2.json`

## 1. Purpose

This document freezes the **Book 5** lane ownership map. It is a *strict subset* of the global `lane-ownership.v2.json` (see `V2-CREATIVE-OS-LANES.md`) restricted to the lanes that participate in Book 5's creative finish-plan. It also binds the **native renderer** lane to its concrete crates and decision rights so the studio cannot accidentally re-scope native renderer to a non-native runtime.

## 2. Book 5 lanes

Book 5 activates exactly nine lanes:

```text
planning ── brand ── brand-identity ── designer ── writing ── social
   │           │           │              │           │          │
   └───► asset-validation ──► native-renderer ──► creative-critic ──► job-plane
                │                  │
                │                  ├─ native-typography
                │                  ├─ native-motion
                │                  └─ native-audio
```

## 3. Native renderer crate boundary

The native renderer lane owns:

- `crates/video-core/src/native_renderer/` — the render-graph compiler.
- `crates/video-core/src/native_typography/` — typography + captions engine.
- `crates/video-core/src/native_motion/` — motion grammar + reframing.
- `crates/video-core/src/native_audio/` — audio finishing engine.
- `schemas/render/*.json` — render graph and node schemas.

The native renderer lane **does not own**:

- `crates/video-jobs/` (job-plane lane).
- `crates/video-core/src/creative_critic/` (creative-critic lane).
- `crates/video-core/src/brand/` (brand lane).
- `crates/video-core/src/designer/` (designer lane).
- `crates/video-core/src/writing/` (writing lane).
- `crates/video-core/src/social/` (social lane).
- `crates/video-core/src/creative_plan/` (planning lane).

## 4. Native renderer decision rights

| Decision right | Owner of the right | Verifying reviewer |
|---|---|---|
| `render-graph` | `native-renderer` | `creative-critic` via `critique` stage |
| `finish-plan` | `native-renderer` | `creative-critic` via `qa-visu` stage |
| `typography` | `native-typography` (sub-lane) | `native-renderer` |
| `motion-grammar` | `native-motion` (sub-lane) | `native-renderer` |
| `audio-finishing` | `native-audio` (sub-lane) | `native-renderer` |
| `visual-qa` | `creative-critic` | `job-plane` |
| `critic-verdict` | `creative-critic` | `job-plane` |

## 5. Native renderer sub-lane matrix

| Sub-lane | Owns | Verifies via |
|---|---|---|
| `native-typography` | `crates/video-core/src/native_typography/` | `critic_evaluation.v2` axis `typography_legibility` |
| `native-motion` | `crates/video-core/src/native_motion/` | `critic_evaluation.v2` axis `motion_grammar` |
| `native-audio` | `crates/video-core/src/native_audio/` | `critic_evaluation.v2` axis `audio_balance` |

Each sub-lane is a *child* of the native renderer lane. A sub-lane cannot publish a render-graph node without its parent (`native-renderer`) co-signing.

## 6. Forbidden surfaces

The native renderer lane **must not** import or call:

- `crates/video-core/src/brand/*` (brand tokens are read-only inputs).
- `crates/video-core/src/creative_critic/*` (critic is a separate evaluator).
- Any legacy `remotion`, `hyperframes`, or `hyper-frames` code path.

The native renderer lane **must** declare its render-graph as a `schemas/render/render-graph.schema.v2.json` instance and route it through the creative critic before the `render` stage.

## 7. Book 5 evidence ledger

Every native-renderer output records:

- `render_graph:<id>` (determinism axis).
- `caption_doc:<id>` (typography_legibility axis).
- `motion_clip:<id>` (motion_grammar axis).
- `audio_profile:<id>` (audio_balance axis).

The creative-critic stage reads these IDs from the evidence ledger and writes a `critic_evaluation.v2` referencing them.

## 8. Freeze scope

This freeze binds:

- The 9 Book 5 lane ids.
- The native renderer crate boundary.
- The 7 native renderer decision rights.
- The 3 sub-lane ownership rows.
- The forbidden surfaces list.

Any change to a Book 5 lane id, a native renderer crate boundary, or a forbidden surface requires a new frozen `v2` schema revision and a new dispatch task.
