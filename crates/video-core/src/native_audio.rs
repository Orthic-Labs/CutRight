//!
//! CutRight native audio finishing engine (CR-V2-B5-020).
//!
//! The audio lane owns:
//! - an `AudioProfile` — loudness target, true-peak ceiling, platform presets
//! - `AudioCue` records — music, SFX, transient markers
//! - `AudioFinish` — the finished audio track with binding evidence
//!
//! The lane performs transient sync and a reverb-throw check (rejects a
//! cue that would exceed the platform's reverb budget).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("reverb throw exceeds platform budget: cue={0}")]
    ReverbExceeds(String),
    #[error("loudness target out of range: integrated_lufs={0}")]
    LoudnessOutOfRange(f64),
    #[error("cue requires an evidence_ref: id={0}")]
    UnboundCue(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioProfile {
    pub id: String,
    pub platform: String,
    pub integrated_lufs: f64,
    pub true_peak_db: f64,
    pub reverb_budget_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioCue {
    pub id: String,
    pub kind: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub evidence_ref: String,
    pub reverb_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFinish {
    pub id: String,
    pub version: String,
    pub profile_id: String,
    pub cues: Vec<AudioCue>,
    pub integrated_lufs: f64,
    pub true_peak_db: f64,
    pub metrics: BTreeMap<String, f64>,
}

pub struct NativeAudioEngine;

impl NativeAudioEngine {
    pub fn finish(profile: &AudioProfile, cues: Vec<AudioCue>) -> Result<AudioFinish, AudioError> {
        if !(-24.0..=-13.0).contains(&profile.integrated_lufs) {
            return Err(AudioError::LoudnessOutOfRange(profile.integrated_lufs));
        }
        for c in &cues {
            if c.evidence_ref.is_empty() {
                return Err(AudioError::UnboundCue(c.id.clone()));
            }
            if c.reverb_ms > profile.reverb_budget_ms {
                return Err(AudioError::ReverbExceeds(c.id.clone()));
            }
        }
        Ok(AudioFinish {
            id: format!("af_{}", profile.id),
            version: "v2".to_string(),
            profile_id: profile.id.clone(),
            cues,
            integrated_lufs: profile.integrated_lufs,
            true_peak_db: profile.true_peak_db,
            metrics: BTreeMap::new(),
        })
    }

    pub fn transient_sync(cue: &AudioCue, beat_start_ms: u64) -> u64 {
        // snap the cue's start_ms to the nearest beat within 50ms
        let delta = (cue.start_ms as i64 - beat_start_ms as i64).abs();
        if delta <= 50 {
            beat_start_ms
        } else {
            cue.start_ms
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> AudioProfile {
        AudioProfile {
            id: "ap_1".to_string(),
            platform: "ig_reels".to_string(),
            integrated_lufs: -16.0,
            true_peak_db: -1.0,
            reverb_budget_ms: 200,
        }
    }

    fn cue() -> AudioCue {
        AudioCue {
            id: "c_0".to_string(),
            kind: "music".to_string(),
            start_ms: 0,
            end_ms: 1000,
            evidence_ref: "evidence:ev_1".to_string(),
            reverb_ms: 50,
        }
    }

    #[test]
    fn accepts_valid_cues() {
        NativeAudioEngine::finish(&profile(), vec![cue()]).expect("ok");
    }

    #[test]
    fn rejects_out_of_range_lufs() {
        let mut p = profile();
        p.integrated_lufs = -5.0;
        let err = NativeAudioEngine::finish(&p, vec![cue()])
            .err()
            .expect("err");
        assert!(matches!(err, AudioError::LoudnessOutOfRange(_)));
    }

    #[test]
    fn rejects_reverb_over_budget() {
        let mut c = cue();
        c.reverb_ms = 500;
        let err = NativeAudioEngine::finish(&profile(), vec![c])
            .err()
            .expect("err");
        assert!(matches!(err, AudioError::ReverbExceeds(_)));
    }

    #[test]
    fn snaps_to_beat_within_50ms() {
        let c = cue();
        let snapped = NativeAudioEngine::transient_sync(&c, 20);
        assert_eq!(snapped, 20);
    }
}
