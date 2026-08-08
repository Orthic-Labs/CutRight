//! CutRight v2 job-plane types and the autonomous path coordinator.
//!
//! This crate owns the job-DAG execution model. The autonomous coordinator
//! only chooses to advance a job when the supplied critic + deterministic QA
//! floors pass and there is no blocking escalation.

pub mod autonomous;
pub mod dag;
pub mod recovery;
pub mod runner;
pub mod store;

pub use autonomous::{
    evaluate_autonomous_digest, AutonomousBlockers, AutonomousCriticVerdict,
    AutonomousDeterministicQaVerdict, AutonomousDigest, AutonomousDigestStatus,
    AutonomousEscalation, AutonomousRunInputs, AutonomousRunOutcome,
};
pub use dag::{DagError, JobDag, ResourceBudget, StageSpec};
pub use recovery::{classify_restart, recover_in_place, RecoveryReport, RestartClassification};
pub use runner::{
    job_record_from_dag, run, run_persisted, run_with_callback, CancellationHandle,
    CancellationToken, RunnerError, RunnerOutcome, StageCallbackError, StageContext, StageOutput,
};
pub use store::{
    AttemptOutcome, AttemptRecord, CheckpointRecord, InputRequired, JobEvent, JobRecord,
    ProjectJobStore, StageReceipt, StageRecord, StageState, StoreError, TerminalLoser,
    TerminalRace, TerminalWinner,
};
