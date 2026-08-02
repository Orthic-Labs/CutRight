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
use crate::process_runner::{CancellationToken, ProcessSpec, TempFileGuard};

/// Fallback WhisperX interpreter. WhisperX is the optional alignment
/// verifier and is intentionally the only provider that still carries a
/// workspace-local default path; it is not part of the HeardRight
/// local-audio boundary.
const DEFAULT_WHISPERX_PYTHON: &str = "/Volumes/D/claude/cutright/.venv-whisperx/bin/python";
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

impl WhisperXProvider {
    pub fn discover() -> Result<Self, ProviderError> {
        let python = env::var_os("CUTRIGHT_WHISPERX_PYTHON")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_WHISPERX_PYTHON));
        if !python.is_file() {
            return Err(ProviderError::WhisperXPythonMissing);
        }
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
        let outcome = crate::process_runner::run_process(&spec, &CancellationToken::new())?;
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
