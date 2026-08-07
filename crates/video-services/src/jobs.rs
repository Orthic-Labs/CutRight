//! Job service (CR-V2-B3-022).
//!
//! The job service wraps the [`video_jobs`] DAG and runner. Studio /
//! CLI submit a [`JobSubmission`]; the service returns a
//! [`JobHandle`] that the caller can poll but cannot mutate.

use serde::{Deserialize, Serialize};

use crate::ServiceConfig;

pub const JOB_SERVICE_CAPABILITY: &str = "cap.render.dispatch";

#[derive(Debug, Clone)]
pub struct JobService {
    config: ServiceConfig,
}

impl JobService {
    pub fn new(config: ServiceConfig) -> Self {
        Self { config }
    }

    pub fn capability_id(&self) -> &'static str {
        JOB_SERVICE_CAPABILITY
    }

    /// Submit a new job. The submission is the only mutable input the
    /// service accepts; the returned handle is opaque.
    pub fn submit(&self, submission: JobSubmission) -> Result<JobHandle, JobError> {
        if submission.job_id.is_empty() {
            return Err(JobError::EmptyJobId);
        }
        Ok(JobHandle(format!("job:handle:{}", submission.job_id)))
    }

    pub fn config(&self) -> &ServiceConfig {
        &self.config
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobSubmission {
    pub job_id: String,
    pub dag_fingerprint: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobHandle(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobError {
    EmptyJobId,
}

impl std::fmt::Display for JobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobError::EmptyJobId => write!(f, "empty job id"),
        }
    }
}

impl std::error::Error for JobError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> ServiceConfig {
        ServiceConfig {
            project_root: "/tmp/proj".into(),
            pack_root: "/tmp/packs".into(),
            evidence_root: "/tmp/evidence".into(),
            lease_root: "/tmp/lease".into(),
        }
    }

    #[test]
    fn submit_returns_handle() {
        let svc = JobService::new(cfg());
        let h = svc
            .submit(JobSubmission {
                job_id: "j1".into(),
                dag_fingerprint: [0u8; 32],
            })
            .unwrap();
        assert_eq!(h.0, "job:handle:j1");
    }

    #[test]
    fn empty_job_id_is_rejected() {
        let svc = JobService::new(cfg());
        let r = svc.submit(JobSubmission {
            job_id: "".into(),
            dag_fingerprint: [0u8; 32],
        });
        assert!(r.is_err());
    }
}
