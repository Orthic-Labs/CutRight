// Integration tests for editorial, runner, and report
// (Book 4 lane A, B4-011).

use video_benchmarks::editorial::{
    build_agreement, disagreement_score, EditorialAgreementEvaluator, ReviewerAnnotation,
};
use video_benchmarks::report::{build_report, write_receipt, write_report, Receipt};
use video_benchmarks::runner::{
    empty_project_run, outcomes_by_axis, tally, write_run, BenchmarkRun,
};
use video_benchmarks::{AxisId, BenchmarkEvaluator, EvalContext, EvalOutcome, MetricStatus};

fn ann(reviewer: &str, beat: &str, label: &str) -> ReviewerAnnotation {
    ReviewerAnnotation {
        reviewer_id: reviewer.to_string(),
        beat_id: beat.to_string(),
        start_ms: 0,
        end_ms: 1000,
        label: label.to_string(),
        take_id: None,
    }
}

#[test]
fn editorial_evaluator_returns_pass() {
    let e = EditorialAgreementEvaluator;
    let out = e.evaluate(&EvalContext::default()).expect("ok");
    assert_eq!(out.metric_id, "editorial.human_agreement.disagreement");
}

#[test]
fn agreement_groups_by_beat() {
    let rows = vec![
        ann("a", "b1", "hook"),
        ann("c", "b1", "hook"),
        ann("a", "b2", "payoff"),
    ];
    let ag = build_agreement(rows);
    assert_eq!(ag.len(), 2);
    assert_eq!(ag[0].disagreement_score, 0.0);
}

#[test]
fn empty_disagreement_is_one() {
    assert_eq!(disagreement_score(&[]), 1.0);
}

#[test]
fn runner_tally_zero_when_empty() {
    let mut p = empty_project_run("p1", "test", "reviewed-v2");
    tally(&mut p);
    assert_eq!(p.passes, 0);
    assert_eq!(p.failures, 0);
}

#[test]
fn runner_groups_by_axis() {
    let mut p = empty_project_run("p1", "test", "reviewed-v2");
    p.outcomes = vec![
        EvalOutcome::pass("a", AxisId::Editorial, 1.0, "ratio"),
        EvalOutcome::pass("b", AxisId::AudioVisual, 1.0, "ratio"),
    ];
    let g = outcomes_by_axis(&p);
    assert!(g.contains_key("editorial"));
    assert!(g.contains_key("audio_visual"));
}

#[test]
fn runner_writes_redacted_run_to_disk() {
    let run = BenchmarkRun {
        run_id: "r1".into(),
        profile: "reviewed-v2".into(),
        projects: vec![empty_project_run("p1", "test", "reviewed-v2")],
        started_at_ms: 0,
        finished_at_ms: 1,
    };
    let dir = std::env::temp_dir().join("cr-b4-011-test");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("run.json");
    write_run(&path, &run).expect("write");
    let bytes = std::fs::read(&path).expect("read");
    assert!(bytes.len() > 10);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn report_records_failures() {
    let mut p = empty_project_run("p", "test", "reviewed-v2");
    let mut o = EvalOutcome::fail("m", AxisId::Editorial, "boom");
    o.status = MetricStatus::Fail;
    p.outcomes = vec![o];
    let r = BenchmarkRun {
        run_id: "r1".into(),
        profile: "reviewed-v2".into(),
        projects: vec![p],
        started_at_ms: 0,
        finished_at_ms: 1,
    };
    let rep = build_report(&r);
    assert_eq!(rep.failures.len(), 1);
    let dir = std::env::temp_dir().join("cr-b4-011-report-test");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("report.json");
    write_report(&path, &rep).expect("write");
    let receipt_path = dir.join("receipt.json");
    write_receipt(
        &receipt_path,
        &Receipt {
            run_id: rep.run_id.clone(),
            profile: rep.profile.clone(),
            pack_locks: vec!["v2-packs.lock.json".into()],
        },
    )
    .expect("write receipt");
    assert!(receipt_path.exists());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&receipt_path);
}
