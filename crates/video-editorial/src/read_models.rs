// Editorial read models (Book 4 lane C, B4-025).
//
// Bounded, paginated, read-only view over the editorial plan and
// benchmark findings. Mutation is intentionally absent: writes go
// through the action kernel. Every displayed rationale traces to
// plan/evidence fields. Stable evidence IDs and exact time ranges
// are preserved so the studio can seek/inspect.

use serde::{Deserialize, Serialize};

use crate::narrative::confidence::ConfidenceEstimate;
use crate::narrative::repair::RepairAttempt;
use crate::narrative::shorts::ShortCandidate;
use crate::narrative::truthfulness::OrderLog;
use crate::plan::EditorialPlan;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeatPage {
    pub beats: Vec<BeatRow>,
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeatRow {
    pub beat_id: String,
    pub evidence_ref: String,
    pub time_range_ms: [i64; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FindingPage {
    pub findings: Vec<BenchmarkFinding>,
    pub run_id: String,
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkFinding {
    pub metric_id: String,
    pub status: String,
    pub evidence_refs: Vec<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorialReadModel {
    pub plan_id: String,
    pub review_mode: String,
    pub order: Vec<String>,
    pub reorder_logs: Vec<OrderLog>,
    pub confidence: ConfidenceEstimate,
    pub escalations: Vec<String>,
    pub repair: Option<RepairAttempt>,
    pub shorts: Vec<ShortCandidate>,
    pub evidence_refs: Vec<String>,
    pub benchmark_refs: Vec<String>,
}

/// Page through beats from an EditorialPlan's underlying beat set.
/// Pure projection: never mutates the plan or evidence graph.
pub fn get_editorial_beats(
    plan: &EditorialPlan,
    beat_times: &[(String, [i64; 2])],
    offset: usize,
    limit: usize,
) -> BeatPage {
    // Filter beat_times to those referenced by the plan's order.
    let order_set: std::collections::HashSet<&str> =
        plan.order.order.iter().map(|s| s.as_str()).collect();
    let mut rows: Vec<BeatRow> = beat_times
        .iter()
        .filter(|(bid, _)| order_set.contains(bid.as_str()))
        .map(|(bid, range)| BeatRow {
            beat_id: bid.clone(),
            evidence_ref: plan.evidence_refs.first().cloned().unwrap_or_default(),
            time_range_ms: *range,
        })
        .collect();
    let total = rows.len();
    rows = rows.into_iter().skip(offset).take(limit).collect();
    BeatPage {
        beats: rows,
        offset,
        limit,
        total,
    }
}

/// Page through benchmark findings from a list of findings keyed by
/// run_id. Pure projection: no mutation.
pub fn get_benchmark_findings(
    run_id: &str,
    findings: &[BenchmarkFinding],
    project_id: &str,
    metric_id: &str,
    offset: usize,
    limit: usize,
) -> FindingPage {
    let _ = (project_id, metric_id); // scoping keys for future use
    let mut rows: Vec<BenchmarkFinding> = findings
        .iter()
        .filter(|f| f.metric_id == metric_id || metric_id.is_empty())
        .cloned()
        .collect();
    let total = rows.len();
    rows = rows.into_iter().skip(offset).take(limit).collect();
    FindingPage {
        findings: rows,
        run_id: run_id.to_string(),
        offset,
        limit,
        total,
    }
}

/// Build a read-only view of the plan. Mutation methods are not
/// exposed — writes remain with the action kernel.
pub fn project_read_model(plan: &EditorialPlan) -> EditorialReadModel {
    let escalations: Vec<String> = plan
        .confidence
        .escalations
        .iter()
        .map(|e| format!("{:?}", e))
        .collect();
    EditorialReadModel {
        plan_id: plan.plan_id.clone(),
        review_mode: format!("{:?}", plan.review_mode),
        order: plan.order.order.clone(),
        reorder_logs: plan.order.logs.clone(),
        confidence: plan.confidence.clone(),
        escalations,
        repair: plan.repair.clone(),
        shorts: plan.shorts.clone(),
        evidence_refs: plan.evidence_refs.clone(),
        benchmark_refs: plan.benchmark_refs.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narrative::confidence::{ConfidenceEstimate, ReviewMode};
    use crate::narrative::order::OrderPlan;

    fn plan() -> EditorialPlan {
        EditorialPlan {
            plan_id: "plan-1".into(),
            proposal_id: "p1".into(),
            review_mode: ReviewMode::Reviewed,
            order: OrderPlan {
                plan_id: "o".into(),
                order: vec!["seg1".into(), "seg2".into()],
                logs: vec![],
                has_truthfulness_risk: false,
            },
            shorts: vec![],
            confidence: ConfidenceEstimate {
                score: 0.9,
                escalations: vec![],
                requested_mode: ReviewMode::Reviewed,
                effective_mode: ReviewMode::Reviewed,
                rationale: vec![],
            },
            repair: None,
            evidence_refs: vec!["e1".into()],
            benchmark_refs: vec![],
            version: 1,
        }
    }

    #[test]
    fn beats_page_is_paginated() {
        let times: Vec<(String, [i64; 2])> = (0..10)
            .map(|i| {
                (
                    format!("seg{}", i),
                    [i as i64 * 1000, (i + 1) as i64 * 1000],
                )
            })
            .collect();
        let mut p = plan();
        // only include seg1..seg2 in the order
        p.order.order = vec!["seg1".into(), "seg2".into()];
        let page = get_editorial_beats(&p, &times, 0, 1);
        assert_eq!(page.total, 2);
        assert_eq!(page.beats.len(), 1);
        assert_eq!(page.beats[0].beat_id, "seg1");
    }

    #[test]
    fn beats_page_evidence_id_stable() {
        let times = vec![("seg1".to_string(), [0i64, 1000i64])];
        let mut p = plan();
        p.order.order = vec!["seg1".into()];
        let page = get_editorial_beats(&p, &times, 0, 10);
        assert_eq!(page.beats[0].evidence_ref, "e1");
    }

    #[test]
    fn benchmark_findings_page_filters() {
        let findings = vec![
            BenchmarkFinding {
                metric_id: "kernel.integrity".into(),
                status: "Pass".into(),
                evidence_refs: vec![],
                reason: None,
            },
            BenchmarkFinding {
                metric_id: "other".into(),
                status: "Fail".into(),
                evidence_refs: vec![],
                reason: Some("r".into()),
            },
        ];
        let page = get_benchmark_findings("run-1", &findings, "proj", "kernel.integrity", 0, 10);
        assert_eq!(page.total, 1);
        assert_eq!(page.findings[0].metric_id, "kernel.integrity");
    }

    #[test]
    fn read_model_projects_invariants() {
        let p = plan();
        let m = project_read_model(&p);
        assert_eq!(m.plan_id, "plan-1");
        assert_eq!(m.order, vec!["seg1".to_string(), "seg2".to_string()]);
        assert_eq!(m.evidence_refs, vec!["e1".to_string()]);
        assert!(m.benchmark_refs.is_empty());
    }
}
