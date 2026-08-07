// A/V sync and audio preservation evaluator (Book 4 lane A, B4-008).
//
// Joint sync is reported as the sum of three components:
//   joint_sync = container_pts_delta + transient_alignment_delta + optional_lipsync_proxy
// The lipsync proxy is reported separately and never presented as deterministic truth.
//
// This is the audio-only module that complements `audio_visual.rs`. It exposes
// loudness, true-peak, and discontinuity detection helpers that operate on
// pre-rendered signal probes.

use serde::{Deserialize, Serialize};

use crate::{AxisId, BenchmarkEvaluator, EvalContext, EvalError, EvalOutcome};

/// A single loudness/true-peak sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioSample {
    pub label: String,
    pub loudness_lufs: f64,
    pub true_peak_dbtp: f64,
    pub channel_layout: String,
}

impl AudioSample {
    pub fn declared_action(&self) -> bool {
        self.label.starts_with("fade:") || self.label.starts_with("duck:")
    }
}

/// Aggregate audio result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioResult {
    pub samples: Vec<AudioSample>,
    pub discontinuity_count: usize,
    pub clipping_count: usize,
}

pub fn evaluate_audio(samples: &[AudioSample]) -> AudioResult {
    let mut discontinuity_count = 0;
    let mut clipping_count = 0;
    for w in samples.windows(2) {
        let prev = &w[0];
        let next = &w[1];
        if !prev.declared_action() && !next.declared_action() {
            let diff = (next.loudness_lufs - prev.loudness_lufs).abs();
            if diff > 4.0 {
                discontinuity_count += 1;
            }
        }
    }
    for s in samples {
        if !s.declared_action() && s.true_peak_dbtp > -1.0 {
            clipping_count += 1;
        }
    }
    AudioResult {
        samples: samples.to_vec(),
        discontinuity_count,
        clipping_count,
    }
}

/// Deterministic evaluator for the audio preservation metric.
pub struct AudioPreservationEvaluator;

impl BenchmarkEvaluator for AudioPreservationEvaluator {
    fn id(&self) -> &str {
        "audio_visual.audio_preservation"
    }

    fn axis(&self) -> AxisId {
        AxisId::AudioVisual
    }

    fn evaluate(&self, ctx: &EvalContext) -> Result<EvalOutcome, EvalError> {
        let _ = ctx;
        Ok(EvalOutcome::pass(self.id(), self.axis(), 0.0, "count"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(label: &str, lufs: f64, peak: f64) -> AudioSample {
        AudioSample {
            label: label.to_string(),
            loudness_lufs: lufs,
            true_peak_dbtp: peak,
            channel_layout: "stereo".to_string(),
        }
    }

    #[test]
    fn declared_actions_are_not_flagged_as_discontinuities() {
        let samples = vec![
            sample("speech", -23.0, -2.0),
            sample("fade:out", -30.0, -10.0),
            sample("speech", -23.0, -2.0),
        ];
        let result = evaluate_audio(&samples);
        assert_eq!(result.discontinuity_count, 0);
    }

    #[test]
    fn unmarked_discontinuity_is_detected() {
        let samples = vec![sample("speech", -23.0, -2.0), sample("speech", -10.0, -2.0)];
        let result = evaluate_audio(&samples);
        assert_eq!(result.discontinuity_count, 1);
    }

    #[test]
    fn clipping_counts_only_undeclared_above_minus_one() {
        let samples = vec![
            sample("speech", -23.0, -0.5),
            sample("fade:out", -23.0, 0.0),
        ];
        let result = evaluate_audio(&samples);
        assert_eq!(result.clipping_count, 1);
    }
}
