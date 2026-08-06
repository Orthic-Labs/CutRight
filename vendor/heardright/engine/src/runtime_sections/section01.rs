// Engine runtime — owns the dictation state machine and the recent
// transcript ring buffer.
//
// The IPC layer (`ipc.rs`) drives this runtime from the shell. The runtime
// is transport-agnostic so the same code can be used by:
//  - this sidecar binary (driven over stdio JSON-RPC)
//  - the Tauri shell (when `HEARDRIGHT_NEXT_ENGINE=sidecar`, a separate
//    supervisor in the shell opens stdio to the sidecar and uses the
//    same `EngineRuntime` shape via the contract)
//
// Audio capture and ASR live in `asr.rs`; this runtime owns the sidecar state
// machine, focus-target tracking, delivery precedence, command dispatch, and
// recent transcript ring.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::asr::AsrEp;
use crate::delivery::{
    copy_fallback_record, copy_text, deliver_text, restore_and_verify, snapshot_target,
    DeliveryOutcome, DeliveryRecord, TargetSnapshot,
};
use crate::focus::FocusTracker;
use crate::owner_diagnostics;
use crate::worker::{spawn_worker, WorkerCmd, WorkerEvent, WorkerHandle};
use heardright_core::delivery::CopyFallbackReason;
use heardright_core::engine::FileTranscript;

const RECENT_HISTORY_CAP: usize = 100;
const TERMINAL_TOMBSTONE_CAP: usize = 64;

fn engine_test_mode() -> bool {
    std::env::var("HEARDRIGHT_ENGINE_TEST_MODE")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Engine info surfaced over the IPC contract as the `EngineInfo` payload.
#[derive(Debug, Clone)]
pub struct EngineInfo {
    pub mode: String,
    pub cold_load_s: Option<f64>,
    pub engine_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineState {
    Idle,
    Recording { session_id: String },
    Transcribing { session_id: String },
    Error { session_id: String, message: String },
}

pub struct EngineRuntime {
    state: EngineState,
    last_delivery: Option<DeliveryRecord>,
    recent: VecDeque<DeliveryRecord>,
    focus: Arc<Mutex<FocusTracker>>,
    sequence: u64,
    terminal_tombstones: VecDeque<String>,
    models_base: PathBuf,
    worker: Option<WorkerHandle>,
    pending_send_enter: bool,
    /// Mirrors `pending_send_enter`: set by `begin_stop` only when the stop
    /// is a `StopKind::CancelToHistory` finalize (engine-side name for the
    /// shell's cancel-to-history flow). Consumed (taken) in
    /// `finalize_phase1_capture` and threaded through to phase 2, which
    /// skips the cloud L1/L2/L3 polish lanes entirely for this stop — the
    /// finalize still runs ASR and local (L0) polish to completion exactly
    /// like a normal Stop.
    pending_local_only: bool,
    stop_started_at: Option<Instant>,
}

#[derive(Debug, Clone)]
pub enum FinalizeOutcome {
    Delivery {
        record: DeliveryRecord,
        send_enter: bool,
    },
    Transcript {
        text: String,
        send_enter: bool,
        raw_text: Option<String>,
        recording_ms: Option<u64>,
    },
    ResetToArmed,
    CommandDispatched {
        action: String,
        detail: String,
    },
    /// A recognized standalone command failed to dispatch (missing macOS
    /// Accessibility grant, no platform equivalent, unknown key). The engine
    /// still recovers to armed; the message lets the shell surface the failure
    /// instead of it looking identical to success (release audit P0-2).
    CommandFailed {
        message: String,
    },
    NoOp,
}
