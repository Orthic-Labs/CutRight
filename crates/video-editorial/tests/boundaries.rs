// Integration tests for boundary consensus and word-safe cut
// compilation (Book 4 lane B, B4-016).

use video_editorial::deterministic::boundaries::{
    compile_segments, consensus, BoundarySource, ConsensusRecord, PadPolicy, SpeechRegion,
    WordEdge,
};

fn w(id: &str, s: i64, e: i64) -> WordEdge {
    WordEdge { word_id: id.into(), start_ms: s, end_ms: e }
}

#[test]
fn segments_include_words_in_region() {
    let speech = vec![SpeechRegion { start_ms: 0, end_ms: 1000 }];
    let words = vec![w("w1", 0, 200), w("w2", 800, 1000)];
    let segs = compile_segments(&speech, &words, PadPolicy::None);
    assert_eq!(segs[0].word_ids, vec!["w1", "w2"]);
}

#[test]
fn multiple_speech_regions_yield_multiple_segments() {
    let speech = vec![
        SpeechRegion { start_ms: 0, end_ms: 1000 },
        SpeechRegion { start_ms: 2000, end_ms: 3000 },
    ];
    let words = vec![
        w("w1", 0, 1000),
        w("w2", 2000, 3000),
    ];
    let segs = compile_segments(&speech, &words, PadPolicy::None);
    assert_eq!(segs.len(), 2);
}

#[test]
fn consensus_aggregates() {
    let r = ConsensusRecord {
        boundary: BoundarySource::VadAndWords,
        agreement_count: 2,
        total_providers: 3,
        fallback: false,
    };
    let c = consensus(&[r.clone(), r]);
    assert_eq!(c.agreement_count, 2);
    assert!(!c.fallback);
}

#[test]
fn fallback_when_no_agreement() {
    let r = ConsensusRecord {
        boundary: BoundarySource::VadOnly,
        agreement_count: 1,
        total_providers: 3,
        fallback: false,
    };
    let c = consensus(&[r.clone(), r]);
    assert!(c.fallback);
    assert_eq!(c.boundary, BoundarySource::Fallback);
}