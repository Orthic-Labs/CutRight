# Palmier observed behavior: export

Same observation provenance as 01-project-behavior.md.

## PALM-B-EXPORT-001 — Export is a managed asynchronous job

- Observable: exporting renders the active timeline to a destination file
  through a typed settings payload (codec, resolution, frame rate, range).
  The export runs as a job: progress is observable, cancellation is
  supported, and the terminal state (succeeded with output path, failed
  with typed reason, cancelled) remains inspectable afterwards. Export
  never mutates the project document.
- Future CutRight mapping: `cutright://action/export.enqueue` producing a
  CutRight job; export settings validated against the CutRight export
  schema before the job starts.

## PALM-B-EXPORT-002 — Export registry is inspectable

- Observable: past and running exports can be listed with their settings,
  destinations, and states; an export can be re-run or cleaned up as an
  explicit operation. Output files are staged outside the live project
  package and installed atomically.
- Future CutRight mapping: `cutright://read/export.registry` and
  `cutright://action/export.manage` (rerun, remove); staging/atomic install
  per PALM-B-PROJECT-001.

## PALM-B-EXPORT-003 — Export shares the render path with preview

- Observable: export uses the same composition calculations as playback and
  inspection; a frame rendered for preview, inspection, and export is
  computed by one code path so results cannot diverge between surfaces.
- Future CutRight mapping: the CutRight native renderer is the single
  composition engine for preview, composited inspection, render samples, and
  final export (parity contract; see NATIVE-RENDERER-MIGRATION golden
  comparisons).
