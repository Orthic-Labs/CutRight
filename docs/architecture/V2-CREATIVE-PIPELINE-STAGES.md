# V2 Creative Pipeline Stages and Handoff Contracts

**Status:** Frozen — `CR-V2-B5-005`
**Owner:** CutRight creative OS (Book 5)
**Schema:** `schemas/creative/pipeline-stage.schema.v2.json`

## 1. Purpose

Define the **stage graph** that every Book 5 FinishPlan walks through. Each stage has:
- A single producing lane.
- A single consuming lane.
- A typed **handoff contract** (input schema + output schema).
- A **budget** (max wall time, max files touched, max cost unit).
- A **critic hook** (which verdict gates the next stage).

A lane may not skip a stage. A lane may not run two stages of the same finish-plan in parallel.

## 2. The thirteen stages

```text
                 ┌──────────────┐
                 │ 1. BriefLock │  (planning)
                 └──────┬───────┘
                        ▼
                 ┌──────────────┐
                 │ 2. BrandBind │  (brand)
                 └──────┬───────┘
                        ▼
                 ┌──────────────┐
                 │ 3. StylePick │  (brand, planning)
                 └──────┬───────┘
                        ▼
                 ┌──────────────┐
                 │ 4. BakeOff   │  (planning)
                 └──────┬───────┘
                        ▼
                 ┌──────────────┐
                 │ 5. ShotPlan  │  (planning)
                 └──────┬───────┘
                        ▼
                 ┌──────────────┐
                 │ 6. RollPlan  │  (planning)
                 └──────┬───────┘
                        ▼
                 ┌──────────────┐
                 │ 7. AssetGate │  (asset-validation)
                 └──────┬───────┘
                        ▼
                 ┌──────────────┐
                 │ 8. Package   │  (writing, social)
                 └──────┬───────┘
                        ▼
                 ┌──────────────┐
                 │ 9. RenderGraph │ (native-renderer)
                 └──────┬───────┘
                        ▼
                 ┌──────────────┐
                 │ 10. Critique │  (creative-critic)
                 └──────┬───────┘
                        ▼
                 ┌──────────────┐
                 │ 11. Render   │  (native-renderer)
                 └──────┬───────┘
                        ▼
                 ┌──────────────┐
                 │ 12. QAVisu   │  (creative-critic — deterministic)
                 └──────┬───────┘
                        ▼
                 ┌──────────────┐
                 │ 13. Publish  │  (job-plane)
                 └──────────────┘
```

## 3. Stage contracts

| # | Stage | Producer | Consumer | Input | Output | Critic hook |
|---|---|---|---|---|---|---|
| 1 | `brief-lock` | `planning` | `brand` | `brief.v2` | `locked_brief.v2` | n/a |
| 2 | `brand-bind` | `brand` | `planning` | `locked_brief.v2` | `brand_card.v2` | `brand_alignment` |
| 3 | `style-pick` | `brand` | `planning` | `brand_card.v2` | `style_direction.v2` | `visual_composition` |
| 4 | `bake-off` | `planning` | `planning` | `style_direction.v2` | `bakeoff.v2` | `visual_composition` |
| 5 | `shot-plan` | `planning` | `planning` | `bakeoff.v2` | `shot_plan.v2` | `narrative_clarity` |
| 6 | `roll-plan` | `planning` | `asset-validation` | `shot_plan.v2` | `roll_plan.v2` | `narrative_clarity` |
| 7 | `asset-gate` | `asset-validation` | `writing`, `social` | `roll_plan.v2` | `asset_review.v2` | `rights_safety` |
| 8 | `package` | `writing`, `social` | `native-renderer` | `asset_review.v2` | `package.v2` | `typography_legibility`, `platform_fit` |
| 9 | `render-graph` | `native-renderer` | `creative-critic` | `package.v2` | `render_graph.v2` | `determinism` |
| 10 | `critique` | `creative-critic` | `native-renderer` | `render_graph.v2` | `critic_evaluation.v2` | all 10 axes |
| 11 | `render` | `native-renderer` | `creative-critic` | `critic_evaluation.v2` (pass/warn) | `rendered_artefact.v2` | `determinism` |
| 12 | `qa-visu` | `creative-critic` | `job-plane` | `rendered_artefact.v2` | `visual_qa.v2` | `determinism`, `accessibility` |
| 13 | `publish` | `job-plane` | external | `visual_qa.v2` (pass) | `published.v2` | none |

## 4. Handoff contract rules

1. **Producer lane is the only writer** of the output schema. The consumer lane may not mutate the producer's output.
2. **Critic hook is mandatory** before the next stage begins. If the hook returns `fail` or `blocked`, the next stage is rejected.
3. **Budget overrun** auto-fails the stage before the next stage is opened. The producer lane must publish a `BudgetExceeded` artefact and the planning lane must re-plan.
4. **Cross-lane references** must be by `id`, never by file path. The producer publishes its artefact to the `video-evidence` content-addressed store.
5. **Idempotent re-runs** are required. Re-running a stage with the same `input.id` and `seed` must produce the same `output.id`.

## 5. Stage DAG vs Job Plane DAG

The Job Plane DAG (`crates/video-jobs`) sequences the *finish plans*. The Creative Pipeline Stage DAG sequences the *stages of one finish plan*. The two DAGs are independent; the Job Plane emits one Stage DAG per finish plan.

## 6. Failure semantics

| Reason | Producer action | Critic action |
|---|---|---|
| Budget exceeded | `BudgetExceeded` artefact | n/a |
| Deterministic QA fails | n/a | `verdict: blocked` |
| Critic returns `fail` | re-plan via `planning` | n/a |
| Critic returns `blocked` | abort immediately | n/a |
| Job Plane publishes failure | n/a | n/a |

## 7. Freeze scope

This freeze binds:

- The 13 stage names.
- The 13 producer / consumer pairs.
- The 13 input / output schemas.
- The 10-axis critic hook.

Any change to a stage name, a producer/consumer pairing, or a critic hook requires a new frozen `v2` schema revision and a new dispatch task.
