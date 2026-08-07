# CutRight v2 Feedback, Preference, and Autonomy

This document freezes the schema and operational contract for the v2 feedback
loop. It is the source of truth for the `video-feedback` crate and the Studio
panels that surface per-format autonomy. Anything not in this document is out
of scope for Book 7 Lane A.

## 1. Why a separate feedback chain

v2 separates three concerns that v1 conflated:

- **Decision records** — immutable, hash-bound, written once and never edited.
- **Preference estimates** — recomputed from decision records; explicit and
  evidence-cited.
- **Autonomy state** — per-format, derived from preference estimates plus
  benchmark, critic and integrity evidence.

User-specific preference evidence is never mixed with shared benchmark floors.
A shared benchmark is the floor; a user-specific preference is the ceiling.

## 2. Decision record contract

Every user or model action that affects a `DecisionTarget` (segment, beat,
take, boundary, caption, graphic, effect, audio, crop, final) is appended to a
hash-chained log. The record binds:

- `project_instance_id` and `project_revision`
- `subject_hash`, `asset_hash`, `effect_id`, `final_hash` (any subset)
- `format_key` (content_type × platform × variant)
- `pack_set_id` and `pack_set_fingerprint`
- `app_version`, `user_origin`, `session_origin`
- `decision_reason`, `decision_axis`, `decision_action`
- `delta` (structured, axis-specific)
- `confidence`, `sample_count`, `review_mode`

A stale or mismatched subject hash is **retained** and tagged `stale_subject`
but is **excluded from learning**. A malformed record is retained but its
`malformed` flag is set. **No record is silently dropped.**

Reasons and axes are exhaustive enums. Any unsupported axis uses
`unsupported_axis` or `unknown_reason`. The schema never invents a category.

## 3. Preference estimates

A preference estimate is a `PreferenceEstimate<T>` with:

- `distribution` — probability-like weights keyed by `decision_reason`
- `confidence`, `sample_count`, `variance`
- `evidence_decision_ids` — every decision whose hash proves the estimate
- `compatibility_fingerprint` — pack set + benchmark profile hash
- `scope` — `user_specific` or `shared_benchmark_floor`

A preference is `supported=false` when:

- the axis is unsupported
- samples are insufficient
- decisions conflict
- packs are incompatible
- only stale-subject records are available

A single project cannot produce a stable preference. Conflicting decisions
widen uncertainty or require explicit review. The estimate is **separate**
from any applied profile.

## 4. Applied format profiles

A `FormatProfile` is an explicit, versioned, user-approved profile. It is
constructed from a recommendation but is never auto-applied in reviewed mode.
A profile version is **immutable**; changes always create a new version.

A profile binds:

- `format` (content_type × platform × variant)
- `version` (monotonic, immutable)
- `compatibility` (pack set, benchmark profile, skill/render versions)
- `values` (inherited defaults + overridden values, kept distinct)
- `source_recommendation_hash`
- `approved_by`, `approved_at`

Application of a profile is blocked when `compatibility` does not match the
active project context. The user can inspect the source decision IDs for each
setting.

## 5. Autonomy advancement

For a format to advance from `reviewed` to `review_light` or `autonomous`:

- `benchmark_pass` must be true
- `user_approval_count` must meet the threshold
- `regression_count`, `critic_disagreement`, `integrity_failures` and
  `qa_failures` must be zero
- `last_user_approval` must be set (explicit user action required)

**No code path self-approves advancement.** The `last_user_approval`
timestamp is written only by the Studio review action.

## 6. Automatic demotion

Demotion is immediate and fires when **any** of:

- `rejected_final` decision attached to the format
- unresolved escalation blocks the format
- `benchmark_regression` registered
- `critic_disagreement` crosses threshold
- `integrity_failure` blocks the format
- the pack or profile set changes incompatibly

Every transition is written to `transition_history` with `audit_id`, `from`,
`to`, `reason` and `at`. Demotion returns the format to `reviewed`.

## 7. Studio controls

The Studio exposes:

- Exact reason enums (no free-form unless `note` is explicitly added)
- Source-decision IDs per profile value
- Threshold counts and last user approval timestamp
- Demotion log

The note field is opt-in. The user is never forced to add a note.

## 8. Schema ownership

| Schema | File |
| --- | --- |
| Decision record | `schemas/feedback/decision.schema.v2.json` |
| Preference estimate | `schemas/feedback/preferences.schema.v2.json` |
| Autonomy state | `schemas/feedback/autonomy.schema.v2.json` |
| Format profile | `schemas/feedback/format-profile.schema.v1.json` |
| Studio contract | `apps/studio/src/contracts/feedback.ts` |

## 9. Forbidden

- Mixing user-specific preferences with shared benchmark floors.
- Dropping decision records to keep schemas clean.
- Self-approval of autonomy advancement.
- Auto-applying an unapproved recommendation in reviewed mode.
- Treating autonomy as a security/integrity floor override.
