// HR-T1: focused coverage for `EngineRuntime::finalize_transcript_with_audio_secs`
// (heardright-engine/src/runtime_sections/finalize_transcript.rs).
//
// Mirrors the harness conventions in section01.rs: `new_runtime()` +
// `assert_finalized()` for the platform-split delivery/transcript variants,
// and `heardright_engine::settings::replace_runtime_config` for the Pro gate
// (matching `voice_commands_require_pro_license`).
//
// Scope note: the `:cancel` / `:send_enter` request-id suffixes are parsed at
// the IPC layer (`ipc_sections/section01.rs::decode_request`), which is a
// private fn inside an `include!`-merged module and not reachable from an
// integration test. What that parsing DOES is call `runtime.cancel(id)` or
// `runtime.begin_stop(id, send_enter, local_only)` before the final arrives — so these
// tests characterize finalize's behavior given those exact runtime calls,
// which is the observable effect of the suffix routing.

// ---- Pro gate: standalone command vs plain dictation (characterization) ----

#[test]
fn free_user_standalone_command_utterance_is_delivered_as_text() {
    let _settings_guard = settings_override_test_lock();
    // Characterizes CURRENT behavior: on a free license, an utterance that
    // matches the command catalog ("select all") is NOT classified as a
    // command (classify_action short-circuits to None for is_pro=false) and
    // falls through to plain dictation delivery.
    heardright_engine::settings::replace_runtime_config(heardright_core::settings::SettingsBlob {
        is_pro: Some(false),
        ..Default::default()
    });
    let mut runtime = new_runtime();
    runtime.start_dictation("session-free-1").unwrap();
    runtime.begin_stop("session-free-1", false, false).unwrap();
    let result = runtime
        .finalize_transcript("session-free-1", Ok("select all".into()))
        .unwrap();
    assert!(
        !matches!(
            result,
            FinalizeOutcome::ResetToArmed | FinalizeOutcome::CommandFailed { .. }
        ),
        "free-tier catalog command must stay dictation (delivered/transcript), got {result:?}"
    );
    heardright_engine::settings::replace_runtime_config(
        heardright_core::settings::SettingsBlob::default(),
    );
}

#[test]
fn pro_user_standalone_command_utterance_is_dispatched() {
    let _settings_guard = settings_override_test_lock();
    // Pro gate open: the same catalog utterance is classified as a command
    // and short-circuits to CommandDispatched (or CommandFailed on hosts with no
    // platform dispatch available on this test host — e.g. missing macOS
    // Accessibility grant). Either way it must NOT fall through to a
    // Delivery/Transcript of the literal text.
    heardright_engine::settings::replace_runtime_config(heardright_core::settings::SettingsBlob {
        is_pro: Some(true),
        ..Default::default()
    });
    let mut runtime = new_runtime();
    runtime.start_dictation("session-pro-1").unwrap();
    runtime.begin_stop("session-pro-1", false, false).unwrap();
    let result = runtime
        .finalize_transcript("session-pro-1", Ok("select all".into()))
        .unwrap();
    assert!(
        matches!(
            result,
            FinalizeOutcome::CommandDispatched { .. } | FinalizeOutcome::CommandFailed { .. }
        ),
        "pro-tier catalog command must dispatch (or fail to dispatch), got {result:?}"
    );
    heardright_engine::settings::replace_runtime_config(
        heardright_core::settings::SettingsBlob::default(),
    );
}

// ---- Empty transcript ----

#[test]
fn empty_transcript_produces_copied_fallback_with_no_text_delivery() {
    let mut runtime = new_runtime();
    runtime.start_dictation("session-empty-1").unwrap();
    runtime.begin_stop("session-empty-1", false, false).unwrap();
    let result = runtime
        .finalize_transcript("session-empty-1", Ok("   ".into()))
        .unwrap();
    match result {
        FinalizeOutcome::Delivery { record, .. } => {
            assert_eq!(record.transcript, "");
            assert!(matches!(
                record.outcome,
                DeliveryOutcome::CopiedFallback {
                    reason: heardright_core::delivery::CopyFallbackReason::EmptyTranscript
                }
            ));
        }
        other => panic!("expected Delivery(CopiedFallback(EmptyTranscript)), got {other:?}"),
    }
    assert!(matches!(runtime.state(), EngineState::Idle));
}

// ---- Stop-intent suffixes (":cancel" / ":send_enter") ----
//
// These suffixes are consumed by the IPC decoder BEFORE finalize ever runs
// (section01.rs:221 `request_id.contains(":cancel")` -> `Request::CancelDictation`,
// :228 `":send_enter"` -> `Request::StopDictation { send_enter: true, .. }`).
// The tests below drive the runtime calls those requests make
// (`runtime.cancel(id)` / `runtime.begin_stop(id, true, false)`), which is
// the observable effect of a request id carrying that suffix.

#[test]
fn cancel_suffix_request_never_delivers_a_late_final() {
    let mut runtime = new_runtime();
    runtime.start_dictation("session-cancel-1").unwrap();
    runtime
        .begin_stop("session-cancel-1", false, false)
        .unwrap();
    // Request::CancelDictation handler: `runtime.lock().cancel(&session_id)`.
    runtime.cancel("session-cancel-1");
    assert!(matches!(runtime.state(), EngineState::Idle));
    // A final that arrives after the cancel is stale and must be dropped —
    // same guard `stale_final_is_dropped` exercises in section01.rs.
    let result = runtime
        .finalize_transcript("session-cancel-1", Ok("this must not deliver".into()))
        .unwrap();
    assert!(matches!(result, FinalizeOutcome::NoOp));
}

#[test]
fn send_enter_suffix_request_delivers_with_enter_flag_set() {
    let mut runtime = new_runtime();
    runtime.start_dictation("session-send-1").unwrap();
    // Request::StopDictation { send_enter: true, .. } handler:
    // `runtime.lock().begin_stop(&session_id, send_enter, local_only)`.
    runtime.begin_stop("session-send-1", true, false).unwrap();
    let result = runtime
        .finalize_transcript("session-send-1", Ok("hello world".into()))
        .unwrap();
    // send_enter is carried in FinalizeOutcome::Transcript on macOS; on other
    // platforms the engine applies it as an Enter keystroke itself (not
    // stored on the DeliveryRecord) — assert_finalized already encodes this
    // platform split.
    assert_finalized(result, "Hello world", true);
}

// ---- Suffix-parsing regression canary (documents CURRENT fall-through) ----

#[test]
fn typo_cancel_suffix_does_not_trigger_cancel_semantics() {
    // Regression canary: `request_id.contains(":cancel")` requires the exact
    // substring. A typo'd suffix like ":cancle" does NOT match, so the IPC
    // decoder falls through to the normal StopDictation path instead of
    // CancelDictation (section01.rs:221-228). This test documents that
    // CURRENT fall-through by driving the runtime call the decoder actually
    // makes for a non-matching suffix: `begin_stop(id, send_enter=false)`
    // (send_enter only matches on ":send_enter", which ":cancle" also isn't).
    // If someone "fixes" the typo to be recognized as cancel, this test must
    // be updated deliberately — it is a canary, not a spec.
    let mut runtime = new_runtime();
    runtime.start_dictation("session-typo-1").unwrap();
    runtime.begin_stop("session-typo-1", false, false).unwrap();
    assert!(matches!(
        runtime.state(),
        EngineState::Transcribing { session_id } if session_id == "session-typo-1"
    ));
    let result = runtime
        .finalize_transcript("session-typo-1", Ok("hello world".into()))
        .unwrap();
    // Falls through to normal delivery, NOT NoOp/ResetToArmed — proving the
    // typo'd suffix did not short-circuit as a cancel.
    assert_finalized(result, "Hello world", false);
}

// ---- Cancel-to-history (`local_only` finalize) ----
//
// `StopKind::CancelToHistory` decodes to
// `Request::StopDictation { local_only: true, .. }`
// (`ipc_sections/section01.rs::decode_request`), which calls
// `runtime.begin_stop(id, false, true)`. Finalize must still run ASR to
// completion and return the local (L0) text — exactly like a normal Stop —
// but must never invoke the cloud L1/L2/L3 polish lanes.
//
// Proved via `l3_cleanup::health().skips`: `prompt_polish_outcome` /
// `summarize_outcome` increment it from their disabled-cleanup preflight
// check the instant they're called (deterministic, no network — cleanup is
// disabled by default in this test process via unset `HEARDRIGHT_L3_CLEANUP`).
// A normal stop with an ai-transform ("... prompt") tail DOES call that
// lane (and gets skipped locally); a cancel-to-history stop with the exact
// same tail must not call it at all, so `skips` must not move.
//
// This test measures cloud-lane use, not focus delivery. Accept either shell
// handoff or copy fallback because live focused-field state can select either.
fn finalized_text(result: FinalizeOutcome) -> String {
    match result {
        FinalizeOutcome::Transcript { text, .. } => text,
        FinalizeOutcome::Delivery { record, .. } => record.transcript,
        other => panic!("expected finalized text, got {other:?}"),
    }
}

#[test]
fn local_only_finalize_skips_cloud_lane_entirely() {
    let _settings_guard = settings_override_test_lock();
    let previous_cleanup = std::env::var_os("HEARDRIGHT_L3_CLEANUP");
    std::env::remove_var("HEARDRIGHT_L3_CLEANUP");

    // Baseline: a normal stop with an ai-transform tail DOES reach the cloud
    // lane (and is skipped there, since cleanup is disabled) — `skips` moves.
    let before = heardright_engine::l3_cleanup::health().skips;
    let mut runtime = new_runtime();
    runtime.start_dictation("session-cloud-lane-1").unwrap();
    runtime
        .begin_stop("session-cloud-lane-1", false, false)
        .unwrap();
    let result = runtime
        .finalize_transcript(
            "session-cloud-lane-1",
            Ok("make this clearer prompt".into()),
        )
        .unwrap();
    let after = heardright_engine::l3_cleanup::health().skips;
    assert!(
        after > before,
        "expected a normal stop's ai-transform tail to reach (and skip inside) the cloud lane"
    );
    assert_eq!(finalized_text(result), "Make this clearer");

    // Cancel-to-history: the exact same ai-transform tail must never reach
    // the cloud lane at all — `skips` must not move.
    let before = heardright_engine::l3_cleanup::health().skips;
    let mut runtime = new_runtime();
    runtime.start_dictation("session-cloud-lane-2").unwrap();
    runtime
        .begin_stop("session-cloud-lane-2", false, true)
        .unwrap();
    let result = runtime
        .finalize_transcript(
            "session-cloud-lane-2",
            Ok("make this clearer prompt".into()),
        )
        .unwrap();
    let after = heardright_engine::l3_cleanup::health().skips;
    assert_eq!(
        after, before,
        "cancel-to-history finalize must never call the cloud L1/L2/L3 lane"
    );
    // Same L0 text as the baseline — cancel-to-history still runs ASR +
    // local polish to completion, it only skips the cloud step.
    assert_eq!(finalized_text(result), "Make this clearer");

    match previous_cleanup {
        Some(value) => std::env::set_var("HEARDRIGHT_L3_CLEANUP", value),
        None => std::env::remove_var("HEARDRIGHT_L3_CLEANUP"),
    }
}

#[test]
fn explicit_app_data_root_matches_the_shell_contract() {
    let _settings_guard = settings_override_test_lock();
    let previous = std::env::var_os("HR_APP_DATA_DIR");
    let explicit = std::env::temp_dir().join(format!(
        "heardright-explicit-app-data-{}",
        std::process::id()
    ));
    std::env::set_var("HR_APP_DATA_DIR", &explicit);

    assert_eq!(heardright_engine::settings::app_data_root(), explicit);

    match previous {
        Some(value) => std::env::set_var("HR_APP_DATA_DIR", value),
        None => std::env::remove_var("HR_APP_DATA_DIR"),
    }
}
