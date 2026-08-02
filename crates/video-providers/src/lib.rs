mod heardright;
mod vad;
mod whisperx;

use serde_json::Value;
use std::path::Path;
use video_core::{
    providers::{
        ProviderError as CoreProviderError, TranscriptionOutput, TranscriptionProvider,
        TranscriptionRequest, VadProvider, VadRequest,
    },
    VadSignal,
};

pub use heardright::{EngineIdentity, ProviderError};
pub use vad::VadProvenance;
pub use whisperx::WhisperXProvider;

/// Default Silero speech threshold forwarded to HeardRight's file-VAD
/// capability when a caller does not supply one. HeardRight owns the model
/// and runtime; CutRight only supplies media and policy.
const DEFAULT_VAD_THRESHOLD: f32 = 0.5;
/// Sample rate CutRight expects for VAD input. HeardRight reports the rate it
/// actually analyzed at in the result; this is only a request hint.
const DEFAULT_VAD_SAMPLE_RATE: u32 = 16_000;
/// Minimum speech region duration (ms) requested from HeardRight's file-VAD.
const DEFAULT_MIN_SPEECH_MS: u32 = 160;
/// Minimum silence duration (ms) requested from HeardRight's file-VAD.
const DEFAULT_MIN_SILENCE_MS: u32 = 180;
/// One supervised HeardRight engine session. Implements both the
/// transcription provider and the file-VAD provider over the same JSON-line
/// stdin/stdout protocol, so CutRight runs a single engine process and never
/// reaches into HeardRight's model internals.
///
/// Protocol behavior (§9.2): health/capability handshake before use, unique
/// request/trace IDs, exact response correlation, protocol-major rejection
/// with minor-version negotiation, per-request timeout, bounded stderr
/// capture, and exactly one controlled restart after an unexpected engine
/// exit — all implemented in [`heardright::HeardRightClient`]. There is
/// never a model download or network fallback.
#[derive(Debug)]
pub struct HeardRightProvider {
    client: heardright::HeardRightClient,
}

impl HeardRightProvider {
    /// Resolve the HeardRight engine location and construct a provider that
    /// has not yet started a session (lazy start on first request). See
    /// [`heardright::discover_engine`] for the discovery order (§9.3).
    pub fn discover() -> Result<Self, ProviderError> {
        Ok(Self {
            client: heardright::HeardRightClient::discover()?,
        })
    }

    #[cfg(test)]
    fn with_engine(engine: std::path::PathBuf) -> Self {
        Self {
            client: heardright::HeardRightClient::with_engine(engine),
        }
    }

    /// The engine/model/protocol identity negotiated during the
    /// handshake, if a session has been started yet.
    pub fn engine_identity(&self) -> Option<EngineIdentity> {
        self.client.identity()
    }

    pub fn model_id(&self) -> &'static str {
        "parakeet-tdt-v3-coreml"
    }

    /// Public, download-free health/capability probe (§11.3). Performs only
    /// the protocol handshake — no transcription and no VAD request is
    /// sent, and no model download or network fallback occurs beyond what
    /// the handshake itself requires — and returns the negotiated engine
    /// identity. Callers (e.g. `videoctl doctor`) can use this to verify
    /// the engine is reachable and protocol-compatible without risking a
    /// model download or a full transcription session.
    pub fn health(&self) -> Result<EngineIdentity, ProviderError> {
        self.client.health()
    }

    pub fn transcribe(
        &self,
        source_id: &str,
        media: &Path,
    ) -> Result<TranscriptionOutput, ProviderError> {
        let request_id = format!(
            "cutright-{}-{}",
            std::process::id(),
            blake3::hash(source_id.as_bytes()).to_hex()
        );
        let media_owned = media.to_path_buf();
        let frame = self.client.request(
            &request_id,
            move |request_id, trace_id| {
                serde_json::json!({
                    "protocol_major": heardright::CLIENT_PROTOCOL_MAJOR,
                    "protocol_minor": heardright::CLIENT_PROTOCOL_MINOR,
                    "schema_name": "file_transcription_request",
                    "schema_version": 1,
                    "engine_version": "cutright/0.1.0",
                    "request_id": request_id,
                    "trace_id": trace_id,
                    "payload": { "kind": "file_transcription_request", "path": media_owned },
                })
            },
            "file_transcription_result",
            heardright::request_timeout(),
        )?;
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
                    Ok(video_core::Word {
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
        let identity = self.client.identity();
        let provenance_envelope = serde_json::json!({
            "result": frame,
            "request": { "request_id": request_id },
            "engine_identity": identity,
        });
        Ok(TranscriptionOutput {
            transcript: video_core::Transcript {
                schema_version: video_core::SCHEMA_VERSION,
                provider: "heardright-parakeet-tdt".into(),
                source_id: source_id.into(),
                language: "en".into(),
                words,
                events: Vec::new(),
            },
            raw_response: provenance_envelope,
            provider_model: self.model_id().into(),
            warnings: Vec::new(),
        })
    }

    /// Run HeardRight's file-VAD capability (`file_vad_regions_v1`) on one
    /// audio file and parse the result into CutRight's [`VadSignal`].
    ///
    /// CutRight supplies the media path and detection policy only; HeardRight
    /// resolves its own Silero model and runtime and reports the sample rate
    /// it analyzed at. Fails clearly if HeardRight is unavailable or does not
    /// yet expose the capability — never a silent fallback to a bundled
    /// model.
    pub fn analyze_file_vad(&self, request: &VadRequest) -> Result<VadSignal, ProviderError> {
        self.analyze_file_vad_with_provenance(request)
            .map(|(signal, _provenance)| signal)
    }

    /// Same as [`Self::analyze_file_vad`], additionally returning the full
    /// provenance record (§10.7): model revision/hash, runtime/backend,
    /// threshold, min speech/silence, sample rate, decode policy, request
    /// hash, and warnings. `VadSignal` itself (defined upstream in
    /// `video-core`) is not extended; this is an additive companion value.
    pub fn analyze_file_vad_with_provenance(
        &self,
        request: &VadRequest,
    ) -> Result<(VadSignal, VadProvenance), ProviderError> {
        let request_id = format!(
            "cutright-{}-{}",
            std::process::id(),
            blake3::hash(request.source_id.as_bytes()).to_hex()
        );
        let audio_path = request.audio_path.clone();
        let threshold = request.threshold;
        let sent_request = std::sync::Mutex::new(None);
        let frame = self.client.request(
            &request_id,
            |request_id, trace_id| {
                let built = serde_json::json!({
                    "protocol_major": heardright::CLIENT_PROTOCOL_MAJOR,
                    "protocol_minor": heardright::CLIENT_PROTOCOL_MINOR,
                    "schema_name": "file_vad_request",
                    "schema_version": 1,
                    "engine_version": "cutright/0.1.0",
                    "request_id": request_id,
                    "trace_id": trace_id,
                    "payload": {
                        "kind": "file_vad_request",
                        "path": audio_path,
                        "threshold": threshold,
                        "min_speech_ms": DEFAULT_MIN_SPEECH_MS,
                        "min_silence_ms": DEFAULT_MIN_SILENCE_MS,
                    },
                });
                *sent_request.lock().expect("provenance request lock") = Some(built.clone());
                built
            },
            "file_vad_result",
            heardright::request_timeout(),
        )?;
        let payload = frame
            .get("payload")
            .ok_or_else(|| ProviderError::Engine("file_vad_result frame has no payload".into()))?;
        let signal = vad::parse_vad_result(&request.source_id, payload)?;
        // Read the decoded audio for provenance hashing only after the
        // engine has actually accepted and answered the request — an
        // engine-unavailable/missing-audio failure should surface the real
        // cause, not an unrelated file-read error.
        let decoded_audio = std::fs::read(&request.audio_path).map_err(|error| {
            ProviderError::Engine(format!(
                "could not read decoded audio {}: {error}",
                request.audio_path.display()
            ))
        })?;
        let sent = sent_request
            .into_inner()
            .expect("provenance request lock")
            .unwrap_or(Value::Null);
        let provenance = vad::build_provenance(
            &sent,
            payload,
            &decoded_audio,
            threshold,
            signal.sample_rate,
        );
        Ok((signal, provenance))
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
/// CutRight does not bundle a Silero model or worker and does not reference
/// any HeardRight-internal model path. It invokes the HeardRight engine (the
/// same supervised JSON-line session used for transcription) and lets
/// HeardRight resolve its own model and runtime.
///
/// # HeardRight location
/// The engine is resolved via `CUTRIGHT_HEARDRIGHT_ENGINE` (or
/// `HEARDRIGHT_ENGINE_BIN`), then an installed platform location, then
/// `heardright-engine` on `PATH` (§9.3). There is no hard-coded absolute
/// default. If HeardRight is unavailable the call fails with a clear
/// [`ProviderError`]; there is never a silent fallback to a bundled model.
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
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
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

    /// A fake engine that answers the handshake and then, for every
    /// subsequent request, prints `response_template` with the caller's
    /// `request_id` substituted for a single `%s` placeholder in the
    /// template, so correlation succeeds by default.
    fn write_fake_engine(root: &Path, response_template: &str) -> PathBuf {
        let engine = root.join("fake-engine");
        fs::write(
            &engine,
            format!(
                "#!/bin/sh\nwhile IFS= read -r line; do\n  rid=$(printf '%s' \"$line\" | sed -n 's/.*\"request_id\":\"\\([^\"]*\\)\".*/\\1/p')\n  case \"$line\" in\n    *session_handshake_request*) printf '{{\"schema_name\":\"session_handshake_result\",\"protocol_major\":1,\"protocol_minor\":0,\"engine_version\":\"fake-engine/1.0\",\"request_id\":\"%s\",\"payload\":{{\"capabilities\":[\"file_transcription_v1\",\"file_vad_regions_v1\"]}}}}\\n' \"$rid\" ;;\n    *) printf '{response_template}\\n' \"$rid\" ;;\n  esac\ndone\n"
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
                "#!/bin/sh\nprintf 'started\\n' >> '{}'\nwhile IFS= read -r line; do\n  rid=$(printf '%s' \"$line\" | sed -n 's/.*\"request_id\":\"\\([^\"]*\\)\".*/\\1/p')\n  case \"$line\" in\n    *session_handshake_request*) printf '{{\"schema_name\":\"session_handshake_result\",\"protocol_major\":1,\"protocol_minor\":0,\"engine_version\":\"fake-engine/1.0\",\"request_id\":\"%s\",\"payload\":{{}}}}\\n' \"$rid\" ;;\n    *) printf '{{\"schema_name\":\"file_transcription_result\",\"request_id\":\"%s\",\"payload\":{{\"text\":\"hello\",\"words\":[{{\"text\":\"hello\",\"start_ms\":0,\"end_ms\":100}}]}}}}\\n' \"$rid\" ;;\n  esac\ndone\n",
                starts.display()
            ),
        )
        .expect("write fake engine");
        fs::set_permissions(&engine, fs::Permissions::from_mode(0o700))
            .expect("make fake engine executable");
        let provider = HeardRightProvider::with_engine(engine);
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
    fn parse_vad_result_accepts_an_empty_region_list() {
        let payload = serde_json::json!({ "sample_rate": 16_000, "regions": [] });
        let signal = vad::parse_vad_result("silent", &payload).expect("parse empty regions");
        assert!(signal.regions.is_empty());
        assert_eq!(signal.provider, "heardright-silero");
    }

    #[test]
    fn parse_vad_result_rejects_missing_fields() {
        let no_regions = serde_json::json!({ "sample_rate": 16_000 });
        assert!(vad::parse_vad_result("source-a", &no_regions).is_err());
        let no_sample_rate = serde_json::json!({ "regions": [] });
        assert!(vad::parse_vad_result("source-a", &no_sample_rate).is_err());
        let bad_region = serde_json::json!({
            "sample_rate": 16_000,
            "regions": [{ "start_ms": 100 }],
        });
        assert!(vad::parse_vad_result("source-a", &bad_region).is_err());
    }

    #[test]
    fn parse_vad_result_rejects_an_inverted_region() {
        let payload = serde_json::json!({
            "sample_rate": 16_000,
            "regions": [{ "start_ms": 900, "end_ms": 100, "mean_probability": 0.5 }],
        });
        let error =
            vad::parse_vad_result("source-a", &payload).expect_err("inverted region rejected");
        assert!(matches!(error, ProviderError::Engine(_)));
    }

    #[test]
    fn analyzes_file_vad_through_a_fake_engine_and_returns_provenance() {
        let root = unique_temp_dir("cutright-vad-test");
        let audio_path = root.join("source-a-16k.f32");
        fs::write(&audio_path, [0u8; 32]).expect("write fake decoded audio");
        let engine = write_fake_engine(
            &root,
            "{\"schema_name\":\"file_vad_result\",\"request_id\":\"%s\",\"payload\":{\"sample_rate\":16000,\"provider\":\"heardright-silero\",\"model_revision\":\"silero-v6\",\"runtime\":\"coreml\",\"threshold\":0.5,\"min_speech_ms\":160,\"min_silence_ms\":180,\"regions\":[{\"start_ms\":100,\"end_ms\":900,\"mean_probability\":0.9}]}}",
        );
        let provider = HeardRightProvider::with_engine(engine);
        let (signal, provenance) = provider
            .analyze_file_vad_with_provenance(&VadRequest {
                source_id: "source-a".into(),
                audio_path,
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
        assert_eq!(provenance.model_revision, "silero-v6");
        assert_eq!(provenance.runtime_backend, "coreml");
        assert_eq!(provenance.sample_rate, 16_000);
        assert!((provenance.threshold - 0.5).abs() < 1e-6);
        assert!(!provenance.decoded_audio_blake3.is_empty());
        assert!(!provenance.request_blake3.is_empty());
        drop(provider);
        fs::remove_dir_all(root).expect("remove vad test directory");
    }

    #[test]
    fn file_vad_surfaces_an_engine_error_clearly() {
        let root = unique_temp_dir("cutright-vad-error-test");
        let audio_path = root.join("source-a-16k.f32");
        fs::write(&audio_path, [0u8; 8]).expect("write fake decoded audio");
        let engine = write_fake_engine(
            &root,
            "{\"schema_name\":\"engine_error\",\"request_id\":\"%s\",\"error\":{\"message\":\"file_vad_regions_v1 not supported\"}}",
        );
        let provider = HeardRightProvider::with_engine(engine);
        let error = provider
            .analyze_file_vad(&VadRequest {
                source_id: "source-a".into(),
                audio_path,
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
        let provider =
            HeardRightProvider::with_engine(root.join("does-not-exist/heardright-engine"));
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
