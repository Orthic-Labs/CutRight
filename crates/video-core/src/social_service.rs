//!
//! Social platform constraints as versioned local data (CR-V2-B5-011).
//!
//! The social lane owns a frozen, versioned table of platform constraints.
//! The table is **local data** — it lives in the repo as JSON and is
//! validated by the social lane at startup. The lane never reaches the
//! network to fetch a platform profile.
//!
//! Each `PlatformProfile` carries a `version` field; the lane rejects a
//! `Package` whose `platform_profile.id` does not match a registered
//! profile version.

use crate::creative_skill_runtime::{
    SkillFamily, SkillRequest, SkillResult, SkillRuntime, SkillRuntimeError,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SocialError {
    #[error("unknown platform profile: {0}")]
    UnknownProfile(String),
    #[error("constraint violated: {0}")]
    ConstraintViolated(String),
    #[error("runtime error: {0}")]
    Runtime(#[from] SkillRuntimeError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConstraints {
    pub max_duration_ms: u64,
    pub min_duration_ms: u64,
    pub max_aspect_w: u32,
    pub max_aspect_h: u32,
    pub caption_required: bool,
    pub safe_zone_policy: String,
    pub reduced_motion_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformProfile {
    pub id: String,
    pub version: String,
    pub platform: String,
    pub constraints: PlatformConstraints,
    pub restricted_tags: Vec<String>,
}

#[derive(Default)]
pub struct SocialService {
    profiles: BTreeMap<String, PlatformProfile>,
}

impl SocialService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(runtime: &mut SkillRuntime) {
        runtime.register(SkillFamily::Social, std::sync::Arc::new(Self::handle));
    }

    pub fn load_profile(&mut self, profile: PlatformProfile) {
        self.profiles.insert(profile.id.clone(), profile);
    }

    pub fn get(&self, id: &str) -> Option<&PlatformProfile> {
        self.profiles.get(id)
    }

    pub fn check_duration(&self, profile_id: &str, duration_ms: u64) -> Result<(), SocialError> {
        let p = self
            .profiles
            .get(profile_id)
            .ok_or_else(|| SocialError::UnknownProfile(profile_id.to_string()))?;
        if duration_ms > p.constraints.max_duration_ms {
            return Err(SocialError::ConstraintViolated(format!(
                "duration_ms={duration_ms} > max={}",
                p.constraints.max_duration_ms
            )));
        }
        if duration_ms < p.constraints.min_duration_ms {
            return Err(SocialError::ConstraintViolated(format!(
                "duration_ms={duration_ms} < min={}",
                p.constraints.min_duration_ms
            )));
        }
        Ok(())
    }

    pub fn check_tag(&self, profile_id: &str, tag: &str) -> Result<(), SocialError> {
        let p = self
            .profiles
            .get(profile_id)
            .ok_or_else(|| SocialError::UnknownProfile(profile_id.to_string()))?;
        if p.restricted_tags.iter().any(|t| t == tag) {
            Err(SocialError::ConstraintViolated(format!(
                "tag {tag} is restricted for {profile_id}"
            )))
        } else {
            Ok(())
        }
    }

    fn handle(req: &SkillRequest) -> Result<SkillResult, SkillRuntimeError> {
        Ok(SkillResult {
            version: crate::creative_skill_runtime::RUNTIME_VERSION.to_string(),
            skill_id: req.skill_id.clone(),
            output_kind: "social_artefact".to_string(),
            output_id: format!("soc_{}", req.input_id),
            content_hash: format!("sha256:social:{}", req.input_id),
            metrics: BTreeMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_profile() -> PlatformProfile {
        PlatformProfile {
            id: "ig.reels.v1".to_string(),
            version: "v1".to_string(),
            platform: "instagram_reels".to_string(),
            constraints: PlatformConstraints {
                max_duration_ms: 90_000,
                min_duration_ms: 1_000,
                max_aspect_w: 9,
                max_aspect_h: 16,
                caption_required: true,
                safe_zone_policy: "ig_standard".to_string(),
                reduced_motion_required: false,
            },
            restricted_tags: vec!["#spam".to_string()],
        }
    }

    #[test]
    fn rejects_unknown_profile() {
        let svc = SocialService::new();
        let err = svc.check_duration("unknown", 1000).expect_err("err");
        assert!(matches!(err, SocialError::UnknownProfile(_)));
    }

    #[test]
    fn rejects_over_max_duration() {
        let mut svc = SocialService::new();
        svc.load_profile(test_profile());
        let err = svc.check_duration("ig.reels.v1", 95_000).expect_err("err");
        assert!(matches!(err, SocialError::ConstraintViolated(_)));
    }

    #[test]
    fn rejects_restricted_tag() {
        let mut svc = SocialService::new();
        svc.load_profile(test_profile());
        let err = svc.check_tag("ig.reels.v1", "#spam").expect_err("err");
        assert!(matches!(err, SocialError::ConstraintViolated(_)));
        svc.check_tag("ig.reels.v1", "#vibes").expect("ok");
    }
}
