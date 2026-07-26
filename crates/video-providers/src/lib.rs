use serde_json::Value;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use thiserror::Error;
use video_core::{Transcript, Word, SCHEMA_VERSION};

const DEFAULT_ENGINE: &str =
    "/Volumes/D/claude/heardright/tauri-app-next/heardright-engine/target/release/heardright-engine";
const DEFAULT_MODELS: &str = "/Volumes/D/claude/heardright/model_registry/coreml";

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
}

#[derive(Debug, Clone)]
pub struct HeardRightProvider {
    engine: PathBuf,
    models: PathBuf,
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
        Ok(Self { engine, models })
    }

    pub fn transcribe(&self, source_id: &str, media: &Path) -> Result<Transcript, ProviderError> {
        let mut child = Command::new(&self.engine)
            .arg(&self.models)
            .env_remove("HEARDRIGHT_ENGINE_TEST_MODE")
            .env("HR_ASR_BACKEND", "parakeet-tdt")
            .env("HR_COREML_MODEL_DIR", self.models.join("parakeet-tdt-v3"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(ProviderError::Start)?;
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
        child
            .stdin
            .as_mut()
            .expect("piped stdin")
            .write_all(format!("{}\n", request).as_bytes())
            .map_err(ProviderError::Start)?;
        let stdout = child.stdout.take().expect("piped stdout");
        let mut response = None;
        for line in BufReader::new(stdout).lines() {
            let line = line.map_err(ProviderError::Start)?;
            let frame: Value = serde_json::from_str(&line).map_err(ProviderError::Json)?;
            match frame.get("schema_name").and_then(Value::as_str) {
                Some("file_transcription_result") => {
                    response = Some(frame);
                    break;
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
        }
        let _ = child.kill();
        let frame = response.ok_or_else(|| ProviderError::Engine("no result frame".into()))?;
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
        Ok(Transcript {
            schema_version: SCHEMA_VERSION,
            provider: "heardright-parakeet-tdt".into(),
            source_id: source_id.into(),
            language: "en".into(),
            words,
            events: Vec::new(),
        })
    }
}
