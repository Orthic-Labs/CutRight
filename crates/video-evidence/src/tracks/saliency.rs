//! Saliency tracks (CR-V2-B3-018).
//!
//! Per-frame saliency maps are reduced to a deterministic integer-millis
//! grid and a centre-of-mass. The reduced shape is what the rest of the
//! crate consumes, so the extractor stays cheap and the master fingerprint
//! stays stable.

use serde::{Deserialize, Serialize};

use super::{
    build_track, fingerprint_value, FrameObservation, ReIdentificationEvidence, SubjectLoss,
    TimedSample, TrackKind, TrackMaster,
};

const GRID_DIM: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SaliencyMap {
    /// 8x8 grid of attention weights in millis (0..=1000).
    pub grid_milli: [[u16; GRID_DIM]; GRID_DIM],
}

impl SaliencyMap {
    pub fn centroid_milli(&self) -> (i32, i32) {
        let mut sx: u64 = 0;
        let mut sy: u64 = 0;
        let mut total: u64 = 0;
        for (y, row) in self.grid_milli.iter().enumerate() {
            for (x, weight) in row.iter().enumerate() {
                sx += (*weight as u64) * (x as u64);
                sy += (*weight as u64) * (y as u64);
                total += *weight as u64;
            }
        }
        if total == 0 {
            return (500, 500);
        }
        let cx = (sx.saturating_mul(1000)) / (total * (GRID_DIM as u64 - 1));
        let cy = (sy.saturating_mul(1000)) / (total * (GRID_DIM as u64 - 1));
        (cx.min(1000) as i32, cy.min(1000) as i32)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SaliencySample {
    pub map: SaliencyMap,
    pub centroid_x_milli: i32,
    pub centroid_y_milli: i32,
}

pub struct SaliencyTrackExtractor;

impl SaliencyTrackExtractor {
    pub fn extract(
        &self,
        observations: &[FrameObservation],
        source_hash: [u8; 32],
    ) -> SaliencyTrack {
        let mut samples = Vec::new();
        for obs in observations {
            let mut grid = [[0u16; GRID_DIM]; GRID_DIM];
            for y in 0..GRID_DIM {
                for x in 0..GRID_DIM {
                    let v = obs
                        .hint(TrackKind::Saliency, &format!("g{y}x{x}"))
                        .unwrap_or(0)
                        .max(0)
                        .min(1000);
                    grid[y][x] = v as u16;
                }
            }
            let map = SaliencyMap { grid_milli: grid };
            let (cx, cy) = map.centroid_milli();
            let sample = SaliencySample {
                map,
                centroid_x_milli: cx,
                centroid_y_milli: cy,
            };
            let fp = fingerprint_value(&sample);
            samples.push(TimedSample::new(obs.source_frame, obs.timestamp, sample, fp));
        }
        let losses = Vec::<SubjectLoss>::new();
        let reids = Vec::<ReIdentificationEvidence>::new();
        let track_id = super::make_track_id(&source_hash, TrackKind::Saliency, 0);
        let master = super::master_fingerprint(&track_id, &samples, &reids, &losses);
        SaliencyTrack {
            inner: build_track(
                track_id,
                TrackKind::Saliency,
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
pub struct SaliencyTrack {
    #[serde(flatten)]
    pub inner: super::TemporalTrack<SaliencySample>,
    pub master_fingerprint: [u8; 32],
}

impl TrackMaster for SaliencyTrack {
    fn master(&self) -> &[u8; 32] {
        &self.master_fingerprint
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracks::RationalTime;

    #[test]
    fn centroid_is_500_500_for_uniform_grid() {
        let mut grid = [[0u16; GRID_DIM]; GRID_DIM];
        for y in 0..GRID_DIM {
            for x in 0..GRID_DIM {
                grid[y][x] = 125;
            }
        }
        let map = SaliencyMap { grid_milli: grid };
        assert_eq!(map.centroid_milli(), (500, 500));
    }

    #[test]
    fn centroid_shifts_toward_mass() {
        let mut grid = [[0u16; GRID_DIM]; GRID_DIM];
        grid[0][0] = 1000;
        let map = SaliencyMap { grid_milli: grid };
        let (cx, cy) = map.centroid_milli();
        assert!(cx < 500);
        assert!(cy < 500);
    }

    #[test]
    fn track_is_deterministic() {
        let ex = SaliencyTrackExtractor;
        let mut obs = FrameObservation::new(0, RationalTime::ZERO, [5u8; 32]);
        obs = obs.with_hint(TrackKind::Saliency, "g0x0", 800);
        let inputs = [obs.clone(), obs.clone()];
        let t = ex.extract(&inputs, [5u8; 32]);
        let t2 = ex.extract(&inputs, [5u8; 32]);
        assert_eq!(t.master_fingerprint, t2.master_fingerprint);
    }
}