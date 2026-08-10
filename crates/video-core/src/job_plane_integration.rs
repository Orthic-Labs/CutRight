//!
//! Generated and procedural assembly integration with the job plane
//! (CR-V2-B5-024).
//!
//! The job plane schedules creative work as a DAG of `CreativeJob`s.
//! This module defines the **integration surface** between the job
//! plane and the (generated + procedural) assembly pipeline:
//!
//! - `CreativeJob` — a single job in the DAG
//! - `CreativeJobKind` — what kind of work the job runs
//! - `JobPlaneIntegration` — registers a job and gets back a `JobHandle`
//!
//! The lane never reaches into the job plane crate; it only consumes
//! the typed handle and the prepared artefact by `id`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum JobPlaneIntegrationError {
    #[error("job must declare a creative_plan_id: id={0}")]
    MissingCreativePlan(String),
    #[error("job must declare a finish_plan_id: id={0}")]
    MissingFinishPlan(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreativeJobKind {
    Brief,
    Brand,
    Style,
    BakeOff,
    Shot,
    Roll,
    Asset,
    Package,
    Render,
    Critique,
    Publish,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativeJob {
    pub id: String,
    pub kind: CreativeJobKind,
    pub creative_plan_id: String,
    pub finish_plan_id: String,
    pub deps: Vec<String>,
    pub budget_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobHandle {
    pub job_id: String,
    pub lane: String,
    pub prepared_artefact_id: String,
    pub metrics: BTreeMap<String, f64>,
}

pub struct JobPlaneIntegration;

impl JobPlaneIntegration {
    pub fn submit(job: &CreativeJob) -> Result<JobHandle, JobPlaneIntegrationError> {
        if job.creative_plan_id.is_empty() {
            return Err(JobPlaneIntegrationError::MissingCreativePlan(
                job.id.clone(),
            ));
        }
        if job.finish_plan_id.is_empty() {
            return Err(JobPlaneIntegrationError::MissingFinishPlan(job.id.clone()));
        }
        let lane = match job.kind {
            CreativeJobKind::Brief => "planning",
            CreativeJobKind::Brand => "brand",
            CreativeJobKind::Style => "brand",
            CreativeJobKind::BakeOff => "planning",
            CreativeJobKind::Shot => "planning",
            CreativeJobKind::Roll => "planning",
            CreativeJobKind::Asset => "asset-validation",
            CreativeJobKind::Package => "writing",
            CreativeJobKind::Render => "native-renderer",
            CreativeJobKind::Critique => "creative-critic",
            CreativeJobKind::Publish => "job-plane",
        };
        Ok(JobHandle {
            job_id: job.id.clone(),
            lane: lane.to_string(),
            prepared_artefact_id: format!("artefact_{}", job.id),
            metrics: BTreeMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> CreativeJob {
        CreativeJob {
            id: "job_1".to_string(),
            kind: CreativeJobKind::Render,
            creative_plan_id: "cp_1".to_string(),
            finish_plan_id: "fpl_1".to_string(),
            deps: vec![],
            budget_ms: 60_000,
        }
    }

    #[test]
    fn submits_valid_job() {
        let h = JobPlaneIntegration::submit(&sample()).expect("ok");
        assert_eq!(h.lane, "native-renderer");
        assert_eq!(h.job_id, "job_1");
    }

    #[test]
    fn rejects_missing_finish_plan() {
        let mut j = sample();
        j.finish_plan_id = "".to_string();
        let err = JobPlaneIntegration::submit(&j).expect_err("err");
        assert!(matches!(
            err,
            JobPlaneIntegrationError::MissingFinishPlan(_)
        ));
    }
}
