//! Restart classification for durable job records.

use serde::{Deserialize, Serialize};

use crate::store::{JobRecord, StageState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartClassification {
    Fresh,
    ResumePending,
    ResumeCheckpoint,
    InputRequired,
    Terminal,
    RepairedUnreceiptedSuccess,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryReport {
    pub classification: RestartClassification,
    pub resumed_stages: Vec<String>,
    pub invalidated_stages: Vec<String>,
}

/// Classify state before any stage callback is invoked.
pub fn classify_restart(job: &JobRecord) -> RestartClassification {
    if job.input_required.is_some() {
        return RestartClassification::InputRequired;
    }
    if job
        .stages
        .values()
        .any(|stage| stage.state == StageState::Succeeded && !job.has_receipt(&stage.stage_id))
    {
        return RestartClassification::RepairedUnreceiptedSuccess;
    }
    if job
        .stages
        .values()
        .any(|stage| stage.state == StageState::Running)
    {
        return RestartClassification::ResumeCheckpoint;
    }
    if job
        .stages
        .values()
        .any(|stage| matches!(stage.state, StageState::Pending | StageState::Ready))
    {
        return if job.checkpoints.is_empty() {
            RestartClassification::ResumePending
        } else {
            RestartClassification::ResumeCheckpoint
        };
    }
    if job.stages.values().all(|stage| {
        matches!(
            stage.state,
            StageState::Succeeded | StageState::Failed | StageState::Cancelled
        )
    }) {
        RestartClassification::Terminal
    } else {
        RestartClassification::Fresh
    }
}

/// Repair only process-owned transient state. A killed `Running` stage is
/// returned to `Ready`; no receipt is invented and its callback must rerun.
pub fn recover_in_place(job: &mut JobRecord) -> RecoveryReport {
    let initial = classify_restart(job);
    let mut resumed_stages = Vec::new();
    let mut invalidated_stages = Vec::new();
    let ids: Vec<String> = job.stages.keys().cloned().collect();
    for id in ids {
        let state = job.stages.get(&id).map(|stage| stage.state);
        if state == Some(StageState::Running) {
            let stage = job.stages.get_mut(&id).expect("stage still exists");
            stage.state = StageState::Ready;
            stage.last_error = Some("restart recovered an incomplete attempt".into());
            resumed_stages.push(id.clone());
            job.push_event(
                "restart_recovered",
                Some(id),
                "running stage returned to ready",
            );
        } else if state == Some(StageState::Succeeded) && !job.has_receipt(&id) {
            let stage = job.stages.get_mut(&id).expect("stage still exists");
            stage.state = StageState::Failed;
            stage.last_error = Some("success had no receipt; outcome invalidated".into());
            invalidated_stages.push(id.clone());
            job.push_event(
                "unreceipted_success_invalidated",
                Some(id),
                "manual review required",
            );
        }
    }
    let classification = if !invalidated_stages.is_empty() {
        RestartClassification::RepairedUnreceiptedSuccess
    } else if !resumed_stages.is_empty() {
        RestartClassification::ResumeCheckpoint
    } else {
        initial
    };
    RecoveryReport {
        classification,
        resumed_stages,
        invalidated_stages,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::job_record_from_dag;
    use crate::{JobDag, ResourceBudget, StageSpec};

    #[test]
    fn running_stage_is_recoverable_without_invented_success() {
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
        let mut job = job_record_from_dag(&dag, "j");
        let stage = job.stages.get_mut("a").unwrap();
        stage.transition(StageState::Ready).unwrap();
        stage.transition(StageState::Running).unwrap();

        let report = recover_in_place(&mut job);
        assert_eq!(
            report.classification,
            RestartClassification::ResumeCheckpoint
        );
        assert_eq!(report.resumed_stages, vec!["a"]);
        assert_eq!(job.stages["a"].state, StageState::Ready);
        assert!(job.receipts.is_empty());
    }
}
