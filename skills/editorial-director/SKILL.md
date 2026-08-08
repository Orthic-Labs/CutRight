---
name: editorial-director
description: "Propose evidence-bound editorial beat order without emitting raw timestamps."
---

# Editorial Director Skill

Book 4 lane C — B4-017 contract.

## Purpose
Schema-bound Director request → editorial proposal. The Director can
reference only supplied candidate IDs; it cannot emit raw timestamps.
Rust compiles selected beat IDs into source ranges.

## Inputs
- `EditorialRequest` from `video_editorial::narrative::provider`
- Arc template from `video_editorial::narrative::arcs::library`

## Outputs
- `EditorialProposal` validated by `validate_proposal`

## Constraints
- All selected takes must appear in the request's candidate list.
- `order` must be a permutation of `selected` IDs.
- `arc_id` must match a known arc template.
- `rationale` must be non-empty and cite evidence IDs.
- The proposal never contains raw timestamps.

## Validation
`validate_proposal(&request, &proposal)` returns `Ok(())` or
`DirectorError::UnknownCandidate | SchemaInvalid`.
