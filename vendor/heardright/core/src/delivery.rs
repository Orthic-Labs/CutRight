//! Delivery — pure record shaping (no clipboard / window / keystroke calls).
//!
//! The OS side effects (clipboard write, Ctrl+V / Enter SendInput, foreground
//! window snapshot, `RealDeliveryBackend`) live in `src-tauri/src/delivery.rs`
//! and build these shapes. Time/pid helpers use std only (no GUI), so they're
//! fine here.

use serde::{Deserialize, Serialize};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

// Every delivery path (real backend, fallback, file transcription) shares this
// sequence. Per-backend counters restart whenever a backend is rebuilt, which
// happens for every sidecar transcript and could collide within one millisecond.
static NEXT_DELIVERY_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeliveryId(String);

impl DeliveryId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyFallbackReason {
    ClipboardUnavailable,
    ElevatedTarget,
    FocusChanged,
    PasteFailed,
    EmptyTranscript,
    /// The focused target was not a text input, so there was nowhere to paste —
    /// the transcript is copied and offered in the recent-transcripts popup.
    NoTextField,
    UnsupportedPlatform,
    Other(String),
}

impl std::fmt::Display for CopyFallbackReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClipboardUnavailable => write!(f, "clipboard_unavailable"),
            Self::ElevatedTarget => write!(f, "elevated_target"),
            Self::FocusChanged => write!(f, "focus_changed"),
            Self::PasteFailed => write!(f, "paste_failed"),
            Self::EmptyTranscript => write!(f, "empty_transcript"),
            Self::NoTextField => write!(f, "no_text_field"),
            Self::UnsupportedPlatform => write!(f, "unsupported_platform"),
            Self::Other(reason) => write!(f, "{reason}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeliveryOutcome {
    Pasted,
    CopiedFallback {
        reason: CopyFallbackReason,
    },
    Error {
        code: String,
        message: String,
    },
    /// A cancel (any source) that finalized to encrypted history instead of an
    /// OS delivery — no clipboard write, no paste, no Enter. `DeliveryRecord`
    /// carries the raw/L0 transcript so the history filter can hide it by
    /// default (see `heardright_core::history::HistoryFilter::cancelled_only`).
    Cancelled,
    /// An in-progress snapshot of the committed transcript, written during
    /// recording from `EngineEvent::TranscriptPartial` so a crash mid-
    /// dictation still has something recoverable (crash-recovery draft
    /// history rows). Never a terminal, user-visible outcome: a normal
    /// delivery or an explicit cancel supersedes/removes the draft row for
    /// its session before writing its own terminal record (see
    /// `EngineRuntime::accept_sidecar_delivery` / `accept_cancel_to_history`
    /// in the shell crate). Any `Draft` still present at the next launch
    /// means the process died mid-recording — the startup sweep
    /// (`recover_orphaned_drafts`) flips it to `Cancelled`. Hidden from the
    /// default history view exactly like `Cancelled` (see
    /// `HistoryFilter::cancelled_only`).
    Draft,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMethod {
    ClipboardThenPaste,
    ClipboardOnly,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryTimingStep {
    pub name: String,
    pub elapsed_ms: u64,
}

impl DeliveryTimingStep {
    pub fn new(name: impl Into<String>, elapsed_ms: u64) -> Self {
        Self {
            name: name.into(),
            elapsed_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryTimings {
    pub total_ms: u64,
    pub steps: Vec<DeliveryTimingStep>,
}

impl DeliveryTimings {
    pub fn new(total_ms: u64) -> Self {
        Self {
            total_ms,
            steps: Vec::new(),
        }
    }

    pub fn from_steps(total_ms: u64, steps: Vec<DeliveryTimingStep>) -> Self {
        Self { total_ms, steps }
    }

    pub fn with_step(mut self, name: impl Into<String>, elapsed_ms: u64) -> Self {
        self.steps.push(DeliveryTimingStep::new(name, elapsed_ms));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetSnapshot {
    pub process_id: Option<u32>,
    pub process_name: Option<String>,
    pub window_title: Option<String>,
    pub window_handle: Option<isize>,
    #[serde(default)]
    pub focused_control_handle: Option<isize>,
    #[serde(default)]
    pub foreground_target: Option<ForegroundTarget>,
    #[serde(default)]
    pub focused_text_input: Option<bool>,
    pub is_elevated: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ForegroundTarget {
    WindowHandle { handle: isize },
    ProcessId { pid: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopOrigin {
    Generic,
    Pill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetDeliveryRoute {
    ExternalLive,
    RestoredPill,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingStopContext {
    pub session_id: String,
    pub origin: StopOrigin,
    pub overlay_handles: Vec<isize>,
}

impl PendingStopContext {
    pub fn new(
        session_id: impl Into<String>,
        origin: StopOrigin,
        overlay_handles: Vec<isize>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            origin,
            overlay_handles,
        }
    }

    pub fn applies_to_overlay(&self, session_id: &str, target: &TargetSnapshot) -> bool {
        self.session_id == session_id
            && self.origin == StopOrigin::Pill
            && target
                .window_handle
                .is_some_and(|handle| self.overlay_handles.contains(&handle))
    }
}

#[derive(Debug, Default)]
pub struct PendingStopContextLatch {
    context: Option<PendingStopContext>,
}

impl PendingStopContextLatch {
    pub fn latch_first(&mut self, context: PendingStopContext) -> bool {
        if self.context.is_some() {
            return false;
        }
        self.context = Some(context);
        true
    }

    pub fn as_ref(&self) -> Option<&PendingStopContext> {
        self.context.as_ref()
    }

    pub fn clear(&mut self) {
        self.context = None;
    }
}

impl TargetSnapshot {
    /// All-`None` snapshot for records that never touched a delivery target —
    /// currently only draft rows (`DeliveryRecord::new_draft`), which exist
    /// purely so a crash mid-recording still leaves something recoverable in
    /// history.
    pub fn empty() -> Self {
        Self {
            process_id: None,
            process_name: None,
            window_title: None,
            window_handle: None,
            focused_control_handle: None,
            foreground_target: None,
            focused_text_input: None,
            is_elevated: None,
        }
    }

    pub fn test_target() -> Self {
        Self {
            process_id: Some(1),
            process_name: Some("test-target".to_string()),
            window_title: Some("Test Target".to_string()),
            window_handle: Some(1),
            focused_control_handle: Some(2),
            foreground_target: Some(ForegroundTarget::WindowHandle { handle: 1 }),
            focused_text_input: Some(true),
            is_elevated: Some(false),
        }
    }

    /// Conservative identity proof for irreversible delivery. A field handle,
    /// when both observations expose one, is stronger than app/window identity.
    /// Missing/unknown identity never proves that two targets are identical.
    pub fn same_stable_target(&self, other: &Self) -> bool {
        match (self.focused_control_handle, other.focused_control_handle) {
            (Some(left), Some(right)) => return left == right,
            (Some(_), None) | (None, Some(_)) => return false,
            (None, None) => {}
        }
        match (&self.foreground_target, &other.foreground_target) {
            (
                Some(ForegroundTarget::WindowHandle { handle: left }),
                Some(ForegroundTarget::WindowHandle { handle: right }),
            ) => left == right,
            (
                Some(ForegroundTarget::ProcessId { pid: left }),
                Some(ForegroundTarget::ProcessId { pid: right }),
            ) => left == right,
            _ => false,
        }
    }
}

pub fn target_matches_expected(
    current: &TargetSnapshot,
    expected: &TargetSnapshot,
    require_editable: bool,
) -> bool {
    current.process_id == expected.process_id
        && current.same_stable_target(expected)
        && if require_editable {
            current.focused_text_input == Some(true)
        } else {
            current.focused_text_input != Some(false)
        }
}

pub fn legacy_non_windows_target_is_pasteable(
    target: &TargetSnapshot,
    own_process_id: u32,
) -> bool {
    target.process_id != Some(own_process_id) && target.focused_text_input != Some(false)
}

/// Execute final-delivery target selection with injected platform operations.
///
/// External live targets preserve current-target behavior. A recording-start
/// target is restored only when the first accepted stop came from an exact pill
/// overlay. Restored targets require fresh editable + stable identity proof.
pub fn execute_targeted_delivery<R>(
    session_id: &str,
    stop_context: Option<&PendingStopContext>,
    start_target: Option<&TargetSnapshot>,
    own_process_id: u32,
    mut snapshot: impl FnMut() -> TargetSnapshot,
    mut restore: impl FnMut(&TargetSnapshot) -> bool,
    mut paste: impl FnMut(TargetSnapshot, bool, TargetDeliveryRoute) -> R,
    mut fallback: impl FnMut(CopyFallbackReason, TargetSnapshot) -> R,
    send_enter: bool,
) -> R {
    let live = snapshot();
    if live.process_id == Some(own_process_id) {
        if !stop_context.is_some_and(|context| context.applies_to_overlay(session_id, &live)) {
            return fallback(CopyFallbackReason::NoTextField, live);
        }

        let Some(start) = start_target.filter(|target| {
            target.process_id.is_some_and(|pid| pid != own_process_id)
                && target.focused_text_input == Some(true)
                && target.same_stable_target(target)
        }) else {
            return fallback(CopyFallbackReason::FocusChanged, live);
        };
        if !restore(start) {
            return fallback(CopyFallbackReason::FocusChanged, live);
        }

        let restored = snapshot();
        let restored_is_same_editable_target = restored.process_id == start.process_id
            && restored.focused_text_input == Some(true)
            && restored.same_stable_target(start);
        if !restored_is_same_editable_target {
            return fallback(CopyFallbackReason::FocusChanged, restored);
        }
        return paste(restored, send_enter, TargetDeliveryRoute::RestoredPill);
    }

    if live.process_id.is_none() || !live.same_stable_target(&live) {
        return fallback(CopyFallbackReason::FocusChanged, live);
    }
    if live.focused_text_input == Some(false) {
        return fallback(CopyFallbackReason::NoTextField, live);
    }
    paste(live, send_enter, TargetDeliveryRoute::ExternalLive)
}

/// Where a history record came from. `None`/absent (the serde default) means a
/// normal dictation delivery — every record predating this field deserializes as
/// dictation, so no DB migration is needed. `FileTranscription` marks a "Transcribe
/// a file" result so the History UI can label it distinctly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum DeliverySource {
    Dictation,
    FileTranscription { file_name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryRecord {
    pub delivery_id: String,
    pub transcript: String,
    pub outcome: DeliveryOutcome,
    pub method: DeliveryMethod,
    pub elapsed_ms: u64,
    pub target: TargetSnapshot,
    pub delivered_at_ms: u64,
    /// The transcript BEFORE AI polish (L3). `None` for records predating this
    /// field or when no AI polish ran — lets the UI toggle raw <-> polished
    /// (undo/redo the AI edit) without re-running anything.
    #[serde(default)]
    pub raw_transcript: Option<String>,
    /// Recorded audio length in ms (the clip duration), when known.
    #[serde(default)]
    pub recording_ms: Option<u64>,
    /// Per-step delivery timings. Additive and optional so old history records,
    /// IPC payloads, and tests that only know `elapsed_ms` keep deserializing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_timing: Option<DeliveryTimings>,
    /// Origin of this record. Absent = dictation (back-compat default).
    #[serde(default)]
    pub source: Option<DeliverySource>,
}

impl DeliveryRecord {
    pub fn new(
        delivery_id: impl Into<String>,
        transcript: impl Into<String>,
        outcome: DeliveryOutcome,
        target: TargetSnapshot,
    ) -> Self {
        Self::new_with_timing(delivery_id, transcript, outcome, target, 0)
    }

    pub fn new_with_timing(
        delivery_id: impl Into<String>,
        transcript: impl Into<String>,
        outcome: DeliveryOutcome,
        target: TargetSnapshot,
        elapsed_ms: u64,
    ) -> Self {
        let method = method_for_outcome(&outcome);
        Self {
            delivery_id: delivery_id.into(),
            transcript: transcript.into(),
            method,
            elapsed_ms,
            outcome,
            target,
            delivered_at_ms: now_ms(),
            raw_transcript: None,
            recording_ms: None,
            delivery_timing: None,
            source: None,
        }
    }

    /// Attach the pre-AI-polish transcript + clip duration. Builder, called at the
    /// delivery site where both are known (the polished text is `transcript`).
    pub fn with_raw(mut self, raw_transcript: Option<String>, recording_ms: Option<u64>) -> Self {
        self.raw_transcript = raw_transcript.filter(|r| !r.is_empty());
        self.recording_ms = recording_ms;
        self
    }

    pub fn with_delivery_timing(mut self, timing: DeliveryTimings) -> Self {
        self.delivery_timing = Some(timing);
        self
    }

    /// Tag this record's origin (e.g. a file transcription). Builder; dictation
    /// records simply never call it (source stays None).
    pub fn with_source(mut self, source: DeliverySource) -> Self {
        self.source = Some(source);
        self
    }

    pub fn for_test(
        delivery_id: DeliveryId,
        transcript: impl Into<String>,
        outcome: DeliveryOutcome,
    ) -> Self {
        Self::new(
            delivery_id.into_string(),
            transcript,
            outcome,
            TargetSnapshot::test_target(),
        )
    }

    /// Build (or rebuild) the draft row for a session. `delivery_id` is
    /// deterministic (`draft_delivery_id`) so every store's `upsert_draft`
    /// targets the same row across repeated calls for the same session.
    pub fn new_draft(session_id: &str, transcript: impl Into<String>) -> Self {
        Self::new(
            draft_delivery_id(session_id),
            transcript,
            DeliveryOutcome::Draft,
            TargetSnapshot::empty(),
        )
    }
}

pub trait DeliveryBackend: Send {
    fn deliver(&mut self, transcript: &str) -> Result<DeliveryRecord, DeliveryError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryError {
    pub code: String,
    pub message: String,
}

impl DeliveryError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for DeliveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for DeliveryError {}

pub fn method_for_outcome(outcome: &DeliveryOutcome) -> DeliveryMethod {
    match outcome {
        DeliveryOutcome::Pasted => DeliveryMethod::ClipboardThenPaste,
        DeliveryOutcome::CopiedFallback { .. } => DeliveryMethod::ClipboardOnly,
        DeliveryOutcome::Error { .. } => DeliveryMethod::None,
        DeliveryOutcome::Cancelled => DeliveryMethod::None,
        DeliveryOutcome::Draft => DeliveryMethod::None,
    }
}

/// Deterministic delivery_id for a session's in-progress draft row (crash-
/// recovery draft history rows). Stable per session so repeated
/// `HistoryStore::upsert_draft` calls target the SAME row instead of
/// accumulating one per `TranscriptPartial` tick.
pub fn draft_delivery_id(session_id: &str) -> String {
    format!("draft-{session_id}")
}

pub fn next_delivery_id(_legacy_counter: u64) -> String {
    let sequence = NEXT_DELIVERY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("delivery-{}-{}-{sequence}", std::process::id(), now_ms())
}

pub fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis() as u64
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn copied_fallback_record_from_clipboard_result(
    transcript: &str,
    reason: CopyFallbackReason,
    target: TargetSnapshot,
    started: Instant,
    clipboard_result: Result<(), CopyFallbackReason>,
) -> DeliveryRecord {
    let outcome = match clipboard_result {
        Ok(()) => DeliveryOutcome::CopiedFallback { reason },
        Err(err) => DeliveryOutcome::CopiedFallback { reason: err },
    };
    DeliveryRecord::new_with_timing(
        next_delivery_id(0),
        transcript,
        outcome,
        target,
        elapsed_ms(started),
    )
}

pub fn copy_fallback_record_with_clipboard(
    transcript: &str,
    reason: CopyFallbackReason,
    target: TargetSnapshot,
    write_clipboard: impl FnOnce(&str) -> Result<(), CopyFallbackReason>,
) -> DeliveryRecord {
    copied_fallback_record_from_clipboard_result(
        transcript,
        reason,
        target,
        Instant::now(),
        write_clipboard(transcript),
    )
}

pub fn copy_text_with_clipboard(
    text: &str,
    write_clipboard: impl FnOnce(&str) -> Result<(), CopyFallbackReason>,
) -> Result<(), DeliveryError> {
    write_clipboard(text).map_err(|reason| DeliveryError::new("E_CLIPBOARD", reason.to_string()))
}

pub fn target_process_id(
    target: &TargetSnapshot,
    message: &'static str,
) -> Result<i32, DeliveryError> {
    target
        .process_id
        .map(|pid| pid as i32)
        .ok_or_else(|| DeliveryError::new("E_NO_TARGET", message))
}

pub fn focus_restore_error(message: &'static str) -> DeliveryError {
    DeliveryError::new("E_FOCUS_RESTORE", message)
}

pub fn unsupported_focus_restore() -> Result<(), DeliveryError> {
    Err(DeliveryError::new(
        "E_UNSUPPORTED",
        "focus restore is unsupported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_delivery_ids_share_a_process_global_monotonic_sequence() {
        let first = next_delivery_id(0);
        let second = next_delivery_id(0);
        assert_ne!(first, second);
        assert!(first.starts_with(&format!("delivery-{}-", std::process::id())));
        let first_sequence = first.rsplit_once('-').unwrap().1.parse::<u64>().unwrap();
        let second_sequence = second.rsplit_once('-').unwrap().1.parse::<u64>().unwrap();
        assert_eq!(second_sequence, first_sequence + 1);
    }

    #[test]
    fn method_tracks_outcome() {
        assert_eq!(
            method_for_outcome(&DeliveryOutcome::Pasted),
            DeliveryMethod::ClipboardThenPaste
        );
        assert_eq!(
            method_for_outcome(&DeliveryOutcome::CopiedFallback {
                reason: CopyFallbackReason::PasteFailed
            }),
            DeliveryMethod::ClipboardOnly
        );
        assert_eq!(
            method_for_outcome(&DeliveryOutcome::Error {
                code: "E".into(),
                message: "m".into()
            }),
            DeliveryMethod::None
        );
        assert_eq!(
            method_for_outcome(&DeliveryOutcome::Cancelled),
            DeliveryMethod::None
        );
        assert_eq!(
            method_for_outcome(&DeliveryOutcome::Draft),
            DeliveryMethod::None
        );
    }

    #[test]
    fn draft_delivery_id_is_deterministic_per_session() {
        assert_eq!(draft_delivery_id("session-1"), draft_delivery_id("session-1"));
        assert_ne!(draft_delivery_id("session-1"), draft_delivery_id("session-2"));
        assert_eq!(draft_delivery_id("session-1"), "draft-session-1");
    }

    #[test]
    fn new_draft_uses_the_deterministic_id_and_empty_target() {
        let record = DeliveryRecord::new_draft("session-1", "partial text so far");
        assert_eq!(record.delivery_id, draft_delivery_id("session-1"));
        assert_eq!(record.outcome, DeliveryOutcome::Draft);
        assert_eq!(record.transcript, "partial text so far");
        assert_eq!(record.target, TargetSnapshot::empty());

        // Rebuilding for the same session reuses the same id (upsert key).
        let second = DeliveryRecord::new_draft("session-1", "more text");
        assert_eq!(record.delivery_id, second.delivery_id);
    }

    #[test]
    fn fallback_reason_display_is_snake_case() {
        assert_eq!(
            CopyFallbackReason::ClipboardUnavailable.to_string(),
            "clipboard_unavailable"
        );
        assert_eq!(CopyFallbackReason::Other("x".into()).to_string(), "x");
    }

    #[test]
    fn test_target_carries_explicit_foreground_identity() {
        assert_eq!(
            TargetSnapshot::test_target().foreground_target,
            Some(ForegroundTarget::WindowHandle { handle: 1 })
        );
    }

    #[test]
    fn stable_target_identity_requires_matching_field_or_foreground_target() {
        let target = TargetSnapshot::test_target();
        assert!(target.same_stable_target(&target));

        let mut other_field = target.clone();
        other_field.focused_control_handle = Some(3);
        assert!(!target.same_stable_target(&other_field));

        let mut unknown = target.clone();
        unknown.focused_control_handle = None;
        assert!(!target.same_stable_target(&unknown));

        let mut no_identity = target.clone();
        no_identity.focused_control_handle = None;
        no_identity.foreground_target = None;
        assert!(!no_identity.same_stable_target(&no_identity));
    }

    #[test]
    fn target_snapshot_deserializes_without_foreground_identity() {
        let snapshot: TargetSnapshot = serde_json::from_str(
            r#"{
                "process_id": 42,
                "process_name": "LegacyApp",
                "window_title": "Legacy Window",
                "window_handle": 1001,
                "is_elevated": false
            }"#,
        )
        .expect("legacy target snapshot should remain readable");

        assert_eq!(snapshot.process_id, Some(42));
        assert_eq!(snapshot.window_handle, Some(1001));
        assert_eq!(snapshot.focused_control_handle, None);
        assert_eq!(snapshot.foreground_target, None);
        assert_eq!(snapshot.focused_text_input, None);
    }

    #[test]
    fn delivery_record_serializes_named_step_timings_and_reads_legacy_records() {
        let timing = DeliveryTimings::new(42)
            .with_step("clipboard_write", 3)
            .with_step("paste_keystroke", 4)
            .with_step("clipboard_cleanup", 5);
        let record = DeliveryRecord::for_test(
            DeliveryId::new("delivery-with-timing"),
            "hello",
            DeliveryOutcome::Pasted,
        )
        .with_delivery_timing(timing);

        let serialized = serde_json::to_value(&record).expect("record serializes");
        assert_eq!(serialized["delivery_timing"]["total_ms"], 42);
        assert_eq!(
            serialized["delivery_timing"]["steps"][1]["name"],
            "paste_keystroke"
        );
        assert_eq!(serialized["delivery_timing"]["steps"][2]["elapsed_ms"], 5);

        let legacy: DeliveryRecord = serde_json::from_str(
            r#"{
                "delivery_id": "legacy",
                "transcript": "old",
                "outcome": { "kind": "pasted" },
                "method": "clipboard_then_paste",
                "elapsed_ms": 9,
                "target": {
                    "process_id": 42,
                    "process_name": "LegacyApp",
                    "window_title": "Legacy Window",
                    "window_handle": 1001,
                    "is_elevated": false
                },
                "delivered_at_ms": 100
            }"#,
        )
        .expect("legacy delivery record should still deserialize");
        assert_eq!(legacy.delivery_timing, None);
    }
}
