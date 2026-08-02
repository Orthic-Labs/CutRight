mod toolchain;

use serde::Deserialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;
use video_core::process_runner::{CancellationToken, ManagedChild, ProcessRunError, ProcessSpec};
use video_core::Timebase;

pub use toolchain::{
    resolve as resolve_toolchain, MediaCapabilities, MediaToolchain, ToolchainError,
};

// --- Shared process runner budgets (hardening plan §10.1) -----------------
//
// Every external command spawned by this module carries a mandatory
// timeout; none may wait indefinitely. Budgets are grouped by class of work
// rather than one flat number, and the render classes additionally scale
// with the duration of media actually being processed (`scaled_timeout` /
// `duration_scaled_timeout` below) so a ten-second clip and a two-hour
// source don't share one guess. Every multiplier below is deliberately
// generous — several times slower than expected real-world throughput — so
// a legitimate slow encode is never clipped; only a genuinely hung process
// is.

/// `ffprobe -show_format -show_streams`: reads container/stream headers,
/// never full-decodes. The smallest budget in this module; a slow network
/// mount or pathological file is still bounded.
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);
/// ffprobe's JSON can grow with stream/tag count but stays well under this
/// even for unusually chaptered/multi-track sources.
const PROBE_STDOUT_CAP_BYTES: usize = 4 * 1024 * 1024;

/// Generic stdout cap for ffmpeg invocations that write their result to a
/// file, not stdout (`-loglevel error` means stdout carries nothing under
/// normal operation).
const STDOUT_CAP_BYTES: usize = 1024 * 1024;
/// Generic stderr cap: `-loglevel error` keeps normal-path stderr tiny, but
/// a real failure (filter graph error, codec probe dump) can be verbose.
const STDERR_CAP_BYTES: usize = 8 * 1024 * 1024;

/// Fixed budget for operations whose cost does not scale with source
/// duration: single-frame extraction, a static waveform/evidence image
/// composite, and the fixed ~1.6s two-segment boundary-probe clip. Two
/// minutes covers seek/decode cost on a slow disk with headroom to spare.
const SHORT_OP_TIMEOUT: Duration = Duration::from_secs(2 * 60);

/// Hardware-accelerated (h264_videotoolbox) rough/preview renders — segment
/// trims and concatenation for in-app scrubbing, not final delivery
/// quality. Floor plus a per-second-of-output-content multiplier: hardware
/// encode is normally much faster than real time, so 3s of budget per 1s of
/// assembled output is several times the expected cost.
const PREVIEW_RENDER_FLOOR: Duration = Duration::from_secs(5 * 60);
const PREVIEW_RENDER_PER_SOURCE_SECOND: Duration = Duration::from_secs(3);

/// Waveform image rendering decodes the audio it draws. `render_waveform`
/// covers a whole source; `render_waveform_range` and the boundary probe
/// cover a caller-bounded slice. Budget per second of audio decoded is
/// small since this is decode-only, no encode.
const WAVEFORM_RENDER_FLOOR: Duration = Duration::from_secs(60);
const WAVEFORM_PER_SOURCE_SECOND: Duration = Duration::from_millis(500);

/// Full-quality libx264 delivery renders (`render_to_preset`,
/// `render_preset_with_captions*`, `render_subtitled`), potentially at 4K
/// with per-cue caption overlay filters. This is the largest budget class:
/// floor plus 2s of wall-clock budget per 1s of source, well above the
/// `veryfast`/`slow` libx264 presets' expected real-time-ish throughput.
const FINAL_RENDER_FLOOR: Duration = Duration::from_secs(10 * 60);
const FINAL_RENDER_PER_SOURCE_SECOND: Duration = Duration::from_secs(2);

/// `measured_loudnorm_filter`'s two-pass loudnorm measurement decodes the
/// full source once with no encode; budget accordingly, smaller than a
/// render but larger than a probe.
const LOUDNESS_MEASURE_FLOOR: Duration = Duration::from_secs(5 * 60);
const LOUDNESS_PER_SOURCE_SECOND: Duration = Duration::from_millis(300);

/// `extract_audio_f32`'s raw decode + resample to f32le PCM: fast, decode
/// only, no filtering beyond resample/downmix.
const AUDIO_EXTRACT_FLOOR: Duration = Duration::from_secs(5 * 60);
const AUDIO_EXTRACT_PER_SOURCE_SECOND: Duration = Duration::from_millis(200);

/// Per-cue caption-card worker: renders one static PNG card. Cheap and
/// independent of source media duration.
const CAPTION_CARD_TIMEOUT: Duration = Duration::from_secs(30);

/// `floor.max(per_source_second * work_ms / 1000)` — the shared scaling
/// rule behind every duration-proportional budget above.
fn scaled_timeout(work_ms: i64, floor: Duration, per_source_second: Duration) -> Duration {
    let work_ms = work_ms.max(0) as u64;
    let scaled = per_source_second
        .checked_mul((work_ms / 1000) as u32)
        .unwrap_or(Duration::MAX)
        .saturating_add(per_source_second.mul_f64((work_ms % 1000) as f64 / 1_000.0));
    floor.max(scaled)
}

/// [`scaled_timeout`], but probes `input` for its duration first. If the
/// probe itself fails, falls back to `floor` — the eventual ffmpeg call
/// remains bounded either way, just by the smaller flat budget instead of a
/// duration-aware one.
fn duration_scaled_timeout(input: &Path, floor: Duration, per_source_second: Duration) -> Duration {
    let duration_ms = probe(input)
        .ok()
        .and_then(|metadata| metadata.duration_ms)
        .unwrap_or(0);
    scaled_timeout(duration_ms, floor, per_source_second)
}

/// [`duration_scaled_timeout`], but reuses an already-resolved
/// [`MediaToolchain`] (§10.3: resolve once, reuse) instead of probing
/// through a fresh toolchain resolution.
fn duration_scaled_timeout_with_toolchain(
    input: &Path,
    toolchain: &MediaToolchain,
    floor: Duration,
    per_source_second: Duration,
) -> Duration {
    let duration_ms = probe_with_toolchain(input, toolchain)
        .ok()
        .and_then(|metadata| metadata.duration_ms)
        .unwrap_or(0);
    scaled_timeout(duration_ms, floor, per_source_second)
}

/// Explicit environment allow-list (§10.1) for every ffmpeg/ffprobe/caption-
/// worker invocation in this module: `PATH`/`HOME` cover typical dynamic
/// library and cache-directory resolution, `TMPDIR` is where these tools
/// place their own scratch files on macOS.
fn media_env_allow() -> Vec<(String, String)> {
    ["PATH", "HOME", "TMPDIR"]
        .iter()
        .filter_map(|key| {
            std::env::var(key)
                .ok()
                .map(|value| (key.to_string(), value))
        })
        .collect()
}

/// Convert a fixed array of `&str` literals into owned `String` args, kept
/// as a tiny helper so call sites building a mixed literal/computed
/// [`ProcessSpec::args`] list stay readable.
fn string_args<const N: usize>(parts: [&str; N]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

/// The rec709 output color tag args repeated by every renderer that
/// tone-maps an HDR source down for delivery.
fn rec709_output_args() -> Vec<String> {
    string_args([
        "-color_primaries",
        "bt709",
        "-color_trc",
        "bt709",
        "-colorspace",
        "bt709",
    ])
}

/// Run one bounded ffmpeg/ffprobe invocation through the shared process
/// runner and translate a nonzero exit, spawn failure, or timeout into the
/// caller's error type. On success returns the full
/// [`video_core::process_runner::ProcessOutcome`] (some callers, like
/// `measured_loudnorm_filter`, need stderr even on a zero exit). On failure,
/// `map_failure` turns the trimmed, possibly-truncated, possibly-signalled
/// stderr text into the caller's "process failed" variant;
/// spawn/timeout/cancellation map through `ProcessRunError` via each error
/// enum's `Process` variant (`#[from]`).
fn run_media_command<E, F>(
    executable: &Path,
    args: Vec<String>,
    timeout: Duration,
    map_failure: F,
) -> Result<video_core::process_runner::ProcessOutcome, E>
where
    E: From<ProcessRunError>,
    F: FnOnce(String) -> E,
{
    let spec = ProcessSpec {
        executable: executable.to_path_buf(),
        args,
        env_allow: media_env_allow(),
        working_dir: None,
        timeout,
        stdout_cap_bytes: STDOUT_CAP_BYTES,
        stderr_cap_bytes: STDERR_CAP_BYTES,
    };
    let outcome = video_core::process_runner::run_process(&spec, &CancellationToken::new())?;
    if outcome.success() {
        Ok(outcome)
    } else {
        let mut message = String::from_utf8_lossy(&outcome.stderr).trim().to_string();
        if outcome.stderr_truncated {
            message.push_str(" ...[stderr truncated]");
        }
        if let Some(signal) = outcome.signal {
            message.push_str(&format!(" (terminated by signal {signal})"));
        }
        Err(map_failure(message))
    }
}

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
    #[error("could not resolve a matched ffmpeg/ffprobe toolchain: {0}")]
    Toolchain(#[from] ToolchainError),
    #[error("could not materialize embedded worker: {0}")]
    WorkerMaterialize(#[from] video_core::ContentStoreError),
    #[error("ffmpeg process error: {0}")]
    Process(#[from] ProcessRunError),
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
    #[error("could not resolve a matched ffmpeg/ffprobe toolchain: {0}")]
    Toolchain(#[from] ToolchainError),
    #[error("ffmpeg process error: {0}")]
    Process(#[from] ProcessRunError),
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
fn probe_with_toolchain(
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

    let toolchain = toolchain::resolve()?;
    let source_filter = source_video_filter(input, &toolchain)?;
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

    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args([
        "-filter_complex",
        &filter,
        "-map",
        "[outv]",
        "-map",
        "[outa]",
    ]));
    args.extend(
        preview_video_args(&toolchain)?
            .into_iter()
            .map(str::to_string),
    );
    args.extend(string_args(["-c:a", "aac"]));
    if source_filter.rec709_output {
        args.extend(rec709_output_args());
    }
    args.extend(string_args(["-movflags", "+faststart"]));
    args.push(output.display().to_string());
    let total_ms: i64 = segments
        .iter()
        .map(|segment| segment.end_ms - segment.start_ms)
        .sum();
    let timeout = scaled_timeout(
        total_ms,
        PREVIEW_RENDER_FLOOR,
        PREVIEW_RENDER_PER_SOURCE_SECOND,
    );
    run_media_command(&toolchain.ffmpeg, args, timeout, RenderError::Failed)?;
    Ok(())
}

/// Same as [`render_segments`], but also returns a [`video_core::StageReceipt`]
/// recording the resolved ffmpeg/ffprobe toolchain identity plus the input
/// and output content hashes (hardening plan §10.4). Additive: existing
/// callers of `render_segments` are unaffected.
pub fn render_segments_with_receipt(
    input: &Path,
    segments: &[RenderSegment],
    output: &Path,
) -> Result<video_core::StageReceipt, RenderError> {
    render_segments(input, segments, output)?;
    build_receipt(
        "render.segments",
        input,
        &serde_json::json!({
            "segments": segments
                .iter()
                .map(|segment| serde_json::json!({
                    "start_ms": segment.start_ms,
                    "end_ms": segment.end_ms,
                }))
                .collect::<Vec<_>>(),
        }),
        output,
    )
}

/// Build a [`video_core::StageReceipt`] for a completed ffmpeg render/probe
/// stage: hashes `input`/`output`, hashes `parameters`, and records the
/// resolved [`MediaToolchain`] identity under the `"ffmpeg"` key.
fn build_receipt(
    stage: &str,
    input: &Path,
    parameters: &serde_json::Value,
    output: &Path,
) -> Result<video_core::StageReceipt, RenderError> {
    build_receipt_multi(stage, &[input], parameters, output)
}

fn build_receipt_multi(
    stage: &str,
    inputs: &[&Path],
    parameters: &serde_json::Value,
    output: &Path,
) -> Result<video_core::StageReceipt, RenderError> {
    let toolchain = toolchain::resolve()?;
    let mut toolchains = std::collections::BTreeMap::new();
    toolchains.insert("ffmpeg".to_string(), toolchain.identity());
    video_core::StageReceipt::build(
        stage,
        env!("CARGO_PKG_VERSION"),
        inputs,
        parameters,
        toolchains,
        &[output],
    )
    .map_err(|error| RenderError::Failed(error.to_string()))
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
    let toolchain = toolchain::resolve()?;
    let source_filters = inputs
        .iter()
        .map(|input| source_video_filter(input, &toolchain))
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
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y"]);
    for input in inputs {
        args.push("-i".to_string());
        args.push(input.display().to_string());
    }
    args.extend(string_args([
        "-filter_complex",
        &filter,
        "-map",
        "[outv]",
        "-map",
        "[outa]",
    ]));
    args.extend(
        preview_video_args(&toolchain)?
            .into_iter()
            .map(str::to_string),
    );
    args.extend(string_args(["-c:a", "aac"]));
    if rec709_output {
        args.extend(rec709_output_args());
    }
    args.extend(string_args(["-movflags", "+faststart"]));
    args.push(output.display().to_string());
    let total_ms: i64 = segments
        .iter()
        .map(|segment| segment.end_ms - segment.start_ms)
        .sum();
    let timeout = scaled_timeout(
        total_ms,
        PREVIEW_RENDER_FLOOR,
        PREVIEW_RENDER_PER_SOURCE_SECOND,
    );
    run_media_command(&toolchain.ffmpeg, args, timeout, RenderError::Failed)?;
    Ok(())
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
    let toolchain = toolchain::resolve()?;
    let metadata = probe_with_toolchain(input, &toolchain)
        .map_err(|error| RenderError::Failed(error.to_string()))?;
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
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args([
        "-filter_complex",
        &filter,
        "-map",
        "[outv]",
        "-map",
        "[outa]",
    ]));
    args.extend(
        preview_video_args(&toolchain)?
            .into_iter()
            .map(str::to_string),
    );
    args.extend(string_args(["-c:a", "aac", "-movflags", "+faststart"]));
    args.push(output.display().to_string());
    run_media_command(
        &toolchain.ffmpeg,
        args,
        SHORT_OP_TIMEOUT,
        RenderError::Failed,
    )?;
    if output.is_file() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            "ffmpeg exited successfully but did not produce the boundary probe output".into(),
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
    let toolchain = toolchain::resolve()?;
    let (filter, rec709_output) = preset_video_filter(input, width, height, None, &toolchain)?;
    let audio_filter = measured_loudnorm_filter(input, &toolchain)?;
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args([
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
    ]));
    if rec709_output {
        args.extend(rec709_output_args());
    }
    args.push(output.display().to_string());
    let timeout = duration_scaled_timeout_with_toolchain(
        input,
        &toolchain,
        FINAL_RENDER_FLOOR,
        FINAL_RENDER_PER_SOURCE_SECOND,
    );
    run_media_command(&toolchain.ffmpeg, args, timeout, RenderError::Failed)?;
    Ok(())
}

/// See [`preset_video_filter`]'s doc comment on `toolchain`.
fn measured_loudnorm_filter(
    input: &Path,
    toolchain: &MediaToolchain,
) -> Result<String, RenderError> {
    let mut args = string_args(["-hide_banner", "-nostats", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args([
        "-af",
        "highpass=f=80,loudnorm=I=-14:TP=-1:LRA=11:print_format=json",
        "-f",
        "null",
        "-",
    ]));
    let timeout = duration_scaled_timeout_with_toolchain(
        input,
        toolchain,
        LOUDNESS_MEASURE_FLOOR,
        LOUDNESS_PER_SOURCE_SECOND,
    );
    let measurement = run_media_command(&toolchain.ffmpeg, args, timeout, RenderError::Failed)?;
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
    let ffmpeg = ffmpeg_path()?;
    let mut args = string_args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-ss",
        &timestamp,
        "-i",
    ]);
    args.push(input.display().to_string());
    args.extend(string_args(["-frames:v", "1", "-q:v", "2"]));
    args.push(output.display().to_string());
    run_media_command(&ffmpeg, args, SHORT_OP_TIMEOUT, RenderError::Failed)?;
    if output.is_file() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            "ffmpeg exited successfully but did not produce the extracted frame".into(),
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
    let ffmpeg = ffmpeg_path()?;
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args([
        "-filter_complex",
        "showwavespic=s=1200x240:colors=0x44D7B6",
        "-frames:v",
        "1",
    ]));
    args.push(output.display().to_string());
    let timeout = duration_scaled_timeout(input, WAVEFORM_RENDER_FLOOR, WAVEFORM_PER_SOURCE_SECOND);
    run_media_command(&ffmpeg, args, timeout, RenderError::Failed)?;
    if output.is_file() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            "ffmpeg exited successfully but did not produce the waveform image".into(),
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
    let ffmpeg = ffmpeg_path()?;
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args(["-filter_complex", &filter, "-frames:v", "1"]));
    args.push(output.display().to_string());
    let timeout = scaled_timeout(
        end_ms - start_ms,
        SHORT_OP_TIMEOUT,
        WAVEFORM_PER_SOURCE_SECOND,
    );
    run_media_command(&ffmpeg, args, timeout, RenderError::Failed)?;
    if output.is_file() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            "ffmpeg exited successfully but did not produce the waveform range image".into(),
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
    let ffmpeg = ffmpeg_path()?;
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y"]);
    for frame in frames {
        args.push("-i".to_string());
        args.push(frame.display().to_string());
    }
    args.push("-i".to_string());
    args.push(waveform.display().to_string());
    args.extend(string_args([
        "-filter_complex",
        "[0:v]scale=400:225[a];[1:v]scale=400:225[b];[2:v]scale=400:225[c];[a][b][c]hstack=inputs=3[filmstrip];[3:v]scale=1200:180[wave];[filmstrip][wave]vstack=inputs=2",
        "-frames:v",
        "1",
    ]));
    args.push(output.display().to_string());
    run_media_command(&ffmpeg, args, SHORT_OP_TIMEOUT, RenderError::Failed)?;
    if output.is_file() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            "ffmpeg exited successfully but did not produce the decision evidence image".into(),
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
    let ffmpeg = ffmpeg_path()?;
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

pub fn render_subtitled(input: &Path, captions: &Path, output: &Path) -> Result<(), RenderError> {
    let toolchain = toolchain::resolve()?;
    let metadata = probe_with_toolchain(input, &toolchain)
        .map_err(|error| RenderError::Failed(error.to_string()))?;
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
        &toolchain,
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
    let toolchain = toolchain::resolve()?;
    let (filter, rec709_output) =
        preset_video_filter(input, width, height, reframe_anchors, &toolchain)?;
    let audio_filter = measured_loudnorm_filter(input, &toolchain)?;
    render_captioned(
        input,
        captions,
        output,
        &toolchain,
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

/// Same as [`render_preset_with_captions_and_reframe`], but also returns a
/// [`video_core::StageReceipt`] (hardening plan §10.4). Additive: existing
/// callers are unaffected.
#[allow(clippy::too_many_arguments)]
pub fn render_preset_with_captions_and_reframe_with_receipt(
    input: &Path,
    captions: &Path,
    output: &Path,
    width: u32,
    height: u32,
    vertical: bool,
    reframe_anchors: Option<&[ReframeAnchor]>,
) -> Result<video_core::StageReceipt, RenderError> {
    render_preset_with_captions_and_reframe(
        input,
        captions,
        output,
        width,
        height,
        vertical,
        reframe_anchors,
    )?;
    build_receipt_multi(
        "render.finish_captioned",
        &[input, captions],
        &serde_json::json!({
            "width": width,
            "height": height,
            "vertical": vertical,
            "reframe_anchor_count": reframe_anchors.map(<[_]>::len).unwrap_or(0),
        }),
        output,
    )
}

fn render_captioned(
    input: &Path,
    captions: &Path,
    output: &Path,
    toolchain: &MediaToolchain,
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
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    for card in &cards {
        args.extend(string_args(["-loop", "1", "-framerate", "30", "-i"]));
        args.push(card.display().to_string());
    }
    args.extend(string_args([
        "-filter_complex",
        &filter,
        "-map",
        &last,
        "-map",
        "0:a?",
    ]));
    args.extend(string_args([
        "-c:v", "libx264", "-pix_fmt", "yuv420p", "-preset", "slow", "-crf", "18", "-c:a", "aac",
        "-ar", "48000",
    ]));
    if let Some(audio_filter) = options.audio_filter {
        args.extend(string_args(["-af", audio_filter]));
    }
    if options.rec709_output {
        args.extend(rec709_output_args());
    }
    args.extend(string_args(["-shortest", "-movflags", "+faststart"]));
    args.push(output.display().to_string());
    let timeout = duration_scaled_timeout_with_toolchain(
        input,
        toolchain,
        FINAL_RENDER_FLOOR,
        FINAL_RENDER_PER_SOURCE_SECOND,
    );
    run_media_command(&toolchain.ffmpeg, args, timeout, RenderError::Failed)?;
    if output.is_file() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            "ffmpeg exited successfully but did not produce the captioned output".into(),
        ))
    }
}

/// `toolchain` is threaded in rather than re-resolved here so a caller that
/// already resolved one for this operation (§10.3: resolve once, reuse)
/// doesn't pay for another `ffmpeg -filters` capability probe.
fn preset_video_filter(
    input: &Path,
    width: u32,
    height: u32,
    reframe_anchors: Option<&[ReframeAnchor]>,
    toolchain: &MediaToolchain,
) -> Result<(String, bool), RenderError> {
    let metadata = probe_with_toolchain(input, toolchain)
        .map_err(|error| RenderError::Failed(error.to_string()))?;
    let resize = reframe_filter(&metadata, width, height, reframe_anchors)?;
    if metadata.is_hdr == Some(true) {
        if !toolchain.capabilities.has_zscale {
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

/// See [`preset_video_filter`]'s doc comment on `toolchain`.
fn source_video_filter(
    input: &Path,
    toolchain: &MediaToolchain,
) -> Result<SourceVideoFilter, RenderError> {
    let metadata = probe_with_toolchain(input, toolchain)
        .map_err(|error| RenderError::Failed(error.to_string()))?;
    if metadata.is_hdr == Some(true) {
        let (filter, _) = preset_video_filter(input, 1, 1, None, toolchain)?;
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

/// See [`preset_video_filter`]'s doc comment on `toolchain`.
fn preview_video_args(toolchain: &MediaToolchain) -> Result<Vec<&'static str>, RenderError> {
    if !toolchain.capabilities.has_h264_videotoolbox {
        return Err(RenderError::CapabilityMissing(
            "rough preview rendering requires h264_videotoolbox on macOS".into(),
        ));
    }
    Ok(vec!["-c:v", "h264_videotoolbox", "-b:v", "10M"])
}

/// The `ffmpeg` half of the resolved, verified toolchain pair (§10.3). Every
/// call site in this module goes through this instead of a bare
/// `Command::new("ffmpeg")`, so `ffmpeg` and `ffprobe` (used by [`probe`])
/// always come from the same resolved build.
fn ffmpeg_path() -> Result<PathBuf, ToolchainError> {
    Ok(toolchain::resolve()?.ffmpeg)
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
            run_caption_card_worker(&worker, &request, &card)?;
            Ok(card)
        })
        .collect()
}

/// Run the embedded caption-card sidecar for one cue through the shared
/// process runner (§10.1): a JSON request over stdin, a bounded timeout, and
/// kill-tree teardown on timeout, matching every other external process in
/// this module. `run_process` can't drive this call site directly because
/// it doesn't expose a writable stdin (its callers today all pass arguments
/// instead), so this uses [`ManagedChild`] — the same streaming-process
/// primitive the HeardRight engine session in `video-providers` is built
/// on — for spawn/env-allow/kill-tree, plus a manual bounded poll loop for
/// "wait for this one-shot child to exit" since `ManagedChild` is written
/// for long-lived sessions and doesn't expose that itself.
fn run_caption_card_worker(
    worker: &Path,
    request: &serde_json::Value,
    card: &Path,
) -> Result<(), RenderError> {
    let spec = ProcessSpec {
        executable: worker.to_path_buf(),
        args: Vec::new(),
        env_allow: media_env_allow(),
        working_dir: None,
        timeout: CAPTION_CARD_TIMEOUT,
        stdout_cap_bytes: STDOUT_CAP_BYTES,
        stderr_cap_bytes: STDERR_CAP_BYTES,
    };
    let (mut managed, mut stdin, stdout) =
        ManagedChild::spawn(&spec).map_err(RenderError::Process)?;
    // The worker's stdout was `Stdio::null()` in the pre-§10.1 code; nothing
    // is expected on it, but `ManagedChild` always pipes stdout, so drain it
    // on a background thread rather than risk a full pipe buffer blocking
    // the child if it ever does write something.
    let drain = thread::spawn(move || {
        let mut sink = stdout;
        let _ = std::io::copy(&mut sink, &mut std::io::sink());
    });
    let request_bytes = serde_json::to_vec(request).expect("caption request JSON");
    let write_result = stdin.write_all(&request_bytes).and_then(|()| stdin.flush());
    drop(stdin); // close the write end so the worker sees EOF
    if let Err(error) = write_result {
        managed.kill_tree();
        let _ = drain.join();
        return Err(RenderError::CaptionStart(error));
    }

    let poll_interval = Duration::from_millis(20);
    let start = Instant::now();
    loop {
        if managed.has_exited() {
            break;
        }
        if start.elapsed() >= CAPTION_CARD_TIMEOUT {
            managed.kill_tree();
            let _ = drain.join();
            return Err(RenderError::CaptionFailed(format!(
                "caption card worker timed out after {:?}",
                CAPTION_CARD_TIMEOUT
            )));
        }
        thread::sleep(poll_interval);
    }
    let _ = drain.join();

    if card.is_file() {
        Ok(())
    } else {
        let (stderr, truncated) = managed.stderr_snapshot();
        let mut message = String::from_utf8_lossy(&stderr).trim().to_string();
        if truncated {
            message.push_str(" ...[stderr truncated]");
        }
        Err(RenderError::CaptionFailed(message))
    }
}

/// Materialize the embedded caption-card sidecar (hardening plan §10.2) at a
/// path addressed by the content hash of its embedded bytes, not by crate
/// version — a worker source edit with no crate version bump lands at a new
/// path instead of silently reusing a stale binary, and the on-disk bytes
/// are re-verified against that hash before every reuse.
fn caption_card_worker() -> Result<PathBuf, RenderError> {
    Ok(video_core::materialize_worker(
        include_bytes!(env!("CUTRIGHT_CAPTION_CARD")),
        "caption-card",
    )?)
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
    use std::process::Command;
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
        let generated = Command::new(ffmpeg_path().expect("resolve ffmpeg toolchain"))
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

    /// §10.1: no command driven through the shared runner may wait
    /// indefinitely. This drives `run_media_command` — the wrapper every
    /// ffmpeg/ffprobe call site in this module goes through — with a fake
    /// hanging "executable" (`/bin/sh -c 'sleep 5'`, the same fixture shape
    /// `video_core::process_runner`'s own tests use) and a short timeout,
    /// and asserts the call returns promptly with a timeout error instead of
    /// blocking for the full 5 seconds.
    #[test]
    fn run_media_command_kills_a_hanging_process_at_its_timeout() {
        let start = std::time::Instant::now();
        let result = run_media_command(
            Path::new("/bin/sh"),
            vec!["-c".to_string(), "sleep 5".to_string()],
            Duration::from_millis(200),
            RenderError::Failed,
        );
        let elapsed = start.elapsed();
        assert!(
            matches!(
                result,
                Err(RenderError::Process(ProcessRunError::Timeout(_, _)))
            ),
            "expected a timeout error, got {result:?}"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "hung process was not killed near its timeout, took {elapsed:?}"
        );
    }

    /// Companion to the timeout test above: the same `run_media_command`
    /// path used by every render/probe function in this module still
    /// completes a normal, quick command successfully end to end (spawn,
    /// bounded wait, exit-code check, stdout capture).
    #[test]
    fn run_media_command_succeeds_on_a_normal_process() {
        let outcome = run_media_command(
            Path::new("/bin/sh"),
            vec!["-c".to_string(), "echo cutright-ok".to_string()],
            Duration::from_secs(5),
            RenderError::Failed,
        )
        .expect("run a quick, well-behaved command");
        assert_eq!(
            String::from_utf8_lossy(&outcome.stdout).trim(),
            "cutright-ok"
        );
    }

    /// End-to-end proof that a real render call through the new plumbing
    /// (§10.1 process runner + §10.3 resolved toolchain) still produces
    /// correct output: extract a single frame from a short fixture clip.
    #[test]
    fn extract_frame_still_succeeds_through_the_shared_process_runner() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cutright-extract-frame-test-{unique}"));
        fs::create_dir_all(&root).expect("create test directory");
        let input = root.join("input.mp4");
        let output = root.join("frame.jpg");
        let generated = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=red:s=320x180:r=30",
                "-t",
                "1",
                "-c:v",
                "libx264",
            ])
            .arg(&input)
            .output()
            .expect("start extract-frame fixture ffmpeg");
        assert!(generated.status.success());

        extract_frame(&input, 500, &output).expect("extract a frame");
        assert!(output.is_file());
        fs::remove_dir_all(root).expect("remove test directory");
    }
}
