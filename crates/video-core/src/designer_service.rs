//!
//! Designer lane (CR-V2-B5-009): typed asset planner and reviewer.
//!
//! The designer lane is the **only** lane that may issue an `AssetRequest`
//! on the producer side and the **only** lane that may issue an `AssetReview`
//! on the consumer side. Other lanes read or write assets but never both
//! sides of the planning-review contract.

use crate::creative_skill_runtime::{
    SkillFamily, SkillRequest, SkillResult, SkillRuntime, SkillRuntimeError,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DesignerError {
    #[error("asset request rejected: {0}")]
    RequestRejected(String),
    #[error("asset review rejected: {0}")]
    ReviewRejected(String),
    #[error("runtime error: {0}")]
    Runtime(#[from] SkillRuntimeError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRequest {
    pub id: String,
    pub version: String,
    pub kind: String,
    pub description: String,
    pub evidence_refs: Vec<String>,
    pub policy_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetReview {
    pub id: String,
    pub version: String,
    pub request_id: String,
    pub verdict: String,
    pub reviewer: String,
}

pub struct DesignerService {
    _private: (),
}

impl Default for DesignerService {
    fn default() -> Self {
        Self::new()
    }
}

impl DesignerService {
    pub fn new() -> Self {
        Self { _private: () }
    }

    pub fn register(runtime: &mut SkillRuntime) {
        runtime.register(SkillFamily::Designer, std::sync::Arc::new(Self::handle));
    }

    fn handle(req: &SkillRequest) -> Result<SkillResult, SkillRuntimeError> {
        Ok(SkillResult {
            version: crate::creative_skill_runtime::RUNTIME_VERSION.to_string(),
            skill_id: req.skill_id.clone(),
            output_kind: "designer_artefact".to_string(),
            output_id: format!("dsg_{}", req.input_id),
            content_hash: format!("sha256:designer:{}", req.input_id),
            metrics: BTreeMap::new(),
        })
    }

    pub fn plan_request(req: &AssetRequest) -> Result<(), DesignerError> {
        if req.evidence_refs.is_empty() {
            return Err(DesignerError::RequestRejected(
                "asset_request requires at least one evidence_ref".to_string(),
            ));
        }
        Ok(())
    }

    pub fn review(review: &AssetReview) -> Result<(), DesignerError> {
        if review.request_id.is_empty() {
            return Err(DesignerError::ReviewRejected(
                "asset_review requires request_id".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_request_without_evidence_refs() {
        let req = AssetRequest {
            id: "ar_1".to_string(),
            version: "v2".to_string(),
            kind: "image".to_string(),
            description: "Cover frame".to_string(),
            evidence_refs: vec![],
            policy_ref: None,
        };
        let err = DesignerService::plan_request(&req).err().expect("err");
        assert!(matches!(err, DesignerError::RequestRejected(_)));
    }

    #[test]
    fn accepts_request_with_evidence_refs() {
        let req = AssetRequest {
            id: "ar_1".to_string(),
            version: "v2".to_string(),
            kind: "image".to_string(),
            description: "Cover frame".to_string(),
            evidence_refs: vec!["evidence:ev_1".to_string()],
            policy_ref: None,
        };
        DesignerService::plan_request(&req).expect("ok");
    }

    #[test]
    fn rejects_review_with_empty_request_id() {
        let review = AssetReview {
            id: "arv_1".to_string(),
            version: "v2".to_string(),
            request_id: "".to_string(),
            verdict: "pass".to_string(),
            reviewer: "designer".to_string(),
        };
        let err = DesignerService::review(&review).err().expect("err");
        assert!(matches!(err, DesignerError::ReviewRejected(_)));
    }
}
