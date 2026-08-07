//! Creative skill runtime (CR-V2-B5-007).
//!
//! Book 5 freezes the embedded creative-skill execution contract. This
//! module is the **product-local** runtime that resolves a typed
//! `SkillRequest` (see `schemas/skills/skill-request.schema.v1.json`) to a
//! concrete in-process handler, executes the handler, and emits a
//! `SkillResult` and a `SkillTrace` per the contract.
//!
//! The runtime is **local-only**: it never invokes a network boundary, an
//! HTTP client, a shell, or an executable path. That property is asserted
//! by the deterministic visual QA gate (see `V2-CRITIC-SEMANTICS.md`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

pub const RUNTIME_VERSION: &str = "v2";

/// Skill id family. Mirrors the producer lane roster in
/// `V2-CREATIVE-OS-LANES.md` so a resolver can map a request to a handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillFamily {
    Brand,
    BrandIdentity,
    Designer,
    Writing,
    Social,
    CreativePlan,
    BakeOff,
    RollPlan,
    AssetValidation,
    NativeRenderer,
    NativeTypography,
    NativeMotion,
    NativeAudio,
    CreativeCritic,
}

/// Single typed skill request. Mirrors the schema `version` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRequest {
    pub version: String,
    pub skill_family: SkillFamily,
    pub skill_id: String,
    pub input_kind: String,
    pub input_id: String,
    pub seed: Option<u64>,
    pub policy_ref: Option<String>,
}

/// Single typed skill result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResult {
    pub version: String,
    pub skill_id: String,
    pub output_kind: String,
    pub output_id: String,
    pub content_hash: String,
    pub metrics: BTreeMap<String, f64>,
}

/// Single typed skill trace (matches `skill-trace.schema.v1.json` summary).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTrace {
    pub version: String,
    pub skill_id: String,
    pub handler_id: String,
    pub started_at: String,
    pub finished_at: String,
    pub elapsed_ms: u64,
    pub memory_bytes: u64,
    pub budget: Budget,
    pub evidence_refs: Vec<String>,
}

/// Per-skill budget envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub max_wall_ms: u64,
    pub max_files: u32,
    pub max_cost_units: u32,
}

#[derive(Debug, Error)]
pub enum SkillRuntimeError {
    #[error("unsupported skill family: {0:?}")]
    UnsupportedFamily(SkillFamily),
    #[error("schema version mismatch: request={request}, runtime={runtime}")]
    SchemaVersion { request: String, runtime: String },
    #[error("handler not registered: {0}")]
    HandlerNotRegistered(String),
    #[error("budget exceeded: max_wall_ms={max_wall_ms}, used={used}")]
    BudgetExceeded { max_wall_ms: u64, used: u64 },
}

pub type SkillHandler = dyn Fn(&SkillRequest) -> Result<SkillResult, SkillRuntimeError> + Send + Sync;

/// The product-local runtime. Handlers are registered by `skill_family` and
/// invoked synchronously. There is no IO, no network, no shell.
pub struct SkillRuntime {
    handlers: BTreeMap<SkillFamily, std::sync::Arc<SkillHandler>>,
}

impl std::fmt::Debug for SkillRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillRuntime")
            .field("handlers", &self.handlers.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Default for SkillRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRuntime {
    pub fn new() -> Self {
        Self {
            handlers: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, family: SkillFamily, handler: std::sync::Arc<SkillHandler>) {
        self.handlers.insert(family, handler);
    }

    pub fn has(&self, family: SkillFamily) -> bool {
        self.handlers.contains_key(&family)
    }

    pub fn execute(&self, req: &SkillRequest) -> Result<(SkillResult, SkillTrace), SkillRuntimeError> {
        if req.version != RUNTIME_VERSION {
            return Err(SkillRuntimeError::SchemaVersion {
                request: req.version.clone(),
                runtime: RUNTIME_VERSION.to_string(),
            });
        }
        let handler = self
            .handlers
            .get(&req.skill_family)
            .ok_or(SkillRuntimeError::HandlerNotRegistered(req.skill_id.clone()))?;
        let started_at = "1970-01-01T00:00:00Z".to_string();
        let result = handler(req)?;
        let finished_at = "1970-01-01T00:00:00Z".to_string();
        let trace = SkillTrace {
            version: RUNTIME_VERSION.to_string(),
            skill_id: req.skill_id.clone(),
            handler_id: format!("{:?}", req.skill_family),
            started_at,
            finished_at,
            elapsed_ms: 0,
            memory_bytes: 0,
            budget: Budget {
                max_wall_ms: 60_000,
                max_files: 8,
                max_cost_units: 5,
            },
            evidence_refs: Vec::new(),
        };
        Ok((result, trace))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pass_handler(_req: &SkillRequest) -> Result<SkillResult, SkillRuntimeError> {
        Ok(SkillResult {
            version: RUNTIME_VERSION.to_string(),
            skill_id: "skill.test".to_string(),
            output_kind: "test".to_string(),
            output_id: "out_1".to_string(),
            content_hash: "sha256:deadbeef".to_string(),
            metrics: BTreeMap::new(),
        })
    }

    #[test]
    fn registers_and_executes() {
        let mut rt = SkillRuntime::new();
        rt.register(SkillFamily::Brand, std::sync::Arc::new(pass_handler));
        let req = SkillRequest {
            version: RUNTIME_VERSION.to_string(),
            skill_family: SkillFamily::Brand,
            skill_id: "brand.basic".to_string(),
            input_kind: "brand_card".to_string(),
            input_id: "bc_1".to_string(),
            seed: Some(7),
            policy_ref: None,
        };
        let (result, _trace) = rt.execute(&req).expect("execute");
        assert_eq!(result.skill_id, "skill.test");
    }

    #[test]
    fn rejects_unknown_handler() {
        let rt = SkillRuntime::new();
        let req = SkillRequest {
            version: RUNTIME_VERSION.to_string(),
            skill_family: SkillFamily::NativeAudio,
            skill_id: "audio.basic".to_string(),
            input_kind: "audio".to_string(),
            input_id: "a_1".to_string(),
            seed: None,
            policy_ref: None,
        };
        let err = rt.execute(&req).err().expect("must error");
        assert!(matches!(err, SkillRuntimeError::HandlerNotRegistered(_)));
    }

    #[test]
    fn rejects_schema_version_mismatch() {
        let mut rt = SkillRuntime::new();
        rt.register(SkillFamily::Brand, std::sync::Arc::new(pass_handler));
        let req = SkillRequest {
            version: "v999".to_string(),
            skill_family: SkillFamily::Brand,
            skill_id: "brand.basic".to_string(),
            input_kind: "brand_card".to_string(),
            input_id: "bc_1".to_string(),
            seed: None,
            policy_ref: None,
        };
        let err = rt.execute(&req).err().expect("must error");
        assert!(matches!(err, SkillRuntimeError::SchemaVersion { .. }));
    }
}
