#[derive(Clone, Debug, Eq, PartialEq)]
struct CaptureRouteSignature {
    device_id: usize,
    name: String,
    native_rate: u32,
    channels: u16,
    sample_format: String,
    transport: CaptureTransport,
    form_factor: heardright_capture::CaptureFormFactor,
}

fn resolve_capture_route(
    saved_device: Option<&str>,
    devices: &[DeviceInfo],
) -> (Option<usize>, Option<CaptureRouteSignature>) {
    let snapshots: Vec<MicDeviceSnapshot> = devices
        .iter()
        .map(|d| MicDeviceSnapshot {
            id: d.id,
            name: d.name.clone(),
            is_default: d.is_default,
        })
        .collect();
    let selection = resolve_mic_selection(saved_device, &snapshots);
    let selected = selection
        .device_id
        .and_then(|id| devices.iter().find(|device| device.id == id))
        .or_else(|| devices.iter().find(|device| device.is_default));
    let signature = selected.map(|device| CaptureRouteSignature {
        device_id: device.id,
        name: device.name.clone(),
        native_rate: device.native_rate,
        channels: device.channels,
        sample_format: device.sample_format.clone(),
        transport: device.transport,
        form_factor: device.form_factor,
    });
    (selection.device_id, signature)
}

fn current_capture_route(
    saved_device: Option<&str>,
) -> Result<Option<CaptureRouteSignature>, String> {
    let devices = list_input_devices().map_err(|e| format!("capture route query: {e}"))?;
    Ok(resolve_capture_route(saved_device, &devices).1)
}

fn capture_route_changed(
    previous: &Option<CaptureRouteSignature>,
    current: &Option<CaptureRouteSignature>,
) -> bool {
    current.is_some() && current != previous
}

/// Open a capture session honoring the saved mic selection.
///
/// Callers own the privacy boundary: do not open until an explicit
/// `StartRecording` (or equivalent user gesture). `CaptureSession::start`
/// begins playing immediately; StartRecording pauses, flushes spin-up
/// garbage, then resumes so the first-word seed matches the warm-reuse path.
/// The returned route signature lets later starts reuse this stream unless the
/// device, rate, channel count, or sample format changed while it was paused.
fn open_capture(
    saved_device: Option<&str>,
) -> Result<(WorkerCapture, Option<CaptureRouteSignature>), String> {
    #[cfg(test)]
    if let Some(capture) = replay_capture() {
        return Ok((WorkerCapture::Replay(capture), None));
    }
    let devices = list_input_devices().map_err(|e| format!("capture devices: {e}"))?;
    let (device_id, route) = resolve_capture_route(saved_device, &devices);
    let cfg = SessionConfig {
        device_id,
        target_rate: SAMPLE_RATE,
        target_channels: 1,
        target_dtype: OutDType::Float32,
        block_ms: 20,
    };
    let capture = CaptureSession::start(cfg).map_err(|e| {
        let detail = e.to_string();
        // WASAPI surfaces package/desktop mic denial as Access Denied. Keep the
        // OS text and add a stable hint so field logs distinguish consent from
        // a missing/broken device.
        if detail.contains("Access is denied")
            || detail.contains("0x80070005")
            || detail.contains("-2147024891")
        {
            format!(
                "capture open: {detail} (HeardRight microphone permission may still be denied in Windows Settings)"
            )
        } else {
            format!("capture open: {detail}")
        }
    })?;
    Ok((WorkerCapture::Live(capture), route))
}

// --- Warm-capture liveness (field incident 2026-07-23, sidecar 16:22:13) ---
// After an update relaunch the warm CoreAudio stream sat paused ~7.5 min, then
// resumed and delivered 13.3 s of samples that Silero scored as pure silence.
// The route-signature guard above correctly found nothing changed: the DEVICE
// was fine, the STREAM was dead. Silero answers "is this audio speech?";
// nothing answered "is this audio alive?".
//
// The discriminator is EXACT zeros, deliberately not an RMS/peak threshold. A
// live input never produces long runs of bit-exact 0.0 — even a silent room
// carries dither and thermal noise around 1e-5 — and any RMS floor low enough
// to catch a dead stream is high enough to punish a quiet talker on a good mic.
// Exact-zero ratio is near-binary and needs no tuning, so a quiet user cannot
// trip it.
const CAPTURE_LIVENESS_MIN_SAMPLES: usize = SAMPLE_RATE as usize / 40;
const CAPTURE_LIVENESS_ZERO_RATIO: f32 = 0.98;

fn pcm_zero_ratio(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let zeros = samples.iter().filter(|sample| **sample == 0.0).count();
    zeros as f32 / samples.len() as f32
}

/// True when samples are arriving but carry no signal whatsoever. Requires a
/// minimum sample count so a short or partial read is never mistaken for a dead
/// stream.
fn capture_stream_looks_dead(samples: &[f32]) -> bool {
    samples.len() >= CAPTURE_LIVENESS_MIN_SAMPLES
        && pcm_zero_ratio(samples) >= CAPTURE_LIVENESS_ZERO_RATIO
}

fn capture_first_buffer_requires_reopen(first: &CaptureFirstBuffer) -> bool {
    match first {
        CaptureFirstBuffer::Data(samples) => {
            samples.len() < CAPTURE_LIVENESS_MIN_SAMPLES || capture_stream_looks_dead(samples)
        }
        CaptureFirstBuffer::NoCallbacks
        | CaptureFirstBuffer::NoSamples
        | CaptureFirstBuffer::StreamError(_) => true,
    }
}

fn capture_settle_budget_ms(route: Option<&CaptureRouteSignature>, repair_attempt: bool) -> u64 {
    if route.is_some_and(|route| route.transport == CaptureTransport::Bluetooth) {
        1_500
    } else if repair_attempt {
        500
    } else {
        250
    }
}

/// Wait through startup callbacks that contain only digital zeros. Bluetooth
/// hands-free routes can publish several such callbacks while their input path
/// settles; returning on the first callback recreates the 150 ms false failure.
fn wait_for_live_first_buffer(
    capture: &mut WorkerCapture,
    timeout_ms: u64,
    decision_clock: &dyn worker_clock::WorkerClock,
) -> Result<CaptureFirstBuffer, String> {
    let timeout = Duration::from_millis(timeout_ms);
    let deadline = decision_clock.now().saturating_add(timeout);
    // Blocking capture waits consume this budget even when a test source has
    // no later event with which to advance virtual time.
    let mut wait_budget = timeout;
    let mut samples = Vec::new();
    let mut last_empty = CaptureFirstBuffer::NoCallbacks;
    loop {
        let remaining = deadline
            .duration_since(decision_clock.now())
            .min(wait_budget);
        if remaining.is_zero() {
            break;
        }
        let slice_ms = remaining.as_millis().clamp(1, 100) as u64;
        wait_budget = wait_budget.saturating_sub(Duration::from_millis(slice_ms));
        match capture
            .wait_first_buffer(CAPTURE_LIVENESS_MIN_SAMPLES, slice_ms)
            .map_err(|error| error.to_string())?
        {
            CaptureFirstBuffer::Data(mut chunk) => {
                samples.append(&mut chunk);
                if samples.len() >= CAPTURE_LIVENESS_MIN_SAMPLES
                    && !capture_stream_looks_dead(&samples)
                {
                    return Ok(CaptureFirstBuffer::Data(samples));
                }
            }
            CaptureFirstBuffer::StreamError(kind) => {
                return Ok(CaptureFirstBuffer::StreamError(kind));
            }
            empty @ (CaptureFirstBuffer::NoCallbacks | CaptureFirstBuffer::NoSamples) => {
                last_empty = empty;
            }
        }
    }
    if samples.is_empty() {
        Ok(last_empty)
    } else {
        Ok(CaptureFirstBuffer::Data(samples))
    }
}

#[derive(Debug)]
struct CapturePcmHistory {
    block: Vec<f32>,
    consecutive_repeats: usize,
}

#[derive(Debug)]
enum CapturePcmValidation {
    Accepted(CapturePcmHistory),
    DiscardedRepeat(CapturePcmHistory),
}

impl CapturePcmValidation {
    fn history(self) -> CapturePcmHistory {
        match self {
            Self::Accepted(history) | Self::DiscardedRepeat(history) => history,
        }
    }

    fn discarded_repeat(&self) -> bool {
        matches!(self, Self::DiscardedRepeat(_))
    }
}

/// Capture is trusted only after one shared ingress check. NaN/Inf would
/// poison VAD comparisons, levels, KWS timestamps & final decode differently;
/// reject it once before any subsystem observes the block. Some Windows
/// drivers replay one callback during capture start, so discard one repeated
/// block; a second consecutive replay still proves capture is stuck.
fn validate_capture_pcm(
    samples: &[f32],
    previous: Option<&CapturePcmHistory>,
) -> Result<CapturePcmValidation, &'static str> {
    if !samples.iter().all(|sample| sample.is_finite()) {
        Err("capture PCM contains non-finite sample")
    } else if samples.len() >= CAPTURE_LIVENESS_MIN_SAMPLES
        && samples.windows(2).all(|pair| pair[0] == pair[1])
    {
        Err("capture PCM is constant")
    } else if samples.len() >= CAPTURE_LIVENESS_MIN_SAMPLES
        && samples.iter().all(|sample| sample.abs() >= 0.999)
    {
        Err("capture PCM is clipped")
    } else {
        let consecutive_repeats = previous
            .filter(|prior| prior.block == samples)
            .map_or(0, |prior| prior.consecutive_repeats.saturating_add(1));
        let history = CapturePcmHistory {
            block: samples.to_vec(),
            consecutive_repeats,
        };
        if consecutive_repeats >= 2 {
            Err("capture PCM repeated consecutive blocks")
        } else if consecutive_repeats == 1 {
            Ok(CapturePcmValidation::DiscardedRepeat(history))
        } else {
            Ok(CapturePcmValidation::Accepted(history))
        }
    }
}

fn capture_should_reopen(capture_missing: bool, saved_device_changed: bool) -> bool {
    capture_missing || saved_device_changed
}

fn drop_failed_capture<T>(capture: &mut Option<T>) {
    *capture = None;
}

#[cfg(test)]
mod capture_recovery_tests {
    use super::*;

    #[test]
    fn sustained_repeat_drops_capture_and_admits_a_fresh_session() {
        let block = vec![0.0, 0.25, -0.25];
        let first = validate_capture_pcm(&block, None).unwrap().history();
        let second = validate_capture_pcm(&block, Some(&first)).unwrap();
        assert!(second.discarded_repeat());
        assert!(validate_capture_pcm(&block, Some(&second.history())).is_err());

        let mut capture = Some("poisoned");
        drop_failed_capture(&mut capture);
        assert!(capture_should_reopen(capture.is_none(), false));
        assert!(validate_capture_pcm(&block, None).is_ok());
    }
}

fn transcribe_buffer(model: &mut AsrRuntime, buffer: &[f32]) -> Result<String, String> {
    transcribe_conditioned(buffer, BlankPolicy::Final, |conditioned| {
        model.with_inference_lease("final_asr", |model| {
            // Restore final-decode bias after any command probe on this same
            // Windows runtime while its decode lease is held.
            crate::asr::apply_utterance_bias(model);
            model.transcribe_final_under_lease(conditioned)
        })
    })
}

fn transcribe_opening_buffer(model: &mut AsrRuntime, buffer: &[f32]) -> Result<String, String> {
    transcribe_conditioned(buffer, BlankPolicy::Probe, |conditioned| {
        model.with_inference_lease("opening_command_asr", |model| {
            crate::asr::apply_utterance_bias(model);
            model.transcribe_under_lease(conditioned)
        })
    })
}

fn warm_main_asr(model: &mut AsrRuntime) -> Result<(), String> {
    let silence = vec![0.0; SAMPLE_RATE as usize];
    model.with_inference_lease("main_asr_warmup", |model| {
        crate::asr::apply_utterance_bias(model);
        model.transcribe_under_lease(&silence).map(|_| ())
    })
}

fn finish_recording_transcript(
    model: &mut AsrRuntime,
    scheduled: &mut ScheduledStatic15,
    buffer: &[f32],
    generation: u64,
) -> Result<String, String> {
    let scheduled_path = model.uses_scheduled_static15();
    let result = if scheduled_path {
        // Every uncommitted sample through the trigger cut is decoded here.
        // Scheduled windows remain the only source of already-committed text.
        scheduled.finish(buffer, |tail| transcribe_buffer(model, tail))
    } else {
        transcribe_buffer(model, buffer)
    };
    match &result {
        Ok(text) => tracing::info!(
            generation,
            total_samples = buffer.len(),
            final_chars = text.chars().count(),
            final_words = text.split_whitespace().count(),
            final_empty = text.trim().is_empty(),
            "recording transcript assembly complete"
        ),
        Err(error) => tracing::error!(
            generation,
            total_samples = buffer.len(),
            error,
            "recording transcript assembly failed"
        ),
    }
    result
}

fn audio_secs(buffer: &[f32]) -> f32 {
    buffer.len() as f32 / SAMPLE_RATE as f32
}

struct CommandProbeLog<'a> {
    lane: &'static str,
    session_id: Option<&'a str>,
    probe_ms: u64,
    recognized_text: &'a str,
    classifier_result: &'static str,
    speech_start_sample: Option<usize>,
    command_start: usize,
    command_samples: usize,
    total_samples: usize,
    pending_prefix: bool,
    error: Option<&'a str>,
}

fn command_probe_payload(log: CommandProbeLog<'_>) -> serde_json::Value {
    let recognized = log.recognized_text.trim();
    json!({
        "event": "command_probe",
        "lane": log.lane,
        "session_id": log.session_id,
        "probe_ms": log.probe_ms,
        "recognized_text": log.recognized_text,
        "recognized_chars": recognized.chars().count(),
        "recognized_words": recognized.split_whitespace().count(),
        "recognized_empty": recognized.is_empty(),
        "classifier_result": log.classifier_result,
        "speech_start_sample": log.speech_start_sample,
        "command_start": log.command_start,
        "command_samples": log.command_samples,
        "total_samples": log.total_samples,
        "pending_prefix": log.pending_prefix,
        "error": log.error,
    })
}

fn trace_command_probe(log: CommandProbeLog<'_>) {
    emit_command_probe(command_probe_payload(log));
}

fn trace_recording_stop(
    session_id: Option<&str>,
    reason: &'static str,
    total_samples: usize,
    heard_voice: bool,
    speech_start_sample: Option<usize>,
    silence_for_ms: Option<u64>,
    send_enter: bool,
) {
    let payload = json!({
        "event": "recording_stop",
        "session_id": session_id,
        "reason": reason,
        "total_samples": total_samples,
        "audio_secs": audio_secs_from_samples(total_samples),
        "heard_voice": heard_voice,
        "speech_start_sample": speech_start_sample,
        "silence_for_ms": silence_for_ms,
        "send_enter": send_enter,
    });
    emit_command_probe(payload);
}

fn trace_recording_start(session_id: &str, phase: &'static str, elapsed_ms: u64) {
    let payload = json!({
        "event": "recording_start",
        "session_id": session_id,
        "phase": phase,
        "elapsed_ms": elapsed_ms,
    });
    emit_command_probe(payload);
}

fn audio_secs_from_samples(samples: usize) -> f32 {
    samples as f32 / SAMPLE_RATE as f32
}

fn emit_command_probe(payload: serde_json::Value) {
    if !crate::settings::diagnostics_enabled() {
        return;
    }
    let mut payload = payload;
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "ts_ms".to_string(),
            json!(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64),
        );
    }
    let redacted = heardright_core::redact_diagnostic_event(payload);
    append_command_probe_event(&redacted);
}

// M6 (perf audit 2026-07-15): `append_command_probe_event` used to open+append
// the JSONL file synchronously on the capture/worker thread, ~3-4x/sec while
// recording (every tail probe + standalone-command probe + start/stop trace).
// A disk stall (AV scan, slow/contended disk) backpressured live capture
// directly. Move the actual write to a dedicated writer thread; the worker
// thread only enqueues (an unbounded mpsc `send` never blocks), preserving
// event ORDER (this function has exactly one caller thread — the worker loop
// — so the channel's FIFO delivery keeps the JSONL in the same sequence as
// before) and preserving every field/format of the written line exactly.
static COMMAND_PROBE_TX: OnceLock<Sender<serde_json::Value>> = OnceLock::new();

fn command_probe_writer() -> &'static Sender<serde_json::Value> {
    COMMAND_PROBE_TX.get_or_init(|| {
        let (tx, rx) = channel::<serde_json::Value>();
        let spawned = std::thread::Builder::new()
            .name("hr-engine-command-probe-log".to_string())
            .spawn(move || {
                while let Ok(payload) = rx.recv() {
                    write_command_probe_event_sync(&payload);
                }
            })
            .is_ok();
        if !spawned {
            tracing::warn!("command probe telemetry writer thread failed to spawn");
        }
        tx
    })
}

fn append_command_probe_event(payload: &serde_json::Value) {
    if !crate::settings::diagnostics_enabled() {
        return;
    }
    // If the writer thread failed to spawn, `send` errors (receiver dropped
    // with the thread) — drop the event rather than block or panic; this is
    // diagnostic telemetry, never load-bearing for transcription/delivery.
    let _ = command_probe_writer().send(payload.clone());
}

fn write_command_probe_event_sync(payload: &serde_json::Value) {
    if !crate::settings::diagnostics_enabled() {
        return;
    }
    // Direct sidecar append keeps command smoke debuggable. It must not share
    // events.jsonl with the app process, because cross-process appends can
    // interleave under probe load and corrupt JSONL.
    let path = crate::settings::app_data_root().join("engine-events.jsonl");
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            tracing::warn!(
                path = %parent.display(),
                error = %err,
                "command probe telemetry directory create failed"
            );
            return;
        }
    }
    if let Err(err) = rotate_command_probe_log_if_needed(&path) {
        tracing::warn!(
            path = %path.display(),
            error = %err,
            "command probe telemetry rotate failed"
        );
    }
    let payload = heardright_core::redact_diagnostic_event(payload.clone());
    match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(mut file) => {
            if let Err(err) = writeln!(file, "{payload}") {
                tracing::warn!(
                    path = %path.display(),
                    error = %err,
                    "command probe telemetry write failed"
                );
            }
        }
        Err(err) => {
            tracing::warn!(
                path = %path.display(),
                error = %err,
                "command probe telemetry open failed"
            );
        }
    }
}

fn rotate_command_probe_log_if_needed(path: &Path) -> std::io::Result<()> {
    let Ok(metadata) = std::fs::metadata(path) else {
        return Ok(());
    };
    if metadata.len() < COMMAND_EVENTS_JSONL_MAX_BYTES {
        return Ok(());
    }
    let rotated = path.with_extension("jsonl.1");
    let _ = std::fs::remove_file(&rotated);
    std::fs::rename(path, rotated)
}

fn command_probe_start_sample(speech_start_sample: Option<usize>, buffer_samples: usize) -> usize {
    // With a VAD-confirmed speech start, probe from there backward (covers
    // the word that armed the gesture + the trigger word itself). Otherwise,
    // fall back to the LAST CMD_WINDOW_SAMPLES of audio — same window the
    // zephyr_tail probe uses — so the standalone lane sees the same speech
    // and gets a chance to classify. Without this, VAD misses (common on
    // Windows WASAPI shared-mode where the VAD CNN was trained on cleaner
    // audio) leave the standalone probe staring at the full buffer's
    // pre-roll silence and failing the pre-decode voice gate.
    // (Field bug 2026-07-06: "right click" never classified as a standalone
    // command because VAD missed the trigger while zephyr_tail classified
    // it cleanly from the tail window.)
    match speech_start_sample {
        Some(start) => start,
        None => buffer_samples.saturating_sub(CMD_WINDOW_SAMPLES),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TailProbeLane {
    Fast,
    Full,
}

fn tail_probe_start_sample(buffer_samples: usize, lane: TailProbeLane) -> usize {
    let window_samples = match lane {
        TailProbeLane::Fast => FAST_TAIL_WINDOW_SAMPLES,
        TailProbeLane::Full => TAIL_WINDOW_SAMPLES,
    };
    buffer_samples.saturating_sub(window_samples)
}

fn tail_text_ends_with_control_verb(text: &str) -> bool {
    let Some(last) = text.split_whitespace().last() else {
        return false;
    };
    // Reuse the canonical fuzzy verb vocabulary rather than maintaining a
    // second list in the worker. This only requests the reliable probe; it can
    // never fire a command by itself.
    heardright_core::text_pipeline::parse_control_command(&format!("zephyr {last}")).is_some()
}

/// A standalone command is candidate-only until a later opening probe resolves
/// to the same normalized action identity.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PendingOpeningAction {
    action_identity: String,
    total_samples: usize,
}

fn normalize_opening_action_text(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalized_opening_action_identity(action: &heardright_core::command::CommandAction) -> String {
    use heardright_core::command::CommandAction;

    match action {
        CommandAction::KeySequence { chords, .. } => format!(
            "key:{}",
            chords
                .iter()
                .map(|chord| normalize_opening_action_text(chord))
                .collect::<Vec<_>>()
                .join("|")
        ),
        CommandAction::LaunchApp { name } => {
            format!("launch:{}", normalize_opening_action_text(name))
        }
        CommandAction::RunShortcut { name } => {
            format!("shortcut:{}", normalize_opening_action_text(name))
        }
        CommandAction::Mouse {
            action,
            button,
            clicks,
            direction,
            page,
        } => format!(
            "mouse:{}:{}:{:?}:{}:{}",
            normalize_opening_action_text(action),
            button
                .as_deref()
                .map(normalize_opening_action_text)
                .unwrap_or_default(),
            clicks,
            direction
                .as_deref()
                .map(normalize_opening_action_text)
                .unwrap_or_default(),
            page,
        ),
        CommandAction::Power {
            op,
            requires_confirm,
        } => format!(
            "power:{}:{}",
            normalize_opening_action_text(op),
            requires_confirm
        ),
        CommandAction::LastPasteTransform { transform } => {
            format!("transform:{}", normalize_opening_action_text(transform))
        }
        CommandAction::Special { op } => format!("special:{}", normalize_opening_action_text(op)),
    }
}

/// Returns true only when two consecutive opening probes resolve to the same
/// action & their total sample count strictly advances. Ambiguous grammar
/// matches additionally require a pause-ready second observation.
fn opening_action_confirmation(
    pending: &mut Option<PendingOpeningAction>,
    action_identity: &str,
    total_samples: usize,
    pause_ready: bool,
    requires_pause_ready: bool,
) -> bool {
    let confirmed = pending.as_ref().is_some_and(|prior| {
        prior.action_identity == action_identity
            && total_samples > prior.total_samples
            && (!requires_pause_ready || pause_ready)
    });
    if confirmed {
        *pending = None;
        return true;
    }

    *pending = Some(PendingOpeningAction {
        action_identity: action_identity.to_owned(),
        total_samples,
    });
    false
}

#[cfg(any(target_os = "windows", test))]
fn should_run_full_tail_fallback(
    fast_text: &str,
    candidate_armed: bool,
    new_audio_samples: usize,
) -> bool {
    (candidate_armed || tail_text_ends_with_control_verb(fast_text))
        && new_audio_samples >= TAIL_PREFIX_NEW_AUDIO_SAMPLES
}

fn should_keep_control_candidate_armed(control_found: bool, saw_control_candidate: bool) -> bool {
    !control_found && saw_control_candidate
}

fn opening_command_due(
    heard_voice: bool,
    pending_prefix: bool,
    pause_ready: bool,
    command_audio_samples: usize,
    last_submitted_samples: usize,
    last_check: Instant,
) -> bool {
    heard_voice
        && (command_audio_samples <= CMD_WINDOW_SAMPLES || pending_prefix)
        && command_audio_samples >= CMD_MIN_PROBE_SAMPLES
        && (pause_ready || last_check.elapsed() >= Duration::from_millis(CMD_RETRY_MS))
        && (command_audio_samples
            >= last_submitted_samples.saturating_add(CMD_NEW_AUDIO_PROBE_SAMPLES)
            || (pause_ready && !pending_prefix))
}

fn opening_command_due_at(
    heard_voice: bool,
    pending_prefix: bool,
    pause_ready: bool,
    command_audio_samples: usize,
    last_submitted_samples: usize,
    last_check: Option<worker_clock::WorkerInstant>,
    now: worker_clock::WorkerInstant,
) -> bool {
    heard_voice
        && (command_audio_samples <= CMD_WINDOW_SAMPLES || pending_prefix)
        && command_audio_samples >= CMD_MIN_PROBE_SAMPLES
        && (pause_ready
            || last_check.is_none_or(|last_check| {
                now.duration_since(last_check) >= Duration::from_millis(CMD_RETRY_MS)
            }))
        && (command_audio_samples
            >= last_submitted_samples.saturating_add(CMD_NEW_AUDIO_PROBE_SAMPLES)
            || (pause_ready && !pending_prefix))
}

fn main_asr_control_fallback_due(
    _checked_this_pause: bool,
    _heard_voice: bool,
    _long_dictation: bool,
    _pause_ready: bool,
) -> bool {
    // Trigger controls are KWS-only. Main ASR never opens a fallback lane that
    // could scan, arm, parse, or confirm a control command.
    false
}

#[cfg(test)]
fn confirmed_main_asr_control(
    _text: &str,
    _pause_ready: bool,
) -> Option<heardright_core::text_pipeline::ControlCommand> {
    None
}

fn observe_voice_chunk(
    chunk: &[f32],
    last_voice_at: &mut Instant,
    heard_voice: &mut bool,
    checked_this_pause: &mut bool,
    mut speech_observer: impl FnMut(&[f32]) -> bool,
    speech_start_sample: &mut Option<usize>,
    chunk_start_sample: usize,
) {
    if chunk.is_empty() {
        return;
    }
    // Silero owns both first-speech detection and the ongoing voice clock. Do
    // not pre-filter its input with an RMS/peak heuristic: one loud transient
    // could otherwise prevent all later quiet speech from reaching the VAD.
    let confirmed = speech_observer(chunk);
    if confirmed {
        if speech_start_sample.is_none() {
            *speech_start_sample = Some(chunk_start_sample.saturating_sub(CMD_PREROLL_SAMPLES));
        }
        *last_voice_at = Instant::now();
        *heard_voice = true;
        *checked_this_pause = false;
    }
}

fn observe_voice_chunk_at(
    chunk: &[f32],
    last_voice_at: &mut worker_clock::WorkerInstant,
    now: worker_clock::WorkerInstant,
    heard_voice: &mut bool,
    checked_this_pause: &mut bool,
    mut speech_observer: impl FnMut(&[f32]) -> bool,
    speech_start_sample: &mut Option<usize>,
    chunk_start_sample: usize,
) {
    if chunk.is_empty() {
        return;
    }
    let confirmed = speech_observer(chunk);
    if confirmed {
        if speech_start_sample.is_none() {
            *speech_start_sample = Some(chunk_start_sample.saturating_sub(CMD_PREROLL_SAMPLES));
        }
        *last_voice_at = now;
        *heard_voice = true;
        *checked_this_pause = false;
    }
}

fn confirm_first_speech(chunk: &[f32], speech_vad: &mut crate::vad::SpeechVad) -> bool {
    speech_vad.observe(chunk)
}

fn pause_can_close_command_probe(heard_voice: bool, pause_ready: bool) -> bool {
    heard_voice && pause_ready
}

fn mark_pause_probe_submitted(checked_this_pause: &mut bool, pause_ready: bool) {
    if pause_ready {
        *checked_this_pause = true;
    }
}

fn mark_opening_prefix_result(
    checked_this_pause: &mut bool,
    pending_command_prefix_since: &mut Option<Instant>,
) {
    *checked_this_pause = false;
    pending_command_prefix_since.get_or_insert_with(Instant::now);
}

fn mark_opening_prefix_result_at(
    checked_this_pause: &mut bool,
    pending_command_prefix_since: &mut Option<worker_clock::WorkerInstant>,
    now: worker_clock::WorkerInstant,
) {
    *checked_this_pause = false;
    pending_command_prefix_since.get_or_insert(now);
}

#[cfg(test)]
mod decision_clock_tests {
    use super::*;

    #[test]
    fn retry_gate_uses_worker_time() {
        let submitted =
            worker_clock::WorkerInstant::from_duration_since_start(Duration::from_millis(1_000));
        let early = worker_clock::WorkerInstant::from_duration_since_start(Duration::from_millis(
            1_000 + CMD_RETRY_MS - 1,
        ));
        let due = worker_clock::WorkerInstant::from_duration_since_start(Duration::from_millis(
            1_000 + CMD_RETRY_MS,
        ));

        assert!(!opening_command_due_at(
            true,
            false,
            false,
            CMD_MIN_PROBE_SAMPLES,
            0,
            Some(submitted),
            early,
        ));
        assert!(opening_command_due_at(
            true,
            false,
            false,
            CMD_MIN_PROBE_SAMPLES,
            0,
            Some(submitted),
            due,
        ));
    }

    #[test]
    fn voice_clock_records_supplied_worker_time() {
        let now = worker_clock::WorkerInstant::from_duration_since_start(Duration::from_millis(42));
        let mut last_voice_at = worker_clock::WorkerInstant::ZERO;
        let mut heard_voice = false;
        let mut checked_this_pause = true;
        let mut speech_start_sample = None;

        observe_voice_chunk_at(
            &[0.25],
            &mut last_voice_at,
            now,
            &mut heard_voice,
            &mut checked_this_pause,
            |_| true,
            &mut speech_start_sample,
            CMD_PREROLL_SAMPLES,
        );

        assert_eq!(last_voice_at, now);
        assert!(heard_voice);
        assert!(!checked_this_pause);
        assert_eq!(speech_start_sample, Some(0));
    }
}

// F4(b) (Sol audit 2026-07-16): mpsc has no "peek and put back", so a command
// pulled here via `try_recv` (to gate a probe launch — see `probe_gate_clear`
// below) is stashed in `stash` rather than dropped. worker_commands.rs's
// dispatcher checks `stash` FIRST, before calling `recv_timeout` again, so
// the stashed command is handled on the very next loop iteration — no
// command is ever lost, and since this is the only call site that pulls from
// `cmd_rx` outside worker_commands.rs, at most one command is ever in flight
// through the stash at a time (ordering relative to the channel is preserved).
fn stash_pending_command(cmd_rx: &Receiver<WorkerCmd>, stash: &mut Option<WorkerCmd>) {
    if stash.is_none() {
        if let Ok(cmd) = cmd_rx.try_recv() {
            *stash = Some(cmd);
        }
    }
}

fn stop_or_cancel_stashed(stash: &Option<WorkerCmd>) -> bool {
    matches!(
        stash,
        Some(WorkerCmd::StopRecording { .. }) | Some(WorkerCmd::Cancel)
    )
}

/// True when it's safe to pay for a probe decode: peeks the command channel
/// and returns false if a Stop/Cancel is already queued, so a probe never
/// launches into a session that's about to end (the worker is single-threaded,
/// so a probe decode — bounded per F4(a) for the Windows Whisper CLI lane, but
/// still real cost for any backend — would otherwise delay Stop/Cancel
/// handling until it returns). A queued command of any OTHER kind is also
/// stashed (never dropped) but does not block the probe.
fn probe_gate_clear(cmd_rx: &Receiver<WorkerCmd>, stash: &mut Option<WorkerCmd>) -> bool {
    stash_pending_command(cmd_rx, stash);
    !stop_or_cancel_stashed(stash)
}

fn transcribe_probe_buffer(model: &mut AsrRuntime, buffer: &[f32]) -> Result<String, String> {
    transcribe_conditioned(buffer, BlankPolicy::Probe, |conditioned| {
        model.with_inference_lease("command_trigger_probe", |model| {
            crate::asr::apply_probe_context_bias(model);
            model.transcribe_under_lease(conditioned)
        })
    })
}

fn transcribe_file_buffer(
    model: &mut AsrRuntime,
    buffer: &[f32],
) -> Result<FileTranscript, String> {
    transcribe_conditioned(buffer, BlankPolicy::Final, |conditioned| {
        model.with_inference_lease("file_asr", |model| {
            crate::asr::apply_utterance_bias(model);
            model.transcribe_file_under_lease(conditioned)
        })
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlankPolicy {
    Final,
    Probe,
}

fn transcribe_conditioned<T, F>(
    buffer: &[f32],
    blank_policy: BlankPolicy,
    mut transcribe: F,
) -> Result<T, String>
where
    F: FnMut(&[f32]) -> Result<T, String>,
    T: TranscriptText,
{
    #[cfg(target_os = "macos")]
    let _asr_window_input = crate::coreml_asr::install_asr_window_input(
        crate::coreml_asr::AsrWindowInput::from_raw_pcm(
            buffer,
            Some(match blank_policy {
                BlankPolicy::Final => "final",
                BlankPolicy::Probe => "opening_probe",
            }),
        ),
    );
    let t = Instant::now();
    let audio_policy =
        std::env::var("HR_AUDIO_CONDITIONING_POLICY").unwrap_or_else(|_| "default".into());
    let conditioned =
        heardright_core::audio_conditioning::condition_for_asr(buffer, SAMPLE_RATE, &audio_policy);
    let raw_rms = heardright_core::audio_conditioning::rms(buffer);

    let conditioned_rms = heardright_core::audio_conditioning::rms(&conditioned);

    let result = transcribe(&conditioned);
    let (text_chars, text_words, text_empty) = result
        .as_ref()
        .map(|value| {
            let text = value.transcript_text();
            (
                text.chars().count(),
                text.split_whitespace().count(),
                text.trim().is_empty(),
            )
        })
        .unwrap_or((0, 0, true));
    tracing::info!(
        kind = ?blank_policy,
        decode_ms = t.elapsed().as_millis() as u64,
        samples = buffer.len(),
        audio_secs = buffer.len() as f32 / SAMPLE_RATE as f32,
        raw_rms,
        conditioned_rms,
        text_chars,
        text_words,
        text_empty,
        ok = result.is_ok(),
        "transcribe finished"
    );

    // Opening probes may remain blank. A final blank is an immediate, audible
    // error: live dictation never retries, reroutes, or replays audio.
    if matches!(result, Ok(ref value) if value.transcript_text().trim().is_empty()) {
        if blank_policy == BlankPolicy::Probe {
            return Ok(T::empty());
        }
        return Err(crate::asr::AUDIBLE_BLANK_TRANSCRIPTION_ERROR.to_string());
    }
    match result {
        Ok(value) if value.transcript_text().trim().is_empty() => Ok(value),
        other => other,
    }
}
