use crate::ProjectError;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) struct LoudnessMeasurement {
    pub(crate) integrated_lufs: Option<f64>,
    pub(crate) true_peak_dbtp: Option<f64>,
    pub(crate) clipped_samples: u64,
}

pub(crate) fn qa_ffmpeg_bin() -> PathBuf {
    std::env::var_os("CUTRIGHT_FFMPEG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("ffmpeg"))
}

/// Decodes the whole file end-to-end and requires a clean exit with no
/// decoder errors on stderr — catches truncated/corrupt renders that probe
/// alone (container metadata only) would not.
pub(crate) fn decode_through_end(path: &Path) -> Result<bool, ProjectError> {
    let output = Command::new(qa_ffmpeg_bin())
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-f", "null", "-"])
        .output()
        .map_err(|error| {
            ProjectError::InvalidState(format!("ffmpeg decode check could not start: {error}"))
        })?;
    Ok(output.status.success() && output.stderr.is_empty())
}

/// Runs blackdetect + freezedetect over just the tail window of the render
/// so a deliverable that silently drops to black or freezes at the very end
/// (a common truncated-render symptom) fails QA instead of shipping.
pub(crate) fn detect_tail_black_or_frozen(
    path: &Path,
    duration_ms: i64,
    tail_ms: i64,
) -> Result<(bool, bool), ProjectError> {
    let start_seconds = (duration_ms - tail_ms).max(0) as f64 / 1000.0;
    let output = Command::new(qa_ffmpeg_bin())
        .args(["-v", "info", "-ss"])
        .arg(format!("{start_seconds:.3}"))
        .arg("-i")
        .arg(path)
        .args([
            "-vf",
            "blackdetect=d=0.5:pic_th=0.98,freezedetect=n=-60dB:d=0.5",
            "-an",
            "-f",
            "null",
            "-",
        ])
        .output()
        .map_err(|error| {
            ProjectError::InvalidState(format!("ffmpeg tail check could not start: {error}"))
        })?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok((
        stderr.contains("black_start"),
        stderr.contains("freeze_start"),
    ))
}

/// Measures integrated loudness (LUFS), true peak (dBTP) and clipped samples
/// for a final deliverable.
///
/// This delegates to `video_media::measure_loudness_and_clipping` rather than
/// parsing `astats` itself. The old implementation summed an
/// `astats`-reported "Number of clipped samples:" line — but ffmpeg 8.1.2's
/// `astats` no longer emits that metric at all (`ffmpeg -h filter=astats`
/// mentions clipping zero times), so the parse always found nothing and the
/// QA clipping check silently reported 0 for every deliverable. A gate that
/// cannot fail is worse than no gate, because it reads as evidence. The
/// shared implementation scans raw `f32le` samples for full-scale magnitude
/// instead, which does not depend on an ffmpeg build's metric names.
pub(crate) fn measure_loudness(path: &Path) -> Result<LoudnessMeasurement, ProjectError> {
    let measured = video_media::measure_loudness_and_clipping(path).map_err(|error| {
        ProjectError::InvalidState(format!("ffmpeg loudness check failed: {error}"))
    })?;
    Ok(LoudnessMeasurement {
        integrated_lufs: measured.integrated_lufs,
        true_peak_dbtp: measured.true_peak_dbtp,
        clipped_samples: measured.clipped_samples,
    })
}

// The `astats`/`ebur128` stderr parsers that used to live here are gone with
// their only caller: loudness, true-peak and clipping measurement now goes
// through `video_media::measure_loudness_and_clipping`, so QA and the audio
// finish stage read the same numbers from the same implementation instead of
// two parsers that could drift apart.

/// Counts SRT cues and finds the last cue's end timestamp so QA can compare
/// caption coverage against the deliverable's actual duration.
pub(crate) fn caption_timing_coverage(
    captions_path: &Path,
) -> Result<(usize, Option<i64>), ProjectError> {
    let text = fs::read_to_string(captions_path)?;
    let mut cue_count = 0_usize;
    let mut last_end_ms = None;
    for line in text.lines() {
        if let Some((_, end)) = line.split_once("-->") {
            cue_count += 1;
            if let Some(ms) = parse_srt_timestamp(end.trim()) {
                last_end_ms = Some(ms);
            }
        }
    }
    Ok((cue_count, last_end_ms))
}

pub(crate) fn parse_srt_timestamp(value: &str) -> Option<i64> {
    let (hms, millis) = value.split_once(',')?;
    let mut parts = hms.split(':');
    let hours: i64 = parts.next()?.parse().ok()?;
    let minutes: i64 = parts.next()?.parse().ok()?;
    let seconds: i64 = parts.next()?.parse().ok()?;
    let millis: i64 = millis.parse().ok()?;
    Some((hours * 3_600 + minutes * 60 + seconds) * 1_000 + millis)
}
