//! Autonomous orchestration glue.
//!
//! This module owns the wire shape used by `video-project` to invoke the
//! job-plane autonomous coordinator. The actual evaluation lives in
//! `video_jobs::autonomous`; this file only adapts between project-level
//! state and the digest inputs.

use serde::{Deserialize, Serialize};

use video_jobs::autonomous::{
    evaluate_autonomous_digest, AutonomousBlockers, AutonomousCriticVerdict,
    AutonomousDeterministicQaVerdict, AutonomousDigest, AutonomousDigestStatus,
    AutonomousRunInputs, AutonomousRunOutcome,
};

/// Project-side view of an escalation the autonomous coordinator may downgrade.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectAutonomousEscalation {
    pub stage: String,
    pub reason: String,
    pub blocking: bool,
}

/// Inputs the project layer hands to the job-plane coordinator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectAutonomousInputs {
    pub format_key: String,
    pub autonomy_mode: String,
    pub critic_id: String,
    pub critic_passed: bool,
    pub qa_passed: bool,
    pub failed_stages: Vec<String>,
    pub escalations: Vec<ProjectAutonomousEscalation>,
    pub last_approved_final_present: bool,
}

impl ProjectAutonomousInputs {
    /// Resolve the effective autonomy mode from format/profile/pack set and
    /// current escalations. Returns the requested mode unless a blocking
    /// escalation forces a downgrade, in which case it returns `"reviewed"`.
    pub fn effective_mode(&self) -> String {
        let blocking = self
            .escalations
            .iter()
            .any(|e| e.blocking);
        if blocking {
            "reviewed".to_string()
        } else {
            self.autonomy_mode.clone()
        }
    }

    pub fn into_run_inputs(&self) -> AutonomousRunInputs {
        let blocker = self
            .escalations
            .iter()
            .find(|e| e.blocking)
            .cloned()
            .map(|b| AutonomousBlockers::new(b.stage, b.reason))
            .unwrap_or_else(AutonomousBlockers::empty);
        AutonomousRunInputs {
            autonomy_mode_requested: self.effective_mode(),
            critic_verdict: AutonomousCriticVerdict {
                critic_id: self.critic_id.clone(),
                passed: self.critic_passed,
            },
            deterministic_qa: AutonomousDeterministicQaVerdict {
                passed: self.qa_passed,
                failed_stages: self.failed_stages.clone(),
            },
            blockers: blocker,
        }
    }

    pub fn evaluate(&self) -> AutonomousRunOutcome {
        let inputs = self.into_run_inputs();
        evaluate_autonomous_digest(&inputs, self.last_approved_final_present)
    }
}

/// Convenience function: compute the digest directly from project-side state.
pub fn evaluate_project_autonomous(inputs: &ProjectAutonomousInputs) -> AutonomousDigest {
    inputs.evaluate().digest
}

/// Re-export for callers that do not want to know about `video-jobs`.
pub fn is_ready(digest: &AutonomousDigest) -> bool {
    digest.status == AutonomousDigestStatus::Ready
}

pub fn outcome_digest(outcome: &AutonomousRunOutcome) -> &AutonomousDigest {
    &outcome.digest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocking_escalation_forces_reviewed_mode() {
        let mut inputs = ProjectAutonomousInputs {
            format_key: "recorded_talking_head|tiktok|v1".to_string(),
            autonomy_mode: "autonomous".to_string(),
            critic_id: "qwen3-vl".to_string(),
            critic_passed: true,
            qa_passed: true,
            failed_stages: vec![],
            escalations: vec![ProjectAutonomousEscalation {
                stage: "scoring".to_string(),
                reason: "low confidence".to_string(),
                blocking: true,
            }],
            last_approved_final_present: true,
        };
        assert_eq!(inputs.effective_mode(), "reviewed");
        let digest = evaluate_project_autonomous(&inputs);
        assert_eq!(digest.status, AutonomousDigestStatus::Failed);
        assert!(inputs.last_approved_final_present);
    }
}
