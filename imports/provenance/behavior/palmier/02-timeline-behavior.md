# Palmier observed behavior: timelines

Same observation provenance as 01-project-behavior.md (commit
`397b82e64093f986cbabd89f1a1c93812ff546c2`, public behavior/documentation
only, no upstream text copied).

## PALM-B-TIMELINE-001 — Active timeline selection

- Observable: projects support multiple timelines; one is active. Activation
  is explicit and changes which timeline receives unqualified edits. The
  active selection survives project reopening as persisted state.
- Future CutRight mapping: `cutright://action/timeline.set_active`; the
  CutRight project read model exposes `active_timeline_id`.

## PALM-B-TIMELINE-002 — Timeline creation and retrieval

- Observable: new timelines can be created with a validated settings payload
  (frame rate, frame size), and a timeline document can be fetched as a
  structured snapshot listing tracks, items, and their stable identifiers.
- Future CutRight mapping: `cutright://action/timeline.create` and
  `cutright://read/timeline.document`.

## PALM-B-TIMELINE-003 — Track structure management

- Observable: tracks are first-class containers: they can be added, removed,
  renamed, reordered, locked, and typed (video, audio, text, caption).
  Locked tracks refuse mutations through their items while remaining
  inspectable.
- Future CutRight mapping: `cutright://action/track.*` family (add, remove,
  rename, reorder, lock, unlock) over typed CutRight track documents.

## PALM-B-TIMELINE-004 — Time domain discipline: source seconds vs timeline frames

- Observable: media time is kept in exact rational time or frame-domain
  integers as long as possible; conversion to approximate floating-point
  happens only at UI/external-format boundaries. Two distinct time domains
  exist: a clip's source media time (seconds/rational within the source
  asset) and its timeline position (frame index on the timeline's frame
  rate). Operations preserve time scale, rounding rules, speed, and trim;
  rounding/clamping behavior is explicit and documented, never silent.
- Future CutRight mapping: CutRight keeps source time as rational seconds and
  timeline time as frame integers; the shared time math module owns
  conversion, rounding, and clamping rules (single source of truth used by
  preview, validation, commit, undo, and agent actions).

## PALM-B-TIMELINE-005 — Boundary and edge behavior

- Observable: editing surfaces must handle empty timelines, zero-duration
  media, missing tracks, offline media, variable frame rates, non-integer
  speeds, linked items, nested timelines, locked tracks, split items,
  overlapping items, and long-duration projects; invalid requests are
  refused with typed errors rather than silently adjusted.
- Future CutRight mapping: CutRight mutation validators cover the same edge
  matrix; refusal reasons are typed (`invalid_time_range`, `track_locked`,
  `media_offline`, ...) and no silent clamp/retarget occurs unless a
  contract promises and reports it.
