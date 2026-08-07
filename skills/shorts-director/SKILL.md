# shorts-director (Book 4 / B4-019)

Discover and rank short-form clips from evidence-bound beats.

## Inputs
- `beats`: array of `{ beat_id, evidence_ref }`
- `hook_strength`, `standalone_context`, `payoff`, `visual_support`,
  `boundary_confidence`, `duplication_penalty`: floats in [0,1]
- `recorded`: bool — must be true to enter the ranking

## Output
- ranked `ShortCandidate`s
- `rationale`: human-readable formula summary per candidate
- `exclusion_reason` for any candidate with `recorded=false`

## Invariants
- Never fabricate hook text.
- Never invent timestamps. The Rust layer compiles the selected
  beat IDs into the actual time ranges using evidence.
- Diversity-filter overlapping candidates (subset beats drop).
- Excluded candidates sort last.