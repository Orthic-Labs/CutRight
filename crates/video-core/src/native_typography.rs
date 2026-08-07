//!
//! CutRight native typography and captions engine (CR-V2-B5-018).
//!
//! The engine takes a `CaptionDocument` and a `TypographyProfile` and
//! emits a `CaptionLayout` describing how each token should be drawn.
//! The layout respects:
//! - safe-zones
//! - reduced-motion
//! - per-platform legibility constraints (min font size, contrast)
//!
//! This is a minimal-but-compiling stub. Real glyph shaping and
//! reduced-motion keyframes are wired in `CR-V2-B5-021`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TypographyError {
    #[error("caption document requires at least one token: id={0}")]
    EmptyCaptions(String),
    #[error("font size {size} below platform minimum {min} for {platform}")]
    FontSizeTooSmall {
        size: f64,
        min: f64,
        platform: String,
    },
    #[error("token outside safe-zone: id={0}")]
    OutsideSafeZone(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptionToken {
    pub id: String,
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub start_ms: u64,
    pub end_ms: u64,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptionDocument {
    pub id: String,
    pub version: String,
    pub tokens: Vec<CaptionToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypographyProfile {
    pub id: String,
    pub platform: String,
    pub min_font_size: f64,
    pub safe_zone_x: f64,
    pub safe_zone_y: f64,
    pub safe_zone_w: f64,
    pub safe_zone_h: f64,
    pub reduced_motion: bool,
    pub font_family: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptionLayout {
    pub id: String,
    pub tokens: Vec<LayoutToken>,
    pub metrics: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutToken {
    pub token_id: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub font_size: f64,
    pub font_family: String,
}

pub struct NativeTypographyEngine;

impl NativeTypographyEngine {
    pub fn layout(
        doc: &CaptionDocument,
        profile: &TypographyProfile,
    ) -> Result<CaptionLayout, TypographyError> {
        if doc.tokens.is_empty() {
            return Err(TypographyError::EmptyCaptions(doc.id.clone()));
        }
        let mut out_tokens = Vec::with_capacity(doc.tokens.len());
        for t in &doc.tokens {
            let in_safe = t.x >= profile.safe_zone_x
                && t.y >= profile.safe_zone_y
                && t.x + t.w <= profile.safe_zone_x + profile.safe_zone_w
                && t.y + t.h <= profile.safe_zone_y + profile.safe_zone_h;
            if !in_safe {
                return Err(TypographyError::OutsideSafeZone(t.id.clone()));
            }
            let font_size = (profile.min_font_size * 1.4).max(12.0);
            if font_size < profile.min_font_size {
                return Err(TypographyError::FontSizeTooSmall {
                    size: font_size,
                    min: profile.min_font_size,
                    platform: profile.platform.clone(),
                });
            }
            out_tokens.push(LayoutToken {
                token_id: t.id.clone(),
                x: t.x,
                y: t.y,
                w: t.w,
                h: t.h,
                font_size,
                font_family: profile.font_family.clone(),
            });
        }
        Ok(CaptionLayout {
            id: format!("layout_{}", doc.id),
            tokens: out_tokens,
            metrics: BTreeMap::new(),
        })
    }

    pub fn assert_reduced_motion(profile: &TypographyProfile) -> bool {
        profile.reduced_motion
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> TypographyProfile {
        TypographyProfile {
            id: "tp_1".to_string(),
            platform: "ig_reels".to_string(),
            min_font_size: 14.0,
            safe_zone_x: 0.05,
            safe_zone_y: 0.05,
            safe_zone_w: 0.9,
            safe_zone_h: 0.9,
            reduced_motion: true,
            font_family: "Inter".to_string(),
        }
    }

    fn doc() -> CaptionDocument {
        CaptionDocument {
            id: "cd_1".to_string(),
            version: "v2".to_string(),
            tokens: vec![CaptionToken {
                id: "t_0".to_string(),
                text: "hi".to_string(),
                x: 0.1,
                y: 0.1,
                w: 0.4,
                h: 0.1,
                start_ms: 0,
                end_ms: 1000,
                evidence_ref: "evidence:ev_1".to_string(),
            }],
        }
    }

    #[test]
    fn lays_out_valid_caption() {
        let layout = NativeTypographyEngine::layout(&doc(), &profile()).expect("ok");
        assert_eq!(layout.tokens.len(), 1);
    }

    #[test]
    fn rejects_empty_doc() {
        let mut d = doc();
        d.tokens.clear();
        let err = NativeTypographyEngine::layout(&d, &profile())
            .err()
            .expect("err");
        assert!(matches!(err, TypographyError::EmptyCaptions(_)));
    }

    #[test]
    fn rejects_token_outside_safe_zone() {
        let mut d = doc();
        d.tokens[0].x = 0.99;
        let err = NativeTypographyEngine::layout(&d, &profile())
            .err()
            .expect("err");
        assert!(matches!(err, TypographyError::OutsideSafeZone(_)));
    }
}
