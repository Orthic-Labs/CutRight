// Engine IPC protocol — frame schema, payloads, and contract validation.
// Pure serde + validation; the process spawning / stdin-stdout machinery
// lives in `src-tauri/src/engine_process.rs`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parallel wire shape for vocabulary entries carrying sound-alike aliases.
/// `term_details` rides beside `terms` in `ReplaceVocabulary` so an old engine
/// without this field falls back gracefully to the bare `terms` list (no
/// aliases); a new engine that receives no `term_details` also falls back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VocabTermWire {
    pub term: String,
    #[serde(default)]
    pub sounds_like: Vec<String>,
}

pub const PROTOCOL_MAJOR: u16 = 1;
pub const PROTOCOL_MINOR: u16 = 0;
pub const CURRENT_SCHEMA_VERSION: u16 = 1;
const FAKE_ENGINE_VERSION: &str = "fake-engine/0.1.0";

/// Max audio length (seconds) the free tier may transcribe from a file. Pro is
/// unlimited (incl. the 14-day trial) — there is no hard ceiling above this.
pub const FREE_TRANSCRIBE_MAX_SECS: u32 = 600; // 10 minutes

/// Gate a file's duration against tier limits. Pure — no I/O. `secs` is the
/// decoded (or probed) audio length in seconds; `is_pro` is the entitlement.
/// Free is capped at FREE_TRANSCRIBE_MAX_SECS; Pro has no length limit.
pub fn check_duration_limit(secs: u32, is_pro: bool) -> Result<(), String> {
    if !is_pro && secs > FREE_TRANSCRIBE_MAX_SECS {
        return Err(format!(
            "Free transcription is limited to {} minutes per file. Upgrade to Pro for unlimited file length.",
            FREE_TRANSCRIBE_MAX_SECS / 60
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineSchemaName {
    EngineHealth,
    EngineCapabilities,
    EngineInfo,
    EngineAck,
    ReplaceEngineConfig,
    ReplaceVocabulary,
    EngineStateRequest,
    EngineStateSnapshot,
    RecentHistoryRequest,
    RecentHistoryResult,
    ReplaceRecentHistory,
    RepasteLastRequest,
    ManualDeliveryResult,
    CopyLastRequest,
    CopyLastResult,
    FileTranscriptionRequest,
    FileTranscriptionResult,
    RecordingStarted,
    RecordingLevel,
    TranscribingStarted,
    TranscriptPartial,
    TranscriptFinal,
    EngineError,
    // Phase A2 wake-word listener (always-listening mode)
    WakeListenStarted,
    WakeListenStopped,
    WakeEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineHealthStatus {
    Ok,
    Degraded,
    Unavailable,
}

/// Typed replacement for the legacy `request_id` suffix convention
/// (`":cancel"` / `":send_enter"`). Protocol v2 additive field — carried on
/// `TranscribingStarted` requests so the IPC decoder no longer has to parse
/// intent out of a string. Optional + `skip_serializing_if` so OLD frames
/// (no `stop_kind`) still deserialize on both old and new binaries; the
/// legacy suffix is still written/read in parallel during the transition
/// (see `heardright-engine/src/ipc_sections/section01.rs::decode_request`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopKind {
    /// Plain stop — finalize without an Enter keystroke.
    Stop,
    /// Stop and deliver with a trailing Enter keystroke.
    SendEnter,
    /// Abandon the in-flight recording/transcription; never deliver.
    Cancel,
    /// A cancel (any source) that still finalizes the in-flight recording —
    /// unlike `Cancel`, the engine runs ASR to completion — but the shell
    /// stores the result to encrypted history as `Cancelled` instead of
    /// delivering it. No clipboard write, no paste, no Enter. The shell
    /// prefers the raw/L0 transcript for the stored record, so the engine
    /// should skip AI polish (L1/L2/L3) for this stop kind where practical.
    CancelToHistory,
}

/// Typed replacement for the stringly-keyed `TranscriptFinal.diagnostics`
/// JSON map (`"delivery_record"`, `"shell_delivery"`, `"send_enter"`,
/// `"raw_text"`, `"recording_ms"`, `"reset_to_armed"`, `"command_dispatched"`,
/// `"command_detail"`, `"command_failed"`).
/// Protocol v2 additive: the wire field stays `Option<serde_json::Value>` (no
/// schema-shape break); senders now build this struct and serialize it into
/// that slot, and readers deserialize it FIRST, falling back to the old
/// string-keyed extraction for frames from an older engine build that hasn't
/// switched over yet. All fields optional/`skip_serializing_if` so old and
/// new frames both round-trip through `Option<Value>` unchanged. See
/// dispatch #9/#10.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RecordingStageMetrics {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vad_observed_frame_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vad_speech_frame_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asr_elapsed_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polish_elapsed_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticsPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_record: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell_delivery: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_enter: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recording_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset_to_armed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_dispatched: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_failed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recording_stage_metrics: Option<RecordingStageMetrics>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineErrorPayload {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub retryable: bool,
    #[serde(default)]
    pub diagnostics: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EnginePayload {
    Health {
        status: EngineHealthStatus,
        #[serde(default)]
        diagnostics: Option<Value>,
    },
    Capabilities {
        capabilities: Vec<String>,
    },
    EngineInfo {
        mode: String,
        #[serde(default)]
        cold_load_s: Option<f64>,
        #[serde(default)]
        engine_version: Option<String>,
    },
    EngineAck {
        status: String,
        #[serde(default)]
        detail: Option<String>,
    },
    ReplaceEngineConfig {
        config: Value,
    },
    ReplaceVocabulary {
        terms: Vec<String>,
        /// Parallel mirror carrying `sounds_like` aliases for terms that have
        /// them. Backward-compatible: an old shell that sends no
        /// `term_details` falls back to the bare `terms` list (no aliases);
        /// an old engine that ignores the unknown field still works from
        /// `terms`. New code prefers `term_details` when present.
        #[serde(default)]
        term_details: Option<Vec<VocabTermWire>>,
    },
    EngineStateRequest,
    EngineStateSnapshot {
        state: String,
    },
    RecentHistoryRequest {
        limit: usize,
    },
    RecentHistoryResult {
        records: Value,
    },
    ReplaceRecentHistory {
        records: Value,
    },
    RepasteLastRequest,
    ManualDeliveryResult {
        record: Value,
    },
    CopyLastRequest,
    CopyLastResult {
        text: String,
    },
    FileTranscriptionRequest {
        path: String,
    },
    FileTranscriptionResult {
        text: String,
        srt: String,
        vtt: String,
        #[serde(default)]
        words: Vec<TimedWord>,
    },
    RecordingStarted {
        session_id: String,
    },
    RecordingLevel {
        level: f32,
    },
    TranscribingStarted {
        /// Protocol v2 additive field (see `StopKind` doc comment). Absent on
        /// legacy frames and on the fresh-recording `TranscribingStarted`
        /// notification the worker pump emits (no stop intent there).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stop_kind: Option<StopKind>,
    },
    TranscriptPartial {
        text: String,
        revision: u64,
        #[serde(default)]
        audio_secs: Option<f32>,
        #[serde(default)]
        diagnostics: Option<Value>,
    },
    TranscriptFinal {
        text: String,
        #[serde(default)]
        confidence: Option<f32>,
        #[serde(default)]
        diagnostics: Option<Value>,
    },
    WakeListenStarted {
        status: String,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        threshold: Option<f32>,
        #[serde(default)]
        fake: Option<bool>,
    },
    WakeListenStopped {
        status: String,
        #[serde(default)]
        total_fires: Option<u64>,
    },
    WakeFired {
        ts: f64,
        #[serde(default)]
        fire_count: Option<u64>,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        threshold: Option<f32>,
        /// `softmax[1]` of the firing window (fable audit #5 — propagate score so
        /// the UI/telemetry can show confidence). `None` until the Phase F
        /// scorer is wired (see `heardright_core::wake`).
        #[serde(default)]
        score: Option<f32>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineFrame {
    pub protocol_major: u16,
    pub protocol_minor: u16,
    pub schema_name: EngineSchemaName,
    pub schema_version: u16,
    pub engine_version: String,
    pub request_id: String,
    #[serde(default)]
    pub session_id: Option<String>,
    pub trace_id: String,
    #[serde(default)]
    pub payload: Option<EnginePayload>,
    #[serde(default)]
    pub error: Option<EngineErrorPayload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimedWord {
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileTranscript {
    pub text: String,
    pub srt: String,
    pub vtt: String,
    #[serde(default)]
    pub words: Vec<TimedWord>,
}

impl EngineFrame {
    pub fn health(request_id: &str, trace_id: &str) -> Self {
        Self::base(
            EngineSchemaName::EngineHealth,
            request_id,
            None,
            trace_id,
            Some(EnginePayload::Health {
                status: EngineHealthStatus::Ok,
                diagnostics: None,
            }),
            None,
        )
    }

    /// Construct an `EngineFrame` from its wire components. Public so the
    /// sidecar binary and shell supervisor can both build frames without a
    /// parallel constructor.
    pub fn base(
        schema_name: EngineSchemaName,
        request_id: &str,
        session_id: Option<String>,
        trace_id: &str,
        payload: Option<EnginePayload>,
        error: Option<EngineErrorPayload>,
    ) -> Self {
        Self {
            protocol_major: PROTOCOL_MAJOR,
            protocol_minor: PROTOCOL_MINOR,
            schema_name,
            schema_version: CURRENT_SCHEMA_VERSION,
            engine_version: FAKE_ENGINE_VERSION.to_string(),
            request_id: request_id.to_string(),
            session_id,
            trace_id: trace_id.to_string(),
            payload,
            error,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineContractError {
    ProtocolMajorMismatch { expected: u16, actual: u16 },
    UnsupportedSchemaVersion { actual: u16 },
    MissingPayload,
    MissingError,
    MissingSessionId,
    UnexpectedErrorPayload,
    PayloadSchemaMismatch,
    UiConceptLeaked(String),
}

impl std::fmt::Display for EngineContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProtocolMajorMismatch { expected, actual } => {
                write!(
                    f,
                    "protocol_major mismatch: expected {expected}, got {actual}"
                )
            }
            Self::UnsupportedSchemaVersion { actual } => {
                write!(f, "unsupported schema_version: {actual}")
            }
            Self::MissingPayload => write!(f, "engine frame missing payload"),
            Self::MissingError => write!(f, "engine error frame missing error"),
            Self::MissingSessionId => write!(f, "session-scoped frame missing session_id"),
            Self::UnexpectedErrorPayload => write!(f, "normal frame carried error payload"),
            Self::PayloadSchemaMismatch => write!(f, "schema_name does not match payload"),
            Self::UiConceptLeaked(field) => write!(f, "engine leaked Rust-owned field {field}"),
        }
    }
}

impl std::error::Error for EngineContractError {}

pub fn validate_engine_frame(value: &Value) -> Result<EngineFrame, EngineContractError> {
    reject_ui_concepts(value)?;
    // Deserialize from the existing `&Value` instead of `value.clone()` +
    // `serde_json::from_value` — `&serde_json::Value` implements `Deserializer`
    // directly, so this walks the same tree once instead of deep-cloning the
    // whole frame (strings, nested objects, everything) before reading it. Every
    // IPC frame pays this, including 10Hz RecordingLevel/partial traffic for the
    // whole recording. Same validation, same error on the same malformed input —
    // `from_value`'s error is discarded into `PayloadSchemaMismatch` either way.
    let frame: EngineFrame = EngineFrame::deserialize(value)
        .map_err(|_| EngineContractError::PayloadSchemaMismatch)?;
    validate_frame(&frame)?;
    Ok(frame)
}
