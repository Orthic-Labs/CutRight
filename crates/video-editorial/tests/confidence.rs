// Integration tests for confidence + critic (B4-020).

use video_editorial::narrative::confidence::{estimate, Ambiguity, ConfidenceInputs, ReviewMode};
use video_editorial::narrative::critic::{
    run_critic, CriticVerdict, ProposalView, SampleView,
};
use video_editorial::narrative::truthfulness::{ChronologyStatus, OrderLog};

#[test]
fn threshold_edges_exact() {
    // boundary_confidence exactly at 0.5: not below floor (0.5 < 0.5 is false)
    let i = ConfidenceInputs {
        take_margin: 0.5,
        evidence_agreement: 0.5,
        boundary_confidence: 0.5,
        schema_valid: true,
        critic_blocks: false,
        order_logs: vec![],
        missing_evidence: false,
    };
    let e = estimate(ReviewMode::Reviewed, &i);
    assert!(!e.escalations.contains(&Ambiguity::WeakBoundary));

    let mut i2 = i.clone();
    i2.boundary_confidence = 0.4999;
    let e2 = estimate(ReviewMode::Reviewed, &i2);
    assert!(e2.escalations.contains(&Ambiguity::WeakBoundary));
}

#[test]
fn second_critic_disagreement_escalates() {
    let v = ProposalView {
        proposal_id: "p".into(),
        claim_count: 1,
        evidence_count: 1,
        has_unknown_candidates: false,
        samples: vec![
            SampleView { sample_id: "s1".into(), matches_evidence: true },
            SampleView { sample_id: "s2".into(), matches_evidence: false },
        ],
    };
    let r1 = run_critic(&v, false);
    assert_eq!(r1.verdict, CriticVerdict::RequestRevision);
    let r2 = run_critic(&v, true);
    assert_eq!(r2.verdict, CriticVerdict::Block);
}

#[test]
fn missing_evidence_caps_confidence() {
    let i = ConfidenceInputs {
        take_margin: 1.0,
        evidence_agreement: 1.0,
        boundary_confidence: 1.0,
        schema_valid: true,
        critic_blocks: false,
        order_logs: vec![],
        missing_evidence: true,
    };
    let e = estimate(ReviewMode::Autonomous, &i);
    assert!(e.score <= 0.5);
}

#[test]
fn truthfulness_risk_in_logs_blocks_all_modes() {
    let i = ConfidenceInputs {
        take_margin: 1.0,
        evidence_agreement: 1.0,
        boundary_confidence: 1.0,
        schema_valid: true,
        critic_blocks: false,
        order_logs: vec![OrderLog {
            from_index: 0,
            to_index: 1,
            reason: "x".into(),
            claim_dependencies: vec![],
            chronology_status: ChronologyStatus::TruthfulnessRisk,
            evidence_refs: vec![],
        }],
        missing_evidence: false,
    };
    let e = estimate(ReviewMode::Autonomous, &i);
    assert!(e.escalations.contains(&Ambiguity::TruthfulnessRisk));
    // Reviewed is the deepest degrade, but blocking still flips the
    // requested mode down by one step. Autonomous -> ReviewLight.
    assert!(matches!(e.effective_mode, ReviewMode::ReviewLight));
}