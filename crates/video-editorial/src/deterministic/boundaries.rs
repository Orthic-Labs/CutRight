// Boundary consensus and word-safe segment compilation
// (Book 4 lane B, B4-016).
//
// Migrates the supplied Cutaway behavior: VAD/audio energy identifies
// speech regions; timed words/verifier evidence identifies complete
// lexical edges. Compiles natural / tight subsegments at word gaps while
// preserving whole words and clamping pads away from neighbouring words.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeechRegion {
    pub start_ms: i64,
    pub end_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WordEdge {
    pub start_ms: i64,
    pub end_ms: i64,
    pub word_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CutSegment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub kind: SegmentKind,
    pub pad_policy: PadPolicy,
    pub word_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub fallback_used: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentKind {
    Natural,
    Tight,
    LongForm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PadPolicy {
    None,
    Short,
    Generous,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsensusRecord {
    pub boundary: BoundarySource,
    pub agreement_count: u8,
    pub total_providers: u8,
    pub fallback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundarySource {
    VadAndWords,
    WordsOnly,
    VadOnly,
    Fallback,
}

/// Compile cut segments within speech regions at word gaps.
pub fn compile_segments(
    speech: &[SpeechRegion],
    words: &[WordEdge],
    pad: PadPolicy,
) -> Vec<CutSegment> {
    let mut segments = Vec::new();
    for region in speech {
        let pad_ms: i64 = match pad {
            PadPolicy::None => 0,
            PadPolicy::Short => 50,
            PadPolicy::Generous => 200,
        };
        let start_ms = clamp_pad(region.start_ms - pad_ms, words, true);
        let end_ms = clamp_pad(region.end_ms + pad_ms, words, false);
        let in_words: Vec<&WordEdge> = words
            .iter()
            .filter(|w| w.start_ms >= start_ms && w.end_ms <= end_ms)
            .collect();
        let word_ids: Vec<String> = in_words.iter().map(|w| w.word_id.clone()).collect();
        segments.push(CutSegment {
            start_ms,
            end_ms,
            kind: SegmentKind::Natural,
            pad_policy: pad,
            word_ids,
            evidence_refs: vec!["vad".into(), "words".into()],
            fallback_used: false,
        });
    }
    segments
}

fn clamp_pad(target_ms: i64, words: &[WordEdge], leading: bool) -> i64 {
    if words.is_empty() {
        return target_ms;
    }
    // Treat every word start and word end as a cuttable boundary.
    let mut boundaries: Vec<i64> = words.iter().flat_map(|w| [w.start_ms, w.end_ms]).collect();
    boundaries.sort_unstable();
    if leading {
        boundaries
            .into_iter()
            .find(|&b| b >= target_ms)
            .unwrap_or(target_ms)
    } else {
        boundaries
            .into_iter()
            .rev()
            .find(|&b| b <= target_ms)
            .unwrap_or(target_ms)
    }
}

/// Compute consensus among multiple providers. Fallback is set when
/// no two providers agree.
pub fn consensus(records: &[ConsensusRecord]) -> ConsensusRecord {
    if records.is_empty() {
        return ConsensusRecord {
            boundary: BoundarySource::Fallback,
            agreement_count: 0,
            total_providers: 0,
            fallback: true,
        };
    }
    let max_agreement = records.iter().map(|r| r.agreement_count).max().unwrap_or(0);
    let fallback = max_agreement < 2;
    ConsensusRecord {
        boundary: if fallback {
            BoundarySource::Fallback
        } else {
            BoundarySource::VadAndWords
        },
        agreement_count: max_agreement,
        total_providers: records.len() as u8,
        fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(id: &str, s: i64, e: i64) -> WordEdge {
        WordEdge { word_id: id.to_string(), start_ms: s, end_ms: e }
    }

    #[test]
    fn compile_segments_basic() {
        let speech = vec![SpeechRegion { start_ms: 1000, end_ms: 2000 }];
        let words = vec![w("w1", 1000, 1100), w("w2", 1900, 2000)];
        let segs = compile_segments(&speech, &words, PadPolicy::None);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].word_ids, vec!["w1", "w2"]);
    }

    #[test]
    fn pad_short_clamps_to_word_edge() {
        let speech = vec![SpeechRegion { start_ms: 1100, end_ms: 1900 }];
        let words = vec![w("w1", 1000, 1100), w("w2", 1900, 2000)];
        let segs = compile_segments(&speech, &words, PadPolicy::Short);
        assert_eq!(segs[0].start_ms, 1100);
        assert_eq!(segs[0].end_ms, 1900);
    }

    #[test]
    fn consensus_empty_falls_back() {
        let c = consensus(&[]);
        assert!(c.fallback);
        assert_eq!(c.boundary, BoundarySource::Fallback);
    }

    #[test]
    fn consensus_high_agreement() {
        let r = ConsensusRecord {
            boundary: BoundarySource::VadAndWords,
            agreement_count: 3,
            total_providers: 4,
            fallback: false,
        };
        let c = consensus(&[r.clone(), r.clone(), r.clone()]);
        assert!(!c.fallback);
        assert_eq!(c.agreement_count, 3);
    }
}