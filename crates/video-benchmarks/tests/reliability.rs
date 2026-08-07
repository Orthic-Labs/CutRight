// Integration tests for reliability (atomicity, undo, crash, cache,
// offline) evaluators (Book 4 lane A, B4-010).

use video_benchmarks::reliability::{
    evaluate_atomicity, AtomicityEvaluator, FaultRun, FaultState, InjectionPoint,
    OfflineEvaluator,
};
use video_benchmarks::{BenchmarkEvaluator, EvalContext};

fn run(seed: u64, ip: InjectionPoint, s: FaultState) -> FaultRun {
    FaultRun {
        seed,
        injection: ip,
        state_after: s,
        network_attempts: 0,
        receipt_valid: true,
        undo_hash_ok: true,
    }
}

fn ctx() -> EvalContext {
    EvalContext::default()
}

#[test]
fn full_matrix_passes() {
    let mut runs = Vec::new();
    for s in 0..16u64 {
        for ip in [
            InjectionPoint::PreCommit,
            InjectionPoint::MidCommit,
            InjectionPoint::PostCommit,
            InjectionPoint::CacheWrite,
        ] {
            runs.push(run(s, ip, FaultState::OldComplete));
            runs.push(run(s + 1000, ip, FaultState::NewComplete));
        }
    }
    let r = evaluate_atomicity(&runs);
    assert!(r.pass());
    assert_eq!(r.inconsistent, 0);
    assert_eq!(r.lost, 0);
}

#[test]
fn single_inconsistent_run_fails_acceptance() {
    let runs = vec![
        run(1, InjectionPoint::MidCommit, FaultState::NewComplete),
        run(2, InjectionPoint::MidCommit, FaultState::Inconsistent),
    ];
    let r = evaluate_atomicity(&runs);
    assert!(!r.pass());
}

#[test]
fn atomicity_evaluator_id_and_axis() {
    let e = AtomicityEvaluator;
    assert_eq!(e.id(), "reliability.atomicity.inconsistent");
    let out = e.evaluate(&ctx()).expect("ok");
    assert_eq!(out.metric_id, "reliability.atomicity.inconsistent");
}

#[test]
fn offline_evaluator_id() {
    let e = OfflineEvaluator;
    assert_eq!(e.id(), "reliability.offline.network_attempts");
    let out = e.evaluate(&ctx()).expect("ok");
    assert_eq!(out.metric_id, "reliability.offline.network_attempts");
}