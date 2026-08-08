//! Strict, non-durable provider output for editorial planning.
//!
//! Providers may return only [`EditorialPlanDraft`].  Callers must compile it
//! through `plan::compile_draft` before a canonical plan can cross a durable
//! boundary.

use serde::{Deserialize, Serialize};

use crate::narrative::confidence::ReviewMode;

fn default_schema() -> String {
    "cutright.agent.editorial_plan_draft/v1".into()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditorialPlanDraft {
    #[serde(default = "default_schema")]
    pub schema: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub plan_id: String,
    #[serde(default)]
    pub source_revision: String,
    #[serde(default)]
    pub evidence_graph_revision: String,
    #[serde(default)]
    pub policy_version: String,
    pub beats: Vec<EditorialBeatDraft>,
    pub order: Vec<String>,
    #[serde(default)]
    pub reorder_logs: Vec<DraftReorderLog>,
    #[serde(default)]
    pub escalations: Vec<DraftEscalation>,
    #[serde(default)]
    pub drop_reasons: Vec<DraftDropReason>,
    pub chronological_status: ChronologicalStatus,
    #[serde(default)]
    pub review_flags: Vec<ReviewFlag>,
    #[serde(default)]
    pub review_mode: Option<ReviewMode>,
}

fn default_schema_version() -> u32 {
    2
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditorialBeatDraft {
    pub beat_id: String,
    pub label: String,
    pub selected_take: String,
    #[serde(default)]
    pub alternates: Vec<DraftTakeScore>,
    pub confidence: f64,
    pub evidence: Vec<BeatEvidence>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftTakeScore {
    pub candidate_id: String,
    #[serde(default)]
    pub scores: DraftScores,
    #[serde(default)]
    pub weight_total: f64,
    #[serde(default)]
    pub total: f64,
    #[serde(default)]
    pub winner_margin: f64,
    #[serde(default)]
    pub missing_evidence: Vec<String>,
    #[serde(default)]
    pub hard_faults: Vec<String>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct DraftScores {
    #[serde(default)]
    pub delivery: f64,
    #[serde(default)]
    pub completeness: f64,
    #[serde(default)]
    pub technical: f64,
    #[serde(default)]
    pub hook_strength: f64,
    #[serde(default)]
    pub payoff_strength: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeatEvidence {
    pub source_range: [i64; 2],
    #[serde(default)]
    pub word_ids: Vec<String>,
    #[serde(default)]
    pub frame_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftReorderLog {
    pub from_index: usize,
    pub to_index: usize,
    pub reason: String,
    #[serde(default)]
    pub claim_dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct DraftEscalation {
    #[serde(default)]
    pub escalation_id: String,
    #[serde(default)]
    pub kind: String,
    pub blocking: bool,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub review_mode_target: Option<String>,
    #[serde(default)]
    pub raised_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftDropReason {
    pub candidate_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChronologicalStatus {
    Truthful,
    TruthfulWithDisclosedReorder,
    FalseChronologyBlocked,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewFlag {
    pub flag: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

impl EditorialPlanDraft {
    /// Compile this transient provider result into the durable model.
    pub fn compile(&self) -> Result<crate::plan::EditorialPlan, crate::plan::DraftValidationError> {
        crate::plan::compile_draft(self)
    }
}
