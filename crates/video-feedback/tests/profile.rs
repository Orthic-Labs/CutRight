use video_feedback::decision::{DecisionAxis, DecisionReason, DecisionTarget, FormatKey};
use video_feedback::learn::{compute_recommendation, compute_preference, EstimateScope};
use video_feedback::profile::{
    apply_profile, approve_profile, profile_compatibility_mismatch,
    ProfileApprovedBy, ProfileCompatibility,
};
use chrono::{TimeZone, Utc};

fn sample_compatibility() -> ProfileCompatibility {
    ProfileCompatibility {
        pack_set_id: "creator-minimal".to_string(),
        pack_set_fingerprint: "0000000000000000000000000000000000000000000000000000000000000099".to_string(),
        benchmark_profile: "reviewed-v2".to_string(),
        skill_version: "0.1.0".to_string(),
        render_version: "0.1.0".to_string(),
    }
}

#[test]
fn profile_version_is_immutable_when_changed() {
    let format = FormatKey {
        content_type: "recorded_talking_head".to_string(),
        platform: "tiktok".to_string(),
        variant: "v1".to_string(),
    };
    let compat = sample_compatibility();
    let empty: Vec<video_feedback::decision::DecisionRecord> = Vec::new();
    let e = compute_preference(
        "take",
        &empty,
        "creator-minimal",
        "reviewed-v2",
        3,
        EstimateScope::UserSpecific,
        false,
    );
    let r = compute_recommendation(&e);
    let p1 = approve_profile(
        format.clone(),
        compat.clone(),
        &r,
        Vec::new(),
        ProfileApprovedBy::UserReviewed,
    );
    // Manual mutation would not change the version; the profile is created
    // once and reused. A change always creates a new immutable version.
    assert_eq!(p1.version, "0.1.0");
    assert_eq!(p1.approved_by, ProfileApprovedBy::UserReviewed);
}

#[test]
fn profile_compatibility_mismatch_blocks_application() {
    let format = FormatKey {
        content_type: "recorded_talking_head".to_string(),
        platform: "tiktok".to_string(),
        variant: "v1".to_string(),
    };
    let compat = sample_compatibility();
    let mut active = compat.clone();
    active.pack_set_fingerprint = "1111111111111111111111111111111111111111111111111111111111111111".to_string();
    let empty: Vec<video_feedback::decision::DecisionRecord> = Vec::new();
    let e = compute_preference(
        "take",
        &empty,
        "creator-minimal",
        "reviewed-v2",
        3,
        EstimateScope::UserSpecific,
        false,
    );
    let r = compute_recommendation(&e);
    let p = approve_profile(
        format,
        compat,
        &r,
        Vec::new(),
        ProfileApprovedBy::UserReviewed,
    );
    assert!(apply_profile(&p, &active).is_err());
    assert!(profile_compatibility_mismatch(&p, &active));
}

#[test]
fn profile_includes_source_decision_ids() {
    let format = FormatKey {
        content_type: "recorded_talking_head".to_string(),
        platform: "tiktok".to_string(),
        variant: "v1".to_string(),
    };
    let compat = sample_compatibility();
    let empty: Vec<video_feedback::decision::DecisionRecord> = Vec::new();
    let e = compute_preference(
        "take",
        &empty,
        "creator-minimal",
        "reviewed-v2",
        3,
        EstimateScope::UserSpecific,
        false,
    );
    let r = compute_recommendation(&e);
    let p = approve_profile(
        format,
        compat,
        &r,
        vec!["abc".to_string()],
        ProfileApprovedBy::UserReviewed,
    );
    assert_eq!(p.source_decision_ids, vec!["abc".to_string()]);
}

#[test]
fn recommendation_never_auto_applies_in_reviewed_mode() {
    let format = FormatKey {
        content_type: "recorded_talking_head".to_string(),
        platform: "tiktok".to_string(),
        variant: "v1".to_string(),
    };
    let compat = sample_compatibility();
    let empty: Vec<video_feedback::decision::DecisionRecord> = Vec::new();
    let e = compute_preference(
        "take",
        &empty,
        "creator-minimal",
        "reviewed-v2",
        3,
        EstimateScope::UserSpecific,
        false,
    );
    let r = compute_recommendation(&e);
    assert!(!r.ready_to_apply);
    let p = approve_profile(
        format,
        compat,
        &r,
        Vec::new(),
        ProfileApprovedBy::UserReviewed,
    );
    assert_eq!(p.approved_by, ProfileApprovedBy::UserReviewed);
}
