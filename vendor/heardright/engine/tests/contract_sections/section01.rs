// Contract tests for the `heardright-engine` sidecar's runtime + protocol.
//
// These tests exercise the in-process `EngineRuntime` and assert:
//  - state machine transitions
//  - delivery target precedence (spec: current editable > original captured
//    > copy fallback + single-transcript mini clipboard)
//  - focus tracking across the recording lifecycle
//  - protocol-version validation
//  - the wire shape of `EngineFrame` (round-trips through
//    `heardright_core::engine::validate_engine_frame`)

use heardright_core::delivery::ForegroundTarget;
use heardright_core::engine::{
    validate_engine_frame, EngineFrame, EnginePayload, EngineSchemaName, PROTOCOL_MAJOR,
};

use heardright_engine::delivery::{DeliveryOutcome, TargetSnapshot};
use heardright_engine::focus::FocusTracker;
use heardright_engine::runtime::{EngineRuntime, EngineState, FinalizeOutcome};

fn new_runtime() -> EngineRuntime {
    std::env::set_var("HEARDRIGHT_ENGINE_TEST_MODE", "1");
    EngineRuntime::new(std::path::PathBuf::from("."))
}

/// Assert a non-empty finalize produced `text` (with the given send intent).
/// Both platforms hand the stripped transcript to the shell, so `send_enter`
/// is carried in `FinalizeOutcome::Transcript`.
fn assert_finalized(result: FinalizeOutcome, text: &str, send_enter: bool) {
    match result {
        FinalizeOutcome::Transcript {
            text: actual_text,
            send_enter: actual_send_enter,
            ..
        } => {
            assert_eq!(actual_text, text);
            assert_eq!(actual_send_enter, send_enter, "send intent");
        }
        other => panic!("expected transcript, got {other:?}"),
    }
}

// ---- State machine ----

#[test]
fn runtime_starts_idle() {
    let runtime = new_runtime();
    assert!(matches!(runtime.state(), EngineState::Idle));
}

#[test]
fn start_dictation_transitions_to_recording() {
    let mut runtime = new_runtime();
    runtime.start_dictation("session-1").unwrap();
    let state = runtime.state();
    assert!(
        matches!(state, EngineState::Recording { session_id } if session_id == "session-1"),
        "expected Recording, got {state:?}"
    );
}

#[test]
fn start_dictation_is_idempotent_for_same_session() {
    let mut runtime = new_runtime();
    runtime.start_dictation("session-1").unwrap();
    runtime.start_dictation("session-1").unwrap();
    let state = runtime.state();
    assert!(
        matches!(state, EngineState::Recording { session_id } if session_id == "session-1"),
        "expected Recording, got {state:?}"
    );
}

#[test]
fn begin_stop_transitions_to_transcribing() {
    let mut runtime = new_runtime();
    runtime.start_dictation("session-1").unwrap();
    runtime.begin_stop("session-1", false, false).unwrap();
    assert!(matches!(
        runtime.state(),
        EngineState::Transcribing { session_id } if session_id == "session-1"
    ));
}

#[test]
fn cancel_returns_to_idle() {
    let mut runtime = new_runtime();
    runtime.start_dictation("session-1").unwrap();
    runtime.cancel("session-1");
    assert!(matches!(runtime.state(), EngineState::Idle));
}

#[test]
fn cancel_on_unknown_session_is_noop() {
    let mut runtime = new_runtime();
    runtime.start_dictation("session-1").unwrap();
    runtime.cancel("session-other");
    assert!(matches!(runtime.state(), EngineState::Recording { .. }));
}

#[test]
fn stale_final_is_dropped() {
    let mut runtime = new_runtime();
    runtime.start_dictation("session-1").unwrap();
    runtime.begin_stop("session-1", false, false).unwrap();
    runtime.cancel("session-1");
    // A late final after the cancel must be dropped, not delivered.
    let result = runtime.finalize_transcript("session-1", Ok("hi".into()));
    assert!(matches!(result, Ok(FinalizeOutcome::NoOp)));
}

#[test]
fn capture_failure_clears_only_matching_recording() {
    let mut runtime = new_runtime();
    runtime.begin_recording("capture-a").unwrap();
    assert!(!runtime.fail_recording_capture("stale"));
    assert_eq!(
        runtime.state(),
        &EngineState::Recording {
            session_id: "capture-a".into()
        }
    );
    assert!(runtime.fail_recording_capture("capture-a"));
    assert_eq!(runtime.state(), &EngineState::Idle);
    runtime.begin_recording("capture-b").unwrap();
    assert!(matches!(
        runtime.state(),
        EngineState::Recording { session_id } if session_id == "capture-b"
    ));
}

#[test]
fn tombstoned_final_cannot_clobber_a_new_recording() {
    let mut runtime = new_runtime();
    runtime.start_dictation("session-1").unwrap();
    runtime.begin_stop("session-1", false, false).unwrap();
    runtime.cancel("session-1");
    runtime.start_dictation("session-2").unwrap();

    let result = runtime.finalize_transcript("session-1", Ok("late text".into()));
    assert!(matches!(result, Ok(FinalizeOutcome::NoOp)));
    assert!(matches!(
        runtime.state(),
        EngineState::Recording { session_id } if session_id == "session-2"
    ));
}

// ---- Voice control commands ----

#[test]
fn final_asr_cancel_phrase_remains_dictation() {
    let mut runtime = new_runtime();
    runtime.start_dictation("session-1").unwrap();
    runtime.begin_stop("session-1", false, false).unwrap();
    let result = runtime
        .finalize_transcript("session-1", Ok("cancel that zephyr cancel".into()))
        .unwrap();
    assert_finalized(result, "Cancel that Zephyr cancel", false);
    assert!(matches!(runtime.state(), EngineState::Idle));
}

#[test]
fn final_asr_stop_phrase_remains_dictation() {
    let mut runtime = new_runtime();
    runtime.start_dictation("session-1").unwrap();
    runtime.begin_stop("session-1", false, false).unwrap();
    let result = runtime
        .finalize_transcript("session-1", Ok("hello world zephyr stop".into()))
        .unwrap();
    assert_finalized(result, "Hello world Zephyr stop", false);
}

#[test]
fn final_asr_send_phrase_remains_dictation_without_send_intent() {
    let mut runtime = new_runtime();
    runtime.start_dictation("session-1").unwrap();
    runtime.begin_stop("session-1", false, false).unwrap();
    let result = runtime
        .finalize_transcript("session-1", Ok("ship it zephyr send".into()))
        .unwrap();
    assert_finalized(result, "Ship it Zephyr send", false);
}

// ---- Pro gate ----

#[test]
fn voice_commands_require_pro_license() {
    let _settings_guard = settings_override_test_lock();
    // ALL standalone commands are Pro (locked 2026-06-30). On a free license
    // "select all" is plain dictation and delivers as text instead of firing.
    heardright_engine::settings::replace_runtime_config(heardright_core::settings::SettingsBlob {
        is_pro: Some(false),
        ..Default::default()
    });
    let mut runtime = new_runtime();
    runtime.start_dictation("session-1").unwrap();
    runtime.begin_stop("session-1", false, false).unwrap();
    let result = runtime
        .finalize_transcript("session-1", Ok("select all".into()))
        .unwrap();
    assert!(
        !matches!(result, FinalizeOutcome::ResetToArmed),
        "free-tier catalog command must stay dictation, got a command dispatch"
    );
    // Restore the test default so test order can't leak is_pro=false.
    heardright_engine::settings::replace_runtime_config(
        heardright_core::settings::SettingsBlob::default(),
    );

    // With Pro, the same utterance IS a command and resets to armed.
    heardright_engine::settings::replace_runtime_config(heardright_core::settings::SettingsBlob {
        is_pro: Some(true),
        ..Default::default()
    });
    let mut runtime = new_runtime();
    runtime.start_dictation("session-2").unwrap();
    runtime.begin_stop("session-2", false, false).unwrap();
    let result = runtime
        .finalize_transcript("session-2", Ok("select all".into()))
        .unwrap();
    // Classified as a command either way; CommandFailed covers test hosts
    // without the Accessibility grant (dispatch pre-check).
    assert!(matches!(
        result,
        FinalizeOutcome::CommandDispatched { .. } | FinalizeOutcome::CommandFailed { .. }
    ));
    heardright_engine::settings::replace_runtime_config(
        heardright_core::settings::SettingsBlob::default(),
    );
}

// ---- Error handling ----

#[test]
fn final_error_transitions_to_error_state() {
    let mut runtime = new_runtime();
    runtime.start_dictation("session-1").unwrap();
    runtime.begin_stop("session-1", false, false).unwrap();
    let result = runtime.finalize_transcript("session-1", Err("transcribe failed".into()));
    assert!(result.is_err());
    assert!(matches!(
        runtime.state(),
        EngineState::Error { message, .. } if message == "transcribe failed"
    ));
}

// ---- Info + history ----

#[test]
fn engine_info_reports_sidecar_mode() {
    let runtime = new_runtime();
    let info = runtime.engine_info();
    assert_eq!(info.mode, "sidecar");
    assert!(info.engine_version.is_some());
}

#[test]
fn recent_history_is_empty_on_fresh_runtime() {
    let runtime = new_runtime();
    assert!(runtime.recent_history(5).is_empty());
}

#[test]
fn shell_delivery_leaves_engine_history_empty() {
    // Both platforms hand delivery and history ownership to the shell.
    let mut runtime = new_runtime();
    runtime.start_dictation("session-1").unwrap();
    runtime.begin_stop("session-1", false, false).unwrap();
    let result = runtime
        .finalize_transcript("session-1", Ok("hello world".into()))
        .unwrap();

    match result {
        FinalizeOutcome::Transcript {
            text, send_enter, ..
        } => {
            assert_eq!(text, "Hello world");
            assert!(!send_enter);
        }
        other => panic!("expected transcript, got {other:?}"),
    }
    assert!(runtime.recent_history(5).is_empty());
}

#[test]
fn transcribe_file_path_round_trips_through_worker_channel() {
    let mut runtime = new_runtime();
    let result = runtime
        .transcribe_file_path(std::path::PathBuf::from("/tmp/heardright-test.wav"))
        .expect("file transcription through worker");
    assert_eq!(result.text, "");
    assert_eq!(result.srt, "");
    assert_eq!(result.vtt, "");
}

#[test]
fn replace_recent_history_seeds_last_delivery() {
    let mut runtime = new_runtime();
    let record = heardright_engine::delivery::DeliveryRecord::new(
        "delivery-1",
        "Hello world",
        DeliveryOutcome::Pasted,
        editable_target("textbox-A"),
    );
    runtime.replace_recent_history(vec![record.clone()]);
    let recent = runtime.recent_history(5);
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].delivery_id, "delivery-1");
    assert_eq!(
        runtime
            .last_delivery()
            .expect("seeded last delivery")
            .delivery_id,
        "delivery-1"
    );
}

#[test]
fn replace_recent_history_overwrites_previous_ring_contents() {
    let mut runtime = new_runtime();
    let first = heardright_engine::delivery::DeliveryRecord::new(
        "delivery-1",
        "First",
        DeliveryOutcome::Pasted,
        editable_target("textbox-A"),
    );
    let second = heardright_engine::delivery::DeliveryRecord::new(
        "delivery-2",
        "Second",
        DeliveryOutcome::Pasted,
        editable_target("textbox-B"),
    );
    runtime.replace_recent_history(vec![first]);
    runtime.replace_recent_history(vec![second.clone()]);
    let recent = runtime.recent_history(5);
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].delivery_id, "delivery-2");
    assert_eq!(
        runtime
            .last_delivery()
            .expect("reseeded last delivery")
            .delivery_id,
        second.delivery_id
    );
}

// ---- Focus tracking ----
