loop {
    let probe_result = match asr_executor.try_recv() {
        Ok(Some(result)) => result,
        Ok(None) => break,
        Err(error) => {
            pending_main_asr_control_result = None;
            if latch_probe_lane_failure(&mut asr_executor_failed) {
                tracing::error!(error, "ASR executor unavailable");
            }
            break;
        }
    };
    #[cfg(test)]
    replay_note_probe_processed(probe_result.request.total_samples, probe_result.probe_ms);
    let same_recording = probe_result.request.generation == recording_generation
        && recording_session.as_deref() == Some(probe_result.request.session_id.as_str());
    if !probe_result_is_current(
        &probe_result,
        recording_generation,
        recording_session.as_deref(),
        kws_stream_generation,
        kws_native_origin_sample,
        buffer.len(),
        decision_clock.now(),
    ) {
        // A stale cursor, origin, generation, or lagging result may have left
        // Sherpa latched. Invalidate both scheduler & native stream before
        // accepting more audio; do not let this result affect a newer session.
        if same_recording {
            kws_stream_generation = kws_stream_generation.wrapping_add(1);
            kws_native_origin_sample = None;
            pending_control_prefix_since = None;
            asr_executor.reset_probe_stream();
        }
        trace_command_probe(CommandProbeLog {
            lane: probe_result.request.lane,
            session_id: Some(&probe_result.request.session_id),
            probe_ms: probe_result.probe_ms,
            recognized_text: "",
            classifier_result: "stale_result_dropped",
            speech_start_sample: probe_result.request.speech_start_sample,
            command_start: probe_result.request.command_start,
            command_samples: probe_result.request.audio.len(),
            total_samples: probe_result.request.total_samples,
            pending_prefix: probe_result.request.pending_control_prefix,
            error: None,
        });
        continue;
    }

    let text = match probe_result.transcript {
        Ok(text) => text,
        Err(error) => {
            kws_stream_generation = kws_stream_generation.wrapping_add(1);
            kws_native_origin_sample = None;
            pending_control_prefix_since = None;
            asr_executor.reset_probe_stream();
            trace_command_probe(CommandProbeLog {
                lane: probe_result.request.lane,
                session_id: Some(&probe_result.request.session_id),
                probe_ms: probe_result.probe_ms,
                recognized_text: "",
                classifier_result: "probe_error",
                speech_start_sample: probe_result.request.speech_start_sample,
                command_start: probe_result.request.command_start,
                command_samples: probe_result.request.audio.len(),
                total_samples: probe_result.request.total_samples,
                pending_prefix: probe_result.request.pending_control_prefix,
                error: Some(error.as_str()),
            });
            continue;
        }
    };
    let trimmed = text.trim();
    // Only Sherpa's anchored full-phrase result may authorize STOP, SEND, or
    // CANCEL. Main-ASR text below can still keep Stage 2 armed, but it cannot
    // become an action through exact or fuzzy parsing.
    let control = probe_result.native_control_intent.map(|intent| {
        heardright_core::text_pipeline::ControlCommand {
            clean_text: String::new(),
            wake_word: "zephyr".into(),
            verb: match intent {
                heardright_core::text_pipeline::ControlIntent::Stop => "stop",
                heardright_core::text_pipeline::ControlIntent::Send => "send",
                heardright_core::text_pipeline::ControlIntent::Cancel => "cancel",
            }
            .into(),
            intent,
        }
    });
    let saw_control_candidate = heardright_core::text_pipeline::has_trailing_control_wake(trimmed)
        || tail_text_ends_with_control_verb(trimmed);
    if should_keep_control_candidate_armed(
        control.is_some(),
        saw_control_candidate,
    ) {
        pending_control_prefix_since.get_or_insert_with(|| decision_clock.now());
    } else {
        pending_control_prefix_since = None;
    }

    if let Some(control) = control {
        trace_command_probe(CommandProbeLog {
            lane: probe_result.request.lane,
            session_id: recording_session.as_deref(),
            probe_ms: probe_result.probe_ms,
            recognized_text: trimmed,
            classifier_result: "control_complete_sherpa",
            speech_start_sample: probe_result.request.speech_start_sample,
            command_start: probe_result.request.command_start,
            command_samples: probe_result.request.audio.len(),
            total_samples: probe_result.request.total_samples,
            pending_prefix: probe_result.request.pending_control_prefix,
            error: None,
        });
        auto_stop_fired = true;
        let final_generation = recording_generation;
        recording_generation = recording_generation.wrapping_add(1);
        asr_executor.clear_pending();
        if let Some(sid) = recording_session.take() {
            recording = false;
            if let Some(cap) = &mut capture {
                if let Ok(extra) = cap.read_f32(usize::MAX) {
                    buffer.extend_from_slice(&extra);
                }
                cap.pause();
            }
            use heardright_core::text_pipeline::ControlIntent;
            if matches!(control.intent, ControlIntent::Cancel) {
                asr_executor.cancel_recording(final_generation);
                owner_diagnostics::capture_audio_parts(&sid, &buffer, &[]);
                trace_recording_stop(
                    Some(&sid),
                    "zephyr_cancel",
                    buffer.len(),
                    heard_voice,
                    speech_start_sample,
                    None,
                    false,
                );
                let _ = event_tx.send(WorkerEvent::RunawayDiscard { session_id: sid });
            } else {
                let _ = event_tx.send(WorkerEvent::AutoStopBegin {
                    session_id: sid.clone(),
                    send_enter: matches!(control.intent, ControlIntent::Send),
                });
                let intent = control.intent;
                trace_recording_stop(
                    Some(&sid),
                    if matches!(intent, ControlIntent::Send) {
                        "zephyr_send"
                    } else {
                        "zephyr_stop"
                    },
                    buffer.len(),
                    heard_voice,
                    speech_start_sample,
                    None,
                    matches!(intent, ControlIntent::Send),
                );
                // Cut the buffer at the trigger word's onset (minus a small
                // guard pad) BEFORE the final decode, so it never has to
                // re-transcribe audio already classified as the command
                // itself and then strip it back out as a string. Read the
                // probe's report before `buffer` is mutated/cleared below;
                // `owner_diagnostics::capture_audio_parts` further down still
                // gets the untouched, full `buffer` — only the slice handed
                // to the decoder is cut.
                let trigger_cut = resolve_trigger_audio_cut(
                    probe_result.trigger_onset_sample,
                    probe_result.trigger_onset_skip_reason,
                    speech_start_sample,
                    buffer.len(),
                    TRIGGER_CUT_GUARD_PAD_SAMPLES,
                );
                let decode_end = match trigger_cut {
                    TriggerAudioCut::Cut {
                        onset_sample,
                        cut_at,
                    } => {
                        trace_trigger_audio_cut(
                            &sid,
                            onset_sample,
                            TRIGGER_CUT_GUARD_PAD_SAMPLES,
                            cut_at,
                            buffer.len(),
                        );
                        cut_at
                    }
                    TriggerAudioCut::Skipped { reason } => {
                        trace_trigger_audio_cut_skipped(Some(&sid), reason);
                        buffer.len()
                    }
                    TriggerAudioCut::EmptyPreCommand { onset_sample } => {
                        trace_trigger_audio_cut(
                            &sid,
                            onset_sample,
                            TRIGGER_CUT_GUARD_PAD_SAMPLES,
                            0,
                            buffer.len(),
                        );
                        0
                    }
                };
                let (result, asr_elapsed_ms) = if decode_end == 0 {
                    (Ok(String::new()), None)
                } else {
                    let started = Instant::now();
                    (
                        asr_executor
                            .transcribe_final(final_generation, buffer[..decode_end].to_vec()),
                        Some(started.elapsed().as_millis() as u64),
                    )
                };
                let result = result.map(|full| {
                    let stripped = heardright_core::text_pipeline::strip_fired_control_tail(
                        &full, intent,
                    );
                    owner_diagnostics::record_event(json!({
                        "event": "final_transcript",
                        "session_id": &sid,
                        "stop_reason": if matches!(intent, ControlIntent::Send) {
                            "zephyr_send"
                        } else {
                            "zephyr_stop"
                        },
                        "raw_transcript": &full,
                        "stripped_transcript": &stripped,
                    }));
                    tracing::info!(
                        raw = command_log_text(full.trim()),
                        stripped = command_log_text(stripped.trim()),
                        "zephyr tail: strip"
                    );
                    stripped
                });
                owner_diagnostics::capture_audio_parts(&sid, &buffer, &[]);
                let vad_aggregate = speech_vad.recording_aggregate();
                let _ = event_tx.send(WorkerEvent::AutoStop {
                    session_id: sid,
                    result,
                    audio_secs: Some(audio_secs(&buffer)),
                    send_enter: matches!(intent, ControlIntent::Send),
                    stage_metrics: RecordingStageMetrics {
                        vad_observed_frame_count: vad_aggregate.map(|value| value.0),
                        vad_speech_frame_count: vad_aggregate.map(|value| value.1),
                        asr_elapsed_ms,
                        polish_elapsed_ms: None,
                    },
                });
            }
            buffer.clear();
            scheduled_submitted_samples = 0;
            voice_ema = 0.0;
            heard_voice = false;
            speech_start_sample = None;
            pending_command_prefix_since = None;
            pending_control_prefix_since = None;
        }
        continue;
    }

    if saw_control_candidate || probe_result.request.diagnostic_sample {
        trace_command_probe(CommandProbeLog {
            lane: probe_result.request.lane,
            session_id: recording_session.as_deref(),
            probe_ms: probe_result.probe_ms,
            recognized_text: trimmed,
            classifier_result: if saw_control_candidate {
                "control_prefix"
            } else {
                "none"
            },
            speech_start_sample: probe_result.request.speech_start_sample,
            command_start: probe_result.request.command_start,
            command_samples: probe_result.request.audio.len(),
            total_samples: probe_result.request.total_samples,
            pending_prefix: probe_result.request.pending_control_prefix,
            error: None,
        });
    }
    continue;
}
