//! Raw audio extraction for local model sidecars.

use std::path::{Path, PathBuf};

use thiserror::Error;

use video_core::process_runner::ProcessRunError;

use crate::process::{
    duration_scaled_timeout, run_media_command, string_args, AUDIO_EXTRACT_FLOOR,
    AUDIO_EXTRACT_PER_SOURCE_SECOND,
};
use crate::toolchain::{self, ToolchainError};

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("audio source does not exist: {0}")]
    MissingInput(PathBuf),
    #[error("audio output must not overwrite the source")]
    OutputIsInput,
    #[error("ffmpeg could not start: {0}")]
    Start(#[source] std::io::Error),
    #[error("ffmpeg audio extraction failed: {0}")]
    Failed(String),
    #[error("could not resolve a matched ffmpeg/ffprobe toolchain: {0}")]
    Toolchain(#[from] ToolchainError),
    #[error("ffmpeg process error: {0}")]
    Process(#[from] ProcessRunError),
}

/// Extract a stable mono f32le stream for local model sidecars.
///
/// The PCM cache is deliberately separate from immutable sources. Callers own
/// invalidation through the source content digest recorded in the project.
pub fn extract_audio_f32(input: &Path, output: &Path, sample_rate: u32) -> Result<(), AudioError> {
    if !input.is_file() {
        return Err(AudioError::MissingInput(input.to_path_buf()));
    }
    if input == output {
        return Err(AudioError::OutputIsInput);
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(AudioError::Start)?;
    }
    let ffmpeg = crate::ffmpeg_path()?;
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args([
        "-vn",
        "-ac",
        "1",
        "-ar",
        &sample_rate.to_string(),
        "-f",
        "f32le",
    ]));
    args.push(output.display().to_string());
    let timeout =
        duration_scaled_timeout(input, AUDIO_EXTRACT_FLOOR, AUDIO_EXTRACT_PER_SOURCE_SECOND);
    run_media_command(&ffmpeg, args, timeout, AudioError::Failed)?;
    Ok(())
}

/// Same as [`extract_audio_f32`], but also returns a
/// [`video_core::StageReceipt`] (hardening plan §10.4). Additive: existing
/// callers of `extract_audio_f32` are unaffected.
pub fn extract_audio_f32_with_receipt(
    input: &Path,
    output: &Path,
    sample_rate: u32,
) -> Result<video_core::StageReceipt, AudioError> {
    extract_audio_f32(input, output, sample_rate)?;
    let toolchain = toolchain::resolve()?;
    let mut toolchains = std::collections::BTreeMap::new();
    toolchains.insert("ffmpeg".to_string(), toolchain.identity());
    video_core::StageReceipt::build(
        "audio.extract_f32",
        env!("CARGO_PKG_VERSION"),
        &[input],
        &serde_json::json!({ "sample_rate": sample_rate }),
        toolchains,
        &[output],
    )
    .map_err(|error| AudioError::Failed(error.to_string()))
}
