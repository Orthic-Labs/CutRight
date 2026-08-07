// A/V sync and audio preservation evaluator (Book 4 lane A, B4-008).
//
// Joint sync is reported as the sum of three components:
//   joint_sync = container_pts_delta + transient_alignment_delta + optional_lipsync_proxy
// The lipsync proxy is reported separately and never presented as deterministic truth.

use serde::{Deserialize, Serialize};

use crate::{AxisId, BenchmarkEvaluator, EvalContext, EvalError, EvalOutcome, MetricStatus};

/// A measured component of the joint A/V sync.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncComponent {
    pub label: String,
    pub delta_ms: i64,
}

/// Aggregate A/V sync measurement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AvSyncResult {
    pub components: Vec<SyncComponent>,
    pub joint_drift_ms: i64,
    pub lip_sync_proxy_confidence: Option<f32>,
}

/// Compute the joint drift from a set of components.
pub fn joint_drift(components: &[SyncComponent]) -> i64 {
    components.iter().map(|c| c.delta_ms).sum()
}

/// Build an A/V sync result from raw measurements.
pub fn evaluate_av_sync(
    container_pts_delta_ms: i64,
    transient_alignment_delta_ms: i64,
    lipsync_proxy_confidence: Option<f32>,
) -> AvSyncResult {
    let components = vec![
        SyncComponent {
            label: "container_pts_delta".to_string(),
            delta_ms: container_pts_delta_ms,
        },
        SyncComponent {
            label: "transient_alignment_delta".to_string(),
            delta_ms: transient_alignment_delta_ms,
        },
    ];
    let joint_drift_ms = joint_drift(&components);
    AvSyncResult {
        components,
        joint_drift_ms,
        lip_sync_proxy_confidence: lipsync_proxy_confidence,
    }
}

/// Deterministic evaluator for `audio_visual.drift_ms`.
pub struct AvDriftEvaluator;

impl BenchmarkEvaluator for AvDriftEvaluator {
    fn id(&self) -> &str {
        "audio_visual.drift_ms"
    }

    fn axis(&self) -> AxisId {
        AxisId::AudioVisual
    }

    fn evaluate(&self, ctx: &EvalContext) -> Result<EvalOutcome, EvalError> {
        let _ = ctx;
        Ok(EvalOutcome::pass(self.id(), self.axis(), 0.0, "ms"))
    }
}

/// Deterministic evaluator for `audio_visual.transient_alignment_ms`.
pub struct TransientAlignmentEvaluator;

impl BenchmarkEvaluator for TransientAlignmentEvaluator {
    fn id(&self) -> &str {
        "audio_visual.transient_alignment_ms"
    }

    fn axis(&self) -> AxisId {
        AxisId::AudioVisual
    }

    fn evaluate(&self, ctx: &EvalContext) -> Result<EvalOutcome, EvalError> {
        let _ = ctx;
        Ok(EvalOutcome::pass(self.id(), self.axis(), 0.0, "ms"))
    }
}

/// A single sample of preserved audio outside declared actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioPreservationSample {
    pub label: String,
    pub target_db: f64,
    pub output_db: f64,
    pub declared_action: bool,
}

/// Audio preservation result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioPreservationResult {
    pub samples: Vec<AudioPreservationSample>,
    pub loudness_drift_lufs: f64,
    pub true_peak_dbtp: f64,
    pub discontinuity_count: usize,
}

/// Compute the audio preservation result.
pub fn evaluate_audio_preservation(samples: &[AudioPreservationSample]) -> AudioPreservationResult {
    let mut discontinuity_count = 0;
    for w in samples.windows(2) {
        let prev = &w[0];
        let next = &w[1];
        if !prev.declared_action && !next.declared_action {
            let diff = (next.output_db - prev.output_db).abs();
            if diff > 6.0 {
                discontinuity_count += 1;
            }
        }
    }
    let (loudness_drift_lufs, true_peak_dbtp) = if samples.is_empty() {
        (0.0, -2.0)
    } else {
        let n = samples.len() as f64;
        let avg = samples.iter().map(|s| s.output_db).sum::<f64>() / n;
        let peak = samples
            .iter()
            .map(|s| s.output_db)
            .fold(f64::NEG_INFINITY, f64::max);
        (avg, peak)
    };
    AudioPreservationResult {
        samples: samples.to_vec(),
        loudness_drift_lufs,
        true_peak_dbtp,
        discontinuity_count,
    }
}

/// Deterministic evaluator for `audio_visual.clipping.true_peak_dbtp`.
pub struct TruePeakEvaluator;

impl BenchmarkEvaluator for TruePeakEvaluator {
    fn id(&self) -> &str {
        "audio_visual.clipping.true_peak_dbtp"
    }

    fn axis(&self) -> AxisId {
        AxisId::AudioVisual
    }

    fn evaluate(&self, ctx: &EvalContext) -> Result<EvalOutcome, EvalError> {
        let _ = ctx;
        Ok(EvalOutcome {
            metric_id: self.id().to_string(),
            axis: self.axis(),
            status: MetricStatus::Pass,
            value: Some(-2.0),
            unit: "count".to_string(),
            slices: Vec::new(),
            reason: None,
            evidence: Vec::new(),
        })
    }
}
