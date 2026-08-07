// Hard-fault disqualification (Book 4 lane B, B4-014).
//
// Defines the catalogue of hard faults that override any weighted score.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HardFault {
    ClippedWord { word_id: String },
    SourceCorruption { detail: String },
    UnusableExposure { luma: f32 },
    UnusableAudio { snr_db: f32 },
    IdentityViolation { subject_id: String },
}

impl HardFault {
    pub fn label(&self) -> &'static str {
        match self {
            HardFault::ClippedWord { .. } => "clipped_word",
            HardFault::SourceCorruption { .. } => "source_corruption",
            HardFault::UnusableExposure { .. } => "unusable_exposure",
            HardFault::UnusableAudio { .. } => "unusable_audio",
            HardFault::IdentityViolation { .. } => "identity_violation",
        }
    }
}

/// Any hard fault disqualifies a take regardless of weighted score.
pub fn disqualifies(fault: &HardFault) -> bool {
    matches!(
        fault,
        HardFault::ClippedWord { .. }
            | HardFault::SourceCorruption { .. }
            | HardFault::UnusableExposure { .. }
            | HardFault::UnusableAudio { .. }
            | HardFault::IdentityViolation { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_catalogue_faults_disqualify() {
        for f in [
            HardFault::ClippedWord {
                word_id: "w1".into(),
            },
            HardFault::SourceCorruption { detail: "x".into() },
            HardFault::UnusableExposure { luma: 0.5 },
            HardFault::UnusableAudio { snr_db: -10.0 },
            HardFault::IdentityViolation {
                subject_id: "s1".into(),
            },
        ] {
            assert!(disqualifies(&f));
        }
    }

    #[test]
    fn labels_are_distinct() {
        let f = HardFault::ClippedWord {
            word_id: "x".into(),
        };
        assert_eq!(f.label(), "clipped_word");
    }
}
