//! Persistent job/stage records (CR-V2-B3-021).
//!
//! A [`JobRecord`] is the on-disk identity of a job. Each stage mutates
//! its [`StageRecord`] atomically: the transitions are
//! `Pending → Ready → Running → Succeeded | Failed | Cancelled`. The
//! runner ([`crate::runner`]) is the only writer. The store never
//! mutates stage output bytes; it only records the state and the
//! fingerprint of the verified output.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::dag::{JobId, StageId};

/// State of a single stage. The only legal transitions are encoded in
/// [`StageRecord::transition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageState {
    Pending,
    Ready,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

/// Bounded attempt history. The runner records the last
/// `MAX_ATTEMPTS_HISTORY` attempts so the durable record is small.
pub const MAX_ATTEMPTS_HISTORY: usize = 8;

/// Per-stage record. The state is the only mutable field; the rest is
/// populated once the stage is scheduled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageRecord {
    pub stage_id: StageId,
    pub state: StageState,
    pub fingerprint: Option<[u8; 32]>,
    pub attempts: Vec<AttemptRecord>,
    pub last_error: Option<String>,
}

/// A single attempt. The runner is responsible for retry classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptRecord {
    pub attempt: u32,
    pub outcome: AttemptOutcome,
    pub started_at_unix_ms: u64,
    pub finished_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptOutcome {
    Succeeded,
    Retryable,
    Permanent,
    Cancelled,
}

impl StageRecord {
    /// Build a pending record.
    pub fn pending(stage_id: impl Into<StageId>) -> Self {
        Self {
            stage_id: stage_id.into(),
            state: StageState::Pending,
            fingerprint: None,
            attempts: Vec::new(),
            last_error: None,
        }
    }

    /// Build a succeeded record with a verified fingerprint.
    pub fn succeeded(stage_id: impl Into<StageId>, fingerprint: [u8; 32]) -> Self {
        Self {
            stage_id: stage_id.into(),
            state: StageState::Succeeded,
            fingerprint: Some(fingerprint),
            attempts: Vec::new(),
            last_error: None,
        }
    }

    /// Transition the state. Returns `Err` if the transition is illegal.
    pub fn transition(&mut self, target: StageState) -> Result<(), StoreError> {
        let ok = matches!(
            (self.state, target),
            (StageState::Pending, StageState::Ready)
                | (StageState::Ready, StageState::Running)
                | (StageState::Running, StageState::Succeeded)
                | (StageState::Running, StageState::Failed)
                | (StageState::Running, StageState::Cancelled)
                | (StageState::Pending, StageState::Cancelled)
                | (StageState::Ready, StageState::Cancelled)
        );
        if ok {
            self.state = target;
            Ok(())
        } else {
            Err(StoreError::IllegalTransition {
                from: self.state,
                to: target,
            })
        }
    }

    /// Record an attempt. The history is bounded.
    pub fn record_attempt(&mut self, attempt: AttemptRecord) {
        self.attempts.push(attempt);
        if self.attempts.len() > MAX_ATTEMPTS_HISTORY {
            let drop = self.attempts.len() - MAX_ATTEMPTS_HISTORY;
            self.attempts.drain(0..drop);
        }
    }
}

/// A whole job, including every stage and the job-level fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobRecord {
    pub job_id: JobId,
    pub dag_fingerprint: [u8; 32],
    pub stages: BTreeMap<StageId, StageRecord>,
    pub created_at_unix_ms: u64,
}

impl JobRecord {
    /// Convenience: how many stages are still pending?
    pub fn pending_count(&self) -> usize {
        self.stages
            .values()
            .filter(|s| matches!(s.state, StageState::Pending | StageState::Ready))
            .count()
    }
}

/// Persistent store errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreError {
    IllegalTransition {
        from: StageState,
        to: StageState,
    },
    UnknownJob(JobId),
    UnknownStage(StageId),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::IllegalTransition { from, to } => {
                write!(f, "illegal transition {from:?} -> {to:?}")
            }
            StoreError::UnknownJob(id) => write!(f, "unknown job {id}"),
            StoreError::UnknownStage(id) => write!(f, "unknown stage {id}"),
        }
    }
}

impl std::error::Error for StoreError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_to_ready_is_legal() {
        let mut s = StageRecord::pending("a");
        s.transition(StageState::Ready).unwrap();
        assert_eq!(s.state, StageState::Ready);
    }

    #[test]
    fn ready_to_succeeded_is_illegal() {
        let mut s = StageRecord::pending("a");
        s.transition(StageState::Ready).unwrap();
        assert!(s.transition(StageState::Succeeded).is_err());
    }

    #[test]
    fn running_to_succeeded_is_legal() {
        let mut s = StageRecord::pending("a");
        s.transition(StageState::Ready).unwrap();
        s.transition(StageState::Running).unwrap();
        s.transition(StageState::Succeeded).unwrap();
        assert_eq!(s.state, StageState::Succeeded);
    }

    #[test]
    fn attempt_history_is_bounded() {
        let mut s = StageRecord::pending("a");
        for i in 0..(MAX_ATTEMPTS_HISTORY + 3) {
            s.record_attempt(AttemptRecord {
                attempt: i as u32,
                outcome: AttemptOutcome::Retryable,
                started_at_unix_ms: 0,
                finished_at_unix_ms: 0,
            });
        }
        assert!(s.attempts.len() <= MAX_ATTEMPTS_HISTORY);
    }
}
