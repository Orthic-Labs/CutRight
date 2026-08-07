# V2 — Capability registry and typed action contract

Frozen by **CR-V2-B2-002**.

## 1. Capability registry entry

Schema: `cutright.capability/v1`. Required fields: `capability_id` (snake_case), `version` (monotone int), `kind` (`read` or `mutation`), `owner_component`, `permission_set`, `inputs` (typed JSON-schema-like object), `outputs.bounded`, `outputs.windowed`, `eval_suites`, `degradation`.

Every mutation references ONE action schema. Every read declares `bounded: true` and `windowed: true`.

## 2. Read models vs mutation actions

Read models return bounded/windowed snapshots. Mutations are wrapped in action batches and produce a new revision + receipt. A mutation MUST NOT be invoked as a read; a read MUST NOT mutate. Enforced at load time.

## 3. Action batch envelope

Schema: `cutright.action_batch/v1`. Required: `batch_id`, `expected_revision`, `intent`, `evidence_refs[]`, `actions[]`, `dry_run`. snake_case JSON. Unknown fields fail closed.

## 4. Unknown action kinds

Unknown action kinds return `unknown_action_kind` referencing the unknown kind. Never silently dropped.
