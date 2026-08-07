// EditorialEngine façade merge receipt (Book 4 lane C, B4-022).

# EditorialEngine façade merge receipt

## Lanes merged

| Lane | Module                                  | Source commit |
|------|-----------------------------------------|---------------|
| A    | `crates/video-benchmarks`               | B4-009..011   |
| B    | `crates/video-editorial/src/deterministic` | B4-012..016 |
| C    | `crates/video-editorial/src/narrative`  | B4-017..021   |

## Façade sequence (engine.rs)

1. Retrieve evidence (caller supplies refs).
2. Deterministic candidates / features -> `ShortCandidate`s.
3. Director proposal (model output, supplied by caller).
4. Schema/semantic validation via `validate_proposal`.
5. Critic (`run_critic`).
6. Bounded revision: at most one; second disagreement escalates.
7. Reflection (`reflect`) + bounded repair (`attempt_repair`).
8. Final `EditorialPlan` aggregation.

## Frozen interfaces

- `EditorialEngineRequest`, `EditorialPlan`, `EditorialPlanResult`,
  `PlanError`, `EditorialEngine::plan`.
- `ShortCandidate`, `ConfidenceEstimate`, `OrderPlan`,
  `ReflectionReport`, `RepairAttempt`, `CriticOutcome`.
- All cross-lane contracts are `serde::{Serialize, Deserialize}`.

## Resolved conflicts

- `EditorialProposal` had two candidate shapes during planning;
  resolved to the original provider.rs shape (`selected`, `order`,
  `arc_id`, `rationale`, `evidence_refs`).
- `ShortInputs::beats` uses `&'a [ShortBeatRef]` to avoid heap
  copies inside the façade.

## Notes

- The façade never writes to disk; mutation remains with the
  ActionExecutor and the project layer.
- Evidence IDs are propagated unchanged through the façade.