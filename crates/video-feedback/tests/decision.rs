use chrono::{TimeZone, Utc};
use video_feedback::decision::{
    append_record, compute_record_hash, hash_chain_zero, record_hash_mismatch, DecisionAction,
    DecisionAxis, DecisionReason, DecisionRecord, DecisionTarget, FormatKey, ReviewMode,
    SessionOrigin, UserOrigin,
};

fn sample_record() -> DecisionRecord {
    DecisionRecord {
        schema_version: "v2".to_string(),
        decision_id: String::new(),
        prev_hash: hash_chain_zero().to_string(),
        record_hash: String::new(),
        project_instance_id: "1111111111111111111111111111111111111111111111111111111111111111"
            .to_string(),
        project_revision: "rev-0000000000000001".to_string(),
        subject_hash: "2222222222222222222222222222222222222222222222222222222222222222"
            .to_string(),
        decision_target: DecisionTarget::Take,
        decision_action: DecisionAction::Approve,
        decision_reason: DecisionReason::TakeChoice,
        decision_axis: DecisionAxis::Take,
        delta: serde_json::json!({"take_index": 0, "score": 0.91}),
        format_key: FormatKey {
            content_type: "recorded_talking_head".to_string(),
            platform: "tiktok".to_string(),
            variant: "v1".to_string(),
        },
        pack_set_id: "creator-minimal".to_string(),
        pack_set_fingerprint: "3333333333333333333333333333333333333333333333333333333333333333"
            .to_string(),
        app_version: "0.1.0".to_string(),
        user_origin: UserOrigin::UserReviewed,
        session_origin: SessionOrigin::StudioReview,
        asset_hash: Some(
            "4444444444444444444444444444444444444444444444444444444444444444".to_string(),
        ),
        effect_id: None,
        final_hash: Some(
            "5555555555555555555555555555555555555555555555555555555555555555".to_string(),
        ),
        review_mode: ReviewMode::Reviewed,
        sample_count: 1,
        confidence: 0.91,
        stale_subject: false,
        malformed: false,
        note: None,
        created_at: Utc.with_ymd_and_hms(2026, 8, 7, 0, 0, 0).unwrap(),
    }
}

fn make_record(
    target: DecisionTarget,
    reason: DecisionReason,
    axis: DecisionAxis,
) -> DecisionRecord {
    let mut r = sample_record();
    r.decision_target = target;
    r.decision_reason = reason;
    r.decision_axis = axis;
    r
}

#[test]
fn record_hash_is_blake3_hex64() {
    let r = sample_record();
    let h = compute_record_hash(&r);
    assert_eq!(h.len(), 64);
    assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn append_chain_links_prev_hash() {
    let r1 = append_record(hash_chain_zero(), sample_record());
    let r2 = append_record(&r1.record_hash, sample_record());
    assert_eq!(r1.prev_hash, hash_chain_zero());
    assert_eq!(r2.prev_hash, r1.record_hash);
}

#[test]
fn stale_subject_is_retained() {
    let mut r = sample_record();
    r.stale_subject = true;
    let r = append_record(hash_chain_zero(), r);
    assert!(r.stale_subject);
    assert!(!r.malformed);
    assert!(!record_hash_mismatch(&r));
}

#[test]
fn every_axis_is_distinguishable() {
    let r_caption = make_record(
        DecisionTarget::Caption,
        DecisionReason::CaptionChoice,
        DecisionAxis::Caption,
    );
    let r_audio = make_record(
        DecisionTarget::Audio,
        DecisionReason::AudioChoice,
        DecisionAxis::Audio,
    );
    let h_caption = compute_record_hash(&r_caption);
    let h_audio = compute_record_hash(&r_audio);
    assert_ne!(h_caption, h_audio);
}

#[test]
fn empty_hash_is_zero_string() {
    assert_eq!(hash_chain_zero().len(), 64);
    assert!(hash_chain_zero().chars().all(|c| c == '0'));
}
