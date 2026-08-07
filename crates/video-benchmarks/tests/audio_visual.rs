
use video_benchmarks::audio::{evaluate_audio, AudioSample, AudioPreservationEvaluator};
use video_benchmarks::audio_visual::{
    evaluate_av_sync, evaluate_audio_preservation, joint_drift, AudioPreservationSample,
    AvDriftEvaluator, TransientAlignmentEvaluator, TruePeakEvaluator,
};
use video_benchmarks::{BenchmarkEvaluator, EvalContext, MetricStatus};

fn sample(label: &str, lufs: f64, peak: f64) -> AudioSample {
    AudioSample {
        label: label.to_string(),
        loudness_lufs: lufs,
        true_peak_dbtp: peak,
        channel_layout: "stereo".to_string(),
    }
}

fn preservation_sample(label: &str, target_db: f64, output_db: f64, declared: bool) -> AudioPreservationSample {
    AudioPreservationSample {
        label: label.to_string(),
        target_db,
        output_db,
        declared_action: declared,
    }
}

#[test]
fn joint_drift_sums_components() {
    let components = vec![
        video_benchmarks::audio_visual::SyncComponent { label: "container_pts_delta".into(), delta_ms: 8 },
        video_benchmarks::audio_visual::SyncComponent { label: "transient_alignment_delta".into(), delta_ms: 12 },
    ];
    assert_eq!(joint_drift(&components), 20);
}

#[test]
fn av_sync_includes_lipsync_proxy_separately() {
    let result = evaluate_av_sync(10, 5, Some(0.7));
    assert_eq!(result.joint_drift_ms, 15);
    assert_eq!(result.lip_sync_proxy_confidence, Some(0.7));
}

#[test]
fn declared_fade_is_not_flagged_as_discontinuity() {
    let samples = vec![
        sample("speech", -23.0, -2.0),
        sample("fade:out", -30.0, -10.0),
        sample("speech", -23.0, -2.0),
    ];
    let result = evaluate_audio(&samples);
    assert_eq!(result.discontinuity_count, 0);
}

#[test]
fn unmarked_discontinuity_is_detected() {
    let samples = vec![sample("speech", -23.0, -2.0), sample("speech", -10.0, -2.0)];
    let result = evaluate_audio(&samples);
    assert_eq!(result.discontinuity_count, 1);
}

#[test]
fn clipping_counts_only_undeclared_above_minus_one_dbtp() {
    let samples = vec![sample("speech", -23.0, -0.5), sample("fade:out", -23.0, 0.0)];
    let result = evaluate_audio(&samples);
    assert_eq!(result.clipping_count, 1);
}

#[test]
fn preservation_evaluator_distinguishes_declared_target_fades() {
    let samples = vec![
        preservation_sample("speech", -23.0, -23.0, false),
        preservation_sample("fade", -20.0, -30.0, true),
        preservation_sample("speech", -23.0, -23.0, false),
    ];
    let result = evaluate_audio_preservation(&samples);
    assert_eq!(result.discontinuity_count, 0);
    assert!(result.true_peak_dbtp <= -1.0);
}

#[test]
fn evaluators_have_stable_identifiers() {
    assert_eq!(AvDriftEvaluator.id(), "audio_visual.drift_ms");
    assert_eq!(TransientAlignmentEvaluator.id(), "audio_visual.transient_alignment_ms");
    assert_eq!(TruePeakEvaluator.id(), "audio_visual.clipping.true_peak_dbtp");
    assert_eq!(AudioPreservationEvaluator.id(), "audio_visual.audio_preservation");
}

#[test]
fn evaluators_return_pass_for_empty_context() {
    let ev = AvDriftEvaluator;
    let outcome = ev.evaluate(&EvalContext::default()).expect("ok");
    assert_eq!(outcome.status, MetricStatus::Pass);
}
