
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_health_frame_passes() {
        let frame = EngineFrame::health("r1", "t1");
        assert!(validate_frame(&frame).is_ok());
        let value = serde_json::to_value(&frame).unwrap();
        assert!(validate_engine_frame(&value).is_ok());
    }

    #[test]
    fn protocol_major_mismatch_rejected() {
        let mut frame = EngineFrame::health("r1", "t1");
        frame.protocol_major = 2;
        assert_eq!(
            validate_frame(&frame),
            Err(EngineContractError::ProtocolMajorMismatch {
                expected: 1,
                actual: 2
            })
        );
    }

    #[test]
    fn ui_concept_leak_rejected() {
        let v = serde_json::json!({
            "protocol_major": 1, "protocol_minor": 0,
            "schema_name": "transcript_final", "schema_version": 1,
            "engine_version": "x", "request_id": "r", "trace_id": "t",
            "session_id": "s",
            "payload": { "kind": "transcript_final", "text": "hi", "pill_state": "leak" }
        });
        match validate_engine_frame(&v) {
            Err(EngineContractError::UiConceptLeaked(field)) => assert_eq!(field, "pill_state"),
            other => panic!("expected UiConceptLeaked, got {other:?}"),
        }
    }

    #[test]
    fn recording_started_requires_session_id() {
        let frame = EngineFrame::base(
            EngineSchemaName::RecordingStarted,
            "r1",
            None, // no session
            "t1",
            Some(EnginePayload::RecordingStarted {
                session_id: "s1".into(),
            }),
            None,
        );
        assert_eq!(
            validate_frame(&frame),
            Err(EngineContractError::MissingSessionId)
        );
    }

    #[test]
    fn file_transcription_frames_validate() {
        let request = EngineFrame::base(
            EngineSchemaName::FileTranscriptionRequest,
            "r1",
            None,
            "t1",
            Some(EnginePayload::FileTranscriptionRequest {
                path: "/tmp/sample.wav".into(),
            }),
            None,
        );
        assert!(validate_frame(&request).is_ok());

        let response = EngineFrame::base(
            EngineSchemaName::FileTranscriptionResult,
            "r1",
            None,
            "t2",
            Some(EnginePayload::FileTranscriptionResult {
                text: "Hello".into(),
                srt: String::new(),
                vtt: String::new(),
                words: Vec::new(),
            }),
            None,
        );
        assert!(validate_frame(&response).is_ok());
    }

    #[test]
    fn error_frame_requires_error_and_no_payload() {
        let mut frame =
            EngineFrame::base(EngineSchemaName::EngineError, "r1", None, "t1", None, None);
        assert_eq!(
            validate_frame(&frame),
            Err(EngineContractError::MissingError)
        );
        frame.error = Some(EngineErrorPayload {
            code: "E".into(),
            message: "m".into(),
            retryable: false,
            diagnostics: None,
        });
        assert!(validate_frame(&frame).is_ok());
    }

    #[test]
    fn free_tier_blocks_over_ten_minutes() {
        let err = check_duration_limit(11 * 60, false).unwrap_err();
        assert!(err.to_lowercase().contains("pro"), "got: {err}");
    }

    #[test]
    fn free_tier_allows_ten_minutes_exactly() {
        assert!(check_duration_limit(FREE_TRANSCRIBE_MAX_SECS, false).is_ok());
    }

    #[test]
    fn pro_has_no_length_limit() {
        assert!(check_duration_limit(90 * 60, true).is_ok());
        assert!(check_duration_limit(100 * 3600, true).is_ok()); // 100h — unlimited
    }

    // ---- Protocol v2 (dispatch #9/#10): StopKind + DiagnosticsPayload ----

    #[test]
    fn stop_kind_round_trips_all_variants() {
        for kind in [
            StopKind::Stop,
            StopKind::SendEnter,
            StopKind::Cancel,
            StopKind::CancelToHistory,
        ] {
            let value = serde_json::to_value(kind).unwrap();
            let back: StopKind = serde_json::from_value(value).unwrap();
            assert_eq!(back, kind);
        }
    }

    #[test]
    fn transcribing_started_frame_with_stop_kind_round_trips() {
        let frame = EngineFrame::base(
            EngineSchemaName::TranscribingStarted,
            "stop:1:send_enter",
            Some("s1".to_string()),
            "t1",
            Some(EnginePayload::TranscribingStarted {
                stop_kind: Some(StopKind::SendEnter),
            }),
            None,
        );
        assert!(validate_frame(&frame).is_ok());
        let value = serde_json::to_value(&frame).unwrap();
        // stop_kind must actually be on the wire when Some(..).
        assert_eq!(
            value["payload"]["stop_kind"],
            serde_json::json!("send_enter")
        );
        let round_tripped = validate_engine_frame(&value).unwrap();
        assert_eq!(round_tripped, frame);
    }

    #[test]
    fn legacy_transcribing_started_frame_without_stop_kind_still_parses() {
        // Simulates a frame from an OLDER binary that predates the `stop_kind`
        // field entirely — the JSON simply doesn't have the key. This MUST
        // still deserialize (backward compatibility is the invariant for
        // this pass) with stop_kind defaulting to None.
        let legacy_json = serde_json::json!({
            "protocol_major": 1, "protocol_minor": 0,
            "schema_name": "transcribing_started", "schema_version": 1,
            "engine_version": "x", "request_id": "stop:1:stop", "trace_id": "t1",
            "session_id": "s1",
            "payload": { "kind": "transcribing_started" }
        });
        let frame = validate_engine_frame(&legacy_json).unwrap();
        match frame.payload {
            Some(EnginePayload::TranscribingStarted { stop_kind }) => {
                assert_eq!(stop_kind, None);
            }
            other => panic!("expected TranscribingStarted payload, got {other:?}"),
        }
    }

    #[test]
    fn diagnostics_payload_round_trips_delivery_shape() {
        let payload = DiagnosticsPayload {
            delivery_record: Some(serde_json::json!({"delivery_id": "d1"})),
            ..Default::default()
        };
        let value = serde_json::to_value(&payload).unwrap();
        // Only the populated field should be on the wire — the rest are
        // skipped via skip_serializing_if, keeping old readers unaffected.
        assert_eq!(value.as_object().unwrap().len(), 1);
        let back: DiagnosticsPayload = serde_json::from_value(value).unwrap();
        assert_eq!(back, payload);
    }

    #[test]
    fn diagnostics_payload_round_trips_transcript_shape() {
        let payload = DiagnosticsPayload {
            shell_delivery: Some(true),
            send_enter: Some(true),
            raw_text: Some("raw hello".into()),
            recording_ms: Some(1234),
            ..Default::default()
        };
        let value = serde_json::to_value(&payload).unwrap();
        let back: DiagnosticsPayload = serde_json::from_value(value).unwrap();
        assert_eq!(back, payload);
    }

    #[test]
    fn diagnostics_payload_round_trips_command_failed_shape() {
        let payload = DiagnosticsPayload {
            reset_to_armed: Some(true),
            command_failed: Some("dispatch failed".into()),
            ..Default::default()
        };
        let value = serde_json::to_value(&payload).unwrap();
        let back: DiagnosticsPayload = serde_json::from_value(value).unwrap();
        assert_eq!(back, payload);
    }

    #[test]
    fn legacy_diagnostics_json_map_without_typed_fields_still_parses() {
        // A frame from an OLDER engine build that still sends a bespoke
        // json!({...}) map instead of the typed struct. All fields are
        // optional/defaulted, so this must parse into a DiagnosticsPayload
        // with the matching fields populated and everything else None.
        let legacy = serde_json::json!({ "reset_to_armed": true, "command_failed": "oops" });
        let parsed: DiagnosticsPayload = serde_json::from_value(legacy).unwrap();
        assert_eq!(parsed.reset_to_armed, Some(true));
        assert_eq!(parsed.command_failed, Some("oops".to_string()));
        assert_eq!(parsed.delivery_record, None);
    }

    #[test]
    fn transcript_final_frame_with_typed_diagnostics_round_trips() {
        let diagnostics = serde_json::to_value(DiagnosticsPayload {
            shell_delivery: Some(true),
            send_enter: Some(false),
            raw_text: Some("raw".into()),
            recording_ms: Some(500),
            ..Default::default()
        })
        .unwrap();
        let frame = EngineFrame::base(
            EngineSchemaName::TranscriptFinal,
            "r1",
            Some("s1".to_string()),
            "t1",
            Some(EnginePayload::TranscriptFinal {
                text: "hello".into(),
                confidence: Some(1.0),
                diagnostics: Some(diagnostics),
            }),
            None,
        );
        assert!(validate_frame(&frame).is_ok());
        let value = serde_json::to_value(&frame).unwrap();
        let round_tripped = validate_engine_frame(&value).unwrap();
        assert_eq!(round_tripped, frame);
    }
}
