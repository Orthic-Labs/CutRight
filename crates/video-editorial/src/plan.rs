// Editorial plan shape (Book 4 lane C, B4-022).
//
// Canonical artefact returned by the EditorialEngine. Binds the
// proposal, confidence, escalations, repair attempt, and the
// evidence/benchmark references used to compile the plan.

use serde::{Deserialize, Serialize};

use crate::narrative::confidence::{ConfidenceEstimate, ReviewMode};
use crate::narrative::order::OrderPlan;
use crate::narrative::repair::RepairAttempt;
use crate::narrative::shorts::ShortCandidate;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorialPlan {
    pub plan_id: String,
    pub proposal_id: String,
    pub review_mode: ReviewMode,
    pub order: OrderPlan,
    pub shorts: Vec<ShortCandidate>,
    pub confidence: ConfidenceEstimate,
    pub repair: Option<RepairAttempt>,
    pub evidence_refs: Vec<String>,
    pub benchmark_refs: Vec<String>,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlanError {
    NoProposal,
    CriticBlocked,
    TruthfulnessRisk,
    RepairEscalated,
    InvalidInputs,
}

/// Final result returned by `EditorialEngine::plan`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorialPlanResult {
    pub plan: Option<EditorialPlan>,
    pub error: Option<PlanError>,
    pub retrieval_receipt: String,
}