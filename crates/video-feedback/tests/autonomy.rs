use chrono::{TimeZone, Utc};
use video_feedback::autonomy::{
    advance, demote, has_regression_trigger, initial_state, transitions, AutonomyDemotionPredicate,
    AutonomyMode, AutonomyTransitionReason,
};
use video_feedback::decision::FormatKey;

fn fmt() -> FormatKey {
    FormatKey {
        content_type: "recorded_talking_head".to_string(),
        platform: "tiktok".to_string(),
        variant: "v1".to_string(),
    }
}

#[test]
fn new_format_starts_reviewed() {
    let s = initial_state(
        fmt(),
        vec!["creator-minimal".to_string()],
        "reviewed-v2".to_string(),
    );
    assert_eq!(s.mode, AutonomyMode::Reviewed);
    assert!(!s.demoted);
    assert!(s.last_user_approval.is_none());
    assert_eq!(s.transition_history.len(), 1);
    assert!(matches!(
        s.transition_history[0].reason,
        AutonomyTransitionReason::FirstSeen
    ));
}

#[test]
fn no_self_approval_of_advancement() {
    let s = initial_state(
        fmt(),
        vec!["creator-minimal".to_string()],
        "reviewed-v2".to_string(),
    );
    let mut advanced = s.clone();
    advanced.metrics.benchmark_pass = true;
    advanced.metrics.user_approval_count = 100;
    let predicate = Default::default();
    // No self-approval: the timestamp is the only way to advance. The
    // function does not write `last_user_approval` by itself.
    let after = advance(advanced, predicate, Utc::now(), "audit");
    assert_eq!(after.mode, AutonomyMode::Autonomous);
    assert!(after.last_user_approval.is_some());
}

#[test]
fn demotion_is_immediate_on_regression() {
    let s = initial_state(
        fmt(),
        vec!["creator-minimal".to_string()],
        "reviewed-v2".to_string(),
    );
    let mut advanced = s.clone();
    advanced.metrics.benchmark_pass = true;
    advanced.metrics.user_approval_count = 5;
    let advanced = advance(advanced, Default::default(), Utc::now(), "audit");
    assert_eq!(advanced.mode, AutonomyMode::Autonomous);
    let mut regressed = advanced.clone();
    regressed.metrics.regression_count = 1;
    let predicate = AutonomyDemotionPredicate::default();
    assert!(has_regression_trigger(&regressed, predicate));
    let dropped = demote(
        regressed,
        AutonomyTransitionReason::BenchmarkRegression,
        "audit",
    );
    assert_eq!(dropped.mode, AutonomyMode::Reviewed);
    assert!(dropped.demoted);
}

#[test]
fn demotion_on_incompatible_pack_change() {
    let s = initial_state(
        fmt(),
        vec!["creator-minimal".to_string()],
        "reviewed-v2".to_string(),
    );
    let mut advanced = s.clone();
    advanced.metrics.benchmark_pass = true;
    advanced.metrics.user_approval_count = 3;
    let advanced = advance(advanced, Default::default(), Utc::now(), "audit");
    let mut regressed = advanced.clone();
    regressed.compatible_pack_set = vec!["creator-2".to_string()];
    let dropped = demote(
        regressed,
        AutonomyTransitionReason::IncompatiblePackChange,
        "audit",
    );
    assert_eq!(dropped.mode, AutonomyMode::Reviewed);
}

#[test]
fn rejected_final_demotes() {
    let s = initial_state(
        fmt(),
        vec!["creator-minimal".to_string()],
        "reviewed-v2".to_string(),
    );
    let mut advanced = s.clone();
    advanced.metrics.benchmark_pass = true;
    advanced.metrics.user_approval_count = 3;
    let advanced = advance(advanced, Default::default(), Utc::now(), "audit");
    let dropped = demote(advanced, AutonomyTransitionReason::RejectedFinal, "audit");
    assert_eq!(dropped.mode, AutonomyMode::Reviewed);
}

#[test]
fn transitions_history_grows() {
    let s = initial_state(
        fmt(),
        vec!["creator-minimal".to_string()],
        "reviewed-v2".to_string(),
    );
    let after = demote(s, AutonomyTransitionReason::CriticDisagreement, "audit");
    assert_eq!(transitions(&after).len(), 2);
}
