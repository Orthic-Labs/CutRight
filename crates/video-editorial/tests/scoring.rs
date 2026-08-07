// Integration tests for take scoring and hard-fault disqualification
// (Book 4 lane B, B4-014).

use video_editorial::deterministic::faults::{disqualifies, HardFault};
use video_editorial::deterministic::scoring::{
    score_take, winner_margin, ComponentScore, TakeStatus,
};

fn comp(signal: &str, value: f32, weight: f32, missing: bool) -> ComponentScore {
    ComponentScore {
        signal: signal.to_string(),
        value,
        weight,
        missing_evidence: missing,
    }
}

#[test]
fn score_basic() {
    let s = score_take(
        "t1",
        vec![
            comp("delivery", 0.8, 0.5, false),
            comp("completeness", 0.6, 0.5, false),
        ],
        vec![],
    );
    assert!((s.total - 0.7).abs() < 1e-5);
    assert_eq!(s.status, TakeStatus::Selected);
}

#[test]
fn hard_fault_disqualifies() {
    let s = score_take(
        "t1",
        vec![comp("x", 0.95, 1.0, false)],
        vec!["clipped_word".into()],
    );
    assert_eq!(s.status, TakeStatus::Disqualified);
}

#[test]
fn missing_evidence_is_inconclusive() {
    let s = score_take("t1", vec![comp("x", 0.5, 1.0, true)], vec![]);
    assert_eq!(s.status, TakeStatus::Inconclusive);
}

#[test]
fn winner_margin_computed() {
    let a = score_take("a", vec![comp("x", 0.8, 1.0, false)], vec![]);
    let b = score_take("b", vec![comp("x", 0.4, 1.0, false)], vec![]);
    let m = winner_margin(&[a, b]);
    assert!((m - 0.4).abs() < 1e-5);
}

#[test]
fn hard_fault_catalogue_complete() {
    assert!(disqualifies(&HardFault::ClippedWord {
        word_id: "w".into()
    }));
    assert!(disqualifies(&HardFault::SourceCorruption {
        detail: "x".into()
    }));
    assert!(disqualifies(&HardFault::UnusableExposure { luma: 0.0 }));
    assert!(disqualifies(&HardFault::UnusableAudio { snr_db: -20.0 }));
    assert!(disqualifies(&HardFault::IdentityViolation {
        subject_id: "s".into()
    }));
}
