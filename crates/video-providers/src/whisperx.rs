//! WhisperX alignment provider, driven through the shared process runner
//! (§10.1): bounded timeout, environment allow-list, byte-capped output, and
//! guaranteed temp-file cleanup even when the process is killed for timing
//! out.

use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use video_core::providers::{
    ProviderError as CoreProviderError, TranscriptionOutput, TranscriptionProvider,
    TranscriptionRequest,
};
use video_core::{Transcript, Word, SCHEMA_VERSION};

use crate::heardright::ProviderError;
use video_core::process_runner::{CancellationToken, ProcessSpec, TempFileGuard};

/// Project-local venv-relative interpreter path, tried only relative to
/// this crate's own manifest directory (never an absolute developer path;
/// §9.3). This lets a `.venv-whisperx` checked out next to the workspace
/// resolve without any environment configuration, on any machine.
const PROJECT_VENV_PYTHON_UNIX: &str = "../../.venv-whisperx/bin/python";
const PROJECT_VENV_PYTHON_WINDOWS: &str = "../../.venv-whisperx/Scripts/python.exe";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);
const OUTPUT_CAP_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct WhisperXProvider {
    python: PathBuf,
    script: PathBuf,
    timeout: Duration,
}

#[derive(Debug, serde::Deserialize)]
struct WhisperXWord {
    s: f64,
    e: f64,
    w: String,
}

/// Resolve the WhisperX Python interpreter (§9.3).
///
/// Discovery order:
/// 1. explicit `CUTRIGHT_WHISPERX_PYTHON` override — if set, it must point
///    at an existing file, or discovery fails immediately naming that path;
/// 2. a project-local `.venv-whisperx` resolved *relative to this crate's
///    own manifest directory* (workspace-root/.venv-whisperx), never an
///    absolute developer path — this is what lets a checked-out venv work
///    with zero configuration on any machine, including this one;
/// 3. `python3`, then `python`, resolved on `PATH`;
/// 4. a clear [`ProviderError::WhisperXPythonMissing`] naming every
///    location that was checked.
///
/// WhisperX is the optional alignment verifier: an unavailable result here
/// is a normal, expected outcome (never a panic), and callers treat it as
/// "verifier unavailable" rather than a hard failure.
fn discover_python() -> Result<PathBuf, ProviderError> {
    if let Some(path) = env::var_os("CUTRIGHT_WHISPERX_PYTHON") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(ProviderError::WhisperXPythonMissing(format!(
            "CUTRIGHT_WHISPERX_PYTHON={} does not exist",
            path.display()
        )));
    }

    let project_venv_rel = if cfg!(windows) {
        PROJECT_VENV_PYTHON_WINDOWS
    } else {
        PROJECT_VENV_PYTHON_UNIX
    };
    let project_venv = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(project_venv_rel);
    if project_venv.is_file() {
        return Ok(project_venv);
    }

    for candidate in ["python3", "python"] {
        if let Some(found) = resolve_on_path(candidate) {
            return Ok(found);
        }
    }

    Err(ProviderError::WhisperXPythonMissing(format!(
        "checked CUTRIGHT_WHISPERX_PYTHON (unset), {} (project-local venv), \
         and python3/python on PATH",
        project_venv.display()
    )))
}

/// Search `PATH` for an executable file named `name`, without shelling out
/// to `which`/`where`. This is the same style of PATH search the OS itself
/// performs at spawn time, done ahead of time only so a missing interpreter
/// can be reported as a clear discovery failure instead of a spawn error.
fn resolve_on_path(name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        if cfg!(windows) {
            let candidate_exe = dir.join(format!("{name}.exe"));
            if candidate_exe.is_file() {
                return Some(candidate_exe);
            }
        }
    }
    None
}

impl WhisperXProvider {
    pub fn discover() -> Result<Self, ProviderError> {
        let python = discover_python()?;
        let script = env::var_os("CUTRIGHT_WHISPERX_SCRIPT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join("cutaway/scripts/whisperx_align.py")
            });
        if !script.is_file() {
            return Err(ProviderError::WhisperXScriptMissing);
        }
        let timeout = env::var("CUTRIGHT_WHISPERX_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(DEFAULT_TIMEOUT);
        Ok(Self {
            python,
            script,
            timeout,
        })
    }

    fn env_allow(&self) -> Vec<(String, String)> {
        // Explicit allow-list (§10.1): only what the interpreter needs to
        // resolve itself and any native extensions it loads.
        ["PATH", "HOME", "PYTHONHOME", "PYTHONPATH"]
            .iter()
            .filter_map(|key| env::var(key).ok().map(|value| (key.to_string(), value)))
            .collect()
    }

    fn transcribe_inner(
        &self,
        request: &TranscriptionRequest,
    ) -> Result<TranscriptionOutput, ProviderError> {
        let output_guard = TempFileGuard::new(
            &format!(
                "cutright-whisperx-{}",
                blake3::hash(request.source_id.as_bytes()).to_hex()
            ),
            ".json",
        );
        let spec = ProcessSpec {
            executable: self.python.clone(),
            args: vec![
                self.script.display().to_string(),
                request.source_path.display().to_string(),
                output_guard.path.display().to_string(),
            ],
            env_allow: self.env_allow(),
            working_dir: None,
            timeout: self.timeout,
            stdout_cap_bytes: OUTPUT_CAP_BYTES,
            stderr_cap_bytes: OUTPUT_CAP_BYTES,
        };
        let outcome = video_core::process_runner::run_process(&spec, &CancellationToken::new())?;
        if !outcome.success() {
            let mut message = String::from_utf8_lossy(&outcome.stderr).trim().to_string();
            if outcome.stderr_truncated {
                message.push_str(" ...[stderr truncated]");
            }
            if let Some(signal) = outcome.signal {
                message.push_str(&format!(" (terminated by signal {signal})"));
            }
            message.push_str(&format!(
                " [exit_code={:?}, duration={:?}]",
                outcome.exit_code, outcome.duration
            ));
            return Err(ProviderError::WhisperXExit(message));
        }
        let bytes = fs::read(&output_guard.path).map_err(|error| {
            ProviderError::WhisperXExit(format!("could not read WhisperX output: {error}"))
        })?;
        let raw_response: Value = serde_json::from_slice(&bytes).map_err(|error| {
            ProviderError::WhisperXExit(format!("invalid WhisperX response: {error}"))
        })?;
        let response: Vec<WhisperXWord> =
            serde_json::from_value(raw_response.clone()).map_err(|error| {
                ProviderError::WhisperXExit(format!("invalid WhisperX word response: {error}"))
            })?;
        if response.is_empty() {
            return Err(ProviderError::WhisperXExit(
                "WhisperX returned no timed words".into(),
            ));
        }
        let mut previous_end_ms = 0;
        let mut words = Vec::with_capacity(response.len());
        for (index, word) in response.into_iter().enumerate() {
            if !word.s.is_finite() || !word.e.is_finite() || word.s < 0.0 || word.e <= word.s {
                return Err(ProviderError::WhisperXExit(format!(
                    "invalid WhisperX interval at word {index}"
                )));
            }
            let text = word.w.trim().to_string();
            if text.is_empty() {
                return Err(ProviderError::WhisperXExit(format!(
                    "empty WhisperX word at index {index}"
                )));
            }
            let start_ms = (word.s * 1_000.0).round() as i64;
            let end_ms = (word.e * 1_000.0).round() as i64;
            if start_ms < previous_end_ms {
                return Err(ProviderError::WhisperXExit(format!(
                    "non-monotonic WhisperX word at index {index}"
                )));
            }
            previous_end_ms = end_ms;
            words.push(Word {
                id: format!("wx_{index:06}"),
                source_word_id: None,
                text,
                start_ms,
                end_ms,
                confidence: 0.0,
                speaker: None,
                kind: "word".into(),
            });
        }
        Ok(TranscriptionOutput {
            transcript: Transcript {
                schema_version: SCHEMA_VERSION,
                provider: "whisperx-alignment".into(),
                source_id: request.source_id.clone(),
                language: request
                    .language_hint
                    .clone()
                    .unwrap_or_else(|| "und".into()),
                words,
                events: Vec::new(),
            },
            raw_response,
            provider_model: "whisperx-align".into(),
            warnings: Vec::new(),
        })
        // `output_guard` drops here regardless of outcome, removing the
        // temp file even on error (§10.1 temp-file cleanup).
    }
}

impl TranscriptionProvider for WhisperXProvider {
    fn id(&self) -> &'static str {
        "whisperx-alignment"
    }

    fn model_id(&self) -> &'static str {
        "whisperx-align"
    }

    fn transcribe(
        &self,
        request: &TranscriptionRequest,
    ) -> Result<TranscriptionOutput, CoreProviderError> {
        self.transcribe_inner(request).map_err(|error| {
            let provider = TranscriptionProvider::id(self).to_string();
            match error {
                // Process could not even be started/run, or was killed by
                // its own timeout — transient, the provider itself was
                // unavailable for this call.
                ProviderError::WhisperXProcess(_) => CoreProviderError::Unavailable {
                    provider,
                    reason: error.to_string(),
                },
                // Everything else (nonzero exit, malformed output, invalid
                // word intervals) is a definite rejection of this request.
                other => CoreProviderError::Rejected {
                    provider,
                    reason: other.to_string(),
                },
            }
        })
    }
}

#[cfg(test)]
mod discovery_tests {
    use super::*;
    use std::sync::Mutex;

    // `discover_python` reads process-global environment variables; guard
    // every test that touches them so parallel test threads cannot
    // interleave sets/clears.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_env() {
        env::remove_var("CUTRIGHT_WHISPERX_PYTHON");
    }

    /// §9.3: no absolute developer path (this machine's or anyone else's)
    /// may ship as a fallback default. Read this crate's own source and
    /// assert the literal is gone, rather than trusting the const's name.
    #[test]
    fn no_hardcoded_developer_path_in_source() {
        let source = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/whisperx.rs"))
            .expect("read whisperx.rs source");
        // Built from fragments so this assertion's own text never contains
        // the literal it is checking for.
        let volumes_prefix = format!("{}{}", "/Vol", "umes/");
        assert!(
            !source.contains(&volumes_prefix),
            "whisperx.rs must not contain an absolute developer path"
        );
        let old_default = format!("{}{}", "/claude/cutright/", ".venv-whisperx");
        assert!(
            !source.contains(&old_default),
            "whisperx.rs must not contain a hard-coded developer-machine venv path"
        );
    }

    /// `CUTRIGHT_WHISPERX_PYTHON` must win over every other discovery step,
    /// including when it points somewhere neither the project-local venv
    /// nor `PATH` would ever resolve to.
    #[test]
    fn env_override_wins_when_set_to_a_real_file() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        let dir = std::env::temp_dir().join(format!("cutright-wx-discover-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp dir");
        let fake_python = dir.join("fake-python-override");
        fs::write(&fake_python, "#!/bin/sh\n").expect("write fake python");

        env::set_var("CUTRIGHT_WHISPERX_PYTHON", &fake_python);
        let resolved = discover_python().expect("env override must resolve");
        assert_eq!(resolved, fake_python);

        clear_env();
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    /// A `CUTRIGHT_WHISPERX_PYTHON` pointing at a nonexistent file must
    /// fail discovery immediately and name the path it checked — it must
    /// never silently fall through to the project venv or PATH.
    #[test]
    fn env_override_pointing_nowhere_fails_clearly() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        env::set_var(
            "CUTRIGHT_WHISPERX_PYTHON",
            "/nonexistent/cutright-whisperx-test-path/python",
        );
        let error = discover_python().expect_err("nonexistent override must fail");
        match error {
            ProviderError::WhisperXPythonMissing(message) => {
                assert!(
                    message.contains("/nonexistent/cutright-whisperx-test-path/python"),
                    "message: {message}"
                );
            }
            other => panic!("expected WhisperXPythonMissing, got {other:?}"),
        }
        clear_env();
    }
}
