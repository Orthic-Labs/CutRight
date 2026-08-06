fn editable_target(label: &str) -> TargetSnapshot {
    TargetSnapshot {
        process_id: Some(1),
        process_name: Some("notepad".into()),
        window_title: Some(label.into()),
        window_handle: Some(11),
        focused_control_handle: Some(22),
        foreground_target: Some(ForegroundTarget::WindowHandle { handle: 11 }),
        focused_text_input: Some(true),
        is_elevated: Some(false),
    }
}

fn non_editable_target(label: &str) -> TargetSnapshot {
    TargetSnapshot {
        process_id: Some(2),
        process_name: Some("explorer".into()),
        window_title: Some(label.into()),
        window_handle: Some(33),
        focused_control_handle: Some(44),
        foreground_target: Some(ForegroundTarget::WindowHandle { handle: 33 }),
        focused_text_input: Some(false),
        is_elevated: Some(false),
    }
}

#[test]
fn focus_tracker_snapshots_editable_target() {
    let mut tracker = FocusTracker::new();
    tracker.update_current(editable_target("textbox-A"));
    tracker.snapshot_at_start();
    let captured = tracker.captured_target().expect("captured");
    assert_eq!(captured.focused_text_input, Some(true));
    assert_eq!(captured.window_title.as_deref(), Some("textbox-A"));
}

#[test]
fn focus_tracker_ignores_non_editable_target_for_capture() {
    let mut tracker = FocusTracker::new();
    tracker.update_current(non_editable_target("desktop"));
    tracker.snapshot_at_start();
    // The spec: only editable controls are eligible to be the captured
    // target. A non-editable focus does not become the captured target —
    // the copy fallback path stays available.
    assert!(tracker.captured_target().is_none());
}

#[test]
fn focus_tracker_updates_current_without_changing_captured() {
    let mut tracker = FocusTracker::new();
    tracker.update_current(editable_target("textbox-A"));
    tracker.snapshot_at_start();
    tracker.update_current(editable_target("textbox-B"));
    assert_eq!(
        tracker.captured_target().unwrap().window_title.as_deref(),
        Some("textbox-A")
    );
    assert_eq!(
        tracker.current_target().unwrap().window_title.as_deref(),
        Some("textbox-B")
    );
}

// ---- Wire format (the IPC contract) ----

#[test]
fn health_frame_round_trips_through_validator() {
    let frame = EngineFrame::health("r1", "t1");
    let value = serde_json::to_value(&frame).unwrap();
    let parsed = validate_engine_frame(&value).expect("health frame must validate");
    assert_eq!(parsed.protocol_major, PROTOCOL_MAJOR);
    assert!(matches!(parsed.payload, Some(EnginePayload::Health { .. })));
}

#[test]
fn health_frame_can_carry_l3_cleanup_diagnostics() {
    let frame = EngineFrame::base(
        EngineSchemaName::EngineHealth,
        "r1",
        None,
        "t1",
        Some(EnginePayload::Health {
            status: heardright_core::engine::EngineHealthStatus::Ok,
            diagnostics: Some(serde_json::json!({
                "l3_cleanup": {
                    "attempts": 1,
                    "local_fallbacks": 0,
                    "circuit_open": false
                }
            })),
        }),
        None,
    );
    let parsed = validate_engine_frame(&serde_json::to_value(&frame).unwrap())
        .expect("health diagnostics must validate");
    match parsed.payload {
        Some(EnginePayload::Health {
            diagnostics: Some(diagnostics),
            ..
        }) => assert_eq!(diagnostics["l3_cleanup"]["attempts"], 1),
        other => panic!("expected health diagnostics, got {other:?}"),
    }
}

#[test]
fn recording_started_frame_requires_session_id() {
    let frame = EngineFrame::base(
        EngineSchemaName::RecordingStarted,
        "r1",
        None, // missing session
        "t1",
        Some(EnginePayload::RecordingStarted {
            session_id: "s1".into(),
        }),
        None,
    );
    let value = serde_json::to_value(&frame).unwrap();
    let err = validate_engine_frame(&value).unwrap_err();
    let s = err.to_string();
    assert!(s.contains("session"), "expected session error, got {s}");
}

#[test]
fn transcript_final_frame_round_trips() {
    let frame = EngineFrame::base(
        EngineSchemaName::TranscriptFinal,
        "r1",
        Some("s1".to_string()),
        "t1",
        Some(EnginePayload::TranscriptFinal {
            text: "hello world".to_string(),
            confidence: Some(1.0),
            diagnostics: Some(serde_json::json!({ "delivery_id": "d1" })),
        }),
        None,
    );
    let value = serde_json::to_value(&frame).unwrap();
    let parsed = validate_engine_frame(&value).expect("transcript final must validate");
    let payload = parsed.payload.expect("payload");
    match payload {
        EnginePayload::TranscriptFinal { text, .. } => assert_eq!(text, "hello world"),
        other => panic!("expected TranscriptFinal, got {other:?}"),
    }
}

#[test]
fn transcript_partial_frame_round_trips() {
    let frame = EngineFrame::base(
        EngineSchemaName::TranscriptPartial,
        "r1",
        Some("s1".to_string()),
        "t1",
        Some(EnginePayload::TranscriptPartial {
            text: "hello".to_string(),
            revision: 2,
            audio_secs: Some(4.48),
            diagnostics: None,
        }),
        None,
    );
    let value = serde_json::to_value(&frame).unwrap();
    let parsed = validate_engine_frame(&value).expect("transcript partial must validate");
    match parsed.payload.expect("payload") {
        EnginePayload::TranscriptPartial { text, revision, .. } => {
            assert_eq!(text, "hello");
            assert_eq!(revision, 2);
        }
        other => panic!("expected TranscriptPartial, got {other:?}"),
    }
}

#[test]
fn error_frame_carries_error_payload_not_data() {
    let frame = EngineFrame::base(
        EngineSchemaName::EngineError,
        "r1",
        None,
        "t1",
        None,
        Some(heardright_core::engine::EngineErrorPayload {
            code: "E_ENGINE".into(),
            message: "boom".into(),
            retryable: false,
            diagnostics: None,
        }),
    );
    let value = serde_json::to_value(&frame).unwrap();
    let parsed = validate_engine_frame(&value).expect("error frame must validate");
    assert!(parsed.error.is_some());
    assert!(parsed.payload.is_none());
}
