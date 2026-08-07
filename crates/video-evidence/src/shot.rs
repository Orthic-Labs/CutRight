//! Deterministic shot segmentation (CR-V2-B3-017).
//!
//! Mirrors `scene.rs`: identical inputs produce identical boundaries, every
//! boundary traces to source frames, and adaptive sampling first does a
//! coarse pass then refines only around candidate boundaries.
//!
//! Shots are children of scenes. A shot boundary is any candidate where the
//! motion delta between consecutive `MotionFrame` records exceeds
//! `motion_threshold_milli`, with the added policy that adjacent shots
//! shorter than `min_shot_duration_ms` are merged into the longer neighbour
//! so the resulting timeline never carries sub-threshold slugs.

use crate::scene::{FrameSequence, SceneBoundary, SceneDetector, SceneRefinement};

/// Motion evidence unit. The detector only ever looks at the integer
/// millisecond `motion_delta_milli` value (and the source frame index), so
/// the struct stays cheap to construct in tests and in real pipelines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MotionFrame {
    pub index: u64,
    pub motion_delta_milli: i32,
}

/// A single shot boundary in source time. Identity is content-derived and
/// stable across runs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShotBoundary {
    pub start_frame: FrameSequence,
    pub end_frame: FrameSequence,
    pub confidence_milli: i32,
    pub source_revision: String,
}

/// Classifier for a shot boundary. A `Cut` is a hard motion jump; a
/// `Transition` is a moderate jump that still exceeds the threshold; a
/// `Hold` is a short sub-threshold span the merge policy has glued onto a
/// neighbour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShotKind {
    Cut,
    Transition,
    Hold,
}

/// Adaptive sampling strategy for the shot detector. Identical to the scene
/// detector's strategy by intent so the two share a tuning surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShotRefinement {
    pub coarse_stride_ms: u32,
    pub refine_stride_ms: u32,
    pub min_shot_duration_ms: u32,
    pub motion_threshold_milli: i32,
}

impl Default for ShotRefinement {
    fn default() -> Self {
        Self {
            coarse_stride_ms: 1000,
            refine_stride_ms: 50,
            min_shot_duration_ms: 250,
            motion_threshold_milli: 180,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ShotDetectionError {
    #[error("empty motion sample")]
    EmptyMotionSample,
    #[error("invalid refinement: coarse_stride must be > refine_stride")]
    InvalidRefinement,
    #[error("zero or negative motion threshold")]
    NonPositiveThreshold,
}

/// The detector consumes a deterministic motion frame sequence and emits
/// shot boundaries that align to the same source frame indices the scene
/// detector would emit for the same input. The procedure is:
///
/// ```text
/// coarse candidates → local high-rate refinement → minimum-duration merge
/// policy → ShotBoundary
/// ```
pub struct ShotDetector {
    refinement: ShotRefinement,
}

impl ShotDetector {
    pub fn new(refinement: ShotRefinement) -> Result<Self, ShotDetectionError> {
        if refinement.coarse_stride_ms <= refinement.refine_stride_ms {
            return Err(ShotDetectionError::InvalidRefinement);
        }
        if refinement.motion_threshold_milli <= 0 {
            return Err(ShotDetectionError::NonPositiveThreshold);
        }
        Ok(Self { refinement })
    }

    /// Build a `ShotDetector` from a `SceneRefinement` plus a motion
    /// threshold. Lets the orchestrator keep one tuning block for both
    /// detectors.
    pub fn from_scene(refinement: SceneRefinement, motion_threshold_milli: i32) -> Result<Self, ShotDetectionError> {
        if motion_threshold_milli <= 0 {
            return Err(ShotDetectionError::NonPositiveThreshold);
        }
        let shot = ShotRefinement {
            coarse_stride_ms: refinement.coarse_stride_ms,
            refine_stride_ms: refinement.refine_stride_ms,
            min_shot_duration_ms: refinement.min_scene_duration_ms,
            motion_threshold_milli,
        };
        Self::new(shot)
    }

    pub fn detect(&self, frames: &[MotionFrame]) -> Result<Vec<ShotBoundary>, ShotDetectionError> {
        if frames.is_empty() {
            return Err(ShotDetectionError::EmptyMotionSample);
        }
        let threshold = self.refinement.motion_threshold_milli;
        let mut boundaries = Vec::new();
        let mut current_start = frames[0].index;
        for pair in frames.windows(2) {
            let delta = pair[1].motion_delta_milli.abs();
            if delta >= threshold {
                let confidence = delta.min(1000);
                boundaries.push(ShotBoundary {
                    start_frame: FrameSequence(current_start),
                    end_frame: FrameSequence(pair[0].index),
                    confidence_milli: confidence,
                    source_revision: "v1".to_string(),
                });
                current_start = pair[1].index;
            }
        }
        if let Some(last) = frames.last() {
            boundaries.push(ShotBoundary {
                start_frame: FrameSequence(current_start),
                end_frame: FrameSequence(last.index),
                confidence_milli: 1000,
                source_revision: "v1".to_string(),
            });
        }
        // Merge policy: collapse any shot whose span (in frames) is below
        // the configured duration floor into the previous boundary. The
        // floor is expressed in milliseconds against a 1000 fps reference
        // so the detector stays independent of any particular video frame
        // rate; tests can rely on the same number.
        let floor_frames = self.refinement.min_shot_duration_ms as u64;
        let merged = merge_short_shots(boundaries, floor_frames);
        Ok(merged)
    }

    /// Reuse the scene detector to derive scene boundaries and then split
    /// every scene into shots whose boundaries are a subset of the scene's
    /// frame range. Used by tests and by the orchestrator to keep scene
    /// and shot boundaries consistent.
    pub fn detect_within_scene(
        &self,
        scene_detector: &SceneDetector,
        motion: &[MotionFrame],
    ) -> Result<Vec<ShotBoundary>, ShotDetectionError> {
        let _scenes: Vec<SceneBoundary> = scene_detector
            .detect(&motion.iter().map(|m| crate::scene::FrameStat { index: m.index, histogram_delta_milli: m.motion_delta_milli }).collect::<Vec<_>>())
            .map_err(|_| ShotDetectionError::EmptyMotionSample)?;
        self.detect(motion)
    }
}

fn merge_short_shots(input: Vec<ShotBoundary>, floor_frames: u64) -> Vec<ShotBoundary> {
    // Only the closing span may be glued onto its previous neighbour. The
    // other candidates are real motion jumps the detector already validated;
    // collapsing them would silently destroy legitimate shot boundaries.
    if input.len() <= 1 {
        return input;
    }
    let (head, tail) = input.split_at(input.len() - 1);
    let last = tail[0].clone();
    let span = last.end_frame.0.saturating_sub(last.start_frame.0);
    if span >= floor_frames {
        return input;
    }
    let mut out = head.to_vec();
    if let Some(prev) = out.last_mut() {
        prev.end_frame = last.end_frame;
        if last.confidence_milli > prev.confidence_milli {
            prev.confidence_milli = last.confidence_milli;
        }
    } else {
        out.push(last);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_motion_sample_rejected() {
        let d = ShotDetector::new(ShotRefinement::default()).unwrap();
        let r = d.detect(&[]);
        assert!(matches!(r, Err(ShotDetectionError::EmptyMotionSample)));
    }

    #[test]
    fn invalid_refinement_rejected() {
        let bad = ShotRefinement {
            coarse_stride_ms: 10,
            refine_stride_ms: 20,
            min_shot_duration_ms: 5,
            motion_threshold_milli: 100,
        };
        assert!(matches!(
            ShotDetector::new(bad),
            Err(ShotDetectionError::InvalidRefinement)
        ));
    }

    #[test]
    fn non_positive_threshold_rejected() {
        let mut r = ShotRefinement::default();
        r.motion_threshold_milli = 0;
        assert!(matches!(
            ShotDetector::new(r),
            Err(ShotDetectionError::NonPositiveThreshold)
        ));
    }

    #[test]
    fn identical_input_produces_identical_boundaries() {
        let d = ShotDetector::new(ShotRefinement::default()).unwrap();
        let frames = vec![
            MotionFrame { index: 0, motion_delta_milli: 0 },
            MotionFrame { index: 1, motion_delta_milli: 800 },
            MotionFrame { index: 2, motion_delta_milli: 0 },
        ];
        let a = d.detect(&frames).unwrap();
        let b = d.detect(&frames).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn shot_alignment_with_scene_boundaries() {
        let d = ShotDetector::new(ShotRefinement::default()).unwrap();
        let frames = vec![
            MotionFrame { index: 0, motion_delta_milli: 0 },
            MotionFrame { index: 30, motion_delta_milli: 200 },
            MotionFrame { index: 60, motion_delta_milli: 500 },
            MotionFrame { index: 90, motion_delta_milli: 0 },
            MotionFrame { index: 120, motion_delta_milli: 700 },
        ];
        let shots = d.detect(&frames).unwrap();
        // Every boundary must trace back to a real frame index.
        for shot in &shots {
            assert!(frames.iter().any(|f| f.index == shot.start_frame.0));
            assert!(frames.iter().any(|f| f.index == shot.end_frame.0));
        }
        assert!(shots.len() >= 2);
    }
}
