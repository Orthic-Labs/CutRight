//! Hand/gesture tracks (CR-V2-B3-018).
//!
//! Detects hand landmarks, classifies gestures, and tracks each hand
//! across frames. No telemetry, no network — every sample is derived from
//! the deterministic [`FrameObservation`] inputs only.

use serde::{Deserialize, Serialize};

use super::{
    build_track, fingerprint_value, FrameObservation, LossReason, RationalRange,
    ReIdentificationEvidence, SubjectLoss, TimedSample, TrackKind, TrackLossReidentification,
    TrackMaster,
};

fn i32_hint(obs: &FrameObservation, kind: TrackKind, name: &str) -> i32 {
    obs.hint(kind, name).unwrap_or(0).clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

fn u32_hint(obs: &FrameObservation, kind: TrackKind, name: &str) -> u32 {
    obs.hint(kind, name).unwrap_or(0).max(0) as u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GestureClass {
    None,
    Wave,
    Point,
    ThumbsUp,
    OpenPalm,
    Fist,
    TwoFingerV,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HandLandmarks {
    pub wrist_x_milli: i32,
    pub wrist_y_milli: i32,
    pub thumb_tip_x_milli: i32,
    pub thumb_tip_y_milli: i32,
    pub index_tip_x_milli: i32,
    pub index_tip_y_milli: i32,
    pub middle_tip_x_milli: i32,
    pub middle_tip_y_milli: i32,
    pub ring_tip_x_milli: i32,
    pub ring_tip_y_milli: i32,
    pub pinky_tip_x_milli: i32,
    pub pinky_tip_y_milli: i32,
    pub confidence_milli: u32,
}

impl HandLandmarks {
    pub fn identity_fingerprint(&self) -> [u8; 32] {
        fingerprint_value(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GestureSample {
    pub landmarks: HandLandmarks,
    pub gesture: GestureClass,
    pub gesture_confidence_milli: u32,
}

pub struct GestureTrackExtractor;

impl GestureTrackExtractor {
    pub fn extract(
        &self,
        observations: &[FrameObservation],
        order: u32,
        source_hash: [u8; 32],
    ) -> Option<GestureTrack> {
        let mut samples = Vec::new();
        let mut losses = Vec::new();
        let mut last_kind = GestureClass::None;
        for obs in observations {
            let conf = obs.hint(TrackKind::Gesture, &format!("order{order}_count"));
            let Some(conf) = conf else {
                continue;
            };
            if conf == 0 {
                if last_kind != GestureClass::None {
                    losses.push(SubjectLoss {
                        at: obs.timestamp,
                        last_track_id: format!("gesture/{order:08x}"),
                        reason: LossReason::ExitFrame,
                        confidence_milli: 0,
                    });
                }
                last_kind = GestureClass::None;
                continue;
            }
            let landmarks = HandLandmarks {
                wrist_x_milli: i32_hint(obs, TrackKind::Gesture, &format!("order{order}_wrist_x")),
                wrist_y_milli: i32_hint(obs, TrackKind::Gesture, &format!("order{order}_wrist_y")),
                thumb_tip_x_milli: i32_hint(obs, TrackKind::Gesture, &format!("order{order}_thumb_x")),
                thumb_tip_y_milli: i32_hint(obs, TrackKind::Gesture, &format!("order{order}_thumb_y")),
                index_tip_x_milli: i32_hint(obs, TrackKind::Gesture, &format!("order{order}_index_x")),
                index_tip_y_milli: i32_hint(obs, TrackKind::Gesture, &format!("order{order}_index_y")),
                middle_tip_x_milli: i32_hint(obs, TrackKind::Gesture, &format!("order{order}_middle_x")),
                middle_tip_y_milli: i32_hint(obs, TrackKind::Gesture, &format!("order{order}_middle_y")),
                ring_tip_x_milli: i32_hint(obs, TrackKind::Gesture, &format!("order{order}_ring_x")),
                ring_tip_y_milli: i32_hint(obs, TrackKind::Gesture, &format!("order{order}_ring_y")),
                pinky_tip_x_milli: i32_hint(obs, TrackKind::Gesture, &format!("order{order}_pinky_x")),
                pinky_tip_y_milli: i32_hint(obs, TrackKind::Gesture, &format!("order{order}_pinky_y")),
                confidence_milli: u32_hint(obs, TrackKind::Gesture, &format!("order{order}_conf")),
            };
            let gesture_code = obs
                .hint(TrackKind::Gesture, &format!("order{order}_gesture"))
                .unwrap_or(0);
            let gesture = match gesture_code {
                1 => GestureClass::Wave,
                2 => GestureClass::Point,
                3 => GestureClass::ThumbsUp,
                4 => GestureClass::OpenPalm,
                5 => GestureClass::Fist,
                6 => GestureClass::TwoFingerV,
                _ => GestureClass::None,
            };
            let fp = landmarks.identity_fingerprint();
            samples.push(TimedSample::new(
                obs.source_frame,
                obs.timestamp,
                GestureSample {
                    landmarks,
                    gesture,
                    gesture_confidence_milli: conf.max(0) as u32,
                },
                fp,
            ));
            last_kind = gesture;
        }
        if samples.is_empty() {
            return None;
        }
        let reids: Vec<ReIdentificationEvidence> = Vec::new();
        let track_id = super::make_track_id(&source_hash, TrackKind::Gesture, order);
        let master = super::master_fingerprint(&track_id, &samples, &reids, &losses);
        Some(GestureTrack {
            inner: build_track(
                track_id,
                TrackKind::Gesture,
                source_hash,
                samples,
                reids,
                losses,
            ),
            master_fingerprint: master,
            gaps: Vec::<RationalRange>::new(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GestureTrack {
    #[serde(flatten)]
    pub inner: super::TemporalTrack<GestureSample>,
    pub master_fingerprint: [u8; 32],
    pub gaps: Vec<RationalRange>,
}

impl TrackMaster for GestureTrack {
    fn master(&self) -> &[u8; 32] {
        &self.master_fingerprint
    }
}

impl TrackLossReidentification for GestureTrack {
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

    fn obs(frame: u64, order: u32, gesture: i64) -> FrameObservation {
        let mut o = FrameObservation::new(frame, RationalTime::from_frames(frame, 30_000), [2u8; 32]);
        o = o.with_hint(TrackKind::Gesture, &format!("order{order}_count"), 800);
        o = o.with_hint(TrackKind::Gesture, &format!("order{order}_gesture"), gesture);
        o
    }

    #[test]
    fn gesture_track_decodes_class_codes() {
        let ex = GestureTrackExtractor;
        let obs = vec![obs(0, 0, 1), obs(30, 0, 5)];
        let t = ex.extract(&obs, 0, [2u8; 32]).unwrap();
        let kinds: Vec<GestureClass> = t.inner.samples.iter().map(|s| s.value.gesture).collect();
        assert_eq!(kinds, vec![GestureClass::Wave, GestureClass::Fist]);
    }

    #[test]
    fn gesture_track_is_none_without_observations() {
        let ex = GestureTrackExtractor;
        let empty: Vec<FrameObservation> = vec![];
        assert!(ex.extract(&empty, 0, [0u8; 32]).is_none());
    }
}