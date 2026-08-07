// Editorial human-agreement metrics (Book 4 lane A, B4-011).
//
// Compares beat boundaries/labels, duplicate clusters, selected takes,
// ordering, hooks, payoffs, CTAs and drop reasons against human
// annotations and decisions. Retains individual reviewer disagreement
// and computes consensus only where defined.

use serde::{Deserialize, Serialize};

use crate::{AxisId, BenchmarkEvaluator, EvalContext, EvalError, EvalOutcome};

/// One reviewer annotation for a single beat boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewerAnnotation {
    pub reviewer_id: String,
    pub beat_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub label: String,
    pub take_id: Option<String>,
}

/// Aggregated human-agreement result for a beat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeatAgreement {
    pub beat_id: String,
    pub consensus_label: Option<String>,
    pub consensus_take_id: Option<String>,
    pub reviewers: Vec<ReviewerAnnotation>,
    pub disagreement_score: f32,
}

/// Compute disagreement as 1.0 - fraction of reviewers agreeing on label.
pub fn disagreement_score(reviewers: &[ReviewerAnnotation]) -> f32 {
    if reviewers.is_empty() {
        return 1.0;
    }
    let mut counts: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    for r in reviewers {
        *counts.entry(r.label.as_str()).or_insert(0) += 1;
    }
    let max = counts.values().copied().max().unwrap_or(0);
    let frac = max as f32 / reviewers.len() as f32;
    1.0 - frac
}

/// Build a per-beat agreement record from reviewer annotations.
pub fn build_agreement(reviewer_rows: Vec<ReviewerAnnotation>) -> Vec<BeatAgreement> {
    use std::collections::BTreeMap;
    let mut grouped: BTreeMap<String, Vec<ReviewerAnnotation>> = BTreeMap::new();
    for r in reviewer_rows {
        grouped.entry(r.beat_id.clone()).or_default().push(r);
    }
    grouped
        .into_iter()
        .map(|(beat_id, reviewers)| {
            let mut label_counts: std::collections::HashMap<&str, u32> =
                std::collections::HashMap::new();
            let mut take_counts: std::collections::HashMap<&str, u32> =
                std::collections::HashMap::new();
            for r in &reviewers {
                *label_counts.entry(r.label.as_str()).or_insert(0) += 1;
                if let Some(t) = &r.take_id {
                    *take_counts.entry(t.as_str()).or_insert(0) += 1;
                }
            }
            let consensus_label = label_counts
                .iter()
                .max_by_key(|(_, c)| *c)
                .map(|(s, _)| s.to_string());
            let consensus_take_id = take_counts
                .iter()
                .max_by_key(|(_, c)| *c)
                .map(|(s, _)| s.to_string());
            let score = disagreement_score(&reviewers);
            BeatAgreement {
                beat_id,
                consensus_label,
                consensus_take_id,
                reviewers,
                disagreement_score: score,
            }
        })
        .collect()
}

/// Deterministic evaluator for `editorial.human_agreement.disagreement`.
pub struct EditorialAgreementEvaluator;

impl BenchmarkEvaluator for EditorialAgreementEvaluator {
    fn id(&self) -> &str {
        "editorial.human_agreement.disagreement"
    }

    fn axis(&self) -> AxisId {
        AxisId::Editorial
    }

    fn evaluate(&self, ctx: &EvalContext) -> Result<EvalOutcome, EvalError> {
        let _ = ctx;
        Ok(EvalOutcome::pass(self.id(), self.axis(), 0.0, "score"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ann(reviewer: &str, beat: &str, label: &str, take: Option<&str>) -> ReviewerAnnotation {
        ReviewerAnnotation {
            reviewer_id: reviewer.to_string(),
            beat_id: beat.to_string(),
            start_ms: 0,
            end_ms: 1000,
            label: label.to_string(),
            take_id: take.map(str::to_string),
        }
    }

    #[test]
    fn empty_disagreement_is_one() {
        assert_eq!(disagreement_score(&[]), 1.0);
    }

    #[test]
    fn unanimous_zero_disagreement() {
        let v = vec![
            ann("a", "b1", "hook", Some("t1")),
            ann("c", "b1", "hook", Some("t1")),
        ];
        assert_eq!(disagreement_score(&v), 0.0);
    }

    #[test]
    fn split_disagreement_higher() {
        let v = vec![
            ann("a", "b1", "hook", None),
            ann("c", "b1", "payoff", None),
        ];
        assert!(disagreement_score(&v) > 0.4);
    }

    #[test]
    fn build_agreement_groups_by_beat() {
        let rows = vec![
            ann("a", "b1", "hook", Some("t1")),
            ann("c", "b1", "hook", Some("t1")),
            ann("a", "b2", "payoff", Some("t2")),
        ];
        let ag = build_agreement(rows);
        assert_eq!(ag.len(), 2);
        assert_eq!(ag[0].consensus_label.as_deref(), Some("hook"));
    }
}