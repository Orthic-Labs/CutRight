//! Project-level caption profile/model orchestration (REV2 plan §15.2).
//!
//! The canonical [`CaptionDocument`]/[`CaptionProfile`] types and the
//! word/phrase grouping algorithm live in `video-media::captions` — this
//! crate already depends on `video-media`, and `video-media` cannot depend
//! back on `video-project`, so that is the only crate either side can share.
//! This module re-exports those types for ergonomic use elsewhere in
//! `video-project`, and adds the project-scoped pieces that only make sense
//! at this layer: the default font/fallback chain CutRight ships with, and
//! [`build_default_caption_document`], the one call site `remap.rs` uses to
//! turn a variant's remapped transcript into the canonical caption artifact.

pub use video_media::{
    build_caption_document, resolve_font_for_text, CaptionCueModel, CaptionDocument,
    CaptionFontDescriptor, CaptionFontNotice, CaptionPlatform, CaptionProfile, CaptionSafeZone,
    CAPTION_MODEL_SCHEMA_VERSION,
};

use video_core::Word;

/// CutRight's default primary caption font. Latin-extended coverage handles
/// the accented characters most Western-European transcripts need without
/// falling back; anything outside that range (CJK, Cyrillic, emoji, …) goes
/// through the deterministic fallback chain below and is recorded as a
/// [`CaptionFontNotice`].
pub fn default_primary_font() -> CaptionFontDescriptor {
    CaptionFontDescriptor::latin_extended("IBM Plex Sans")
}

/// Ordered fallback chain consulted when the primary font is missing a
/// glyph. `Noto Sans` is the practical "covers almost everything" fallback;
/// `DejaVu Sans` is the final, always-available fallback so the chain never
/// leaves a resolution undetermined.
pub fn default_fallback_chain() -> Vec<CaptionFontDescriptor> {
    vec![
        CaptionFontDescriptor {
            name: "Noto Sans".into(),
            // Latin + Latin Extended-A/B + a broad general punctuation band;
            // deliberately not exhaustive (real coverage data belongs to the
            // font-rendering worker), but wide enough to resolve the common
            // non-ASCII cases the reading-speed/line-length tests exercise.
            coverage: vec![(0x0020, 0x024F), (0x2000, 0x206F)],
        },
        CaptionFontDescriptor {
            name: "DejaVu Sans".into(),
            coverage: vec![(0x0000, 0x10FFFF)],
        },
    ]
}

/// The profile CutRight uses for a project/variant-level caption sidecar
/// (SRT/VTT written alongside a variant's remapped transcript, before any
/// preset-specific burn). Presets pick their own platform profile via
/// [`CaptionProfile::for_platform`] at render time; this is the neutral
/// landscape default used for the generic sidecar.
pub fn default_profile() -> CaptionProfile {
    CaptionProfile::youtube_lower_third()
}

/// Build the canonical [`CaptionDocument`] for a variant's word timeline
/// using CutRight's default profile and font/fallback chain. The one call
/// site is `remap.rs`, which persists the result as the variant's canonical
/// caption artifact and derives the SRT/VTT sidecars from it.
pub fn build_default_caption_document(words: &[Word]) -> CaptionDocument {
    let profile = default_profile();
    let primary = default_primary_font();
    let fallback = default_fallback_chain();
    build_caption_document(words, &profile, &primary, &fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_BASIC: &str =
        include_str!("../../../fixtures/schemas/captions/v1/valid/basic.json");

    #[test]
    fn valid_basic_fixture_round_trips_as_a_caption_document() {
        let document: CaptionDocument =
            serde_json::from_str(VALID_BASIC).expect("valid/basic.json must deserialize");
        let reserialized = serde_json::to_value(&document).unwrap();
        let round_tripped: CaptionDocument = serde_json::from_value(reserialized).unwrap();
        assert_eq!(round_tripped, document);
        assert!(!document.cues.is_empty());
    }

    #[test]
    fn default_document_builder_is_deterministic() {
        let words = vec![
            Word {
                id: "w1".into(),
                source_word_id: None,
                text: "Hello".into(),
                start_ms: 0,
                end_ms: 300,
                confidence: 1.0,
                speaker: None,
                kind: "word".into(),
            },
            Word {
                id: "w2".into(),
                source_word_id: None,
                text: "world.".into(),
                start_ms: 300,
                end_ms: 700,
                confidence: 1.0,
                speaker: None,
                kind: "word".into(),
            },
        ];
        let first = build_default_caption_document(&words);
        let second = build_default_caption_document(&words);
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
        assert_eq!(first.profile_id, "youtube-lower-third.v1");
    }
}
