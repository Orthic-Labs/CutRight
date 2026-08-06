{
        loop {
            match asr_executor.try_recv_idle_wake() {
                Ok(Some(result)) => match result.hit {
                    Ok(Some((_onset, end_sample))) if wake_listening => {
                        wake_listening = false;
                        wake_start_pending = true;
                        wake_fire_count = wake_fire_count.saturating_add(1);
                        asr_executor.stop_idle_wake();
                        let handoff_start = end_sample.saturating_add(SAMPLE_RATE as usize / 25);
                        let offset = handoff_start.saturating_sub(idle_audio_origin);
                        wake_handoff_audio = idle_audio_window.iter().skip(offset).copied().collect();
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs_f64();
                        tracing::info!(
                            total_samples = result.total_samples,
                            decode_ms = result.decode_ms,
                            fire_count = wake_fire_count,
                            "idle wake fired"
                        );
                        if let Some(diagnostics) = wake_diagnostics.as_mut() {
                            diagnostics.observe_decode(result.decode_ms);
                            diagnostics.fired(wake_fire_count);
                            diagnostics.stopped(wake_fire_count);
                        }
                        wake_diagnostics = None;
                        let _ = event_tx.send(WorkerEvent::WakeFired {
                            ts,
                            fire_count: wake_fire_count,
                            threshold: wake_threshold,
                        });
                    }
                    Ok(_) => {
                        if let Some(diagnostics) = wake_diagnostics.as_mut() {
                            diagnostics.observe_decode(result.decode_ms);
                        }
                    }
                    Err(error) if wake_listening => {
                        tracing::error!(error, "idle wake decode failed");
                        if let Some(diagnostics) = wake_diagnostics.as_mut() {
                            diagnostics.failed("kws_decode_error");
                            diagnostics.stopped(wake_fire_count);
                        }
                        wake_diagnostics = None;
                        wake_listening = false;
                        asr_executor.stop_idle_wake();
                        if let Some(cap) = &capture {
                            cap.pause();
                        }
                    }
                    Err(_) => {}
                },
                Ok(None) => break,
                Err(error) => {
                    tracing::error!(error, "idle wake result lane failed");
                    if let Some(diagnostics) = wake_diagnostics.as_mut() {
                        diagnostics.failed("kws_lane_error");
                        diagnostics.stopped(wake_fire_count);
                    }
                    wake_diagnostics = None;
                    wake_listening = false;
                    break;
                }
            }
        }

        if wake_listening || wake_start_pending {
            if let Some(cap) = &mut capture {
                cap.wait_for_audio(CAPTURE_POLL);
                match cap.read_f32(usize::MAX) {
                    Ok(chunk) if !chunk.is_empty() => {
                        if wake_listening {
                            if let Some(diagnostics) = wake_diagnostics.as_mut() {
                                diagnostics.observe_audio(&chunk);
                            }
                            let start_sample = idle_sample_clock;
                            idle_sample_clock = idle_sample_clock.saturating_add(chunk.len());
                            idle_audio_window.extend(chunk.iter().copied());
                            let capacity = SAMPLE_RATE as usize * 4;
                            while idle_audio_window.len() > capacity {
                                idle_audio_window.pop_front();
                                idle_audio_origin = idle_audio_origin.saturating_add(1);
                            }
                            asr_executor.submit_idle_wake(
                                chunk,
                                start_sample,
                                idle_sample_clock,
                            );
                        } else {
                            wake_handoff_audio.extend_from_slice(&chunk);
                            let capacity = SAMPLE_RATE as usize * 4;
                            if wake_handoff_audio.len() > capacity {
                                let excess = wake_handoff_audio.len() - capacity;
                                wake_handoff_audio.drain(..excess);
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(error) => {
                        tracing::error!(%error, "idle wake capture read failed");
                        if let Some(diagnostics) = wake_diagnostics.as_mut() {
                            diagnostics.failed("capture_read_error");
                            diagnostics.stopped(wake_fire_count);
                        }
                        wake_diagnostics = None;
                        wake_listening = false;
                        wake_start_pending = false;
                        asr_executor.stop_idle_wake();
                        capture = None;
                    }
                }
            }
        }

        if recording {
            if let Some(cap) = &mut capture {
                cap.wait_for_audio(CAPTURE_POLL);
                match cap.read_f32(usize::MAX) {
                    Ok(chunk) if !chunk.is_empty() => {
                        let previous_for_validation = if cap.is_replay() {
                            None
                        } else {
                            previous_capture_block.as_ref()
                        };
                        let validation = match validate_capture_pcm(&chunk, previous_for_validation) {
                            Ok(validation) => validation,
                            Err(error) => {
                                tracing::error!(error, "capture ingress rejected");
                                cap.pause();
                                drop_failed_capture(&mut capture);
                                if let Some(sid) = recording_session.take() {
                                    let _ = event_tx.send(WorkerEvent::CaptureFailed {
                                        session_id: sid,
                                        message: error.into(),
                                    });
                                }
                                let canceled_generation = recording_generation;
                                recording_generation = recording_generation.wrapping_add(1);
                                asr_executor.clear_pending();
                                asr_executor.cancel_recording(canceled_generation);
                                recording = false;
                                pending_opening_action = None;
                                buffer.clear();
                                previous_capture_block = None;
                                heard_voice = false;
                                speech_start_sample = None;
                                continue;
                            }
                        };
                        let discarded_repeat = validation.discarded_repeat();
                        previous_capture_block = Some(validation.history());
                        if discarded_repeat {
                            tracing::warn!("capture ingress discarded one repeated block");
                            continue;
                        }
                        voice_ema = voice_ema * 0.6 + chunk_level(&chunk) * 0.4;
                        observe_voice_chunk_at(
                            &chunk,
                            &mut last_voice_at,
                            decision_clock.now(),
                            &mut heard_voice,
                            &mut checked_this_pause,
                            |samples| confirm_first_speech(samples, &mut speech_vad),
                            &mut speech_start_sample,
                            buffer.len(),
                        );
                        if last_level_emit.elapsed() >= Duration::from_millis(100) {
                            last_level_emit = Instant::now();
                            if let Some(sid) = &recording_session {
                                let _ = event_tx.send(WorkerEvent::RecordingLevel {
                                    session_id: sid.clone(),
                                    level: voice_ema.clamp(0.0, 1.0),
                                });
                            }
                        }
                        buffer.extend_from_slice(&chunk);
                        audio_arrived_this_loop = true;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("capture read error: {}", e);
                        cap.pause();
                        if let Some(sid) = recording_session.take() {
                            let _ = event_tx.send(WorkerEvent::CaptureFailed {
                                session_id: sid,
                                message: format!("capture read: {e}"),
                            });
                        }
                        let canceled_generation = recording_generation;
                        recording_generation = recording_generation.wrapping_add(1);
                        asr_executor.clear_pending();
                        asr_executor.cancel_recording(canceled_generation);
                        recording = false;
                        pending_opening_action = None;
                        buffer.clear();
                        heard_voice = false;
                        speech_start_sample = None;
                        kws_stream_generation = kws_stream_generation.wrapping_add(1);
                        kws_native_origin_sample = None;
                        asr_executor.reset_probe_stream();
                        capture = None;
                    }
                }
            }
            let decision_now = decision_clock.now();
            if last_focus_refresh.is_none_or(|last_refresh| {
                decision_now.duration_since(last_refresh)
                    >= Duration::from_millis(FOCUS_REFRESH_THROTTLE_MS)
            }) {
                last_focus_refresh = Some(decision_now);
                focus.lock().refresh_current();
            }

            if pending_control_prefix_since
                .is_some_and(|since| {
                    decision_now.duration_since(since)
                        >= Duration::from_millis(TAIL_PREFIX_GRACE_MS)
                })
            {
                pending_control_prefix_since = None;
            }
            if pending_command_prefix_since
                .is_some_and(|since| {
                    decision_now.duration_since(since)
                        > Duration::from_millis(CMD_PREFIX_GRACE_MS)
                })
            {
                trace_command_probe(CommandProbeLog {
                    lane: "unified_command_trigger",
                    session_id: recording_session.as_deref(),
                    probe_ms: 0,
                    recognized_text: "",
                    classifier_result: "prefix_timeout",
                    speech_start_sample,
                    command_start: command_probe_start_sample(speech_start_sample, buffer.len())
                        .min(buffer.len()),
                    command_samples: 0,
                    total_samples: buffer.len(),
                    pending_prefix: true,
                    error: None,
                });
                pending_command_prefix_since = None;
                checked_this_pause = true;
            }

            let vad_ready = speech_vad.is_ready();
            let command_start =
                command_probe_start_sample(speech_start_sample, buffer.len()).min(buffer.len());
            let command_audio_len = buffer.len().saturating_sub(command_start);
            let pause_ready = vad_ready
                && decision_now.duration_since(last_voice_at) >= Duration::from_millis(PAUSE_GATE_MS);
            let control_fallback_due = main_asr_control_fallback_due(
                checked_this_pause,
                heard_voice,
                command_audio_len > CMD_WINDOW_SAMPLES,
                pause_ready,
            );
            let opening_due = control_fallback_due || (!checked_this_pause
                && opening_command_due_at(
                    heard_voice,
                    pending_command_prefix_since.is_some(),
                    pause_ready,
                    command_audio_len,
                    last_command_probe_samples,
                    last_cmd_check,
                    decision_now,
                ));
            let pending_control_prefix = pending_control_prefix_since.is_some();
            let tail_due = buffer.len() > SAMPLE_RATE as usize / 4 && audio_arrived_this_loop;

            if !auto_stop_fired
                && !asr_executor_failed
                && !buffer.is_empty()
                && (tail_due || opening_due)
                && probe_gate_clear(&cmd_rx, &mut pending_cmd_peek)
            {
                if opening_due {
                    let opening_start = if control_fallback_due {
                        tail_probe_start_sample(buffer.len(), TailProbeLane::Full)
                    } else {
                        command_start
                    };
                    last_cmd_check = Some(decision_now);
                    last_command_probe_samples = buffer.len().saturating_sub(opening_start);
                    mark_pause_probe_submitted(&mut checked_this_pause, pause_ready);
                    if let Some(session_id) = recording_session.clone() {
                        asr_executor.submit_opening(OpeningCommandRequest {
                            generation: recording_generation,
                            session_id,
                            submitted_at: decision_now,
                            created_at: Instant::now(),
                            audio: buffer[opening_start..].to_vec(),
                            speech_start_sample,
                            command_start: opening_start,
                            total_samples: buffer.len(),
                            pause_ready,
                            pending_prefix: pending_command_prefix_since.is_some()
                                || pending_control_prefix_since.is_some(),
                            fallback_after_kws_sequence: control_fallback_due
                                .then(|| kws_probe_count.wrapping_add(1)),
                        });
                    }
                }

                if tail_due {
                    #[cfg(target_os = "windows")]
                    let use_full_tail = should_run_full_tail_fallback(
                        "",
                        pending_control_prefix,
                        buffer.len().saturating_sub(last_full_tail_probe_samples),
                    );
                    #[cfg(not(target_os = "windows"))]
                    let use_full_tail = false;

                    let tail_lane = if use_full_tail {
                        TailProbeLane::Full
                    } else {
                        TailProbeLane::Fast
                    };
                    let snapshot_start = tail_probe_start_sample(buffer.len(), tail_lane);
                    let lane = if use_full_tail {
                        "zephyr_tail_full_async"
                    } else {
                        "zephyr_tail_async"
                    };

                    #[cfg(target_os = "windows")]
                    if use_full_tail {
                        last_full_tail_probe_samples = buffer.len();
                    }
                    if let Some(session_id) = recording_session.clone() {
                        kws_probe_count = kws_probe_count.wrapping_add(1);
                        let native_origin_sample = snapshot_start;
                        kws_native_origin_sample = Some(native_origin_sample);
                        asr_executor.submit_probe(ProbeRequest {
                            generation: recording_generation,
                            kws_stream_generation,
                            session_id,
                            submitted_at: decision_now,
                            created_at: Instant::now(),
                            audio: buffer[snapshot_start..].to_vec(),
                            speech_start_sample,
                            command_start: snapshot_start,
                            stream_base_sample: snapshot_start,
                            native_origin_sample,
                            total_samples: buffer.len(),
                            pending_control_prefix,
                            lane,
                            probe_sequence: kws_probe_count,
                            diagnostic_sample: kws_probe_count == 1
                                || kws_probe_count % KWS_DIAGNOSTIC_SAMPLE_EVERY_PROBES == 0,
                        });
                        #[cfg(test)]
                        replay_note_probe_submitted(buffer.len());
                    }
                }
            }

            if asr_executor.scheduled_static15_enabled() {
                if let Some(segment) =
                    scheduled_static15_ready_segment(&buffer, scheduled_submitted_samples)
                {
                    let start_sample = segment.start;
                    let end_sample = segment.end;
                    asr_executor.submit_background(
                        recording_generation,
                        start_sample,
                        end_sample,
                        buffer[segment].to_vec(),
                    );
                    scheduled_submitted_samples = end_sample;
                }
            }

            if vad_ready && recording
                && !auto_stop_fired
                && silence_decision(heard_voice, decision_now.duration_since(last_voice_at))
                    == SilenceDecision::DiscardInitialSilence
            {
                tracing::warn!("initial-silence backstop: discarding recording as a false fire");
                auto_stop_fired = true;
                let canceled_generation = recording_generation;
                recording_generation = recording_generation.wrapping_add(1);
                asr_executor.clear_pending();
                asr_executor.cancel_recording(canceled_generation);
                if let Some(sid) = recording_session.take() {
                    recording = false;
                    if let Some(cap) = &capture {
                        cap.pause();
                    }
                    trace_recording_stop(
                        Some(&sid),
                        "initial_silence_discard",
                        buffer.len(),
                        heard_voice,
                        speech_start_sample,
                        Some(decision_now.duration_since(last_voice_at).as_millis() as u64),
                        false,
                    );
                    let _ = event_tx.send(WorkerEvent::RunawayDiscard { session_id: sid });
                    buffer.clear();
                    voice_ema = 0.0;
                    heard_voice = false;
                    speech_start_sample = None;
                }
            }

            let duration_cap_hit = recording_duration_cap_exceeded(buffer.len());
            if vad_ready && recording
                && !auto_stop_fired
                && (silence_decision(heard_voice, decision_now.duration_since(last_voice_at))
                    == SilenceDecision::StopAfterPostSpeechSilence
                    || duration_cap_hit)
            {
                // A silence decision is provisional until device-ring tail is
                // consumed. A renewed word may already be queued behind the
                // last callback that VAD observed; keep this interaction live
                // if that tail re-opens speech.
                if !duration_cap_hit {
                    if let Some(cap) = &mut capture {
                        if let Ok(tail) = cap.read_f32(usize::MAX) {
                            if !tail.is_empty() {
                                observe_voice_chunk_at(
                                    &tail,
                                    &mut last_voice_at,
                                    decision_clock.now(),
                                    &mut heard_voice,
                                    &mut checked_this_pause,
                                    |samples| confirm_first_speech(samples, &mut speech_vad),
                                    &mut speech_start_sample,
                                    buffer.len(),
                                );
                                buffer.extend_from_slice(&tail);
                            }
                        }
                    }
                    if silence_decision(
                        heard_voice,
                        decision_clock.now().duration_since(last_voice_at),
                    )
                        != SilenceDecision::StopAfterPostSpeechSilence
                    {
                        continue;
                    }
                }
                let stop_reason: &'static str = if duration_cap_hit {
                    "max_duration_cap"
                } else {
                    "post_speech_silence"
                };
                if duration_cap_hit {
                    tracing::warn!(
                        total_samples = buffer.len(),
                        max_seconds = MAX_RECORDING_SECONDS,
                        "recording duration cap reached: auto-stopping and transcribing recording"
                    );
                } else {
                    tracing::info!("post-speech silence: auto-stopping and transcribing recording");
                }
                auto_stop_fired = true;
                let final_generation = recording_generation;
                recording_generation = recording_generation.wrapping_add(1);
                asr_executor.clear_pending();
                if let Some(sid) = recording_session.take() {
                    recording = false;
                    let diagnostic_tail = if let Some(cap) = &mut capture {
                        let tail = cap.read_f32(usize::MAX).unwrap_or_default();
                        cap.pause();
                        tail
                    } else {
                        Vec::new()
                    };
                    // Capture ownership does not end at silence decision: audio
                    // already queued in device ring is part of this interaction.
                    // Append it before final decode; hard cap remains exact.
                    if duration_cap_hit {
                        let remaining = MAX_RECORDING_SAMPLES.saturating_sub(buffer.len());
                        buffer.extend_from_slice(&diagnostic_tail[..diagnostic_tail.len().min(remaining)]);
                    } else {
                        buffer.extend_from_slice(&diagnostic_tail);
                    }
                    trace_recording_stop(
                        Some(&sid),
                        stop_reason,
                        buffer.len(),
                        heard_voice,
                        speech_start_sample,
                        Some(
                            decision_clock
                                .now()
                                .duration_since(last_voice_at)
                                .as_millis() as u64,
                        ),
                        false,
                    );
                    owner_diagnostics::capture_audio_parts(&sid, &buffer, &diagnostic_tail);
                    let _ = event_tx.send(WorkerEvent::AutoStopBegin {
                        session_id: sid.clone(),
                        send_enter: false,
                    });
                    let final_audio_secs = audio_secs(&buffer);
                    let final_audio = std::mem::take(&mut buffer);
                    let asr_started = Instant::now();
                    let result = asr_executor.transcribe_final(final_generation, final_audio);
                    let asr_elapsed_ms = asr_started.elapsed().as_millis() as u64;
                    let vad_aggregate = speech_vad.recording_aggregate();
                    owner_diagnostics::record_event(json!({
                        "event": "final_transcript",
                        "session_id": &sid,
                        "stop_reason": stop_reason,
                        "raw_transcript": result.as_ref().ok(),
                        "error": result.as_ref().err(),
                    }));
                    let _ = event_tx.send(WorkerEvent::AutoStop {
                        session_id: sid,
                        result,
                        audio_secs: Some(final_audio_secs),
                        send_enter: false,
                        stage_metrics: RecordingStageMetrics {
                            vad_observed_frame_count: vad_aggregate.map(|value| value.0),
                            vad_speech_frame_count: vad_aggregate.map(|value| value.1),
                            asr_elapsed_ms: Some(asr_elapsed_ms),
                            polish_elapsed_ms: None,
                        },
                    });
                    scheduled_submitted_samples = 0;
                    voice_ema = 0.0;
                    heard_voice = false;
                    speech_start_sample = None;
                }
            }
        }
}
