// EditorialEngine façade (Book 4 lane C, B4-022).
//
// Applies lanes A, B, C in fixed order:
//   1. retrieve evidence
//   2. deterministic candidates / features
//   3. director proposal (model output)
//   4. schema/semantic validation
//   5. critic
//   6. bounded revision
//   7. final plan
//
// Never writes a project file directly; returns canonical artefacts.

use serde::{Deserialize, Serialize};

use crate::narrative::confidence::{
    estimate, Ambiguity, ConfidenceEstimate, ConfidenceInputs, ReviewMode,
};
use crate::narrative::critic::{run_critic, CriticOutcome, CriticVerdict, ProposalView};
use crate::narrative::order::OrderPlan;
use crate::narrative::provider::{
    validate_proposal, DirectorError, EditorialProposal, EditorialRequest,
};
use crate::narrative::reflection::{reflect, ReflectionReport};
use crate::narrative::repair::{attempt_repair, RepairAttempt, RepairOutcome};
use crate::narrative::shorts::{ShortCandidate, ShortInputs};
use crate::narrative::truthfulness::{evaluate_reorder, OrderLog, Reorder};
use crate::plan::{EditorialPlan, EditorialPlanResult, PlanError};

#[derive(Debug, Clone, PartialEq)]
pub struct EditorialEngineRequest {
    pub request_id: String,
    pub review_mode: ReviewMode,
    pub director_request: EditorialRequest,
    pub director_proposal: Option<EditorialProposal>,
    pub beat_inputs: Vec<ShortInputs<'static>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EditorialEngineOutcome {
    pub plan: Option<EditorialPlan>,
    pub error: Option<PlanError>,
    pub retrieval_receipt: String,
    pub version: u32,
}

pub struct EditorialEngine {
    pub version: u32,
}

impl EditorialEngine {
    pub fn new() -> Self {
        Self { version: 1 }
    }

    /// Run the lane A/B/C sequence and return a plan. Never mutates
    /// any project file.
    pub fn plan(&self, req: &EditorialEngineRequest) -> Result<EditorialPlanResult, PlanError> {
        // 1. Retrieve evidence (caller supplies refs).
        if req.director_request.evidence_refs.is_empty() {
            return Err(PlanError::InvalidInputs);
        }
        let receipt = format!(
            "retrieval:{}:{}:v{}",
            req.request_id,
            req.director_request.evidence_refs.len(),
            self.version
        );

        // 2. Deterministic candidates/features -> Shorts
        let shorts: Vec<ShortCandidate> = req
            .beat_inputs
            .iter()
            .enumerate()
            .map(|(i, inputs)| {
                let id = format!("short-{}", i);
                crate::narrative::shorts::build_candidate(&id, &id, "recorded-hook", inputs.clone())
            })
            .collect();

        // 3. Director proposal
        let proposal = match &req.director_proposal {
            Some(p) => p,
            None => {
                return Ok(EditorialPlanResult {
                    plan: None,
                    error: Some(PlanError::NoProposal),
                    retrieval_receipt: receipt,
                });
            }
        };

        // 4. Schema/semantic validation.
        if let Err(e) = validate_proposal(&req.director_request, proposal) {
            match e {
                DirectorError::InvalidRequest(_) | DirectorError::SchemaInvalid(_) => {
                    return Ok(EditorialPlanResult {
                        plan: None,
                        error: Some(PlanError::InvalidInputs),
                        retrieval_receipt: receipt,
                    });
                }
                DirectorError::UnknownCandidate(_) => {
                    return Ok(EditorialPlanResult {
                        plan: None,
                        error: Some(PlanError::InvalidInputs),
                        retrieval_receipt: receipt,
                    });
                }
            }
        }

        // Derive OrderLogs from reorders vs. selected-by-request order.
        // For the deterministic lane, the source order matches the
        // director's request candidate order; only the proposal's
        // `order` is the new arrangement. We emit a log for each
        // reindexing where the proposal order differs from the
        // request order (cold-open allowed).
        let request_order: Vec<String> = req
            .director_request
            .candidates
            .iter()
            .map(|c| c.take_id.clone())
            .collect();
        let mut order_logs: Vec<OrderLog> = Vec::new();
        let proposal_order: Vec<String> = proposal.order.clone();
        for (new_idx, take_id) in proposal_order.iter().enumerate() {
            let from_idx = request_order.iter().position(|r| r == take_id);
            if let Some(from) = from_idx {
                if from != new_idx {
                    let log = evaluate_reorder(&Reorder {
                        from_index: from,
                        to_index: new_idx,
                        claim: crate::narrative::truthfulness::Claim {
                            claim_id: format!("claim-{}", take_id),
                            depends_on: vec![],
                        },
                        introduces_false_sequence: false,
                        breaks_claim_dependency: false,
                    });
                    order_logs.push(log);
                }
            }
        }

        // 5. Critic
        let critic_view = ProposalView {
            proposal_id: proposal.proposal_id.clone(),
            claim_count: proposal.selected.len(),
            evidence_count: req.director_request.evidence_refs.len(),
            has_unknown_candidates: false,
            samples: vec![],
        };
        let mut revision_used = false;
        let mut critic: CriticOutcome = run_critic(&critic_view, revision_used);

        // 6. Bounded revision: at most one revision; second disagreement escalates.
        if matches!(critic.verdict, CriticVerdict::RequestRevision) {
            revision_used = true;
            critic = run_critic(&critic_view, revision_used);
        }

        // Build confidence from inputs.
        let conf_inputs = ConfidenceInputs {
            take_margin: 0.5,
            evidence_agreement: 0.5,
            boundary_confidence: 0.5,
            schema_valid: true,
            critic_blocks: matches!(critic.verdict, CriticVerdict::Block),
            order_logs: order_logs.clone(),
            missing_evidence: false,
        };
        let confidence: ConfidenceEstimate = estimate(req.review_mode, &conf_inputs);

        if matches!(critic.verdict, CriticVerdict::Block) {
            return Ok(EditorialPlanResult {
                plan: None,
                error: Some(PlanError::CriticBlocked),
                retrieval_receipt: receipt,
            });
        }
        if confidence
            .escalations
            .contains(&Ambiguity::TruthfulnessRisk)
        {
            return Ok(EditorialPlanResult {
                plan: None,
                error: Some(PlanError::TruthfulnessRisk),
                retrieval_receipt: receipt,
            });
        }

        // 6b. Reflection + bounded repair (uses critic and confidence).
        let reflection: Option<ReflectionReport> =
            reflect(&proposal.proposal_id, &confidence, &critic);
        let mut repair_attempt: Option<RepairAttempt> = None;
        if let Some(r) = &reflection {
            let attempt = attempt_repair(r, revision_used, &confidence, &critic);
            if matches!(attempt.outcome, RepairOutcome::EscalatedSecondRepair) {
                return Ok(EditorialPlanResult {
                    plan: None,
                    error: Some(PlanError::RepairEscalated),
                    retrieval_receipt: receipt,
                });
            }
            repair_attempt = Some(attempt);
        }

        // 7. Final plan
        let order_plan = OrderPlan {
            plan_id: format!("order-{}", proposal.proposal_id),
            order: proposal.order.clone(),
            logs: order_logs,
            has_truthfulness_risk: confidence
                .escalations
                .contains(&Ambiguity::TruthfulnessRisk),
        };
        let plan = EditorialPlan {
            plan_id: format!("plan-{}", proposal.proposal_id),
            proposal_id: proposal.proposal_id.clone(),
            review_mode: confidence.effective_mode,
            order: order_plan,
            shorts,
            confidence,
            repair: repair_attempt,
            evidence_refs: req.director_request.evidence_refs.clone(),
            benchmark_refs: vec![],
            version: self.version,
        };
        Ok(EditorialPlanResult {
            plan: Some(plan),
            error: None,
            retrieval_receipt: receipt,
        })
    }
}

impl Default for EditorialEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narrative::provider::CandidateRef;

    fn req() -> EditorialRequest {
        EditorialRequest {
            user_brief: "demo".into(),
            format: "shorts".into(),
            duration_ms_target: 30_000,
            candidates: vec![
                CandidateRef {
                    beat_id: "b1".into(),
                    take_id: "t1".into(),
                    role: "hook".into(),
                },
                CandidateRef {
                    beat_id: "b2".into(),
                    take_id: "t2".into(),
                    role: "payoff".into(),
                },
            ],
            evidence_refs: vec!["e1".into()],
            model_revision: "v1".into(),
        }
    }

    fn proposal_ok() -> EditorialProposal {
        EditorialProposal {
            proposal_id: "p1".into(),
            selected: vec![
                CandidateRef {
                    beat_id: "b1".into(),
                    take_id: "t1".into(),
                    role: "hook".into(),
                },
                CandidateRef {
                    beat_id: "b2".into(),
                    take_id: "t2".into(),
                    role: "payoff".into(),
                },
            ],
            order: vec!["t1".into(), "t2".into()],
            arc_id: "shorts.hook-payoff".into(),
            rationale: vec!["hook -> payoff".into()],
            evidence_refs: vec!["e1".into()],
        }
    }

    fn engine_req() -> EditorialEngineRequest {
        EditorialEngineRequest {
            request_id: "req1".into(),
            review_mode: ReviewMode::Reviewed,
            director_request: req(),
            director_proposal: Some(proposal_ok()),
            beat_inputs: vec![],
        }
    }

    #[test]
    fn missing_evidence_returns_invalid() {
        let e = EditorialEngine::new();
        let mut r = engine_req();
        r.director_request.evidence_refs.clear();
        assert!(matches!(e.plan(&r), Err(PlanError::InvalidInputs)));
    }

    #[test]
    fn missing_proposal_returns_no_proposal() {
        let e = EditorialEngine::new();
        let mut r = engine_req();
        r.director_proposal = None;
        let res = e.plan(&r).unwrap();
        assert!(matches!(res.error, Some(PlanError::NoProposal)));
    }

    #[test]
    fn plan_succeeds_with_clean_inputs() {
        let e = EditorialEngine::new();
        let res = e.plan(&engine_req()).unwrap();
        assert!(res.error.is_none());
        let plan = res.plan.unwrap();
        assert_eq!(plan.proposal_id, "p1");
        assert_eq!(plan.review_mode, ReviewMode::Reviewed);
    }
}
