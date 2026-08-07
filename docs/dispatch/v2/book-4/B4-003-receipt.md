# CR-V2-B4-003 — metric definitions, units, aggregation, release floors

## Status

Schema and registry were authored for this task:

- `schemas/benchmarks/metric.schema.v1.json` — metric definition shape.
- `benchmarks/metrics/registry.json` — initial release-floor registry.
- `docs/benchmarks/V2-METRICS.md` — definitions + versioning rules.

## Co-committed path

The same three files were rolled into the concurrently-made
`CR-V2-B2-013` commit on `main` by an overlapping Book 2 lane. The
metric-shape work is correct and accepted. This task's intent is satisfied
by the registry, schema, and doc together with this receipt.

The next task (`CR-V2-B4-004`) is what binds the evaluator independence
rules to this registry.
