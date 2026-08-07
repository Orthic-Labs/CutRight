//!
//! Lane merger and versioned CompiledFinishPlan compiler (CR-V2-B5-022).
//!
//! The merger consumes the per-lane `CompiledPlan` outputs and emits a
//! single `CompiledFinishPlan` for the job plane. The CompiledFinishPlan carries:
//! - the `render_plan` (from the render-graph compiler)
//! - the `caption_plan` (from native typography)
//! - the `motion_plan` (from native motion)
//! - the `audio_plan` (from native audio)
//! - the `audit` (from the creative critic)
//!
//! Each subplan is referenced by `id`. The CompiledFinishPlan is versioned; the
//! lane roster and the schema fields are frozen by `CR-V2-B5-006`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompiledFinishPlanError {
    #[error("missing subplan: {0}")]
    MissingSubplan(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledFinishPlan {
    pub id: String,
    pub version: String,
    pub creative_plan_id: String,
    pub render_plan_id: String,
    pub caption_plan_id: String,
    pub motion_plan_id: String,
    pub audio_plan_id: String,
    pub audit_id: String,
    pub metrics: BTreeMap<String, f64>,
}

pub struct CompiledFinishPlanCompiler;

impl CompiledFinishPlanCompiler {
    pub fn compile(
        creative_plan_id: &str,
        render_plan_id: &str,
        caption_plan_id: &str,
        motion_plan_id: &str,
        audio_plan_id: &str,
        audit_id: &str,
    ) -> Result<CompiledFinishPlan, CompiledFinishPlanError> {
        if creative_plan_id.is_empty() {
            return Err(CompiledFinishPlanError::MissingSubplan(
                "creative_plan_id".to_string(),
            ));
        }
        if render_plan_id.is_empty() {
            return Err(CompiledFinishPlanError::MissingSubplan(
                "render_plan_id".to_string(),
            ));
        }
        if caption_plan_id.is_empty() {
            return Err(CompiledFinishPlanError::MissingSubplan(
                "caption_plan_id".to_string(),
            ));
        }
        if motion_plan_id.is_empty() {
            return Err(CompiledFinishPlanError::MissingSubplan(
                "motion_plan_id".to_string(),
            ));
        }
        if audio_plan_id.is_empty() {
            return Err(CompiledFinishPlanError::MissingSubplan(
                "audio_plan_id".to_string(),
            ));
        }
        if audit_id.is_empty() {
            return Err(CompiledFinishPlanError::MissingSubplan(
                "audit_id".to_string(),
            ));
        }
        Ok(CompiledFinishPlan {
            id: format!("fpl_{}", creative_plan_id),
            version: "v2".to_string(),
            creative_plan_id: creative_plan_id.to_string(),
            render_plan_id: render_plan_id.to_string(),
            caption_plan_id: caption_plan_id.to_string(),
            motion_plan_id: motion_plan_id.to_string(),
            audio_plan_id: audio_plan_id.to_string(),
            audit_id: audit_id.to_string(),
            metrics: BTreeMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_valid_finish_plan() {
        let plan = CompiledFinishPlanCompiler::compile(
            "cp_1",
            "compiled_rg_1",
            "layout_cd_1",
            "motion_mc_1",
            "af_ap_1",
            "audit_1",
        )
        .expect("ok");
        assert_eq!(plan.creative_plan_id, "cp_1");
        assert_eq!(plan.audit_id, "audit_1");
    }

    #[test]
    fn rejects_missing_audit() {
        let err = CompiledFinishPlanCompiler::compile("cp_1", "r", "c", "m", "a", "")
            .err()
            .expect("err");
        assert!(matches!(err, CompiledFinishPlanError::MissingSubplan(_)));
    }
}
