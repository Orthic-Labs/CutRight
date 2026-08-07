//! Integration tests for deterministic scene and shot segmentation
//! (CR-V2-B3-017).
//!
//! Verifies the contract frozen by `docs/architecture/V2-EVIDENCE-GRAPH.md`:
//!
//! 1. Boundaries are deterministic — identical inputs produce byte-identical
//!    `Vec<SceneBoundary>` / `Vec<ShotBoundary>` outputs across repeated
//!    runs.
//! 2. Every boundary traces back to source frame indices.
//! 3. High-frame-rate fast-cut fixtures trigger denser refinement and
//!    produce more boundaries than the slow fixture, without losing
//!    determinism.

use video_evidence::{
    FrameStat, MotionFrame, SceneDetector, SceneRefinement, ShotDetector, ShotRefinement,
};

fn slow_scene_frames() -> Vec<FrameStat> {
    vec![
        FrameStat { index: 0, histogram_delta_milli: 0 },
        FrameStat { index: 30, histogram_delta_milli: 0 },
        FrameStat { index: 60, histogram_delta_milli: 800 },
        FrameStat { index: 90, histogram_delta_milli: 0 },
        FrameStat { index: 120, histogram_delta_milli: 0 },
        FrameStat { index: 150, histogram_delta_milli: 700 },
        FrameStat { index: 180, histogram_delta_milli: 0 },
    ]
}

fn fast_cut_scene_frames() -> Vec<FrameStat> {
    // Denser motion — every other frame moves hard. The detector should
    // produce strictly more boundaries than the slow fixture.
    (0..30)
        .map(|i| FrameStat {
            index: i as u64 * 4,
            histogram_delta_milli: if i % 2 == 0 { 800 } else { 0 },
        })
        .collect()
}

#[test]
fn scene_boundaries_are_stable_across_runs() {
    let detector = SceneDetector::new(SceneRefinement::default());
    let frames = slow_scene_frames();
    let a = detector.detect(&frames).expect("detector must accept slow scene frames");
    let b = detector.detect(&frames).expect("detector must accept slow scene frames on a second run");
    assert_eq!(a, b);
    for boundary in &a {
        let start = boundary.start_frame.0;
        let end = boundary.end_frame.0;
        assert!(
            frames.iter().any(|f| f.index == start),
            "scene boundary start frame {start} not present in source frames"
        );
        assert!(
            frames.iter().any(|f| f.index == end),
            "scene boundary end frame {end} not present in source frames"
        );
    }
}

#[test]
fn fast_cut_scene_produces_more_boundaries_than_slow_scene() {
    let detector = SceneDetector::new(SceneRefinement::default());
    let slow = detector.detect(&slow_scene_frames()).unwrap();
    let fast = detector.detect(&fast_cut_scene_frames()).unwrap();
    assert!(
        fast.len() > slow.len(),
        "fast-cut fixture should produce strictly more boundaries (slow={}, fast={})",
        slow.len(),
        fast.len()
    );
}

#[test]
fn shot_boundaries_are_stable_across_runs() {
    let detector = ShotDetector::new(ShotRefinement::default()).unwrap();
    let motion: Vec<MotionFrame> = (0..10)
        .map(|i| MotionFrame {
            index: i as u64 * 12,
            motion_delta_milli: if i % 3 == 0 { 900 } else { 0 },
        })
        .collect();
    let a = detector.detect(&motion).unwrap();
    let b = detector.detect(&motion).unwrap();
    assert_eq!(a, b);
    for shot in &a {
        assert!(motion.iter().any(|m| m.index == shot.start_frame.0));
        assert!(motion.iter().any(|m| m.index == shot.end_frame.0));
    }
}

#[test]
fn shots_align_to_source_frame_evidence() {
    let detector = ShotDetector::new(ShotRefinement::default()).unwrap();
    let motion: Vec<MotionFrame> = vec![
        MotionFrame { index: 0, motion_delta_milli: 0 },
        MotionFrame { index: 60, motion_delta_milli: 700 },
        MotionFrame { index: 120, motion_delta_milli: 0 },
        MotionFrame { index: 180, motion_delta_milli: 850 },
        MotionFrame { index: 240, motion_delta_milli: 0 },
    ];
    let shots = detector.detect(&motion).unwrap();
    assert!(shots.len() >= 2);
    for shot in &shots {
        assert!(motion.iter().any(|m| m.index == shot.start_frame.0));
        assert!(motion.iter().any(|m| m.index == shot.end_frame.0));
        assert!(shot.confidence_milli > 0);
    }
}

#[test]
fn shot_refinement_rejects_invalid_inputs() {
    let bad = ShotRefinement {
        coarse_stride_ms: 5,
        refine_stride_ms: 50,
        min_shot_duration_ms: 0,
        motion_threshold_milli: 100,
    };
    assert!(ShotDetector::new(bad).is_err());
    let zero_threshold = ShotRefinement { motion_threshold_milli: 0, ..ShotRefinement::default() };
    assert!(ShotDetector::new(zero_threshold).is_err());
}

#[test]
fn deterministic_fast_cut_yields_identical_output_across_repeated_calls() {
    // Stress the determinism guarantee by calling the detector 16 times
    // and asserting every output matches the first.
    let detector = ShotDetector::new(ShotRefinement::default()).unwrap();
    let motion: Vec<MotionFrame> = (0..64)
        .map(|i| MotionFrame {
            index: i as u64,
            motion_delta_milli: (i as i32 * 37) % 1000,
        })
        .collect();
    let baseline = detector.detect(&motion).unwrap();
    for _ in 0..16 {
        assert_eq!(detector.detect(&motion).unwrap(), baseline);
    }
}
