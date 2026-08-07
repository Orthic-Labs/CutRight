//! Brand and Brand Identity typed services (CR-V2-B5-008).
//!
//! Both services are bound to the skill runtime via a `SkillFamily::Brand`
//! or `SkillFamily::BrandIdentity` handler. They are read-only with respect
//! to the brand tokens and never mutate the brand card.

use crate::creative_skill_runtime::{
    SkillFamily, SkillRequest, SkillResult, SkillRuntime, SkillRuntimeError,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrandServiceError {
    #[error("locked brand token cannot be overwritten: {0}")]
    LockedToken(String),
    #[error("runtime error: {0}")]
    Runtime(#[from] SkillRuntimeError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandCard {
    pub id: String,
    pub version: String,
    pub name: String,
    pub voice: String,
    pub typography: BTreeMap<String, String>,
    pub palette: BTreeMap<String, String>,
    pub marks: Vec<String>,
    pub motion_language: String,
    pub audio_identity: String,
    pub restrictions: Vec<String>,
    pub accessibility: BTreeMap<String, String>,
    pub provenance: String,
    pub locked_token_ids: Vec<String>,
}

impl BrandCard {
    pub fn has_locked(&self, token_id: &str) -> bool {
        self.locked_token_ids.iter().any(|t| t == token_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandSystem {
    pub id: String,
    pub version: String,
    pub cards: Vec<BrandCard>,
    pub locked_ids: Vec<String>,
}

pub struct BrandService {
    _private: (),
}

impl Default for BrandService {
    fn default() -> Self {
        Self::new()
    }
}

impl BrandService {
    pub fn new() -> Self {
        Self { _private: () }
    }

    pub fn register(runtime: &mut SkillRuntime) {
        runtime.register(SkillFamily::Brand, std::sync::Arc::new(Self::handle));
    }

    fn handle(req: &SkillRequest) -> Result<SkillResult, SkillRuntimeError> {
        // Deterministic stub: a real impl would load the brand card from the
        // evidence store and compute the requested derived artefact.
        Ok(SkillResult {
            version: crate::creative_skill_runtime::RUNTIME_VERSION.to_string(),
            skill_id: req.skill_id.clone(),
            output_kind: "brand_derived".to_string(),
            output_id: format!("bd_{}", req.input_id),
            content_hash: format!("sha256:brand:{}", req.input_id),
            metrics: BTreeMap::new(),
        })
    }

    pub fn assert_token_not_locked(
        card: &BrandCard,
        token_id: &str,
    ) -> Result<(), BrandServiceError> {
        if card.has_locked(token_id) {
            Err(BrandServiceError::LockedToken(token_id.to_string()))
        } else {
            Ok(())
        }
    }
}

pub struct BrandIdentityService {
    _private: (),
}

impl Default for BrandIdentityService {
    fn default() -> Self {
        Self::new()
    }
}

impl BrandIdentityService {
    pub fn new() -> Self {
        Self { _private: () }
    }

    pub fn register(runtime: &mut SkillRuntime) {
        runtime.register(
            SkillFamily::BrandIdentity,
            std::sync::Arc::new(Self::handle),
        );
    }

    fn handle(req: &SkillRequest) -> Result<SkillResult, SkillRuntimeError> {
        Ok(SkillResult {
            version: crate::creative_skill_runtime::RUNTIME_VERSION.to_string(),
            skill_id: req.skill_id.clone(),
            output_kind: "brand_identity_derived".to_string(),
            output_id: format!("bi_{}", req.input_id),
            content_hash: format!("sha256:brandidentity:{}", req.input_id),
            metrics: BTreeMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn brand_service_registers_and_executes() {
        let mut rt = SkillRuntime::new();
        BrandService::register(&mut rt);
        let req = SkillRequest {
            version: crate::creative_skill_runtime::RUNTIME_VERSION.to_string(),
            skill_family: SkillFamily::Brand,
            skill_id: "brand.derive_typography".to_string(),
            input_kind: "brand_card".to_string(),
            input_id: "bc_1".to_string(),
            seed: Some(1),
            policy_ref: None,
        };
        let (result, _trace) = rt.execute(&req).expect("must execute");
        assert_eq!(result.output_id, "bd_bc_1");
    }

    #[test]
    fn locked_token_mutation_is_rejected() {
        let card = BrandCard {
            id: "bc_1".to_string(),
            version: "v2".to_string(),
            name: "Test".to_string(),
            voice: "Calm".to_string(),
            typography: BTreeMap::new(),
            palette: BTreeMap::new(),
            marks: vec![],
            motion_language: "smooth".to_string(),
            audio_identity: "warm".to_string(),
            restrictions: vec![],
            accessibility: BTreeMap::new(),
            provenance: "brand_card.json".to_string(),
            locked_token_ids: vec!["mark.primary".to_string()],
        };
        let err = BrandService::assert_token_not_locked(&card, "mark.primary")
            .err()
            .expect("err");
        assert!(matches!(err, BrandServiceError::LockedToken(_)));
        BrandService::assert_token_not_locked(&card, "color.bg").expect("ok");
    }

    #[test]
    fn brand_identity_service_registers() {
        let mut rt = SkillRuntime::new();
        BrandIdentityService::register(&mut rt);
        let req = SkillRequest {
            version: crate::creative_skill_runtime::RUNTIME_VERSION.to_string(),
            skill_family: SkillFamily::BrandIdentity,
            skill_id: "brand_identity.lock".to_string(),
            input_kind: "brand_system".to_string(),
            input_id: "bs_1".to_string(),
            seed: None,
            policy_ref: None,
        };
        let (result, _trace) = rt.execute(&req).expect("must execute");
        assert_eq!(result.output_id, "bi_bs_1");
    }
}
