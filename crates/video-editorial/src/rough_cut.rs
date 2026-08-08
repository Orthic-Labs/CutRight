//! Evidence-bound rough-cut selection.
//!
//! This is deliberately a small reader for the on-disk editorial-plan v2
//! contract. It selects only explicitly ordered beats and leaves source-media
//! validation to the project kernel that owns candidates and timestamps.

use std::collections::{BTreeMap, BTreeSet};

pub use crate::draft::{
    BeatEvidence, ChronologicalStatus, DraftEscalation as EditorialEscalation,
    EditorialBeatDraft as SemanticBeat, EditorialPlanDraft, ReviewFlag,
};

/// Compatibility name for legacy rough-cut readers. This is a transient
/// alias to the provider draft, never a persisted model.
pub type SemanticEditorialPlan = EditorialPlanDraft;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedBeat {
    pub beat_id: String,
    pub label: String,
    pub selected_take: String,
    pub source_ranges: Vec<[i64; 2]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticSelectionError {
    UnsupportedSchema,
    EmptyOrder,
    DuplicateBeat(String),
    UnknownBeat(String),
    InvalidEvidence(String),
    ReviewRequired(String),
    TruthfulnessRisk,
}

impl std::fmt::Display for SemanticSelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema => write!(f, "unsupported editorial plan schema"),
            Self::EmptyOrder => write!(f, "editorial plan has no selected beats"),
            Self::DuplicateBeat(id) => write!(f, "editorial plan repeats beat {id}"),
            Self::UnknownBeat(id) => write!(f, "editorial plan orders unknown beat {id}"),
            Self::InvalidEvidence(id) => write!(f, "beat {id} has invalid or missing evidence"),
            Self::ReviewRequired(flag) => write!(f, "editorial plan requires review: {flag}"),
            Self::TruthfulnessRisk => write!(f, "editorial plan blocks false chronology"),
        }
    }
}

impl std::error::Error for SemanticSelectionError {}

impl EditorialPlanDraft {
    /// Select a stable red-thread order only when supplied evidence is usable.
    /// A selected beat never arises from text inference or a candidate label.
    pub fn select_beats(&self) -> Result<Vec<SelectedBeat>, SemanticSelectionError> {
        if self.schema_version != 2 {
            return Err(SemanticSelectionError::UnsupportedSchema);
        }
        if matches!(
            self.chronological_status,
            ChronologicalStatus::FalseChronologyBlocked
        ) {
            return Err(SemanticSelectionError::TruthfulnessRisk);
        }
        if let Some(flag) = self.review_flags.iter().find(|flag| {
            matches!(
                flag.flag.as_str(),
                "human_required" | "needs_truthfulness_review" | "ambiguous"
            )
        }) {
            return Err(SemanticSelectionError::ReviewRequired(flag.flag.clone()));
        }
        if self
            .escalations
            .iter()
            .any(|escalation| escalation.blocking)
        {
            return Err(SemanticSelectionError::ReviewRequired(
                "blocking_escalation".into(),
            ));
        }
        if self.order.is_empty() {
            return Err(SemanticSelectionError::EmptyOrder);
        }
        let by_id: BTreeMap<&str, &SemanticBeat> = self
            .beats
            .iter()
            .map(|beat| (beat.beat_id.as_str(), beat))
            .collect();
        let mut seen = BTreeSet::new();
        self.order
            .iter()
            .map(|beat_id| {
                if !seen.insert(beat_id.as_str()) {
                    return Err(SemanticSelectionError::DuplicateBeat(beat_id.clone()));
                }
                let beat = by_id
                    .get(beat_id.as_str())
                    .ok_or_else(|| SemanticSelectionError::UnknownBeat(beat_id.clone()))?;
                if beat.selected_take.trim().is_empty()
                    || !(0.0..=1.0).contains(&beat.confidence)
                    || beat.evidence.is_empty()
                    || beat
                        .evidence
                        .iter()
                        .any(|e| e.source_range[1] <= e.source_range[0])
                {
                    return Err(SemanticSelectionError::InvalidEvidence(beat_id.clone()));
                }
                Ok(SelectedBeat {
                    beat_id: beat.beat_id.clone(),
                    label: beat.label.clone(),
                    selected_take: beat.selected_take.clone(),
                    source_ranges: beat.evidence.iter().map(|e| e.source_range).collect(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> EditorialPlanDraft {
        EditorialPlanDraft {
            schema: "cutright.agent.editorial_plan_draft/v1".into(),
            schema_version: 2,
            plan_id: "plan-1".into(),
            source_revision: "source-1".into(),
            evidence_graph_revision: "evidence-1".into(),
            policy_version: "1.0".into(),
            beats: vec![
                SemanticBeat {
                    beat_id: "hook".into(),
                    label: "hook".into(),
                    selected_take: "take-1".into(),
                    alternates: vec![],
                    confidence: 0.9,
                    evidence: vec![BeatEvidence {
                        source_range: [100, 400],
                        word_ids: vec![],
                        frame_refs: vec![],
                    }],
                    notes: None,
                },
                SemanticBeat {
                    beat_id: "payoff".into(),
                    label: "payoff".into(),
                    selected_take: "take-2".into(),
                    alternates: vec![],
                    confidence: 0.8,
                    evidence: vec![BeatEvidence {
                        source_range: [500, 900],
                        word_ids: vec![],
                        frame_refs: vec![],
                    }],
                    notes: None,
                },
            ],
            order: vec!["hook".into(), "payoff".into()],
            reorder_logs: vec![],
            escalations: vec![],
            drop_reasons: vec![],
            chronological_status: ChronologicalStatus::Truthful,
            review_flags: vec![],
            review_mode: None,
        }
    }

    #[test]
    fn selection_follows_explicit_red_thread_stably() {
        let first = plan().select_beats().unwrap();
        let second = plan().select_beats().unwrap();
        assert_eq!(first, second);
        assert_eq!(first[0].selected_take, "take-1");
    }

    #[test]
    fn blocked_chronology_never_selects_a_cut() {
        let mut plan = plan();
        plan.chronological_status = ChronologicalStatus::FalseChronologyBlocked;
        assert_eq!(
            plan.select_beats(),
            Err(SemanticSelectionError::TruthfulnessRisk)
        );
    }

    #[test]
    fn missing_evidence_is_unproven_not_invented() {
        let mut plan = plan();
        plan.beats[0].evidence.clear();
        assert_eq!(
            plan.select_beats(),
            Err(SemanticSelectionError::InvalidEvidence("hook".into()))
        );
    }

    #[test]
    fn blocking_escalation_requires_manual_review() {
        let mut plan = plan();
        plan.escalations.push(EditorialEscalation {
            blocking: true,
            ..Default::default()
        });
        assert_eq!(
            plan.select_beats(),
            Err(SemanticSelectionError::ReviewRequired(
                "blocking_escalation".into()
            ))
        );
    }
}
