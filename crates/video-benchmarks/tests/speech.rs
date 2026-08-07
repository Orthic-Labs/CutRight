// Tests for the speech preservation evaluator (Book 4 lane A, B4-007).
use video_benchmarks::speech::{
    evaluate_speech, is_word_kept, protected_interval, BoundaryConsensusEvaluator, OutputMapping,
    Word, WordClippingEvaluator, DEFAULT_TOLERANCE_IN_MS, DEFAULT_TOLERANCE_OUT_MS,
};
use video_benchmarks::{BenchmarkEvaluator, EvalContext, MetricStatus};

fn make_word(id: &str, text: &str, start_ms: i64, end_ms: i64, confidence: f32) -> Word {
    Word {
        word_id: id.to_string(),
        text: text.to_string(),
        start_ms,
        end_ms,
        confidence,
        speaker: None,
    }
}

#[test]
fn no_cut_control_reports_perfect_preservation() {
    let words = vec![
        make_word("w1", "hello", 1000, 1100, 0.95),
        make_word("w2", "world", 1200, 1350, 0.95),
    ];
    let mappings = vec![OutputMapping {
        source_start_ms: 0,
        source_end_ms: 2000,
        output_start_ms: 0,
        output_end_ms: 2000,
        kept: true,
    }];
    let result = evaluate_speech(&words, &mappings);
    assert_eq!(result.clipped.len(), 0);
    assert!((result.preserved_coverage - 1.0).abs() < 1e-9);
}

#[test]
fn known_clip_failure_returns_specific_word_id() {
    let words = vec![
        make_word("w1", "hello", 1000, 1200, 0.95),
        make_word("w2", "world", 1200, 1450, 0.95),
    ];
    let mappings = vec![
        OutputMapping {
            source_start_ms: 1000,
            source_end_ms: 1100,
            output_start_ms: 0,
            output_end_ms: 100,
            kept: true,
        },
        OutputMapping {
            source_start_ms: 1200,
            source_end_ms: 1350,
            output_start_ms: 100,
            output_end_ms: 250,
            kept: true,
        },
    ];
    let result = evaluate_speech(&words, &mappings);
    assert_eq!(result.clipped.len(), 2);
    assert_eq!(result.clipped[0].word_id, "w1");
    assert_eq!(result.clipped[1].word_id, "w2");
}

#[test]
fn protected_interval_subtracts_tolerance() {
    let word = make_word("w1", "hello", 1000, 1100, 0.95);
    let (start, end) = protected_interval(&word, 20, 20);
    assert_eq!(start, 1020);
    assert_eq!(end, 1080);
}

#[test]
fn is_word_kept_requires_fully_covering_mapping() {
    let word = make_word("w1", "hello", 1000, 1100, 0.95);
    let mappings_partial = vec![OutputMapping {
        source_start_ms: 1000,
        source_end_ms: 1050,
        output_start_ms: 0,
        output_end_ms: 50,
        kept: true,
    }];
    assert!(!is_word_kept(
        &word,
        &mappings_partial,
        DEFAULT_TOLERANCE_IN_MS,
        DEFAULT_TOLERANCE_OUT_MS
    ));
    let mappings_full = vec![OutputMapping {
        source_start_ms: 1000,
        source_end_ms: 1100,
        output_start_ms: 0,
        output_end_ms: 100,
        kept: true,
    }];
    assert!(is_word_kept(
        &word,
        &mappings_full,
        DEFAULT_TOLERANCE_IN_MS,
        DEFAULT_TOLERANCE_OUT_MS
    ));
}

#[test]
fn dropped_mappings_never_count_as_kept() {
    let word = make_word("w1", "hello", 1000, 1100, 0.95);
    let mappings = vec![OutputMapping {
        source_start_ms: 1000,
        source_end_ms: 1100,
        output_start_ms: 0,
        output_end_ms: 100,
        kept: false,
    }];
    assert!(!is_word_kept(
        &word,
        &mappings,
        DEFAULT_TOLERANCE_IN_MS,
        DEFAULT_TOLERANCE_OUT_MS
    ));
}

#[test]
fn metrics_are_deterministic() {
    let words = vec![make_word("w1", "hello", 1000, 1100, 0.95)];
    let mappings = vec![OutputMapping {
        source_start_ms: 0,
        source_end_ms: 2000,
        output_start_ms: 0,
        output_end_ms: 2000,
        kept: true,
    }];
    let a = evaluate_speech(&words, &mappings);
    let b = evaluate_speech(&words, &mappings);
    assert_eq!(a, b);
}

#[test]
fn evaluator_id_matches_metric_id() {
    let ev = WordClippingEvaluator::default();
    assert_eq!(ev.id(), "speech.word_clipping.high_confidence");
    let ev2 = BoundaryConsensusEvaluator;
    assert_eq!(ev2.id(), "speech.boundary.consensus_coverage");
}

#[test]
fn empty_input_yields_pass_with_zero_count() {
    let ev = WordClippingEvaluator::default();
    let outcome = ev.evaluate(&EvalContext::default()).expect("ok");
    assert_eq!(outcome.status, MetricStatus::Pass);
    assert_eq!(outcome.value, Some(0.0));
}
