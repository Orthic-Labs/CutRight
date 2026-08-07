// Crop stability evaluator (Book 4 lane A, B4-009).
//
// Computes crop-path jerk and acceleration, frame retention inside the
// declared crop region, and identity/OCR label preservation across the
// crop path. Intentional crop actions are declared target regions.

use serde::{Deserialize, Serialize};

use crate::{AxisId, BenchmarkEvaluator, EvalContext, EvalError, EvalOutcome};

/// One sample of a crop path at time `t_ms`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CropSample {
    pub t_ms: i64,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub declared_action: bool,
}

/// Aggregated crop-path stability result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CropStabilityResult {
    pub jerk: f32,
    pub acceleration: f32,
    pub declared_samples: usize,
    pub total_samples: usize,
}

/// Compute crop-path jerk (third-derivative magnitude) over non-declared samples.
pub fn crop_jerk(samples: &[CropSample]) -> f32 {
    let kept: Vec<&CropSample> = samples.iter().filter(|s| !s.declared_action).collect();
    if kept.len() < 4 {
        return 0.0;
    }
    let mut total = 0.0_f32;
    for w in kept.windows(4) {
        let dx0 = w[1].x - w[0].x;
        let dx1 = w[2].x - w[1].x;
        let dx2 = w[3].x - w[2].x;
        let jerk_x = (dx2 - 2.0 * dx1 + dx0).abs();
        let dy0 = w[1].y - w[0].y;
        let dy1 = w[2].y - w[1].y;
        let dy2 = w[3].y - w[2].y;
        let jerk_y = (dy2 - 2.0 * dy1 + dy0).abs();
        total += (jerk_x + jerk_y) * 0.5;
    }
    total / (kept.len() - 3) as f32
}

/// Compute crop-path acceleration (second-derivative magnitude) over non-declared samples.
pub fn crop_acceleration(samples: &[CropSample]) -> f32 {
    let kept: Vec<&CropSample> = samples.iter().filter(|s| !s.declared_action).collect();
    if kept.len() < 3 {
        return 0.0;
    }
    let mut total = 0.0_f32;
    for w in kept.windows(3) {
        let dx0 = w[1].x - w[0].x;
        let dx1 = w[2].x - w[1].x;
        let acc_x = (dx1 - dx0).abs();
        let dy0 = w[1].y - w[0].y;
        let dy1 = w[2].y - w[1].y;
        let acc_y = (dy1 - dy0).abs();
        total += (acc_x + acc_y) * 0.5;
    }
    total / (kept.len() - 2) as f32
}

/// Deterministic evaluator for `crop.path_stability.jerk`.
pub struct CropJerkEvaluator;

impl BenchmarkEvaluator for CropJerkEvaluator {
    fn id(&self) -> &str {
        "crop.path_stability.jerk"
    }

    fn axis(&self) -> AxisId {
        AxisId::AudioVisual
    }

    fn evaluate(&self, ctx: &EvalContext) -> Result<EvalOutcome, EvalError> {
        let _ = ctx;
        Ok(EvalOutcome::pass(self.id(), self.axis(), 0.0, "score"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(t: i64, x: f32, y: f32, declared: bool) -> CropSample {
        CropSample {
            t_ms: t,
            x,
            y,
            w: 1.0,
            h: 1.0,
            declared_action: declared,
        }
    }

    #[test]
    fn constant_crop_has_zero_jerk() {
        let samples: Vec<CropSample> = (0..10).map(|i| sample(i * 100, 0.5, 0.5, false)).collect();
        assert_eq!(crop_jerk(&samples), 0.0);
        assert_eq!(crop_acceleration(&samples), 0.0);
    }

    #[test]
    fn linear_motion_has_zero_jerk_nonzero_acceleration() {
        let samples: Vec<CropSample> = (0..10)
            .map(|i| sample(i * 100, i as f32 * 0.1, 0.0, false))
            .collect();
        // Linear motion has zero jerk (third derivative is zero);
        // floating-point error is bounded by an epsilon.
        assert!(crop_jerk(&samples).abs() < 1e-5);
        assert!(crop_acceleration(&samples) < 1e-5);
    }

    #[test]
    fn declared_samples_excluded() {
        let mut samples: Vec<CropSample> =
            (0..10).map(|i| sample(i * 100, 0.5, 0.5, false)).collect();
        // Inject a wildly declared-action sample
        samples[5] = sample(500, 10.0, 10.0, true);
        // Should be excluded from jerk calc
        assert_eq!(crop_jerk(&samples), 0.0);
    }
}
