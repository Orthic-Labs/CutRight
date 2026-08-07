# V2 Resource Policy (CR-V2-B3-004)

## Purpose

Freeze the bound on every resource the runtime may consume, and the
ordered degradation policy the runtime follows when those bounds are
exceeded. Cloud fallback, silent feature disablement, and unbounded
retries are forbidden.

## Two Schemas

- `schemas/runtime/resource-budget.schema.v1.json` — the bound.
- `schemas/runtime/degradation.schema.v1.json` — the ordered steps.

A default budget for the `laptop` target class is shipped at
`config/runtime/default-budgets.json`. Benchmark measurements replace the
default during qualification.

## Budget Fields

Every job is bound by:

- `cpu.threads` (1..256), `cpu.needs_gpu`, `cpu.simd` (none/sse4/avx2/avx512/neon).
- `memory.ram_mb`, `memory.vram_mb`, `memory.swap_mb`.
- `disk.disk_mb`, `disk.iops`.
- `processes` (1..1024).
- `fd` (64..65536).
- `temp_bytes` — max bytes written to temp directories for a single job.
- `wall_clock_ms` — max wall-clock execution time for a single job.
- `output_bytes` — max cumulative output bytes for a single job.
- `model_context.max_tokens`, `model_context.max_kv_cache_mb`.

## Target Classes

Five target classes are supported: `laptop`, `desktop`, `workstation`,
`server`, `ci`. Each ships a default budget; the default is conservative
and may be tightened during benchmark qualification.

## Degradation Steps

When a stage exceeds its budget, the runtime applies the registered
degradation steps in order:

1. `reduce_batch` — lower batch size or sample density.
2. `reduce_sample_density` — lower sampling rate or output frame rate.
3. `cpu_fallback` — drop accelerator and use CPU implementations.
4. `alternate_qualified_pack` — swap to a smaller but qualified pack.
5. `needs_review` — surface to human review with a `reason_code`.
6. `unsupported` — refuse with a `reason_code`.

Steps are tried in array order. The first step that succeeds transitions
the stage. If the last step fails, the stage is marked `needs_review` or
`unsupported` per the policy.

## Forbidden Operations

The `forbidden` field enumerates operations that are NEVER allowed in any
degradation step:

- `cloud_fallback` — silently offloading to a cloud service.
- `silent_feature_disable` — disabling a feature without surfacing it.
- `unbounded_retry` — retrying without a finite cap.

A policy that includes any of these in `steps` is rejected at validation
time.

## Coverage

The bound is enforced before scheduling a stage. The runtime refuses to
schedule a stage whose declared resources exceed the budget. The bound is
also enforced at execution time: the runtime polls resource counters
and aborts a stage that crosses a hard limit.

## Acceptance

- Cloud fallback is impossible.
- Silent feature disablement is impossible.
- Unbounded retries are impossible.
- Conservative defaults ship by target class; benchmark measurements
  replace them during qualification.
