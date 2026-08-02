//! Dialogue-only audio finishing (REV2 plan §15.2 "Audio"): a configurable
//! high-pass/compressor/de-esser/limiter chain, integrated loudness + true
//! peak + clipped-sample measurement, ffmpeg-native sidechain-free music
//! ducking driven by caller-supplied speech regions, and a noise-floor step
//! probe for room-tone continuity across a join.
//!
//! This module only executes ffmpeg filter graphs; it holds no policy
//! (target LUFS, tolerances, gate pass/fail) — that lives in the versioned
//! `AudioProfile` artifact in `video-project` (REV2 plan §15.2: "profile
//! defaults ... versioned rather than hard-coded globally"). Every
//! invocation here goes through [`crate::process::run_media_command`] with a
//! duration-scaled timeout, matching every other ffmpeg call site in this
//! crate (hardening plan §10.1) — never a bare `Command::new(...).output()`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use thiserror::Error;

use video_core::process_runner::ProcessRunError;

use crate::process::{duration_scaled_timeout, run_media_command, string_args};
use crate::toolchain::ToolchainError;

#[derive(Debug, Error)]
pub enum AudioFinishError {
    #[error("audio finish input does not exist: {0}")]
    MissingInput(PathBuf),
    #[error("audio finish output must not overwrite the input")]
    OutputIsInput,
    #[error("ffmpeg could not start: {0}")]
    Start(#[source] std::io::Error),
    #[error("ffmpeg audio finish stage failed: {0}")]
    Failed(String),
    #[error("could not resolve a matched ffmpeg/ffprobe toolchain: {0}")]
    Toolchain(#[from] ToolchainError),
    #[error("ffmpeg process error: {0}")]
    Process(#[from] ProcessRunError),
}

/// The dialogue chain decodes, filters, and re-encodes once per cache miss;
/// no video is touched. Budget is smaller than a final render but larger
/// than a plain probe/extract pass since it runs four chained filters.
const DIALOGUE_CHAIN_FLOOR: Duration = Duration::from_secs(3 * 60);
const DIALOGUE_CHAIN_PER_SOURCE_SECOND: Duration = Duration::from_millis(500);

/// Loudness/true-peak/clip measurement is decode-only (`-f null -`), same
/// class of cost as `measured_loudnorm_filter` in `final_render.rs`.
const LOUDNESS_GATE_FLOOR: Duration = Duration::from_secs(3 * 60);
const LOUDNESS_GATE_PER_SOURCE_SECOND: Duration = Duration::from_millis(300);

/// Ducking re-encodes the music stem with a per-region `volume` automation;
/// same order of cost as the dialogue chain.
const DUCKING_FLOOR: Duration = Duration::from_secs(3 * 60);
const DUCKING_PER_SOURCE_SECOND: Duration = Duration::from_millis(500);

/// Room-tone probes decode a short bounded window (`-t <window>`), not the
/// whole file — cheap and independent of overall source duration.
const ROOM_TONE_PROBE_TIMEOUT: Duration = Duration::from_secs(60);

/// Gentle-compression parameters for the dialogue chain (ffmpeg
/// `acompressor`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressorParams {
    pub threshold_db: f64,
    pub ratio: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
}

/// De-ess parameters (ffmpeg `deesser`): both normalized to ffmpeg's native
/// `0.0..=1.0` range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeEsserParams {
    pub intensity: f64,
    pub frequency: f64,
}

/// The full configurable dialogue chain: high-pass -> gentle compression ->
/// de-ess -> limiter, in that order (REV2 plan §15.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DialogueChainParams {
    pub high_pass_hz: f64,
    pub compressor: CompressorParams,
    pub deesser: DeEsserParams,
    /// Limiter ceiling in dBTP (e.g. `-1.0`), converted to ffmpeg
    /// `alimiter`'s linear `0.0..=1.0` scale internally.
    pub limiter_ceiling_dbtp: f64,
}

fn db_to_linear(db: f64) -> f64 {
    10f64.powf(db / 20.0)
}

/// Build the ffmpeg `-af` filter-graph string for the configurable dialogue
/// chain. Pure and side-effect-free so it is independently testable without
/// invoking ffmpeg.
pub fn dialogue_chain_filter(params: &DialogueChainParams) -> String {
    let limit_linear = db_to_linear(params.limiter_ceiling_dbtp).clamp(0.000_001, 1.0);
    format!(
        "highpass=f={hp},acompressor=threshold={th}dB:ratio={ratio}:attack={attack}:release={release},deesser=i={intensity}:f={frequency},alimiter=limit={limit:.6}",
        hp = params.high_pass_hz,
        th = params.compressor.threshold_db,
        ratio = params.compressor.ratio,
        attack = params.compressor.attack_ms,
        release = params.compressor.release_ms,
        intensity = params.deesser.intensity,
        frequency = params.deesser.frequency,
        limit = limit_linear,
    )
}

/// Run the dialogue chain over `input`, writing a processed 48kHz mono PCM
/// stem to `output`. Callers own caching (REV2 plan §15.2: "cached processed
/// stem") — this function always (re)computes; a caller that wants the cache
/// checks/writes a cache key derived from the input content hash + the
/// audio-profile version before calling this.
pub fn process_dialogue_stem(
    input: &Path,
    output: &Path,
    params: &DialogueChainParams,
) -> Result<(), AudioFinishError> {
    if !input.is_file() {
        return Err(AudioFinishError::MissingInput(input.to_path_buf()));
    }
    if input == output {
        return Err(AudioFinishError::OutputIsInput);
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(AudioFinishError::Start)?;
    }
    let ffmpeg = crate::ffmpeg_path()?;
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args(["-af"]));
    args.push(dialogue_chain_filter(params));
    args.extend(string_args(["-ar", "48000", "-ac", "1"]));
    args.push(output.display().to_string());
    let timeout = duration_scaled_timeout(
        input,
        DIALOGUE_CHAIN_FLOOR,
        DIALOGUE_CHAIN_PER_SOURCE_SECOND,
    );
    run_media_command(&ffmpeg, args, timeout, AudioFinishError::Failed)?;
    Ok(())
}

/// Same as [`process_dialogue_stem`], but also returns a
/// [`video_core::StageReceipt`] (hardening plan §10.4), matching
/// `extract_audio_f32_with_receipt`'s pattern in `audio.rs`.
pub fn process_dialogue_stem_with_receipt(
    input: &Path,
    output: &Path,
    params: &DialogueChainParams,
    profile_version: u32,
) -> Result<video_core::StageReceipt, AudioFinishError> {
    process_dialogue_stem(input, output, params)?;
    crate::build_receipt(
        "audio.dialogue_chain",
        input,
        &serde_json::json!({
            "high_pass_hz": params.high_pass_hz,
            "compressor_threshold_db": params.compressor.threshold_db,
            "compressor_ratio": params.compressor.ratio,
            "compressor_attack_ms": params.compressor.attack_ms,
            "compressor_release_ms": params.compressor.release_ms,
            "deesser_intensity": params.deesser.intensity,
            "deesser_frequency": params.deesser.frequency,
            "limiter_ceiling_dbtp": params.limiter_ceiling_dbtp,
            "profile_version": profile_version,
        }),
        output,
    )
    .map_err(|error| AudioFinishError::Failed(error.to_string()))
}

/// Integrated loudness (LUFS), true peak (dBTP), and a clipped-sample count.
/// Loudness/true-peak come from one `ebur128` pass, matching
/// `video-project::qa_probes::measure_loudness`'s approach but through the
/// shared bounded process runner rather than a bare `Command`. Clipping is
/// NOT sourced from `astats`' "Number of clipped samples" — that metric does
/// not exist in current FFmpeg builds (verified against 8.1.2's `astats`
/// option list; the closest surviving fields are `Peak count`/`Abs Peak
/// count`, which count samples *at* the observed peak, not samples clamped
/// to full scale, so they cannot stand in for a clip count). Instead this
/// decodes to raw `f32le` and counts samples at or beyond full scale
/// directly — a definition of "clipped" that holds regardless of which
/// astats field names a given FFmpeg build ships.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoudnessMeasurement {
    pub integrated_lufs: Option<f64>,
    pub true_peak_dbtp: Option<f64>,
    pub clipped_samples: u64,
}

/// A sample at or above this magnitude (out of the `f32le` `-1.0..=1.0`
/// full-scale range) counts as clipped. Set just under `1.0` so a sample
/// that round-tripped through 16-bit PCM at exactly full scale
/// (`32767/32768 = 0.999969...`) still counts.
const CLIP_MAGNITUDE_THRESHOLD: f32 = 0.999;

pub fn measure_loudness_and_clipping(path: &Path) -> Result<LoudnessMeasurement, AudioFinishError> {
    if !path.is_file() {
        return Err(AudioFinishError::MissingInput(path.to_path_buf()));
    }
    let ffmpeg = crate::ffmpeg_path()?;
    let mut args = string_args(["-hide_banner", "-nostats", "-i"]);
    args.push(path.display().to_string());
    args.extend(string_args(["-af", "ebur128=peak=true", "-f", "null", "-"]));
    let timeout =
        duration_scaled_timeout(path, LOUDNESS_GATE_FLOOR, LOUDNESS_GATE_PER_SOURCE_SECOND);
    let outcome = run_media_command(&ffmpeg, args, timeout, AudioFinishError::Failed)?;
    let stderr = String::from_utf8_lossy(&outcome.stderr);
    let integrated_lufs = parse_last_labeled_f64(&stderr, "I:");
    let true_peak_dbtp = parse_last_labeled_f64(&stderr, "Peak:");
    let clipped_samples = count_clipped_samples(path)?;
    Ok(LoudnessMeasurement {
        integrated_lufs,
        true_peak_dbtp,
        clipped_samples,
    })
}

fn clip_scan_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    name.push(".clip-scan.f32le");
    path.with_file_name(name)
}

/// Decode `path` to raw mono `f32le` and count samples at or beyond
/// [`CLIP_MAGNITUDE_THRESHOLD`]. Writes/reads a real sidecar file rather
/// than stdout, since [`run_media_command`]'s stdout cap is sized for
/// diagnostic output, not a full raw PCM stream.
fn count_clipped_samples(path: &Path) -> Result<u64, AudioFinishError> {
    let ffmpeg = crate::ffmpeg_path()?;
    let raw_path = clip_scan_path(path);
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(path.display().to_string());
    args.extend(string_args(["-f", "f32le", "-ac", "1"]));
    args.push(raw_path.display().to_string());
    let timeout =
        duration_scaled_timeout(path, LOUDNESS_GATE_FLOOR, LOUDNESS_GATE_PER_SOURCE_SECOND);
    let run_result = run_media_command(&ffmpeg, args, timeout, AudioFinishError::Failed);
    let bytes = run_result.and_then(|_| std::fs::read(&raw_path).map_err(AudioFinishError::Start));
    let _ = std::fs::remove_file(&raw_path);
    let bytes = bytes?;
    Ok(bytes
        .chunks_exact(4)
        .filter(|chunk| {
            let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            value.abs() >= CLIP_MAGNITUDE_THRESHOLD
        })
        .count() as u64)
}

/// Build the ffmpeg `volume` automation that ducks a music/background track
/// under the supplied speech regions (already available from VAD, mapped
/// into the same timebase as the audio being ducked by the caller). `None`
/// when there is nothing to duck under. `duck_db` is negative (attenuation).
pub fn ducking_filter(regions: &[(i64, i64)], duck_db: f64) -> Option<String> {
    if regions.is_empty() {
        return None;
    }
    let expr = regions
        .iter()
        .map(|(start_ms, end_ms)| {
            format!(
                "between(t,{:.3},{:.3})",
                *start_ms as f64 / 1000.0,
                *end_ms as f64 / 1000.0
            )
        })
        .collect::<Vec<_>>()
        .join("+");
    Some(format!("volume={duck_db}dB:enable='{expr}'"))
}

/// Apply [`ducking_filter`] to `input` (a music/background stem), writing
/// the ducked result to `output`. With no speech regions the track is
/// passed through unchanged (still re-encoded to the same PCM shape so
/// downstream mixing sees a consistent format).
pub fn duck_track_under_speech(
    input: &Path,
    output: &Path,
    speech_regions: &[(i64, i64)],
    duck_db: f64,
) -> Result<(), AudioFinishError> {
    if !input.is_file() {
        return Err(AudioFinishError::MissingInput(input.to_path_buf()));
    }
    if input == output {
        return Err(AudioFinishError::OutputIsInput);
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(AudioFinishError::Start)?;
    }
    let ffmpeg = crate::ffmpeg_path()?;
    let filter =
        ducking_filter(speech_regions, duck_db).unwrap_or_else(|| "volume=0dB".to_string());
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args(["-af"]));
    args.push(filter);
    args.extend(string_args(["-ar", "48000", "-ac", "1"]));
    args.push(output.display().to_string());
    let timeout = duration_scaled_timeout(input, DUCKING_FLOOR, DUCKING_PER_SOURCE_SECOND);
    run_media_command(&ffmpeg, args, timeout, AudioFinishError::Failed)?;
    Ok(())
}

/// Mean-volume noise-floor reading just before and just after a join point,
/// so a cut that produces an audible room-tone step (REV2 plan §15.2) can be
/// detected and flagged for evidence rather than shipped silently.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoomToneStep {
    pub before_mean_db: Option<f64>,
    pub after_mean_db: Option<f64>,
    /// `|after - before|`, when both sides measured.
    pub step_db: Option<f64>,
}

/// Measure the noise-floor step across `join_output_ms` in `path` using a
/// `window_ms`-wide `volumedetect` window on each side of the join.
pub fn measure_room_tone_step(
    path: &Path,
    join_output_ms: i64,
    window_ms: i64,
) -> Result<RoomToneStep, AudioFinishError> {
    let before = mean_volume_window(path, (join_output_ms - window_ms).max(0), window_ms)?;
    let after = mean_volume_window(path, join_output_ms, window_ms)?;
    let step_db = match (before, after) {
        (Some(before), Some(after)) => Some((after - before).abs()),
        _ => None,
    };
    Ok(RoomToneStep {
        before_mean_db: before,
        after_mean_db: after,
        step_db,
    })
}

fn mean_volume_window(
    path: &Path,
    start_ms: i64,
    duration_ms: i64,
) -> Result<Option<f64>, AudioFinishError> {
    if !path.is_file() {
        return Err(AudioFinishError::MissingInput(path.to_path_buf()));
    }
    let ffmpeg = crate::ffmpeg_path()?;
    let mut args = string_args(["-hide_banner", "-nostats", "-ss"]);
    args.push(format!("{:.3}", start_ms.max(0) as f64 / 1000.0));
    args.extend(string_args(["-i"]));
    args.push(path.display().to_string());
    args.extend(string_args(["-t"]));
    args.push(format!("{:.3}", duration_ms.max(0) as f64 / 1000.0));
    args.extend(string_args(["-af", "volumedetect", "-f", "null", "-"]));
    let outcome = run_media_command(
        &ffmpeg,
        args,
        ROOM_TONE_PROBE_TIMEOUT,
        AudioFinishError::Failed,
    )?;
    let stderr = String::from_utf8_lossy(&outcome.stderr);
    Ok(parse_last_labeled_f64_anywhere(&stderr, "mean_volume:"))
}

/// Same lookup as `qa_probes::parse_last_labeled_f64` (label at the start of
/// the trimmed line, as `ebur128`/`astats` emit).
fn parse_last_labeled_f64(text: &str, label: &str) -> Option<f64> {
    text.lines().rev().find_map(|line| {
        line.trim()
            .strip_prefix(label)
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|token| token.parse::<f64>().ok())
    })
}

/// Looser lookup for filters like `volumedetect` whose log lines carry a
/// `[Parsed_volumedetect_0 @ 0x...]` prefix before the label.
fn parse_last_labeled_f64_anywhere(text: &str, label: &str) -> Option<f64> {
    text.lines().rev().find_map(|line| {
        let index = line.find(label)?;
        line[index + label.len()..]
            .split_whitespace()
            .next()?
            .parse::<f64>()
            .ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn default_params() -> DialogueChainParams {
        DialogueChainParams {
            high_pass_hz: 80.0,
            compressor: CompressorParams {
                threshold_db: -18.0,
                ratio: 2.5,
                attack_ms: 10.0,
                release_ms: 100.0,
            },
            deesser: DeEsserParams {
                intensity: 0.3,
                frequency: 0.5,
            },
            limiter_ceiling_dbtp: -1.0,
        }
    }

    #[test]
    fn dialogue_chain_filter_orders_highpass_compressor_deesser_limiter() {
        let filter = dialogue_chain_filter(&default_params());
        let highpass_at = filter.find("highpass=f=80").expect("highpass present");
        let compressor_at = filter
            .find("acompressor=threshold=-18dB:ratio=2.5:attack=10:release=100")
            .expect("compressor present");
        let deesser_at = filter.find("deesser=i=0.3:f=0.5").expect("deesser present");
        let limiter_at = filter.find("alimiter=limit=").expect("limiter present");
        assert!(highpass_at < compressor_at);
        assert!(compressor_at < deesser_at);
        assert!(deesser_at < limiter_at);
    }

    #[test]
    fn ducking_filter_is_none_with_no_speech_regions() {
        assert_eq!(ducking_filter(&[], -12.0), None);
    }

    #[test]
    fn ducking_filter_builds_a_between_expression_per_region() {
        let filter = ducking_filter(&[(1_000, 2_000), (5_000, 6_500)], -12.0).unwrap();
        assert!(filter.starts_with("volume=-12dB:enable='"));
        assert!(filter.contains("between(t,1.000,2.000)"));
        assert!(filter.contains("between(t,5.000,6.500)"));
        assert!(filter.contains('+'));
    }

    /// Generates a short sine-tone WAV fixture via ffmpeg's `lavfi` source —
    /// no checked-in binary media required.
    fn generate_tone_fixture(path: &Path, seconds: f64, amplitude: f64) {
        let status = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
            ])
            .arg(format!(
                "sine=frequency=220:sample_rate=48000:duration={seconds}"
            ))
            .args(["-af", &format!("volume={amplitude}")])
            .arg(path)
            .status()
            .expect("spawn ffmpeg to build fixture");
        assert!(status.success(), "fixture generation failed");
    }

    #[test]
    fn process_dialogue_stem_produces_decodable_output_and_rejects_output_is_input() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("dialogue.wav");
        generate_tone_fixture(&input, 1.0, 0.2);

        let output = dir.path().join("dialogue-processed.wav");
        process_dialogue_stem(&input, &output, &default_params()).expect("chain runs");
        assert!(output.is_file());

        let same = process_dialogue_stem(&input, &input, &default_params());
        assert!(matches!(same, Err(AudioFinishError::OutputIsInput)));
    }

    #[test]
    fn measure_loudness_and_clipping_detects_clipped_samples_on_an_overdriven_tone() {
        let dir = tempfile::tempdir().unwrap();
        let clipped = dir.path().join("clipped.wav");
        // The lavfi `sine` source's default peak amplitude is 0.125 full
        // scale, not 1.0 — amplitude 20 (2.5x full scale) forces sustained
        // flat-topped clipping, verified empirically against this
        // environment's ffmpeg build.
        generate_tone_fixture(&clipped, 1.0, 20.0);
        let measurement = measure_loudness_and_clipping(&clipped).expect("measured");
        assert!(
            measurement.clipped_samples > 0,
            "expected clipped samples, got {measurement:?}"
        );

        let clean = dir.path().join("clean.wav");
        generate_tone_fixture(&clean, 1.0, 0.1);
        let clean_measurement = measure_loudness_and_clipping(&clean).expect("measured");
        assert_eq!(clean_measurement.clipped_samples, 0);
    }

    #[test]
    fn duck_track_under_speech_reduces_level_inside_speech_regions_only() {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("music.wav");
        generate_tone_fixture(&music, 4.0, 0.5);

        let ducked = dir.path().join("music-ducked.wav");
        duck_track_under_speech(&music, &ducked, &[(1_000, 2_000)], -18.0).expect("duck runs");

        // Inside the ducked window the level must read lower than the
        // unducked head of the same file.
        let inside = mean_volume_window(&ducked, 1_200, 500).unwrap().unwrap();
        let outside = mean_volume_window(&ducked, 2_500, 1_000).unwrap().unwrap();
        assert!(
            inside < outside - 6.0,
            "expected ducked window well below unducked window: inside={inside} outside={outside}"
        );
    }

    #[test]
    fn measure_room_tone_step_reads_near_zero_across_a_uniform_tone() {
        let dir = tempfile::tempdir().unwrap();
        let tone = dir.path().join("uniform.wav");
        generate_tone_fixture(&tone, 4.0, 0.2);
        let step = measure_room_tone_step(&tone, 2_000, 500).expect("measured");
        let step_db = step.step_db.expect("both windows measured");
        assert!(
            step_db < 1.0,
            "expected a near-zero step on a uniform tone, got {step_db}"
        );
    }

    #[test]
    fn measure_room_tone_step_reads_a_real_step_across_a_level_jump() {
        let dir = tempfile::tempdir().unwrap();
        let quiet = dir.path().join("quiet.wav");
        generate_tone_fixture(&quiet, 2.0, 0.02);
        let loud = dir.path().join("loud.wav");
        generate_tone_fixture(&loud, 2.0, 0.9);
        let joined = dir.path().join("joined.wav");
        let status = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
            .arg(&quiet)
            .arg("-i")
            .arg(&loud)
            .args([
                "-filter_complex",
                "[0:a][1:a]concat=n=2:v=0:a=1[a]",
                "-map",
                "[a]",
            ])
            .arg(&joined)
            .status()
            .expect("spawn ffmpeg to join fixtures");
        assert!(status.success());

        let step = measure_room_tone_step(&joined, 2_000, 500).expect("measured");
        let step_db = step.step_db.expect("both windows measured");
        assert!(
            step_db > 6.0,
            "expected a real step at the join, got {step_db}"
        );
    }
}
