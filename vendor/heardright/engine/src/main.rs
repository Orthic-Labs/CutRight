#![cfg_attr(windows, windows_subsystem = "windows")]

// `heardright-engine` — resident dictation engine sidecar.
//
// Talks to the Tauri shell over JSON-RPC over stdio. Owns the model warm
// residency, mic capture, the recording state machine, focus-target tracking,
// delivery, and the recent transcript list. The shell is a thin client.
//
// The IPC contract is the shared `EngineFrame` schema in
// `heardright_core::engine`. This binary emits and consumes `EngineFrame` JSON
// over stdin/stdout, one frame per line, so the channel is trivially
// recoverable.

use std::sync::Arc;

use heardright_engine::{asr, ipc, runtime::EngineRuntime};
use parking_lot::Mutex;

fn main() -> anyhow::Result<()> {
    init_tracing();
    // Sidecar postmortem: the shell can only dump its own process when it
    // notices the engine died. Install our own panic hook so a real dump of
    // THIS process lands in the shared crash dir before abort.
    heardright_engine::crash_capture::install_panic_hook();
    configure_ort_dylib();

    let mut args = std::env::args();
    let _bin = args.next();
    let first = args.next();

    // Onboarding's one-shot setup process. The shell retains exactly one
    // 16 kHz mono f32LE recording, invokes this once, then receives a single
    // JSON result after every candidate has consumed those exact bytes.
    if first.as_deref() == Some("calibrate") {
        let pcm = args.next().unwrap_or_default();
        let path = std::path::PathBuf::from(&pcm);
        let bytes = std::fs::read(&path)
            .map_err(|error| anyhow::anyhow!("read calibration PCM {}: {error}", path.display()))?;
        if bytes.len() % 4 != 0 {
            return Err(anyhow::anyhow!("calibration PCM must be f32LE mono"));
        }
        let samples = bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect::<Vec<_>>();
        let models = asr::models_base(None);
        let result = heardright_engine::calibration::run(&models, &samples);
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    // One-shot ANE-compile warmer: `heardright-engine prepare-whisper <dir>`.
    // Loads the Whisper CoreML model once (and runs one throwaway inference) so
    // the OS caches its ANE compilation — the ~3-6 min first-load cost. The shell
    // runs this during install so the compile happens behind a visible
    // "Preparing model…" step instead of lazily blocking the engine at boot.
    #[cfg(target_os = "macos")]
    if first.as_deref() == Some("prepare-whisper") {
        let dir = args.next().unwrap_or_default();
        tracing::info!("prepare-whisper: compiling ANE cache for {dir}");
        match heardright_engine::whisper_coreml::WhisperCoreMl::load(std::path::Path::new(&dir)) {
            Ok(w) => {
                // Exercise encoder + decoder so the full graph is ANE-compiled.
                let _ = w.transcribe_lang_windowed(&vec![0f32; 16_000], w.lang_token("auto"));
                tracing::info!("prepare-whisper: done");
                return Ok(());
            }
            Err(e) => {
                tracing::error!("prepare-whisper failed: {e}");
                std::process::exit(1);
            }
        }
    }

    let models_hint = first.map(std::path::PathBuf::from);
    let models_base = crate::asr::models_base(models_hint.as_deref());
    tracing::info!("models base: {}", models_base.display());

    let runtime = Arc::new(Mutex::new(EngineRuntime::new(models_base)));
    ipc::run(runtime)
}

fn init_tracing() {
    // Non-panicking stderr writer. The sidecar logs to stderr, which the parent app
    // drains into telemetry. During a burst of logging (transcribe + polish + delivery)
    // the parent can momentarily stop draining; the pipe fills and a write returns
    // EPIPE/EAGAIN. tracing-subscriber reacts to a writer error by `eprintln!`-ing it,
    // which itself panics ("failed printing to stderr") and ABORTS the engine. Swallow
    // the write error so a dropped log line can never kill the process.
    struct SafeStderr;
    impl std::io::Write for SafeStderr {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let _ = std::io::Write::write_all(&mut std::io::stderr(), buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,heardright_engine=info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(|| SafeStderr)
        .try_init();
}

fn configure_ort_dylib() {
    if std::env::var("ORT_DYLIB_PATH")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
    {
        return;
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));

    #[cfg(target_os = "windows")]
    let candidates = {
        let exe_sibling = exe_dir.as_ref().map(|d| d.join("onnxruntime.dll"));
        let packaged_resource = exe_dir
            .as_ref()
            .map(|d| d.join("resources/runtime/onnxruntime.dll"));
        #[cfg(debug_assertions)]
        let dev_path = Some(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../.venv-build-dml/Lib/site-packages/onnxruntime/capi/onnxruntime.dll"),
        );
        #[cfg(not(debug_assertions))]
        let dev_path: Option<std::path::PathBuf> = None;
        vec![exe_sibling, packaged_resource, dev_path]
    };

    #[cfg(target_os = "macos")]
    let candidates = {
        let dylib = "libonnxruntime.dylib";
        let mut paths = Vec::new();
        if let Some(exe_dir) = &exe_dir {
            paths.push(Some(exe_dir.join("runtime").join(dylib)));
            paths.push(Some(exe_dir.join("resources/runtime").join(dylib)));
            if let Some(contents) = exe_dir.parent() {
                paths.push(Some(contents.join("Resources/runtime").join(dylib)));
                paths.push(Some(
                    contents.join("Resources/resources/runtime").join(dylib),
                ));
            }
        }
        #[cfg(debug_assertions)]
        paths.push(Some(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../src-tauri/resources/runtime")
                .join(dylib),
        ));
        paths
    };

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let candidates: Vec<Option<std::path::PathBuf>> = Vec::new();

    if let Some(dll) = candidates.into_iter().flatten().find(|p| p.exists()) {
        std::env::set_var("ORT_DYLIB_PATH", &dll);
        tracing::info!("ORT_DYLIB_PATH={}", dll.display());
    } else {
        tracing::warn!("ORT_DYLIB_PATH unresolved; ORT may fall back to the OS loader");
    }
}
