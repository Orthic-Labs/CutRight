//! Face detection and landmark tracks (CR-V2-B3-018).
//!
//! The extractor consumes deterministic [`FrameObservation`] inputs and
//! emits a face track for every stable subject. Re-identification evidence
//! is recorded when a previously-lost subject reappears. No outbound
//! network attempt is made; the extractor is wired to the local vision
//! pack only.

use serde::{Deserialize, Serialize};

use super::{
    build_track, fingerprint_value, FrameObservation, LossReason, ReIdentificationEvidence,
    SubjectLoss, TimedSample, TrackKind, TrackLossReidentification, TrackMaster,
};

/// Helper for pulling an i32 hint from a frame observation. The hint is
/// stored as `i64` to match the rest of the extractor surface; the cast
/// clamps into the i32 range so a malformed fixture cannot crash the
/// extractor.
fn i32_hint(obs: &FrameObservation, kind: TrackKind, name: &str) -> i32 {
    obs.hint(kind, name).unwrap_or(0).clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// Same as [`i32_hint`] but for non-negative millis values.
fn u32_hint(obs: &FrameObservation, kind: TrackKind, name: &str) -> u32 {
    obs.hint(kind, name).unwrap_or(0).max(0) as u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FaceTrackKind {
    /// Adult primary face — the dominant subject.
    Primary,
    /// Secondary face (background subject, interviewer, etc).
    Secondary,
    /// Crowd-detected face without re-identification.
    Crowd,
}

/// Detected face landmarks. Coordinates are integer millis in the source
/// frame's normalized 0..=1000 box, so the structure stays portable across
/// resolutions without lossy float math.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FaceLandmarks {
    pub left_eye_x_milli: i32,
    pub left_eye_y_milli: i32,
    pub right_eye_x_milli: i32,
    pub right_eye_y_milli: i32,
    pub nose_x_milli: i32,
    pub nose_y_milli: i32,
    pub mouth_x_milli: i32,
    pub mouth_y_milli: i32,
    pub bbox_x_milli: i32,
    pub bbox_y_milli: i32,
    pub bbox_w_milli: i32,
    pub bbox_h_milli: i32,
    pub confidence_milli: u32,
}

impl FaceLandmarks {
    /// Stable identity hash for this face observation. The hash is content
    /// derived (BLAKE3 over the canonicalised landmark payload) so the same
    /// physical face on a different frame index yields the same identity.
    pub fn identity_fingerprint(&self) -> [u8; 32] {
        fingerprint_value(self)
    }
}

/// One face observation in source time.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FaceSample {
    pub landmarks: FaceLandmarks,
    pub kind: FaceTrackKind,
}

/// Extracts face tracks from a sequence of deterministic frame observations.
/// The extractor is stateless beyond its configuration; identical inputs
/// always yield identical tracks.
pub struct FaceTrackExtractor {
    /// Minimum similarity (millis) required to re-identify a face.
    pub reid_similarity_milli: u32,
}

impl Default for FaceTrackExtractor {
    fn default() -> Self {
        Self {
            reid_similarity_milli: 750,
        }
    }
}

impl FaceTrackExtractor {
    /// Build a [`FaceTrack`] for the requested subject order (0=primary,
    /// 1=secondary, etc.). The extractor walks the observations in
    /// deterministic order.
    pub fn extract(
        &self,
        observations: &[FrameObservation],
        order: u32,
        source_hash: [u8; 32],
        fps_milli: u32,
    ) -> Option<FaceTrack> {
        let mut samples = Vec::new();
        let mut reids = Vec::new();
        let mut losses = Vec::new();
        let mut last_face_id: Option<[u8; 32]> = None;
        let mut absent_streak: u32 = 0;
        for obs in observations {
            let count_milli = obs.hint(TrackKind::Face, &format!("order{order}_count"));
            let Some(count_milli) = count_milli else {
                absent_streak = absent_streak.saturating_add(1);
                continue;
            };
            if count_milli == 0 {
                absent_streak = absent_streak.saturating_add(1);
                continue;
            }
            let kind = match order {
                0 => FaceTrackKind::Primary,
                1 => FaceTrackKind::Secondary,
                _ => FaceTrackKind::Crowd,
            };
            let landmarks = FaceLandmarks {
                left_eye_x_milli: i32_hint(obs, TrackKind::Face, &format!("order{order}_left_eye_x")),
                left_eye_y_milli: i32_hint(obs, TrackKind::Face, &format!("order{order}_left_eye_y")),
                right_eye_x_milli: i32_hint(obs, TrackKind::Face, &format!("order{order}_right_eye_x")),
                right_eye_y_milli: i32_hint(obs, TrackKind::Face, &format!("order{order}_right_eye_y")),
                nose_x_milli: i32_hint(obs, TrackKind::Face, &format!("order{order}_nose_x")),
                nose_y_milli: i32_hint(obs, TrackKind::Face, &format!("order{order}_nose_y")),
                mouth_x_milli: i32_hint(obs, TrackKind::Face, &format!("order{order}_mouth_x")),
                mouth_y_milli: i32_hint(obs, TrackKind::Face, &format!("order{order}_mouth_y")),
                bbox_x_milli: i32_hint(obs, TrackKind::Face, &format!("order{order}_bbox_x")),
                bbox_y_milli: i32_hint(obs, TrackKind::Face, &format!("order{order}_bbox_y")),
                bbox_w_milli: i32_hint(obs, TrackKind::Face, &format!("order{order}_bbox_w")),
                bbox_h_milli: i32_hint(obs, TrackKind::Face, &format!("order{order}_bbox_h")),
                confidence_milli: u32_hint(obs, TrackKind::Face, &format!("order{order}_conf")),
            };
            let fp = landmarks.identity_fingerprint();
            // Re-identification: if we had a face, lost it, then a new face
            // appears with high similarity to the lost one, record it.
            if absent_streak > 0 {
                if let Some(prev) = last_face_id {
                    let similarity = similarity_milli(&fp, &prev);
                    if similarity >= self.reid_similarity_milli {
                        reids.push(ReIdentificationEvidence {
                            previous_track_id: format!("face/{order:08x}"),
                            similarity_milli: similarity,
                            confidence_milli: landmarks.confidence_milli,
                            at: obs.timestamp,
                        });
                    }
                    losses.push(SubjectLoss {
                        at: obs.timestamp,
                        last_track_id: format!("face/{order:08x}"),
                        reason: LossReason::Occlusion,
                        confidence_milli: landmarks.confidence_milli,
                    });
                }
            }
            absent_streak = 0;
            last_face_id = Some(fp);
            samples.push(TimedSample::new(
                obs.source_frame,
                obs.timestamp,
                FaceSample { landmarks, kind },
                fp,
            ));
        }
        if samples.is_empty() {
            return None;
        }
        let track_id = super::make_track_id(&source_hash, TrackKind::Face, order);
        let master = super::master_fingerprint(&track_id, &samples, &reids, &losses);
        Some(FaceTrack {
            inner: build_track(track_id, TrackKind::Face, source_hash, samples, reids, losses),
            master_fingerprint: master,
            fps_milli,
        })
    }
}

/// Stable similarity between two face identity digests. Counts the
/// matching bytes; identical digests collapse to 1000.
fn similarity_milli(a: &[u8; 32], b: &[u8; 32]) -> u32 {
    let matching = a.iter().zip(b.iter()).filter(|(x, y)| x == y).count() as u32;
    (matching.saturating_mul(1000)) / 32
}

/// A face track with the supporting bookkeeping fields the extractor keeps
/// alongside.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaceTrack {
    #[serde(flatten)]
    pub inner: super::TemporalTrack<FaceSample>,
    pub master_fingerprint: [u8; 32],
    pub fps_milli: u32,
}

impl TrackMaster for FaceTrack {
    fn master(&self) -> &[u8; 32] {
        &self.master_fingerprint
    }
}

impl TrackLossReidentification for FaceTrack {
    fn reidentifications(&self) -> &[ReIdentificationEvidence] {
        &self.inner.reidentifications
    }
    fn losses(&self) -> &[SubjectLoss] {
        &self.inner.losses
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracks::RationalTime;

    fn obs(frame: u64, order: u32, bbox_w: i64, conf: i64) -> FrameObservation {
        let mut o = FrameObservation::new(frame, RationalTime::from_frames(frame, 30_000), [1u8; 32]);
        o = o.with_hint(TrackKind::Face, &format!("order{order}_count"), conf);
        o = o.with_hint(TrackKind::Face, &format!("order{order}_bbox_w"), bbox_w);
        o = o.with_hint(TrackKind::Face, &format!("order{order}_left_eye_x"), 100);
        o
    }

    #[test]
    fn face_track_is_deterministic_for_same_observations() {
        let ex = FaceTrackExtractor::default();
        let obs = vec![obs(0, 0, 100, 900), obs(30, 0, 110, 850)];
        let a = ex.extract(&obs, 0, [1u8; 32], 30_000).unwrap();
        let b = ex.extract(&obs, 0, [1u8; 32], 30_000).unwrap();
        assert_eq!(a.inner.samples, b.inner.samples);
        assert_eq!(a.master_fingerprint, b.master_fingerprint);
    }

    #[test]
    fn face_track_is_none_when_subject_never_present() {
        let ex = FaceTrackExtractor::default();
        let empty: Vec<FrameObservation> = vec![];
        assert!(ex.extract(&empty, 0, [0u8; 32], 30_000).is_none());
    }

    #[test]
    fn similarity_is_1000_for_identical_digests() {
        let h = [7u8; 32];
        assert_eq!(similarity_milli(&h, &h), 1000);
    }
}