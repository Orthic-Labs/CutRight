//! Evidence-backed preference learning.
//!
//! A preference estimate is computed only from decisions that are
//! compatible, hash-valid, and not stale. The estimate is split by scope:
//! `user_specific` and `shared_benchmark_floor`. Insufficient evidence
//! returns a `supported: false` estimate rather than inventing a value.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::decision::{DecisionRecord, ReviewMode};
use crate::distributions::AxisDistribution;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsufficientReason {
    UnsupportedAxis,
    InsufficientSamples,
    ConflictingDecisions,
    IncompatiblePacks,
    StaleSubjectsOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EstimateScope {
    UserSpecific,
    SharedBenchmarkFloor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreferenceEstimate {
    pub schema_version: String,
    pub preference_id: String,
    pub axis: String,
    pub distribution: AxisDistribution,
    pub confidence: f64,
    pub sample_count: u32,
    pub variance: f64,
    pub evidence_decision_ids: Vec<String>,
    pub compatibility_fingerprint: String,
    pub scope: EstimateScope,
    pub supported: bool,
    pub insufficient_reason: Option<InsufficientReason>,
    pub recency_weighted: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopedEstimate {
    pub user_specific: PreferenceEstimate,
    pub shared_floor: PreferenceEstimate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    pub axis: String,
    pub top_reason: Option<String>,
    pub weights: AxisDistribution,
    pub ready_to_apply: bool,
}

/// Returns true when the estimate is supported for the chosen scope.
pub fn estimate_is_supported(estimate: &PreferenceEstimate) -> bool {
    estimate.supported
}

/// Filter `decisions` to those that are hash-valid, not stale, and
/// compatible with the given pack fingerprint.
fn filter_compatible(decisions: &[DecisionRecord], pack_fingerprint: &str) -> Vec<DecisionRecord> {
    decisions
        .iter()
        .filter(|d| !d.stale_subject && !d.malformed && d.pack_set_fingerprint == pack_fingerprint)
        .cloned()
        .collect()
}

/// Compute a compatibility fingerprint. The fingerprint is the BLAKE3 of
/// the canonicalised pack id and benchmark profile list.
fn compute_compatibility_fingerprint(pack_set_id: &str, benchmark_profile: &str) -> String {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(pack_set_id.as_bytes());
    hasher.update(&[0u8]);
    hasher.update(benchmark_profile.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest.as_bytes() {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

/// Compute a `PreferenceEstimate` for the given axis from a slice of
/// decisions. Returns `supported: false` when the evidence is insufficient.
pub fn compute_preference(
    axis: &str,
    decisions: &[DecisionRecord],
    pack_set_id: &str,
    benchmark_profile: &str,
    min_samples: u32,
    scope: EstimateScope,
    recency_weighted: bool,
) -> PreferenceEstimate {
    let pack_fingerprint_input = format!("{}|{}", pack_set_id, benchmark_profile);
    let compatibility_fingerprint =
        compute_compatibility_fingerprint(pack_set_id, benchmark_profile);

    let compatible = filter_compatible(decisions, &pack_fingerprint_input);
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    let mut evidence: Vec<String> = Vec::new();
    for d in &compatible {
        if decision_axis(d) != axis {
            continue;
        }
        let key = decision_reason_key(d);
        *counts.entry(key).or_insert(0) += 1;
        evidence.push(d.decision_id.clone());
    }
    let sample_count: u32 = counts.values().sum();
    let total = sample_count as f64;
    let distribution: AxisDistribution = counts
        .iter()
        .map(|(k, v)| (k.clone(), (*v as f64) / total))
        .collect();
    let variance = compute_variance(&distribution);
    let supported = sample_count >= min_samples && variance < 0.5;

    let insufficient_reason = if !supported {
        Some(pick_insufficient_reason(
            sample_count,
            min_samples,
            variance,
            &compatible,
        ))
    } else {
        None
    };

    // preference_id is the BLAKE3 of the canonical payload.
    let preference_id = {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(axis.as_bytes());
        hasher.update(&[0u8]);
        hasher.update(scope.as_str().as_bytes());
        for (k, v) in &distribution {
            hasher.update(k.as_bytes());
            hasher.update(&v.to_be_bytes());
        }
        let digest = hasher.finalize();
        let mut out = String::with_capacity(64);
        for byte in digest.as_bytes() {
            out.push_str(&format!("{:02x}", byte));
        }
        out
    };

    PreferenceEstimate {
        schema_version: "v2".to_string(),
        preference_id,
        axis: axis.to_string(),
        distribution,
        confidence: if supported { 1.0 - variance } else { 0.0 },
        sample_count,
        variance,
        evidence_decision_ids: evidence,
        compatibility_fingerprint,
        scope,
        supported,
        insufficient_reason,
        recency_weighted,
        created_at: Utc::now(),
    }
}

/// Compute a recommendation from a preference estimate. The recommendation
/// is **separate** from any applied profile.
pub fn compute_recommendation(estimate: &PreferenceEstimate) -> Recommendation {
    let top_reason = estimate
        .distribution
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(k, _)| k.clone());
    Recommendation {
        axis: estimate.axis.clone(),
        top_reason,
        weights: estimate.distribution.clone(),
        ready_to_apply: estimate.supported,
    }
}

fn decision_axis(d: &DecisionRecord) -> &str {
    match d.decision_axis {
        crate::decision::DecisionAxis::Take => "take",
        crate::decision::DecisionAxis::Boundary => "boundary",
        crate::decision::DecisionAxis::Filler => "filler",
        crate::decision::DecisionAxis::Pause => "pause",
        crate::decision::DecisionAxis::Hook => "hook",
        crate::decision::DecisionAxis::Cta => "cta",
        crate::decision::DecisionAxis::BeatOrder => "beat_order",
        crate::decision::DecisionAxis::Crop => "crop",
        crate::decision::DecisionAxis::Caption => "caption",
        crate::decision::DecisionAxis::Graphic => "graphic",
        crate::decision::DecisionAxis::Motion => "motion",
        crate::decision::DecisionAxis::BRoll => "broll",
        crate::decision::DecisionAxis::Sfx => "sfx",
        crate::decision::DecisionAxis::Music => "music",
        crate::decision::DecisionAxis::Color => "color",
        crate::decision::DecisionAxis::Audio => "audio",
        crate::decision::DecisionAxis::Identity => "identity",
        crate::decision::DecisionAxis::Final => "final",
        crate::decision::DecisionAxis::UnsupportedAxis => "unsupported_axis",
    }
}

fn decision_reason_key(d: &DecisionRecord) -> String {
    use crate::decision::DecisionReason;
    match d.decision_reason {
        DecisionReason::TakeChoice => "take_choice",
        DecisionReason::BoundaryChoice => "boundary_choice",
        DecisionReason::FillerChoice => "filler_choice",
        DecisionReason::PauseChoice => "pause_choice",
        DecisionReason::HookChoice => "hook_choice",
        DecisionReason::CtaChoice => "cta_choice",
        DecisionReason::BeatOrder => "beat_order",
        DecisionReason::CropChoice => "crop_choice",
        DecisionReason::CaptionChoice => "caption_choice",
        DecisionReason::GraphicChoice => "graphic_choice",
        DecisionReason::EffectDensity => "effect_density",
        DecisionReason::BRollChoice => "broll_choice",
        DecisionReason::SfxChoice => "sfx_choice",
        DecisionReason::MusicChoice => "music_choice",
        DecisionReason::ColorChoice => "color_choice",
        DecisionReason::AudioChoice => "audio_choice",
        DecisionReason::IdentityChoice => "identity_choice",
        DecisionReason::FinalVerdict => "final_verdict",
        DecisionReason::UnknownReason => "unknown_reason",
    }
    .to_string()
}

fn compute_variance(distribution: &AxisDistribution) -> f64 {
    if distribution.is_empty() {
        return 0.0;
    }
    let n = distribution.len() as f64;
    let mean = 1.0 / n;
    let v: f64 = distribution
        .values()
        .map(|w| {
            let d = w - mean;
            d * d
        })
        .sum::<f64>()
        / n;
    v
}

fn pick_insufficient_reason(
    sample_count: u32,
    min_samples: u32,
    variance: f64,
    compatible: &[DecisionRecord],
) -> InsufficientReason {
    if sample_count == 0 {
        return InsufficientReason::InsufficientSamples;
    }
    if sample_count < min_samples {
        return InsufficientReason::InsufficientSamples;
    }
    if variance >= 0.5 {
        return InsufficientReason::ConflictingDecisions;
    }
    if compatible.is_empty() {
        return InsufficientReason::StaleSubjectsOnly;
    }
    InsufficientReason::IncompatiblePacks
}

impl EstimateScope {
    fn as_str(&self) -> &'static str {
        match self {
            EstimateScope::UserSpecific => "user_specific",
            EstimateScope::SharedBenchmarkFloor => "shared_benchmark_floor",
        }
    }
}

// Ensure review-mode shows up in the public API for downstream callers.
pub use crate::decision::ReviewMode as PublicReviewMode;

#[allow(dead_code)]
fn _review_mode_marker(_: ReviewMode) {}
