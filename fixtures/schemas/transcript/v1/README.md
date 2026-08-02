# transcript.schema.json v1 fixtures

Used by `crates/video-core/src/models.rs::tests::transcript_schema_v1_fixtures_*` for the
deserialize/serialize round-trip guard (REV2 plan §8.4/§8.5).

## valid/

- `basic.json` — two words, one with an optional `source_word_id`, one without. Must deserialize,
  round-trip byte-for-byte through `serde_json`, and pass `Transcript::validate()`.

## invalid/

Each fixture violates exactly one contract rule so the failure mode is unambiguous:

- `missing_required_field.json` — a word omits the required `confidence` field
  (schema `words.items.required`). Rust: `serde_json::from_str::<Transcript>` fails.
- `unknown_field.json` — a word carries an extra `unexpected_field`
  (schema `additionalProperties: false`, REV2 §8.4 strict unknown-field handling). Rust:
  `#[serde(deny_unknown_fields)]` on `Word` makes `from_str` fail.
- `bad_source_word_id_pattern.json` — `source_word_id` has no `:` separator, so it does not match
  the compound `<source_id>:<word_id>` pattern (schema `words.items.properties.source_word_id.pattern`).
  This is shape-valid JSON, so it deserializes; `Transcript::validate()` must reject it via
  `is_valid_source_word_id`.
- `unsorted_word_timeline.json` — the second word starts before the first word ends (REV2 §8.5
  sorted/non-overlapping word timelines). Deserializes; `Transcript::validate()` must reject it.
