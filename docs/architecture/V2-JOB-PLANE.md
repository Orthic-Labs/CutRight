# V2 Job Plane (CR-V2-B3-003)

## Purpose

Freeze the durable, content-addressed job plane that runs every CutRight
runtime execution. A job is the unit of resume, retry, and cancellation.
Success is never inferred from process exit alone.

## Three Schemas

- `schemas/jobs/job.schema.v1.json` — the top-level job.
- `schemas/jobs/stage.schema.v1.json` — a single stage inside a job.
- `schemas/jobs/fingerprint.schema.v1.json` — the content-derived fingerprint
  for cache-hit and resume.

## Job Lifecycle

A job has the following status transitions:

```
pending -> ready -> running -> succeeded | needs_review | failed | cancelled
              |          `-> cancelled
              `-> cancelled
```

`needs_review` is a terminal status that requires a human or downstream
process to resolve. `failed` is terminal; recovery requires a new job.

## Stage DAG

A job is a DAG of stages. Stages declare `depends_on` to express ordering.
The job runner refuses to schedule a stage whose dependencies are not in a
terminal status (`succeeded`, `needs_review`, `failed`, `cancelled`).

If any dependency is `failed`, the dependent stage is also `failed` (cascade).
If any dependency is `needs_review`, the dependent stage waits until the
dependency is resolved.

## Fingerprints

Every stage carries a fingerprint. The fingerprint is the BLAKE3 hash of a
canonicalised JSON object containing:

- `source_hashes` — SHA-256 and BLAKE3 of every source asset.
- `parameters_hash` — hash of the canonicalised parameters object.
- `capability` and `capability_version` — the producing capability.
- `pack_locks` — hashes of the lock files of every pack that contributed.
- `schemas` — URIs and hashes of every schema that constrained the stage.
- `preference_hash` — hash of the analyst's preference state.

A change in any field invalidates the fingerprint, which invalidates the
stage and every downstream stage.

## Inputs and Outputs

Stages declare typed inputs and outputs:

- Inputs bind to `source`, `asset`, `evidence_node`, `stage_output`,
  `literal`, or `preference`.
- Outputs are `evidence_node`, `asset`, `receipt`, or `report`.

Outputs are populated lazily, when the stage reaches a terminal status. The
`fingerprint` field on an output is the BLAKE3 hash of the canonicalised
output value.

## Retry

`retry_class` controls how the runner retries a failed stage:

- `none` — never retry.
- `transient` — retry once on transient errors.
- `idempotent` — safe to replay indefinitely until success.
- `user` — requires an explicit user action to retry.

`max_attempts` caps the attempt count. Each attempt is recorded in the
`attempts` array with its `status`, `error`, and stdout/stderr hashes.

## Cancellation

A stage can be cancelled at any time. `cancellation.requested_at_ns` and
`cancellation.actor_id` are recorded so the receipt can prove who triggered
the cancellation. A cancelled stage is terminal; the runner cannot resume
it.

## Resume

Resume is allowed only when every recorded input and output binding still
verifies. The runner walks the stage DAG and verifies each binding's
fingerprint against the cache. If any binding fails, the stage is
re-executed.

## Cache Hit

`cache_hit: true` is allowed only when the cached output fingerprint matches
the stage fingerprint. A stage with unverifiable output cannot be a cache
hit.

## Errors

Errors are structured: `{code, message, stage_id, retryable, diagnostics}`.
`code` is a stable dotted string. `message` is bounded. `retryable` is a
boolean. `diagnostics` is a bounded object.

## Acceptance

- A changed pack, parameter, or source invalidates only dependent stages.
- Cancellation and retry are explicit state transitions.
- A stage with unverifiable output cannot be a cache hit.
- Success is never inferred from process exit alone.
