# CutRight v2 Studio Action Binding, Optimistic State, and Semantic-Diff UX

## 1. Purpose

This document freezes the action-binding contract that connects every UI
mutation in v2 Studio to a typed backend action. It governs how frontend
intents become backend-validated `ActionBatch`es, when semantic diffs must be
shown, and how the UI patches from persisted `ActionResult`s rather than
from optimistic guesses.

This contract is owned by the serial freeze tasks `CR-V2-B6-003` through
`CR-V2-B6-006`. Lane A, B, and C must not redefine it inside their exclusive
paths.

## 2. Authoritative schema

`schemas/studio/action-intent.schema.v1.json` is the wire shape for a
frontend-issued intent. The schema is closed (`additionalProperties: false`)
and the discriminator (`target.kind`) names which companion field is
required (`clip_id`, `track_id`, `beat_id`, `take_id`, `graphic_id`,
`caption_id`, `effect_id`, `asset_id`, `word_id`, `anchor_id`, `project_id`).

The backend validates every intent against this schema before any
`ActionBatch` is built. Unknown verbs are rejected; unknown discriminators
are rejected; unknown params are rejected per-verb by the executor.

## 3. Action pipeline

```text
intent → backend builds batch against observed revision
      → dry-run (validation + conflict + capability)
      → policy gate (low-risk reversible → direct apply; medium/high → confirm)
      → execute against staged clone
      → persist ActionResult + new revision + inverse batch + receipts
      → emit to UI; UI patches only from the persisted result
```

The frontend **never** mutates canonical project JSON directly. UI state
may render optimistically *during the dry-run* (so the user sees the
expected diff), but the persisted state always comes from the
`ActionResult`. A failed action leaves the UI aligned to the most recent
persisted state after the next refresh.

## 4. Optimistic state

Optimistic state is permitted only for:

- in-flight tooltips that show the dry-run outcome,
- selection / playhead / focused panel state,
- ephemeral previews during a drag operation.

Optimistic state is **forbidden** for:

- clip / track / beat / take identities,
- timeline revision pointer,
- selection history,
- QA / critic findings,
- final-selection history.

The rule is: anything that survives an app restart is read from the
project package or its persisted decisions file. UI state that would
otherwise persist must round-trip through the executor.

## 5. Semantic diff

`schemas/actions/semantic-diff.schema.v1.json` defines the shared diff
format. Every risky or multi-object action must display a diff before
confirmation. The diff always shows:

- before/after for the affected object,
- the new revision id and its parent,
- evidence references,
- receipts generated so far,
- downstream invalidations (which finals/auditions/QA entries become stale).

Low-risk reversible local actions may skip the modal and apply directly
through the executor. The decision is policy-driven and lives in the
backend capability registry; the frontend never invents "low-risk" by
itself.

## 6. Stale revision refresh and conflict UX

When the executor returns a `revision_conflict` error, the UI must:

1. discard optimistic state for the affected object,
2. pull the current revision from the backend,
3. show a "this action was made against an older revision" panel,
4. offer "rebase + retry" or "discard".

The frontend does not retry the same intent against a newer revision
without re-running the dry-run against the new revision. Silent retry is a
test failure.

## 7. Risk policy

| Risk      | Examples                                              | Modal? | Reason required? |
| --------- | ----------------------------------------------------- | ------ | ---------------- |
| `low`     | edit a transcript word, disable an effect, undo       | No     | No               |
| `medium`  | trim / split / move a clip, reorder a beat            | Yes    | Optional         |
| `high`    | delete a final, replace an asset, rerun a stage       | Yes    | Yes              |

The policy table is generated from the capability registry tags
(`risk_band`) and is not editable in the UI.

## 8. Lane ownership

The action binding itself is shared. The serializers, executor, and
schema live outside any single lane. Lane A owns the UI binding for
core-workflow actions; Lane B owns the UI binding for authoring actions;
Lane C owns the UI binding for embedded-agent actions. The MCP surface
(Lane C) and the optional MCP project navigation (Lane C) reuse the same
intent shape; the executor is shared.

## 9. Anti-promises

- The UI never executes actions directly. Everything goes through the
  shared executor.
- The UI never invents an `ActionResult`. The result is always persisted.
- The UI never silently retries on `revision_conflict`.
- The UI never lets the user edit JSON to issue an action.