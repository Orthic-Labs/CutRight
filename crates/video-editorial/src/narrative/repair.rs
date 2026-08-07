// Bounded repair attempts (Book 4 lane C, B4-021).
//
// Implements a single bounded revision cycle. If the reflection
// report indicates `can_repair`, attempt exactly one repair. A
// second required repair escalates to a hard Block — never silently
// suppressed.

use serde::{Deserialize, Serialize};

use crate::narrative::confidence::{ConfidenceEstimate, ReviewMode};
use crate::narrative::critic::{CriticOutcome, CriticVerdict};
use crate::narrative::reflection::{ReflectionCause, ReflectionReport};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepairOutcome {
    NoRepairNeeded,
    Repaired,
    BlockedNoRepair,
    EscalatedSecondRepair,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepairAttempt {
    pub proposal_id: String,
    pub outcome: RepairOutcome,
    pub cause: ReflectionCause,
    pub applied: bool,
    pub rationale: String,
    pub review_mode: ReviewMode,
}

/// Attempt a single bounded repair. `revision_used` is the existing
/// state — once true, this call escalates rather than repairs.
pub fn attempt_repair(
    reflection: &ReflectionReport,
    revision_used: bool,
    confidence_after: &ConfidenceEstimate,
    critic_after: &CriticOutcome,
) -> RepairAttempt {
    if !reflection.can_repair {
        return RepairAttempt {
            proposal_id: reflection.proposal_id.clone(),
            outcome: RepairOutcome::BlockedNoRepair,
            cause: reflection.cause,
            applied: false,
            rationale: format!(
                "reflection marks cause {:?} as not repairable",
                reflection.cause
            ),
            review_mode: reflection.review_mode,
        };
    }

    if revision_used {
        return RepairAttempt {
            proposal_id: reflection.proposal_id.clone(),
            outcome: RepairOutcome::EscalatedSecondRepair,
            cause: reflection.cause,
            applied: false,
            rationale: "bounded revision already used; escalating".into(),
            review_mode: ReviewMode::Reviewed,
        };
    }

    // After applying repair, decide if it succeeded.
    let critic_passed = matches!(critic_after.verdict, CriticVerdict::Approve);
    let confident = confidence_after.score >= 0.5
        && !confidence_after
            .escalations
            .iter()
            .any(|e| matches!(e, crate::narrative::confidence::Ambiguity::TruthfulnessRisk));

    if critic_passed && confident {
        RepairAttempt {
            proposal_id: reflection.proposal_id.clone(),
            outcome: RepairOutcome::Repaired,
            cause: reflection.cause,
            applied: true,
            rationale: format!("repaired by addressing {:?}", reflection.cause),
            review_mode: confidence_after.effective_mode,
        }
    } else {
        // Repair did not satisfy invariants; treat as second repair needed -> escalate.
        RepairAttempt {
            proposal_id: reflection.proposal_id.clone(),
            outcome: RepairOutcome::EscalatedSecondRepair,
            cause: reflection.cause,
            applied: false,
            rationale: "repair did not satisfy critic/confidence; escalating".into(),
            review_mode: ReviewMode::Reviewed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narrative::confidence::{ConfidenceEstimate, ReviewMode};
    use crate::narrative::critic::{CriticOutcome, CriticVerdict};
    use crate::narrative::reflection::{ReflectionCause, ReflectionReport};

    fn report(can_repair: bool) -> ReflectionReport {
        ReflectionReport {
            proposal_id: "p".into(),
            cause: ReflectionCause::LowConfidence,
            severity: 0.5,
            recommendation: "x".into(),
            review_mode: ReviewMode::Reviewed,
            can_repair,
        }
    }

    fn conf(score: f32) -> ConfidenceEstimate {
        ConfidenceEstimate {
            score,
            escalations: vec![],
            requested_mode: ReviewMode::Reviewed,
            effective_mode: ReviewMode::Reviewed,
            rationale: vec![],
        }
    }

    fn approve() -> CriticOutcome {
        CriticOutcome {
            verdict: CriticVerdict::Approve,
            findings: vec![],
            revision_requested: false,
        }
    }

    #[test]
    fn no_repair_needed_when_not_repairable() {
        let r = attempt_repair(&report(false), false, &conf(1.0), &approve());
        assert_eq!(r.outcome, RepairOutcome::BlockedNoRepair);
    }

    #[test]
    fn repair_succeeds_when_critic_and_confidence_pass() {
        let r = attempt_repair(&report(true), false, &conf(0.8), &approve());
        assert_eq!(r.outcome, RepairOutcome::Repaired);
        assert!(r.applied);
    }

    #[test]
    fn second_repair_escalates() {
        let r = attempt_repair(&report(true), true, &conf(1.0), &approve());
        assert_eq!(r.outcome, RepairOutcome::EscalatedSecondRepair);
        assert!(!r.applied);
    }
}