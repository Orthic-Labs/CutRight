# Palmier observed behavior: text and captions

Same observation provenance as 01-project-behavior.md.

## PALM-B-TEXT-001 — Text overlays are timeline objects

- Observable: text items are added to text tracks with content, style, and a
  time range; they carry stable identifiers, participate in selection and
  undo like any other timeline object, and are updated through a typed
  update operation (content, style, position, range).
- Future CutRight mapping: `cutright://action/text.add` /
  `cutright://action/text.update` over the CutRight text-overlay document;
  text items get stable `object_id`s like clips.

## PALM-B-TEXT-002 — Layout uses a constrained position vocabulary

- Observable: text placement offers a small fixed vocabulary of anchored
  positions (corners and edge-centered placements) plus numeric overrides;
  layout changes are validated against that vocabulary.
- Future CutRight mapping: CutRight text layout schema with a closed enum of
  anchor positions; the effect/overlay safe-zone rules from the native
  renderer migration contract (CR-V2-B1-021) reuse the same vocabulary.

## PALM-B-CAPTION-001 — Captions derive from a transcript with word timing

- Observable: captioning starts from a transcript whose words carry time
  ranges; captions are generated onto a caption track as grouped word
  ranges with styling. The transcript is retrievable as a structured
  document tied to its source media.
- Future CutRight mapping: CutRight transcript pipeline (whisper.cpp
  verifier) produces the word-timed transcript; `cutright://action/caption.
  generate` consumes it; transcripts are evidence artifacts, not authority.

## PALM-B-CAPTION-002 — Transcript-driven media edits

- Observable: removing words from the transcript removes the corresponding
  media ranges (text-based editing), and removing silence removes detected
  silent ranges; both are timeline mutations with receipts and undo, and
  both can be previewed before commit.
- Future CutRight mapping: CutRight word-safe cut stages (see cutaway/finish
  golden behavior) — `cutright://action/edit.remove_words` and
  `cutright://action/edit.remove_silence`; preview and commit share one
  calculation.

## PALM-B-CAPTION-003 — Caption styling is data, not code

- Observable: caption appearance (font, size, color, background, position)
  is a per-project style profile applied uniformly, editable as typed data.
- Future CutRight mapping: CutRight caption-profile data documents (versioned
  data), never compiled styling logic.
