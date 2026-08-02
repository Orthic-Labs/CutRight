# caption-document.schema.json v1 fixtures

Used by `crates/video-project/src/caption_profile.rs::tests::valid_basic_fixture_round_trips_as_a_caption_document`
for the deserialize/serialize round-trip guard (REV2 plan §15.2), mirroring the pattern in
`fixtures/schemas/transcript/v1/`.

## valid/

- `basic.json` — two cues, no font notices. Must deserialize as
  `video_media::CaptionDocument`, round-trip byte-for-byte through `serde_json`.
