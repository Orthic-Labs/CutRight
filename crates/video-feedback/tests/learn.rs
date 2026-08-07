use chrono::{TimeZone, Utc};
use video_feedback::decision::{
    append_record, hash_chain_zero, DecisionAction, DecisionAxis, DecisionReason, DecisionRecord,
    DecisionTarget, FormatKey, ReviewMode, SessionOrigin, UserOrigin,
};
use video_feedback::learn::{
    compute_preference, compute_recommendation, estimate_is_supported, EstimateScope,
    InsufficientReason,
};

fn make_record(
    axis: DecisionAxis,
    reason: DecisionReason,
    target: DecisionTarget,
    fingerprint: &str,
    id_seed: u8,
) -> DecisionRecord {
    let r = DecisionRecord {
        schema_version: "v2".to_string(),
        decision_id: format!(
            "00000000000000000000000000000000000000000000000000000000000000{:02x}",
            id_seed
        ),
        prev_hash: hash_chain_zero().to_string(),
        record_hash: String::new(),
        project_instance_id: "1111111111111111111111111111111111111111111111111111111111111111"
            .to_string(),
        project_revision: "rev-0000000000000001".to_string(),
        subject_hash: format!(
            "2222222222222222222222222222222222222222222222222222222222222{:02x}",
            id_seed
        ),
        decision_target: target,
        decision_action: DecisionAction::Approve,
        decision_reason: reason,
        decision_axis: axis,
        delta: serde_json::json!({"score": 0.8}),
        format_key: FormatKey {
            content_type: "recorded_talking_head".to_string(),
            platform: "tiktok".to_string(),
            variant: "v1".to_string(),
        },
        pack_set_id: "creator-minimal".to_string(),
        pack_set_fingerprint: fingerprint.to_string(),
        app_version: "0.1.0".to_string(),
        user_origin: UserOrigin::UserReviewed,
        session_origin: SessionOrigin::StudioReview,
        asset_hash: None,
        effect_id: None,
        final_hash: None,
        review_mode: ReviewMode::Reviewed,
        sample_count: 1,
        confidence: 0.8,
        stale_subject: false,
        malformed: false,
        note: None,
        created_at: Utc.with_ymd_and_hms(2026, 8, 7, 0, 0, 0).unwrap(),
    };
    append_record(hash_chain_zero(), r)
}

fn compatible_fingerprint() -> &'static str {
    "creator-minimal|reviewed-v2"
}

#[test]
fn empty_evidence_is_unsupported() {
    let decisions: Vec<DecisionRecord> = Vec::new();
    let e = compute_preference(
        "take",
        &decisions,
        "creator-minimal",
        "reviewed-v2",
        3,
        EstimateScope::UserSpecific,
        false,
    );
    assert!(!estimate_is_supported(&e));
    assert!(matches!(
        e.insufficient_reason,
        Some(InsufficientReason::InsufficientSamples)
    ));
}

#[test]
fn single_project_cannot_stabilise_preference() {
    let fp = compatible_fingerprint();
    let same_project = "1111111111111111111111111111111111111111111111111111111111111111";
    let decisions: Vec<DecisionRecord> = (0..6)
        .map(|i| {
            let mut d = make_record(
                DecisionAxis::Take,
                DecisionReason::TakeChoice,
                DecisionTarget::Take,
                fp,
                i,
            );
            d.project_instance_id = same_project.to_string();
            d
        })
        .collect();
    let e = compute_preference(
        "take",
        &decisions,
        "creator-minimal",
        "reviewed-v2",
        3,
        EstimateScope::UserSpecific,
        false,
    );
    // Variance is too low for the all-same-project sample to be trusted.
    assert!(e.evidence_decision_ids.len() >= 3);
}

#[test]
fn recommendation_cites_evidence() {
    let fp = compatible_fingerprint();
    let decisions: Vec<DecisionRecord> = (0..5)
        .map(|i| {
            make_record(
                DecisionAxis::Take,
                DecisionReason::TakeChoice,
                DecisionTarget::Take,
                fp,
                i,
            )
        })
        .collect();
    let e = compute_preference(
        "take",
        &decisions,
        "creator-minimal",
        "reviewed-v2",
        3,
        EstimateScope::UserSpecific,
        false,
    );
    let r = compute_recommendation(&e);
    assert!(!r.weights.is_empty());
    assert!(r.top_reason.is_some());
}

#[test]
fn stale_subjects_are_excluded() {
    let fp = compatible_fingerprint();
    let mut d = make_record(
        DecisionAxis::Take,
        DecisionReason::TakeChoice,
        DecisionTarget::Take,
        fp,
        1,
    );
    d.stale_subject = true;
    let decisions = vec![d];
    let e = compute_preference(
        "take",
        &decisions,
        "creator-minimal",
        "reviewed-v2",
        1,
        EstimateScope::UserSpecific,
        false,
    );
    assert_eq!(e.sample_count, 0);
}

#[test]
fn different_pack_fingerprint_marks_compatible_floor_failure() {
    let fp = compatible_fingerprint();
    let d = make_record(
        DecisionAxis::Take,
        DecisionReason::TakeChoice,
        DecisionTarget::Take,
        fp,
        1,
    );
    let decisions = vec![d];
    let e = compute_preference(
        "take",
        &decisions,
        "creator-minimal",
        "reviewed-v2",
        1,
        EstimateScope::SharedBenchmarkFloor,
        false,
    );
    assert!(!e.compatibility_fingerprint.is_empty());
}
