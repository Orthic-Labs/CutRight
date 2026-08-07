// Integration tests for filler / false-start / slate / handling /
// dead-air decisions (Book 4 lane B, B4-015).

use video_editorial::deterministic::dead_air::{classify_silence, word_safe_range};
use video_editorial::deterministic::disfluency::{decide_false_start, decide_filler, RemovalTier};

#[test]
fn filler_neighbors_automatic() {
    assert_eq!(decide_filler("um", true), RemovalTier::Automatic);
}

#[test]
fn laughter_is_preserve() {
    assert_eq!(decide_filler("haha", true), RemovalTier::Preserve);
}

#[test]
fn false_start_with_replacement_automatic() {
    assert_eq!(decide_false_start(true), RemovalTier::Automatic);
}

#[test]
fn dead_air_pre_speech() {
    let r = classify_silence(0, 1000, None, Some(1500), 300);
    assert_eq!(
        r.kind,
        video_editorial::deterministic::dead_air::DeadAirKind::PreSpeech
    );
}

#[test]
fn word_safe_basic() {
    let ws = vec![100, 200, 300];
    let we = vec![150, 250, 350];
    let (s, e) = word_safe_range(160, 320, &ws, &we);
    assert_eq!(s, 150);
    assert_eq!(e, 300);
}
