//!
//! Creative planning: beats, shots, styles from editorial evidence (CR-V2-B5-012).
//!
//! The planning lane consumes an `EditorialPlan` and emits a
//! `CreativePlan` containing three ordered lists:
//! - `beats[]` — high-level narrative beats sourced from the editorial arc
//! - `shots[]` — per-beat shot list with duration_ms and source_clip_id
//! - `styles[]` — per-shot style direction references
//!
//! Every entry must reference at least one `evidence_ref` from the
//! editorial plan. Unreferenced entries are rejected.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlanningError {
    #[error("plan requires at least one evidence_ref: id={0}")]
    UnboundPlan(String),
    #[error("beat requires at least one evidence_ref: id={0}")]
    UnboundBeat(String),
    #[error("shot requires at least one evidence_ref: id={0}")]
    UnboundShot(String),
    #[error("style requires at least one evidence_ref: id={0}")]
    UnboundStyle(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorialPlan {
    pub id: String,
    pub version: String,
    pub arc: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Beat {
    pub id: String,
    pub narrative: String,
    pub duration_ms: u64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shot {
    pub id: String,
    pub beat_id: String,
    pub kind: String,
    pub duration_ms: u64,
    pub source_clip_id: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Style {
    pub id: String,
    pub shot_id: String,
    pub style_direction_id: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativePlan {
    pub id: String,
    pub version: String,
    pub editorial_id: String,
    pub beats: Vec<Beat>,
    pub shots: Vec<Shot>,
    pub styles: Vec<Style>,
    pub metrics: BTreeMap<String, f64>,
}

pub struct CreativePlanner;

impl CreativePlanner {
    pub fn plan(editorial: &EditorialPlan) -> Result<CreativePlan, PlanningError> {
        if editorial.evidence_refs.is_empty() {
            return Err(PlanningError::UnboundPlan(editorial.id.clone()));
        }
        let mut plan = CreativePlan {
            id: format!("cp_{}", editorial.id),
            version: "v2".to_string(),
            editorial_id: editorial.id.clone(),
            beats: Vec::new(),
            shots: Vec::new(),
            styles: Vec::new(),
            metrics: BTreeMap::new(),
        };

        for (i, arc) in editorial.arc.iter().enumerate() {
            let beat_id = format!("b_{i}");
            plan.beats.push(Beat {
                id: beat_id.clone(),
                narrative: arc.clone(),
                duration_ms: 5_000,
                evidence_refs: vec![editorial.evidence_refs[0].clone()],
            });
            let shot_id = format!("s_{i}");
            plan.shots.push(Shot {
                id: shot_id.clone(),
                beat_id: beat_id.clone(),
                kind: "A-roll".to_string(),
                duration_ms: 5_000,
                source_clip_id: format!("clip_{i}"),
                evidence_refs: vec![editorial.evidence_refs[0].clone()],
            });
            plan.styles.push(Style {
                id: format!("st_{i}"),
                shot_id,
                style_direction_id: "sd_default".to_string(),
                evidence_refs: vec![editorial.evidence_refs[0].clone()],
            });
        }
        Ok(plan)
    }

    pub fn assert_bound(plan: &CreativePlan) -> Result<(), PlanningError> {
        for b in &plan.beats {
            if b.evidence_refs.is_empty() {
                return Err(PlanningError::UnboundBeat(b.id.clone()));
            }
        }
        for s in &plan.shots {
            if s.evidence_refs.is_empty() {
                return Err(PlanningError::UnboundShot(s.id.clone()));
            }
        }
        for st in &plan.styles {
            if st.evidence_refs.is_empty() {
                return Err(PlanningError::UnboundStyle(st.id.clone()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unbound_editorial() {
        let editorial = EditorialPlan {
            id: "ed_1".to_string(),
            version: "v2".to_string(),
            arc: vec!["hook".to_string()],
            evidence_refs: vec![],
        };
        let err = CreativePlanner::plan(&editorial).expect_err("err");
        assert!(matches!(err, PlanningError::UnboundPlan(_)));
    }

    #[test]
    fn plans_with_evidence() {
        let editorial = EditorialPlan {
            id: "ed_1".to_string(),
            version: "v2".to_string(),
            arc: vec!["hook".to_string(), "payoff".to_string()],
            evidence_refs: vec!["evidence:ev_1".to_string()],
        };
        let plan = CreativePlanner::plan(&editorial).expect("ok");
        CreativePlanner::assert_bound(&plan).expect("bound");
        assert_eq!(plan.beats.len(), 2);
        assert_eq!(plan.shots.len(), 2);
        assert_eq!(plan.styles.len(), 2);
    }
}
