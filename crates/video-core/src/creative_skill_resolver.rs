//! Creative skill resolver (CR-V2-B5-007).
//!
//! Maps a `SkillRequest` to the producer lane that owns the handler.
//! The resolver is **deterministic** — it never reads from network, env,
//! or filesystem outside the registered registry.

use crate::creative_skill_runtime::{SkillFamily, SkillRequest, SkillRuntime, SkillRuntimeError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("no runtime registered for skill_id={0}")]
    Unresolved(String),
    #[error("runtime error: {0}")]
    Runtime(#[from] SkillRuntimeError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionPlan {
    pub version: String,
    pub skill_id: String,
    pub lane: String,
    pub family: SkillFamily,
    pub policy_ref: Option<String>,
}

/// A static, deterministic resolver. Holds an immutable `SkillRuntime`.
#[derive(Debug, Clone)]
pub struct CreativeSkillResolver {
    runtime: std::sync::Arc<SkillRuntime>,
    /// skill_id (lowercase) → lane name
    skill_to_lane: BTreeMap<String, String>,
}

impl CreativeSkillResolver {
    pub fn new(runtime: std::sync::Arc<SkillRuntime>) -> Self {
        let mut skill_to_lane = BTreeMap::new();
        for fam in [
            SkillFamily::Brand,
            SkillFamily::BrandIdentity,
            SkillFamily::Designer,
            SkillFamily::Writing,
            SkillFamily::Social,
            SkillFamily::CreativePlan,
            SkillFamily::BakeOff,
            SkillFamily::RollPlan,
            SkillFamily::AssetValidation,
            SkillFamily::NativeRenderer,
            SkillFamily::NativeTypography,
            SkillFamily::NativeMotion,
            SkillFamily::NativeAudio,
            SkillFamily::CreativeCritic,
        ] {
            let lane = match fam {
                SkillFamily::Brand => "brand",
                SkillFamily::BrandIdentity => "brand-identity",
                SkillFamily::Designer => "designer",
                SkillFamily::Writing => "writing",
                SkillFamily::Social => "social",
                SkillFamily::CreativePlan => "planning",
                SkillFamily::BakeOff => "planning",
                SkillFamily::RollPlan => "planning",
                SkillFamily::AssetValidation => "asset-validation",
                SkillFamily::NativeRenderer => "native-renderer",
                SkillFamily::NativeTypography => "native-typography",
                SkillFamily::NativeMotion => "native-motion",
                SkillFamily::NativeAudio => "native-audio",
                SkillFamily::CreativeCritic => "creative-critic",
            };
            skill_to_lane.insert(format!("{lane}."), lane.to_string());
        }
        Self {
            runtime,
            skill_to_lane,
        }
    }

    pub fn plan(&self, req: &SkillRequest) -> Result<ResolutionPlan, ResolverError> {
        let key = req.skill_id.to_ascii_lowercase();
        let lane = self
            .skill_to_lane
            .iter()
            .find(|(k, _)| key.starts_with(k.as_str()))
            .map(|(_, v)| v.clone())
            .ok_or_else(|| ResolverError::Unresolved(req.skill_id.clone()))?;
        if !self.runtime.has(req.skill_family) {
            return Err(ResolverError::Runtime(
                SkillRuntimeError::HandlerNotRegistered(req.skill_id.clone()),
            ));
        }
        Ok(ResolutionPlan {
            version: crate::creative_skill_runtime::RUNTIME_VERSION.to_string(),
            skill_id: req.skill_id.clone(),
            lane,
            family: req.skill_family,
            policy_ref: req.policy_ref.clone(),
        })
    }

    pub fn execute(
        &self,
        req: &SkillRequest,
    ) -> Result<
        (
            crate::creative_skill_runtime::SkillResult,
            crate::creative_skill_runtime::SkillTrace,
        ),
        ResolverError,
    > {
        let _plan = self.plan(req)?;
        Ok(self.runtime.execute(req)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creative_skill_runtime::{SkillRequest, SkillResult, SkillRuntime, RUNTIME_VERSION};
    use std::sync::Arc;

    fn pass_handler(req: &SkillRequest) -> Result<SkillResult, SkillRuntimeError> {
        Ok(SkillResult {
            version: RUNTIME_VERSION.to_string(),
            skill_id: req.skill_id.clone(),
            output_kind: "test".to_string(),
            output_id: "out_x".to_string(),
            content_hash: "sha256:00".to_string(),
            metrics: BTreeMap::new(),
        })
    }

    #[test]
    fn resolves_brand_skill_to_brand_lane() {
        let mut rt = SkillRuntime::new();
        rt.register(SkillFamily::Brand, Arc::new(pass_handler));
        let resolver = CreativeSkillResolver::new(Arc::new(rt));
        let req = SkillRequest {
            version: RUNTIME_VERSION.to_string(),
            skill_family: SkillFamily::Brand,
            skill_id: "brand.derive_typography".to_string(),
            input_kind: "brand_card".to_string(),
            input_id: "bc_1".to_string(),
            seed: Some(1),
            policy_ref: None,
        };
        let plan = resolver.plan(&req).expect("plan");
        assert_eq!(plan.lane, "brand");
        assert_eq!(plan.family, SkillFamily::Brand);
    }

    #[test]
    fn rejects_unknown_skill_id() {
        let rt = SkillRuntime::new();
        let resolver = CreativeSkillResolver::new(Arc::new(rt));
        let req = SkillRequest {
            version: RUNTIME_VERSION.to_string(),
            skill_family: SkillFamily::Brand,
            skill_id: "totally.unknown".to_string(),
            input_kind: "x".to_string(),
            input_id: "y".to_string(),
            seed: None,
            policy_ref: None,
        };
        let err = resolver.plan(&req).err().expect("must error");
        assert!(matches!(err, ResolverError::Unresolved(_)));
    }
}
