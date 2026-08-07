# Review and correction

Every clip, beat, take, boundary, caption, graphic, effect and final
in a version can be reviewed.

* **Approve**: the decision is bound to the project's decision chain
  and the format's preference distribution is updated.
* **Reject**: the decision is recorded with the reason enum and
  triggers a re-plan of the affected segment.
* **Replace**: the decision records a structured delta (a new take,
  a new graphic, a new caption). The replacement is hashed and
  bound to the same project revision.
* **Note**: an optional free-text note is attached to the decision.
  A note is never required.

## Why a note is optional

Notes are useful, but a missing note never blocks a decision. The
reviewer can be in a hurry, on a phone, or simply out of words; the
decision still counts.

## Correction and undo

Every action has an inverse. The `Undo` command in Studio applies
the inverse action and records both actions in the decision log. A
correction is therefore also a decision, and the format's preference
distribution is updated accordingly.
