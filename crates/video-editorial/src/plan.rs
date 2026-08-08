// Editorial plan shape (Book 4 lane C, B4-022).
//
// Canonical artefact returned by the EditorialEngine. Binds the
// proposal, confidence, escalations, repair attempt, and the
// evidence/benchmark references used to compile the plan.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use thiserror::Error;

use crate::draft::{ChronologicalStatus, EditorialPlanDraft};
use crate::narrative::confidence::{ConfidenceEstimate, ReviewMode};
use crate::narrative::order::OrderPlan;
use crate::narrative::repair::RepairAttempt;
use crate::narrative::shorts::ShortCandidate;
use crate::narrative::truthfulness::{ChronologyStatus, OrderLog};

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

impl EditorialPlan {
    pub fn from_draft(draft: &EditorialPlanDraft) -> Result<Self, DraftValidationError> {
        compile_draft(draft)
    }
}

impl TryFrom<EditorialPlanDraft> for EditorialPlan {
    type Error = DraftValidationError;

    fn try_from(draft: EditorialPlanDraft) -> Result<Self, Self::Error> {
        compile_draft(&draft)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DraftValidationError {
    #[error("unsupported editorial draft schema version: {0}")]
    UnsupportedSchema(u32),
    #[error("editorial draft has invalid schema identifier: {0}")]
    InvalidSchema(String),
    #[error("editorial draft requires a plan_id")]
    MissingPlanId,
    #[error("editorial draft has no beats")]
    EmptyBeats,
    #[error("editorial draft has no selected beats")]
    EmptyOrder,
    #[error("editorial draft repeats beat {0}")]
    DuplicateBeat(String),
    #[error("editorial draft orders unknown beat {0}")]
    UnknownBeat(String),
    #[error("editorial draft has invalid beat {0}")]
    InvalidBeat(String),
    #[error("editorial draft has invalid evidence for beat {0}")]
    InvalidEvidence(String),
    #[error("editorial draft has invalid reorder log at index {0}")]
    InvalidReorderLog(usize),
    #[error("editorial draft requires review: {0}")]
    ReviewRequired(String),
    #[error("editorial draft blocks false chronology")]
    TruthfulnessRisk,
}

/// Validate and compile a transient provider draft without mutating caller
/// state. All checks complete before the canonical plan is constructed.
pub fn compile_draft(draft: &EditorialPlanDraft) -> Result<EditorialPlan, DraftValidationError> {
    if draft.schema != "cutright.agent.editorial_plan_draft/v1" {
        return Err(DraftValidationError::InvalidSchema(draft.schema.clone()));
    }
    if draft.schema_version != 2 {
        return Err(DraftValidationError::UnsupportedSchema(
            draft.schema_version,
        ));
    }
    if draft.plan_id.trim().is_empty() {
        return Err(DraftValidationError::MissingPlanId);
    }
    if draft.beats.is_empty() {
        return Err(DraftValidationError::EmptyBeats);
    }
    if draft.order.is_empty() {
        return Err(DraftValidationError::EmptyOrder);
    }
    if matches!(
        draft.chronological_status,
        ChronologicalStatus::FalseChronologyBlocked
    ) {
        return Err(DraftValidationError::TruthfulnessRisk);
    }

    let mut beats = HashMap::with_capacity(draft.beats.len());
    for beat in &draft.beats {
        if beat.beat_id.trim().is_empty()
            || beat.label.trim().is_empty()
            || beat.selected_take.trim().is_empty()
            || !beat.confidence.is_finite()
            || !(0.0..=1.0).contains(&beat.confidence)
        {
            return Err(DraftValidationError::InvalidBeat(beat.beat_id.clone()));
        }
        if beat.evidence.is_empty()
            || beat
                .evidence
                .iter()
                .any(|e| e.source_range[0] < 0 || e.source_range[1] <= e.source_range[0])
        {
            return Err(DraftValidationError::InvalidEvidence(beat.beat_id.clone()));
        }
        if beats.insert(beat.beat_id.as_str(), beat).is_some() {
            return Err(DraftValidationError::DuplicateBeat(beat.beat_id.clone()));
        }
    }

    let mut seen = BTreeSet::new();
    for beat_id in &draft.order {
        if !seen.insert(beat_id.as_str()) {
            return Err(DraftValidationError::DuplicateBeat(beat_id.clone()));
        }
        if !beats.contains_key(beat_id.as_str()) {
            return Err(DraftValidationError::UnknownBeat(beat_id.clone()));
        }
    }
    for (index, log) in draft.reorder_logs.iter().enumerate() {
        if log.reason.trim().is_empty()
            || log.from_index >= draft.order.len()
            || log.to_index >= draft.order.len()
        {
            return Err(DraftValidationError::InvalidReorderLog(index));
        }
    }
    if let Some(flag) = draft.review_flags.iter().find(|flag| {
        matches!(
            flag.flag.as_str(),
            "human_required" | "needs_truthfulness_review" | "ambiguous"
        )
    }) {
        return Err(DraftValidationError::ReviewRequired(flag.flag.clone()));
    }
    if let Some(escalation) = draft
        .escalations
        .iter()
        .find(|escalation| escalation.blocking)
    {
        return Err(DraftValidationError::ReviewRequired(
            if escalation.kind.is_empty() {
                "blocking_escalation".into()
            } else {
                escalation.kind.clone()
            },
        ));
    }

    let logs: Vec<OrderLog> = draft
        .reorder_logs
        .iter()
        .map(|log| OrderLog {
            from_index: log.from_index,
            to_index: log.to_index,
            reason: log.reason.clone(),
            claim_dependencies: log.claim_dependencies.clone(),
            chronology_status: if log.from_index == log.to_index {
                ChronologyStatus::Preserved
            } else {
                ChronologyStatus::ColdOpen
            },
            evidence_refs: vec![],
        })
        .collect();
    let score = draft
        .beats
        .iter()
        .map(|beat| beat.confidence as f32)
        .sum::<f32>()
        / draft.beats.len() as f32;
    let evidence_refs = draft
        .beats
        .iter()
        .map(|beat| format!("beat:{}", beat.beat_id))
        .collect();
    let review_mode = draft.review_mode.unwrap_or(ReviewMode::Reviewed);
    let confidence = ConfidenceEstimate {
        score,
        escalations: vec![],
        requested_mode: review_mode,
        effective_mode: review_mode,
        rationale: vec!["compiled from validated EditorialPlanDraft".into()],
    };

    Ok(EditorialPlan {
        plan_id: draft.plan_id.clone(),
        proposal_id: draft.plan_id.clone(),
        review_mode,
        order: OrderPlan {
            plan_id: format!("order-{}", draft.plan_id),
            order: draft.order.clone(),
            logs,
            has_truthfulness_risk: false,
        },
        shorts: Vec::new(),
        confidence,
        repair: None,
        evidence_refs,
        benchmark_refs: Vec::new(),
        version: draft.schema_version,
    })
}

/// Apply a draft atomically from the caller's perspective: an invalid draft
/// is rejected before the destination is changed.
pub fn apply_draft(
    destination: &mut Option<EditorialPlan>,
    draft: &EditorialPlanDraft,
) -> Result<(), DraftValidationError> {
    let compiled = compile_draft(draft)?;
    *destination = Some(compiled);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft::{BeatEvidence, ChronologicalStatus, EditorialBeatDraft};

    fn draft() -> EditorialPlanDraft {
        EditorialPlanDraft {
            schema: "cutright.agent.editorial_plan_draft/v1".into(),
            schema_version: 2,
            plan_id: "plan-1".into(),
            source_revision: "source-1".into(),
            evidence_graph_revision: "evidence-1".into(),
            policy_version: "1.0".into(),
            beats: vec![EditorialBeatDraft {
                beat_id: "hook".into(),
                label: "hook".into(),
                selected_take: "take-1".into(),
                alternates: vec![],
                confidence: 0.9,
                evidence: vec![BeatEvidence {
                    source_range: [0, 100],
                    word_ids: vec![],
                    frame_refs: vec![],
                }],
                notes: None,
            }],
            order: vec!["hook".into()],
            reorder_logs: vec![],
            escalations: vec![],
            drop_reasons: vec![],
            chronological_status: ChronologicalStatus::Truthful,
            review_flags: vec![],
            review_mode: None,
        }
    }

    #[test]
    fn invalid_draft_returns_typed_error_without_mutating_destination() {
        let mut invalid = draft();
        invalid.beats[0].evidence[0].source_range = [100, 100];
        let existing = compile_draft(&draft()).expect("valid draft");
        let before = Some(existing.clone());
        let mut destination = before.clone();

        assert_eq!(
            apply_draft(&mut destination, &invalid),
            Err(DraftValidationError::InvalidEvidence("hook".into()))
        );
        assert_eq!(destination, before);
    }

    #[test]
    fn only_compiled_plan_bytes_cross_durable_boundary() {
        let plan = compile_draft(&draft()).expect("valid draft");
        let persisted = serde_json::to_vec(&plan).expect("serialize canonical plan");
        let bytes = String::from_utf8(persisted).expect("utf8 JSON");
        assert!(!bytes.contains("SemanticEditorialPlan"));
        assert!(!bytes.contains("EditorialPlanDraft"));
        assert!(bytes.contains("plan-1"));
    }
}
