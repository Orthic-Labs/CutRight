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
        (Some(b), Some(a)) if start_ms >= b && end_ms <= a => DeadAirKind::InterSpeech,
        _ => DeadAirKind::Breathing,
    };
    let keep = dur < min_keep_ms || matches!(kind, DeadAirKind::NoUsableContent);
    DeadAirRegion {
        start_ms,
        end_ms,
        kind,
        keep,
        evidence_refs: Vec::new(),
    }
}

/// Compute a word-safe range by clamping to the nearest word boundaries.
pub fn word_safe_range(start_ms: i64, end_ms: i64, word_starts: &[i64], word_ends: &[i64]) -> (i64, i64) {
    if word_starts.is_empty() || word_ends.is_empty() {
        return (start_ms, end_ms);
    }
    let safe_start = word_ends
        .iter()
        .filter(|&&e| e <= start_ms)
        .max()
        .copied()
        .unwrap_or(start_ms);
    let safe_end = word_starts
        .iter()
        .filter(|&&s| s >= end_ms)
        .min()
        .copied()
        .unwrap_or(end_ms);
    (safe_start.max(start_ms), safe_end.min(end_ms))
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