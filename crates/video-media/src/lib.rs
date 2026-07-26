use serde::Deserialize;
use std::io::Write;
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
    pub has_video: bool,
    pub has_audio: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRenderSegment {
    pub input_index: usize,
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
    #[error("native caption renderer could not start: {0}")]
    CaptionStart(#[source] std::io::Error),
    #[error("native caption renderer failed: {0}")]
    CaptionFailed(String),
}

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

pub fn render_source_segments(
    inputs: &[PathBuf],
    segments: &[SourceRenderSegment],
    output: &Path,
) -> Result<(), RenderError> {
    if inputs.is_empty() || segments.is_empty() {
        return Err(RenderError::NoSegments);
    }
    if inputs.iter().any(|input| input == output) {
        return Err(RenderError::OutputIsInput);
    }
    for segment in segments {
        if segment.input_index >= inputs.len()
            || segment.start_ms < 0
            || segment.end_ms <= segment.start_ms
        {
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
            "[{}:v]trim=start={start:.3}:end={end:.3},setpts=PTS-STARTPTS[v{index}];",
            segment.input_index
        ));
        filter.push_str(&format!(
            "[{}:a]atrim=start={start:.3}:end={end:.3},asetpts=PTS-STARTPTS[a{index}];",
            segment.input_index
        ));
    }
    for index in 0..segments.len() {
        filter.push_str(&format!("[v{index}][a{index}]"));
    }
    filter.push_str(&format!("concat=n={}:v=1:a=1[outv][outa]", segments.len()));
    let mut command = Command::new("ffmpeg");
    command.args(["-hide_banner", "-loglevel", "error", "-y"]);
    for input in inputs {
        command.args(["-i"]).arg(input);
    }
    let result = command
        .args([
            "-filter_complex",
            &filter,
            "-map",
            "[outv]",
            "-map",
            "[outa]",
        ])
        .args([
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
    } else {
        Err(RenderError::Failed(
            String::from_utf8_lossy(&result.stderr).trim().to_string(),
        ))
    }
}

pub fn render_to_preset(
    input: &Path,
    output: &Path,
    width: u32,
    height: u32,
) -> Result<(), RenderError> {
    if !input.is_file() {
        return Err(RenderError::Failed(format!(
            "input does not exist: {}",
            input.display()
        )));
    }
    if input == output {
        return Err(RenderError::OutputIsInput);
    }
    if width == 0 || height == 0 {
        return Err(RenderError::Failed(
            "output dimensions must be nonzero".into(),
        ));
    }
    let filter = format!(
        "scale={width}:{height}:force_original_aspect_ratio=increase,crop={width}:{height},setsar=1"
    );
    let audio_filter = measured_loudnorm_filter(input)?;
    let result = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(input)
        .args([
            "-vf",
            &filter,
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-preset",
            "veryfast",
            "-crf",
            "23",
            "-c:a",
            "aac",
            "-ar",
            "48000",
            "-af",
            &audio_filter,
            "-color_primaries",
            "bt709",
            "-color_trc",
            "bt709",
            "-colorspace",
            "bt709",
            "-movflags",
            "+faststart",
        ])
        .arg(output)
        .output()
        .map_err(RenderError::Start)?;
    if result.status.success() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            String::from_utf8_lossy(&result.stderr).trim().to_string(),
        ))
    }
}

fn measured_loudnorm_filter(input: &Path) -> Result<String, RenderError> {
    let measurement = Command::new("ffmpeg")
        .args(["-hide_banner", "-nostats", "-i"])
        .arg(input)
        .args([
            "-af",
            "highpass=f=80,loudnorm=I=-14:TP=-1:LRA=11:print_format=json",
            "-f",
            "null",
            "-",
        ])
        .output()
        .map_err(RenderError::Start)?;
    if !measurement.status.success() {
        return Err(RenderError::Failed(
            String::from_utf8_lossy(&measurement.stderr)
                .trim()
                .to_string(),
        ));
    }
    let stderr = String::from_utf8_lossy(&measurement.stderr);
    let start = stderr.rfind('{').ok_or_else(|| {
        RenderError::Failed("FFmpeg loudnorm did not emit measurement JSON".into())
    })?;
    let end = stderr[start..].find('}').ok_or_else(|| {
        RenderError::Failed("FFmpeg loudnorm emitted incomplete measurement JSON".into())
    })? + start
        + 1;
    let values: serde_json::Value = serde_json::from_str(&stderr[start..end]).map_err(|error| {
        RenderError::Failed(format!("invalid FFmpeg loudnorm measurement: {error}"))
    })?;
    let value = |key: &str| {
        values
            .get(key)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| RenderError::Failed(format!("FFmpeg loudnorm is missing {key}")))
    };
    Ok(format!(
        "highpass=f=80,loudnorm=I=-14:TP=-1:LRA=11:measured_I={}:measured_LRA={}:measured_TP={}:measured_thresh={}:offset={}:linear=true:print_format=summary",
        value("input_i")?,
        value("input_lra")?,
        value("input_tp")?,
        value("input_thresh")?,
        value("target_offset")?,
    ))
}

pub fn extract_frame(input: &Path, timestamp_ms: i64, output: &Path) -> Result<(), RenderError> {
    if !input.is_file() {
        return Err(RenderError::Failed(format!(
            "input does not exist: {}",
            input.display()
        )));
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(RenderError::Start)?;
    }
    let timestamp = format!("{:.3}", timestamp_ms.max(0) as f64 / 1_000.0);
    let result = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-ss",
            &timestamp,
            "-i",
        ])
        .arg(input)
        .args(["-frames:v", "1", "-q:v", "2"])
        .arg(output)
        .output()
        .map_err(RenderError::Start)?;
    if result.status.success() && output.is_file() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            String::from_utf8_lossy(&result.stderr).trim().to_string(),
        ))
    }
}

pub fn render_waveform(input: &Path, output: &Path) -> Result<(), RenderError> {
    if !input.is_file() {
        return Err(RenderError::Failed(format!(
            "input does not exist: {}",
            input.display()
        )));
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(RenderError::Start)?;
    }
    let result = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(input)
        .args([
            "-filter_complex",
            "showwavespic=s=1200x240:colors=0x44D7B6",
            "-frames:v",
            "1",
        ])
        .arg(output)
        .output()
        .map_err(RenderError::Start)?;
    if result.status.success() && output.is_file() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            String::from_utf8_lossy(&result.stderr).trim().to_string(),
        ))
    }
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
    let result = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(input)
        .args([
            "-vn",
            "-ac",
            "1",
            "-ar",
            &sample_rate.to_string(),
            "-f",
            "f32le",
        ])
        .arg(output)
        .output()
        .map_err(AudioError::Start)?;
    if result.status.success() {
        Ok(())
    } else {
        Err(AudioError::Failed(
            String::from_utf8_lossy(&result.stderr).trim().to_string(),
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
    let sidecar = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("sidecars/render-worker/captions-macos.swift");
    if !sidecar.is_file() {
        return Err(RenderError::CaptionFailed(format!(
            "caption worker source is missing: {}",
            sidecar.display()
        )));
    }
    let worker = output
        .parent()
        .ok_or_else(|| RenderError::CaptionFailed("caption output has no parent directory".into()))?
        .join(".cutright-captions-macos");
    let source_newer = std::fs::metadata(&sidecar)
        .and_then(|source| source.modified())
        .ok()
        .zip(
            std::fs::metadata(&worker)
                .and_then(|binary| binary.modified())
                .ok(),
        )
        .is_some_and(|(source, binary)| source > binary);
    if !worker.is_file() || source_newer {
        let compile = Command::new("swiftc")
            .arg(&sidecar)
            .arg("-O")
            .arg("-o")
            .arg(&worker)
            .output()
            .map_err(RenderError::CaptionStart)?;
        if !compile.status.success() {
            return Err(RenderError::CaptionFailed(
                String::from_utf8_lossy(&compile.stderr).trim().to_string(),
            ));
        }
    }
    let request = serde_json::json!({
        "input_path": input,
        "captions_path": captions,
        "output_path": output,
    });
    let mut child = Command::new(worker)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(RenderError::CaptionStart)?;
    child
        .stdin
        .as_mut()
        .expect("piped stdin")
        .write_all(&serde_json::to_vec(&request).expect("caption request JSON"))
        .map_err(RenderError::CaptionStart)?;
    let result = child
        .wait_with_output()
        .map_err(RenderError::CaptionStart)?;
    if result.status.success() && output.is_file() {
        Ok(())
    } else {
        Err(RenderError::CaptionFailed(
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
