// Integration tests for shorts discovery (B4-019).

use video_editorial::narrative::shorts::{
    build_candidate, diversity_filter, rank, ShortBeatRef, ShortCandidate, ShortInputs,
};

fn beats() -> Vec<ShortBeatRef> {
    vec![
        ShortBeatRef {
            beat_id: "b1".into(),
            evidence_ref: "ev1".into(),
        },
        ShortBeatRef {
            beat_id: "b2".into(),
            evidence_ref: "ev2".into(),
        },
        ShortBeatRef {
            beat_id: "b3".into(),
            evidence_ref: "ev3".into(),
        },
    ]
}

fn static_inputs<'a>(
    beats: &'a [ShortBeatRef],
    hook: f32,
    ctx: f32,
    payoff: f32,
    visual: f32,
    boundary: f32,
    dup: f32,
    recorded: bool,
) -> ShortInputs<'a> {
    ShortInputs {
        beats,
        hook_strength: hook,
        standalone_context: ctx,
        payoff,
        visual_support: visual,
        boundary_confidence: boundary,
        duplication_penalty: dup,
        recorded,
    }
}

#[test]
fn self_contained_window_from_beats() {
    let b = beats();
    let c = build_candidate(
        "c1",
        "Why X",
        "X is broken because...",
        static_inputs(&b, 0.9, 0.8, 0.7, 0.6, 0.5, 0.0, true),
    );
    assert!(c.exclusion_reason.is_none());
    assert_eq!(c.beat_ids.len(), 3);
    assert!(c.evidence_refs.iter().all(|e| e.starts_with("ev")));
}

#[test]
fn overlapping_candidates_diversity_filtered() {
    let b_full = beats();
    let b_partial: Vec<ShortBeatRef> = b_full.iter().take(2).cloned().collect();
    let c_full = build_candidate(
        "full",
        "t",
        "h",
        static_inputs(&b_full, 0.8, 0.8, 0.8, 0.8, 0.8, 0.0, true),
    );
    let c_partial = build_candidate(
        "partial",
        "t",
        "h",
        static_inputs(&b_partial, 0.9, 0.9, 0.9, 0.9, 0.9, 0.0, true),
    );
    let kept = diversity_filter(vec![c_partial, c_full]);
    // full ranks above partial by score; partial is a subset of full -> dropped
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].candidate_id, "full");
}

#[test]
fn source_ranges_compiled_from_evidence_bound_beats() {
    let b = beats();
    let c = build_candidate(
        "c1",
        "t",
        "h",
        static_inputs(&b, 0.5, 0.5, 0.5, 0.5, 0.5, 0.0, true),
    );
    // No timestamps are produced by the model itself; beats are the source.
    assert!(c.beat_ids.iter().all(|id| id.starts_with("b")));
    assert!(c.evidence_refs.iter().all(|e| e.starts_with("ev")));
}

#[test]
fn unrecorded_excluded_with_reason() {
    let b = beats();
    let c = build_candidate(
        "c1",
        "t",
        "h",
        static_inputs(&b, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0, false),
    );
    assert!(c.exclusion_reason.is_some());
    let v: Vec<ShortCandidate> = vec![c];
    let r = rank(&v);
    assert_eq!(r.len(), 1);
    assert!(r[0].exclusion_reason.is_some());
}
