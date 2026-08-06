# Observed behavior: one-click pipeline

Behavior observed at the pinned revision of the reference product.

## Observable behavior

- From a newly created project the user can start the whole chain with a
  single action; no per-stage configuration is required to begin.
- The chain runs in a fixed order: extract the audio track from the source
  recording, produce a transcript of that audio, analyze the transcript for
  high-impact moments, and rank those moments into a candidate list.
- Each stage writes a durable result that the next stage consumes; stages
  do not re-run work that already completed.
- The project's visible stage indicator advances as the chain moves, and
  any stage failure stops the chain with a visible, plain-language error.
- The chain can be started again after a failure; completed earlier stages
  are reused rather than recomputed.

## Implementation-neutral constraints adopted by CutRight

- One user gesture launches the full chain; intermediate stages are not
  user-facing jobs.
- Stage order is fixed and each stage's output is a durable artifact the
  next stage reads — no stage holds ephemeral state across the chain.
- CutRight adds evidence reconciliation the observed product lacks: model
  outputs are proposals, and every proposed boundary is checked against
  transcript and decode evidence before a cut is committed (rejected
  behavior: taking model timestamps as canonical).

## Acceptance test statements

1. Given a project with a decodable source, when the single start action
   is triggered, then the chain runs to a ranked candidate list without
   further user input.
2. When a middle stage fails, then the chain stops at that stage, the
   project shows the failing stage, and no later stage runs.
3. Given the chain failed at the analysis stage, when the start action is
   triggered again, then audio extraction and transcription are reused and
   only the failed stage onward runs.
4. When the chain completes, then every candidate carries a rank order and
   a human-readable reason string.
