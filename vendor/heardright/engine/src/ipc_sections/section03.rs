fn spawn_worker_event_pump(
    runtime: Arc<Mutex<EngineRuntime>>,
    writer: Arc<Mutex<io::Stdout>>,
) -> Result<(), String> {
    let Some(worker_rx) = runtime.lock().worker_event_rx() else {
        return Err("worker event receiver is unavailable".to_string());
    };
    std::thread::Builder::new()
        .name("hr-engine-worker-events".to_string())
        .spawn(move || loop {
            let event = {
                let rx = worker_rx.lock();
                rx.recv()
            };
            let Ok(event) = event else { break };
            match event {
                crate::worker::WorkerEvent::WakeFired {
                    ts,
                    fire_count,
                    threshold,
                } => {
                    emit_event(
                        &writer,
                        EngineSchemaName::WakeEvent,
                        None,
                        Some(EnginePayload::WakeFired {
                            ts,
                            fire_count: Some(fire_count),
                            model: Some("sherpa-onnx-hey-zephyr".into()),
                            threshold: Some(threshold),
                            score: None,
                        }),
                    );
                }
                crate::worker::WorkerEvent::StartFailed { message } => {
                    let frame = error_frame(&next_id(), &next_id(), None, &message);
                    let mut stdout = writer.lock();
                    if let Ok(line) = serde_json::to_string(&frame) {
                        let _ = stdout.write_all(line.as_bytes());
                        let _ = stdout.write_all(b"\n");
                        let _ = stdout.flush();
                    }
                    break;
                }
                crate::worker::WorkerEvent::RecordingLevel { session_id, level } => {
                    emit_event(
                        &writer,
                        EngineSchemaName::RecordingLevel,
                        Some(session_id),
                        Some(EnginePayload::RecordingLevel { level }),
                    );
                }
                crate::worker::WorkerEvent::CaptureFailed {
                    session_id,
                    message,
                } => {
                    runtime.lock().fail_recording_capture(&session_id);
                    let frame = error_frame(
                        &next_id(),
                        &next_id(),
                        Some(&session_id),
                        &message,
                    );
                    let mut stdout = writer.lock();
                    if let Ok(line) = serde_json::to_string(&frame) {
                        let _ = stdout.write_all(line.as_bytes());
                        let _ = stdout.write_all(b"\n");
                        let _ = stdout.flush();
                    }
                }
                crate::worker::WorkerEvent::TranscriptFinal {
                    session_id,
                    result,
                    audio_secs,
                    mut stage_metrics,
                } => {
                    // Capture -> unlock -> commit (Sol audit 2026-07-16,
                    // finding F3): finalize no longer holds the runtime mutex
                    // across stop-time UIA, cloud L1/L2/L3 polish HTTP, or
                    // delivery — see `crate::runtime::finalize_unlocked`'s
                    // doc comment.
                    let polish_started = std::time::Instant::now();
                    let finalized = crate::runtime::finalize_unlocked(
                        &runtime,
                        &session_id,
                        result,
                        audio_secs,
                        None,
                    );
                    stage_metrics.polish_elapsed_ms =
                        Some(polish_started.elapsed().as_millis() as u64);
                    emit_finalized(&writer, session_id, finalized, stage_metrics);
                }
                crate::worker::WorkerEvent::AutoStop {
                    session_id,
                    result,
                    audio_secs,
                    send_enter,
                    mut stage_metrics,
                } => {
                    // AutoStopBegin normally moved the runtime to Transcribing
                    // before final ASR. finalize_unlocked repeats begin_stop
                    // idempotently so a reordered/missing begin signal cannot fail
                    // the stale guard. `send_enter` carries the Enter intent for a
                    // stripped `zephyr send` tail. Polish/delivery runs unlocked.
                    let polish_started = std::time::Instant::now();
                    let finalized = crate::runtime::finalize_unlocked(
                        &runtime,
                        &session_id,
                        result,
                        audio_secs,
                        Some(send_enter),
                    );
                    stage_metrics.polish_elapsed_ms =
                        Some(polish_started.elapsed().as_millis() as u64);
                    emit_finalized(&writer, session_id, finalized, stage_metrics);
                }
                crate::worker::WorkerEvent::AutoStopBegin {
                    session_id,
                    send_enter,
                } => {
                    // Hands-free control tail recognized; the full transcribe is about
                    // to run. Move the runtime to Transcribing NOW so stop timing
                    // includes final ASR, and push TranscribingStarted so the shell
                    // leaves Recording immediately. The later AutoStop begin_stop is
                    // an idempotent no-op.
                    // Hands-free auto-stop never carries the cancel-to-history
                    // intent (that only originates from an explicit shell
                    // `StopDictation { local_only: true }` request), so `false`.
                    let _ = runtime.lock().begin_stop(&session_id, send_enter, false);
                    emit_event(
                        &writer,
                        EngineSchemaName::TranscribingStarted,
                        Some(session_id),
                        Some(EnginePayload::TranscribingStarted { stop_kind: None }),
                    );
                    tracing::info!(
                        drain_ms = 60,
                        "autostop begin: draining shell PTT hook before command dispatch"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(60));
                }
                crate::worker::WorkerEvent::RunawayDiscard { session_id } => {
                    // No-voice backstop: drop the recording, reset to idle.
                    runtime.lock().cancel(&session_id);
                    emit_event(
                        &writer,
                        EngineSchemaName::TranscriptFinal,
                        Some(session_id),
                        Some(EnginePayload::TranscriptFinal {
                            text: String::new(),
                            confidence: Some(1.0),
                            diagnostics: diagnostics_value(DiagnosticsPayload {
                                reset_to_armed: Some(true),
                                ..Default::default()
                            }),
                        }),
                    );
                }
            }
        })
        .map(|_| ())
        .map_err(|error| format!("spawn worker event pump: {error}"))
}

/// Serialize a typed `DiagnosticsPayload` into the wire `Option<Value>` slot.
/// The `TranscriptFinal.diagnostics` field itself stays untyped (protocol v2
/// is additive, not a schema-shape break) — this is purely a sender-side
/// convenience so the engine builds diagnostics from a typed struct instead
/// of ad-hoc `json!({...})` maps. Falls back to `None` only if serialization
/// itself fails (should not happen for this plain-data struct).
fn diagnostics_value(payload: DiagnosticsPayload) -> Option<Value> {
    serde_json::to_value(payload).ok()
}

/// Emit the engine event for a `finalize_transcript` outcome. Shared by the
/// manual-stop `TranscriptFinal` path and the hands-free `AutoStop` path.
fn emit_finalized(
    writer: &Arc<Mutex<io::Stdout>>,
    session_id: String,
    finalized: Result<FinalizeOutcome, String>,
    stage_metrics: heardright_core::engine::RecordingStageMetrics,
) {
    match finalized {
        Ok(FinalizeOutcome::Delivery { record, send_enter }) => {
            let record_value = serde_json::to_value(&record).ok();
            emit_event(
                writer,
                EngineSchemaName::TranscriptFinal,
                Some(session_id),
                Some(EnginePayload::TranscriptFinal {
                    text: record.transcript.clone(),
                    confidence: Some(1.0),
                    diagnostics: diagnostics_value(DiagnosticsPayload {
                        delivery_record: record_value,
                        send_enter: Some(send_enter),
                        recording_stage_metrics: Some(stage_metrics),
                        ..Default::default()
                    }),
                }),
            )
        }
        Ok(FinalizeOutcome::Transcript {
            text,
            send_enter,
            raw_text,
            recording_ms,
        }) => emit_event(
            writer,
            EngineSchemaName::TranscriptFinal,
            Some(session_id),
            Some(EnginePayload::TranscriptFinal {
                text,
                confidence: Some(1.0),
                diagnostics: diagnostics_value(DiagnosticsPayload {
                    shell_delivery: Some(true),
                    send_enter: Some(send_enter),
                    raw_text,
                    recording_ms,
                    recording_stage_metrics: Some(stage_metrics),
                    ..Default::default()
                }),
            }),
        ),
        Ok(FinalizeOutcome::ResetToArmed | FinalizeOutcome::NoOp) => emit_event(
            writer,
            EngineSchemaName::TranscriptFinal,
            Some(session_id),
            Some(EnginePayload::TranscriptFinal {
                text: String::new(),
                confidence: Some(1.0),
                diagnostics: diagnostics_value(DiagnosticsPayload {
                    reset_to_armed: Some(true),
                    ..Default::default()
                }),
            }),
        ),
        Ok(FinalizeOutcome::CommandDispatched { action, detail }) => emit_event(
            writer,
            EngineSchemaName::TranscriptFinal,
            Some(session_id),
            Some(EnginePayload::TranscriptFinal {
                text: String::new(),
                confidence: Some(1.0),
                diagnostics: diagnostics_value(DiagnosticsPayload {
                    reset_to_armed: Some(true),
                    command_dispatched: Some(action),
                    command_detail: Some(detail),
                    ..Default::default()
                }),
            }),
        ),
        // Command classified but dispatch failed: still resets to armed, but the
        // shell gets the failure to surface on the pill (release audit P0-2).
        Ok(FinalizeOutcome::CommandFailed { message }) => emit_event(
            writer,
            EngineSchemaName::TranscriptFinal,
            Some(session_id),
            Some(EnginePayload::TranscriptFinal {
                text: String::new(),
                confidence: Some(1.0),
                diagnostics: diagnostics_value(DiagnosticsPayload {
                    reset_to_armed: Some(true),
                    command_failed: Some(message),
                    ..Default::default()
                }),
            }),
        ),
        Err(message) => {
            let frame = error_frame(&next_id(), &next_id(), Some(&session_id), &message);
            let mut stdout = writer.lock();
            if let Ok(line) = serde_json::to_string(&frame) {
                let _ = stdout.write_all(line.as_bytes());
                let _ = stdout.write_all(b"\n");
                let _ = stdout.flush();
            }
        }
    }
}

use std::sync::atomic::AtomicU64;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> String {
    let n = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    format!("req-{n}")
}
