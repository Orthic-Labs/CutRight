# Palmier observed behavior: media

Same observation provenance as 01-project-behavior.md.

## PALM-B-MEDIA-001 — Import copies media into the project package

- Observable: importing media validates the source, probes its properties,
  and installs a managed copy inside the project package through the
  serialized package writer; the import produces a media entry with a stable
  identifier. Importing never writes directly into the live package from
  feature code, and identical/invalid sources are refused before any file
  work.
- Future CutRight mapping: `cutright://action/media.import` producing a
  typed `MediaEntry` with stable `media_id`; package writes routed through
  the CutRight package coordinator equivalent.

## PALM-B-MEDIA-002 — Media probing reports exact technical properties

- Observable: each media entry exposes probed properties (duration, frame
  rate, dimensions, rotation, audio layout, availability state) loaded
  asynchronously; offline or corrupt media is reported as a typed state, not
  as a crash or an empty success.
- Future CutRight mapping: `cutright://read/media.properties` on the CutRight
  media read model; states: `ready`, `probing`, `offline`, `unsupported`.

## PALM-B-MEDIA-003 — Library organization and search

- Observable: the media library supports organizing entries (grouping,
  renaming, tagging) and searching by name/metadata; both are plain
  mutations with receipts and undo.
- Future CutRight mapping: `cutright://action/media.organize` and
  `cutright://read/media.search` over CutRight media metadata.

## PALM-B-MEDIA-004 — Frame capture from media

- Observable: a still frame can be captured from a media entry at an exact
  source time; output is a derived asset bound back to the source entry and
  time.
- Future CutRight mapping: `cutright://action/media.capture_frame` producing
  a derived-image asset with source media id + rational source time recorded
  in its provenance.

## PALM-B-MEDIA-005 — Derived media transformations are jobs, not inline edits

- Observable: expensive media transformations (e.g. upscaling, audio
  cleanup) run as asynchronous jobs; results install as new managed media
  entries; the source entry is never mutated in place.
- Future CutRight mapping: typed CutRight async jobs (see
  07-jobs-undo-identity.md) — `cutright://action/media.transform` enqueues a
  job; the source entry stays immutable.

## PALM-B-MEDIA-006 — Lazy hydration

- Observable: thumbnails, waveforms, filmstrips, and metadata are hydrated
  only when a consumer needs them; bulk grids never eagerly decode media.
- Future CutRight mapping: CutRight media read model hydrates derived
  artifacts on demand with bounded concurrency and explicit cache
  invalidation.
