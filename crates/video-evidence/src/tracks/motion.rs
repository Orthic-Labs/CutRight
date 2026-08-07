//! Global and camera motion tracks (CR-V2-B3-018).
//!
//! Walks the same deterministic [`FrameObservation`] inputs the perceptual
//! extractors use, derives per-frame motion deltas, and emits two
//! parallel tracks: one for global scene motion and one for the
//! classified camera-motion kind (pan, tilt, zoom, dolly, static).
//! Numeric values are millis so the extractor stays portable and the
//! fingerprint stays stable.

use serde::{Deserialize, Serialize};

use super::{
    build_track, FrameObservation, ReIdentificationEvidence, SubjectLoss, TimedSample, TrackKind,
    TrackMaster,
};

fn i32_hint(obs: &FrameObservation, kind: TrackKind, name: &str) -> i32 {
    obs.hint(kind, name).unwrap_or(0).clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

fn u32_hint(obs: &FrameObservation, kind: TrackKind, name: &str) -> u32 {
    obs.hint(kind, name).unwrap_or(0).max(0) as u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CameraMotionKind {
    Static,
    Pan,
    Tilt,
    ZoomIn,
    ZoomOut,
    Dolly,
    Handheld,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CameraMotionSample {
    pub kind: CameraMotionKind,
    pub magnitude_milli: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GlobalMotionSample {
    pub translation_x_milli: i32,
    pub translation_y_milli: i32,
    pub scale_milli: i32,
    pub rotation_milli: i32,
    pub confidence_milli: u32,
}

pub struct MotionTrackExtractor;

impl MotionTrackExtractor {
    pub fn extract_camera(
        &self,
        observations: &[FrameObservation],
        source_hash: [u8; 32],
    ) -> CameraMotionTrack {
        let mut samples = Vec::new();
        for obs in observations {
            let kind_code = obs.hint(TrackKind::CameraMotion, "kind").unwrap_or(0);
            let magnitude = u32_hint(obs, TrackKind::CameraMotion, "magnitude");
            let kind = match kind_code {
                1 => CameraMotionKind::Pan,
                2 => CameraMotionKind::Tilt,
                3 => CameraMotionKind::ZoomIn,
                4 => CameraMotionKind::ZoomOut,
                5 => CameraMotionKind::Dolly,
                6 => CameraMotionKind::Handheld,
                _ => CameraMotionKind::Static,
            };
            let fp = super::fingerprint_value(&kind);
            samples.push(TimedSample::new(
                obs.source_frame,
                obs.timestamp,
                CameraMotionSample { kind, magnitude_milli: magnitude },
                fp,
            ));
        }
        let losses = Vec::<SubjectLoss>::new();
        let reids = Vec::<ReIdentificationEvidence>::new();
        let track_id = super::make_track_id(&source_hash, TrackKind::CameraMotion, 0);
        let master = super::master_fingerprint(&track_id, &samples, &reids, &losses);
        CameraMotionTrack {
            inner: build_track(
                track_id,
                TrackKind::CameraMotion,
                source_hash,
                samples,
                reids,
                losses,
            ),
            master_fingerprint: master,
        }
    }

    pub fn extract_global(
        &self,
        observations: &[FrameObservation],
        source_hash: [u8; 32],
    ) -> GlobalMotionTrack {
        let mut samples = Vec::new();
        for obs in observations {
            let value = GlobalMotionSample {
                translation_x_milli: i32_hint(obs, TrackKind::GlobalMotion, "tx"),
                translation_y_milli: i32_hint(obs, TrackKind::GlobalMotion, "ty"),
                scale_milli: i32_hint(obs, TrackKind::GlobalMotion, "scale"),
                rotation_milli: i32_hint(obs, TrackKind::GlobalMotion, "rot"),
                confidence_milli: u32_hint(obs, TrackKind::GlobalMotion, "confidence"),
            };
            let fp = super::fingerprint_value(&value);
            samples.push(TimedSample::new(obs.source_frame, obs.timestamp, value, fp));
        }
        let losses = Vec::<SubjectLoss>::new();
        let reids = Vec::<ReIdentificationEvidence>::new();
        let track_id = super::make_track_id(&source_hash, TrackKind::GlobalMotion, 0);
        let master = super::master_fingerprint(&track_id, &samples, &reids, &losses);
        GlobalMotionTrack {
            inner: build_track(
                track_id,
                TrackKind::GlobalMotion,
                source_hash,
                samples,
                reids,
                losses,
            ),
            master_fingerprint: master,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraMotionTrack {
    #[serde(flatten)]
    pub inner: super::TemporalTrack<CameraMotionSample>,
    pub master_fingerprint: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalMotionTrack {
    #[serde(flatten)]
    pub inner: super::TemporalTrack<GlobalMotionSample>,
    pub master_fingerprint: [u8; 32],
}

impl TrackMaster for CameraMotionTrack {
    fn master(&self) -> &[u8; 32] {
        &self.master_fingerprint
    }
}

impl TrackMaster for GlobalMotionTrack {
    fn master(&self) -> &[u8; 32] {
        &self.master_fingerprint
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracks::RationalTime;

    fn obs(frame: u64) -> FrameObservation {
        let mut o = FrameObservation::new(frame, RationalTime::from_frames(frame, 30_000), [3u8; 32]);
        o = o.with_hint(TrackKind::CameraMotion, "kind", 1);
        o = o.with_hint(TrackKind::CameraMotion, "magnitude", 250);
        o = o.with_hint(TrackKind::GlobalMotion, "tx", 10);
        o
    }

    #[test]
    fn camera_motion_decodes_pan() {
        let ex = MotionTrackExtractor;
        let t = ex.extract_camera(&[obs(0)], [3u8; 32]);
        assert_eq!(t.inner.samples[0].value.kind, CameraMotionKind::Pan);
        assert_eq!(t.inner.samples[0].value.magnitude_milli, 250);
    }

    #[test]
    fn global_motion_decodes_translation() {
        let ex = MotionTrackExtractor;
        let t = ex.extract_global(&[obs(0)], [3u8; 32]);
        assert_eq!(t.inner.samples[0].value.translation_x_milli, 10);
    }
}