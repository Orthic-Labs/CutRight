# Palmier observed behavior: identity, inspection, undo, variants, jobs

Same observation provenance as 01-project-behavior.md.

## PALM-B-IDENT-001 — Stable object identity for every timeline entity

- Observable: every project, timeline, track, clip, text, caption, media
  entry, and job carries a stable identifier that survives edits,
  reordering, and reopen. Positional indexes and display labels may be
  returned as context but are never the only durable identity after edits;
  automation addresses entities exclusively by stable identifier.
- Future CutRight mapping: CutRight `object_id` scheme across project,
  timeline, track, clip, overlay, media, and job documents; agent actions
  accept only ids, never indexes.

## PALM-B-INSPECT-001 — Composited timeline inspection

- Observable: the timeline can be inspected as a composited render: given a
  timeline and a frame window, the system produces rendered samples together
  with the visible objects and their stable identifiers for those frames.
  Inspection reads exactly what the editor would render — same calculations
  as playback and export — and never mutates the project.
- Future CutRight mapping: `cutright://read/timeline.composited_inspect`
  (input: timeline id + frame window; output: rendered samples + visible
  stable object ids) implemented on the CutRight native renderer; this is
  the evidence source for the director/visual-critic loop.

## PALM-B-UNDO-001 — One shared undo history for human and agent edits

- Observable: interactive edits and programmatic/agent edits flow through
  the same domain mutation operations and one undo history. One coherent
  user intent produces one undoable action; internal substeps are not
  exposed as separate entries. Validation happens before an undo group
  opens; failed, cancelled, refused, and unchanged operations create no undo
  entry. Undo restores exact state: no cumulative frame rounding, no
  derived-state drift, no orphaned links, no stale selection.
- Future CutRight mapping: CutRight undo service over action receipts; every
  CutRight action is validated pre-commit and is undoable as one unit or
  explicitly documented as non-undoable (jobs, exports).

## PALM-B-VARIANT-001 — Generated candidates are versioned variants

- Observable: iterative media creation keeps every produced version as a
  distinct, named media variant; the edit chooses among variants by swapping
  clip media; organization (renaming/grouping) keeps large variant sets
  maintainable, and the project remains the single context for all
  iterations rather than scattered external files.
- Future CutRight mapping: CutRight `MediaVariant` rows on media entries;
  variant selection via `cutright://action/clip.swap_media`. CutRight's own
  generation runs only through local, bundled runtimes (no hosted
  providers) — see rejected behaviors.

## PALM-B-JOB-001 — Async jobs are durable and inspectable

- Observable: long-running work (media transforms, audio cleanup, beat
  detection, generation, export) runs as jobs with explicit lifecycle
  states (queued, running, succeeded, failed, cancelled). A job's terminal
  result remains inspectable after the initiating call returns; asynchronous
  failures never disappear. Concurrent jobs are bounded per scarce resource,
  cancellation is cooperative, and cancelled work never commits partial
  results.
- Future CutRight mapping: CutRight typed job store with terminal-state
  receipts; bounded concurrency per runtime pack; cancellation checks at
  chunk boundaries. Job ids are stable objects per PALM-B-IDENT-001.

## PALM-B-SKILL-001 — Agent capability catalog is readable documentation

- Observable: the assistant exposes a catalog of capability descriptions
  that agents can read at runtime to learn supported workflows; capability
  documents are data read from a catalog, not executable code shipped to the
  agent.
- Future CutRight mapping: CutRight `skills/` catalog plus the v2 skill
  compiler/monitor (CR-V2-B1-018); skills are compiled into deterministic
  packs and read through `cutright://read/skill`.
