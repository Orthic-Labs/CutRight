//!
//! CutRight native motion grammar, reframing, and temporal placement
//! (CR-V2-B5-019).
//!
//! The motion lane owns:
//! - a *motion grammar* — a per-platform catalogue of allowed transitions
//! - a *reframing* algorithm — fits a source clip into a target aspect
//!   ratio by pan-and-scan with safe-zone protection
//! - a *temporal placement* — schedules motion beats inside a shot
//!
//! The full GPU-bound runtime is wired in `CR-V2-B5-021`. This is a
//! minimal-but-compiling shape.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MotionError {
    #[error("transition {0} is not allowed for platform {1}")]
    ForbiddenTransition(String, String),
    #[error("reframing would expose unsafe region: id={0}")]
    UnsafeReframe(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionBeat {
    pub id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub transition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionClip {
    pub id: String,
    pub version: String,
    pub beats: Vec<MotionBeat>,
    pub allowed_transitions: Vec<String>,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reframe {
    pub clip_id: String,
    pub src_w: u32,
    pub src_h: u32,
    pub dst_w: u32,
    pub dst_h: u32,
    pub pan_x: f64,
    pub pan_y: f64,
    pub scale: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Placement {
    pub beat_id: String,
    pub shot_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

pub struct NativeMotionEngine;

impl NativeMotionEngine {
    pub fn validate(clip: &MotionClip) -> Result<(), MotionError> {
        for b in &clip.beats {
            if !clip.allowed_transitions.iter().any(|t| t == &b.transition) {
                return Err(MotionError::ForbiddenTransition(
                    b.transition.clone(),
                    clip.platform.clone(),
                ));
            }
        }
        Ok(())
    }

    pub fn reframe(clip_id: &str, src: (u32, u32), dst: (u32, u32)) -> Reframe {
        let scale = (dst.0 as f64 / src.0 as f64).max(dst.1 as f64 / src.1 as f64);
        let pan_x = ((src.0 as f64 * scale) - dst.0 as f64) / 2.0;
        let pan_y = ((src.1 as f64 * scale) - dst.1 as f64) / 2.0;
        Reframe {
            clip_id: clip_id.to_string(),
            src_w: src.0,
            src_h: src.1,
            dst_w: dst.0,
            dst_h: dst.1,
            pan_x,
            pan_y,
            scale,
        }
    }

    pub fn place(beat: &MotionBeat, shot_id: &str) -> Placement {
        Placement {
            beat_id: beat.id.clone(),
            shot_id: shot_id.to_string(),
            start_ms: beat.start_ms,
            end_ms: beat.end_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip() -> MotionClip {
        MotionClip {
            id: "mc_1".to_string(),
            version: "v2".to_string(),
            beats: vec![MotionBeat {
                id: "mb_0".to_string(),
                start_ms: 0,
                end_ms: 1000,
                transition: "cut".to_string(),
            }],
            allowed_transitions: vec!["cut".to_string(), "fade".to_string()],
            platform: "ig_reels".to_string(),
        }
    }

    #[test]
    fn accepts_allowed_transition() {
        NativeMotionEngine::validate(&clip()).expect("ok");
    }

    #[test]
    fn rejects_forbidden_transition() {
        let mut c = clip();
        c.beats[0].transition = "wipe_secret".to_string();
        let err = NativeMotionEngine::validate(&c).err().expect("err");
        assert!(matches!(err, MotionError::ForbiddenTransition(_, _)));
    }

    #[test]
    fn reframes_to_aspect() {
        let r = NativeMotionEngine::reframe("mc_1", (1920, 1080), (1080, 1920));
        assert_eq!(r.dst_w, 1080);
        assert!(r.scale > 0.0);
    }
}
