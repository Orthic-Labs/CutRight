use video_core::{VadRegion, Word};
use video_project::{
    compile_locked_cut, compile_locked_cut_from_evidence, deterministic_variants,
    rank_asset_candidates, AssetCandidateScore, EditorTakeover, LockedCut, RationalRate,
    RationalTime, WordSafeSegment, WordSpan,
};

fn sample() -> LockedCut {
    LockedCut {
        schema_version: 1,
        cut_plan_sha256: String::new(),
        timeline_rate: RationalRate {
            numerator: 30,
            denominator: 1,
        },
        segments: vec![WordSafeSegment {
            source_id: "source-a".into(),
            source_in: RationalTime::millis(0),
            source_out: RationalTime::millis(1000),
            timeline_in: RationalTime::millis(0),
            timeline_out: RationalTime::millis(1000),
            first_word_id: "w1".into(),
            last_word_id: "w2".into(),
            speech_region_ids: vec!["vad-1".into()],
            gap: false,
        }],
    }
}

#[test]
fn forced_word_vad_cut_compiles_and_locks() {
    let cut = compile_locked_cut(sample()).expect("valid word/VAD cut");
    assert!(!cut.cut_plan_sha256.is_empty());
    assert!(cut.assert_hash(&cut.cut_plan_sha256).is_ok());
}

#[test]
fn mutation_after_lock_is_rejected() {
    let mut cut = compile_locked_cut(sample()).unwrap();
    let hash = cut.cut_plan_sha256.clone();
    cut.segments[0].timeline_out = RationalTime::millis(900);
    assert!(cut.assert_hash(&hash).is_err());
}

fn word(id: &str, start_ms: i64, end_ms: i64) -> Word {
    Word {
        id: id.into(),
        source_word_id: Some(format!("source-a:{id}")),
        text: id.into(),
        start_ms,
        end_ms,
        confidence: 0.99,
        speaker: None,
        kind: "word".into(),
    }
}

#[test]
fn forced_word_boundaries_and_vad_are_required() {
    let words = vec![word("w1", 0, 500), word("w2", 500, 1000)];
    let vad = vec![VadRegion {
        start_ms: 0,
        end_ms: 1000,
        mean_probability: 0.9,
    }];
    assert!(compile_locked_cut_from_evidence(sample(), &words, &vad).is_ok());

    let mut raw_asr = words.clone();
    raw_asr[0].source_word_id = None;
    assert!(compile_locked_cut_from_evidence(sample(), &raw_asr, &vad).is_err());
    assert!(compile_locked_cut_from_evidence(sample(), &words, &[]).is_err());

    let mut inside_word = sample();
    inside_word.segments[0].source_out = RationalTime::millis(900);
    assert!(compile_locked_cut_from_evidence(inside_word, &words, &vad).is_err());
}

#[test]
fn asset_ranking_variants_and_takeover_are_deterministic_and_bounded() {
    let variants = deterministic_variants("pullback", "blake3:source");
    assert_eq!(variants.len(), 4);
    assert_eq!(
        variants,
        deterministic_variants("pullback", "blake3:source")
    );

    let candidate = |id: &str, license: f32| AssetCandidateScore {
        asset_id: id.into(),
        meaning: 0.8,
        motion_direction: 0.8,
        source_focus: 0.8,
        lifetime: 0.8,
        negative_space: 0.8,
        crop_safety: 0.8,
        license,
        availability: 1.0,
        rejection_reasons: vec![],
    };
    let ranked = rank_asset_candidates(vec![
        candidate("b", 1.0),
        candidate("a", 1.0),
        candidate("unlicensed", 0.0),
    ]);
    assert_eq!(
        ranked
            .iter()
            .map(|row| row.asset_id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"]
    );

    let locked = compile_locked_cut_from_evidence(
        sample(),
        &[word("w1", 0, 500), word("w2", 500, 1000)],
        &[VadRegion {
            start_ms: 0,
            end_ms: 1000,
            mean_probability: 0.9,
        }],
    )
    .unwrap();
    let takeover = EditorTakeover {
        asset_id: "asset-a".into(),
        span: WordSpan {
            first_word_id: "w1".into(),
            last_word_id: "w2".into(),
        },
        lifetime_start_ms: 0,
        lifetime_end_ms: 1000,
    };
    assert!(takeover.validate(&locked).is_ok());
    let mut leaked = takeover;
    leaked.lifetime_end_ms = 1001;
    assert!(leaked.validate(&locked).is_err());
}
