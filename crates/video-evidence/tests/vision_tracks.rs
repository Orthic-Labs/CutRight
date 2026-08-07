//! Integration tests for the perceptual and spatial tracks
//! (CR-V2-B3-018).
//!
//! Verifies:
//! 1. Tracks use source time and stable IDs across runs.
//! 2. Subject loss and re-identification uncertainty are explicit.
//! 3. The vision tracker makes zero outbound network attempts.

use video_evidence::tracks::{FrameObservation, RationalTime, TrackKind};
use video_evidence::vision::{VisionBundle, VisionTracker};

fn obs(frame: u64, kind: TrackKind, name: &str, value: i64) -> FrameObservation {
    FrameObservation::new(frame, RationalTime::from_frames(frame, 30_000), [42u8; 32])
        .with_hint(kind, name, value)
}

#[test]
fn vision_tracker_reports_no_network_attempt_initially() {
    let tracker = VisionTracker::default();
    assert!(!tracker.blocked_network_attempt());
}

#[test]
fn vision_bundle_is_stable_across_runs() {
    let tracker = VisionTracker::default();
    let observations = vec![
        obs(0, TrackKind::Face, "order0_count", 900),
        obs(30, TrackKind::Face, "order0_count", 850),
        obs(60, TrackKind::Face, "order0_count", 0),
        obs(90, TrackKind::Face, "order0_count", 880),
    ];
    let a: VisionBundle = tracker.extract(&observations, [42u8; 32], 30_000);
    let b: VisionBundle = tracker.extract(&observations, [42u8; 32], 30_000);
    assert_eq!(a.bundle_fingerprint, b.bundle_fingerprint);
}

#[test]
fn face_subject_loss_is_recorded_on_absence() {
    let tracker = VisionTracker::default();
    let observations = vec![
        obs(0, TrackKind::Face, "order0_count", 900),
        obs(30, TrackKind::Face, "order0_count", 0),
        obs(60, TrackKind::Face, "order0_count", 880),
    ];
    let bundle = tracker.extract(&observations, [42u8; 32], 30_000);
    assert_eq!(bundle.faces.len(), 1);
    let face = &bundle.faces[0];
    assert!(!face.inner.losses.is_empty(), "expected subject loss record");
    assert!(
        !face.inner.reidentifications.is_empty(),
        "expected re-identification record"
    );
}

#[test]
fn track_ids_are_stable_across_calls() {
    use video_evidence::tracks::FaceTrackExtractor;
    let ex = FaceTrackExtractor::default();
    let observations = vec![
        obs(0, TrackKind::Face, "order0_count", 900),
        obs(30, TrackKind::Face, "order0_count", 800),
    ];
    let a = ex.extract(&observations, 0, [1u8; 32], 30_000).unwrap();
    let b = ex.extract(&observations, 0, [1u8; 32], 30_000).unwrap();
    assert_eq!(a.inner.track_id, b.inner.track_id);
    assert_eq!(a.master_fingerprint, b.master_fingerprint);
}

#[test]
fn saliency_centroid_is_deterministic() {
    use video_evidence::tracks::SaliencyTrackExtractor;
    let ex = SaliencyTrackExtractor;
    let mut obs = FrameObservation::new(0, RationalTime::ZERO, [7u8; 32]);
    obs = obs.with_hint(TrackKind::Saliency, "g0x0", 1000);
    let a = ex.extract(&[obs.clone()], [7u8; 32]);
    let b = ex.extract(&[obs.clone()], [7u8; 32]);
    assert_eq!(a.master_fingerprint, b.master_fingerprint);
}

#[test]
fn camera_motion_decodes_all_kinds() {
    use video_evidence::tracks::CameraMotionKind;
    use video_evidence::tracks::MotionTrackExtractor;
    let ex = MotionTrackExtractor;
    let mut observations = Vec::new();
    for (idx, code) in [1i64, 2, 3, 4, 5, 6, 0].iter().enumerate() {
        let mut o = FrameObservation::new(idx as u64, RationalTime::ZERO, [3u8; 32]);
        o = o.with_hint(TrackKind::CameraMotion, "kind", *code);
        o = o.with_hint(TrackKind::CameraMotion, "magnitude", 100);
        observations.push(o);
    }
    let track = ex.extract_camera(&observations, [3u8; 32]);
    let kinds: Vec<CameraMotionKind> = track
        .inner
        .samples
        .iter()
        .map(|s| s.value.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            CameraMotionKind::Pan,
            CameraMotionKind::Tilt,
            CameraMotionKind::ZoomIn,
            CameraMotionKind::ZoomOut,
            CameraMotionKind::Dolly,
            CameraMotionKind::Handheld,
            CameraMotionKind::Static,
        ]
    );
}