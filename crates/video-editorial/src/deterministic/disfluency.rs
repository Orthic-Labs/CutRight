// Contextual filler, false-start, slate, handling, and dead-air
// decisions (Book 4 lane B, B4-015).
//
// Classifies isolated filler vs emphasis, abandoned false starts with
// or without a complete replacement, slate/setup, handling, and dead
// air. Decisions are tiered: Automatic / SuggestOnly / Preserve.
// Canonical transcript words are never deleted.

use serde::{Deserialize, Serialize};

/// Tier of automaticity for a disfluency decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemovalTier {
    Automatic,
    SuggestOnly,
    Preserve,
}

/// Decision for a single disfluency region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisfluencyDecision {
    pub range: [i64; 2],
    pub kind: DisfluencyKind,
    pub tier: RemovalTier,
    pub word_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisfluencyKind {
    Filler,
    FalseStart,
    Slate,
    Handling,
    Emphasis,
    DeadAir,
    Laughter,
}

/// Tokens that are pure fillers when isolated, emphasis when repeated.
const FILLER_WORDS: &[&str] = &["um", "uh", "er", "ah", "hmm"];

/// Classify a single word as filler/emphasis/laughter.
pub fn classify_token(token: &str) -> DisfluencyKind {
    let lower = token.to_lowercase();
    if FILLER_WORDS.contains(&lower.as_str()) {
        DisfluencyKind::Filler
    } else if lower == "haha" || lower == "hehe" {
        DisfluencyKind::Laughter
    } else {
        DisfluencyKind::Emphasis
    }
}

/// Decide removal tier for a filler token. Laughter is always Preserve.
pub fn decide_filler(token: &str, has_neighbors: bool) -> RemovalTier {
    match classify_token(token) {
        DisfluencyKind::Laughter => RemovalTier::Preserve,
        DisfluencyKind::Filler if has_neighbors => RemovalTier::Automatic,
        DisfluencyKind::Filler => RemovalTier::SuggestOnly,
        _ => RemovalTier::Preserve,
    }
}

/// Decide removal tier for a false start. Automatic only when a complete
/// replacement exists; otherwise Preserve or SuggestOnly.
pub fn decide_false_start(has_complete_replacement: bool) -> RemovalTier {
    if has_complete_replacement {
        RemovalTier::Automatic
    } else {
        RemovalTier::Preserve
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filler_with_neighbors_is_automatic() {
        assert_eq!(decide_filler("um", true), RemovalTier::Automatic);
    }

    #[test]
    fn filler_without_neighbors_is_suggest_only() {
        assert_eq!(decide_filler("uh", false), RemovalTier::SuggestOnly);
    }

    #[test]
    fn laughter_is_always_preserve() {
        assert_eq!(decide_filler("haha", true), RemovalTier::Preserve);
    }

    #[test]
    fn false_start_without_replacement_is_preserve() {
        assert_eq!(decide_false_start(false), RemovalTier::Preserve);
    }

    #[test]
    fn false_start_with_replacement_is_automatic() {
        assert_eq!(decide_false_start(true), RemovalTier::Automatic);
    }
}