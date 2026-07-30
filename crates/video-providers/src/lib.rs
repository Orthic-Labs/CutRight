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

/// Fallback WhisperX interpreter. WhisperX is the optional alignment verifier
/// and is intentionally the only provider that still carries a workspace-local
/// default path; it is not part of the HeardRight local-audio boundary.
const DEFAULT_WHISPERX_PYTHON: &str = "/Volumes/D/claude/cutright/.venv-whisperx/bin/python";

/// Default Silero speech threshold forwarded to HeardRight's file-VAD
/// capability when a caller does not supply one. HeardRight owns the model and
/// runtime; CutRight only supplies media and policy.
const DEFAULT_VAD_THRESHOLD: f32 = 0.5;
/// Sample rate CutRight expects for VAD input. HeardRight reports the rate it
/// actually analyzed at in the result; this is only a request hint.
const DEFAULT_VAD_SAMPLE_RATE: u32 = 16_000;
/// Minimum speech region duration (ms) requested from HeardRight's file-VAD.
const DEFAULT_MIN_SPEECH_MS: u32 = 160;
/// Minimum silence duration (ms) requested from HeardRight's file-VAD.
const DEFAULT_MIN_SILENCE_MS: u32 = 180;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error(
        "HeardRight engine was not found; set CUTRIGHT_HEARDRIGHT_ENGINE or put heardright-engine on PATH"
    )]
    EngineMissing,
    #[error("HeardRight engine could not start: {0}")]
    Start(#[source] std::io::Error),
    #[error("HeardRight engine request failed: {0}")]
    Engine(String),
    #[error("HeardRight returned invalid JSON: {0}")]
    Json(#[source] serde_json::Error),
    #[error("HeardRight returned transcript text without native timed words")]
    MissingTimedWords,
    #[error("WhisperX Python was not found; set CUTRIGHT_WHISPERX_PYTHON")]
    WhisperXPythonMissing,
    #[error("WhisperX alignment script was not found; set CUTRIGHT_WHISPERX_SCRIPT")]
    WhisperXScriptMissing,
}

/// One supervised HeardRight engine session. Implements both the transcription
/// provider and the file-VAD provider over the same JSON-line stdin/stdout
/// protocol, so CutRight runs a single engine process and never reaches into
/// HeardRight's model internals.
#[derive(Debug)]
pub struct HeardRightProvider {
    engine: PathBuf,
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

impl HeardRightProvider {
    /// Resolve the HeardRight engine location.
    ///
    /// Discovery order (hardening plan §9.3):
    /// 1. explicit `CUTRIGHT_HEARDRIGHT_ENGINE` (or `HEARDRIGHT_ENGINE_BIN`)
    ///    development override;
    /// 2. `heardright-engine` resolved on `PATH`;
    /// 3. a clear [`ProviderError::EngineMissing`] result.
    ///
    /// There is deliberately no hard-coded absolute default and no
    /// model-directory path: HeardRight resolves its own models and runtime.
    pub fn discover() -> Result<Self, ProviderError> {
        let engine = env::var_os("CUTRIGHT_HEARDRIGHT_ENGINE")
            .or_else(|| env::var_os("HEARDRIGHT_ENGINE_BIN"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("heardright-engine"));
        // A bare command name resolves on PATH at spawn time; a path that
        // already carries components must exist now or we fail clearly.
        if engine.components().count() > 1 && !engine.is_file() {
            return Err(ProviderError::EngineMissing);
        }
        Ok(Self {
            engine,
            session: Mutex::new(None),
        })
    }

    fn start_session(&self) -> Result<HeardRightSession, ProviderError> {
        // HeardRight owns model discovery, runtime loading, and platform
        // backend choice. CutRight passes no model-directory paths; the
        // HR_ASR_BACKEND value is a policy hint, not an internal model
        // location.
        let mut child = Command::new(&self.engine)
            .env_remove("HEARDRIGHT_ENGINE_TEST_MODE")
            .env("HR_ASR_BACKEND", "parakeet-tdt")
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

    /// Send one request frame over the supervised session and wait for the
    /// frame whose `schema_name` matches `result_schema`. Returns the full
    /// result frame. `engine_error` frames and a closed stdout surface as
    /// explicit errors — there is never a silent fallback to a bundled model.
    fn request(&self, request: Value, result_schema: &str) -> Result<Value, ProviderError> {
        let mut session = self
            .session
            .lock()
            .map_err(|_| ProviderError::Engine("HeardRight session lock poisoned".into()))?;
        if session.is_none() {
            *session = Some(self.start_session()?);
        }
        let session = session.as_mut().expect("session is initialized");
        session
            .stdin
            .write_all(format!("{}\n", request).as_bytes())
            .map_err(ProviderError::Start)?;
        loop {
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
                Some(name) if name == result_schema => return Ok(frame),
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
    }

    pub fn transcribe(
        &self,
        source_id: &str,
        media: &Path,
    ) -> Result<TranscriptionOutput, ProviderError> {
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
        let frame = self.request(request, "file_transcription_result")?;
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

    /// Run HeardRight's file-VAD capability (`file_vad_regions_v1`) on one
    /// audio file and parse the result into CutRight's [`VadSignal`].
    ///
    /// CutRight supplies the media path and detection policy only; HeardRight
    /// resolves its own Silero model and runtime and reports the sample rate it
    /// analyzed at. Fails clearly if HeardRight is unavailable or does not yet
    /// expose the capability — never a silent fallback to a bundled model.
    pub fn analyze_file_vad(&self, request: &VadRequest) -> Result<VadSignal, ProviderError> {
        let frame_request = serde_json::json!({
            "protocol_major": 1,
            "protocol_minor": 0,
            "schema_name": "file_vad_request",
            "schema_version": 1,
            "engine_version": "cutright/0.1.0",
            "request_id": format!("cutright-{}", std::process::id()),
            "trace_id": "cutright-vad",
            "payload": {
                "kind": "file_vad_request",
                "path": request.audio_path,
                "threshold": request.threshold,
                "min_speech_ms": DEFAULT_MIN_SPEECH_MS,
                "min_silence_ms": DEFAULT_MIN_SILENCE_MS,
            },
        });
        let frame = self.request(frame_request, "file_vad_result")?;
        let payload = frame
            .get("payload")
            .ok_or_else(|| ProviderError::Engine("file_vad_result frame has no payload".into()))?;
        parse_vad_result(&request.source_id, payload)
    }
}

/// Parse a HeardRight `file_vad_result` payload into CutRight's [`VadSignal`].
///
/// The provider label always reflects HeardRight (`heardright-silero`); the
/// sample rate is taken from the engine result so CutRight never assumes a
/// model input rate. Regions are validated to be non-inverted.
fn parse_vad_result(source_id: &str, payload: &Value) -> Result<VadSignal, ProviderError> {
    let sample_rate = payload
        .get("sample_rate")
        .and_then(Value::as_u64)
        .ok_or_else(|| ProviderError::Engine("file_vad_result has no sample_rate".into()))?
        as u32;
    let raw_regions = payload
        .get("regions")
        .and_then(Value::as_array)
        .ok_or_else(|| ProviderError::Engine("file_vad_result has no regions".into()))?;
    let mut regions = Vec::with_capacity(raw_regions.len());
    for (index, region) in raw_regions.iter().enumerate() {
        let start_ms = region
            .get("start_ms")
            .and_then(Value::as_i64)
            .ok_or_else(|| ProviderError::Engine(format!("vad region {index} has no start_ms")))?;
        let end_ms = region
            .get("end_ms")
            .and_then(Value::as_i64)
            .ok_or_else(|| ProviderError::Engine(format!("vad region {index} has no end_ms")))?;
        let mean_probability = region
            .get("mean_probability")
            .and_then(Value::as_f64)
            .ok_or_else(|| {
                ProviderError::Engine(format!("vad region {index} has no mean_probability"))
            })? as f32;
        if end_ms < start_ms {
            return Err(ProviderError::Engine(format!(
                "vad region {index} ends before it starts"
            )));
        }
        regions.push(VadRegion {
            start_ms,
            end_ms,
            mean_probability,
        });
    }
    Ok(VadSignal {
        schema_version: SCHEMA_VERSION,
        source_id: source_id.to_string(),
        sample_rate,
        provider: "heardright-silero".into(),
        regions,
    })
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
                provider: TranscriptionProvider::id(self).into(),
                reason: error.to_string(),
            },
        )
    }
}

impl VadProvider for HeardRightProvider {
    fn id(&self) -> &'static str {
        "heardright-silero"
    }

    fn analyze(&self, request: &VadRequest) -> Result<VadSignal, CoreProviderError> {
        self.analyze_file_vad(request)
            .map_err(|error| CoreProviderError::Unavailable {
                provider: VadProvider::id(self).into(),
                reason: error.to_string(),
            })
    }
}

/// Thin CutRight client for HeardRight's file-VAD capability.
///
/// CutRight does not bundle a Silero model or worker and does not reference any
/// HeardRight-internal model path. It invokes the HeardRight engine (the same
/// supervised JSON-line session used for transcription) and lets HeardRight
/// resolve its own model and runtime.
///
/// # HeardRight location
/// The engine is resolved via `CUTRIGHT_HEARDRIGHT_ENGINE` (or
/// `HEARDRIGHT_ENGINE_BIN`), falling back to `heardright-engine` on `PATH`.
/// There is no hard-coded absolute default. If HeardRight is unavailable the
/// call fails with a clear [`ProviderError`]; there is never a silent fallback
/// to a bundled model.
pub mod audio_vad {
    use crate::{
        HeardRightProvider, ProviderError, DEFAULT_VAD_SAMPLE_RATE, DEFAULT_VAD_THRESHOLD,
    };
    use std::path::Path;
    use video_core::{providers::VadRequest, VadSignal};

    /// Analyze one audio file through HeardRight's file-VAD capability.
    ///
    /// The `source_id` defaults to the file stem. Use
    /// [`HeardRightProvider::analyze_file_vad`] directly to control the full
    /// [`VadRequest`] (explicit source id, threshold, expected sample rate).
    pub fn analyze_file(audio_path: &Path) -> Result<VadSignal, ProviderError> {
        let source_id = audio_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("audio")
            .to_string();
        let provider = HeardRightProvider::discover()?;
        provider.analyze_file_vad(&VadRequest {
            source_id,
            audio_path: audio_path.to_path_buf(),
            sample_rate: DEFAULT_VAD_SAMPLE_RATE,
            threshold: DEFAULT_VAD_THRESHOLD,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("{prefix}-{unique}"));
        fs::create_dir_all(&root).expect("create engine test directory");
        root
    }

    fn write_fake_engine(root: &Path, response_line: &str) -> PathBuf {
        let engine = root.join("fake-engine");
        fs::write(
            &engine,
            format!(
                "#!/bin/sh\nwhile IFS= read -r line; do\n  printf '%s\\n' '{response_line}'\ndone\n"
            ),
        )
        .expect("write fake engine");
        fs::set_permissions(&engine, fs::Permissions::from_mode(0o700))
            .expect("make fake engine executable");
        engine
    }

    #[test]
    fn keeps_one_engine_session_for_multiple_transcriptions() {
        let root = unique_temp_dir("cutright-engine-test");
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

    #[test]
    fn parses_heardright_vad_result_into_vad_signal() {
        let payload = serde_json::json!({
            "sample_rate": 16_000,
            "provider": "heardright-silero",
            "model_revision": "silero-v6",
            "threshold": 0.5,
            "min_speech_ms": 160,
            "min_silence_ms": 180,
            "regions": [
                { "start_ms": 120, "end_ms": 940, "mean_probability": 0.91 },
                { "start_ms": 1_500, "end_ms": 2_300, "mean_probability": 0.77 }
            ],
        });
        let signal = parse_vad_result("source-a", &payload).expect("parse vad result");
        assert_eq!(signal.schema_version, SCHEMA_VERSION);
        assert_eq!(signal.source_id, "source-a");
        assert_eq!(signal.sample_rate, 16_000);
        assert_eq!(signal.provider, "heardright-silero");
        assert_eq!(signal.regions.len(), 2);
        assert_eq!(signal.regions[0].start_ms, 120);
        assert_eq!(signal.regions[0].end_ms, 940);
        assert!((signal.regions[0].mean_probability - 0.91).abs() < 1e-6);
        assert_eq!(signal.regions[1].start_ms, 1_500);
        assert_eq!(signal.regions[1].end_ms, 2_300);
    }

    #[test]
    fn parse_vad_result_accepts_an_empty_region_list() {
        let payload = serde_json::json!({ "sample_rate": 16_000, "regions": [] });
        let signal = parse_vad_result("silent", &payload).expect("parse empty regions");
        assert!(signal.regions.is_empty());
        assert_eq!(signal.provider, "heardright-silero");
    }

    #[test]
    fn parse_vad_result_rejects_missing_fields() {
        let no_regions = serde_json::json!({ "sample_rate": 16_000 });
        assert!(parse_vad_result("source-a", &no_regions).is_err());
        let no_sample_rate = serde_json::json!({ "regions": [] });
        assert!(parse_vad_result("source-a", &no_sample_rate).is_err());
        let bad_region = serde_json::json!({
            "sample_rate": 16_000,
            "regions": [{ "start_ms": 100 }],
        });
        assert!(parse_vad_result("source-a", &bad_region).is_err());
    }

    #[test]
    fn parse_vad_result_rejects_an_inverted_region() {
        let payload = serde_json::json!({
            "sample_rate": 16_000,
            "regions": [{ "start_ms": 900, "end_ms": 100, "mean_probability": 0.5 }],
        });
        let error = parse_vad_result("source-a", &payload).expect_err("inverted region rejected");
        assert!(matches!(error, ProviderError::Engine(_)));
    }

    #[test]
    fn analyzes_file_vad_through_a_fake_engine() {
        let root = unique_temp_dir("cutright-vad-test");
        let engine = write_fake_engine(
            &root,
            "{\"schema_name\":\"file_vad_result\",\"payload\":{\"sample_rate\":16000,\"provider\":\"heardright-silero\",\"model_revision\":\"silero-v6\",\"threshold\":0.5,\"min_speech_ms\":160,\"min_silence_ms\":180,\"regions\":[{\"start_ms\":100,\"end_ms\":900,\"mean_probability\":0.9}]}}",
        );
        let provider = HeardRightProvider {
            engine,
            session: Mutex::new(None),
        };
        let signal = provider
            .analyze_file_vad(&VadRequest {
                source_id: "source-a".into(),
                audio_path: PathBuf::from("/tmp/source-a-16k.f32"),
                sample_rate: 16_000,
                threshold: 0.5,
            })
            .expect("fake engine returns a vad result");
        assert_eq!(signal.provider, "heardright-silero");
        assert_eq!(signal.source_id, "source-a");
        assert_eq!(signal.sample_rate, 16_000);
        assert_eq!(signal.regions.len(), 1);
        assert_eq!(signal.regions[0].start_ms, 100);
        assert_eq!(signal.regions[0].end_ms, 900);
        drop(provider);
        fs::remove_dir_all(root).expect("remove vad test directory");
    }

    #[test]
    fn file_vad_surfaces_an_engine_error_clearly() {
        let root = unique_temp_dir("cutright-vad-error-test");
        let engine = write_fake_engine(
            &root,
            "{\"schema_name\":\"engine_error\",\"error\":{\"message\":\"file_vad_regions_v1 not supported\"}}",
        );
        let provider = HeardRightProvider {
            engine,
            session: Mutex::new(None),
        };
        let error = provider
            .analyze_file_vad(&VadRequest {
                source_id: "source-a".into(),
                audio_path: PathBuf::from("/tmp/source-a-16k.f32"),
                sample_rate: 16_000,
                threshold: 0.5,
            })
            .expect_err("engine_error must surface");
        match error {
            ProviderError::Engine(message) => assert!(message.contains("file_vad_regions_v1")),
            other => panic!("expected ProviderError::Engine, got {other:?}"),
        }
        drop(provider);
        fs::remove_dir_all(root).expect("remove vad error test directory");
    }

    #[test]
    fn file_vad_fails_clearly_when_the_engine_is_unreachable() {
        let root = unique_temp_dir("cutright-vad-missing-test");
        let provider = HeardRightProvider {
            engine: root.join("does-not-exist/heardright-engine"),
            session: Mutex::new(None),
        };
        let error = provider
            .analyze_file_vad(&VadRequest {
                source_id: "source-a".into(),
                audio_path: PathBuf::from("/tmp/source-a-16k.f32"),
                sample_rate: 16_000,
                threshold: 0.5,
            })
            .expect_err("missing engine must fail clearly");
        assert!(
            matches!(error, ProviderError::Start(_)),
            "unexpected: {error:?}"
        );
        fs::remove_dir_all(root).expect("remove vad missing test directory");
    }
}
