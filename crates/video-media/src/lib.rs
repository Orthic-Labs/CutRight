use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;
use video_core::Timebase;

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaMetadata {
    pub duration_ms: Option<i64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub rotation_degrees: Option<i32>,
    pub is_hdr: Option<bool>,
    pub timebase: Option<Timebase>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderSegment {
    pub start_ms: i64,
    pub end_ms: i64,
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("render requires at least one segment")]
    NoSegments,
    #[error("render segment has an invalid range: {start_ms}..{end_ms}")]
    InvalidSegment { start_ms: i64, end_ms: i64 },
    #[error("render output must not overwrite the source")]
    OutputIsInput,
    #[error("ffmpeg could not start: {0}")]
    Start(#[source] std::io::Error),
    #[error("ffmpeg failed: {0}")]
    Failed(String),
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
    if !path.is_file() {
        return Err(ProbeError::MissingInput(path.to_path_buf()));
    }

    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .map_err(ProbeError::Start)?;
    if !output.status.success() {
        return Err(ProbeError::Failed {
            path: path.to_path_buf(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    let response: ProbeResponse =
        serde_json::from_slice(&output.stdout).map_err(|source| ProbeError::InvalidJson {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(metadata_from_probe(response))
}

pub fn render_segments(
    input: &Path,
    segments: &[RenderSegment],
    output: &Path,
) -> Result<(), RenderError> {
    if segments.is_empty() {
        return Err(RenderError::NoSegments);
    }
    if input == output {
        return Err(RenderError::OutputIsInput);
    }
    for segment in segments {
        if segment.start_ms < 0 || segment.end_ms <= segment.start_ms {
            return Err(RenderError::InvalidSegment {
                start_ms: segment.start_ms,
                end_ms: segment.end_ms,
            });
        }
    }

    let mut filter = String::new();
    for (index, segment) in segments.iter().enumerate() {
        let start = segment.start_ms as f64 / 1_000.0;
        let end = segment.end_ms as f64 / 1_000.0;
        filter.push_str(&format!(
            "[0:v]trim=start={start:.3}:end={end:.3},setpts=PTS-STARTPTS[v{index}];"
        ));
        filter.push_str(&format!(
            "[0:a]atrim=start={start:.3}:end={end:.3},asetpts=PTS-STARTPTS[a{index}];"
        ));
    }
    for index in 0..segments.len() {
        filter.push_str(&format!("[v{index}][a{index}]"));
    }
    filter.push_str(&format!("concat=n={}:v=1:a=1[outv][outa]", segments.len()));

    let output_result = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(input)
        .args([
            "-filter_complex",
            &filter,
            "-map",
            "[outv]",
            "-map",
            "[outa]",
        ])
        .args([
            "-c:v", "libx264", "-preset", "veryfast", "-crf", "23", "-c:a", "aac",
        ])
        .args(["-movflags", "+faststart"])
        .arg(output)
        .output()
        .map_err(RenderError::Start)?;
    if output_result.status.success() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            String::from_utf8_lossy(&output_result.stderr)
                .trim()
                .to_string(),
        ))
    }
}

pub fn render_subtitled(input: &Path, captions: &Path, output: &Path) -> Result<(), RenderError> {
    if !input.is_file() {
        return Err(RenderError::Failed(format!(
            "input does not exist: {}",
            input.display()
        )));
    }
    if !captions.is_file() {
        return Err(RenderError::Failed(format!(
            "captions do not exist: {}",
            captions.display()
        )));
    }
    if input == output {
        return Err(RenderError::OutputIsInput);
    }
    let caption_path = captions
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "\\'");
    let filter = format!("subtitles=filename={caption_path}");
    let result = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(input)
        .args([
            "-vf",
            &filter,
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-crf",
            "23",
            "-c:a",
            "aac",
            "-movflags",
            "+faststart",
        ])
        .arg(output)
        .output()
        .map_err(RenderError::Start)?;
    if result.status.success() {
        Ok(())
    } else if String::from_utf8_lossy(&result.stderr).contains("No such filter: 'subtitles'") {
        let copy_result = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
            .arg(input)
            .args(["-c", "copy"])
            .arg(output)
            .output()
            .map_err(RenderError::Start)?;
        if copy_result.status.success() {
            Ok(())
        } else {
            Err(RenderError::Failed(
                String::from_utf8_lossy(&copy_result.stderr)
                    .trim()
                    .to_string(),
            ))
        }
    } else {
        Err(RenderError::Failed(
            String::from_utf8_lossy(&result.stderr).trim().to_string(),
        ))
    }
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
