//! Versioned color-profile artifact and per-deliverable export preset table
//! (REV2 plan §15.2 "Color" / "Export"). Every grading parameter a master
//! render applies lives in `color/profile.json`, validated against
//! `schemas/color-profile.schema.json` — never a scattered constant in
//! render code. [`export_preset_settings`] is the one place that names
//! encoder/color/audio expectations per deliverable (YouTube, Reels,
//! TikTok, archive/master).

use std::path::Path;

use serde::{Deserialize, Serialize};
use video_media::{ColorCorrection, CreativeLut, ShotMatchTarget};

use crate::io::{read_json_if_file, write_json_atomic};
use crate::ProjectError;

pub const COLOR_PROFILE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShotMatchProfile {
    pub mean_luma: f64,
    pub saturation_scale: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreativeLutProfile {
    pub name: String,
    pub path: String,
    pub strength: f64,
}

/// The full grading parameter set for a project's master render (plan
/// §15.2: "all parameters live in a versioned color profile artifact with a
/// schema, not in scattered constants"). Persisted at `color/profile.json`
/// and validated against `schemas/color-profile.schema.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorProfile {
    pub schema_version: u32,
    pub id: String,
    pub exposure_ev: f64,
    pub white_balance_temp_shift: f64,
    pub white_balance_tint_shift: f64,
    #[serde(default)]
    pub shot_match: Option<ShotMatchProfile>,
    #[serde(default)]
    pub creative_lut: Option<CreativeLutProfile>,
}

impl ColorProfile {
    /// The neutral default: no exposure/white-balance/shot-match/LUT
    /// adjustment. Every conversion still routes through
    /// `color_filter_chain`, so input-space-to-SDR conversion still applies
    /// even under the default profile.
    pub fn default_profile() -> Self {
        ColorProfile {
            schema_version: COLOR_PROFILE_SCHEMA_VERSION,
            id: "default-sdr-v1".to_string(),
            exposure_ev: 0.0,
            white_balance_temp_shift: 0.0,
            white_balance_tint_shift: 0.0,
            shot_match: None,
            creative_lut: None,
        }
    }

    /// Loads `color/profile.json` if present and validates its
    /// `schema_version`; falls back to [`Self::default_profile`] when the
    /// file does not exist (a project need not have graded its master yet).
    /// A present-but-invalid file is a hard error, never silently ignored.
    pub fn load_or_default(project_path: &Path) -> Result<Self, ProjectError> {
        let path = project_path.join("color/profile.json");
        match read_json_if_file::<ColorProfile>(&path) {
            Some(profile) => {
                if profile.schema_version != COLOR_PROFILE_SCHEMA_VERSION {
                    return Err(ProjectError::UnsupportedSchema(profile.schema_version));
                }
                if let Some(lut) = &profile.creative_lut {
                    if !(0.0..=1.0).contains(&lut.strength) {
                        return Err(ProjectError::InvalidState(format!(
                            "color profile {} has an out-of-bounds LUT strength {} (must be 0.0..=1.0)",
                            profile.id, lut.strength
                        )));
                    }
                }
                Ok(profile)
            }
            None => {
                if path.is_file() {
                    // Present but failed to parse/read as valid JSON for
                    // this schema.
                    return Err(ProjectError::InvalidManifest(format!(
                        "color profile at {} is invalid",
                        path.display()
                    )));
                }
                Ok(Self::default_profile())
            }
        }
    }

    pub fn write(&self, project_path: &Path) -> Result<(), ProjectError> {
        write_json_atomic(&project_path.join("color/profile.json"), self)
    }

    pub fn correction(&self) -> ColorCorrection {
        ColorCorrection {
            exposure_ev: self.exposure_ev,
            white_balance_temp_shift: self.white_balance_temp_shift,
            white_balance_tint_shift: self.white_balance_tint_shift,
        }
    }

    pub fn shot_match_target(&self) -> Option<ShotMatchTarget> {
        self.shot_match.as_ref().map(|target| ShotMatchTarget {
            mean_luma: target.mean_luma,
            saturation_scale: target.saturation_scale,
        })
    }

    pub fn creative_lut(&self) -> Option<CreativeLut> {
        self.creative_lut
            .as_ref()
            .map(|lut| CreativeLut::new(lut.path.clone().into(), lut.strength))
    }
}

/// Declarative encoder/color/audio expectations for one deliverable preset
/// (plan §15.2 Export: "preset profiles for YouTube, Reels, TikTok, and
/// archive/master, each with its own encoder settings and color/audio
/// expectations"). This is the one table other stages read instead of
/// re-deriving per-preset expectations ad hoc.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ExportPresetSettings {
    pub preset_id: &'static str,
    pub video_codec: &'static str,
    pub audio_codec: &'static str,
    /// Integrated loudness target in LUFS.
    pub loudness_target_lufs: f64,
    /// True-peak ceiling in dBTP.
    pub true_peak_ceiling_dbtp: f64,
    pub color_transfer: &'static str,
    pub color_primaries: &'static str,
}

pub const YOUTUBE_EXPORT_PRESET: ExportPresetSettings = ExportPresetSettings {
    preset_id: "youtube",
    video_codec: "libx264",
    audio_codec: "aac",
    loudness_target_lufs: -14.0,
    true_peak_ceiling_dbtp: -1.0,
    color_transfer: "bt709",
    color_primaries: "bt709",
};

pub const REELS_EXPORT_PRESET: ExportPresetSettings = ExportPresetSettings {
    preset_id: "reels",
    video_codec: "libx264",
    audio_codec: "aac",
    loudness_target_lufs: -14.0,
    true_peak_ceiling_dbtp: -1.0,
    color_transfer: "bt709",
    color_primaries: "bt709",
};

pub const TIKTOK_EXPORT_PRESET: ExportPresetSettings = ExportPresetSettings {
    preset_id: "tiktok",
    video_codec: "libx264",
    audio_codec: "aac",
    loudness_target_lufs: -14.0,
    true_peak_ceiling_dbtp: -1.0,
    color_transfer: "bt709",
    color_primaries: "bt709",
};

/// The archival/master preset: `prores_ks` (software), uncompressed PCM
/// audio, and a looser -16 LUFS/-2 dBTP archival target rather than the
/// platform-normalized -14/-1 delivery target (an archival master is graded
/// once and re-normalized per platform later, not delivered directly).
pub const ARCHIVE_EXPORT_PRESET: ExportPresetSettings = ExportPresetSettings {
    preset_id: "archive",
    video_codec: "prores_ks",
    audio_codec: "pcm_s24le",
    loudness_target_lufs: -16.0,
    true_peak_ceiling_dbtp: -2.0,
    color_transfer: "bt709",
    color_primaries: "bt709",
};

/// Resolves the export preset settings for a preset id. Unknown ids fall
/// back to the YouTube preset's encoder/loudness shape (the same default
/// every `OutputPreset` in `project.json` already assumes for delivery
/// encoding) rather than failing, since this table is descriptive metadata
/// consumed by provenance/package writers, not a gate.
pub fn export_preset_settings(preset_id: &str) -> ExportPresetSettings {
    match preset_id {
        "youtube" => YOUTUBE_EXPORT_PRESET,
        "reels" => REELS_EXPORT_PRESET,
        "tiktok" => TIKTOK_EXPORT_PRESET,
        "archive" | "master" => ARCHIVE_EXPORT_PRESET,
        _ => YOUTUBE_EXPORT_PRESET,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_round_trips_through_disk() {
        let temp = tempfile::tempdir().unwrap_or_else(|_| {
            panic!("tempdir");
        });
        std::fs::create_dir_all(temp.path().join("color")).unwrap();
        let profile = ColorProfile::default_profile();
        profile.write(temp.path()).unwrap();
        let loaded = ColorProfile::load_or_default(temp.path()).unwrap();
        assert_eq!(loaded, profile);
    }

    #[test]
    fn missing_profile_file_falls_back_to_default() {
        let temp = tempfile::tempdir().unwrap();
        let profile = ColorProfile::load_or_default(temp.path()).unwrap();
        assert_eq!(profile, ColorProfile::default_profile());
    }

    #[test]
    fn rejects_an_out_of_bounds_lut_strength_on_load() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("color")).unwrap();
        write_json_atomic(
            &temp.path().join("color/profile.json"),
            &serde_json::json!({
                "schema_version": COLOR_PROFILE_SCHEMA_VERSION,
                "id": "bad-lut",
                "exposure_ev": 0.0,
                "white_balance_temp_shift": 0.0,
                "white_balance_tint_shift": 0.0,
                "creative_lut": { "name": "x", "path": "x.cube", "strength": 5.0 }
            }),
        )
        .unwrap();
        let result = ColorProfile::load_or_default(temp.path());
        assert!(
            result.is_err(),
            "expected an out-of-bounds LUT strength to be rejected"
        );
    }

    #[test]
    fn export_preset_settings_names_a_distinct_archive_encoder() {
        let archive = export_preset_settings("archive");
        let youtube = export_preset_settings("youtube");
        assert_eq!(archive.video_codec, "prores_ks");
        assert_ne!(archive.video_codec, youtube.video_codec);
        assert_ne!(archive.audio_codec, youtube.audio_codec);
    }
}
