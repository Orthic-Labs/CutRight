// Integration tests for narrative arcs and Director contract
// (Book 4 lane C, B4-017).

use video_editorial::narrative::arcs::{library, validate_arc, ArcKind};
use video_editorial::narrative::provider::{
    validate_proposal, CandidateRef, EditorialProposal, EditorialRequest,
};

#[test]
fn library_has_minimum_required_kinds() {
    let l = library();
    assert!(l.iter().any(|a| matches!(a.kind, ArcKind::LongForm)));
    assert!(l.iter().any(|a| matches!(a.kind, ArcKind::Shorts)));
    assert!(l.iter().any(|a| matches!(a.kind, ArcKind::Explainer)));
}

#[test]
fn long_form_validates_with_correct_roles() {
    let l = library();
    let t = l.iter().find(|a| matches!(a.kind, ArcKind::LongForm)).unwrap();
    let counts = vec![("hook", 1u32), ("setup", 1), ("payoff", 1)];
    assert!(validate_arc(t, &counts));
}

#[test]
fn director_proposal_with_unknown_candidate_fails() {
    let req = EditorialRequest {
        user_brief: "x".into(),
        format: "shorts".into(),
        duration_ms_target: 30_000,
        candidates: vec![CandidateRef { beat_id: "b1".into(), take_id: "t1".into(), role: "hook".into() }],
        evidence_refs: vec![],
        model_revision: "v1".into(),
    };
    let prop = EditorialProposal {
        proposal_id: "p1".into(),
        selected: vec![CandidateRef { beat_id: "bx".into(), take_id: "tx".into(), role: "x".into() }],
        order: vec!["tx".into()],
        arc_id: "shorts.hook-payoff".into(),
        rationale: vec!["x".into()],
        evidence_refs: vec![],
    };
    assert!(validate_proposal(&req, &prop).is_err());
}

#[test]
fn director_valid_proposal_passes() {
    let req = EditorialRequest {
        user_brief: "x".into(),
        format: "shorts".into(),
        duration_ms_target: 30_000,
        candidates: vec![
            CandidateRef { beat_id: "b1".into(), take_id: "t1".into(), role: "hook".into() },
            CandidateRef { beat_id: "b2".into(), take_id: "t2".into(), role: "payoff".into() },
        ],
        evidence_refs: vec![],
        model_revision: "v1".into(),
    };
    let prop = EditorialProposal {
        proposal_id: "p1".into(),
        selected: req.candidates.clone(),
        order: vec!["t1".into(), "t2".into()],
        arc_id: "shorts.hook-payoff".into(),
        rationale: vec!["hook first".into()],
        evidence_refs: vec![],
    };
    assert!(validate_proposal(&req, &prop).is_ok());
}