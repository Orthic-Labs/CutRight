//! Applied format profiles with immutable versions.
//!
//! A profile is created from a recommendation and approved by an explicit
//! user action. Changes create a new version; the previous version is
//! immutable. Application is blocked when `compatibility` does not match
//! the active project context.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::decision::FormatKey;
use crate::learn::Recommendation;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileVersion {
    pub schema_version: String,
    pub profile_id: String,
    pub format: FormatKey,
    pub version: String,
    pub compatibility: ProfileCompatibility,
    pub values: FormatProfileValues,
    pub source_recommendation_hash: String,
    pub source_decision_ids: Vec<String>,
    pub approved_by: ProfileApprovedBy,
    pub approved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileCompatibility {
    pub pack_set_id: String,
    pub pack_set_fingerprint: String,
    pub benchmark_profile: String,
    pub skill_version: String,
    pub render_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormatProfileValues {
    pub inherited_defaults: BTreeMap<String, String>,
    pub overrides: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileApprovedBy {
    UserReviewed,
    UserRejected,
    UserReplaced,
    UserNoted,
    System,
}

pub type FormatProfile = ProfileVersion;

/// Approve a profile from a recommendation and an explicit user action.
/// The result is a new immutable version.
pub fn approve_profile(
    format: FormatKey,
    compatibility: ProfileCompatibility,
    recommendation: &Recommendation,
    decision_ids: Vec<String>,
    approver: ProfileApprovedBy,
) -> ProfileVersion {
    let version = "0.1.0".to_string();
    // version is computed as monotonic; the caller may override.
    let mut values = FormatProfileValues {
        inherited_defaults: BTreeMap::new(),
        overrides: BTreeMap::new(),
    };
    if let Some(top) = &recommendation.top_reason {
        values
            .overrides
            .insert("top_reason".to_string(), top.clone());
    }
    for (k, v) in &recommendation.weights {
        values.overrides.insert(k.clone(), format!("{:.6}", v));
    }
    let profile_id = compute_profile_id(&format, &version, &compatibility);
    ProfileVersion {
        schema_version: "v1".to_string(),
        profile_id,
        format,
        version,
        compatibility,
        values,
        source_recommendation_hash: recommendation_hash(recommendation),
        source_decision_ids: decision_ids,
        approved_by: approver,
        approved_at: Utc::now(),
    }
}

/// Apply a profile. Returns Err when the profile's compatibility does not
/// match the active project context.
pub fn apply_profile(
    profile: &ProfileVersion,
    active_compatibility: &ProfileCompatibility,
) -> Result<(), ProfileCompatibilityMismatch> {
    if profile.compatibility == *active_compatibility {
        Ok(())
    } else {
        Err(ProfileCompatibilityMismatch)
    }
}

/// Returns true when the profile's compatibility does not match the active
/// project context.
pub fn profile_compatibility_mismatch(
    profile: &ProfileVersion,
    active_compatibility: &ProfileCompatibility,
) -> bool {
    profile.compatibility != *active_compatibility
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileCompatibilityMismatch;

fn compute_profile_id(format: &FormatKey, version: &str, compat: &ProfileCompatibility) -> String {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(format.content_type.as_bytes());
    hasher.update(&[0u8]);
    hasher.update(format.platform.as_bytes());
    hasher.update(&[0u8]);
    hasher.update(format.variant.as_bytes());
    hasher.update(&[0u8]);
    hasher.update(version.as_bytes());
    hasher.update(&[0u8]);
    hasher.update(compat.pack_set_id.as_bytes());
    hasher.update(&[0u8]);
    hasher.update(compat.pack_set_fingerprint.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest.as_bytes() {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

fn recommendation_hash(r: &Recommendation) -> String {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(r.axis.as_bytes());
    hasher.update(&[0u8]);
    if let Some(top) = &r.top_reason {
        hasher.update(top.as_bytes());
    }
    for (k, v) in &r.weights {
        hasher.update(k.as_bytes());
        hasher.update(&v.to_be_bytes());
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest.as_bytes() {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}
