// Integration tests for visual preservation, crop stability, and
// collision evaluators (Book 4 lane A, B4-009).

use video_benchmarks::collision::{box_overlap_ratio, detect_collisions, Box2D};
use video_benchmarks::crop::{crop_acceleration, crop_jerk, CropSample};
use video_benchmarks::visual::{
    evaluate_visual, label_retention, non_target_similarity, subject_retention,
    FrameSample, LabelRetentionEvaluator, OcrLabel, SubjectRef, SubjectRetentionEvaluator,
    NonTargetFrameSimilarityEvaluator,
};
use video_benchmarks::{BenchmarkEvaluator, EvalContext};

fn ctx() -> EvalContext {
    EvalContext::default()
}

#[test]
fn visual_non_target_similarity_pass_no_action() {
    let frames = vec![
        FrameSample {
            frame_id: "a".into(),
            t_ms: 0,
            source_ssim: 1.0,
            output_ssim: 0.99,
            declared_action: false,
        },
        FrameSample {
            frame_id: "b".into(),
            t_ms: 1000,
            source_ssim: 1.0,
            output_ssim: 0.97,
            declared_action: false,
        },
    ];
    let sim = non_target_similarity(&frames);
    assert!(sim > 0.95);
}

#[test]
fn visual_subject_retention_drops_missing_subjects() {
    let subjects = vec![
        SubjectRef {
            subject_id: "s1".into(),
            label: "alice".into(),
            frames_present_ms: vec![0, 100, 200],
            frames_present_output_ms: vec![0, 100, 200],
            declared_action: false,
        },
        SubjectRef {
            subject_id: "s2".into(),
            label: "bob".into(),
            frames_present_ms: vec![300, 400],
            frames_present_output_ms: vec![],
            declared_action: false,
        },
    ];
    let r = subject_retention(&subjects);
    assert!((r - 0.5).abs() < 1e-5);
}

#[test]
fn visual_label_retention_handles_empty() {
    assert_eq!(label_retention(&[]), 1.0);
}

#[test]
fn visual_evaluator_returns_pass() {
    let e = NonTargetFrameSimilarityEvaluator;
    let out = e.evaluate(&ctx()).expect("ok");
    assert_eq!(out.metric_id, "visual.frame_similarity.non_target");
}

#[test]
fn subject_retention_evaluator_id() {
    let e = SubjectRetentionEvaluator;
    assert_eq!(e.id(), "visual.subject_retention");
}

#[test]
fn label_retention_evaluator_id() {
    let e = LabelRetentionEvaluator;
    assert_eq!(e.id(), "visual.label_retention");
}

#[test]
fn visual_full_evaluate_call() {
    let frames = vec![FrameSample {
        frame_id: "f".into(),
        t_ms: 0,
        source_ssim: 1.0,
        output_ssim: 1.0,
        declared_action: false,
    }];
    let subjects = vec![SubjectRef {
        subject_id: "s".into(),
        label: "x".into(),
        frames_present_ms: vec![0],
        frames_present_output_ms: vec![0],
        declared_action: false,
    }];
    let labels = vec![OcrLabel {
        label_id: "l".into(),
        text: "x".into(),
        source_frame_id: "src".into(),
        output_frame_id: Some("out".into()),
        declared_action: false,
    }];
    let r = evaluate_visual(&frames, &subjects, &labels);
    assert_eq!(r.frames.len(), 1);
    assert_eq!(r.subjects.len(), 1);
    assert_eq!(r.labels.len(), 1);
}

#[test]
fn crop_jerk_constant_is_zero() {
    let samples: Vec<CropSample> = (0..10)
        .map(|i| CropSample {
            t_ms: i * 100,
            x: 0.5,
            y: 0.5,
            w: 0.4,
            h: 0.4,
            declared_action: false,
        })
        .collect();
    assert_eq!(crop_jerk(&samples), 0.0);
    assert_eq!(crop_acceleration(&samples), 0.0);
}

#[test]
fn crop_jerk_short_input_safe() {
    let samples = vec![CropSample {
        t_ms: 0,
        x: 0.5,
        y: 0.5,
        w: 0.4,
        h: 0.4,
        declared_action: false,
    }];
    assert_eq!(crop_jerk(&samples), 0.0);
}

#[test]
fn collision_overlap_ratio_full() {
    let a = Box2D { t_ms: 0, x: 0.0, y: 0.0, w: 0.5, h: 0.5 };
    let b = Box2D { t_ms: 0, x: 0.0, y: 0.0, w: 0.5, h: 0.5 };
    assert!((box_overlap_ratio(&a, &b) - 1.0).abs() < 1e-5);
}

#[test]
fn collision_detect_threshold() {
    let overlay = vec![Box2D { t_ms: 0, x: 0.0, y: 0.0, w: 0.5, h: 0.5 }];
    let subject = vec![Box2D { t_ms: 0, x: 0.4, y: 0.4, w: 0.5, h: 0.5 }];
    let events = detect_collisions("cap", &overlay, "face", &subject, 0.05);
    assert!(!events.is_empty());
}