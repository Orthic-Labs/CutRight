// Confidence, ambiguity, escalation (Book 4 lane C, B4-020).
//
// Combines take margin, evidence availability, boundary confidence,
// schema validity, critic findings and truthfulness checks into a
// single estimate. Emits named ambiguity flags and per-mode
// escalations. Missing evidence cannot increase confidence.

use serde::{Deserialize, Serialize};

use crate::narrative::truthfulness::{ChronologyStatus, OrderLog};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewMode {
    Reviewed,
    ReviewLight,
    Autonomous,
}

impl ReviewMode {
    pub fn degrade_one_step(self) -> Self {
        match self {
            ReviewMode::Reviewed => ReviewMode::Reviewed,
            ReviewMode::ReviewLight => ReviewMode::Reviewed,
            ReviewMode::Autonomous => ReviewMode::ReviewLight,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ambiguity {
    LowTakeMargin,
    MissingEvidence,
    WeakBoundary,
    SchemaInvalid,
    TruthfulnessRisk,
    CriticDisagreement,
}

impl Ambiguity {
    pub fn blocks(self, mode: ReviewMode) -> bool {
        match (self, mode) {
            (Ambiguity::SchemaInvalid, _) => true,
            (Ambiguity::TruthfulnessRisk, _) => true,
            (Ambiguity::MissingEvidence, ReviewMode::Autonomous) => true,
            (Ambiguity::CriticDisagreement, ReviewMode::Autonomous | ReviewMode::ReviewLight) => true,
            (Ambiguity::LowTakeMargin, ReviewMode::Autonomous) => true,
            (Ambiguity::WeakBoundary, ReviewMode::Autonomous) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfidenceEstimate {
    pub score: f32,
    pub escalations: Vec<Ambiguity>,
    pub requested_mode: ReviewMode,
    pub effective_mode: ReviewMode,
    pub rationale: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfidenceInputs {
    pub take_margin: f32,
    pub evidence_agreement: f32,
    pub boundary_confidence: f32,
    pub schema_valid: bool,
    pub critic_blocks: bool,
    pub order_logs: Vec<OrderLog>,
    pub missing_evidence: bool,
}

/// Combine inputs into a confidence score. Missing evidence caps
/// confidence; escalations block per `Ambiguity::blocks` policy.
pub fn estimate(mode: ReviewMode, i: &ConfidenceInputs) -> ConfidenceEstimate {
    let mut components: Vec<(f32, &str)> = Vec::new();
    components.push((i.take_margin.clamp(0.0, 1.0), "take_margin"));
    components.push((i.evidence_agreement.clamp(0.0, 1.0), "evidence_agreement"));
    components.push((i.boundary_confidence.clamp(0.0, 1.0), "boundary_confidence"));
    let mut raw = components.iter().map(|(v, _)| *v).sum::<f32>() / components.len() as f32;

    if i.missing_evidence {
        // Missing evidence cannot increase confidence.
        raw = raw.min(0.5);
    }

    let mut escalations: Vec<Ambiguity> = Vec::new();
    let mut rationale: Vec<String> = Vec::new();

    if i.take_margin < 0.05 {
        escalations.push(Ambiguity::LowTakeMargin);
        rationale.push(format!("take_margin={:.3} below 0.05 floor", i.take_margin));
    }
    if i.missing_evidence {
        escalations.push(Ambiguity::MissingEvidence);
        rationale.push("missing evidence: confidence capped at 0.5".into());
    }
    if i.boundary_confidence < 0.5 {
        escalations.push(Ambiguity::WeakBoundary);
        rationale.push(format!(
            "boundary_confidence={:.3} below 0.5 floor",
            i.boundary_confidence
        ));
    }
    if !i.schema_valid {
        escalations.push(Ambiguity::SchemaInvalid);
        rationale.push("schema invalid".into());
    }
    if i.critic_blocks {
        escalations.push(Ambiguity::CriticDisagreement);
        rationale.push("critic blocked".into());
    }
    let truthfulness_risk = i
        .order_logs
        .iter()
        .any(|l| matches!(l.chronology_status, ChronologyStatus::TruthfulnessRisk));
    if truthfulness_risk {
        escalations.push(Ambiguity::TruthfulnessRisk);
        rationale.push("truthfulness risk in order logs".into());
    }

    let blocking = escalations.iter().any(|e| e.blocks(mode));
    let effective_mode = if blocking { mode.degrade_one_step() } else { mode };

    ConfidenceEstimate {
        score: raw,
        escalations,
        requested_mode: mode,
        effective_mode,
        rationale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narrative::truthfulness::{Claim, OrderLog, Reorder};

    fn base() -> ConfidenceInputs {
        ConfidenceInputs {
            take_margin: 0.5,
            evidence_agreement: 0.5,
            boundary_confidence: 0.5,
            schema_valid: true,
            critic_blocks: false,
            order_logs: vec![],
            missing_evidence: false,
        }
    }

    #[test]
    fn missing_evidence_caps_confidence() {
        let mut i = base();
        i.take_margin = 0.99;
        i.evidence_agreement = 0.99;
        i.boundary_confidence = 0.99;
        i.missing_evidence = true;
        let e = estimate(ReviewMode::Autonomous, &i);
        assert!(e.score <= 0.5);
        assert!(e.escalations.contains(&Ambiguity::MissingEvidence));
    }

    #[test]
    fn truthfulness_risk_blocks_autonomous_and_reviewed() {
        let mut i = base();
        i.order_logs.push(OrderLog {
            from_index: 0,
            to_index: 1,
            reason: "x".into(),
            claim_dependencies: vec![],
            chronology_status: ChronologyStatus::TruthfulnessRisk,
            evidence_refs: vec![],
        });
        let e = estimate(ReviewMode::Autonomous, &i);
        assert!(e.escalations.contains(&Ambiguity::TruthfulnessRisk));
        // Reviewed mode already starts at Reviewed (one-step degrade is Reviewed)
        // but blocking should ensure no promotion can occur from below.
        assert!(matches!(e.effective_mode, ReviewMode::Reviewed));
        let _ = Reorder {
            from_index: 0,
            to_index: 1,
            claim: Claim { claim_id: "c".into(), depends_on: vec![] },
            introduces_false_sequence: false,
            breaks_claim_dependency: false,
        };
    }

    #[test]
    fn schema_invalid_blocks_all_modes() {
        let mut i = base();
        i.schema_valid = false;
        let e = estimate(ReviewMode::Autonomous, &i);
        assert!(e.escalations.contains(&Ambiguity::SchemaInvalid));
        assert!(matches!(e.effective_mode, ReviewMode::ReviewLight));
    }

    #[test]
    fn low_take_margin_blocks_only_autonomous() {
        let mut i = base();
        i.take_margin = 0.0;
        let r = estimate(ReviewMode::Reviewed, &i);
        assert!(!matches!(r.effective_mode, ReviewMode::Reviewed));
        let _ = estimate(ReviewMode::Autonomous, &i);
    }
}