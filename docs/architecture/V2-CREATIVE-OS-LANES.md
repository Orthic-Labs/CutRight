# V2 Creative OS Lane Ownership

**Status:** Frozen — `CR-V2-B5-004`
**Owner:** CutRight creative OS (Book 5)
**Schema:** `schemas/creative/lane-ownership.schema.v2.json`

## 1. Purpose

Define the **single-writer** rule for the CutRight creative OS. Each lane owns:

- A fixed set of crates (`crates/<lane>/*`).
- A fixed set of schemas (`schemas/creative/*`, `schemas/render/*`).
- A fixed set of decision rights (what the lane may unilaterally publish).
- A fixed set of artefacts (the named outputs the lane emits).

No lane may modify a file outside its exclusive ownership without a joint commit co-signed by the lanes listed in `shared_contracts`.

## 2. The lane roster

| Lane id | Name | Crate root | Decision rights |
|---|---|---|---|
| `brand` | Brand | `crates/video-core/src/brand` | `brand-system`, `style-direction` |
| `brand-identity` | Brand Identity | `crates/video-core/src/brand_identity` | `brand-system` |
| `designer` | Designer | `crates/video-core/src/designer` | `creative-plan`, `creative-types` |
| `writing` | Writing | `crates/video-core/src/writing` | `copy-and-captions` |
| `social` | Social | `crates/video-core/src/social` | `platform-fit` |
| `planning` | Creative Planning | `crates/video-core/src/creative_plan` | `creative-plan`, `bake-off` |
| `asset-validation` | Asset Validation | `crates/video-core/src/asset_validation` | `asset-rights` |
| `native-renderer` | Native Renderer | `crates/video-core/src/native_renderer` | `render-graph`, `finish-plan` |
| `native-typography` | Native Typography | `crates/video-core/src/native_typography` | `typography` |
| `native-motion` | Native Motion | `crates/video-core/src/native_motion` | `motion-grammar` |
| `native-audio` | Native Audio | `crates/video-core/src/native_audio` | `audio-finishing` |
| `render-graph` | Render Graph | `crates/video-core/src/render_graph` | `render-graph` |
| `creative-critic` | Creative Critic | `crates/video-core/src/creative_critic` | `visual-qa`, `critic-verdict` |
| `job-plane` | Job Plane | `crates/video-jobs` | `job-assignment` |

## 3. Exclusive file ownership

Each lane has an **exclusive file list**. A lane may freely modify any file in its own list. Cross-lane edits are gated by the **shared contract** list.

| Path | Owner |
|---|---|
| `schemas/creative/brand-card.schema.v2.json` | `brand` |
| `schemas/creative/brand-system.schema.v2.json` | `brand-identity` |
| `schemas/creative/style-direction.schema.v2.json` | `brand` |
| `schemas/creative/bakeoff.schema.v2.json` | `planning` |
| `schemas/render/render-graph.schema.v2.json` | `native-renderer` |
| `schemas/creative/critic-evaluation.schema.v2.json` | `creative-critic` |
| `schemas/creative/asset-request.schema.v2.json` | `designer` |
| `schemas/creative/asset-delivery.schema.v2.json` | `designer` |
| `schemas/creative/asset-review.schema.v2.json` | `asset-validation` |
| `schemas/skills/skill-request.schema.v1.json` | `planning` |
| `schemas/skills/skill-result.schema.v1.json` | `planning` |
| `schemas/skills/skill-trace.schema.v1.json` | `planning` |

## 4. Shared contracts

A contract is **shared** when two or more lanes must agree on its shape. The first-listed owner is the schema author; the second-listed owner is the schema reviewer.

| Contract | Author / Reviewer |
|---|---|
| `skill-request` | `planning` / `brand`, `designer`, `writing`, `social` |
| `asset-delivery` | `designer` / `writing`, `native-renderer` |
| `finish-plan` | `native-renderer` / `planning`, `creative-critic` |
| `render-graph` | `native-renderer` / `native-typography`, `native-motion`, `native-audio` |
| `critic-evaluation` | `creative-critic` / `brand`, `designer`, `planning` |

## 5. Producer-critic independence

The critic lane owns the verdicts. A producer lane (Brand, Designer, Writing, Social, Native Renderer, Native Audio, Native Motion, Native Typography) cannot write to the critic's `crates/`, schemas, or verdict surface. The critic reads from producer outputs only via **typed contracts** declared in `shared_contracts`.

## 6. Job plane boundary

The job plane (`crates/video-jobs`) owns:

- `decision_rights: job-assignment`
- The DAG of declared creative tasks.
- The handoff contract between lanes.

The job plane does **not** own: brand tokens, style directions, render graphs, finish plans, or critic verdicts. It only sequences the work.

## 7. Freeze scope

This freeze binds:

- The 14 lane ids.
- The 13 crate roots.
- The 14 decision rights.
- The exclusive file list.
- The shared contract list.

Any change to a lane id, a crate root, or a decision right requires a new frozen `v2` schema revision and a new dispatch task.
