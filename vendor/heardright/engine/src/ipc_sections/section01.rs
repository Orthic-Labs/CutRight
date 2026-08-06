// JSON-RPC over stdio transport for the `heardright-engine` sidecar.
//
// One `EngineFrame` per line on stdin (shell -> engine) and stdout (engine ->
// shell). stderr is reserved for tracing; the shell ignores it. The transport
// is line-delimited so a single corrupted frame never desyncs the stream and
// the channel is trivially recoverable (drop the line, log, continue).
//
// The contract reuses `heardright_core::engine::EngineFrame` so the schema
// and validation live in one place.

use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use heardright_core::engine::{
    DiagnosticsPayload, EngineErrorPayload, EngineFrame, EnginePayload, EngineSchemaName, StopKind,
    CURRENT_SCHEMA_VERSION, PROTOCOL_MAJOR, PROTOCOL_MINOR,
};
use heardright_core::settings::SettingsBlob;
use parking_lot::Mutex;
use serde_json::{json, Value};

use crate::delivery::DeliveryRecord;
use crate::runtime::{EngineRuntime, FinalizeOutcome};

/// Marker engine version baked into every frame we emit.
#[cfg(target_os = "macos")]
const ENGINE_VERSION: &str = concat!(
    "heardright-engine/",
    env!("CARGO_PKG_VERSION"),
    "+parakeet-unified-coreml"
);
#[cfg(not(target_os = "macos"))]
const ENGINE_VERSION: &str = concat!(
    "heardright-engine/",
    env!("CARGO_PKG_VERSION"),
    "+parakeet-unified"
);

/// Run the sidecar's read loop. Blocks until EOF on stdin (shell quit) or
/// a `Request::Shutdown`.
pub fn run(runtime: Arc<Mutex<EngineRuntime>>) -> anyhow::Result<()> {
    let writer = Arc::new(Mutex::new(io::stdout()));
    let worker_pump_started = Arc::new(AtomicBool::new(false));
    // Do not announce readiness until the resident worker has loaded the ASR
    // model. The shell's prewarm/onboarding path waits for this first frame, so
    // emitting it before `ensure_worker()` would make the UI claim the engine is
    // warm while the first real recording still pays model-load cost.
    {
        let mut guard = runtime.lock();
        guard
            .ensure_worker()
            .map_err(|err| anyhow::anyhow!("worker warmup failed before ready: {err}"))?;
    }
    ensure_worker_event_pump(&runtime, &writer, &worker_pump_started)
        .map_err(|err| anyhow::anyhow!("worker event pump failed before ready: {err}"))?;
    // Emit a single ready frame so the shell can synchronize on the sidecar
    // being alive, protocol-compatible, and model-hot before it sends work.
    emit_event(
        &writer,
        EngineSchemaName::EngineInfo,
        None,
        Some(EnginePayload::EngineInfo {
            mode: "sidecar".into(),
            cold_load_s: None,
            engine_version: Some(ENGINE_VERSION.to_string()),
        }),
    );
    tracing::info!(
        "heardright-engine ready: protocol {}.{}, schema {}",
        PROTOCOL_MAJOR,
        PROTOCOL_MINOR,
        CURRENT_SCHEMA_VERSION
    );

    let stdin = io::stdin();
    let mut reader = stdin.lock();

    let mut buf = String::new();
    loop {
        buf.clear();
        let n = reader.read_line(&mut buf)?;
        if n == 0 {
            tracing::info!("stdin closed; exiting");
            return Ok(());
        }
        let trimmed = buf.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(error = %err, "dropping malformed frame");
                continue;
            }
        };
        let frame = match heardright_core::engine::validate_engine_frame(&value) {
            Ok(f) => f,
            Err(err) => {
                tracing::warn!(error = %err, "dropping invalid frame");
                continue;
            }
        };
        let request_id = frame.request_id.clone();
        let Some(req) = decode_request(frame) else {
            continue;
        };
        if matches!(req, Request::Shutdown) {
            tracing::info!("shutdown request received");
            return Ok(());
        }
        // Handle each request on its own thread so a slow op never blocks the
        // stdin read loop. Combined with the lock-narrowed handlers (which release
        // the runtime mutex across worker round-trips), concurrent requests no
        // longer serialize. The shell matches responses by request_id, so
        // out-of-order completion is fine. stdout uses a short-lived lock per
        // write (shared with the worker event pump) — never held across I/O.
        let req_runtime = runtime.clone();
        let req_writer = writer.clone();
        let req_pump = worker_pump_started.clone();
        std::thread::spawn(move || {
            let response = handle(&req_runtime, &req_writer, &req_pump, &request_id, req);
            if let Ok(line) = serde_json::to_string(&response) {
                let mut stdout = req_writer.lock();
                let _ = stdout.write_all(line.as_bytes());
                let _ = stdout.write_all(b"\n");
                let _ = stdout.flush();
            }
        });
    }
}

#[derive(Debug)]
pub enum Request {
    Health,
    Capabilities,
    Info,
    ReplaceEngineConfig {
        config: SettingsBlob,
    },
    ReplaceVocabulary {
        terms: Vec<String>,
        /// Parallel optional list carrying sound-alike aliases per term. Old
        /// shells that send no `term_details` fall back to the bare `terms`
        /// list (no aliases).
        term_details: Option<Vec<heardright_core::engine::VocabTermWire>>,
    },
    StartDictation {
        session_id: String,
    },
    TranscribeFile {
        path: std::path::PathBuf,
    },
    StopDictation {
        session_id: String,
        send_enter: bool,
        /// `true` only for a `StopKind::CancelToHistory` finalize (the
        /// shell's cancel-to-history flow): the sidecar still runs ASR to
        /// completion and returns the transcript, but skips the cloud
        /// L1/L2/L3 AI polish lanes entirely — see
        /// `runtime_sections/finalize_transcript.rs`. `false` for every
        /// other `StopDictation` request (Stop/SendEnter and the legacy
        /// request_id-suffix fallback), which must behave exactly as
        /// before this field was added.
        local_only: bool,
    },
    CancelDictation {
        session_id: String,
    },
    RepasteLast,
    CopyLast,
    GetState,
    GetRecentHistory {
        limit: usize,
    },
    ReplaceRecentHistory {
        records: Vec<DeliveryRecord>,
    },
    StartWakeListen {
        model: Option<String>,
        threshold: Option<f32>,
    },
    StopWakeListen,
    Shutdown,
}

fn decode_request(frame: EngineFrame) -> Option<Request> {
    use EngineSchemaName::*;
    let payload = frame.payload?;
    let session_id = frame.session_id.clone();
    let request_id = frame.request_id.clone();
    match (frame.schema_name, payload) {
        (EngineHealth, EnginePayload::Health { .. }) => Some(Request::Health),
        (EngineCapabilities, EnginePayload::Capabilities { .. }) => Some(Request::Capabilities),
        (EngineInfo, EnginePayload::EngineInfo { .. }) => Some(Request::Info),
        (ReplaceEngineConfig, EnginePayload::ReplaceEngineConfig { config }) => {
            serde_json::from_value::<SettingsBlob>(config)
                .ok()
                .map(|config| Request::ReplaceEngineConfig { config })
        }
        (
            ReplaceVocabulary,
            EnginePayload::ReplaceVocabulary {
                terms,
                term_details,
            },
        ) => Some(Request::ReplaceVocabulary {
            terms,
            term_details,
        }),
        (EngineStateRequest, EnginePayload::EngineStateRequest) => Some(Request::GetState),
        (RepasteLastRequest, EnginePayload::RepasteLastRequest) => Some(Request::RepasteLast),
        (CopyLastRequest, EnginePayload::CopyLastRequest) => Some(Request::CopyLast),
        (RecentHistoryRequest, EnginePayload::RecentHistoryRequest { limit }) => {
            Some(Request::GetRecentHistory { limit })
        }
        (ReplaceRecentHistory, EnginePayload::ReplaceRecentHistory { records }) => {
            let records =
                serde_json::from_value::<Vec<DeliveryRecord>>(records).unwrap_or_default();
            Some(Request::ReplaceRecentHistory { records })
        }
        (
            WakeListenStarted,
            EnginePayload::WakeListenStarted {
                model, threshold, ..
            },
        ) => Some(Request::StartWakeListen { model, threshold }),
        (WakeListenStopped, EnginePayload::WakeListenStopped { .. }) => {
            Some(Request::StopWakeListen)
        }
        (RecordingStarted, EnginePayload::RecordingStarted { session_id }) => {
            Some(Request::StartDictation { session_id })
        }
        (FileTranscriptionRequest, EnginePayload::FileTranscriptionRequest { path }) => {
            Some(Request::TranscribeFile {
                path: std::path::PathBuf::from(path),
            })
        }
        (RecordingLevel, EnginePayload::RecordingLevel { .. }) => None,
        (TranscribingStarted, EnginePayload::TranscribingStarted { stop_kind }) => {
            let id = session_id?;
            // Prefer the typed `stop_kind` (protocol v2) when the sender set
            // it; fall back to the legacy request_id suffix convention for
            // frames from an older shell build that doesn't send it yet.
            // Both paths are kept live during the transition — see the
            // module doc comment and dispatch #9/#10.
            match stop_kind {
                Some(StopKind::Cancel) => {
                    return Some(Request::CancelDictation { session_id: id });
                }
                Some(StopKind::SendEnter) => {
                    return Some(Request::StopDictation {
                        session_id: id,
                        send_enter: true,
                        local_only: false,
                    });
                }
                Some(StopKind::Stop) => {
                    return Some(Request::StopDictation {
                        session_id: id,
                        send_enter: false,
                        local_only: false,
                    });
                }
                Some(StopKind::CancelToHistory) => {
                    return Some(Request::StopDictation {
                        session_id: id,
                        send_enter: false,
                        local_only: true,
                    });
                }
                None => {}
            }
            if request_id.contains(":cancel") {
                return Some(Request::CancelDictation { session_id: id });
            }
            let send_enter = request_id.contains(":send_enter");
            Some(Request::StopDictation {
                session_id: id,
                send_enter,
                local_only: false,
            })
        }
        (TranscriptFinal, EnginePayload::TranscriptFinal { .. }) => None,
        (EngineError, _) => None,
        _ => None,
    }
}

fn cancel_source_from_request_id(request_id: &str) -> &'static str {
    match request_id.rsplit(':').next() {
        Some("webview_command") => "webview_command",
        Some("fn_escape") => "fn_escape",
        Some("pill_cancel_button") => "pill_cancel_button",
        Some("ptt_chord_abort") => "ptt_chord_abort",
        Some("keyboard_cancel_hook") => "keyboard_cancel_hook",
        _ => "unknown",
    }
}

#[cfg(test)]
mod cancel_source_tests {
    use super::cancel_source_from_request_id;

    #[test]
    fn cancel_source_accepts_only_known_content_free_suffixes() {
        assert_eq!(
            cancel_source_from_request_id("cancel:req-9:cancel:fn_escape"),
            "fn_escape"
        );
        assert_eq!(
            cancel_source_from_request_id("cancel:req-9:cancel:pill_cancel_button"),
            "pill_cancel_button"
        );
        assert_eq!(
            cancel_source_from_request_id("cancel:req-9:cancel:user dictated words"),
            "unknown"
        );
        assert_eq!(
            cancel_source_from_request_id("cancel:req-9:cancel"),
            "unknown"
        );
    }
}
