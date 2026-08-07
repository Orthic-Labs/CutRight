# V2 — Embedded Creative Operating System

Frozen by **CR-V2-B5-001**. Defines the product-local creative execution
contract: how the imported skills become typed services that the rest of the
application can call without ever invoking a sibling repo, a global skill
directory, a bare executable, Ollama, a cloud service or a downloaded browser.

This document is normative for tasks `CR-V2-B5-007..011` (skill runtime and
typed services), `CR-V2-B5-022..024` (merge/integration) and the freeze of
`Book 5`. Lane-ownership rules live in
[`V2-CREATIVE-RENDER-DAG.md`](V2-CREATIVE-RENDER-DAG.md).

## 1. Goals

1. Turn every imported skill into an **embedded, typed service** with a
   declared request envelope, deterministic result envelope and canonical
   trace envelope. No skill is callable as an opaque prompt.
2. Replace "skills-as-prompts" with "skills-as-evidence-bound planners".
   Skills consume read-only project evidence; they emit plans, requests,
   deliveries, reviews and action proposals — never raw timeline mutations
   or arbitrary filesystem writes.
3. Make selection, capability resolution and degradation explicit and
   reproducible.
4. Bound resource use and evidence retrieval on every call.

## 2. Schemas

The contract is encoded in three JSON Schemas (draft-07 subset) and the
companion Rust types in `crates/video-skills`. Every call must validate
against the schema in force.

| Schema id | Path | Purpose |
| --- | --- | --- |
| `cutright.skill_request/v1` | `schemas/skills/skill-request.schema.v1.json` | Caller-side request envelope |
| `cutright.skill_result/v1` | `schemas/skills/skill-result.schema.v1.json` | Caller-side typed artefact output |
| `cutright.skill_trace/v1` | `schemas/skills/skill-trace.schema.v1.json` | Canonical per-call trace |

These schemas:

- Reject **undeclared permissions** (e.g. `timeline_write`, `export`,
  `filesystem_raw`, `network`, `shell_exec`).
- Reject **undeclared model/runtime pack** capability strings.
- Require **input hash** and **output hash** (`blake3`, 64 hex chars).
- Require **evidence hashes** for every cited evidence node.
- Forbid `raw_reasoning` / `hidden_chain_of_thought` artefacts.
- Bound `evidence_budget` with explicit `max_items` / `max_bytes` /
  `used_items` / `used_bytes` on the trace.

## 3. Trait shape

```rust
pub trait SkillExecutor {
    fn execute(&self, request: SkillRequest, ctx: &SkillContext) -> Result<SkillResult>;
}

pub struct SkillContext {
    pub project: ProjectScope,
    pub revision: RevisionId,
    pub evidence: EvidenceService,
    pub capabilities: CapabilityView,
    pub output_staging: StagingScope,
}
```

- `SkillRequest` is the validated request envelope.
- `SkillContext` exposes only **read** views (`project`, `revision`,
  `evidence`, `capabilities`) and a writable **staging** scope that is
  scoped to the call.
- `SkillResult` is the validated result envelope. Every artefact carries a
  blake3 hash and the schema id it conforms to.

## 4. Permission and capability gating

The runtime resolves and gates the request **before any evidence is read**.
It MUST reject the request when:

1. The skill_id is not in the compiled embedded skill catalogue for the
   active creative pack.
2. The skill_revision is not equal to the compiled revision in the active
   creative pack.
3. Any requested permission is not declared by the skill's compiled
   permission set.
4. Any requested model/runtime capability cannot be resolved against the
   capability registry (`schemas/capabilities/registry.schema.v1.json`).
5. The signed creative pack hash does not match the loaded bundle.
6. The skill bytecode hash does not match the manifest.
7. The pack licence ledger does not authorise the requested capability.

When the request degrades (e.g. model absent, evidence thin, evidence drift)
the runtime follows the explicit `degradation_policy`:

- `abort` — refuse to run; result is `aborted` with `degradation_reason`.
- `deterministic_fallback` — fall back to the deterministic branch the
  skill declares; result is `ok_degraded`.
- `needs_review` — run the best deterministic branch and emit a verdict of
  `needs_review`; the result is `ok_degraded` plus a review artefact.

## 5. Resource budget

Every request declares `resource_budget.wallclock_ms`,
`memory_mb`, `tokens_in`, `tokens_out`, optional `model_calls`. The
runtime enforces these ceilings; exceeding them produces `budget_exceeded`
status. The runtime records actual usage in `resource_use` on the result.

## 6. Output staging

Skills may only write through the staging scope. Staging writes are:

- scoped to the active project and revision,
- hash-bound at write,
- never published until the caller explicitly promotes them,
- not visible to other concurrent skills unless explicitly shared.

## 7. Evidence access

Skill reads via `EvidenceService`:

- read-only windowed views over transcript, frame, audio segment, face
  track, logo track, label track, saliency map, document, brand card and
  style direction evidence,
- bounded by `evidence_budget` on the trace,
- recorded as `evidence_fetch` events with full evidence refs.

Skills MUST NOT:

- read outside `SkillContext.project`,
- write outside `SkillContext.output_staging`,
- invoke a sibling repo, global skill, bare executable, user Python/Node
  interpreter, Ollama, cloud service or downloaded browser,
- require raw hidden reasoning as a deliverable.

## 8. Determinism

When the request carries `deterministic: true`, the runtime:

1. Pins the skill bytecode hash, model version, runtime pack revision and
   signed creative pack hash.
2. Pins the seed.
3. Executes with explicit RNG and explicit ordering.
4. Re-validates the result hash before returning.

Two calls with identical `inputs`, `seed`, `pack` and skill_revision must
produce byte-identical result payloads.

## 9. Catalog and version pinning

The compiled embedded skill catalogue is loaded from a signed creative pack
resource. The runtime refuses to load skills from any other source. A
skill's `skill_revision` is bound to the compiled bytecode hash, so a
bytecode change is a revision change. Mismatched hashes fail closed.

## 10. Cross-references

- Capability registry: `schemas/capabilities/registry.schema.v1.json`,
  `docs/architecture/V2-CAPABILITY-ACTION-CONTRACT.md`.
- Action contract: `docs/architecture/V2-CAPABILITY-ACTION-CONTRACT.md`.
- Editorial immutability: `docs/architecture/V2-IDENTITY-TIME-REVISION.md`.
- Renderer (consumer, never producer): `docs/architecture/V2-NATIVE-RENDER-GRAPH.md`.
- Lane ownership: `docs/dispatch/v2/book-5/interface-freeze.md` and
  `docs/architecture/V2-CREATIVE-RENDER-DAG.md`.
