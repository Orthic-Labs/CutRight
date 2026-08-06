// Windows Whisper engine (Track W-Win) — shells out to the staged whisper.cpp
// Vulkan CLI (`whisper-cli.exe`) once per utterance. Windows-only; the macOS
// equivalent is the in-process CoreML/ANE engine in `whisper_coreml.rs`.
//
// v1 shape: write the already-conditioned 16 kHz mono f32 samples to a temp
// WAV, run `whisper-cli.exe -m <gguf> -l <lang> -nt -np -f <wav>`, and parse
// stdout as the transcript text. This is CLI-per-utterance, not an in-process
// FFI binding — acceptable per the Lane D plan because Vulkan GPU decode of a
// short utterance is well under the 2s latency bar (measured ~0.6s warm on a
// 2s clip with ggml-tiny on this box; ship ships a larger GGUF, but decode
// time scales with audio length, not just model size, and Vulkan keeps it
// fast). FFI against `whisper.dll` is the better long-term shape if
// process-spawn overhead ever becomes the bottleneck.
#![cfg(target_os = "windows")]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const SAMPLE_RATE: u32 = 16_000;
// F4(a) (Sol audit 2026-07-16): hard cap on a single whisper-cli.exe call.
// This runs on the worker's single thread (capture drain + Stop/Cancel all
// live there too) — before this fix, `Command::output()` had no deadline, so
// a wedged Vulkan/GPU driver or a corrupt model file could hang the child
// forever and Stop would never be processed for the rest of the session.
const WHISPER_CLI_TIMEOUT: Duration = Duration::from_secs(10);
const WHISPER_CLI_POLL_INTERVAL: Duration = Duration::from_millis(20);

pub struct WhisperWin {
    /// Resolved path to `whisper-cli.exe`.
    cli: PathBuf,
    /// Resolved path to the GGUF model file.
    model: PathBuf,
}

impl WhisperWin {
    /// Locate the staged whisper.cpp Vulkan CLI + the downloaded GGUF model.
    /// `models_dir` is the app-data models base the engine was given at boot
    /// (`crate::asr::models_base`); the Whisper GGUF lives under
    /// `<app-data>/models/whisper-win` (see `whisper_model.rs` on the shell
    /// side, which places it there after a Pro-gated download).
    pub fn load(models_dir: &Path) -> Result<Self, String> {
        let cli = resolve_whisper_cli()
            .ok_or_else(|| "whisper-cli.exe not found (staged runtime or dev build)".to_string())?;
        let model = resolve_whisper_model(models_dir).ok_or_else(|| {
            "Whisper GGUF model not found — download it from Settings first".to_string()
        })?;
        tracing::info!(
            "Windows Whisper engine loaded: cli={} model={}",
            cli.display(),
            model.display()
        );
        Ok(Self { cli, model })
    }

    /// Transcribe 16 kHz mono f32 samples (already `condition_for_asr`-processed
    /// by the shared worker pipeline — this engine does NOT re-condition).
    /// `lang` is a Whisper language code (`"auto"` or e.g. `"hi"`, `"en"`).
    ///
    /// F4(a) (Sol audit 2026-07-16): bounded to `WHISPER_CLI_TIMEOUT` — see the
    /// constant's doc comment. `Command::output()` cannot express a deadline,
    /// so this polls `try_wait()` instead and `kill()`s the child past the cap,
    /// returning an error result rather than blocking forever. The stdout/
    /// stderr parsing and the WAV write/cleanup are unchanged, so a
    /// successful transcribe produces byte-identical output to before.
    pub fn transcribe_lang(&self, samples: &[f32], lang: &str) -> Result<String, String> {
        let started = Instant::now();
        let wav_path = write_temp_wav(samples)?;
        let _guard = TmpGuard(wav_path.clone());

        // CREATE_NO_WINDOW: the engine sidecar runs without a console, so a
        // plain spawn would allocate (and flash) a new console window on every
        // utterance. Same flag app_launch.rs / engine_supervisor use.
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut child = Command::new(&self.cli)
            .arg("-m")
            .arg(&self.model)
            .arg("-l")
            .arg(lang)
            .arg("-nt") // no timestamps — clean text
            .arg("-np") // no diagnostic prints beyond the result line
            .arg("-f")
            .arg(&wav_path)
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn whisper-cli.exe: {e}"))?;

        // Drain stdout/stderr on their own threads. `Command::output()` does
        // this internally but offers no deadline; replicating it by hand here
        // is required so a child that fills a pipe buffer before we notice the
        // timeout can't deadlock instead of exiting.
        let (stdout_tx, stdout_rx) = mpsc::channel();
        match child.stdout.take() {
            Some(mut out) => {
                std::thread::spawn(move || {
                    let mut buf = Vec::new();
                    let _ = out.read_to_end(&mut buf);
                    let _ = stdout_tx.send(buf);
                });
            }
            None => {
                let _ = stdout_tx.send(Vec::new());
            }
        }
        let (stderr_tx, stderr_rx) = mpsc::channel();
        match child.stderr.take() {
            Some(mut err) => {
                std::thread::spawn(move || {
                    let mut buf = Vec::new();
                    let _ = err.read_to_end(&mut buf);
                    let _ = stderr_tx.send(buf);
                });
            }
            None => {
                let _ = stderr_tx.send(Vec::new());
            }
        }

        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if started.elapsed() >= WHISPER_CLI_TIMEOUT {
                        tracing::error!(
                            "whisper-cli.exe exceeded {:?} deadline; killing",
                            WHISPER_CLI_TIMEOUT
                        );
                        let _ = child.kill();
                        let _ = child.wait(); // reap so it doesn't linger as a zombie handle
                        return Err(format!(
                            "whisper-cli.exe timed out after {WHISPER_CLI_TIMEOUT:?}"
                        ));
                    }
                    std::thread::sleep(WHISPER_CLI_POLL_INTERVAL);
                }
                Err(e) => return Err(format!("wait whisper-cli.exe: {e}")),
            }
        };

        let elapsed_ms = started.elapsed().as_millis();
        // The reader threads finish essentially immediately once the child
        // exits (or is killed) — this recv deadline is only a safety net
        // against a wedged pipe read and is not expected to ever trip.
        let stdout_bytes = stdout_rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap_or_default();
        let stderr_bytes = stderr_rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap_or_default();

        if !status.success() {
            let stderr = String::from_utf8_lossy(&stderr_bytes);
            return Err(format!(
                "whisper-cli.exe exited {}: {}",
                status,
                stderr.trim()
            ));
        }
        let text = parse_whisper_cli_stdout(&String::from_utf8_lossy(&stdout_bytes));
        tracing::info!(
            "Windows Whisper transcribe: lang={} samples={} secs={:.1} ms={} chars={}",
            lang,
            samples.len(),
            samples.len() as f32 / SAMPLE_RATE as f32,
            elapsed_ms,
            text.len()
        );
        Ok(text)
    }
}

/// `whisper-cli.exe -np` still prints the Vulkan device-enumeration banner and
/// blank lines to stdout ahead of the transcript (see the CLI's own startup
/// log) — collect all non-empty, non-banner lines and join them. The CLI's
/// plain-text result mode prints one line per line; blank input yields no
/// output line at all (empty transcript is valid, not an error).
fn parse_whisper_cli_stdout(stdout: &str) -> String {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("ggml_vulkan:"))
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

/// Resolve `whisper-cli.exe`. Order: packaged app resources (`<exe_dir>/runtime/`,
/// the production layout `package.ps1` stages into `resources/runtime`), then the
/// dev fallback under the repo's local whisper.cpp Vulkan build. Mirrors the
/// ONNX runtime DLL resolution order in `main.rs::configure_ort_dylib`.
fn resolve_whisper_cli() -> Option<PathBuf> {
    if let Some(env) = std::env::var_os("HR_WHISPER_CLI") {
        let p = PathBuf::from(env);
        if p.exists() {
            return Some(p);
        }
    }
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    if let Some(dir) = &exe_dir {
        let exe_sibling = dir.join("whisper-cli.exe");
        if exe_sibling.exists() {
            return Some(exe_sibling);
        }
        let runtime_sibling = dir.join("runtime").join("whisper-cli.exe");
        if runtime_sibling.exists() {
            return Some(runtime_sibling);
        }
        let packaged_resource = dir
            .join("resources")
            .join("runtime")
            .join("whisper-cli.exe");
        if packaged_resource.exists() {
            return Some(packaged_resource);
        }
    }
    #[cfg(debug_assertions)]
    {
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../.wcpp_vulkan/whisper.cpp/build_cl/bin/Release/whisper-cli.exe");
        if dev_path.exists() {
            return Some(dev_path);
        }
    }
    None
}

/// Resolve the model file: `<models_dir>/whisper-win/*.{bin,gguf}` — first
/// match found. whisper.cpp models are GGML `.bin` files (`ggml-*.bin`);
/// `.gguf` is accepted too in case the published archive ever switches
/// formats. `HR_WHISPER_GGUF` overrides for QA/dev.
fn resolve_whisper_model(models_dir: &Path) -> Option<PathBuf> {
    if let Some(env) = std::env::var_os("HR_WHISPER_GGUF") {
        let p = PathBuf::from(env);
        if p.exists() {
            return Some(p);
        }
    }
    let dir = models_dir.join("whisper-win");
    let entries = std::fs::read_dir(&dir).ok()?;
    entries.filter_map(|e| e.ok()).map(|e| e.path()).find(|p| {
        matches!(
            p.extension().and_then(|s| s.to_str()),
            Some("bin") | Some("gguf")
        )
    })
}

/// Write 16 kHz mono f32 samples to a temp WAV (16-bit PCM — whisper-cli reads
/// both, but PCM16 keeps the file small and matches the format whisper.cpp's
/// own examples use). Unique per-call filename so concurrent utterances never
/// collide (the worker is single-threaded per session, but a stale temp file
/// from a crashed prior run must never be silently reused).
fn write_temp_wav(samples: &[f32]) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("hr_whisper_win");
    std::fs::create_dir_all(&dir).map_err(|e| format!("temp dir: {e}"))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = dir.join(format!("utt_{}_{stamp}.wav", std::process::id()));

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer =
        hound::WavWriter::create(&path, spec).map_err(|e| format!("create wav: {e}"))?;
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let i16_sample = (clamped * i16::MAX as f32) as i16;
        writer
            .write_sample(i16_sample)
            .map_err(|e| format!("write wav sample: {e}"))?;
    }
    writer
        .finalize()
        .map_err(|e| format!("finalize wav: {e}"))?;
    Ok(path)
}

struct TmpGuard(PathBuf);

impl Drop for TmpGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
