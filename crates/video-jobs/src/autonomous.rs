//! Autonomous orchestration digest evaluation.
//!
//! The job plane exposes a single function, [`evaluate_autonomous_digest`],
//! that decides whether the underlying editor/render/QA pipeline may run end
//! to end without intermediate approval. The decision is purely a function of
//! the supplied inputs:
//!
//! * [`AutonomousCriticVerdict`] — independent critic must pass.
//! * [`AutonomousDeterministicQaVerdict`] — every deterministic floor must pass.
//! * [`AutonomousBlockers`] — any blocking escalation forces a downgrade.
//!
//! The function never publishes, deletes alternatives or overwrites the last
//! approved final. It only reports whether the digest is `ready`,
//! `needs_review` or `failed`. The Studio runner is responsible for any
//! mutation.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousDigestStatus {
    Ready,
    NeedsReview,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousCriticVerdict {
    pub critic_id: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousDeterministicQaVerdict {
    pub passed: bool,
    pub failed_stages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousEscalation {
    pub stage: String,
    pub reason: String,
    pub blocking: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousBlockers {
    pub blocker_stage: Option<String>,
    pub blocker_reason: Option<String>,
}

impl AutonomousBlockers {
    pub fn empty() -> Self {
        Self {
            blocker_stage: None,
            blocker_reason: None,
        }
    }
    pub fn new(stage: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            blocker_stage: Some(stage.into()),
            blocker_reason: Some(reason.into()),
        }
    }
    pub fn is_blocking(&self) -> bool {
        self.blocker_stage.is_some() && self.blocker_reason.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousRunInputs {
    pub autonomy_mode_requested: String,
    pub critic_verdict: AutonomousCriticVerdict,
    pub deterministic_qa: AutonomousDeterministicQaVerdict,
    pub blockers: AutonomousBlockers,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousDigest {
    pub status: AutonomousDigestStatus,
    pub summary: String,
    pub ready_for_final_approval: bool,
    pub reviewer_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomousRunOutcome {
    pub digest: AutonomousDigest,
    pub finals_kept: bool,
    pub alternatives_kept: bool,
}

/// Compute the digest from the supplied inputs and produce the run outcome.
///
/// Inputs:
/// * `inputs` — the consolidated autonomous run inputs.
/// * `last_approved_final_present` — whether a previous approved final exists.
///
/// The function never mutates `inputs`. It only reports.
///
/// Acceptance rules (from B7-011):
/// * `Ready` requires `autonomy_mode_requested == "autonomous"`, critic pass,
///   every deterministic stage pass and no blocker.
/// * `NeedsReview` is returned when critic/QA both pass but there is a
///   non-blocking reviewer signal (or autonomy is requested at a lighter mode).
/// * `Failed` is returned when critic or QA fail, or when there is a blocker.
pub fn evaluate_autonomous_digest(
    inputs: &AutonomousRunInputs,
    last_approved_final_present: bool,
) -> AutonomousRunOutcome {
    let mut status = AutonomousDigestStatus::Ready;
    let mut summary = String::from("autonomous run completed cleanly");
    let mut reviewer_required = false;

    if inputs.autonomy_mode_requested != "autonomous" {
        status = AutonomousDigestStatus::NeedsReview;
        summary = format!(
            "mode requested is {}; full autonomous requires explicit autonomous mode",
            inputs.autonomy_mode_requested
        );
        reviewer_required = true;
    }

    if inputs.blockers.is_blocking() {
        status = AutonomousDigestStatus::Failed;
        summary = format!(
            "blocking escalation at stage {} ({})",
            inputs.blockers.blocker_stage.as_deref().unwrap_or("?"),
            inputs.blockers.blocker_reason.as_deref().unwrap_or("?")
        );
    } else if !inputs.critic_verdict.passed {
        status = AutonomousDigestStatus::Failed;
        summary = format!("critic {} disagreed", inputs.critic_verdict.critic_id);
    } else if !inputs.deterministic_qa.passed {
        status = AutonomousDigestStatus::Failed;
        let stage = inputs
            .deterministic_qa
            .failed_stages
            .first()
            .cloned()
            .unwrap_or_else(|| "<unknown>".to_string());
        summary = format!("deterministic QA failed at stage {}", stage);
    }

    let ready_for_final_approval = status == AutonomousDigestStatus::Ready;

    AutonomousRunOutcome {
        digest: AutonomousDigest {
            status,
            summary,
            ready_for_final_approval,
            reviewer_required,
        },
        finals_kept: last_approved_final_present,
        alternatives_kept: true,
    }
}
