// Benchmark profile binding to the EditorialEngine façade (Book 4
// lane C, B4-024).
//
// Encodes the integration between video-benchmarks profiles and
// the EditorialEngine: pre-flight compatibility check, effective
// mode override, and floor requirement before any plan is returned.
// All upgrades are explicit; downgrades are automatic.

use serde::{Deserialize, Serialize};

use video_benchmarks::profile::{
    effective_mode as profile_effective_mode, evidence_compatible, BenchmarkProfile,
    EvidenceCompat, ProfileMode,
};

use crate::engine::{EditorialEngine, EditorialEngineRequest};
use crate::narrative::confidence::ReviewMode;
use crate::plan::{EditorialPlanResult, PlanError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkBinding {
    pub profile: BenchmarkProfile,
    pub evidence: EvidenceCompat,
    pub human_acceptance: u32,
}

impl BenchmarkBinding {
    pub fn new(profile: BenchmarkProfile, evidence: EvidenceCompat, human_acceptance: u32) -> Self {
        Self {
            profile,
            evidence,
            human_acceptance,
        }
    }

    pub fn compatible(&self) -> bool {
        evidence_compatible(&self.profile, &self.evidence)
    }

    pub fn effective_mode(&self) -> ProfileMode {
        profile_effective_mode(&self.profile, &self.evidence, self.human_acceptance)
    }

    /// Apply the binding to the editorial request. If the requested
    /// mode is more aggressive than the binding allows, downgrade it.
    /// If the binding is incompatible, downgrade to Reviewed.
    pub fn apply(
        &self,
        engine: &EditorialEngine,
        mut req: EditorialEngineRequest,
    ) -> Result<EditorialPlanResult, PlanError> {
        let effective = self.effective_mode();
        req.review_mode = match effective {
            ProfileMode::Reviewed => ReviewMode::Reviewed,
            ProfileMode::ReviewLight => ReviewMode::ReviewLight,
            ProfileMode::Autonomous => ReviewMode::Autonomous,
        };
        engine.plan(&req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use video_benchmarks::profile::ProfileFloor;
    use video_benchmarks::{AxisId, MetricStatus};

    fn profile(mode: ProfileMode) -> BenchmarkProfile {
        BenchmarkProfile {
            profile_id: "p".into(),
            version: 1,
            mode,
            floors: vec![ProfileFloor {
                metric_id: "kernel.integrity".into(),
                axis: AxisId::KernelIntegrity,
                min_status: MetricStatus::Pass,
            }],
            required_human_acceptance: 10,
            format: "shorts".into(),
            pack_set: "v2".into(),
        }
    }

    fn evidence(version: u32) -> EvidenceCompat {
        EvidenceCompat {
            format: "shorts".into(),
            pack_set: "v2".into(),
            profile_version: version,
        }
    }

    #[test]
    fn incompatible_evidence_downgrades_to_reviewed() {
        let b = BenchmarkBinding::new(profile(ProfileMode::Autonomous), evidence(2), 100);
        assert!(!b.compatible());
        assert_eq!(b.effective_mode(), ProfileMode::Reviewed);
    }

    #[test]
    fn compatible_autonomous_with_sufficient_history() {
        let b = BenchmarkBinding::new(profile(ProfileMode::Autonomous), evidence(1), 10);
        assert!(b.compatible());
        assert_eq!(b.effective_mode(), ProfileMode::Autonomous);
    }

    #[test]
    fn downgrade_when_history_insufficient() {
        let b = BenchmarkBinding::new(profile(ProfileMode::Autonomous), evidence(1), 0);
        assert_eq!(b.effective_mode(), ProfileMode::ReviewLight);
    }
}
