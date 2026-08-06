{
        // F4(b) (Sol audit 2026-07-16): a Stop/Cancel peeked (and stashed, not
        // dropped) by `probe_gate_clear` in worker_streaming.rs is dispatched
        // here FIRST, ahead of a fresh blocking `recv_timeout` — see
        // `pending_cmd_peek`'s declaration in section02.rs.
        let next_cmd = match pending_cmd_peek.take() {
            Some(cmd) => Ok(cmd),
            None if recording || wake_listening || wake_start_pending => match cmd_rx.try_recv() {
                Ok(command) => Ok(command),
                Err(std::sync::mpsc::TryRecvError::Empty) => Err(RecvTimeoutError::Timeout),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Err(RecvTimeoutError::Disconnected)
                }
            },
            None => cmd_rx.recv_timeout(CAPTURE_POLL),
        };
        match next_cmd {
            Ok(WorkerCmd::StartWakeListen { threshold, reply }) => {
                if recording {
                    let _ = reply.send(Err("cannot start wake listening while recording".into()));
                    continue;
                }
                if wake_listening {
                    let _ = reply.send(Ok(()));
                    continue;
                }
                if let Err(error) = asr_executor.start_idle_wake(threshold) {
                    WakeDiagnostics::startup_failure(
                        threshold.clamp(0.01, 0.99),
                        "wake_kws_start_error",
                    );
                    let _ = reply.send(Err(error));
                    continue;
                }
                let saved = crate::settings::input_device();
                if capture_should_reopen(capture.is_none(), saved != last_saved_device) {
                    capture = None;
                    match open_capture(saved.as_deref()) {
                        Ok((c, route)) => {
                            capture = Some(c);
                            last_capture_route = route;
                            last_saved_device = saved;
                        }
                        Err(error) => {
                            asr_executor.stop_idle_wake();
                            WakeDiagnostics::startup_failure(
                                threshold.clamp(0.01, 0.99),
                                "wake_capture_open_error",
                            );
                            let _ = reply.send(Err(error));
                            continue;
                        }
                    }
                } else if let Some(cap) = &capture {
                    if let Err(error) = cap.resume() {
                        asr_executor.stop_idle_wake();
                        WakeDiagnostics::startup_failure(
                            threshold.clamp(0.01, 0.99),
                            "wake_capture_resume_error",
                        );
                        let _ = reply.send(Err(format!("capture resume: {error}")));
                        continue;
                    }
                }
                idle_sample_clock = 0;
                idle_audio_window.clear();
                idle_audio_origin = 0;
                wake_handoff_audio.clear();
                wake_start_pending = false;
                wake_threshold = threshold.clamp(0.01, 0.99);
                wake_listening = true;
                wake_diagnostics = Some(WakeDiagnostics::started(wake_threshold));
                let _ = reply.send(Ok(()));
            }
            Ok(WorkerCmd::StopWakeListen { reply }) => {
                if let Some(diagnostics) = wake_diagnostics.as_mut() {
                    diagnostics.stopped(wake_fire_count);
                }
                wake_diagnostics = None;
                wake_listening = false;
                wake_start_pending = false;
                asr_executor.stop_idle_wake();
                if !recording {
                    if let Some(cap) = &capture {
                        cap.pause();
                    }
                }
                idle_audio_window.clear();
                wake_handoff_audio.clear();
                let _ = reply.send(wake_fire_count);
            }
            Ok(WorkerCmd::ReloadModel { reply }) => {
                // Proactive switch (settings changed) — load+warm off the hotkey
                // path so the next recording is instant. Keep the old model on
                // failure; the StartRecording fallback below surfaces any error.
                asr_executor.clear_pending();
                let result = asr_executor.reload_model();
                if result.is_ok() {
                    kws_stream_generation = kws_stream_generation.wrapping_add(1);
                    kws_native_origin_sample = None;
                    asr_executor.reset_probe_stream();
                }
                if let Err(e) = &result {
                    tracing::error!("worker ASR reload failed: {e}");
                }
                if let Some(reply) = reply {
                    let _ = reply.send(result);
                }
            }
            Ok(WorkerCmd::StartRecording { session_id }) => {
                let wake_handoff = wake_start_pending;
                if wake_listening {
                    wake_listening = false;
                    asr_executor.stop_idle_wake();
                    if let Some(cap) = &capture {
                        cap.pause();
                    }
                }
                wake_start_pending = false;
                let start_received_at = Instant::now();
                trace_recording_start(&session_id, "worker_start_received", 0);
                recording_generation = recording_generation.wrapping_add(1);
                asr_executor.clear_pending();
                pending_opening_action = None;
                loop {
                    match asr_executor.try_recv() {
                        Ok(Some(_)) => continue,
                        Ok(None) => break,
                        Err(error) => {
                            if latch_probe_lane_failure(&mut asr_executor_failed) {
                                tracing::error!(error, "ASR executor unavailable");
                            }
                            break;
                        }
                    }
                }
                buffer.clear();
                previous_capture_block = None;
                scheduled_submitted_samples = 0;
                voice_ema = 0.0;
                last_level_emit = Instant::now() - Duration::from_millis(120);
                last_voice_at = decision_clock.now();
                checked_this_pause = false;
                auto_stop_fired = false;
                pending_command_prefix_since = None;
                heard_voice = false;
                speech_start_sample = None;
                kws_stream_generation = kws_stream_generation.wrapping_add(1);
                kws_native_origin_sample = None;
                kws_probe_count = 0;
                pending_main_asr_control_result = None;
                asr_executor.reset_probe_stream();
                speech_vad.reset();
                #[cfg(target_os = "windows")]
                {
                    last_full_tail_probe_samples = 0;
                }
                pending_control_prefix_since = None;
                last_cmd_check = None;
                last_command_probe_samples = 0;
                recording_session = Some(session_id.clone());
                recording = true;
                trace_recording_start(
                    &session_id,
                    "recording_active",
                    start_received_at.elapsed().as_millis() as u64,
                );

                macro_rules! fail_recording_start {
                    ($message:expr) => {{
                        let message = $message;
                        recording = false;
                        recording_session = None;
                        buffer.clear();
                        previous_capture_block = None;
                        pending_opening_action = None;
                        pending_main_asr_control_result = None;
                        asr_executor.clear_pending();
                        recording_generation = recording_generation.wrapping_add(1);
                        if let Some(cap) = &capture {
                            cap.pause();
                        }
                        let _ = event_tx.send(WorkerEvent::CaptureFailed {
                            session_id: session_id.clone(),
                            message,
                        });
                        continue;
                    }};
                }
                // Fallback reload: normally a no-op because ReloadModel already
                // switched the model when the setting changed. Only blocks here if
                // the user records faster than the proactive reload. On failure
                // (e.g. Whisper missing) keep the model and surface the error.
                match asr_executor.reload_model() {
                    Ok(_) => {}
                    Err(e) => {
                        fail_recording_start!(format!(
                            "ASR reload ({}): {e}",
                            asr_reload_key()
                        ));
                    }
                }
                // Privacy boundary: capture stays closed until StartRecording.
                // Reuse a paused stream when possible; cold-open then pause so the
                // shared flush+resume path below never flushes while the device is
                // already playing (that raced the first syllable after the startup
                // open was removed).
                let saved = crate::settings::input_device();
                if !wake_handoff
                    && capture_should_reopen(capture.is_none(), saved != last_saved_device)
                {
                    capture = None;
                    match open_capture(saved.as_deref()) {
                        Ok((c, route)) => {
                            c.pause();
                            capture = Some(c);
                            last_capture_route = route;
                            last_saved_device = saved.clone();
                        }
                        Err(e) => {
                            fail_recording_start!(e);
                        }
                    }
                }
                // Clear stale samples while the stream is paused. Flushing after
                // resume raced the first fresh callback and could discard a few
                // milliseconds from an immediate leading phoneme.
                if !wake_handoff {
                    if let Some(cap) = &mut capture {
                        let discarded = cap.flush();
                        if discarded > 0 {
                            tracing::info!("flushed {discarded} stale input samples before record");
                        }
                    }
                }
                if !wake_handoff {
                    if let Some(cap) = &capture {
                        if let Err(e) = cap.resume() {
                            fail_recording_start!(format!("capture resume: {e}"));
                        }
                    }
                }
                trace_recording_start(
                    &session_id,
                    "capture_resumed",
                    start_received_at.elapsed().as_millis() as u64,
                );

                // Inspect the OS capture route only AFTER capture resumes. Device
                // enumeration can take hundreds of milliseconds, but audio
                // now accumulates in the ring during that work instead of being lost.
                // A real device/rate/channel/format change gets one cold reopen.
                // This covers CoreAudio call-route changes and the equivalent
                // WASAPI default/Bluetooth hands-free endpoint transition.
                if !wake_handoff {
                    match current_capture_route(saved.as_deref()) {
                    Ok(current_route) if capture_route_changed(&last_capture_route, &current_route) =>
                    {
                        // Preserve any prefix the old route delivered while the
                        // signature check ran. Shared-mode capture often contains
                        // the user's first word even when another app has just
                        // changed the device configuration.
                        if let Some(cap) = &mut capture {
                            if let Ok(prefix) = cap.read_f32(usize::MAX) {
                                buffer.extend_from_slice(&prefix);
                            }
                            cap.pause();
                        }
                        capture = None;
                        match open_capture(saved.as_deref()) {
                            Ok((mut c, route)) => {
                                c.flush();
                                if let Err(e) = c.resume() {
                                    fail_recording_start!(format!("capture route resume: {e}"));
                                }
                                capture = Some(c);
                                last_capture_route = route;
                                trace_recording_start(
                                    &session_id,
                                    "capture_route_reopened",
                                    start_received_at.elapsed().as_millis() as u64,
                                );
                            }
                            Err(e) => {
                                fail_recording_start!(e);
                            }
                        }
                    }
                    Ok(_) => {}
                        Err(e) => tracing::warn!("{e}; reusing the active capture route"),
                    }
                }
                // H6 (perf audit 2026-07-15): kick off the (AI-polish-gated,
                // Windows/macOS-only) focused-field UIA/AX probe on a
                // background thread NOW, so its up-to-140ms Chromium-lazy-a11y
                // reprobe sleep (see `focus.rs::focused_field_context`)
                // overlaps with the blocking seed-capture read right below.
                // `snapshot_at_start_finish` joins before streaming work uses
                // the captured field context.
                let focus_field_prefetch = focus.lock().snapshot_at_start_prefetch();
                // Wait for live first frames without clipping the leading word.
                // Bluetooth gets a longer bounded settle because hands-free
                // routes can publish startup zeros before real PCM.
                let mut capture_needs_reopen = false;
                let mut capture_health_outcome = "live";
                let mut capture_live = true;
                if wake_handoff {
                    buffer.append(&mut wake_handoff_audio);
                    trace_recording_start(
                        &session_id,
                        "wake_audio_handoff",
                        start_received_at.elapsed().as_millis() as u64,
                    );
                } else if let Some(cap) = &mut capture {
                    let settle_ms =
                        capture_settle_budget_ms(last_capture_route.as_ref(), false);
                    match wait_for_live_first_buffer(cap, settle_ms, decision_clock.as_ref()) {
                        Ok(first) => {
                            capture_needs_reopen =
                                !cap.is_replay() && capture_first_buffer_requires_reopen(&first);
                            match first {
                                CaptureFirstBuffer::Data(seed) if !capture_needs_reopen => {
                                    observe_voice_chunk_at(
                                        &seed,
                                        &mut last_voice_at,
                                        decision_clock.now(),
                                        &mut heard_voice,
                                        &mut checked_this_pause,
                                        |samples| confirm_first_speech(samples, &mut speech_vad),
                                        &mut speech_start_sample,
                                        buffer.len(),
                                    );
                                    buffer.extend_from_slice(&seed);
                                }
                                CaptureFirstBuffer::Data(_) => {
                                    capture_health_outcome = "capture_stream_silent_stale";
                                }
                                CaptureFirstBuffer::NoCallbacks => {
                                    capture_health_outcome = "capture_stream_no_callbacks";
                                }
                                CaptureFirstBuffer::NoSamples => {
                                    capture_health_outcome = "capture_stream_no_samples";
                                }
                                CaptureFirstBuffer::StreamError(kind) => {
                                    capture_health_outcome = kind.as_str();
                                }
                            }
                        }
                        Err(error) => {
                            capture_needs_reopen = true;
                            capture_health_outcome = "capture_stream_read_failed";
                            tracing::error!(%error, "capture first-buffer read failed");
                        }
                    }
                    let health = cap.metrics_snapshot();
                    tracing::info!(
                        target: "capture_health",
                        phase = "first_buffer",
                        outcome = capture_health_outcome,
                        repair_attempt = 0u8,
                        callbacks = health.callback_invocations,
                        input_samples = health.input_samples_received,
                        first_callback_ms = health.first_callback_latency_us.map(|us| us / 1000),
                        async_errors = health.async_error_count,
                        input_rate = cap.input_rate(),
                        channels = cap.input_channels(),
                        sample_format = cap.input_sample_format(),
                        transport = ?last_capture_route.as_ref().map(|route| route.transport),
                        form_factor = ?last_capture_route.as_ref().map(|route| route.form_factor),
                        "capture health transition"
                    );
                    trace_recording_start(
                        &session_id,
                        "seed_captured",
                        start_received_at.elapsed().as_millis() as u64,
                    );
                    // Auto-calibrate was attempted here but BREAKS in practice:
                    // on Windows WASAPI the first ~250ms of audio after the PTT
                    // key fires is typically the user starting to speak
                    // (PTT-down + first word lands in <250ms), so we use the
                    // first syllable of their command as the noise floor and
                    // then set speech_threshold = 8x of THAT, which can never
                    // be reached. VAD fires zero times after. Field bug
                    // 2026-07-06: session-1 worked (0.20 magic threshold from
                    // Silero default), session-4 broke (auto-cal picked up
                    // speech as noise). Calibration is now opt-in via the
                    // HR_VAD_AUTO_CALIBRATE env var for users who want to
                    // tune for a fixed room; the default uses the 0.20
                    // threshold proven on session-1.
                    if !capture_needs_reopen
                        && std::env::var_os("HR_VAD_AUTO_CALIBRATE").is_some()
                    {
                        if let Ok(cal) = cap.read_f32_blocking(SAMPLE_RATE as usize, 250) {
                            speech_vad.calibrate(&cal);
                            tracing::info!(
                                "vad auto-calibrated: noise_floor={:.5} speech_threshold={:.4}",
                                speech_vad.noise_floor_rms(),
                                speech_vad.speech_threshold(),
                            );
                        }
                    }
                }
                // A resumed warm stream that delivers only digital silence is
                // dead, not quiet. Give it exactly ONE cold reopen. A mute
                // switch, macOS mic privacy, and idle virtual/aggregate devices
                // also read as zeros, so a retry loop would spin forever. A
                // second failure terminates this session cleanly.
                if capture_needs_reopen {
                    trace_recording_start(
                        &session_id,
                        capture_health_outcome,
                        start_received_at.elapsed().as_millis() as u64,
                    );
                    if let Some(cap) = &mut capture {
                        cap.pause();
                    }
                    capture = None;
                    match open_capture(saved.as_deref()) {
                        Ok((mut reopened, route)) => {
                            reopened.flush();
                            match reopened.resume() {
                                Ok(()) => {
                                    last_capture_route = route;
                                    let settle_ms =
                                        capture_settle_budget_ms(last_capture_route.as_ref(), true);
                                    let reopened_first = wait_for_live_first_buffer(
                                        &mut reopened,
                                        settle_ms,
                                        decision_clock.as_ref(),
                                    );
                                    let still_dead = match reopened_first {
                                        Ok(first) => {
                                            let unhealthy =
                                                capture_first_buffer_requires_reopen(&first);
                                            if let CaptureFirstBuffer::Data(seed) = first {
                                                if !unhealthy {
                                                    observe_voice_chunk_at(
                                                        &seed,
                                                        &mut last_voice_at,
                                                        decision_clock.now(),
                                                        &mut heard_voice,
                                                        &mut checked_this_pause,
                                                        |samples| {
                                                            confirm_first_speech(
                                                                samples,
                                                                &mut speech_vad,
                                                            )
                                                        },
                                                        &mut speech_start_sample,
                                                        buffer.len(),
                                                    );
                                                    buffer.extend_from_slice(&seed);
                                                }
                                            }
                                            unhealthy
                                        }
                                        Err(error) => {
                                            tracing::error!(
                                                %error,
                                                "capture first-buffer read failed after reopen"
                                            );
                                            true
                                        }
                                    };
                                    capture_live = !still_dead;
                                    let health = reopened.metrics_snapshot();
                                    tracing::info!(
                                        target: "capture_health",
                                        phase = "first_buffer",
                                        outcome = if still_dead { "failed" } else { "recovered" },
                                        repair_attempt = 1u8,
                                        callbacks = health.callback_invocations,
                                        input_samples = health.input_samples_received,
                                        first_callback_ms =
                                            health.first_callback_latency_us.map(|us| us / 1000),
                                        async_errors = health.async_error_count,
                                        input_rate = reopened.input_rate(),
                                        channels = reopened.input_channels(),
                                        sample_format = reopened.input_sample_format(),
                                        transport = ?last_capture_route.as_ref().map(|route| route.transport),
                                        form_factor = ?last_capture_route.as_ref().map(|route| route.form_factor),
                                        "capture health transition"
                                    );
                                    capture = Some(reopened);
                                    trace_recording_start(
                                        &session_id,
                                        if still_dead {
                                            "capture_stream_flatline_after_reopen"
                                        } else {
                                            "capture_stream_recovered_after_reopen"
                                        },
                                        start_received_at.elapsed().as_millis() as u64,
                                    );
                                }
                                Err(e) => {
                                    capture_live = false;
                                    tracing::error!(
                                        "capture liveness reopen resume failed: {e}"
                                    );
                                    trace_recording_start(
                                        &session_id,
                                        "capture_stream_reopen_failed",
                                        start_received_at.elapsed().as_millis() as u64,
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            capture_live = false;
                            tracing::error!("capture liveness reopen failed: {e}");
                            trace_recording_start(
                                &session_id,
                                "capture_stream_reopen_failed",
                                start_received_at.elapsed().as_millis() as u64,
                            );
                        }
                    }
                    // The dead stream burned wall clock before the user could be
                    // heard. Restart the no-speech clock so the backstop measures
                    // the user's silence, not the stream's.
                    last_voice_at = decision_clock.now();
                }
                if !capture_live || capture.is_none() {
                    fail_recording_start!("capture stream is not live".to_string());
                }
                focus.lock().snapshot_at_start_finish(focus_field_prefetch);
                tracing::info!("worker recording started");
            }
            Ok(WorkerCmd::StopRecording { send_enter }) => {
                let Some(session_id) = recording_session.take() else {
                    continue;
                };
                let final_generation = recording_generation;
                recording_generation = recording_generation.wrapping_add(1);
                asr_executor.clear_pending();
                recording = false;
                pending_opening_action = None;
                if let Some(cap) = &mut capture {
                    // Drain the tail, then pause (indicator off; stream stays built).
                    if let Ok(chunk) = cap.read_f32(usize::MAX) {
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
                        buffer.extend_from_slice(&chunk);
                    }
                    cap.pause();
                }
                tracing::info!(
                    "worker recording stopped; transcribing {} samples ({:.1}s)",
                    buffer.len(),
                    buffer.len() as f32 / SAMPLE_RATE as f32
                );
                trace_recording_stop(
                    Some(&session_id),
                    "manual_stop",
                    buffer.len(),
                    heard_voice,
                    speech_start_sample,
                    None,
                    send_enter,
                );
                owner_diagnostics::capture_audio_parts(&session_id, &buffer, &[]);
                let final_audio_secs = audio_secs(&buffer);
                let final_audio = std::mem::take(&mut buffer);
                let asr_started = Instant::now();
                let result = asr_executor.transcribe_final(final_generation, final_audio);
                let asr_elapsed_ms = asr_started.elapsed().as_millis() as u64;
                let vad_aggregate = speech_vad.recording_aggregate();
                owner_diagnostics::record_event(json!({
                    "event": "final_transcript",
                    "session_id": &session_id,
                    "stop_reason": "manual_stop",
                    "raw_transcript": result.as_ref().ok(),
                    "error": result.as_ref().err(),
                }));
                let _ = event_tx.send(WorkerEvent::TranscriptFinal {
                    session_id,
                    result,
                    audio_secs: Some(final_audio_secs),
                    stage_metrics: RecordingStageMetrics {
                        vad_observed_frame_count: vad_aggregate.map(|value| value.0),
                        vad_speech_frame_count: vad_aggregate.map(|value| value.1),
                        asr_elapsed_ms: Some(asr_elapsed_ms),
                        polish_elapsed_ms: None,
                    },
                });
                previous_capture_block = None;
                scheduled_submitted_samples = 0;
                voice_ema = 0.0;
                heard_voice = false;
                speech_start_sample = None;
                kws_stream_generation = kws_stream_generation.wrapping_add(1);
                kws_native_origin_sample = None;
                asr_executor.reset_probe_stream();
            }
            Ok(WorkerCmd::Cancel) => {
                pending_opening_action = None;
                if let Some(session_id) = recording_session.take() {
                    let canceled_generation = recording_generation;
                    recording_generation = recording_generation.wrapping_add(1);
                    asr_executor.clear_pending();
                    asr_executor.cancel_recording(canceled_generation);
                    recording = false;
                    if let Some(cap) = &capture {
                        cap.pause();
                    }
                    trace_recording_stop(
                        Some(&session_id),
                        "external_cancel",
                        buffer.len(),
                        heard_voice,
                        speech_start_sample,
                        None,
                        false,
                    );
                    buffer.clear();
                    previous_capture_block = None;
                    scheduled_submitted_samples = 0;
                    voice_ema = 0.0;
                    heard_voice = false;
                    speech_start_sample = None;
                    kws_stream_generation = kws_stream_generation.wrapping_add(1);
                    kws_native_origin_sample = None;
                    asr_executor.reset_probe_stream();
                    tracing::info!(
                        target: "cancel_lifecycle",
                        phase = "worker_consumed",
                        session_id,
                        "worker recording cancelled"
                    );
                }
            }
            Ok(WorkerCmd::TranscribeFile { path, reply }) => {
                // H5(a) (perf audit 2026-07-15): this worker thread is the
                // ONLY thread draining live mic capture and processing
                // Stop/Cancel. A file transcription is multi-second synchronous
                // work; running it inline while a dictation is active would
                // starve capture drain and queue Stop/Cancel behind it. Fail
                // fast instead — callers (Settings > file transcription) never
                // legitimately fire this mid-recording, so this only guards
                // the pathological case, and error handling for a failed
                // `transcribe_file` reply already exists on every caller.
                if recording {
                    let _ = reply.send(Err(
                        "cannot transcribe a file while dictation is active".to_string(),
                    ));
                    continue;
                }
                let is_pro = crate::settings::is_pro();
                let result = (|| -> Result<FileTranscript, String> {
                    // Cheap pre-decode gate: reject over-limit files before paying
                    // the full decode + RAM load.
                    if let Some(secs) = crate::file_transcribe::probe_duration_secs(&path)? {
                        heardright_core::engine::check_duration_limit(secs, is_pro)?;
                    }
                    let audio = crate::file_transcribe::decode_to_16k_mono(&path)?;
                    // Exact backstop (probe may have returned None).
                    let secs = (audio.len() as u32) / SAMPLE_RATE;
                    heardright_core::engine::check_duration_limit(secs, is_pro)?;
                    let mut transcript = asr_executor.transcribe_file(audio)?;
                    transcript.text = crate::text_polish::polish(&transcript.text);
                    Ok(transcript)
                })();
                let _ = reply.send(result);
            }
            Ok(WorkerCmd::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                asr_executor.shutdown();
                if let Some(cap) = &mut capture {
                    cap.stop();
                }
                tracing::info!("worker shutting down");
                break;
            }
            Err(RecvTimeoutError::Timeout) => {}
        }

}
