//! Offline file transcription decode path for the sidecar.
//!
//! The shell chooses a file; the sidecar owns media decoding and ASR. ffmpeg is
//! resolved from `HR_FFMPEG_PATH`, a packaged runtime sibling, or PATH.

use std::path::{Path, PathBuf};
use std::process::Command;

pub fn decode_to_16k_mono(input: &Path) -> Result<Vec<f32>, String> {
    if !input.exists() {
        return Err(format!("file not found: {}", input.display()));
    }

    // ffmpeg is the ONLY decoder. The 2026-07-18 AVFoundation-first path was
    // reverted on 2026-07-20 after re-measurement showed it bought nothing:
    //
    //   * Latency: the "145 ms vs 2 860 ms" figure compared a COLD ffmpeg exec
    //     against a warm AVFoundation call. Warm-vs-warm on the same 37.6 s file,
    //     ffmpeg wins 50 ms to 73 ms; across a 775 s corpus the two paths were
    //     indistinguishable (8.61 s vs 8.60 s total decode).
    //   * Size: no saving was ever realised — ffmpeg still ships as the fallback
    //     (30.8 MB in the bundle) because AVFoundation cannot open webm/mkv.
    //   * Accuracy: the "byte-identical" parity claim was validated against
    //     `adaptive/app_capture/laptop_close_cond.wav`, which is ALREADY 16 kHz
    //     mono — so the check never exercised the resampler, the one stage where
    //     the engines differ. On 48 kHz stereo input the same harness reports
    //     max|Δ| = 1.81e-1, sample_ok = false. A 20-clip WER run over the graded
    //     corpus, decoded both ways through this function, measured ffmpeg 13.76%
    //     vs AVFoundation 14.30%.
    //
    // The WER delta is within noise at n=20 and does not prove harm, but the
    // upside was zero on every axis, and Parakeet TDT's published WER was
    // measured on ffmpeg-decoded audio. `av_decode` is retained for the
    // `av_decode_check` harness; it is no longer on the shipping path.
    decode_with_ffmpeg(input)
}

fn decode_with_ffmpeg(input: &Path) -> Result<Vec<f32>, String> {
    let ffmpeg = ffmpeg_command();
    let dir = std::env::temp_dir().join("hr_file_transcribe");
    std::fs::create_dir_all(&dir).map_err(|e| format!("temp dir: {e}"))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = dir.join(format!("ft_{}_{stamp}.wav", std::process::id()));
    let _guard = TmpGuard(tmp.clone());

    let out = Command::new(&ffmpeg)
        .args(["-y", "-hide_banner", "-loglevel", "error", "-i"])
        .arg(input)
        .args(["-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le"])
        .arg(&tmp)
        .output()
        .map_err(|e| {
            format!(
                "could not run ffmpeg at {} ({e}). Install ffmpeg or use a bundled build.",
                ffmpeg.display()
            )
        })?;
    if !out.status.success() {
        return Err(format!(
            "ffmpeg could not read this file: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    read_wav_16k_mono(&tmp)
}

/// Best-effort media duration in whole seconds WITHOUT decoding the file. Runs
/// `ffmpeg -i <file>` (prints metadata to stderr, exits non-zero because no output
/// was requested — expected) and parses the `Duration: HH:MM:SS.ss` line. Returns
/// Ok(None) when ffmpeg reports no duration; the caller then relies on the
/// post-decode sample-count check. Err only when ffmpeg can't be run.
pub fn probe_duration_secs(input: &Path) -> Result<Option<u32>, String> {
    if !input.exists() {
        return Err(format!("file not found: {}", input.display()));
    }

    // AVFoundation reads duration from the container header, so this replaces a
    // whole process spawn plus stderr parsing. Falls through to ffmpeg for
    // containers it cannot open, matching the decode path above.
    #[cfg(target_os = "macos")]
    if let Some(secs) = crate::av_decode::probe_duration_secs(input) {
        return Ok(Some(secs));
    }

    let ffmpeg = ffmpeg_command();
    let out = Command::new(&ffmpeg)
        .args(["-hide_banner", "-i"])
        .arg(input)
        .output()
        .map_err(|e| format!("could not run ffmpeg at {} ({e}).", ffmpeg.display()))?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    for line in stderr.lines() {
        if let Some(rest) = line.trim().strip_prefix("Duration:") {
            let hms = rest.trim().split(',').next().unwrap_or("").trim();
            if hms.eq_ignore_ascii_case("N/A") {
                return Ok(None);
            }
            let parts: Vec<&str> = hms.split(':').collect();
            if parts.len() == 3 {
                let h: f64 = parts[0].parse().unwrap_or(0.0);
                let m: f64 = parts[1].parse().unwrap_or(0.0);
                let s: f64 = parts[2].parse().unwrap_or(0.0);
                return Ok(Some((h * 3600.0 + m * 60.0 + s) as u32));
            }
        }
    }
    Ok(None)
}

fn ffmpeg_command() -> PathBuf {
    if let Some(path) = std::env::var_os("HR_FFMPEG_PATH") {
        let path = PathBuf::from(path);
        if path.exists() {
            return path;
        }
    }
    let name = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("runtime").join(name);
            if sibling.exists() {
                return sibling;
            }
            #[cfg(target_os = "macos")]
            if let Some(contents_dir) = dir.parent() {
                let resource = contents_dir.join("Resources").join("runtime").join(name);
                if resource.exists() {
                    return resource;
                }
            }
        }
    }
    PathBuf::from(name)
}

fn read_wav_16k_mono(path: &Path) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    let samples: Result<Vec<f32>, _> = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
            .collect(),
        hound::SampleFormat::Float => reader.samples::<f32>().collect(),
    };
    let samples = samples.map_err(|e| e.to_string())?;
    if spec.sample_rate != 16_000 || spec.channels != 1 {
        return Err(format!(
            "decoded wav had unexpected format: {} Hz, {} channel(s)",
            spec.sample_rate, spec.channels
        ));
    }
    Ok(samples)
}

struct TmpGuard(PathBuf);

impl Drop for TmpGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
