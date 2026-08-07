// Dead-air classification and word-safe range handling
// (Book 4 lane B, B4-015).

use serde::{Deserialize, Serialize};

/// A dead-air region (silence with no usable content).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeadAirRegion {
    pub start_ms: i64,
    pub end_ms: i64,
    pub kind: DeadAirKind,
    pub keep: bool,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeadAirKind {
    PreSpeech,
    InterSpeech,
    PostSpeech,
    Breathing,
    NoUsableContent,
}

/// Classify a silence region based on its position relative to nearby
/// speech and its duration.
pub fn classify_silence(
    start_ms: i64,
    end_ms: i64,
    speech_before_end_ms: Option<i64>,
    speech_after_start_ms: Option<i64>,
    min_keep_ms: i64,
) -> DeadAirRegion {
    let dur = end_ms - start_ms;
    let kind = match (speech_before_end_ms, speech_after_start_ms) {
            (None, _) => DeadAirKind::PreSpeech,
            (_, None) => DeadAirKind::PostSpeech,
            (Some(b), Some(a)) => {
                // Silence is "between" speech if both speech markers
                // bound it (b <= start and a >= end). In that case a
                // short gap is breathing; a longer gap is
                // inter-speech.
                // If either marker overlaps the silence, the silence
                // is fully inside speech and is inter-speech.
                if b > end_ms || a < start_ms {
                    DeadAirKind::InterSpeech
                } else if dur < min_keep_ms {
                    DeadAirKind::Breathing
                } else {
                    DeadAirKind::InterSpeech
                }
            }
            _ => DeadAirKind::Breathing,
        };
    let keep = dur < min_keep_ms || matches!(kind, DeadAirKind::Breathing);
    DeadAirRegion {
        start_ms,
        end_ms,
        kind,
        keep,
        evidence_refs: Vec::new(),
    }
}

/// Compute a word-safe range by snapping each end to the nearest word
/// boundary (any word start or word end). When two boundaries are
/// equidistant, `safe_start` snaps forward and `safe_end` snaps
/// backward. This guarantees we never cut inside a word while
/// minimising the displacement from the requested range.
pub fn word_safe_range(start_ms: i64, end_ms: i64, word_starts: &[i64], word_ends: &[i64]) -> (i64, i64) {
    if word_starts.is_empty() || word_ends.is_empty() {
        return (start_ms, end_ms);
    }
    let boundaries: Vec<i64> = word_starts
        .iter()
        .chain(word_ends.iter())
        .copied()
        .collect();

    let safe_start = nearest_boundary(&boundaries, start_ms, /*prefer_forward*/ true);
    let safe_end = nearest_boundary(&boundaries, end_ms, /*prefer_forward*/ false);
    (safe_start, safe_end)
}

fn nearest_boundary(boundaries: &[i64], target: i64, prefer_forward: bool) -> i64 {
    let mut best = boundaries[0];
    let mut best_dist = (best - target).abs();
    let mut best_forward = best >= target;
    for &b in &boundaries[1..] {
        let d = (b - target).abs();
        let forward = b >= target;
        if d < best_dist || (d == best_dist && prefer_forward && !best_forward) {
            best = b;
            best_dist = d;
            best_forward = forward;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_pre_speech() {
        let r = classify_silence(0, 1000, None, Some(1500), 300);
        assert_eq!(r.kind, DeadAirKind::PreSpeech);
        assert!(!r.keep);
    }

    #[test]
    fn classify_breathing_short() {
        let r = classify_silence(1000, 1100, Some(900), Some(1200), 300);
        assert_eq!(r.kind, DeadAirKind::Breathing);
        assert!(r.keep);
    }

    #[test]
    fn classify_inter_speech() {
        let r = classify_silence(1000, 1100, Some(1200), Some(900), 300);
        assert_eq!(r.kind, DeadAirKind::InterSpeech);
    }

    #[test]
    fn word_safe_clamps_to_boundaries() {
        let ws = vec![500, 1000, 1500];
        let we = vec![600, 1100, 1600];
        let (s, e) = word_safe_range(550, 1550, &ws, &we);
        assert_eq!(s, 600);
        assert_eq!(e, 1500);
    }
}