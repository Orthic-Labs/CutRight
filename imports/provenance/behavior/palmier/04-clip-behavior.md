# Palmier observed behavior: clips

Same observation provenance as 01-project-behavior.md.

## PALM-B-CLIP-001 — Clips are placed by explicit typed mutations

- Observable: clips are added, inserted at positions, moved, removed, and
  split through separate validated operations. Every mutation validates the
  full request (target track, time ranges, identifiers) before touching
  state and applies multi-item changes atomically; a failed or refused
  mutation leaves the timeline exactly unchanged.
- Future CutRight mapping: `cutright://action/clip.add`, `clip.insert`,
  `clip.move`, `clip.remove`, `clip.split` — each with typed payloads
  validated before commit; atomic multi-entity apply.

## PALM-B-CLIP-002 — Ripple removal closes the gap

- Observable: removing ranges can ripple following material closed so the
  timeline stays continuous; plain removal leaves a gap. The two behaviors
  are distinct operations with distinct receipts.
- Future CutRight mapping: `cutright://action/clip.remove` takes an explicit
  `ripple: bool`; receipts report the shifted ranges.

## PALM-B-CLIP-003 — Clip properties are a validated property set

- Observable: per-clip properties (trim in/out, speed, scale, position,
  opacity, audio gain, name) are updated through one property-set operation;
  invalid values are refused; unchanged properties create no undo entry.
  Source-time trims stay in the source time domain; timeline placement stays
  in frames.
- Future CutRight mapping: `cutright://action/clip.set_properties` with the
  CutRight clip property schema; time-domain rules per
  PALM-B-TIMELINE-004.

## PALM-B-CLIP-004 — Media swap preserves edit position

- Observable: a clip's backing media can be swapped for another media entry
  while preserving timeline position and trim intent; incompatible media
  (e.g. duration shorter than the current usage) is refused or reported per
  an explicit contract, never silently truncated.
- Future CutRight mapping: `cutright://action/clip.swap_media` with typed
  incompatibility refusal reasons.

## PALM-B-CLIP-005 — Linked clips move as a unit

- Observable: clips can be linked (e.g. a picture clip with its audio or
  caption companions); moving or removing a linked clip acts on the whole
  link set unless explicitly unlinked; link management is itself a typed
  operation, and undo never leaves orphaned links.
- Future CutRight mapping: CutRight `link_group` on clip documents;
  `cutright://action/clip.manage_links` (link, unlink); undo restores link
  sets exactly.

## PALM-B-CLIP-006 — Sync keeps related clips aligned

- Observable: clips can be synchronized so edits to one ripple alignment
  changes to its sync group; sync state is inspectable and explicitly
  managed.
- Future CutRight mapping: `cutright://action/clip.sync` over CutRight sync
  groups; alignment math lives in the shared time module.

## PALM-B-CLIP-007 — Multicam groups switch angles by type

- Observable: multicam clips form groups with angle entries; switching the
  active angle is a typed operation on the group, and group state (angles,
  active angle, sync) is retrievable as a structured document.
- Future CutRight mapping: `cutright://action/clip.change_angle` and
  `cutright://read/multicam.group` in CutRight; the CutRight action
  vocabulary names this angle-switching, not a vendor concept.

## PALM-B-CLIP-008 — Mutation receipts describe exactly what changed

- Observable: every clip mutation returns a structured receipt: affected
  stable identifiers, what changed, explicit no-op status, warnings, skipped
  items, and typed errors. A success-shaped response is never returned when
  the outcome was adjusted or not achieved.
- Future CutRight mapping: CutRight action receipts follow the same contract
  (`CutRightActionReceipt`: status, changed ids, warnings, refusals); this
  shape also governs the director/agent loop's evidence.
