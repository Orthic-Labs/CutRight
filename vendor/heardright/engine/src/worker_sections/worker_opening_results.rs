loop {
    let opening_result = if let Some(result) = pending_main_asr_control_result.take() {
        result
    } else {
        match asr_executor.try_recv_opening() {
            Ok(Some(result)) => result,
            Ok(None) => break,
            Err(error) => {
                if latch_probe_lane_failure(&mut asr_executor_failed) {
                    tracing::error!(error, "main ASR opening lane unavailable");
                }
                break;
            }
        }
    };
    if !opening_result_is_current(
        &opening_result,
        recording_generation,
        recording_session.as_deref(),
    ) {
        pending_opening_action = None;
        trace_command_probe(CommandProbeLog {
            lane: "opening_command_main_asr",
            session_id: Some(&opening_result.request.session_id),
            probe_ms: opening_result.decode_ms,
            recognized_text: "",
            classifier_result: "stale_result_dropped",
            speech_start_sample: opening_result.request.speech_start_sample,
            command_start: opening_result.request.command_start,
            command_samples: opening_result.request.audio.len(),
            total_samples: opening_result.request.total_samples,
            pending_prefix: opening_result.request.pending_prefix,
            error: None,
        });
        continue;
    }

    let text = match opening_result.transcript {
        Ok(text) => text,
        Err(error) => {
            pending_opening_action = None;
            trace_command_probe(CommandProbeLog {
                lane: "opening_command_main_asr",
                session_id: Some(&opening_result.request.session_id),
                probe_ms: opening_result.decode_ms,
                recognized_text: "",
                classifier_result: "probe_error",
                speech_start_sample: opening_result.request.speech_start_sample,
                command_start: opening_result.request.command_start,
                command_samples: opening_result.request.audio.len(),
                total_samples: opening_result.request.total_samples,
                pending_prefix: opening_result.request.pending_prefix,
                error: Some(error.as_str()),
            });
            continue;
        }
    };
    let trimmed = text.trim();
    if opening_result.request.fallback_after_kws_sequence.is_some() {
        pending_main_asr_control_result = None;
        trace_command_probe(CommandProbeLog {
            lane: "opening_command_main_asr",
            session_id: recording_session.as_deref(),
            probe_ms: opening_result.decode_ms,
            recognized_text: trimmed,
            classifier_result: "main_asr_control_fallback_disabled",
            speech_start_sample: opening_result.request.speech_start_sample,
            command_start: opening_result.request.command_start,
            command_samples: opening_result.request.audio.len(),
            total_samples: opening_result.request.total_samples,
            pending_prefix: false,
            error: None,
        });
        continue;
    }
    let classification =
        crate::command_classify::classify_streaming(trimmed, crate::settings::is_pro());
    let should_fire = match &classification {
        CommandClassification::Complete(action) => opening_action_confirmation(
            &mut pending_opening_action,
            &normalized_opening_action_identity(action),
            opening_result.request.total_samples,
            opening_result.request.pause_ready,
            false,
        ),
        CommandClassification::AmbiguousComplete(action) => opening_action_confirmation(
            &mut pending_opening_action,
            &normalized_opening_action_identity(action),
            opening_result.request.total_samples,
            opening_result.request.pause_ready,
            true,
        ),
        CommandClassification::Prefix | CommandClassification::None => {
            pending_opening_action = None;
            false
        }
    };
    let classifier_result = match &classification {
        CommandClassification::Complete(_) if should_fire => "complete_confirmed",
        CommandClassification::Complete(_) => "complete_awaiting_confirmation",
        CommandClassification::AmbiguousComplete(_) if should_fire => "ambiguous_confirmed",
        CommandClassification::AmbiguousComplete(_) => "ambiguous_awaiting_confirmation",
        CommandClassification::Prefix => "prefix",
        CommandClassification::None if trimmed.is_empty() => "empty",
        CommandClassification::None => "none",
    };
    trace_command_probe(CommandProbeLog {
        lane: "opening_command_main_asr",
        session_id: recording_session.as_deref(),
        probe_ms: opening_result.decode_ms,
        recognized_text: trimmed,
        classifier_result,
        speech_start_sample: opening_result.request.speech_start_sample,
        command_start: opening_result.request.command_start,
        command_samples: opening_result.request.audio.len(),
        total_samples: opening_result.request.total_samples,
        pending_prefix: opening_result.request.pending_prefix,
        error: None,
    });

    if should_fire {
        pending_opening_action = None;
        checked_this_pause = true;
        pending_command_prefix_since = None;
        auto_stop_fired = true;
        let canceled_generation = recording_generation;
        recording_generation = recording_generation.wrapping_add(1);
        asr_executor.clear_pending();
        asr_executor.cancel_recording(canceled_generation);
        if let Some(sid) = recording_session.take() {
            recording = false;
            let diagnostic_tail = if let Some(cap) = &mut capture {
                let tail = cap.read_f32(usize::MAX).unwrap_or_default();
                cap.pause();
                tail
            } else {
                Vec::new()
            };
            let _ = event_tx.send(WorkerEvent::AutoStopBegin {
                session_id: sid.clone(),
                send_enter: false,
            });
            let stop_reason =
                if matches!(classification, CommandClassification::AmbiguousComplete(_)) {
                    "standalone_ambiguous_command"
                } else {
                    "standalone_command"
                };
            trace_recording_stop(
                Some(&sid),
                stop_reason,
                buffer.len(),
                heard_voice,
                speech_start_sample,
                None,
                false,
            );
            owner_diagnostics::capture_audio_parts(&sid, &buffer, &diagnostic_tail);
            owner_diagnostics::record_event(json!({
                "event": "final_transcript",
                "session_id": &sid,
                "stop_reason": stop_reason,
                "raw_transcript": &text,
                "stripped_transcript": &text,
            }));
            let vad_aggregate = speech_vad.recording_aggregate();
            let _ = event_tx.send(WorkerEvent::AutoStop {
                session_id: sid,
                result: Ok(text),
                audio_secs: Some(audio_secs(&buffer)),
                send_enter: false,
                stage_metrics: RecordingStageMetrics {
                    vad_observed_frame_count: vad_aggregate.map(|value| value.0),
                    vad_speech_frame_count: vad_aggregate.map(|value| value.1),
                    asr_elapsed_ms: None,
                    polish_elapsed_ms: None,
                },
            });
            buffer.clear();
            scheduled_submitted_samples = 0;
            voice_ema = 0.0;
            heard_voice = false;
            speech_start_sample = None;
            pending_control_prefix_since = None;
        }
        continue;
    }

    match classification {
        CommandClassification::Complete(_)
        | CommandClassification::AmbiguousComplete(_)
        | CommandClassification::Prefix => {
            mark_opening_prefix_result_at(
                &mut checked_this_pause,
                &mut pending_command_prefix_since,
                decision_clock.now(),
            );
        }
        CommandClassification::None if !trimmed.is_empty() => {
            if pause_can_close_command_probe(heard_voice, opening_result.request.pause_ready) {
                checked_this_pause = true;
                pending_command_prefix_since = None;
            }
        }
        CommandClassification::None => {}
    }
}
