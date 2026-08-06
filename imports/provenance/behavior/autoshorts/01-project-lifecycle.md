# Observed behavior: project library and lifecycle

Behavior observed at the pinned revision of the reference product.

## Observable behavior

- A dashboard lists all user projects as cards, newest activity first.
- Each card shows the project's human name, source recording length, and
  a current pipeline-stage indicator (e.g. "ingesting", "transcribing",
  "analyzing", "done", or a failure note).
- The user can create a project by choosing one local media file; the
  product records the file path and duration at creation time.
- The user can open, rename, and delete projects from the dashboard
  without opening them first.
- Deleting a project removes it from the library listing.
- Project state persists across application relaunches: reopening the app
  restores the same library and the same per-project stage indicators.

## Implementation-neutral constraints adopted by CutRight

- The project's on-disk package is canonical; the library listing is a
  rebuildable index, never the source of truth.
- Every project carries an observable stage value that the surface can
  render without running any pipeline work.

## Acceptance test statements

1. Given a project exists, when the app is relaunched, then the library
   lists the project with the same name and last-known stage.
2. When the user renames a project, then the library and the opened
   project both show the new name after a relaunch.
3. When the user deletes a project, then it no longer appears in the
   library and cannot be opened.
4. When creation is given a file the app cannot decode, then the project
   shows a failure stage instead of silently proceeding.
