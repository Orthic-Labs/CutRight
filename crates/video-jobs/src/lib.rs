//! CutRight v2 job-plane types and the autonomous path coordinator.
//!
//! This crate owns the job-DAG execution model. The autonomous coordinator
//! only chooses to advance a job when the supplied critic + deterministic QA
//! floors pass and there is no blocking escalation.

pub mod autonomous;
pub mod dag;
pub mod runner;
pub mod store;

pub use autonomous::{
    evaluate_autonomous_digest, AutonomousBlockers, AutonomousCriticVerdict,
    AutonomousDeterministicQaVerdict, AutonomousDigest, AutonomousDigestStatus,
    AutonomousEscalation, AutonomousRunInputs, AutonomousRunOutcome,
};
pub use dag::{DagError, JobDag, ResourceBudget, StageSpec};
pub use runner::{job_record_from_dag, run, CancellationToken, RunnerError, RunnerOutcome};
pub use store::{AttemptOutcome, AttemptRecord, JobRecord, StageRecord, StageState, StoreError};
