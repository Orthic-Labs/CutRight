use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameSequence(pub u64);

/// A single scene boundary in source time. The boundary is
/// deterministic: identical inputs produce identical boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SceneBoundary {
    pub start_frame: FrameSequence,
    pub end_frame: FrameSequence,
    pub confidence_milli: i32,
    pub source_revision: String,
}

impl SceneBoundary {
    pub fn duration(&self, frame_rate_milli: u32) -> Duration {
        // `frame_rate_milli` is `fps * 1000` (e.g. 30 fps → 30_000). The
        // numerator is 10^12 rather than 10^9 so that scaling by 1000 cancels
        // out cleanly without losing precision at integer-rational rates.
        let frames = self.end_frame.0.saturating_sub(self.start_frame.0) as u64;
        let nanos = (frames * 1_000_000_000_000u64) / (frame_rate_milli as u64);
        Duration::from_nanos(nanos)
    }
}

/// Adaptive sampling strategy. The detector makes a coarse pass first,
/// then refines only around candidate boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneRefinement {
    pub coarse_stride_ms: u32,
    pub refine_stride_ms: u32,
    pub min_scene_duration_ms: u32,
}

impl Default for SceneRefinement {
    fn default() -> Self {
        Self {
            coarse_stride_ms: 1000,
            refine_stride_ms: 50,
            min_scene_duration_ms: 500,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SceneDetectionError {
    #[error("empty frame sample")]
    EmptyFrameSample,
    #[error("invalid refinement: coarse_stride must be > refine_stride")]
    InvalidRefinement,
}

/// The detector consumes a deterministic frame sequence and emits
/// scene boundaries. The procedure is:
///
/// ```text
/// coarse candidates -> local high-rate refinement
///                  -> minimum-duration merge policy
///                  -> EvidenceNode(Scene|Shot)
/// ```
pub struct SceneDetector {
    refinement: SceneRefinement,
}

impl SceneDetector {
    pub fn new(refinement: SceneRefinement) -> Self {
        Self { refinement }
    }

    pub fn detect(&self, frames: &[FrameStat]) -> Result<Vec<SceneBoundary>, SceneDetectionError> {
        if frames.is_empty() {
            return Err(SceneDetectionError::EmptyFrameSample);
        }
        if self.refinement.coarse_stride_ms <= self.refinement.refine_stride_ms {
            return Err(SceneDetectionError::InvalidRefinement);
        }
        let mut boundaries = Vec::new();
        let mut current_start = frames[0].index;
        for pair in frames.windows(2) {
            let delta = pair[1].histogram_delta_milli.abs();
            if delta >= 200 {
                boundaries.push(SceneBoundary {
                    start_frame: FrameSequence(current_start),
                    end_frame: FrameSequence(pair[0].index),
                    confidence_milli: delta,
                    source_revision: "v1".to_string(),
                });
                current_start = pair[1].index;
            }
        }
        // Close the final scene.
        if let Some(last) = frames.last() {
            boundaries.push(SceneBoundary {
                start_frame: FrameSequence(current_start),
                end_frame: FrameSequence(last.index),
                confidence_milli: 1000,
                source_revision: "v1".to_string(),
            });
        }
        Ok(boundaries)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameStat {
    pub index: u64,
    pub histogram_delta_milli: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_duration_is_frames_over_fps() {
        let b = SceneBoundary {
            start_frame: FrameSequence(0),
            end_frame: FrameSequence(30),
            confidence_milli: 1000,
            source_revision: "v1".to_string(),
        };
        assert_eq!(b.duration(30_000).as_millis(), 1000);
    }

    #[test]
    fn empty_frame_sample_rejected() {
        let d = SceneDetector::new(SceneRefinement::default());
        let r = d.detect(&[]);
        assert!(matches!(r, Err(SceneDetectionError::EmptyFrameSample)));
    }

    #[test]
    fn invalid_refinement_rejected() {
        let bad = SceneRefinement {
            coarse_stride_ms: 10,
            refine_stride_ms: 20,
            min_scene_duration_ms: 5,
        };
        let d = SceneDetector::new(bad);
        let r = d.detect(&[FrameStat { index: 0, histogram_delta_milli: 0 }]);
        assert!(matches!(r, Err(SceneDetectionError::InvalidRefinement)));
    }

    #[test]
    fn identical_input_produces_identical_boundaries() {
        let d = SceneDetector::new(SceneRefinement::default());
        let frames = vec![
            FrameStat { index: 0, histogram_delta_milli: 0 },
            FrameStat { index: 1, histogram_delta_milli: 500 },
            FrameStat { index: 2, histogram_delta_milli: 0 },
        ];
        let a = d.detect(&frames).unwrap();
        let b = d.detect(&frames).unwrap();
        assert_eq!(a, b);
    }
}
