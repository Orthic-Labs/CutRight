use serde_json::Value;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;
use thiserror::Error;
use video_core::{
    providers::{
        ProviderError as CoreProviderError, TranscriptionOutput, TranscriptionProvider,
        TranscriptionRequest, VadProvider, VadRequest,
    },
    Transcript, VadRegion, VadSignal, Word, SCHEMA_VERSION,
};

const DEFAULT_ENGINE: &str =
    "/Volumes/D/claude/heardright/tauri-app-next/heardright-engine/target/release/heardright-engine";
const DEFAULT_MODELS: &str = "/Volumes/D/claude/heardright/model_registry/coreml";
const DEFAULT_SILERO_MODEL: &str =
    "/Volumes/D/claude/heardright/tauri-app-next/src-tauri/resources/vad/silero_vad_16k.mlmodelc";
const DEFAULT_WHISPERX_PYTHON: &str = "/Volumes/D/claude/cutright/.venv-whisperx/bin/python";

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("HeardRight engine was not found; set CUTRIGHT_HEARDRIGHT_ENGINE")]
    EngineMissing,
    #[error("HeardRight model directory was not found; set CUTRIGHT_HEARDRIGHT_MODELS_DIR")]
    ModelsMissing,
    #[error("HeardRight engine could not start: {0}")]
    Start(#[source] std::io::Error),
    #[error("HeardRight engine request failed: {0}")]
    Engine(String),
    #[error("HeardRight returned invalid JSON: {0}")]
    Json(#[source] serde_json::Error),
    #[error("HeardRight returned transcript text without native timed words")]
    MissingTimedWords,
    #[error("CutRight Silero VAD model was not found; set CUTRIGHT_SILERO_MODEL")]
    VadModelMissing,
    #[error("CutRight Silero VAD worker binary was not found; set CUTRIGHT_SILERO_VAD_BIN")]
    VadWorkerMissing,
    #[error("CutRight Silero VAD worker failed: {0}")]
    VadWorker(String),
    #[error("WhisperX Python was not found; set CUTRIGHT_WHISPERX_PYTHON")]
    WhisperXPythonMissing,
    #[error("WhisperX alignment script was not found; set CUTRIGHT_WHISPERX_SCRIPT")]
    WhisperXScriptMissing,
}

#[derive(Debug)]
pub struct HeardRightProvider {
    engine: PathBuf,
    models: PathBuf,
    session: Mutex<Option<HeardRightSession>>,
}

#[derive(Debug)]
struct HeardRightSession {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

#[derive(Debug, Clone)]
pub struct WhisperXProvider {
    python: PathBuf,
    script: PathBuf,
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
        Ok(Self { python, script })
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
        let output = std::env::temp_dir().join(format!(
            "cutright-whisperx-{}-{}.json",
            std::process::id(),
            blake3::hash(request.source_id.as_bytes()).to_hex()
        ));
        let result = Command::new(&self.python)
            .arg(&self.script)
            .arg(&request.source_path)
            .arg(&output)
            .output()
            .map_err(|error| CoreProviderError::Unavailable {
                provider: self.id().into(),
                reason: error.to_string(),
            })?;
        if !result.status.success() {
            return Err(CoreProviderError::Rejected {
                provider: self.id().into(),
                reason: String::from_utf8_lossy(&result.stderr).trim().to_string(),
            });
        }
        let bytes = fs::read(&output).map_err(|error| CoreProviderError::Rejected {
            provider: self.id().into(),
            reason: error.to_string(),
        })?;
        let _ = fs::remove_file(&output);
        let raw_response: Value =
            serde_json::from_slice(&bytes).map_err(|error| CoreProviderError::Rejected {
                provider: self.id().into(),
                reason: format!("invalid WhisperX response: {error}"),
            })?;
        let response: Vec<WhisperXWord> =
            serde_json::from_value(raw_response.clone()).map_err(|error| {
                CoreProviderError::Rejected {
                    provider: self.id().into(),
                    reason: format!("invalid WhisperX word response: {error}"),
                }
            })?;
        if response.is_empty() {
            return Err(CoreProviderError::Rejected {
                provider: self.id().into(),
                reason: "WhisperX returned no timed words".into(),
            });
        }
        let mut previous_end_ms = 0;
        let mut words = Vec::with_capacity(response.len());
        for (index, word) in response.into_iter().enumerate() {
            if !word.s.is_finite() || !word.e.is_finite() || word.s < 0.0 || word.e <= word.s {
                return Err(CoreProviderError::Rejected {
                    provider: self.id().into(),
                    reason: format!("invalid WhisperX interval at word {index}"),
                });
            }
            let text = word.w.trim().to_string();
            if text.is_empty() {
                return Err(CoreProviderError::Rejected {
                    provider: self.id().into(),
                    reason: format!("empty WhisperX word at index {index}"),
                });
            }
            let start_ms = (word.s * 1_000.0).round() as i64;
            let end_ms = (word.e * 1_000.0).round() as i64;
            if start_ms < previous_end_ms {
                return Err(CoreProviderError::Rejected {
                    provider: self.id().into(),
                    reason: format!("non-monotonic WhisperX word at index {index}"),
                });
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
                provider: self.id().into(),
                source_id: request.source_id.clone(),
                language: request
                    .language_hint
                    .clone()
                    .unwrap_or_else(|| "und".into()),
                words,
                events: Vec::new(),
            },
            raw_response,
            provider_model: self.model_id().into(),
            warnings: Vec::new(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SileroVadProvider {
    model: PathBuf,
}

#[derive(Debug, serde::Deserialize)]
struct SileroResponse {
    provider: String,
    sample_rate: u32,
    regions: Vec<SileroRegion>,
}

#[derive(Debug, serde::Deserialize)]
struct SileroRegion {
    start_ms: i64,
    end_ms: i64,
    mean_probability: f32,
}

impl SileroVadProvider {
    pub fn discover() -> Result<Self, ProviderError> {
        let model = env::var_os("CUTRIGHT_SILERO_MODEL")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_SILERO_MODEL));
        if !model.is_dir() {
            return Err(ProviderError::VadModelMissing);
        }
        Ok(Self { model })
    }

    fn worker_binary(&self) -> Result<PathBuf, ProviderError> {
        if let Some(path) = env::var_os("CUTRIGHT_SILERO_VAD_BIN").map(PathBuf::from) {
            if path.is_file() {
                return Ok(path);
            }
            return Err(ProviderError::VadWorkerMissing);
        }
        let binary =
            std::env::temp_dir().join(format!("cutright-silero-vad-{}", env!("CARGO_PKG_VERSION")));
        let embedded = include_bytes!(env!("CUTRIGHT_SILERO_VAD"));
        if fs::read(&binary).ok().as_deref() != Some(embedded.as_slice()) {
            fs::write(&binary, embedded).map_err(ProviderError::Start)?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&binary, fs::Permissions::from_mode(0o700))
                .map_err(ProviderError::Start)?;
        }
        Ok(binary)
    }
}

impl VadProvider for SileroVadProvider {
    fn id(&self) -> &'static str {
        "silero-coreml"
    }

    fn analyze(&self, request: &VadRequest) -> Result<VadSignal, CoreProviderError> {
        if request.sample_rate != 16_000 {
            return Err(CoreProviderError::Rejected {
                provider: self.id().into(),
                reason: "Silero CoreML worker requires 16000 Hz PCM".into(),
            });
        }
        let binary = self
            .worker_binary()
            .map_err(|error| CoreProviderError::Unavailable {
                provider: self.id().into(),
                reason: error.to_string(),
            })?;
        let request_json = serde_json::json!({
            "audio_path": request.audio_path,
            "model_path": self.model,
            "threshold": request.threshold,
            "sample_rate": request.sample_rate,
            "min_speech_ms": 160,
            "min_silence_ms": 180,
        });
        let mut child = Command::new(binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| CoreProviderError::Unavailable {
                provider: self.id().into(),
                reason: error.to_string(),
            })?;
        child
            .stdin
            .as_mut()
            .expect("piped stdin")
            .write_all(&serde_json::to_vec(&request_json).expect("request JSON"))
            .map_err(|error| CoreProviderError::Unavailable {
                provider: self.id().into(),
                reason: error.to_string(),
            })?;
        let output = child
            .wait_with_output()
            .map_err(|error| CoreProviderError::Unavailable {
                provider: self.id().into(),
                reason: error.to_string(),
            })?;
        if !output.status.success() {
            return Err(CoreProviderError::Rejected {
                provider: self.id().into(),
                reason: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        let output: SileroResponse = serde_json::from_slice(&output.stdout).map_err(|error| {
            CoreProviderError::Rejected {
                provider: self.id().into(),
                reason: format!("invalid worker response: {error}"),
            }
        })?;
        if output.provider != self.id() || output.sample_rate != request.sample_rate {
            return Err(CoreProviderError::Rejected {
                provider: self.id().into(),
                reason: "worker response identity or sample rate mismatch".into(),
            });
        }
        Ok(VadSignal {
            schema_version: SCHEMA_VERSION,
            source_id: request.source_id.clone(),
            sample_rate: request.sample_rate,
            provider: output.provider,
            regions: output
                .regions
                .into_iter()
                .map(|region| VadRegion {
                    start_ms: region.start_ms,
                    end_ms: region.end_ms,
                    mean_probability: region.mean_probability,
                })
                .collect(),
        })
    }
}

impl HeardRightProvider {
    pub fn discover() -> Result<Self, ProviderError> {
        let engine = env::var_os("CUTRIGHT_HEARDRIGHT_ENGINE")
            .or_else(|| env::var_os("HEARDRIGHT_ENGINE_BIN"))
            .map(PathBuf::from)
            .or_else(|| {
                let path = PathBuf::from(DEFAULT_ENGINE);
                path.is_file().then_some(path)
            })
            .unwrap_or_else(|| PathBuf::from("heardright-engine"));
        let models = env::var_os("CUTRIGHT_HEARDRIGHT_MODELS_DIR")
            .or_else(|| env::var_os("HR_MODELS_DIR"))
            .map(PathBuf::from)
            .or_else(|| {
                let path = PathBuf::from(DEFAULT_MODELS);
                path.is_dir().then_some(path)
            })
            .unwrap_or_else(|| PathBuf::from(DEFAULT_MODELS));
        if engine.components().count() > 1 && !engine.is_file() {
            return Err(ProviderError::EngineMissing);
        }
        if !models.is_dir() {
            return Err(ProviderError::ModelsMissing);
        }
        let bundle = models.join("parakeet-tdt-v3");
        if !bundle.join("pipeline.json").is_file() {
            return Err(ProviderError::ModelsMissing);
        }
        Ok(Self {
            engine,
            models,
            session: Mutex::new(None),
        })
    }

    fn start_session(&self) -> Result<HeardRightSession, ProviderError> {
        let mut child = Command::new(&self.engine)
            .arg(&self.models)
            .env_remove("HEARDRIGHT_ENGINE_TEST_MODE")
            .env("HR_ASR_BACKEND", "parakeet-tdt")
            .env("HR_COREML_MODEL_DIR", self.models.join("parakeet-tdt-v3"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(ProviderError::Start)?;
        Ok(HeardRightSession {
            stdin: child
                .stdin
                .take()
                .expect("HeardRight session stdin is piped"),
            stdout: BufReader::new(
                child
                    .stdout
                    .take()
                    .expect("HeardRight session stdout is piped"),
            ),
            child,
        })
    }

    pub fn transcribe(
        &self,
        source_id: &str,
        media: &Path,
    ) -> Result<TranscriptionOutput, ProviderError> {
        let mut session = self
            .session
            .lock()
            .map_err(|_| ProviderError::Engine("HeardRight session lock poisoned".into()))?;
        if session.is_none() {
            *session = Some(self.start_session()?);
        }
        let session = session.as_mut().expect("session is initialized");
        let request = serde_json::json!({
            "protocol_major": 1,
            "protocol_minor": 0,
            "schema_name": "file_transcription_request",
            "schema_version": 1,
            "engine_version": "cutright/0.1.0",
            "request_id": format!("cutright-{}", std::process::id()),
            "trace_id": "cutright-transcribe",
            "payload": { "kind": "file_transcription_request", "path": media },
        });
        session
            .stdin
            .write_all(format!("{}\n", request).as_bytes())
            .map_err(ProviderError::Start)?;
        let frame = loop {
            let mut line = String::new();
            let bytes = session
                .stdout
                .read_line(&mut line)
                .map_err(ProviderError::Start)?;
            if bytes == 0 {
                return Err(ProviderError::Engine(
                    "HeardRight engine closed stdout before returning a result; inspect stderr above"
                        .into(),
                ));
            }
            let frame: Value = serde_json::from_str(&line).map_err(ProviderError::Json)?;
            match frame.get("schema_name").and_then(Value::as_str) {
                Some("file_transcription_result") => {
                    break frame;
                }
                Some("engine_error") => {
                    return Err(ProviderError::Engine(
                        frame
                            .pointer("/error/message")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown engine error")
                            .to_string(),
                    ));
                }
                _ => {}
            }
        };
        let payload = frame
            .get("payload")
            .ok_or_else(|| ProviderError::Engine("result frame has no payload".into()))?;
        let text = payload
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let native_words = payload
            .get("words")
            .and_then(Value::as_array)
            .ok_or(ProviderError::MissingTimedWords)?;
        if native_words.is_empty() && !text.is_empty() {
            return Err(ProviderError::MissingTimedWords);
        }
        let words =
            native_words
                .iter()
                .enumerate()
                .map(|(index, word)| {
                    Ok(Word {
                        id: format!("w_{index:06}"),
                        source_word_id: None,
                        text: word
                            .get("text")
                            .and_then(Value::as_str)
                            .ok_or_else(|| ProviderError::Engine("timed word has no text".into()))?
                            .to_string(),
                        start_ms: word.get("start_ms").and_then(Value::as_i64).ok_or_else(
                            || ProviderError::Engine("timed word has no start_ms".into()),
                        )?,
                        end_ms: word.get("end_ms").and_then(Value::as_i64).ok_or_else(|| {
                            ProviderError::Engine("timed word has no end_ms".into())
                        })?,
                        confidence: 1.0,
                        speaker: None,
                        kind: "word".into(),
                    })
                })
                .collect::<Result<Vec<_>, ProviderError>>()?;
        Ok(TranscriptionOutput {
            transcript: Transcript {
                schema_version: SCHEMA_VERSION,
                provider: "heardright-parakeet-tdt".into(),
                source_id: source_id.into(),
                language: "en".into(),
                words,
                events: Vec::new(),
            },
            raw_response: frame,
            provider_model: self.model_id().into(),
            warnings: Vec::new(),
        })
    }
}

impl Drop for HeardRightProvider {
    fn drop(&mut self) {
        if let Ok(session) = self.session.get_mut() {
            if let Some(session) = session.as_mut() {
                let _ = session.child.kill();
                let _ = session.child.wait();
            }
        }
    }
}

impl TranscriptionProvider for HeardRightProvider {
    fn id(&self) -> &'static str {
        "heardright-parakeet-tdt"
    }

    fn model_id(&self) -> &'static str {
        "parakeet-tdt-v3-coreml"
    }

    fn transcribe(
        &self,
        request: &TranscriptionRequest,
    ) -> Result<TranscriptionOutput, CoreProviderError> {
        HeardRightProvider::transcribe(self, &request.source_id, &request.source_path).map_err(
            |error| CoreProviderError::Unavailable {
                provider: self.id().into(),
                reason: error.to_string(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn materializes_the_build_time_silero_worker() {
        let provider = SileroVadProvider {
            model: PathBuf::new(),
        };
        let worker = provider.worker_binary().expect("materialize Silero worker");
        assert!(worker.is_file());
        assert_eq!(
            fs::read(&worker).expect("read materialized worker"),
            include_bytes!(env!("CUTRIGHT_SILERO_VAD"))
        );
        assert_ne!(
            worker.extension().and_then(|value| value.to_str()),
            Some("swift")
        );
    }

    #[test]
    fn keeps_one_engine_session_for_multiple_transcriptions() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cutright-engine-test-{unique}"));
        fs::create_dir_all(&root).expect("create engine test directory");
        let starts = root.join("starts.log");
        let engine = root.join("fake-engine");
        fs::write(
            &engine,
            format!(
                "#!/bin/sh\nprintf 'started\\n' >> '{}'\nwhile IFS= read -r line; do\n  printf '%s\\n' '{{\"schema_name\":\"file_transcription_result\",\"payload\":{{\"text\":\"hello\",\"words\":[{{\"text\":\"hello\",\"start_ms\":0,\"end_ms\":100}}]}}}}'\ndone\n",
                starts.display()
            ),
        )
        .expect("write fake engine");
        fs::set_permissions(&engine, fs::Permissions::from_mode(0o700))
            .expect("make fake engine executable");
        let provider = HeardRightProvider {
            engine,
            models: root.clone(),
            session: Mutex::new(None),
        };
        let first = provider.transcribe("one", Path::new("/tmp/one.mp4"));
        let second = provider.transcribe("two", Path::new("/tmp/two.mp4"));
        assert_eq!(first.expect("first result").transcript.words.len(), 1);
        assert_eq!(second.expect("second result").transcript.source_id, "two");
        drop(provider);
        assert_eq!(
            fs::read_to_string(&starts)
                .expect("read starts")
                .lines()
                .count(),
            1
        );
        fs::remove_dir_all(root).expect("remove engine test directory");
    }
}
