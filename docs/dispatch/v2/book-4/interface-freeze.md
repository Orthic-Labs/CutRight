# Book 4 Interface Freeze (CR-V2-B4-006)

This document is the binding contract between the three Book 4 parallel
lanes (A, B, C) and the serial integration tasks (022–027). It freezes
the public lane boundaries, the shared trait definitions, and the
ownership rules for the workspace.

## 1. Lane ownership

| Lane | Path | Purpose |
|---|---|---|
| A | `crates/video-benchmarks/**` | Benchmark evaluators, runner, report, profile |
| B | `crates/video-editorial/src/deterministic/**` | Beat, take, score, boundary, scoring, fault detection |
| C | `crates/video-editorial/src/narrative/**` | Arc templates, Director request, ordering, shorts, confidence, critic |

Concurrency model:

- A, B, C are independent inside Book 4 — they may edit only their own
  lane path.
- Shared documents (manifests, integration lists, capability drift) are
  owned by the serial integration tasks 022–027 only.
- The `EditorialEngine` façade is the only entry point that calls both
  lanes. It lives in `crates/video-editorial/src/engine.rs` and is owned
  by task 021.

## 2. Schema and trait surfaces

```text
crates/video-benchmarks/src/speech.rs            // lane A
crates/video-benchmarks/src/audio_visual.rs     // lane A
crates/video-benchmarks/src/audio.rs            // lane A
crates/video-benchmarks/src/visual.rs           // lane A
crates/video-benchmarks/src/crop.rs             // lane A
crates/video-benchmarks/src/collision.rs        // lane A
crates/video-benchmarks/src/reliability.rs      // lane A
crates/video-benchmarks/src/editorial.rs        // lane A
crates/video-benchmarks/src/runner.rs           // lane A
crates/video-benchmarks/src/report.rs           // lane A
crates/video-benchmarks/src/profile.rs          // lane A (added by 023)

crates/video-editorial/src/deterministic/beats.rs       // lane B
crates/video-editorial/src/deterministic/takes.rs       // lane B
crates/video-editorial/src/deterministic/scoring.rs     // lane B
crates/video-editorial/src/deterministic/faults.rs      // lane B
crates/video-editorial/src/deterministic/disfluency.rs  // lane B
crates/video-editorial/src/deterministic/dead_air.rs    // lane B
crates/video-editorial/src/deterministic/boundaries.rs  // lane B

crates/video-editorial/src/narrative/arcs.rs            // lane C
crates/video-editorial/src/narrative/provider.rs        // lane C
crates/video-editorial/src/narrative/order.rs           // lane C
crates/video-editorial/src/narrative/hook.rs            // lane C
crates/video-editorial/src/narrative/truthfulness.rs   // lane C
crates/video-editorial/src/narrative/shorts.rs          // lane C
crates/video-editorial/src/narrative/confidence.rs      // lane C
crates/video-editorial/src/narrative/critic.rs          // lane C
crates/video-editorial/src/engine.rs                    // lane C (021)
crates/video-editorial/src/plan.rs                      // lane C (021)
```

## 3. Shared traits

```text
trait BenchmarkEvaluator {
    fn id(&self) -> &str;
    fn axis(&self) -> Axis;
    fn evaluate(&self, ctx: &EvalContext) -> Result<EvalOutcome>;
}

trait EditorialProvider {
    fn propose(&self, request: EditorialRequest) -> Result<EditorialProposal>;
}

trait EditorialEngine {
    fn plan(&self, request: EditorialEngineRequest) -> Result<EditorialEngineResult>;
}
```

The three traits are the only entry points crossing lane boundaries:
`BenchmarkEvaluator` is consumed by the runner in lane A.
`EditorialProvider` is consumed by the engine in lane C. `EditorialEngine`
is the façade invoked by `video-project` (task 022).

## 4. Ownership rules

- A lane may not write another lane's source files.
- A lane may read schema files from `schemas/benchmarks/` and
  `schemas/editorial/` but must not modify them.
- A lane may not write to `Cargo.toml` or `Cargo.lock` (workspace
  manifest) outside the integration tasks 021 and 022.
- The benchmark runner is read-only against completed project revisions.
- Benchmark code NEVER writes production project state.

## 5. Integration chokepoints

The integration tasks own the following:

- `crates/video-project/src/editorial_v2.rs` (task 022) — compiles plan
  into variant timelines.
- `crates/video-project/src/cut_plan.rs` (task 022) — bound to the
  validated plan.
- `crates/video-project/src/timeline.rs` (task 022) — bound to the
  boundary consensus.
- `crates/video-project/src/autonomy_guard.rs` (task 023) — profile-aware
  review-mode guard.
- `apps/studio/src-tauri/src/editorial_commands.rs` (task 024) — Studio
  read API.
- `apps/studio/src/contracts/editorial.ts` (task 024) — Studio TS types.

No other task may modify these paths.

## 6. Acceptance criteria

- Lane roots do not overlap.
- Production code depends on evaluator interfaces, not benchmark fixtures.
- Benchmark runner is read-only against completed project revisions.
- Every lane's interfaces resolve without cross-lane imports.
