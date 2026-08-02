//! `ffprobe`-backed media metadata probing.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

use video_core::process_runner::{CancellationToken, ProcessRunError, ProcessSpec};
use video_core::Timebase;

use crate::process::{media_env_allow, PROBE_STDOUT_CAP_BYTES, PROBE_TIMEOUT, STDERR_CAP_BYTES};
use crate::toolchain::{self, MediaToolchain, ToolchainError};

#[derive(Debug, Error)]
pub enum ProbeError {
    #[error("media file does not exist: {0}")]
    MissingInput(PathBuf),
    #[error("ffprobe could not start: {0}")]
    Start(#[source] std::io::Error),
    #[error("ffprobe failed for {path}: {stderr}")]
    Failed { path: PathBuf, stderr: String },
    #[error("ffprobe returned invalid JSON for {path}: {source}")]
    InvalidJson {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("could not resolve a matched ffmpeg/ffprobe toolchain: {0}")]
    Toolchain(#[from] ToolchainError),
    #[error("ffprobe process error: {0}")]
    Process(#[from] ProcessRunError),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaMetadata {
    pub duration_ms: Option<i64>,
    pub has_video: bool,
    pub has_audio: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub rotation_degrees: Option<i32>,
    pub is_hdr: Option<bool>,
    pub timebase: Option<Timebase>,
}

#[derive(Debug, Deserialize)]
struct ProbeResponse {
    #[serde(default)]
    streams: Vec<ProbeStream>,
    format: Option<ProbeFormat>,
}

#[derive(Debug, Deserialize)]
struct ProbeFormat {
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    color_transfer: Option<String>,
    tags: Option<ProbeTags>,
}

#[derive(Debug, Deserialize)]
struct ProbeTags {
    rotate: Option<String>,
}

pub fn probe(path: &Path) -> Result<MediaMetadata, ProbeError> {
    let toolchain = toolchain::resolve()?;
    probe_with_toolchain(path, &toolchain)
}

/// Same as [`probe`], but reuses an already-resolved [`MediaToolchain`]
/// instead of re-resolving one (§10.3: resolve once, reuse, rather than
/// re-probing `ffmpeg -filters`/`-encoders` and re-checking `ffprobe`'s
/// version per call site within one render/probe operation).
pub(crate) fn probe_with_toolchain(
    path: &Path,
    toolchain: &MediaToolchain,
) -> Result<MediaMetadata, ProbeError> {
    if !path.is_file() {
        return Err(ProbeError::MissingInput(path.to_path_buf()));
    }

    let spec = ProcessSpec {
        executable: toolchain.ffprobe.clone(),
        args: vec![
            "-v".into(),
            "error".into(),
            "-print_format".into(),
            "json".into(),
            "-show_format".into(),
            "-show_streams".into(),
            path.display().to_string(),
        ],
        env_allow: media_env_allow(),
        working_dir: None,
        timeout: PROBE_TIMEOUT,
        stdout_cap_bytes: PROBE_STDOUT_CAP_BYTES,
        stderr_cap_bytes: STDERR_CAP_BYTES,
    };
    let outcome = video_core::process_runner::run_process(&spec, &CancellationToken::new())?;
    if !outcome.success() {
        let mut stderr = String::from_utf8_lossy(&outcome.stderr).trim().to_string();
        if outcome.stderr_truncated {
            stderr.push_str(" ...[stderr truncated]");
        }
        return Err(ProbeError::Failed {
            path: path.to_path_buf(),
            stderr,
        });
    }

    let response: ProbeResponse =
        serde_json::from_slice(&outcome.stdout).map_err(|source| ProbeError::InvalidJson {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(metadata_from_probe(response))
}

fn metadata_from_probe(response: ProbeResponse) -> MediaMetadata {
    let video = response
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("video"));
    let duration_ms = response
        .format
        .and_then(|format| format.duration)
        .and_then(|duration| duration.parse::<f64>().ok())
        .map(|duration| duration.mul_add(1_000.0, 0.0).round() as i64);
    let timebase = video
        .and_then(|stream| stream.r_frame_rate.as_deref())
        .and_then(parse_frame_rate);
    let rotation_degrees = video
        .and_then(|stream| stream.tags.as_ref())
        .and_then(|tags| tags.rotate.as_deref())
        .and_then(|rotation| rotation.parse::<i32>().ok());
    let is_hdr = video.map(|stream| {
        matches!(
            stream.color_transfer.as_deref(),
            Some("smpte2084") | Some("arib-std-b67") | Some("bt2020-10") | Some("bt2020-12")
        )
    });

    MediaMetadata {
        duration_ms,
        has_video: video.is_some(),
        has_audio: response
            .streams
            .iter()
            .any(|stream| stream.codec_type.as_deref() == Some("audio")),
        width: video.and_then(|stream| stream.width),
        height: video.and_then(|stream| stream.height),
        rotation_degrees,
        is_hdr,
        timebase,
    }
}

fn parse_frame_rate(value: &str) -> Option<Timebase> {
    let (num, den) = value.split_once('/')?;
    let fps_num = num.parse().ok()?;
    let fps_den = den.parse().ok()?;
    if fps_num == 0 || fps_den == 0 {
        return None;
    }
    Some(Timebase { fps_num, fps_den })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_probe_metadata_without_shell_fragments() {
        let response: ProbeResponse = serde_json::from_str(
            r#"{
              "format": {"duration": "12.3456"},
              "streams": [{
                "codec_type": "video",
                "width": 1920,
                "height": 1080,
                "r_frame_rate": "30000/1001",
                "color_transfer": "smpte2084",
                "tags": {"rotate": "90"}
              }]
            }"#,
        )
        .unwrap();
        let metadata = metadata_from_probe(response);
        assert_eq!(metadata.duration_ms, Some(12_346));
        assert!(metadata.has_video);
        assert!(!metadata.has_audio);
        assert_eq!(metadata.width, Some(1_920));
        assert_eq!(metadata.height, Some(1_080));
        assert_eq!(metadata.rotation_degrees, Some(90));
        assert_eq!(metadata.is_hdr, Some(true));
        assert_eq!(
            metadata.timebase,
            Some(Timebase {
                fps_num: 30_000,
                fps_den: 1_001
            })
        );
    }

    #[test]
    fn rejects_missing_input_before_starting_ffprobe() {
        let error = probe(Path::new("/does/not/exist.mp4")).unwrap_err();
        assert!(matches!(error, ProbeError::MissingInput(_)));
    }
}
