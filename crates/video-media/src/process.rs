//! Shared process/timeout plumbing for every ffmpeg/ffprobe/caption-worker
//! invocation in this crate (hardening plan §10.1).
//!
//! Every external command spawned by this module carries a mandatory
//! timeout; none may wait indefinitely. Budgets are grouped by class of work
//! rather than one flat number, and the render classes additionally scale
//! with the duration of media actually being processed (`scaled_timeout` /
//! `duration_scaled_timeout` below) so a ten-second clip and a two-hour
//! source don't share one guess. Every multiplier below is deliberately
//! generous — several times slower than expected real-world throughput — so
//! a legitimate slow encode is never clipped; only a genuinely hung process
//! is.

use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use video_core::process_runner::{CancellationToken, ManagedChild, ProcessRunError, ProcessSpec};

use crate::probe::probe_with_toolchain;
use crate::toolchain::MediaToolchain;
use crate::RenderError;

/// `ffprobe -show_format -show_streams`: reads container/stream headers,
/// never full-decodes. The smallest budget in this module; a slow network
/// mount or pathological file is still bounded.
pub(crate) const PROBE_TIMEOUT: Duration = Duration::from_secs(30);
/// ffprobe's JSON can grow with stream/tag count but stays well under this
/// even for unusually chaptered/multi-track sources.
pub(crate) const PROBE_STDOUT_CAP_BYTES: usize = 4 * 1024 * 1024;

/// Generic stdout cap for ffmpeg invocations that write their result to a
/// file, not stdout (`-loglevel error` means stdout carries nothing under
/// normal operation).
pub(crate) const STDOUT_CAP_BYTES: usize = 1024 * 1024;
/// Generic stderr cap: `-loglevel error` keeps normal-path stderr tiny, but
/// a real failure (filter graph error, codec probe dump) can be verbose.
pub(crate) const STDERR_CAP_BYTES: usize = 8 * 1024 * 1024;

/// Fixed budget for operations whose cost does not scale with source
/// duration: single-frame extraction, a static waveform/evidence image
/// composite, and the fixed ~1.6s two-segment boundary-probe clip. Two
/// minutes covers seek/decode cost on a slow disk with headroom to spare.
pub(crate) const SHORT_OP_TIMEOUT: Duration = Duration::from_secs(2 * 60);

/// Hardware-accelerated (h264_videotoolbox) rough/preview renders — segment
/// trims and concatenation for in-app scrubbing, not final delivery
/// quality. Floor plus a per-second-of-output-content multiplier: hardware
/// encode is normally much faster than real time, so 3s of budget per 1s of
/// assembled output is several times the expected cost.
pub(crate) const PREVIEW_RENDER_FLOOR: Duration = Duration::from_secs(5 * 60);
pub(crate) const PREVIEW_RENDER_PER_SOURCE_SECOND: Duration = Duration::from_secs(3);

/// Waveform image rendering decodes the audio it draws. `render_waveform`
/// covers a whole source; `render_waveform_range` and the boundary probe
/// cover a caller-bounded slice. Budget per second of audio decoded is
/// small since this is decode-only, no encode.
pub(crate) const WAVEFORM_RENDER_FLOOR: Duration = Duration::from_secs(60);
pub(crate) const WAVEFORM_PER_SOURCE_SECOND: Duration = Duration::from_millis(500);

/// Full-quality libx264 delivery renders (`render_to_preset`,
/// `render_preset_with_captions*`, `render_subtitled`), potentially at 4K
/// with per-cue caption overlay filters. This is the largest budget class:
/// floor plus 2s of wall-clock budget per 1s of source, well above the
/// `veryfast`/`slow` libx264 presets' expected real-time-ish throughput.
pub(crate) const FINAL_RENDER_FLOOR: Duration = Duration::from_secs(10 * 60);
pub(crate) const FINAL_RENDER_PER_SOURCE_SECOND: Duration = Duration::from_secs(2);

/// `measured_loudnorm_filter`'s two-pass loudnorm measurement decodes the
/// full source once with no encode; budget accordingly, smaller than a
/// render but larger than a probe.
pub(crate) const LOUDNESS_MEASURE_FLOOR: Duration = Duration::from_secs(5 * 60);
pub(crate) const LOUDNESS_PER_SOURCE_SECOND: Duration = Duration::from_millis(300);

/// `extract_audio_f32`'s raw decode + resample to f32le PCM: fast, decode
/// only, no filtering beyond resample/downmix.
pub(crate) const AUDIO_EXTRACT_FLOOR: Duration = Duration::from_secs(5 * 60);
pub(crate) const AUDIO_EXTRACT_PER_SOURCE_SECOND: Duration = Duration::from_millis(200);

/// Per-cue caption-card worker: renders one static PNG card. Cheap and
/// independent of source media duration.
pub(crate) const CAPTION_CARD_TIMEOUT: Duration = Duration::from_secs(30);

/// `floor.max(per_source_second * work_ms / 1000)` — the shared scaling
/// rule behind every duration-proportional budget above.
pub(crate) fn scaled_timeout(
    work_ms: i64,
    floor: Duration,
    per_source_second: Duration,
) -> Duration {
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
pub(crate) fn duration_scaled_timeout(
    input: &Path,
    floor: Duration,
    per_source_second: Duration,
) -> Duration {
    let duration_ms = crate::probe::probe(input)
        .ok()
        .and_then(|metadata| metadata.duration_ms)
        .unwrap_or(0);
    scaled_timeout(duration_ms, floor, per_source_second)
}

/// [`duration_scaled_timeout`], but reuses an already-resolved
/// [`MediaToolchain`] (§10.3: resolve once, reuse) instead of probing
/// through a fresh toolchain resolution.
pub(crate) fn duration_scaled_timeout_with_toolchain(
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
pub(crate) fn media_env_allow() -> Vec<(String, String)> {
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
pub(crate) fn string_args<const N: usize>(parts: [&str; N]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

/// The rec709 output color tag args repeated by every renderer that
/// tone-maps an HDR source down for delivery.
pub(crate) fn rec709_output_args() -> Vec<String> {
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
pub(crate) fn run_media_command<E, F>(
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
pub(crate) fn run_caption_card_worker(
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

#[cfg(test)]
mod tests {
    use super::*;
    use video_core::process_runner::ProcessRunError;

    /// §10.1: no command driven through the shared runner may wait
    /// indefinitely. This drives `run_media_command` — the wrapper every
    /// ffmpeg/ffprobe call site in this module goes through — with a fake
    /// hanging "executable" (`/bin/sh -c 'sleep 5'`, the same fixture shape
    /// `video_core::process_runner`'s own tests use) and a short timeout,
    /// and asserts the call returns promptly with a timeout error instead of
    /// blocking for the full 5 seconds.
    #[test]
    fn run_media_command_kills_a_hanging_process_at_its_timeout() {
        let start = Instant::now();
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
}
