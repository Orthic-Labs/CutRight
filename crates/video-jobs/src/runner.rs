//! Deterministic job execution with cooperative cancellation and receipts.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::dag::{DagError, JobDag, StageId};
use crate::recovery::recover_in_place;
use crate::store::{
    AttemptOutcome, AttemptRecord, JobRecord, ProjectJobStore, StageReceipt, StageRecord,
    StageState, StoreError, TerminalLoser, TerminalRace, TerminalWinner,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerOutcome {
    Completed,
    Cancelled,
    InputRequired { stage_id: StageId },
    FailedPermanent { stage_id: StageId },
}

#[derive(Debug, Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
    pub fn handle(&self) -> CancellationHandle {
        CancellationHandle(self.clone())
    }
}

#[derive(Debug, Clone)]
pub struct CancellationHandle(CancellationToken);

impl CancellationHandle {
    pub fn cancel(&self) {
        self.0.cancel();
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.is_cancelled()
    }
}

#[derive(Debug, Clone)]
pub struct StageContext {
    pub job_id: String,
    pub stage_id: StageId,
    pub attempt: u32,
    pub stage_fingerprint: [u8; 32],
    pub checkpoint: Option<crate::store::CheckpointRecord>,
    pub cancellation: CancellationToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageOutput {
    pub fingerprint: [u8; 32],
    pub checkpoint: Option<crate::store::CheckpointRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageCallbackError {
    Retryable(String),
    Permanent(String),
    InputRequired(String),
    Cancelled(String),
}

pub type StageCallback = dyn Fn(&StageContext) -> Result<StageOutput, StageCallbackError>;

/// Existing callers retain a deterministic, receipt-producing entry point.
pub fn run(
    dag: &JobDag,
    job: &mut JobRecord,
    token: &CancellationToken,
) -> Result<RunnerOutcome, RunnerError> {
    run_with_callback(dag, job, token, &|context| {
        Ok(StageOutput {
            fingerprint: context.stage_fingerprint,
            checkpoint: None,
        })
    })
}

/// Run stages in deterministic topological order. Every terminal transition
/// is written to a receipt before success can be observed by a caller.
pub fn run_with_callback<F>(
    dag: &JobDag,
    job: &mut JobRecord,
    token: &CancellationToken,
    callback: &F,
) -> Result<RunnerOutcome, RunnerError>
where
    F: Fn(&StageContext) -> Result<StageOutput, StageCallbackError> + ?Sized,
{
    run_with_hooks(dag, job, token, callback, &mut |_| Ok(()))
}

fn run_with_hooks<F, H>(
    dag: &JobDag,
    job: &mut JobRecord,
    token: &CancellationToken,
    callback: &F,
    hook: &mut H,
) -> Result<RunnerOutcome, RunnerError>
where
    F: Fn(&StageContext) -> Result<StageOutput, StageCallbackError> + ?Sized,
    H: FnMut(&mut JobRecord) -> Result<(), RunnerError>,
{
    let order = dag.topological_order().map_err(RunnerError::from)?;
    let empty_order = order.is_empty();
    let recovery = recover_in_place(job);
    if !recovery.resumed_stages.is_empty() || !recovery.invalidated_stages.is_empty() {
        hook(job)?;
    }
    if let Some(stage_id) = recovery.invalidated_stages.first() {
        return Ok(RunnerOutcome::FailedPermanent {
            stage_id: stage_id.clone(),
        });
    }
    for stage_id in order {
        if token.is_cancelled() {
            cancel_remaining(job, &stage_id, None)?;
            hook(job)?;
            return Ok(RunnerOutcome::Cancelled);
        }
        let spec = dag
            .stages
            .get(&stage_id)
            .ok_or_else(|| RunnerError::UnknownStage(stage_id.clone()))?;
        let state = job
            .stages
            .get(&stage_id)
            .ok_or_else(|| RunnerError::UnknownStage(stage_id.clone()))?
            .state;
        if state == StageState::Succeeded {
            if !job.has_receipt(&stage_id) {
                return Err(RunnerError::Store(StoreError::MissingReceipt(stage_id)));
            }
            continue;
        }
        if !matches!(state, StageState::Pending | StageState::Ready) {
            continue;
        }
        let record = job
            .stages
            .get_mut(&stage_id)
            .ok_or_else(|| RunnerError::UnknownStage(stage_id.clone()))?;
        if state == StageState::Pending {
            record.transition(StageState::Ready)?;
        }
        let attempt = record.attempts.last().map_or(0, |a| a.attempt + 1);
        record.transition(StageState::Running)?;
        record.fingerprint = Some(spec.fingerprint());
        job.push_event(
            "stage_started",
            Some(stage_id.clone()),
            format!("attempt {attempt}"),
        );
        hook(job)?;
        let started = logical_time(job);
        let context = StageContext {
            job_id: job.job_id.clone(),
            stage_id: stage_id.clone(),
            attempt,
            stage_fingerprint: spec.fingerprint(),
            checkpoint: job.checkpoints.get(&stage_id).cloned(),
            cancellation: token.clone(),
        };
        let callback_result = callback(&context);
        let cancellation_observed = token.is_cancelled();
        match callback_result {
            Ok(_output) if cancellation_observed => {
                finish_cancelled(
                    job,
                    &stage_id,
                    attempt,
                    started,
                    Some(TerminalRace {
                        winner: TerminalWinner::Cancellation,
                        loser: TerminalLoser::Completion,
                    }),
                )?;
                cancel_remaining(job, &stage_id, Some(attempt))?;
                hook(job)?;
                return Ok(RunnerOutcome::Cancelled);
            }
            Ok(output) => {
                if let Some(checkpoint) = output.checkpoint {
                    job.checkpoints.insert(stage_id.clone(), checkpoint);
                }
                let finished = logical_time(job);
                let record = job.stages.get_mut(&stage_id).expect("stage still exists");
                record.record_attempt(AttemptRecord {
                    attempt,
                    outcome: AttemptOutcome::Succeeded,
                    started_at_unix_ms: started,
                    finished_at_unix_ms: finished,
                });
                record.transition(StageState::Succeeded)?;
                job.receipts.push(StageReceipt {
                    stage_id: stage_id.clone(),
                    attempt,
                    terminal_state: StageState::Succeeded,
                    output_fingerprint: Some(output.fingerprint),
                    revision: job.revision,
                    race: None,
                });
                job.push_event(
                    "stage_succeeded",
                    Some(stage_id),
                    format!("attempt {attempt}"),
                );
                hook(job)?;
            }
            Err(StageCallbackError::Cancelled(_reason))
            | Err(StageCallbackError::InputRequired(_reason))
                if token.is_cancelled() =>
            {
                finish_cancelled(
                    job,
                    &stage_id,
                    attempt,
                    started,
                    Some(TerminalRace {
                        winner: TerminalWinner::Cancellation,
                        loser: TerminalLoser::Completion,
                    }),
                )?;
                cancel_remaining(job, &stage_id, Some(attempt))?;
                hook(job)?;
                return Ok(RunnerOutcome::Cancelled);
            }
            Err(StageCallbackError::Cancelled(_reason)) => {
                finish_cancelled(job, &stage_id, attempt, started, None)?;
                cancel_remaining(job, &stage_id, Some(attempt))?;
                hook(job)?;
                return Ok(RunnerOutcome::Cancelled);
            }
            Err(StageCallbackError::InputRequired(reason)) => {
                finish_attempt(
                    job,
                    &stage_id,
                    attempt,
                    AttemptOutcome::Permanent,
                    started,
                    StageState::Failed,
                    Some(reason.clone()),
                )?;
                job.input_required = Some(crate::store::InputRequired {
                    stage_id: stage_id.clone(),
                    reason,
                });
                hook(job)?;
                return Ok(RunnerOutcome::InputRequired { stage_id });
            }
            Err(StageCallbackError::Retryable(error)) if attempt < spec.max_attempts => {
                finish_attempt(
                    job,
                    &stage_id,
                    attempt,
                    AttemptOutcome::Retryable,
                    started,
                    StageState::Ready,
                    Some(error),
                )?;
                hook(job)?;
            }
            Err(StageCallbackError::Retryable(error))
            | Err(StageCallbackError::Permanent(error)) => {
                finish_attempt(
                    job,
                    &stage_id,
                    attempt,
                    AttemptOutcome::Permanent,
                    started,
                    StageState::Failed,
                    Some(error),
                )?;
                cancel_remaining(job, &stage_id, Some(attempt))?;
                hook(job)?;
                return Ok(RunnerOutcome::FailedPermanent { stage_id });
            }
        }
    }
    if empty_order {
        hook(job)?;
    }
    if let Some(stage_id) = job
        .stages
        .values()
        .find(|stage| stage.state == StageState::Failed)
        .map(|stage| stage.stage_id.clone())
    {
        return Ok(RunnerOutcome::FailedPermanent { stage_id });
    }
    Ok(RunnerOutcome::Completed)
}

/// Persist every durable boundary, including `Running` before callback entry.
pub fn run_persisted<F>(
    store: &mut ProjectJobStore,
    dag: &JobDag,
    job_id: &str,
    token: &CancellationToken,
    callback: &F,
) -> Result<RunnerOutcome, RunnerError>
where
    F: Fn(&StageContext) -> Result<StageOutput, StageCallbackError> + ?Sized,
{
    let mut job = store.load(job_id).map_err(RunnerError::Store)?;
    let mut expected = job.revision;
    let mut persist = |snapshot: &mut JobRecord| {
        snapshot.revision = expected + 1;
        let saved = store
            .compare_and_swap(job_id, expected, snapshot.clone())
            .map_err(RunnerError::Store)?;
        expected = saved.revision;
        Ok(())
    };
    run_with_hooks(dag, &mut job, token, callback, &mut persist)
}

fn logical_time(job: &JobRecord) -> u64 {
    job.events.len() as u64 + 1
}

fn finish_attempt(
    job: &mut JobRecord,
    stage_id: &StageId,
    attempt: u32,
    outcome: AttemptOutcome,
    started: u64,
    state: StageState,
    error: Option<String>,
) -> Result<(), RunnerError> {
    let finished = logical_time(job);
    {
        let record = job.stages.get_mut(stage_id).expect("stage still exists");
        record.record_attempt(AttemptRecord {
            attempt,
            outcome,
            started_at_unix_ms: started,
            finished_at_unix_ms: finished,
        });
        record.last_error = error;
        record.transition(state)?;
    }
    if state != StageState::Ready {
        job.receipts.push(StageReceipt {
            stage_id: stage_id.clone(),
            attempt,
            terminal_state: state,
            output_fingerprint: None,
            revision: job.revision,
            race: None,
        });
    }
    job.push_event(
        "stage_terminal",
        Some(stage_id.clone()),
        format!("{state:?}"),
    );
    Ok(())
}

fn finish_cancelled(
    job: &mut JobRecord,
    stage_id: &StageId,
    attempt: u32,
    started: u64,
    race: Option<TerminalRace>,
) -> Result<(), RunnerError> {
    finish_attempt(
        job,
        stage_id,
        attempt,
        AttemptOutcome::Cancelled,
        started,
        StageState::Cancelled,
        None,
    )?;
    job.receipts
        .last_mut()
        .expect("finish_attempt appends a receipt")
        .race = race;
    Ok(())
}

fn cancel_remaining(
    job: &mut JobRecord,
    stopped_at: &StageId,
    attempt: Option<u32>,
) -> Result<(), RunnerError> {
    let ids: Vec<StageId> = job.stages.keys().cloned().collect();
    for id in ids {
        if id == *stopped_at {
            continue;
        }
        let state = job.stages.get(&id).map(|r| r.state);
        if matches!(
            state,
            Some(StageState::Pending | StageState::Ready | StageState::Running)
        ) {
            let next_attempt = attempt.unwrap_or(0);
            finish_cancelled(job, &id, next_attempt, logical_time(job), None)?;
        }
    }
    Ok(())
}

pub fn ready_stages(dag: &JobDag, job: &JobRecord) -> Vec<StageId> {
    dag.ready_stages(&job.stages)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunnerError {
    Dag(DagError),
    Store(StoreError),
    UnknownStage(StageId),
}

impl From<DagError> for RunnerError {
    fn from(e: DagError) -> Self {
        Self::Dag(e)
    }
}
impl From<StoreError> for RunnerError {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}
impl std::fmt::Display for RunnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dag(e) => write!(f, "dag: {e}"),
            Self::Store(e) => write!(f, "store: {e}"),
            Self::UnknownStage(id) => write!(f, "unknown stage {id}"),
        }
    }
}
impl std::error::Error for RunnerError {}

pub fn job_record_from_dag(dag: &JobDag, job_id: &str) -> JobRecord {
    let mut stages = BTreeMap::new();
    for id in dag.stages.keys() {
        stages.insert(id.clone(), StageRecord::pending(id.clone()));
    }
    JobRecord {
        job_id: job_id.to_string(),
        dag_fingerprint: dag.fingerprint(),
        stages,
        created_at_unix_ms: 0,
        revision: 0,
        events: vec![],
        checkpoints: BTreeMap::new(),
        receipts: vec![],
        input_required: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::{ResourceBudget, StageSpec};

    fn build_dag() -> JobDag {
        JobDag::new(
            "j".into(),
            "linear".into(),
            vec![
                StageSpec {
                    id: "a".into(),
                    kind: "noop".into(),
                    dependencies: vec![],
                    parameters: serde_json::json!({}),
                    resources: ResourceBudget::default(),
                    max_attempts: 0,
                },
                StageSpec {
                    id: "b".into(),
                    kind: "noop".into(),
                    dependencies: vec!["a".into()],
                    parameters: serde_json::json!({}),
                    resources: ResourceBudget::default(),
                    max_attempts: 0,
                },
            ],
        )
        .unwrap()
    }

    #[test]
    fn run_completes_only_with_receipts() {
        let dag = build_dag();
        let mut job = job_record_from_dag(&dag, "j1");
        assert_eq!(
            run(&dag, &mut job, &CancellationToken::new()).unwrap(),
            RunnerOutcome::Completed
        );
        assert_eq!(job.receipts.len(), 2);
        assert!(job
            .stages
            .values()
            .all(|s| s.state == StageState::Succeeded));
    }

    #[test]
    fn callback_can_cancel_racing_completion() {
        let dag = build_dag();
        let mut job = job_record_from_dag(&dag, "j2");
        let token = CancellationToken::new();
        let handle = token.handle();
        let callback = |context: &StageContext| {
            if context.stage_id == "a" {
                handle.cancel();
            }
            Ok(StageOutput {
                fingerprint: context.stage_fingerprint,
                checkpoint: None,
            })
        };
        assert_eq!(
            run_with_callback(&dag, &mut job, &token, &callback).unwrap(),
            RunnerOutcome::Cancelled
        );
        assert_eq!(job.receipts[0].terminal_state, StageState::Cancelled);
        assert!(job.receipts[0].race.is_some());
    }

    #[test]
    fn restart_does_not_turn_running_into_success() {
        let dag = JobDag::new(
            "j".into(),
            "restart".into(),
            vec![StageSpec {
                id: "a".into(),
                kind: "noop".into(),
                dependencies: vec![],
                parameters: serde_json::json!({}),
                resources: ResourceBudget::default(),
                max_attempts: 0,
            }],
        )
        .unwrap();
        let mut job = job_record_from_dag(&dag, "j3");
        job.stages
            .get_mut("a")
            .unwrap()
            .transition(StageState::Ready)
            .unwrap();
        job.stages
            .get_mut("a")
            .unwrap()
            .transition(StageState::Running)
            .unwrap();
        assert_eq!(
            run(&dag, &mut job, &CancellationToken::new()).unwrap(),
            RunnerOutcome::Completed
        );
        assert!(job.has_receipt(&"a".into()));
    }

    #[test]
    fn retryable_callback_returns_stage_to_ready_before_retry() {
        let dag = JobDag::new(
            "j".into(),
            "retry".into(),
            vec![StageSpec {
                id: "a".into(),
                kind: "noop".into(),
                dependencies: vec![],
                parameters: serde_json::json!({}),
                resources: ResourceBudget::default(),
                max_attempts: 1,
            }],
        )
        .unwrap();
        let mut job = job_record_from_dag(&dag, "j4");
        let calls = std::cell::Cell::new(0);
        let callback = |context: &StageContext| {
            let call = calls.get();
            calls.set(call + 1);
            if call == 0 {
                Err(StageCallbackError::Retryable("try again".into()))
            } else {
                Ok(StageOutput {
                    fingerprint: context.stage_fingerprint,
                    checkpoint: None,
                })
            }
        };
        assert_eq!(
            run_with_callback(&dag, &mut job, &CancellationToken::new(), &callback).unwrap(),
            RunnerOutcome::Completed
        );
        assert_eq!(calls.get(), 2);
        assert_eq!(job.stages["a"].state, StageState::Succeeded);
        assert_eq!(job.stages["a"].attempts.len(), 2);
        assert_eq!(job.receipts.len(), 1);
    }
}
