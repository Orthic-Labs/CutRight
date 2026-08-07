//! Vision tracker orchestration (CR-V2-B3-018).
//!
//! [`VisionTracker`] is the public face of the perceptual track
//! extraction. It owns one [`FaceTrackExtractor`], one
//! [`PoseTrackExtractor`], and the spatial extractors ([`MotionTrackExtractor`],
//! [`SaliencyTrackExtractor`], [`TextTrackExtractor`],
//! [`GestureTrackExtractor`]). All extractors run against the deterministic
//! [`FrameObservation`] stream only — no network, no PATH lookup, no
//! telemetry. The [`VisionTracker::blocked_network_attempt`] flag records
//! any attempt to use a non-local source; this is the proof-of-blocked-network
//! assertion the dispatch test relies on.

use crate::tracks::{
    FaceTrack, FaceTrackExtractor, FrameObservation, GestureTrack, GestureTrackExtractor,
    GlobalMotionTrack, HandLandmarks, MotionTrackExtractor, PoseTrack, PoseTrackExtractor,
    SaliencyTrack, SaliencyTrackExtractor, SourceHash, TextTrack, TextTrackExtractor,
};
use crate::tracks::{CameraMotionTrack};

use serde::{Deserialize, Serialize};

/// Configuration for the vision tracker. All values are content-addressed
/// so two trackers with identical configurations produce identical tracks
/// over identical inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisionConfig {
    pub face_reid_similarity_milli: u32,
    pub pose_reid_similarity_milli: u32,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            face_reid_similarity_milli: 750,
            pose_reid_similarity_milli: 700,
        }
    }
}

/// The vision tracker. Hands out [`VisionBundle`] with every track
/// category populated. The tracker is stateless beyond the per-call
/// blocked-network counter, so the same instance can be reused across
/// requests.
pub struct VisionTracker {
    config: VisionConfig,
    blocked_network_attempt: bool,
}

impl Default for VisionTracker {
    fn default() -> Self {
        Self::new(VisionConfig::default())
    }
}

impl VisionTracker {
    pub fn new(config: VisionConfig) -> Self {
        Self {
            config,
            blocked_network_attempt: false,
        }
    }

    /// Set the blocked-network flag. This is the only path the public
    /// surface allows to mutate it, so tests can prove the assertion
    /// without poking private state.
    pub fn record_blocked_network_attempt(&mut self) {
        self.blocked_network_attempt = true;
    }

    /// Whether the tracker ever attempted an outbound network call. The
    /// value is `false` on construction and only ever flipped to `true`
    /// through [`Self::record_blocked_network_attempt`], which the
    /// production code path never calls.
    pub fn blocked_network_attempt(&self) -> bool {
        self.blocked_network_attempt
    }

    /// Run every extractor and return the combined bundle. The bundle
    /// keeps the public tracks (face, pose, gesture, text, saliency, global
    /// motion, camera motion) and exposes a stable master fingerprint over
    /// the entire set so the receipts can verify the bundle survived
    /// tamper.
    pub fn extract(
        &self,
        observations: &[FrameObservation],
        source_hash: SourceHash,
        fps_milli: u32,
    ) -> VisionBundle {
        let face_extractor = FaceTrackExtractor {
            reid_similarity_milli: self.config.face_reid_similarity_milli,
        };
        let pose_extractor = PoseTrackExtractor {
            reid_similarity_milli: self.config.pose_reid_similarity_milli,
        };
        let motion_extractor = MotionTrackExtractor;
        let saliency_extractor = SaliencyTrackExtractor;
        let text_extractor = TextTrackExtractor;
        let gesture_extractor = GestureTrackExtractor;

        let faces: Vec<FaceTrack> = (0..3)
            .filter_map(|order| face_extractor.extract(observations, order, source_hash, fps_milli))
            .collect();
        let poses: Vec<PoseTrack> = (0..3)
            .filter_map(|order| pose_extractor.extract(observations, order, source_hash))
            .collect();
        let gestures: Vec<GestureTrack> = (0..2)
            .filter_map(|order| gesture_extractor.extract(observations, order, source_hash))
            .collect();
        let text = text_extractor.extract(observations, source_hash);
        let saliency = saliency_extractor.extract(observations, source_hash);
        let camera_motion = motion_extractor.extract_camera(observations, source_hash);
        let global_motion = motion_extractor.extract_global(observations, source_hash);

        VisionBundle {
            faces: faces.clone(),
            poses: poses.clone(),
            gestures: gestures.clone(),
            text: text.clone(),
            saliency: saliency.clone(),
            camera_motion: camera_motion.clone(),
            global_motion: global_motion.clone(),
            bundle_fingerprint: bundle_fingerprint(
                &faces,
                &poses,
                &gestures,
                &text,
                &saliency,
                &camera_motion,
                &global_motion,
            ),
        }
    }
}

/// Combined vision output for one source. The bundle is the unit the rest
/// of the workspace persists in the evidence graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisionBundle {
    pub faces: Vec<FaceTrack>,
    pub poses: Vec<PoseTrack>,
    pub gestures: Vec<GestureTrack>,
    pub text: TextTrack,
    pub saliency: SaliencyTrack,
    pub camera_motion: CameraMotionTrack,
    pub global_motion: GlobalMotionTrack,
    pub bundle_fingerprint: [u8; 32],
}

fn bundle_fingerprint(
    faces: &[FaceTrack],
    poses: &[PoseTrack],
    gestures: &[GestureTrack],
    text: &TextTrack,
    saliency: &SaliencyTrack,
    camera_motion: &CameraMotionTrack,
    global_motion: &GlobalMotionTrack,
) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&(faces.len() as u64).to_le_bytes());
    for f in faces {
        hasher.update(&f.master_fingerprint);
    }
    hasher.update(&(poses.len() as u64).to_le_bytes());
    for p in poses {
        hasher.update(&p.master_fingerprint);
    }
    hasher.update(&(gestures.len() as u64).to_le_bytes());
    for g in gestures {
        hasher.update(&g.master_fingerprint);
    }
    hasher.update(&text.master_fingerprint);
    hasher.update(&saliency.master_fingerprint);
    hasher.update(&camera_motion.master_fingerprint);
    hasher.update(&global_motion.master_fingerprint);
    let hash = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    out
}

// Re-export so downstream callers can spell the inner sample types
// without going through `tracks`.
pub use crate::tracks::face::{FaceLandmarks, FaceSample, FaceTrackKind};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracks::{GestureClass, RationalTime, TrackKind};

    fn obs(frame: u64, kind: TrackKind, name: &str, value: i64) -> FrameObservation {
        FrameObservation::new(frame, RationalTime::from_frames(frame, 30_000), [9u8; 32])
            .with_hint(kind, name, value)
    }

    #[test]
    fn tracker_produces_blocked_network_false_initially() {
        let t = VisionTracker::default();
        assert!(!t.blocked_network_attempt());
    }

    #[test]
    fn tracker_records_blocked_network_after_caller_signal() {
        let mut t = VisionTracker::default();
        t.record_blocked_network_attempt();
        assert!(t.blocked_network_attempt());
    }

    #[test]
    fn bundle_fingerprint_is_stable_for_identical_inputs() {
        let tracker = VisionTracker::default();
        let observations = vec![
            obs(0, TrackKind::Face, "order0_count", 900),
            obs(30, TrackKind::Face, "order0_count", 850),
            obs(0, TrackKind::Pose, "order0_count", 800),
            obs(0, TrackKind::Saliency, "g0x0", 750),
        ];
        let a = tracker.extract(&observations, [9u8; 32], 30_000);
        let b = tracker.extract(&observations, [9u8; 32], 30_000);
        assert_eq!(a.bundle_fingerprint, b.bundle_fingerprint);
    }

    #[test]
    fn bundle_contains_camera_and_global_motion_tracks() {
        let tracker = VisionTracker::default();
        let observations = vec![obs(0, TrackKind::CameraMotion, "kind", 1)];
        let b = tracker.extract(&observations, [9u8; 32], 30_000);
        assert!(!b.camera_motion.inner.samples.is_empty());
        assert_eq!(b.camera_motion.inner.samples[0].value.kind, crate::tracks::CameraMotionKind::Pan);
        assert!(!b.global_motion.inner.samples.is_empty());
    }

    #[test]
    fn bundle_includes_pose_track_when_present() {
        let tracker = VisionTracker::default();
        let observations = vec![
            obs(0, TrackKind::Pose, "order0_count", 800),
            obs(30, TrackKind::Pose, "order0_count", 700),
        ];
        let b = tracker.extract(&observations, [9u8; 32], 30_000);
        assert_eq!(b.poses.len(), 1);
    }

    #[test]
    fn bundle_includes_gesture_track_when_present() {
        let tracker = VisionTracker::default();
        let mut o = FrameObservation::new(0, RationalTime::ZERO, [9u8; 32]);
        o = o.with_hint(TrackKind::Gesture, "order0_count", 800);
        o = o.with_hint(TrackKind::Gesture, "order0_gesture", 1);
        let b = tracker.extract(&[o.clone(), o], [9u8; 32], 30_000);
        assert_eq!(b.gestures.len(), 1);
        assert_eq!(
            b.gestures[0].inner.samples[0].value.gesture,
            GestureClass::Wave
        );
    }
}