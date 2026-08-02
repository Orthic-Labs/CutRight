//! Versioned dialogue audio-finish profile (REV2 plan §15.2 "Audio").
//!
//! Parameters that drive the dialogue chain (high-pass, gentle compression,
//! de-ess, limiter), music ducking, and the loudness/true-peak gate live in
//! this artifact rather than as hard-coded globals, so a future revision of
//! the defaults is an explicit, receipted change (bumping `profile_version`)
//! rather than a silent constant edit deep in a filter-string builder.
//! Persisted at `audio/profile.json` and validated against
//! `schemas/audio-profile.schema.json`; the finish output this stage
//! produces is validated against `schemas/audio-finish.schema.json`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use video_media::{CompressorParams, DeEsserParams, DialogueChainParams, LoudnessMeasurement};

use crate::io::{read_json_if_file, write_json_atomic};
use crate::ProjectError;

pub const AUDIO_PROFILE_SCHEMA_VERSION: u32 = 1;

/// The versioned, persisted audio-finish profile. Defaults land on the
/// REV2 plan §15.2 targets: -14 LUFS integrated loudness, -1 dBTP true
/// peak, an 80Hz dialogue high-pass, gentle compression, and a light
/// de-ess.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct AudioProfile {
    pub schema_version: u32,
    /// Bumped whenever the defaults below change; recorded on every audio
    /// finish stage receipt and cache key so a reprocessed profile never
    /// silently reuses a stem built under the previous parameters.
    pub profile_version: u32,
    pub high_pass_hz: f64,
    pub compressor_threshold_db: f64,
    pub compressor_ratio: f64,
    pub compressor_attack_ms: f64,
    pub compressor_release_ms: f64,
    pub deesser_intensity: f64,
    pub deesser_frequency: f64,
    pub limiter_ceiling_dbtp: f64,
    pub target_integrated_lufs: f64,
    pub loudness_tolerance_lu: f64,
    pub target_true_peak_dbtp: f64,
    pub true_peak_tolerance_db: f64,
    pub duck_reduction_db: f64,
    pub room_tone_step_tolerance_db: f64,
}

impl Default for AudioProfile {
    fn default() -> Self {
        Self {
            schema_version: AUDIO_PROFILE_SCHEMA_VERSION,
            profile_version: 1,
            high_pass_hz: 80.0,
            compressor_threshold_db: -18.0,
            compressor_ratio: 2.5,
            compressor_attack_ms: 10.0,
            compressor_release_ms: 100.0,
            deesser_intensity: 0.3,
            deesser_frequency: 0.5,
            limiter_ceiling_dbtp: -1.0,
            target_integrated_lufs: -14.0,
            loudness_tolerance_lu: 1.0,
            target_true_peak_dbtp: -1.0,
            true_peak_tolerance_db: 0.5,
            duck_reduction_db: -12.0,
            room_tone_step_tolerance_db: 3.0,
        }
    }
}

/// The result of checking one [`LoudnessMeasurement`] against an
/// [`AudioProfile`]'s tolerances (REV2 plan §15.2: "the gate fails the
/// finish when the measured result falls outside the profile's tolerance").
#[derive(Debug, Clone, Serialize)]
pub struct LoudnessGateResult {
    pub passed: bool,
    pub integrated_lufs: Option<f64>,
    pub true_peak_dbtp: Option<f64>,
    pub clipped_samples: u64,
    pub failures: Vec<String>,
}

impl AudioProfile {
    pub fn dialogue_chain_params(&self) -> DialogueChainParams {
        DialogueChainParams {
            high_pass_hz: self.high_pass_hz,
            compressor: CompressorParams {
                threshold_db: self.compressor_threshold_db,
                ratio: self.compressor_ratio,
                attack_ms: self.compressor_attack_ms,
                release_ms: self.compressor_release_ms,
            },
            deesser: DeEsserParams {
                intensity: self.deesser_intensity,
                frequency: self.deesser_frequency,
            },
            limiter_ceiling_dbtp: self.limiter_ceiling_dbtp,
        }
    }

    /// Evaluate a measured stem against this profile's loudness, true-peak,
    /// and no-clipping tolerances. Measuring alone (what
    /// `measure_loudness_and_clipping` does) is not the gate; this is —
    /// callers treat a `passed: false` result as a hard finish failure.
    pub fn evaluate_loudness_gate(&self, measurement: &LoudnessMeasurement) -> LoudnessGateResult {
        let mut failures = Vec::new();
        match measurement.integrated_lufs {
            Some(lufs)
                if (lufs - self.target_integrated_lufs).abs() <= self.loudness_tolerance_lu => {}
            Some(lufs) => failures.push(format!(
                "integrated loudness {lufs:.2} LUFS is outside {:.2} +/- {:.2} LU",
                self.target_integrated_lufs, self.loudness_tolerance_lu
            )),
            None => failures.push("integrated loudness could not be measured".into()),
        }
        match measurement.true_peak_dbtp {
            Some(true_peak) if true_peak <= self.target_true_peak_dbtp + self.true_peak_tolerance_db => {}
            Some(true_peak) => failures.push(format!(
                "true peak {true_peak:.2} dBTP exceeds the {:.2} dBTP ceiling (+{:.2} dB tolerance)",
                self.target_true_peak_dbtp, self.true_peak_tolerance_db
            )),
            None => failures.push("true peak could not be measured".into()),
        }
        if measurement.clipped_samples > 0 {
            failures.push(format!(
                "{} clipped samples detected; no clipped samples are permitted",
                measurement.clipped_samples
            ));
        }
        LoudnessGateResult {
            passed: failures.is_empty(),
            integrated_lufs: measurement.integrated_lufs,
            true_peak_dbtp: measurement.true_peak_dbtp,
            clipped_samples: measurement.clipped_samples,
            failures,
        }
    }
}

pub(crate) fn audio_profile_path(project_path: &Path) -> PathBuf {
    project_path.join("audio/profile.json")
}

/// Load the project's persisted audio profile, or initialize and persist
/// the versioned default the first time this stage runs (§15.2: versioned
/// artifact, never a hard-coded global). Once written, a profile is never
/// silently rewritten by a later run with different defaults — only an
/// explicit edit (or a code change bumping `profile_version` for a fresh
/// project) changes it.
pub(crate) fn load_or_init_audio_profile(
    project_path: &Path,
) -> Result<AudioProfile, ProjectError> {
    let path = audio_profile_path(project_path);
    if let Some(profile) = read_json_if_file::<AudioProfile>(&path) {
        return Ok(profile);
    }
    let profile = AudioProfile::default();
    write_json_atomic(&path, &profile)?;
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use video_media::LoudnessMeasurement;

    #[test]
    fn gate_passes_within_tolerance() {
        let profile = AudioProfile::default();
        let measurement = LoudnessMeasurement {
            integrated_lufs: Some(-14.3),
            true_peak_dbtp: Some(-1.2),
            clipped_samples: 0,
        };
        let result = profile.evaluate_loudness_gate(&measurement);
        assert!(
            result.passed,
            "expected pass, got failures: {:?}",
            result.failures
        );
    }

    #[test]
    fn gate_fails_when_loudness_outside_tolerance() {
        let profile = AudioProfile::default();
        let measurement = LoudnessMeasurement {
            integrated_lufs: Some(-20.0),
            true_peak_dbtp: Some(-1.2),
            clipped_samples: 0,
        };
        let result = profile.evaluate_loudness_gate(&measurement);
        assert!(!result.passed);
        assert!(result
            .failures
            .iter()
            .any(|failure| failure.contains("integrated loudness")));
    }

    #[test]
    fn gate_fails_when_true_peak_exceeds_ceiling() {
        let profile = AudioProfile::default();
        let measurement = LoudnessMeasurement {
            integrated_lufs: Some(-14.0),
            true_peak_dbtp: Some(0.3),
            clipped_samples: 0,
        };
        let result = profile.evaluate_loudness_gate(&measurement);
        assert!(!result.passed);
        assert!(result
            .failures
            .iter()
            .any(|failure| failure.contains("true peak")));
    }

    #[test]
    fn gate_fails_on_any_clipped_sample() {
        let profile = AudioProfile::default();
        let measurement = LoudnessMeasurement {
            integrated_lufs: Some(-14.0),
            true_peak_dbtp: Some(-1.0),
            clipped_samples: 1,
        };
        let result = profile.evaluate_loudness_gate(&measurement);
        assert!(!result.passed);
        assert!(result
            .failures
            .iter()
            .any(|failure| failure.contains("clipped")));
    }

    #[test]
    fn load_or_init_persists_a_versioned_default_and_reuses_it() {
        let dir = tempfile::tempdir().unwrap();
        let profile = load_or_init_audio_profile(dir.path()).unwrap();
        assert_eq!(profile.profile_version, 1);
        assert!(audio_profile_path(dir.path()).is_file());

        // A second load reuses the persisted file rather than reinitializing.
        let mut on_disk: AudioProfile =
            crate::io::read_json(&audio_profile_path(dir.path())).unwrap();
        on_disk.profile_version = 99;
        write_json_atomic(&audio_profile_path(dir.path()), &on_disk).unwrap();
        let reloaded = load_or_init_audio_profile(dir.path()).unwrap();
        assert_eq!(reloaded.profile_version, 99);
    }
}
