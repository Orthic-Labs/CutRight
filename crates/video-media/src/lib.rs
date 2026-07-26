use serde::Deserialize;
use std::fs;
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReframeAnchor {
    pub output_start_ms: i64,
    pub center_x: f64,
    pub center_y: f64,
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
    #[error("required FFmpeg capability is unavailable: {0}")]
    CapabilityMissing(String),
    #[error("caption card worker could not start: {0}")]
    CaptionStart(#[source] std::io::Error),
    #[error("caption card worker failed: {0}")]
    CaptionFailed(String),
}

#[derive(Debug, Clone)]
struct CaptionCue {
    start_seconds: f64,
    end_seconds: f64,
    text: String,
}

struct CaptionRenderOptions<'a> {
    width: u32,
    height: u32,
    vertical: bool,
    video_filter: &'a str,
    audio_filter: Option<&'a str>,
    rec709_output: bool,
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

    let source_filter = source_video_filter(input)?;
    let mut filter = String::new();
    for (index, segment) in segments.iter().enumerate() {
        let start = segment.start_ms as f64 / 1_000.0;
        let end = segment.end_ms as f64 / 1_000.0;
        filter.push_str(&format!(
            "[0:v]{},trim=start={start:.3}:end={end:.3},setpts=PTS-STARTPTS[v{index}];",
            source_filter.filter
        ));
        filter.push_str(&format!(
            "[0:a]atrim=start={start:.3}:end={end:.3},asetpts=PTS-STARTPTS[a{index}];"
        ));
    }
    for index in 0..segments.len() {
        filter.push_str(&format!("[v{index}][a{index}]"));
    }
    filter.push_str(&format!("concat=n={}:v=1:a=1[outv][outa]", segments.len()));

    let mut command = Command::new(ffmpeg_binary());
    command
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
        .args(preview_video_args()?)
        .args(["-c:a", "aac"]);
    if source_filter.rec709_output {
        command.args([
            "-color_primaries",
            "bt709",
            "-color_trc",
            "bt709",
            "-colorspace",
            "bt709",
        ]);
    }
    let output_result = command
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
    let source_filters = inputs
        .iter()
        .map(|input| source_video_filter(input))
        .collect::<Result<Vec<_>, _>>()?;
    let mut filter = String::new();
    for (index, segment) in segments.iter().enumerate() {
        let start = segment.start_ms as f64 / 1_000.0;
        let end = segment.end_ms as f64 / 1_000.0;
        filter.push_str(&format!(
            "[{}:v]{},trim=start={start:.3}:end={end:.3},setpts=PTS-STARTPTS[v{index}];",
            segment.input_index, source_filters[segment.input_index].filter
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
    let rec709_output = source_filters.iter().any(|filter| filter.rec709_output);
    let mut command = Command::new(ffmpeg_binary());
    command.args(["-hide_banner", "-loglevel", "error", "-y"]);
    for input in inputs {
        command.args(["-i"]).arg(input);
    }
    command
        .args([
            "-filter_complex",
            &filter,
            "-map",
            "[outv]",
            "-map",
            "[outa]",
        ])
        .args(preview_video_args()?)
        .args(["-c:a", "aac"]);
    if rec709_output {
        command.args([
            "-color_primaries",
            "bt709",
            "-color_trc",
            "bt709",
            "-colorspace",
            "bt709",
        ]);
    }
    let result = command
        .args(["-movflags", "+faststart"])
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

pub fn render_boundary_probe(
    input: &Path,
    boundary_ms: i64,
    output: &Path,
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
    let metadata = probe(input).map_err(|error| RenderError::Failed(error.to_string()))?;
    let start_ms = boundary_ms.saturating_sub(800).max(0);
    let mut end_ms = boundary_ms.saturating_add(800);
    if let Some(duration) = metadata.duration_ms {
        end_ms = end_ms.min(duration);
    }
    if boundary_ms <= start_ms || end_ms <= boundary_ms {
        return Err(RenderError::InvalidSegment { start_ms, end_ms });
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(RenderError::Start)?;
    }
    let start = start_ms as f64 / 1_000.0;
    let boundary = boundary_ms as f64 / 1_000.0;
    let end = end_ms as f64 / 1_000.0;
    let filter = format!(
        "[0:v]trim=start={start:.3}:end={boundary:.3},setpts=PTS-STARTPTS[v0];[0:a]atrim=start={start:.3}:end={boundary:.3},asetpts=PTS-STARTPTS[a0];[0:v]trim=start={boundary:.3}:end={end:.3},setpts=PTS-STARTPTS[v1];[0:a]atrim=start={boundary:.3}:end={end:.3},asetpts=PTS-STARTPTS[a1];[v0][a0][v1][a1]concat=n=2:v=1:a=1[outv][outa]"
    );
    let result = Command::new(ffmpeg_binary())
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
        .args(preview_video_args()?)
        .args(["-c:a", "aac", "-movflags", "+faststart"])
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
    let (filter, rec709_output) = preset_video_filter(input, width, height, None)?;
    let audio_filter = measured_loudnorm_filter(input)?;
    let mut command = Command::new(ffmpeg_binary());
    command
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
            "-movflags",
            "+faststart",
        ]);
    if rec709_output {
        command.args([
            "-color_primaries",
            "bt709",
            "-color_trc",
            "bt709",
            "-colorspace",
            "bt709",
        ]);
    }
    let result = command.arg(output).output().map_err(RenderError::Start)?;
    if result.status.success() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            String::from_utf8_lossy(&result.stderr).trim().to_string(),
        ))
    }
}

fn measured_loudnorm_filter(input: &Path) -> Result<String, RenderError> {
    let measurement = Command::new(ffmpeg_binary())
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
    let result = Command::new(ffmpeg_binary())
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
    let result = Command::new(ffmpeg_binary())
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

pub fn render_waveform_range(
    input: &Path,
    start_ms: i64,
    end_ms: i64,
    output: &Path,
) -> Result<(), RenderError> {
    if !input.is_file() {
        return Err(RenderError::Failed(format!(
            "input does not exist: {}",
            input.display()
        )));
    }
    if end_ms <= start_ms {
        return Err(RenderError::InvalidSegment { start_ms, end_ms });
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(RenderError::Start)?;
    }
    let start = start_ms as f64 / 1_000.0;
    let end = end_ms as f64 / 1_000.0;
    let filter = format!(
        "atrim=start={start:.3}:end={end:.3},asetpts=PTS-STARTPTS,showwavespic=s=1200x180:colors=0x44D7B6"
    );
    let result = Command::new(ffmpeg_binary())
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(input)
        .args(["-filter_complex", &filter, "-frames:v", "1"])
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

pub fn compose_decision_evidence(
    frames: &[PathBuf],
    waveform: &Path,
    output: &Path,
) -> Result<(), RenderError> {
    if frames.len() != 3 || frames.iter().any(|frame| !frame.is_file()) || !waveform.is_file() {
        return Err(RenderError::Failed(
            "decision evidence requires three frames and one waveform".into(),
        ));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(RenderError::Start)?;
    }
    let result = Command::new(ffmpeg_binary())
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(["-i"]).arg(&frames[0])
        .args(["-i"]).arg(&frames[1])
        .args(["-i"]).arg(&frames[2])
        .args(["-i"]).arg(waveform)
        .args([
            "-filter_complex",
            "[0:v]scale=400:225[a];[1:v]scale=400:225[b];[2:v]scale=400:225[c];[a][b][c]hstack=inputs=3[filmstrip];[3:v]scale=1200:180[wave];[filmstrip][wave]vstack=inputs=2",
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
    let result = Command::new(ffmpeg_binary())
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
    let metadata = probe(input).map_err(|error| RenderError::Failed(error.to_string()))?;
    let width = metadata
        .width
        .ok_or_else(|| RenderError::Failed("caption input has no width".into()))?;
    let height = metadata
        .height
        .ok_or_else(|| RenderError::Failed("caption input has no height".into()))?;
    render_captioned(
        input,
        captions,
        output,
        CaptionRenderOptions {
            width,
            height,
            vertical: false,
            video_filter: "setsar=1",
            audio_filter: None,
            rec709_output: false,
        },
    )
}

pub fn render_preset_with_captions(
    input: &Path,
    captions: &Path,
    output: &Path,
    width: u32,
    height: u32,
    vertical: bool,
) -> Result<(), RenderError> {
    render_preset_with_captions_and_reframe(input, captions, output, width, height, vertical, None)
}

pub fn render_preset_with_captions_and_reframe(
    input: &Path,
    captions: &Path,
    output: &Path,
    width: u32,
    height: u32,
    vertical: bool,
    reframe_anchors: Option<&[ReframeAnchor]>,
) -> Result<(), RenderError> {
    if width == 0 || height == 0 {
        return Err(RenderError::Failed(
            "output dimensions must be nonzero".into(),
        ));
    }
    let (filter, rec709_output) = preset_video_filter(input, width, height, reframe_anchors)?;
    let audio_filter = measured_loudnorm_filter(input)?;
    render_captioned(
        input,
        captions,
        output,
        CaptionRenderOptions {
            width,
            height,
            vertical,
            video_filter: &filter,
            audio_filter: Some(&audio_filter),
            rec709_output,
        },
    )
}

fn render_captioned(
    input: &Path,
    captions: &Path,
    output: &Path,
    options: CaptionRenderOptions<'_>,
) -> Result<(), RenderError> {
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
    let cues = read_srt(captions)?;
    let cards = render_caption_cards(
        &cues,
        output,
        options.width,
        options.height,
        options.vertical,
    )?;
    let mut filter = format!("[0:v]{}[v0]", options.video_filter);
    for (index, cue) in cues.iter().enumerate() {
        let previous = format!("[v{index}]");
        let next = format!("[v{}]", index + 1);
        filter.push_str(&format!(
            ";{previous}[{}:v]overlay=0:0:enable='between(t,{:.3},{:.3})'{next}",
            index + 1,
            cue.start_seconds,
            cue.end_seconds
        ));
    }
    let last = format!("[v{}]", cues.len());
    let mut command = Command::new(ffmpeg_binary());
    command
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(input);
    for card in &cards {
        command
            .args(["-loop", "1", "-framerate", "30", "-i"])
            .arg(card);
    }
    command
        .args(["-filter_complex", &filter, "-map", &last, "-map", "0:a?"])
        .args([
            "-c:v", "libx264", "-pix_fmt", "yuv420p", "-preset", "slow", "-crf", "18", "-c:a",
            "aac", "-ar", "48000",
        ]);
    if let Some(audio_filter) = options.audio_filter {
        command.args(["-af", audio_filter]);
    }
    if options.rec709_output {
        command.args([
            "-color_primaries",
            "bt709",
            "-color_trc",
            "bt709",
            "-colorspace",
            "bt709",
        ]);
    }
    command
        .args(["-shortest", "-movflags", "+faststart"])
        .arg(output);
    let result = command.output().map_err(RenderError::Start)?;
    if result.status.success() && output.is_file() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            String::from_utf8_lossy(&result.stderr).trim().to_string(),
        ))
    }
}

fn preset_video_filter(
    input: &Path,
    width: u32,
    height: u32,
    reframe_anchors: Option<&[ReframeAnchor]>,
) -> Result<(String, bool), RenderError> {
    let metadata = probe(input).map_err(|error| RenderError::Failed(error.to_string()))?;
    let resize = reframe_filter(&metadata, width, height, reframe_anchors)?;
    if metadata.is_hdr == Some(true) {
        if !ffmpeg_has_filter("zscale")? {
            return Err(RenderError::CapabilityMissing(
                "HDR delivery requires FFmpeg built with the zscale filter; install a zimg-enabled FFmpeg build".into(),
            ));
        }
        Ok((
            format!(
                "zscale=transfer=linear:npl=100,format=gbrpf32le,tonemap=tonemap=hable:desat=0,zscale=primaries=bt709:transfer=bt709:matrix=bt709,format=yuv420p,{resize}"
            ),
            true,
        ))
    } else {
        Ok((resize, false))
    }
}

struct SourceVideoFilter {
    filter: String,
    rec709_output: bool,
}

fn source_video_filter(input: &Path) -> Result<SourceVideoFilter, RenderError> {
    let metadata = probe(input).map_err(|error| RenderError::Failed(error.to_string()))?;
    if metadata.is_hdr == Some(true) {
        let (filter, _) = preset_video_filter(input, 1, 1, None)?;
        let tone_map = filter
            .strip_suffix(",scale=1:1:force_original_aspect_ratio=increase,crop=1:1,setsar=1")
            .ok_or_else(|| RenderError::Failed("invalid HDR normalization filter".into()))?;
        Ok(SourceVideoFilter {
            filter: tone_map.to_string(),
            rec709_output: true,
        })
    } else {
        Ok(SourceVideoFilter {
            filter: "null".into(),
            rec709_output: false,
        })
    }
}

fn reframe_filter(
    metadata: &MediaMetadata,
    width: u32,
    height: u32,
    anchors: Option<&[ReframeAnchor]>,
) -> Result<String, RenderError> {
    let Some(anchors) = anchors.filter(|anchors| !anchors.is_empty()) else {
        return Ok(format!(
            "scale={width}:{height}:force_original_aspect_ratio=increase,crop={width}:{height},setsar=1"
        ));
    };
    let input_width = metadata
        .width
        .ok_or_else(|| RenderError::Failed("reframe input has no width".into()))?
        as f64;
    let input_height = metadata
        .height
        .ok_or_else(|| RenderError::Failed("reframe input has no height".into()))?
        as f64;
    let scale = (width as f64 / input_width).max(height as f64 / input_height);
    let scaled_width = (input_width * scale).round().max(width as f64) as u32;
    let scaled_height = (input_height * scale).round().max(height as f64) as u32;
    let crop_x = |anchor: &ReframeAnchor| {
        ((scaled_width as f64 * anchor.center_x.clamp(0.0, 1.0) - width as f64 / 2.0)
            .clamp(0.0, (scaled_width - width) as f64))
        .round() as u32
    };
    let crop_y = |anchor: &ReframeAnchor| {
        ((scaled_height as f64 * anchor.center_y.clamp(0.0, 1.0) - height as f64 / 2.0)
            .clamp(0.0, (scaled_height - height) as f64))
        .round() as u32
    };
    let initial = anchors[0];
    let commands = anchors
        .iter()
        .map(|anchor| {
            format!(
                "{:.3} crop@reframe x {};{:.3} crop@reframe y {}",
                anchor.output_start_ms.max(0) as f64 / 1_000.0,
                crop_x(anchor),
                anchor.output_start_ms.max(0) as f64 / 1_000.0,
                crop_y(anchor)
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    Ok(format!(
        "scale={scaled_width}:{scaled_height},sendcmd=c='{commands}',crop@reframe={width}:{height}:x={}:y={},setsar=1",
        crop_x(&initial),
        crop_y(&initial)
    ))
}

fn preview_video_args() -> Result<Vec<&'static str>, RenderError> {
    if !ffmpeg_has_encoder("h264_videotoolbox")? {
        return Err(RenderError::CapabilityMissing(
            "rough preview rendering requires h264_videotoolbox on macOS".into(),
        ));
    }
    Ok(vec!["-c:v", "h264_videotoolbox", "-b:v", "10M"])
}

fn ffmpeg_has_filter(name: &str) -> Result<bool, RenderError> {
    ffmpeg_list_contains("-filters", name)
}

fn ffmpeg_has_encoder(name: &str) -> Result<bool, RenderError> {
    ffmpeg_list_contains("-encoders", name)
}

fn ffmpeg_list_contains(flag: &str, name: &str) -> Result<bool, RenderError> {
    let output = Command::new(ffmpeg_binary())
        .args(["-hide_banner", flag])
        .output()
        .map_err(RenderError::Start)?;
    if !output.status.success() {
        return Err(RenderError::Failed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.split_whitespace().any(|token| token == name)))
}

fn ffmpeg_binary() -> PathBuf {
    if let Some(path) = std::env::var_os("CUTRIGHT_FFMPEG") {
        return PathBuf::from(path);
    }
    let bundled = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".cutright-tools/ffmpeg-zimg/bin/ffmpeg");
    if bundled.is_file() {
        bundled
    } else {
        PathBuf::from("ffmpeg")
    }
}

fn read_srt(path: &Path) -> Result<Vec<CaptionCue>, RenderError> {
    let source = fs::read_to_string(path).map_err(RenderError::CaptionStart)?;
    source
        .split("\n\n")
        .filter(|chunk| !chunk.trim().is_empty())
        .map(|chunk| {
            let lines = chunk.lines().collect::<Vec<_>>();
            let timing = lines.get(1).ok_or_else(|| {
                RenderError::CaptionFailed("caption cue is missing timing".into())
            })?;
            let (start, end) = timing.split_once(" --> ").ok_or_else(|| {
                RenderError::CaptionFailed("caption cue has invalid timing".into())
            })?;
            let start_seconds = parse_srt_timestamp(start)?;
            let end_seconds = parse_srt_timestamp(end)?;
            let text = lines.get(2..).unwrap_or_default().join("\n");
            if end_seconds <= start_seconds || text.trim().is_empty() {
                return Err(RenderError::CaptionFailed(
                    "caption cue has invalid range or text".into(),
                ));
            }
            Ok(CaptionCue {
                start_seconds,
                end_seconds,
                text,
            })
        })
        .collect()
}

fn parse_srt_timestamp(value: &str) -> Result<f64, RenderError> {
    let normalized = value.replace(',', ".");
    let parts = normalized.split(':').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(RenderError::CaptionFailed(
            "caption timestamp has invalid format".into(),
        ));
    }
    let hours = parts[0]
        .parse::<f64>()
        .map_err(|_| RenderError::CaptionFailed("caption timestamp has invalid hours".into()))?;
    let minutes = parts[1]
        .parse::<f64>()
        .map_err(|_| RenderError::CaptionFailed("caption timestamp has invalid minutes".into()))?;
    let seconds = parts[2]
        .parse::<f64>()
        .map_err(|_| RenderError::CaptionFailed("caption timestamp has invalid seconds".into()))?;
    Ok(hours * 3_600.0 + minutes * 60.0 + seconds)
}

fn render_caption_cards(
    cues: &[CaptionCue],
    output: &Path,
    width: u32,
    height: u32,
    vertical: bool,
) -> Result<Vec<PathBuf>, RenderError> {
    let parent = output.parent().ok_or_else(|| {
        RenderError::CaptionFailed("caption output has no parent directory".into())
    })?;
    let cards_dir = parent.join(".cutright-caption-cards");
    fs::create_dir_all(&cards_dir).map_err(RenderError::CaptionStart)?;
    let worker = caption_card_worker()?;
    cues.iter()
        .enumerate()
        .map(|(index, cue)| {
            let card = cards_dir.join(format!("{:04}.png", index + 1));
            let request = serde_json::json!({
                "output_path": card,
                "width": width,
                "height": height,
                "text": cue.text,
                "vertical": vertical,
            });
            let mut child = Command::new(&worker)
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
            if result.status.success() && card.is_file() {
                Ok(card)
            } else {
                Err(RenderError::CaptionFailed(
                    String::from_utf8_lossy(&result.stderr).trim().to_string(),
                ))
            }
        })
        .collect()
}

fn caption_card_worker() -> Result<PathBuf, RenderError> {
    let worker = std::env::temp_dir().join(format!(
        "cutright-caption-card-{}",
        env!("CARGO_PKG_VERSION")
    ));
    if !worker.is_file() {
        fs::write(&worker, include_bytes!(env!("CUTRIGHT_CAPTION_CARD")))
            .map_err(RenderError::CaptionStart)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&worker, fs::Permissions::from_mode(0o700))
                .map_err(RenderError::CaptionStart)?;
        }
    }
    Ok(worker)
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
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn reframe_filter_schedules_each_anchor() {
        let metadata = MediaMetadata {
            duration_ms: Some(2_000),
            has_video: true,
            has_audio: true,
            width: Some(640),
            height: Some(360),
            rotation_degrees: None,
            is_hdr: Some(false),
            timebase: None,
        };
        let anchors = [
            ReframeAnchor {
                output_start_ms: 0,
                center_x: 0.25,
                center_y: 0.5,
            },
            ReframeAnchor {
                output_start_ms: 1_000,
                center_x: 0.75,
                center_y: 0.5,
            },
        ];
        let filter = reframe_filter(&metadata, 360, 640, Some(&anchors)).expect("filter");
        assert!(filter.contains("sendcmd"));
        assert!(filter.contains("0.000 crop@reframe x"));
        assert!(filter.contains("1.000 crop@reframe x"));
        assert!(filter.contains("crop@reframe=360:640"));
    }

    #[test]
    fn rendered_reframe_follows_timeline_anchors() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cutright-reframe-test-{unique}"));
        fs::create_dir_all(&root).expect("create reframe test directory");
        let input = root.join("input.mp4");
        let captions = root.join("captions.srt");
        let output = root.join("output.mp4");
        let generated = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=red:s=320x360:r=30",
                "-f",
                "lavfi",
                "-i",
                "color=blue:s=320x360:r=30",
                "-filter_complex",
                "[0:v][1:v]hstack",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=1000:sample_rate=48000",
                "-t",
                "2",
                "-c:v",
                "libx264",
                "-c:a",
                "aac",
            ])
            .arg(&input)
            .output()
            .expect("start reframe fixture ffmpeg");
        assert!(generated.status.success());
        fs::write(&captions, "").expect("write empty captions");
        let anchors = [
            ReframeAnchor {
                output_start_ms: 0,
                center_x: 0.25,
                center_y: 0.5,
            },
            ReframeAnchor {
                output_start_ms: 1_000,
                center_x: 0.75,
                center_y: 0.5,
            },
        ];
        render_preset_with_captions_and_reframe(
            &input,
            &captions,
            &output,
            360,
            640,
            true,
            Some(&anchors),
        )
        .expect("render reframed preset");
        let luminance = |time: &str| {
            let frame = Command::new("ffmpeg")
                .args(["-hide_banner", "-loglevel", "error", "-ss", time, "-i"])
                .arg(&output)
                .args([
                    "-frames:v",
                    "1",
                    "-vf",
                    "crop=10:10:0:0,signalstats,metadata=print:file=-",
                    "-f",
                    "null",
                    "-",
                ])
                .output()
                .expect("start reframe luminance ffmpeg");
            assert!(frame.status.success());
            String::from_utf8(frame.stdout)
                .expect("frame luminance is UTF-8")
                .lines()
                .find_map(|line| {
                    line.strip_prefix("lavfi.signalstats.YAVG=")
                        .map(str::parse::<f64>)
                })
                .expect("frame luminance is present")
                .expect("frame luminance is numeric")
        };
        assert!(luminance("0.25") > luminance("1.25") + 20.0);
        fs::remove_dir_all(root).expect("remove reframe test directory");
    }

    #[test]
    fn boundary_probe_renders_a_short_av_edit() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cutright-boundary-test-{unique}"));
        fs::create_dir_all(&root).expect("create boundary test directory");
        let input = root.join("input.mp4");
        let output = root.join("probe.mp4");
        let generated = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=black:s=640x360:r=30",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=1000:sample_rate=48000",
                "-t",
                "3",
                "-c:v",
                "libx264",
                "-c:a",
                "aac",
            ])
            .arg(&input)
            .output()
            .expect("start boundary fixture ffmpeg");
        assert!(generated.status.success());
        render_boundary_probe(&input, 1_500, &output).expect("render boundary probe");
        let metadata = probe(&output).expect("probe rendered boundary");
        assert!(metadata.has_video && metadata.has_audio);
        assert!(metadata
            .duration_ms
            .is_some_and(|duration| (1_500..=1_700).contains(&duration)));
        fs::remove_dir_all(root).expect("remove boundary test directory");
    }

    #[test]
    fn hdr_preview_is_tone_mapped_and_tagged_rec709() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cutright-hdr-test-{unique}"));
        fs::create_dir_all(&root).expect("create HDR test directory");
        let input = root.join("input-hdr.mp4");
        let output = root.join("preview.mp4");
        let generated = Command::new(ffmpeg_binary())
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=s=320x180:r=30",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=1000:sample_rate=48000",
                "-t",
                "1",
                "-c:v",
                "hevc_videotoolbox",
                "-pix_fmt",
                "p010le",
                "-color_primaries",
                "bt2020",
                "-color_trc",
                "smpte2084",
                "-colorspace",
                "bt2020nc",
                "-bsf:v",
                "hevc_metadata=colour_primaries=9:transfer_characteristics=16:matrix_coefficients=9",
                "-c:a",
                "aac",
            ])
            .arg(&input)
            .output()
            .expect("start HDR fixture ffmpeg");
        assert!(
            generated.status.success(),
            "HDR fixture ffmpeg failed: {}",
            String::from_utf8_lossy(&generated.stderr)
        );
        assert_eq!(probe(&input).expect("probe HDR fixture").is_hdr, Some(true));

        render_segments(
            &input,
            &[RenderSegment {
                start_ms: 0,
                end_ms: 900,
            }],
            &output,
        )
        .expect("tone-map HDR preview");

        let tags = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=color_transfer,color_primaries,color_space",
                "-of",
                "default=noprint_wrappers=1",
            ])
            .arg(&output)
            .output()
            .expect("probe HDR preview tags");
        assert!(tags.status.success());
        let tags = String::from_utf8(tags.stdout).expect("tags are UTF-8");
        assert!(tags.contains("color_transfer=bt709"), "{tags}");
        assert!(tags.contains("color_primaries=bt709"), "{tags}");
        assert!(tags.contains("color_space=bt709"), "{tags}");
        assert_eq!(
            probe(&output).expect("probe HDR preview").is_hdr,
            Some(false)
        );
        fs::remove_dir_all(root).expect("remove HDR test directory");
    }

    #[test]
    fn captioned_preset_shows_each_cue_only_during_its_interval() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cutright-caption-test-{unique}"));
        fs::create_dir_all(&root).expect("create test directory");
        let input = root.join("input.mp4");
        let captions = root.join("captions.srt");
        let output = root.join("output.mp4");
        let generated = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=black:s=640x360:r=30",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=1000:sample_rate=48000",
                "-t",
                "3",
                "-c:v",
                "libx264",
                "-c:a",
                "aac",
            ])
            .arg(&input)
            .output()
            .expect("start ffmpeg fixture");
        assert!(
            generated.status.success(),
            "fixture ffmpeg failed: {}",
            String::from_utf8_lossy(&generated.stderr)
        );
        fs::write(
            &captions,
            "1\n00:00:00,500 --> 00:00:01,000\nFIRST CUE\n\n2\n00:00:02,000 --> 00:00:02,500\nSECOND CUE\n",
        )
        .expect("write captions");
        render_preset_with_captions(&input, &captions, &output, 640, 360, false)
            .expect("render captioned preset");
        let frame_luminance = |time: &str| {
            let frame = Command::new("ffmpeg")
                .args(["-hide_banner", "-loglevel", "error", "-ss", time, "-i"])
                .arg(&output)
                .args([
                    "-frames:v",
                    "1",
                    "-vf",
                    "signalstats,metadata=print:file=-",
                    "-f",
                    "null",
                    "-",
                ])
                .output()
                .expect("start frame luminance ffmpeg");
            assert!(frame.status.success());
            String::from_utf8(frame.stdout)
                .expect("frame luminance is UTF-8")
                .lines()
                .find_map(|line| {
                    line.strip_prefix("lavfi.signalstats.YAVG=")
                        .map(str::parse::<f64>)
                })
                .expect("frame luminance is present")
                .expect("frame luminance is numeric")
        };
        let before = frame_luminance("0.25");
        let first_cue = frame_luminance("0.75");
        let gap = frame_luminance("1.50");
        let second_cue = frame_luminance("2.25");
        let after = frame_luminance("2.75");
        assert!(before < 17.0 && gap < 17.0 && after < 17.0);
        assert!(
            first_cue > before + 0.1 && second_cue > before + 0.1,
            "luminance before={before}, first={first_cue}, gap={gap}, second={second_cue}, after={after}"
        );
        fs::remove_dir_all(root).expect("remove test directory");
    }
}
