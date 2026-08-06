fn handle(
    runtime: &Arc<Mutex<EngineRuntime>>,
    writer: &Arc<Mutex<io::Stdout>>,
    worker_pump_started: &Arc<AtomicBool>,
    request_id: &str,
    req: Request,
) -> EngineFrame {
    let trace_id = next_id();
    match req {
        Request::Health => {
            let l3 = crate::l3_cleanup::health();
            let status = if l3.circuit_open || l3.failures > 0 || l3.local_fallbacks > 0 {
                heardright_core::engine::EngineHealthStatus::Degraded
            } else {
                heardright_core::engine::EngineHealthStatus::Ok
            };
            EngineFrame::base(
                EngineSchemaName::EngineHealth,
                request_id,
                None,
                &trace_id,
                Some(EnginePayload::Health {
                    status,
                    diagnostics: Some(json!({
                        "l3_cleanup": {
                            "circuit_open": l3.circuit_open,
                            "groq_circuit_open": l3.groq_circuit_open,
                            "cerebras_circuit_open": l3.cerebras_circuit_open,
                            "nvidia_circuit_open": l3.nvidia_circuit_open,
                            "openrouter_circuit_open": l3.openrouter_circuit_open,
                            "consecutive_failures": l3.consecutive_failures,
                            "attempts": l3.attempts,
                            "successes": l3.successes,
                            "failures": l3.failures,
                            "skips": l3.skips,
                            "local_fallbacks": l3.local_fallbacks,
                            "circuit_opens": l3.circuit_opens
                        }
                    })),
                }),
                None,
            )
        }
        Request::Capabilities => EngineFrame::base(
            EngineSchemaName::EngineCapabilities,
            request_id,
            None,
            &trace_id,
            Some(EnginePayload::Capabilities {
                capabilities: vec![
                    "dictation".into(),
                    "final_only".into(),
                    "focus_tracking".into(),
                    "delivery_precedence".into(),
                ],
            }),
            None,
        ),
        Request::Info => {
            let mut runtime_guard = runtime.lock();
            if let Err(message) = runtime_guard.ensure_worker() {
                return error_frame(request_id, &trace_id, None, &message);
            }
            drop(runtime_guard);
            if let Err(message) = ensure_worker_event_pump(runtime, writer, worker_pump_started) {
                return error_frame(request_id, &trace_id, None, &message);
            }
            let info = runtime.lock().engine_info();
            EngineFrame::base(
                EngineSchemaName::EngineInfo,
                request_id,
                None,
                &trace_id,
                Some(EnginePayload::EngineInfo {
                    mode: info.mode,
                    cold_load_s: info.cold_load_s,
                    engine_version: info.engine_version,
                }),
                None,
            )
        }
        Request::ReplaceEngineConfig { config } => {
            crate::settings::replace_runtime_config(config);
            // Apply model changes off the hotkey path when idle. StartDictation
            // still gates on the same readiness check before it can acknowledge
            // RecordingStarted, so this is an eager UX optimization, not the
            // correctness boundary. Warm on a cloned handle OUTSIDE the lock so a
            // cold reload doesn't block other requests on the runtime mutex.
            let warm: Option<crate::worker::WorkerHandle> = {
                let mut r = runtime.lock();
                if r.is_idle() {
                    r.worker_handle().ok()
                } else {
                    None
                }
            };
            if let Some(handle) = warm {
                if let Err(message) = handle.reload_model() {
                    return error_frame(request_id, &trace_id, None, &message);
                }
            }
            EngineFrame::base(
                EngineSchemaName::EngineAck,
                request_id,
                None,
                &trace_id,
                Some(EnginePayload::EngineAck {
                    status: "ok".into(),
                    detail: Some("engine_config_replaced".into()),
                }),
                None,
            )
        }
        Request::ReplaceVocabulary {
            terms,
            term_details,
        } => {
            // Prefer `term_details` (carries sounds_like aliases) when the
            // shell sends it; fall back to bare `terms` so an old shell still
            // works. The mirror keeps the string-only `terms()` accessor for
            // ASR bias / casing restore while exposing sound-alike pairs for
            // the LLM prompt block.
            let count = if let Some(details) = term_details.as_ref() {
                if !details.is_empty() {
                    let pairs: Vec<(String, Vec<String>)> = details
                        .iter()
                        .map(|detail| (detail.term.clone(), detail.sounds_like.clone()))
                        .collect();
                    let bare: Vec<String> = pairs.iter().map(|(term, _)| term.clone()).collect();
                    crate::vocabulary::replace_terms_with_aliases(bare, pairs);
                    details.len()
                } else {
                    crate::vocabulary::replace_terms(terms.clone());
                    terms.len()
                }
            } else {
                crate::vocabulary::replace_terms(terms.clone());
                terms.len()
            };
            EngineFrame::base(
                EngineSchemaName::EngineAck,
                request_id,
                None,
                &trace_id,
                Some(EnginePayload::EngineAck {
                    status: "ok".into(),
                    detail: Some(format!("vocabulary_terms={count}")),
                }),
                None,
            )
        }
        Request::StartDictation { session_id } => {
            // Get the already-warm worker handle under a brief lock. Model
            // changes are proactively warmed by ReplaceEngineConfig, and the
            // worker keeps the authoritative reload-if-changed fallback. Do not
            // add a second synchronous reload round trip to this hotkey path.
            let handle = match runtime.lock().worker_handle() {
                Ok(h) => h,
                Err(message) => {
                    return error_frame(request_id, &trace_id, Some(&session_id), &message)
                }
            };
            // Flip state under a BRIEF lock. `begin_recording` rejects instead
            // of overwriting when a previous session is still recording or
            // finalizing. Queue Start immediately after that transition. The
            // worker resumes capture first, then overlaps its AX/UIA prefetch
            // with seed capture; doing a synchronous snapshot here delayed and
            // duplicated that exact worker path.
            // `Ok(None)` is the idempotent same-session re-entry — nothing
            // further to do (the dispatch already ran the first time).
            match runtime.lock().begin_recording(&session_id) {
                Ok(Some(_focus)) => {
                    let _ = handle.send(crate::worker::WorkerCmd::StartRecording {
                        session_id: session_id.clone(),
                    });
                }
                Ok(None) => {}
                Err(message) => {
                    return error_frame(request_id, &trace_id, Some(&session_id), &message)
                }
            }
            EngineFrame::base(
                EngineSchemaName::RecordingStarted,
                request_id,
                Some(session_id.clone()),
                &trace_id,
                Some(EnginePayload::RecordingStarted { session_id }),
                None,
            )
        }
        Request::TranscribeFile { path } => {
            // Clone the worker handle under a brief lock, then run the
            // (multi-second) transcription OUTSIDE the runtime lock so it doesn't
            // block prewarm / ptt / settings calls on the mutex.
            let handle = match runtime.lock().worker_handle() {
                Ok(h) => h,
                Err(message) => return error_frame(request_id, &trace_id, None, &message),
            };
            match handle.transcribe_file(path) {
                Ok(transcript) => EngineFrame::base(
                    EngineSchemaName::FileTranscriptionResult,
                    request_id,
                    None,
                    &trace_id,
                    Some(EnginePayload::FileTranscriptionResult {
                        text: transcript.text,
                        srt: transcript.srt,
                        vtt: transcript.vtt,
                        words: transcript.words,
                    }),
                    None,
                ),
                Err(message) => error_frame(request_id, &trace_id, None, &message),
            }
        }
        Request::StopDictation {
            session_id,
            send_enter,
            local_only,
        } => {
            let mut r = runtime.lock();
            match r.begin_stop(&session_id, send_enter, local_only) {
                Ok(()) => EngineFrame::base(
                    EngineSchemaName::TranscribingStarted,
                    request_id,
                    Some(session_id),
                    &trace_id,
                    Some(EnginePayload::TranscribingStarted {
                        stop_kind: Some(if local_only {
                            heardright_core::engine::StopKind::CancelToHistory
                        } else if send_enter {
                            heardright_core::engine::StopKind::SendEnter
                        } else {
                            heardright_core::engine::StopKind::Stop
                        }),
                    }),
                    None,
                ),
                Err(message) => error_frame(request_id, &trace_id, Some(&session_id), &message),
            }
        }
        Request::CancelDictation { session_id } => {
            let source = cancel_source_from_request_id(request_id);
            tracing::info!(
                target: "cancel_lifecycle",
                phase = "engine_received",
                source,
                session_id,
                request_id,
                "external cancel received"
            );
            runtime
                .lock()
                .cancel_traced(&session_id, request_id, source);
            EngineFrame::base(
                EngineSchemaName::TranscribingStarted,
                request_id,
                Some(session_id),
                &trace_id,
                Some(EnginePayload::TranscribingStarted {
                    stop_kind: Some(heardright_core::engine::StopKind::Cancel),
                }),
                None,
            )
        }
        Request::RepasteLast => {
            let mut r = runtime.lock();
            match r.repaste_last() {
                Ok(record) => EngineFrame::base(
                    EngineSchemaName::ManualDeliveryResult,
                    request_id,
                    None,
                    &trace_id,
                    Some(EnginePayload::ManualDeliveryResult {
                        record: serde_json::to_value(record).unwrap_or(Value::Null),
                    }),
                    None,
                ),
                Err(message) => error_frame(request_id, &trace_id, None, &message),
            }
        }
        Request::CopyLast => {
            let mut r = runtime.lock();
            match r.copy_last() {
                Ok(transcript) => EngineFrame::base(
                    EngineSchemaName::CopyLastResult,
                    request_id,
                    None,
                    &trace_id,
                    Some(EnginePayload::CopyLastResult { text: transcript }),
                    None,
                ),
                Err(message) => error_frame(request_id, &trace_id, None, &message),
            }
        }
        Request::GetState => {
            let state = runtime.lock().state().clone();
            let payload = EnginePayload::EngineStateSnapshot {
                state: format!("{:?}", state),
            };
            EngineFrame::base(
                EngineSchemaName::EngineStateSnapshot,
                request_id,
                None,
                &trace_id,
                Some(payload),
                None,
            )
        }
        Request::GetRecentHistory { limit } => {
            let records = runtime.lock().recent_history(limit);
            let payload = EnginePayload::RecentHistoryResult {
                records: serde_json::to_value(records).unwrap_or(Value::Null),
            };
            EngineFrame::base(
                EngineSchemaName::RecentHistoryResult,
                request_id,
                None,
                &trace_id,
                Some(payload),
                None,
            )
        }
        Request::ReplaceRecentHistory { records } => {
            let count = records.len();
            runtime.lock().replace_recent_history(records);
            EngineFrame::base(
                EngineSchemaName::EngineAck,
                request_id,
                None,
                &trace_id,
                Some(EnginePayload::EngineAck {
                    status: "ok".into(),
                    detail: Some(format!("records={count}")),
                }),
                None,
            )
        }
        Request::StartWakeListen { model, threshold } => {
            if !crate::settings::wake_lab_marker_present() {
                return error_frame(request_id, &trace_id, None, "Wake Lab marker is absent");
            }
            let threshold = threshold.unwrap_or(0.25).clamp(0.01, 0.99);
            let handle = match runtime.lock().worker_handle() {
                Ok(handle) => handle,
                Err(message) => return error_frame(request_id, &trace_id, None, &message),
            };
            if let Err(message) = handle.start_wake_listen(threshold) {
                return error_frame(request_id, &trace_id, None, &message);
            }
            EngineFrame::base(
                EngineSchemaName::WakeListenStarted,
                request_id,
                None,
                &trace_id,
                Some(EnginePayload::WakeListenStarted {
                    status: "started".into(),
                    model: model.or_else(|| Some("sherpa-onnx-hey-zephyr".into())),
                    threshold: Some(threshold),
                    fake: None,
                }),
                None,
            )
        }
        Request::StopWakeListen => {
            let total_fires = match runtime
                .lock()
                .worker_handle()
                .and_then(|handle| handle.stop_wake_listen())
            {
                Ok(total_fires) => total_fires,
                Err(message) => return error_frame(request_id, &trace_id, None, &message),
            };
            EngineFrame::base(
                EngineSchemaName::WakeListenStopped,
                request_id,
                None,
                &trace_id,
                Some(EnginePayload::WakeListenStopped {
                    status: "stopped".into(),
                    total_fires: Some(total_fires),
                }),
                None,
            )
        }
        Request::Shutdown => EngineFrame::health(request_id, &trace_id),
    }
}

fn ensure_worker_event_pump(
    runtime: &Arc<Mutex<EngineRuntime>>,
    writer: &Arc<Mutex<io::Stdout>>,
    started: &Arc<AtomicBool>,
) -> Result<(), String> {
    if started
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Ok(());
    }
    if let Err(error) = spawn_worker_event_pump(runtime.clone(), writer.clone()) {
        started.store(false, Ordering::SeqCst);
        return Err(error);
    }
    Ok(())
}

fn error_frame(
    request_id: &str,
    trace_id: &str,
    session_id: Option<&str>,
    message: &str,
) -> EngineFrame {
    EngineFrame::base(
        EngineSchemaName::EngineError,
        request_id,
        session_id.map(str::to_string),
        trace_id,
        None,
        Some(EngineErrorPayload {
            code: "E_ENGINE".into(),
            message: message.to_string(),
            retryable: false,
            diagnostics: None,
        }),
    )
}

fn emit_event(
    writer: &Arc<Mutex<io::Stdout>>,
    schema_name: EngineSchemaName,
    session_id: Option<String>,
    payload: Option<EnginePayload>,
) {
    let frame = EngineFrame::base(
        schema_name,
        &next_id(),
        session_id,
        &next_id(),
        payload,
        None,
    );
    let mut stdout = writer.lock();
    let result = serde_json::to_string(&frame)
        .map_err(|error| format!("serialize IPC event: {error}"))
        .and_then(|line| {
            stdout
                .write_all(line.as_bytes())
                .and_then(|_| stdout.write_all(b"\n"))
                .and_then(|_| stdout.flush())
                .map_err(|error| format!("write IPC event: {error}"))
        });
    if let Err(error) = result {
        // A sidecar that cannot report terminal state must not retain delivery
        // authority. Exiting makes the supervisor revoke/restart this process.
        eprintln!("heardright-engine fatal IPC output failure: {error}");
        std::process::exit(1);
    }
}
