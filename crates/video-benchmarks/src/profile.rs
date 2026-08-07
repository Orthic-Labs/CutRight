// Benchmark profile binding (Book 4 lane A, B4-024).
//
// Encodes initial floors from the benchmark plan, requires
// integrity/safety floors for every mode, and human-acceptance
// history for review-light/autonomous. Autonomous execution is
// blocked when format, model/skill/renderer pack set, or profile
// version lacks compatible evidence. Downgrades are allowed;
// automatic upgrades are not.

use serde::{Deserialize, Serialize};

use crate::{AxisId, MetricStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProfileMode {
    Reviewed,
    ReviewLight,
    Autonomous,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileFloor {
    pub metric_id: String,
    pub axis: AxisId,
    pub min_status: MetricStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkProfile {
    pub profile_id: String,
    pub version: u32,
    pub mode: ProfileMode,
    pub floors: Vec<ProfileFloor>,
    pub required_human_acceptance: u32,
    pub format: String,
    pub pack_set: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCompat {
    pub format: String,
    pub pack_set: String,
    pub profile_version: u32,
}

/// Decide whether evidence is compatible with the active profile.
/// Used by the autonomy guard before execution.
pub fn evidence_compatible(profile: &BenchmarkProfile, ev: &EvidenceCompat) -> bool {
    profile.format == ev.format
        && profile.pack_set == ev.pack_set
        && profile.version == ev.profile_version
}

/// Determine the effective mode given evidence compatibility and
/// the human acceptance history. Downgrades are allowed; upgrades
/// are not automatic — the caller must hold an explicit
/// user-approved advancement record.
pub fn effective_mode(
    profile: &BenchmarkProfile,
    ev: &EvidenceCompat,
    human_acceptance: u32,
) -> ProfileMode {
    if !evidence_compatible(profile, ev) {
        return ProfileMode::Reviewed;
    }
    match profile.mode {
        ProfileMode::Reviewed => ProfileMode::Reviewed,
        ProfileMode::ReviewLight => {
            if human_acceptance >= profile.required_human_acceptance {
                ProfileMode::ReviewLight
            } else {
                ProfileMode::Reviewed
            }
        }
        ProfileMode::Autonomous => {
            if human_acceptance >= profile.required_human_acceptance {
                ProfileMode::Autonomous
            } else {
                ProfileMode::ReviewLight
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn new_format_resolves_reviewed() {
        let p = profile(ProfileMode::Autonomous);
        let ev = EvidenceCompat {
            format: "new-format".into(),
            pack_set: "v2".into(),
            profile_version: 1,
        };
        assert_eq!(effective_mode(&p, &ev, 100), ProfileMode::Reviewed);
    }

    #[test]
    fn insufficient_history_downgrades_autonomous() {
        let p = profile(ProfileMode::Autonomous);
        let ev = EvidenceCompat {
            format: "shorts".into(),
            pack_set: "v2".into(),
            profile_version: 1,
        };
        assert_eq!(effective_mode(&p, &ev, 0), ProfileMode::ReviewLight);
    }

    #[test]
    fn no_automatic_upgrade() {
        // Reviewed profile can never return ReviewLight without an
        // explicit advancement record (caller responsibility).
        let p = profile(ProfileMode::Reviewed);
        let ev = EvidenceCompat {
            format: "shorts".into(),
            pack_set: "v2".into(),
            profile_version: 1,
        };
        assert_eq!(effective_mode(&p, &ev, 10_000), ProfileMode::Reviewed);
    }

    #[test]
    fn pack_change_invalidates() {
        let p = profile(ProfileMode::Autonomous);
        let ev = EvidenceCompat {
            format: "shorts".into(),
            pack_set: "v3".into(),
            profile_version: 1,
        };
        assert_eq!(effective_mode(&p, &ev, 100), ProfileMode::Reviewed);
    }
}
