// Self-reflection (Book 4 lane C, B4-021).
//
// When confidence is low or a critic raised warnings, a self-reflection
// report identifies the cause and proposes a structured repair. The
// report is consumed by `repair.rs` to attempt bounded revision.

use serde::{Deserialize, Serialize};

use crate::narrative::confidence::{Ambiguity, ConfidenceEstimate, ReviewMode};
use crate::narrative::critic::{CriticFinding, CriticOutcome, CriticVerdict};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReflectionCause {
    LowConfidence,
    CriticWarnings,
    CriticBlock,
    TruthfulnessRisk,
    MissingEvidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReflectionReport {
    pub proposal_id: String,
    pub cause: ReflectionCause,
    pub severity: f32,
    pub recommendation: String,
    pub review_mode: ReviewMode,
    pub can_repair: bool,
}

/// Build a reflection report. Returns `None` when no repair is
/// needed (everything passes).
pub fn reflect(
    proposal_id: &str,
    confidence: &ConfidenceEstimate,
    critic: &CriticOutcome,
) -> Option<ReflectionReport> {
    let critic_blocks = matches!(critic.verdict, CriticVerdict::Block);
    let critic_warns = matches!(critic.verdict, CriticVerdict::RequestRevision);
    let truthfulness = confidence
        .escalations
        .contains(&Ambiguity::TruthfulnessRisk);
    let missing = confidence.escalations.contains(&Ambiguity::MissingEvidence);

    let (cause, severity, can_repair, recommendation) = if critic_blocks || truthfulness {
        (
            if truthfulness {
                ReflectionCause::TruthfulnessRisk
            } else {
                ReflectionCause::CriticBlock
            },
            1.0,
            false,
            "do not repair: downgrade mode and re-scope".to_string(),
        )
    } else if critic_warns {
        (
            ReflectionCause::CriticWarnings,
            0.7,
            true,
            "bounded revision: address critic warnings".to_string(),
        )
    } else if missing {
        (
            ReflectionCause::MissingEvidence,
            0.8,
            true,
            "retrieve additional evidence before retry".to_string(),
        )
    } else if confidence.score < 0.5 {
        (
            ReflectionCause::LowConfidence,
            1.0 - confidence.score,
            true,
            "tighten selectors and re-score".to_string(),
        )
    } else {
        return None;
    };

    let _ = critic_block_finding_count(critic);
    Some(ReflectionReport {
        proposal_id: proposal_id.to_string(),
        cause,
        severity,
        recommendation,
        review_mode: confidence.effective_mode,
        can_repair,
    })
}

fn critic_block_finding_count(c: &CriticOutcome) -> usize {
    c.findings
        .iter()
        .filter(|f| matches!(f.severity, crate::narrative::critic::FindingSeverity::Block))
        .count()
}

#[allow(dead_code)]
fn critic_findings(c: &CriticOutcome) -> &[CriticFinding] {
    &c.findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narrative::confidence::{ConfidenceEstimate, ConfidenceInputs, ReviewMode};
    use crate::narrative::critic::{
        run_critic, CriticVerdict, FindingSeverity, ProposalView, SampleView,
    };

    fn clean_confidence() -> ConfidenceEstimate {
        ConfidenceEstimate {
            score: 0.9,
            escalations: vec![],
            requested_mode: ReviewMode::Reviewed,
            effective_mode: ReviewMode::Reviewed,
            rationale: vec![],
        }
    }

    fn warn_critic() -> CriticOutcome {
        let v = ProposalView {
            proposal_id: "p".into(),
            claim_count: 1,
            evidence_count: 1,
            has_unknown_candidates: false,
            samples: vec![SampleView {
                sample_id: "s2".into(),
                matches_evidence: false,
            }],
        };
        run_critic(&v, false)
    }

    #[test]
    fn no_report_when_clean() {
        let r = reflect(
            "p",
            &clean_confidence(),
            &CriticOutcome {
                verdict: CriticVerdict::Approve,
                findings: vec![],
                revision_requested: false,
            },
        );
        assert!(r.is_none());
    }

    #[test]
    fn report_on_warnings_with_repair() {
        let r = reflect("p", &clean_confidence(), &warn_critic()).unwrap();
        assert!(matches!(r.cause, ReflectionCause::CriticWarnings));
        assert!(r.can_repair);
    }

    #[test]
    fn report_on_low_confidence() {
        let c = ConfidenceEstimate {
            score: 0.2,
            escalations: vec![],
            requested_mode: ReviewMode::Reviewed,
            effective_mode: ReviewMode::Reviewed,
            rationale: vec![],
        };
        let r = reflect(
            "p",
            &c,
            &CriticOutcome {
                verdict: CriticVerdict::Approve,
                findings: vec![],
                revision_requested: false,
            },
        )
        .unwrap();
        assert!(matches!(r.cause, ReflectionCause::LowConfidence));
        assert!(r.can_repair);
    }

    #[test]
    fn block_prevents_repair() {
        let v = ProposalView {
            proposal_id: "p".into(),
            claim_count: 1,
            evidence_count: 1,
            has_unknown_candidates: true,
            samples: vec![],
        };
        let c = run_critic(&v, false);
        let conf = clean_confidence();
        let r = reflect("p", &conf, &c).unwrap();
        assert!(!r.can_repair);
    }

    #[allow(dead_code)]
    fn _unused_severity(_: FindingSeverity) {}
    #[allow(dead_code)]
    fn _unused_inputs(_: &ConfidenceInputs) {}
}
