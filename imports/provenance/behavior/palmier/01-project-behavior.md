# Palmier observed behavior: projects

Observed from public behavior and documentation of `palmier-pro` at commit
`397b82e64093f986cbabd89f1a1c93812ff546c2`. Behavior only; no upstream code,
types, schemas, or descriptions were copied. All future implementation must
use CutRight terminology and action contracts.

## PALM-B-PROJECT-001 — Project is the single source of truth

- Observable: a project is one self-contained package on disk that owns the
  edit document, media references, and derived artifacts. The editor treats
  the project package as canonical; side indexes or caches are disposable and
  rebuilt from the package.
- Constraints: filesystem work happens off the interactive thread; writes are
  staged outside the live package and installed atomically; operations that
  target the same package are serialized (save, import, generated-result
  install, export, close never race).
- Future CutRight mapping: `cutright://action/project.create`,
  `cutright://action/project.save`, and the CutRight project package read
  model. CutRight keeps its project package canonical and its index
  disposable (same principle as the v2 architecture's project-package truth).

## PALM-B-PROJECT-002 — Project settings are a typed, validated surface

- Observable: project-level settings (resolution, frame rate, duration
  domain) are read and changed through one validated operation; invalid
  values are refused before any mutation, and no-op changes do not create
  undo entries.
- Future CutRight mapping: `cutright://action/project.set_settings` with a
  typed settings payload validated before commit.

## PALM-B-PROJECT-003 — Close, save-as, and teardown preserve last good state

- Observable: closing, duplicating/saving-as, and quitting wait for admitted
  mutations, reject late commits, and preserve the latest successful state;
  user-requested file failures are surfaced, never hidden behind
  success-shaped responses.
- Future CutRight mapping: CutRight save/close lifecycle rules in the project
  service; failures reported as typed error states.

## PALM-B-PROJECT-004 — One project, multiple timelines

- Observable: a project can hold more than one timeline; exactly one timeline
  is active for editing, and activation is an explicit operation. Editing
  operations apply to the active timeline unless an explicit timeline
  identifier is supplied.
- Future CutRight mapping: `cutright://action/timeline.set_active` and the
  active-timeline field on the CutRight project read model.
