//! On-screen text region tracks (CR-V2-B3-018).
//!
//! Detects text regions per frame, OCRs them, and emits a track per
//! detected region. Identical inputs always collapse to identical tracks,
//! and no OCR is performed against any mutable or networked service — the
//! extractor trusts the upstream vision pack's deterministic OCR result
//! recorded in [`FrameObservation::hint`].

use serde::{Deserialize, Serialize};

use super::{
    build_track, fingerprint_value, FrameObservation, ReIdentificationEvidence, SubjectLoss,
    TimedSample, TrackKind, TrackMaster,
};

fn i32_hint(obs: &FrameObservation, kind: TrackKind, name: &str) -> i32 {
    obs.hint(kind, name).unwrap_or(0).clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

fn u32_hint(obs: &FrameObservation, kind: TrackKind, name: &str) -> u32 {
    obs.hint(kind, name).unwrap_or(0).max(0) as u32
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextRegion {
    pub x_milli: i32,
    pub y_milli: i32,
    pub w_milli: i32,
    pub h_milli: i32,
    pub text: String,
    pub language: String,
    pub confidence_milli: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextSample {
    pub region: TextRegion,
}

pub struct TextTrackExtractor;

impl TextTrackExtractor {
    pub fn extract(
        &self,
        observations: &[FrameObservation],
        source_hash: [u8; 32],
    ) -> TextTrack {
        let mut samples = Vec::new();
        for obs in observations {
            let count = obs.hint(TrackKind::TextRegion, "count").unwrap_or(0).max(0) as u32;
            for index in 0..count {
                let region = TextRegion {
                    x_milli: i32_hint(obs, TrackKind::TextRegion, &format!("r{index}_x")),
                    y_milli: i32_hint(obs, TrackKind::TextRegion, &format!("r{index}_y")),
                    w_milli: i32_hint(obs, TrackKind::TextRegion, &format!("r{index}_w")),
                    h_milli: i32_hint(obs, TrackKind::TextRegion, &format!("r{index}_h")),
                    text: obs
                        .hint(TrackKind::TextRegion, &format!("r{index}_text"))
                        .map(|c| c.to_string())
                        .unwrap_or_default(),
                    language: obs
                        .hint(TrackKind::TextRegion, &format!("r{index}_lang"))
                        .map(|c| c.to_string())
                        .unwrap_or_default(),
                    confidence_milli: u32_hint(obs, TrackKind::TextRegion, &format!("r{index}_conf")),
                };
                let fp = fingerprint_value(&region);
                samples.push(TimedSample::new(
                    obs.source_frame,
                    obs.timestamp,
                    TextSample { region },
                    fp,
                ));
            }
        }
        let losses = Vec::<SubjectLoss>::new();
        let reids = Vec::<ReIdentificationEvidence>::new();
        let track_id = super::make_track_id(&source_hash, TrackKind::TextRegion, 0);
        let master = super::master_fingerprint(&track_id, &samples, &reids, &losses);
        TextTrack {
            inner: build_track(
                track_id,
                TrackKind::TextRegion,
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
pub struct TextTrack {
    #[serde(flatten)]
    pub inner: super::TemporalTrack<TextSample>,
    pub master_fingerprint: [u8; 32],
}

impl TrackMaster for TextTrack {
    fn master(&self) -> &[u8; 32] {
        &self.master_fingerprint
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracks::RationalTime;

    #[test]
    fn text_track_is_deterministic_for_same_inputs() {
        let mut obs = FrameObservation::new(0, RationalTime::ZERO, [6u8; 32]);
        obs = obs.with_hint(TrackKind::TextRegion, "count", 1);
        obs = obs.with_hint(TrackKind::TextRegion, "r0_text", 1234);
        let ex = TextTrackExtractor;
        let a = ex.extract(&[obs.clone()], [6u8; 32]);
        let b = ex.extract(&[obs.clone()], [6u8; 32]);
        assert_eq!(a.master_fingerprint, b.master_fingerprint);
    }
}