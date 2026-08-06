# Observed behavior: progress, relaunch recovery, and export

Behavior observed at the pinned revision of the reference product.

## Observable behavior

- While any long-running stage works (download, analysis, render), the
  surface shows a live indicator naming which item is being worked on.
- If the application is quit mid-pipeline and relaunched, the project card
  still shows the stage it was in, and the user can continue from that
  point rather than starting over.
- A render that is interrupted by quitting the app does not corrupt the
  project: on relaunch the candidate returns to a not-rendered state and
  can be rendered again cleanly.
- Completed clips are exported as standalone vertical video files the user
  can open outside the application; the export location is surfaced to the
  user.
- Render failures attach a readable failure note to the affected candidate
  rather than failing the whole project.

## Implementation-neutral constraints adopted by CutRight

- Progress is attributed to a named item (project / candidate), never a
  bare global spinner.
- Recovery is by stage, not by whole-pipeline replay: the last-known stage
  is visible and resumable after relaunch.
- An interrupted render leaves no partial artifact presented as complete.
- CutRight rejects the observed product's storage model: the project
  package on disk is canonical, and any index is disposable (the observed
  product treats its database as the canonical truth).

## Acceptance test statements

1. Given a render is in flight, when the app is quit and relaunched, then
   the project reopens at its last-known stage within one refresh and no
   partial clip is shown as ready.
2. When the interrupted candidate is rendered again, then it completes to
   a ready state with a playable clip.
3. When a single render fails, then that candidate shows the failure note
   and all other candidates remain actionable.
4. When a clip completes, then the user is shown where the exported file
   lives and can open it with a system player.
5. Given the index store is deleted while the app is closed, then on
   relaunch the library can be rebuilt from the project packages without
   data loss.
