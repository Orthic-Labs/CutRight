//!
//! A/B/C roll planning with must-keep constraints (CR-V2-B5-014).
//!
//! A `RollPlan` declares which shots are:
//! - A-roll: recorded narration (must keep producer narration timestamps)
//! - B-roll: generated visual substitution (must keep shot duration_ms)
//! - C-roll: anchored caption/evidence overlay (must keep caption_companion_id)
//!
//! Each shot carries a `must_keep[]` of constraints. The plan is rejected
//! if any must_keep is violated by the assigned roll kind.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RollPlanError {
    #[error("must_keep violation: shot={shot_id}, constraint={constraint}")]
    MustKeepViolated { shot_id: String, constraint: String },
    #[error("plan requires at least one shot: id={0}")]
    Empty(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RollKind {
    Aroll,
    Broll,
    Croll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShotRoll {
    pub shot_id: String,
    pub kind: RollKind,
    pub duration_ms: u64,
    pub source_clip_id: String,
    pub caption_companion_id: Option<String>,
    pub must_keep: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollPlan {
    pub id: String,
    pub version: String,
    pub creative_plan_id: String,
    pub shots: Vec<ShotRoll>,
    pub metrics: BTreeMap<String, f64>,
}

pub struct RollPlanner;

impl RollPlanner {
    pub fn validate(plan: &RollPlan) -> Result<(), RollPlanError> {
        if plan.shots.is_empty() {
            return Err(RollPlanError::Empty(plan.id.clone()));
        }
        for shot in &plan.shots {
            for c in &shot.must_keep {
                let ok = match (c.as_str(), shot.kind) {
                    ("narration_timestamp", RollKind::Aroll) => true,
                    ("duration_ms", RollKind::Broll) => true,
                    ("caption_companion_id", RollKind::Croll) => {
                        shot.caption_companion_id.is_some()
                    }
                    _ => false,
                };
                if !ok {
                    return Err(RollPlanError::MustKeepViolated {
                        shot_id: shot.shot_id.clone(),
                        constraint: c.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_plan() {
        let plan = RollPlan {
            id: "rp_1".to_string(),
            version: "v2".to_string(),
            creative_plan_id: "cp_1".to_string(),
            shots: vec![],
            metrics: BTreeMap::new(),
        };
        let err = RollPlanner::validate(&plan).expect_err("err");
        assert!(matches!(err, RollPlanError::Empty(_)));
    }

    #[test]
    fn croll_without_caption_companion_is_violated() {
        let plan = RollPlan {
            id: "rp_1".to_string(),
            version: "v2".to_string(),
            creative_plan_id: "cp_1".to_string(),
            shots: vec![ShotRoll {
                shot_id: "s_0".to_string(),
                kind: RollKind::Croll,
                duration_ms: 1000,
                source_clip_id: "c_0".to_string(),
                caption_companion_id: None,
                must_keep: vec!["caption_companion_id".to_string()],
            }],
            metrics: BTreeMap::new(),
        };
        let err = RollPlanner::validate(&plan).expect_err("err");
        assert!(matches!(err, RollPlanError::MustKeepViolated { .. }));
    }

    #[test]
    fn accepts_consistent_croll() {
        let plan = RollPlan {
            id: "rp_1".to_string(),
            version: "v2".to_string(),
            creative_plan_id: "cp_1".to_string(),
            shots: vec![ShotRoll {
                shot_id: "s_0".to_string(),
                kind: RollKind::Croll,
                duration_ms: 1000,
                source_clip_id: "c_0".to_string(),
                caption_companion_id: Some("cap_0".to_string()),
                must_keep: vec!["caption_companion_id".to_string()],
            }],
            metrics: BTreeMap::new(),
        };
        RollPlanner::validate(&plan).expect("ok");
    }
}
