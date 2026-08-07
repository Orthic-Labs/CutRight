// Integration tests for hook ranking, truthfulness-aware ordering,
// and order plan aggregation (Book 4 lane C, B4-018).

use video_editorial::narrative::hook::{rank, score_hook, HookCandidate};
use video_editorial::narrative::order::build_plan;
use video_editorial::narrative::truthfulness::{
    evaluate_reorder, Claim, ChronologyStatus, Reorder,
};

#[test]
fn unrecorded_hook_zero_score() {
    let h = HookCandidate {
        take_id: "h1".into(),
        text: "x".into(),
        specificity: 1.0,
        promise: 1.0,
        self_containment: 1.0,
        evidence_strength: 1.0,
        recorded: false,
    };
    assert_eq!(score_hook(&h), 0.0);
}

#[test]
fn rank_picks_best_recorded() {
    let hooks = vec![
        HookCandidate {
            take_id: "a".into(),
            text: "x".into(),
            specificity: 0.5, promise: 0.5, self_containment: 0.5, evidence_strength: 0.5,
            recorded: true,
        },
        HookCandidate {
            take_id: "b".into(),
            text: "y".into(),
            specificity: 0.9, promise: 0.9, self_containment: 0.9, evidence_strength: 0.9,
            recorded: true,
        },
    ];
    let r = rank(&hooks);
    assert_eq!(r[0].take_id, "b");
}

#[test]
fn cold_open_log_is_chronology_status_cold_open() {
    let r = Reorder {
        from_index: 2,
        to_index: 0,
        claim: Claim { claim_id: "c".into(), depends_on: vec![] },
        introduces_false_sequence: false,
        breaks_claim_dependency: false,
    };
    let log = evaluate_reorder(&r);
    assert_eq!(log.chronology_status, ChronologyStatus::ColdOpen);
}

#[test]
fn plan_truthfulness_flag_propagates() {
    let r = Reorder {
        from_index: 0,
        to_index: 1,
        claim: Claim { claim_id: "c".into(), depends_on: vec![] },
        introduces_false_sequence: true,
        breaks_claim_dependency: false,
    };
    let log = evaluate_reorder(&r);
    let plan = build_plan("p1", vec!["a".into(), "b".into()], vec![log]);
    assert!(plan.has_truthfulness_risk);
}