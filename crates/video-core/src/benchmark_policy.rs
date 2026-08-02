//! Versioned transcription benchmark policy (REV2 plan §8.1/§8.2).
//!
//! Acceptance thresholds for the HeardRight/WhisperX transcription benchmark
//! live in a single versioned JSON file (`schemas/benchmark-policy.v1.json`),
//! not as scattered constants across the pipeline. Bumping the policy means
//! adding a new versioned file and a new embedded loader, never silently
//! editing the numbers behind an existing `policy_version`.

use serde::{Deserialize, Serialize};

/// The raw contents of the v1 benchmark policy file, embedded at compile
/// time so the binary is self-contained and the policy that shipped is the
/// policy that ran.
const BENCHMARK_POLICY_V1_JSON: &str = include_str!("../../../schemas/benchmark-policy.v1.json");

/// Acceptance thresholds for the transcription benchmark decision (REV2 plan
/// §8.1/§8.2). Every threshold that used to be a scattered magic number now
/// lives here, versioned.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkPolicy {
    pub policy_version: u32,
    #[serde(default)]
    pub description: String,
    /// Minimum fraction of HeardRight words that must align to a verifier
    /// word for the verifier's coverage to be considered sufficient.
    pub min_alignment_coverage: f64,
    /// Maximum fraction of HeardRight words classified as genuine
    /// (non-trivial) content disagreement before the benchmark treats the
    /// primary transcript as unclean.
    pub max_unmatched_content_rate: f64,
    /// Default requested boundary padding in milliseconds, kept as an
    /// explicit, overridable benchmark parameter rather than a hardcoded
    /// constant.
    pub requested_padding_ms_default: i64,
}

impl BenchmarkPolicy {
    /// Load the embedded policy version 1. This is the only supported
    /// policy version today; future versions get their own embedded file
    /// and loader rather than mutating this one in place.
    pub fn v1() -> Self {
        serde_json::from_str(BENCHMARK_POLICY_V1_JSON)
            .expect("embedded benchmark-policy.v1.json must be valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_v1_policy_loads_and_is_versioned() {
        let policy = BenchmarkPolicy::v1();
        assert_eq!(policy.policy_version, 1);
        assert!(policy.min_alignment_coverage > 0.0 && policy.min_alignment_coverage <= 1.0);
        assert!(
            policy.max_unmatched_content_rate >= 0.0 && policy.max_unmatched_content_rate <= 1.0
        );
        assert!(policy.requested_padding_ms_default >= 0);
    }
}
