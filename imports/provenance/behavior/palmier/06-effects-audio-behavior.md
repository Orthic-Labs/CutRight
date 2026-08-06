# Palmier observed behavior: effects, color, and audio

Same observation provenance as 01-project-behavior.md.

## PALM-B-FX-001 — Effects attach to clips with keyframable parameters

- Observable: an effect is applied to a clip as a typed entry from an effect
  catalog, each with a parameter set; parameters can be keyframed over time
  (value at explicit timeline positions, interpolated between). Applying an
  effect is a mutation with a receipt and undo; unknown or unsupported
  effect identifiers are refused.
- Future CutRight mapping: CutRight effect registry
  (`schemas/effect-registry.schema.json`) plus
  `cutright://action/effect.apply` and `cutright://action/effect.set_keyframes`;
  keyframe interpolation math lives in the shared time/value module.

## PALM-B-FX-002 — Color is applied and inspected through one source of truth

- Observable: color adjustments are applied to clips through the same domain
  operation that preview and export use; the applied color state is
  inspectable as structured data. Preview, commit, and export never diverge
  in how the adjustment is calculated.
- Future CutRight mapping: `cutright://action/color.apply` and
  `cutright://read/color.state`; the CutRight renderer computes color once
  for preview, sample renders, and final export (composited-inspection
  parity, PALM-B-INSPECT-001).

## PALM-B-AUDIO-001 — Audio cleanup runs as a bounded job

- Observable: audio denoising is requested on a clip's audio; it runs as an
  asynchronous job producing a cleaned audio result bound to the clip; the
  job's terminal state is inspectable after the initiating call returns.
- Future CutRight mapping: CutRight audio stages run as typed jobs; see
  cutaway/finish golden stages for the native audio node equivalents.

## PALM-B-AUDIO-002 — Beat detection produces typed markers

- Observable: beat detection analyzes music media and produces beat markers
  in the timeline's frame domain; markers are data that editing operations
  (snap, cut) can consume; detection is a job with a typed result.
- Future CutRight mapping: `cutright://action/audio.detect_beats` returning
  frame-indexed markers; marker snapping is shared timeline math.
