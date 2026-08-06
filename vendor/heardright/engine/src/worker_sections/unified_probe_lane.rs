#[derive(Debug)]
struct ProbeRequest {
    generation: u64,
    /// Scheduler-owned identity for one native KWS acoustic stream. Recording
    /// generation alone cannot distinguish an old in-flight decode after a
    /// same-recording reset/recovery snapshot.
    kws_stream_generation: u64,
    session_id: String,
    /// Worker decision time at submission. Replay supplies virtual time here;
    /// wall time below remains diagnostic-only mailbox timing.
    submitted_at: worker_clock::WorkerInstant,
    created_at: Instant,
    audio: Vec<f32>,
    speech_start_sample: Option<usize>,
    command_start: usize,
    stream_base_sample: usize,
    /// Absolute first sample accepted by native KWS for this stream. The
    /// worker verifies this at reset time & returns actual value with result.
    native_origin_sample: usize,
    total_samples: usize,
    pending_control_prefix: bool,
    lane: &'static str,
    probe_sequence: u64,
    diagnostic_sample: bool,
}

#[derive(Debug)]
struct ProbeResult {
    request: ProbeRequest,
    transcript: Result<String, String>,
    probe_ms: u64,
    measured_decode_duration: Duration,
    native_origin_sample: usize,
    /// Sherpa graph output is a closed, canonical command set. Preserve that
    /// typed decision instead of reclassifying its text through fuzzy ASR.
    native_control_intent: Option<heardright_core::text_pipeline::ControlIntent>,
    /// Absolute sample index into the FULL recording buffer where the trigger
    /// word ("zephyr" or a fuzzy near-homophone — see
    /// `heardright_core::text_pipeline::has_trailing_control_wake`) was heard
    /// to start, if this probe's decode carried timestamps and a trigger-shaped
    /// word was found in them. `None` when no cut should be attempted; see
    /// `trigger_onset_skip_reason` for why.
    trigger_onset_sample: Option<usize>,
    /// Set whenever `trigger_onset_sample` is `None`, so the consumer can log
    /// WHY the audio cut is being skipped without recomputing anything:
    /// `"no_timestamps"` (this ASR backend/decode did not produce timed
    /// pieces) or `"piece_not_found"` (timed pieces decoded fine, but nothing
    /// in them looked like the trigger word). The consumer additionally
    /// derives an `"implausible"` fallback reason itself, since that check
    /// needs the CURRENT buffer/speech-start state at consumption time, not
    /// what was true when this probe was decoded.
    trigger_onset_skip_reason: Option<&'static str>,
}

#[derive(Debug)]
struct IdleWakeRequest {
    audio: Vec<f32>,
    start_sample: usize,
    total_samples: usize,
}

#[derive(Debug)]
struct IdleWakeResult {
    hit: Result<Option<(usize, usize)>, String>,
    total_samples: usize,
    decode_ms: u64,
}

#[derive(Debug)]
struct OpeningCommandRequest {
    generation: u64,
    session_id: String,
    submitted_at: worker_clock::WorkerInstant,
    created_at: Instant,
    audio: Vec<f32>,
    speech_start_sample: Option<usize>,
    command_start: usize,
    total_samples: usize,
    pause_ready: bool,
    pending_prefix: bool,
    /// KWS probe that must finish empty before this request may act as a
    /// long-dictation control fallback. `None` is an ordinary opening command.
    fallback_after_kws_sequence: Option<u64>,
}

#[derive(Debug)]
struct OpeningCommandResult {
    request: OpeningCommandRequest,
    transcript: Result<String, String>,
    decode_ms: u64,
    measured_decode_duration: Duration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LaneCompletion {
    lane: worker_clock::ExecutorLane,
    submitted_at: worker_clock::WorkerInstant,
    measured_decode_duration: Duration,
}

fn should_use_fresh_confirmation_fallback(lane: &str, pending_prefix: bool) -> bool {
    pending_prefix && lane == "zephyr_tail_full_async"
}

impl LaneCompletion {
    fn schedule_on(self, schedule: &mut worker_clock::ReplayEventSchedule) {
        schedule.schedule_lane_completion(
            self.lane,
            self.submitted_at,
            self.measured_decode_duration,
        );
    }
}

trait HasLaneCompletion {
    fn lane_completion(&self) -> LaneCompletion;
}

impl HasLaneCompletion for ProbeResult {
    fn lane_completion(&self) -> LaneCompletion {
        LaneCompletion {
            lane: worker_clock::ExecutorLane::Kws,
            submitted_at: self.request.submitted_at,
            measured_decode_duration: self.measured_decode_duration,
        }
    }
}

impl HasLaneCompletion for OpeningCommandResult {
    fn lane_completion(&self) -> LaneCompletion {
        LaneCompletion {
            lane: worker_clock::ExecutorLane::MainAsr,
            submitted_at: self.request.submitted_at,
            measured_decode_duration: self.measured_decode_duration,
        }
    }
}

/// 320 ms @ 16 kHz. Subtracted from detected trigger onset before cutting
/// recording buffer, and biased EARLY on purpose: a leaked half-syllable
/// ahead of the trigger is still recoverable by `strip_fired_control_tail`,
/// but a swallowed word-final phoneme on the user's real last word is not.
const TRIGGER_CUT_GUARD_PAD_SAMPLES: usize = 5_120;

/// Reconstructs word boundaries from piece-level ASR timestamps (SentencePiece
/// `▁` convention: a piece whose display text starts with whitespace opens a
/// new word — both the CoreML and ONNX Parakeet decoders already follow this
/// convention, see `coreml_asr_sections/section02.rs::transcribe_pieces_timed`
/// and `parakeet-rs`'s `decoder_tdt.rs::decode_with_timestamps`) and returns
/// the ABSOLUTE sample index — into the FULL recording buffer, not just this
/// probe's audio slice — where the LAST trigger-shaped word begins. Taking the
/// LAST match (rather than the first) matches the control grammar, which
/// anchors the wake word immediately before the verb at the END of the
/// utterance.
fn find_trigger_onset_sample(
    stream_base_sample: usize,
    tokens: &[parakeet_rs::TimedToken],
) -> Option<usize> {
    let mut best_start_sec: Option<f32> = None;
    let mut current_word = String::new();
    let mut current_start_sec: f32 = 0.0;
    let mut has_word = false;

    for token in tokens {
        let starts_new_word = token.text.chars().next().is_some_and(char::is_whitespace);
        if starts_new_word && has_word {
            if word_looks_like_trigger(&current_word) {
                best_start_sec = Some(current_start_sec);
            }
            current_word.clear();
            has_word = false;
        }
        if !has_word {
            current_start_sec = token.start;
            has_word = true;
        }
        current_word.push_str(&token.text);
    }
    if has_word && word_looks_like_trigger(&current_word) {
        best_start_sec = Some(current_start_sec);
    }

    let onset_sec = best_start_sec?;
    let onset_samples_in_slice = (onset_sec.max(0.0) * SAMPLE_RATE as f32).round() as usize;
    Some(stream_base_sample.saturating_add(onset_samples_in_slice))
}

fn word_looks_like_trigger(word: &str) -> bool {
    let trimmed = word.trim();
    !trimmed.is_empty() && heardright_core::text_pipeline::has_trailing_control_wake(trimmed)
}

fn native_control_intent(text: &str) -> Option<heardright_core::text_pipeline::ControlIntent> {
    use heardright_core::text_pipeline::ControlIntent;
    match text.trim().to_ascii_lowercase().as_str() {
        "zephyr stop" => Some(ControlIntent::Stop),
        "zephyr send" => Some(ControlIntent::Send),
        "zephyr cancel" => Some(ControlIntent::Cancel),
        _ => None,
    }
}

fn incremental_probe_offset(
    processed_total: usize,
    snapshot_start: usize,
    snapshot_len: usize,
) -> Option<usize> {
    processed_total
        .checked_sub(snapshot_start)
        .map(|offset| offset.min(snapshot_len))
}

fn probe_reset_stream_base(
    requested_stream_base: usize,
    snapshot_start: usize,
    incremental_offset: Option<usize>,
) -> usize {
    if incremental_offset.is_some() {
        requested_stream_base
    } else {
        snapshot_start
    }
}

/// CPU KWS transcribe with piece timestamps so trigger onset can be located
/// without a second decode pass. Sherpa owns a continuous incremental stream:
/// conditioning each 20 ms delta independently resets HPF/DC/gain state and
/// destroys recall. Feed Sherpa raw PCM, matching production replay. Other
/// probe backends keep normal ASR conditioning.
fn transcribe_probe_buffer_timed(
    model: &mut AsrRuntime,
    buffer: &[f32],
) -> Result<TranscriptionResult, String> {
    if model.requires_timed_control_probe() {
        return model.transcribe_probe_result(buffer);
    }
    let audio_policy =
        std::env::var("HR_AUDIO_CONDITIONING_POLICY").unwrap_or_else(|_| "default".into());
    let conditioned =
        heardright_core::audio_conditioning::condition_for_asr(buffer, SAMPLE_RATE, &audio_policy);
    model.transcribe_probe_result(&conditioned)
}

#[derive(Debug, PartialEq)]
enum TriggerAudioCut {
    Cut { onset_sample: usize, cut_at: usize },
    EmptyPreCommand { onset_sample: usize },
    Skipped { reason: &'static str },
}

/// Decides whether/where to cut the recording buffer before the final decode.
/// `trigger_onset_sample`/`onset_skip_reason` come from the firing probe;
/// `speech_start_sample`/`buffer_len` reflect CURRENT state at consumption
/// time (the probe may have decoded slightly stale audio). An onset before
/// speech start or beyond the current buffer is nonsensical — never cut
/// below speech start, never produce an empty buffer, never panic.
fn resolve_trigger_audio_cut(
    trigger_onset_sample: Option<usize>,
    onset_skip_reason: Option<&'static str>,
    speech_start_sample: Option<usize>,
    buffer_len: usize,
    pad_samples: usize,
) -> TriggerAudioCut {
    let Some(onset) = trigger_onset_sample else {
        return TriggerAudioCut::Skipped {
            reason: onset_skip_reason.unwrap_or("no_timestamps"),
        };
    };
    let floor = speech_start_sample.unwrap_or(0);
    // VAD confirms speech after a short frame; native KWS intentionally has
    // 250 ms preroll. A command onset inside that bounded lead is valid.
    let max_vad_preroll = CMD_PREROLL_SAMPLES + SAMPLE_RATE as usize / 32;
    if onset > buffer_len || onset.saturating_add(max_vad_preroll) < floor {
        return TriggerAudioCut::Skipped {
            reason: "implausible",
        };
    }
    let cut_at = onset.saturating_sub(pad_samples).max(floor);
    // A command-only recording has valid zero pre-command audio. It must not
    // fall back to decoding command audio merely to strip text afterward.
    if cut_at == 0 {
        return TriggerAudioCut::EmptyPreCommand {
            onset_sample: onset,
        };
    }
    TriggerAudioCut::Cut {
        onset_sample: onset,
        cut_at,
    }
}

fn trace_trigger_audio_cut(
    session_id: &str,
    onset_sample: usize,
    pad_samples: usize,
    cut_at: usize,
    buffer_len: usize,
) {
    tracing::info!(
        onset_sample,
        pad_samples,
        cut_at,
        buffer_len,
        "trigger_audio_cut"
    );
    emit_command_probe(json!({
        "event": "trigger_audio_cut",
        "session_id": session_id,
        "onset_sample": onset_sample,
        "pad_samples": pad_samples,
        "cut_at": cut_at,
        "buffer_len": buffer_len,
    }));
}

fn trace_trigger_audio_cut_skipped(session_id: Option<&str>, reason: &'static str) {
    tracing::info!(reason, "trigger_audio_cut_skipped");
    emit_command_probe(json!({
        "event": "trigger_audio_cut_skipped",
        "session_id": session_id,
        "reason": reason,
    }));
}

enum MainAsrControl {
    Reload { reply: Sender<Result<bool, String>> },
    Cancel { generation: u64 },
    Shutdown,
}

enum FinalRequest {
    Dictation {
        generation: u64,
        audio: Vec<f32>,
        reply: Sender<Result<String, String>>,
    },
    File {
        audio: Vec<f32>,
        reply: Sender<Result<FileTranscript, String>>,
    },
}

struct BackgroundRequest {
    generation: u64,
    start_sample: usize,
    end_sample: usize,
    audio: Vec<f32>,
}

/// Commit only contiguous background windows. If one fails, caller retains it
/// for retry before advancing, so final decode never receives a silent gap.
fn commit_background_window<F>(
    scheduled: &mut ScheduledStatic15,
    request: &BackgroundRequest,
    decode: F,
) -> Result<String, String>
where
    F: FnOnce(&[f32]) -> Result<String, String>,
{
    let text = decode(&request.audio)?;
    scheduled.commit_window(request.start_sample, request.end_sample, &text)?;
    Ok(text)
}

enum MainAsrJob {
    Final(FinalRequest),
    Control(MainAsrControl),
    Opening(OpeningCommandRequest),
    Background(BackgroundRequest),
}

enum ProbeControl {
    Reload {
        reply: Sender<Result<bool, String>>,
    },
    StartIdleWake {
        threshold: f32,
        reply: Sender<Result<(), String>>,
    },
    StopIdleWake {
        reply: Sender<()>,
    },
    Reset,
    Shutdown,
}

enum ProbeJob {
    Control(ProbeControl),
    Probe(ProbeRequest),
    IdleWake(IdleWakeRequest),
}

#[derive(Default)]
struct MainAsrQueueState {
    final_requests: std::collections::VecDeque<FinalRequest>,
    controls: std::collections::VecDeque<MainAsrControl>,
    latest_opening: Option<OpeningCommandRequest>,
    background_requests: std::collections::VecDeque<BackgroundRequest>,
}

#[derive(Default)]
struct MainAsrMailbox {
    state: Mutex<MainAsrQueueState>,
    ready: parking_lot::Condvar,
}

impl MainAsrMailbox {
    fn submit_opening(&self, request: OpeningCommandRequest) {
        self.state.lock().latest_opening = Some(request);
        self.ready.notify_one();
    }

    fn submit_background(&self, request: BackgroundRequest) {
        self.state.lock().background_requests.push_back(request);
        self.ready.notify_one();
    }

    fn push_final(&self, final_request: FinalRequest) {
        let mut state = self.state.lock();
        state.latest_opening = None;
        state.background_requests.clear();
        state.final_requests.push_back(final_request);
        drop(state);
        self.ready.notify_one();
    }

    fn push_control(&self, control: MainAsrControl) {
        let mut state = self.state.lock();
        if matches!(control, MainAsrControl::Cancel { .. }) {
            state.latest_opening = None;
            state.background_requests.clear();
        }
        state.controls.push_back(control);
        drop(state);
        self.ready.notify_one();
    }

    fn recv(&self) -> MainAsrJob {
        let mut state = self.state.lock();
        loop {
            if let Some(final_request) = state.final_requests.pop_front() {
                return MainAsrJob::Final(final_request);
            }
            if let Some(control) = state.controls.pop_front() {
                return MainAsrJob::Control(control);
            }
            if let Some(request) = state.latest_opening.take() {
                return MainAsrJob::Opening(request);
            }
            if let Some(background_request) = state.background_requests.pop_front() {
                return MainAsrJob::Background(background_request);
            }
            self.ready.wait(&mut state);
        }
    }
}

#[derive(Default)]
struct ProbeQueueState {
    controls: std::collections::VecDeque<ProbeControl>,
    latest_probe: Option<ProbeRequest>,
    idle_wake: std::collections::VecDeque<IdleWakeRequest>,
}

#[derive(Default)]
struct ProbeMailbox {
    state: Mutex<ProbeQueueState>,
    ready: parking_lot::Condvar,
}

impl ProbeMailbox {
    fn submit_latest(&self, request: ProbeRequest) {
        trace_probe_lifecycle(&request, "queued", None, None);
        if let Some(replaced) = self.state.lock().latest_probe.replace(request) {
            trace_probe_lifecycle(&replaced, "pending_replaced", None, None);
        }
        self.ready.notify_one();
    }

    fn clear_pending(&self) {
        let mut state = self.state.lock();
        if let Some(cleared) = state.latest_probe.take() {
            trace_probe_lifecycle(&cleared, "pending_cleared", None, None);
        }
        state.idle_wake.clear();
    }

    fn submit_idle_wake(&self, mut request: IdleWakeRequest) {
        let mut state = self.state.lock();
        if let Some(previous) = state.idle_wake.back_mut() {
            if previous.total_samples == request.start_sample {
                previous.audio.append(&mut request.audio);
                previous.total_samples = request.total_samples;
                drop(state);
                self.ready.notify_one();
                return;
            }
        }
        state.idle_wake.push_back(request);
        drop(state);
        self.ready.notify_one();
    }

    fn push_control(&self, control: ProbeControl) {
        self.state.lock().controls.push_back(control);
        self.ready.notify_one();
    }

    fn recv(&self) -> ProbeJob {
        let mut state = self.state.lock();
        loop {
            if let Some(control) = state.controls.pop_front() {
                return ProbeJob::Control(control);
            }
            if let Some(request) = state.latest_probe.take() {
                trace_probe_lifecycle(&request, "inference_started", None, None);
                return ProbeJob::Probe(request);
            }
            if let Some(request) = state.idle_wake.pop_front() {
                return ProbeJob::IdleWake(request);
            }
            self.ready.wait(&mut state);
        }
    }
}

struct AsrExecutor {
    main_mailbox: Arc<MainAsrMailbox>,
    probe_mailbox: Arc<ProbeMailbox>,
    result_rx: Receiver<ProbeResult>,
    idle_wake_result_rx: Receiver<IdleWakeResult>,
    opening_result_rx: Receiver<OpeningCommandResult>,
    main_alive: Arc<std::sync::atomic::AtomicBool>,
    probe_alive: Arc<std::sync::atomic::AtomicBool>,
    scheduled_static15_enabled: bool,
}

struct AsrWorkerAlive(Arc<std::sync::atomic::AtomicBool>);

impl Drop for AsrWorkerAlive {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::Release);
    }
}

impl AsrExecutor {
    fn spawn(models_dir: PathBuf, ep: AsrEp) -> Result<Self, String> {
        let main_mailbox = Arc::new(MainAsrMailbox::default());
        let main_worker_mailbox = Arc::clone(&main_mailbox);
        let (main_ready_tx, main_ready_rx) = channel::<Result<bool, String>>();
        let (opening_result_tx, opening_result_rx) = channel::<OpeningCommandResult>();
        let main_alive = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let main_worker_alive = Arc::clone(&main_alive);
        let main_models_dir = models_dir.clone();
        #[cfg(all(test, any(target_os = "macos", target_os = "windows")))]
        let trigger_replay = TRIGGER_REPLAY_ACTIVE.load(std::sync::atomic::Ordering::Acquire);

        std::thread::Builder::new()
            .name("hr-main-asr-worker".to_string())
            .spawn(move || {
                let _alive_guard = AsrWorkerAlive(main_worker_alive);
                #[cfg(all(test, any(target_os = "macos", target_os = "windows")))]
                let mut model = if trigger_replay {
                    AsrRuntime::ProbeDisabled
                } else {
                    match AsrRuntime::load(&main_models_dir, ep) {
                        Ok(model) => model,
                        Err(error) => {
                            let _ = main_ready_tx
                                .send(Err(format!("main ASR model load: {error}")));
                            return;
                        }
                    }
                };
                #[cfg(not(all(test, any(target_os = "macos", target_os = "windows"))))]
                let mut model = match AsrRuntime::load(&main_models_dir, ep) {
                    Ok(model) => model,
                    Err(error) => {
                        let _ =
                            main_ready_tx.send(Err(format!("main ASR model load: {error}")));
                        return;
                    }
                };
                if let Err(error) = warm_main_asr(&mut model) {
                    let _ = main_ready_tx.send(Err(format!("main ASR warmup: {error}")));
                    return;
                }
                let scheduled_enabled = model.uses_scheduled_static15()
                    && std::env::var("HR_SCHEDULED_STATIC15").ok().as_deref() != Some("0");
                let mut last_model_key = asr_reload_key();
                let mut scheduled_static = ScheduledStatic15::default();
                let mut active_generation: Option<u64> = None;
                // Retain a failed window until it commits. Later windows stay
                // unprocessed rather than creating a gap in committed audio.
                let mut deferred_background: Option<BackgroundRequest> = None;
                let _ = main_ready_tx.send(Ok(scheduled_enabled));

                loop {
                    match main_worker_mailbox.recv() {
                        MainAsrJob::Final(FinalRequest::Dictation {
                            generation,
                            audio,
                            reply,
                        }) => {
                            if active_generation != Some(generation) {
                                scheduled_static.reset();
                            }
                            let result = finish_recording_transcript(
                                &mut model,
                                &mut scheduled_static,
                                &audio,
                                generation,
                            );
                            scheduled_static.reset();
                            active_generation = None;
                            deferred_background = None;
                            let _ = reply.send(result);
                        }
                        MainAsrJob::Final(FinalRequest::File { audio, reply }) => {
                            let result = transcribe_file_buffer(&mut model, &audio);
                            let _ = reply.send(result);
                        }
                        MainAsrJob::Control(MainAsrControl::Reload { reply }) => {
                            let result = reload_asr_if_changed(
                                &mut model,
                                &mut last_model_key,
                                &main_models_dir,
                                ep,
                            );
                            let _ = reply.send(result);
                        }
                        MainAsrJob::Control(MainAsrControl::Cancel { generation }) => {
                            if active_generation == Some(generation) {
                                scheduled_static.reset();
                                active_generation = None;
                                deferred_background = None;
                            }
                        }
                        MainAsrJob::Control(MainAsrControl::Shutdown) => break,
                        MainAsrJob::Opening(request) => {
                            let queue_ms = request.created_at.elapsed().as_millis() as u64;
                            let started = Instant::now();
                            let transcript = transcribe_opening_buffer(&mut model, &request.audio);
                            let measured_decode_duration = started.elapsed();
                            tracing::info!(
                                generation = request.generation,
                                queue_ms,
                                decode_ms = measured_decode_duration.as_millis() as u64,
                                "opening command main ASR decode complete"
                            );
                            let opening_result = OpeningCommandResult {
                                request,
                                transcript,
                                decode_ms: measured_decode_duration.as_millis() as u64,
                                measured_decode_duration,
                            };
                            #[cfg(test)]
                            {
                                let completion = opening_result.lane_completion();
                                replay_schedule_lane_completion(completion);
                                while !replay_lane_completion_ready(completion) {
                                    std::thread::yield_now();
                                }
                            }
                            let _ = opening_result_tx.send(opening_result);
                        }
                        MainAsrJob::Background(request) => {
                            if !scheduled_enabled {
                                continue;
                            }
                            if active_generation != Some(request.generation) {
                                scheduled_static.reset();
                                active_generation = Some(request.generation);
                                deferred_background = None;
                            }
                            if let Some(retry) = deferred_background.take() {
                                let started = Instant::now();
                                match commit_background_window(
                                    &mut scheduled_static,
                                    &retry,
                                    |audio| transcribe_buffer(&mut model, audio),
                                ) {
                                    Ok(text) => tracing::info!(
                                        generation = retry.generation,
                                        start_sample = retry.start_sample,
                                        end_sample = retry.end_sample,
                                        window_samples = retry.audio.len(),
                                        window_chars = text.chars().count(),
                                        window_words = text.split_whitespace().count(),
                                        decode_ms = started.elapsed().as_millis() as u64,
                                        "scheduled static-15 background window recovered"
                                    ),
                                    Err(error) => {
                                        tracing::error!(
                                            generation = retry.generation,
                                            start_sample = retry.start_sample,
                                            end_sample = retry.end_sample,
                                            window_samples = retry.audio.len(),
                                            error,
                                            "scheduled static-15 background retry failed; retaining contiguous tail for final decode"
                                        );
                                        deferred_background = Some(retry);
                                        continue;
                                    }
                                }
                            }
                            let started = Instant::now();
                            match commit_background_window(
                                &mut scheduled_static,
                                &request,
                                |audio| transcribe_buffer(&mut model, audio),
                            ) {
                                Ok(text) => {
                                        let (committed_chars, committed_words) =
                                            scheduled_static.committed_text_counts();
                                        tracing::info!(
                                            generation = request.generation,
                                            start_sample = request.start_sample,
                                            end_sample = request.end_sample,
                                            window_samples = request.audio.len(),
                                            window_chars = text.chars().count(),
                                            window_words = text.split_whitespace().count(),
                                            window_empty = text.trim().is_empty(),
                                            background_windows =
                                                scheduled_static.background_windows(),
                                            committed_samples =
                                                scheduled_static.committed_samples(),
                                            committed_chars,
                                            committed_words,
                                            decode_ms = started.elapsed().as_millis() as u64,
                                            "scheduled static-15 background decode complete"
                                        );
                                }
                                Err(error) => {
                                    tracing::error!(
                                        generation = request.generation,
                                        start_sample = request.start_sample,
                                        end_sample = request.end_sample,
                                        window_samples = request.audio.len(),
                                        error,
                                        "scheduled static-15 background window failed; retaining it for retry before later work"
                                    );
                                    deferred_background = Some(request);
                                }
                            }
                        }
                    }
                }
            })
            .map_err(|error| format!("spawn main ASR worker: {error}"))?;

        let scheduled_static15_enabled = main_ready_rx
            .recv()
            .map_err(|_| "main ASR worker exited before ready".to_string())??;

        let probe_mailbox = Arc::new(ProbeMailbox::default());
        let probe_worker_mailbox = Arc::clone(&probe_mailbox);
        let (result_tx, result_rx) = channel::<ProbeResult>();
        let (idle_wake_result_tx, idle_wake_result_rx) = channel::<IdleWakeResult>();
        let (probe_ready_tx, probe_ready_rx) = channel::<Result<(), String>>();
        let probe_alive = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let probe_worker_alive = Arc::clone(&probe_alive);

        std::thread::Builder::new()
            .name("hr-kws-worker".to_string())
            .spawn(move || {
                let _alive_guard = AsrWorkerAlive(probe_worker_alive);
                let mut probe_model = match AsrRuntime::load_probe(&models_dir, ep) {
                    Ok(probe_model) => probe_model,
                    Err(error) => {
                        let _ = probe_ready_tx.send(Err(format!("KWS model load: {error}")));
                        return;
                    }
                };
                if let Err(error) = transcribe_probe_buffer_timed(
                    &mut probe_model,
                    &vec![0.0; SAMPLE_RATE as usize],
                ) {
                    let _ = probe_ready_tx.send(Err(format!("KWS warmup: {error}")));
                    return;
                }
                if let Err(error) = probe_model.reset_probe_stream() {
                    let _ = probe_ready_tx.send(Err(format!("KWS stream reset: {error}")));
                    return;
                }
                crate::asr::apply_probe_context_bias(&mut probe_model);
                let mut probe_last_model_key = asr_reload_key();
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                let mut probe_session_id = String::new();
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                let mut probe_processed_total = 0usize;
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                let mut probe_stream_base_sample = 0usize;
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                let mut probe_stream_generation = 0u64;
                #[cfg(any(target_os = "macos", target_os = "windows"))]
                let mut probe_native_origin_sample = 0usize;
                let _ = probe_ready_tx.send(Ok(()));

                loop {
                    match probe_worker_mailbox.recv() {
                        ProbeJob::Control(ProbeControl::Reload { reply }) => {
                            let result = reload_probe_asr_if_changed(
                                &mut probe_model,
                                &mut probe_last_model_key,
                                &models_dir,
                                ep,
                            );
                            if matches!(result, Ok(true)) {
                                crate::asr::apply_probe_context_bias(&mut probe_model);
                                let _ = probe_model.reset_probe_stream();
                                #[cfg(any(target_os = "macos", target_os = "windows"))]
                                {
                                    probe_session_id.clear();
                                    probe_processed_total = 0;
                                    probe_stream_base_sample = 0;
                                    probe_stream_generation = 0;
                                    probe_native_origin_sample = 0;
                                }
                            }
                            let _ = reply.send(result);
                        }
                        ProbeJob::Control(ProbeControl::Reset) => {
                            let _ = probe_model.reset_probe_stream();
                            #[cfg(any(target_os = "macos", target_os = "windows"))]
                            {
                                probe_session_id.clear();
                                probe_processed_total = 0;
                                probe_stream_base_sample = 0;
                                probe_stream_generation = 0;
                                probe_native_origin_sample = 0;
                            }
                        }
                        ProbeJob::Control(ProbeControl::StartIdleWake { threshold, reply }) => {
                            let _ = reply.send(probe_model.start_idle_wake(threshold));
                        }
                        ProbeJob::Control(ProbeControl::StopIdleWake { reply }) => {
                            probe_model.stop_idle_wake();
                            let _ = reply.send(());
                        }
                        ProbeJob::Control(ProbeControl::Shutdown) => break,
                        ProbeJob::IdleWake(request) => {
                            let started = Instant::now();
                            let hit = probe_model.feed_idle_wake(
                                &request.audio,
                                request.start_sample,
                                request.total_samples,
                            );
                            let _ = idle_wake_result_tx.send(IdleWakeResult {
                                hit,
                                total_samples: request.total_samples,
                                decode_ms: started.elapsed().as_millis() as u64,
                            });
                        }
                        ProbeJob::Probe(request) => {
                            let started = Instant::now();
                            // Prefer the timestamped decode so the trigger word's onset
                            // is known without a second pass over the same audio (see
                            // `find_trigger_onset_sample`). Backends without timed
                            // support (Whisper CoreML/Win) return `Err` here immediately
                            // — no real decode is attempted — so falling back to the
                            // plain-text probe costs nothing extra on that path and
                            // command/trigger classification is completely unaffected.
                            let probe_model = &mut probe_model;
                            #[cfg(any(target_os = "macos", target_os = "windows"))]
                            let sherpa_cascade = probe_model.requires_timed_control_probe();
                            #[cfg(any(target_os = "macos", target_os = "windows"))]
                            let probe_offset = incremental_probe_offset(
                                probe_processed_total,
                                request.command_start,
                                request.audio.len(),
                            );
                            #[cfg(any(target_os = "macos", target_os = "windows"))]
                            let probe_reset = request.session_id != probe_session_id
                                || request.kws_stream_generation != probe_stream_generation
                                || (!sherpa_cascade
                                    && (request.stream_base_sample != probe_stream_base_sample
                                        || request.native_origin_sample
                                            != probe_native_origin_sample
                                        || probe_offset.is_none()));
                            #[cfg(any(target_os = "macos", target_os = "windows"))]
                            let reset_stream_base_sample = probe_reset_stream_base(
                                request.stream_base_sample,
                                request.command_start,
                                probe_offset,
                            );
                            #[cfg(any(target_os = "macos", target_os = "windows"))]
                            let prepare_error = if probe_reset {
                                match probe_model.reset_probe_stream() {
                                    Ok(()) => {
                                        probe_session_id.clone_from(&request.session_id);
                                        probe_stream_base_sample = reset_stream_base_sample;
                                        probe_stream_generation = request.kws_stream_generation;
                                        // Native reset feeds probe_audio from index zero,
                                        // whose absolute cursor is command_start.
                                        probe_native_origin_sample = request.command_start;
                                        None
                                    }
                                    Err(error) => Some(error),
                                }
                            } else {
                                None
                            };
                            #[cfg(any(target_os = "macos", target_os = "windows"))]
                            let probe_audio_offset = if sherpa_cascade || probe_reset {
                                0
                            } else {
                                probe_offset.unwrap_or(0)
                            };
                            #[cfg(any(target_os = "macos", target_os = "windows"))]
                            let probe_audio = &request.audio[probe_audio_offset..];
                            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                            let probe_audio = request.audio.as_slice();
                            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                            let prepare_error: Option<String> = None;
                            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                            let probe_stream_base_sample = request.command_start;
                            let (
                                transcript,
                                trigger_onset_sample,
                                trigger_onset_skip_reason,
                                native_control_intent,
                            ) = if let Some(error) = prepare_error {
                                (Err(error), None, Some("probe_stream_reset_failed"), None)
                            } else {
                                let timed_result = if sherpa_cascade
                                    && should_use_fresh_confirmation_fallback(
                                        request.lane,
                                        request.pending_control_prefix,
                                    ) {
                                    probe_native_origin_sample = request.command_start;
                                    match probe_model.transcribe_probe_fresh_confirmation_result(
                                        request.audio.as_slice(),
                                    ) {
                                        Ok(fresh) if fresh.text.trim().is_empty() => probe_model
                                            .transcribe_probe_cascade_result(
                                                request.audio.as_slice(),
                                                request.command_start,
                                                request.total_samples,
                                                request.pending_control_prefix,
                                            ),
                                        fresh => fresh,
                                    }
                                } else if sherpa_cascade {
                                    probe_native_origin_sample = request.command_start;
                                    probe_model.transcribe_probe_cascade_result(
                                        request.audio.as_slice(),
                                        request.command_start,
                                        request.total_samples,
                                        request.pending_control_prefix,
                                    )
                                } else {
                                    transcribe_probe_buffer_timed(probe_model, probe_audio)
                                };
                                match timed_result {
                                    Ok(timed) => {
                                        let onset = find_trigger_onset_sample(
                                            probe_native_origin_sample,
                                            &timed.tokens,
                                        );
                                        let skip_reason = if onset.is_some() {
                                            None
                                        } else {
                                            Some("piece_not_found")
                                        };
                                        let native_intent = native_control_intent(&timed.text);
                                        (Ok(timed.text), onset, skip_reason, native_intent)
                                    }
                                    Err(timed_err)
                                        if probe_model.requires_timed_control_probe() =>
                                    {
                                        // Never fire a Sherpa control without its emission-frame
                                        // timing. A text-only retry would make final TDT decode
                                        // command audio that must have been cut first.
                                        (Err(timed_err), None, Some("timed_probe_failed"), None)
                                    }
                                    Err(_timed_err) => {
                                        let text =
                                            transcribe_probe_buffer(probe_model, probe_audio);
                                        (text, None, Some("no_timestamps"), None)
                                    }
                                }
                            };
                            #[cfg(any(target_os = "macos", target_os = "windows"))]
                            if transcript.is_ok() {
                                probe_processed_total = request.total_samples;
                            } else {
                                // A failed timed KWS result can mean Sherpa found
                                // an implausibly long-lived keyword path. Drop all
                                // stream identity so the next rolling request
                                // rebuilds from its own snapshot start; retaining
                                // the old base would make new timestamps point at
                                // unrelated earlier audio.
                                let _ = probe_model.reset_probe_stream();
                                probe_session_id.clear();
                                probe_processed_total = 0;
                                probe_stream_base_sample = 0;
                                probe_stream_generation = 0;
                                probe_native_origin_sample = 0;
                            }
                            let probe_ms = started.elapsed().as_millis() as u64;
                            let measured_decode_duration = started.elapsed();
                            trace_probe_lifecycle(
                                &request,
                                "inference_finished",
                                Some(probe_ms),
                                Some(transcript.is_ok()),
                            );
                            let probe_result = ProbeResult {
                                request,
                                transcript,
                                probe_ms,
                                measured_decode_duration,
                                #[cfg(any(target_os = "macos", target_os = "windows"))]
                                native_origin_sample: probe_native_origin_sample,
                                #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                                native_origin_sample: request.command_start,
                                native_control_intent,
                                trigger_onset_sample,
                                trigger_onset_skip_reason,
                            };
                            #[cfg(test)]
                            {
                                let completion = probe_result.lane_completion();
                                replay_schedule_lane_completion(completion);
                                while !replay_lane_completion_ready(completion) {
                                    std::thread::yield_now();
                                }
                            }
                            let _ = result_tx.send(probe_result);
                        }
                    }
                }
            })
            .map_err(|error| format!("spawn KWS worker: {error}"))?;

        if let Err(error) = probe_ready_rx
            .recv()
            .map_err(|_| "KWS worker exited before ready".to_string())?
        {
            main_mailbox.push_control(MainAsrControl::Shutdown);
            return Err(error);
        }
        Ok(Self {
            main_mailbox,
            probe_mailbox,
            result_rx,
            idle_wake_result_rx,
            opening_result_rx,
            main_alive,
            probe_alive,
            scheduled_static15_enabled,
        })
    }

    fn submit_probe(&self, request: ProbeRequest) {
        #[cfg(test)]
        replay_mark_lane_in_flight(worker_clock::ExecutorLane::Kws, request.submitted_at);
        self.probe_mailbox.submit_latest(request);
    }

    fn start_idle_wake(&self, threshold: f32) -> Result<(), String> {
        let (reply, result) = channel();
        self.probe_mailbox
            .push_control(ProbeControl::StartIdleWake { threshold, reply });
        result
            .recv_timeout(Duration::from_secs(30))
            .map_err(|_| "idle wake start timed out".to_string())?
    }

    fn stop_idle_wake(&self) {
        self.probe_mailbox.clear_pending();
        let (reply, result) = channel();
        self.probe_mailbox
            .push_control(ProbeControl::StopIdleWake { reply });
        let _ = result.recv_timeout(Duration::from_secs(2));
    }

    fn submit_idle_wake(&self, audio: Vec<f32>, start_sample: usize, total_samples: usize) {
        if self.probe_alive.load(std::sync::atomic::Ordering::Acquire) {
            self.probe_mailbox.submit_idle_wake(IdleWakeRequest {
                audio,
                start_sample,
                total_samples,
            });
        }
    }

    fn try_recv_idle_wake(&self) -> Result<Option<IdleWakeResult>, String> {
        match self.idle_wake_result_rx.try_recv() {
            Ok(result) => Ok(Some(result)),
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Err("idle wake result channel disconnected".into())
            }
        }
    }

    fn submit_opening(&self, request: OpeningCommandRequest) {
        if self.main_alive.load(std::sync::atomic::Ordering::Acquire) {
            #[cfg(test)]
            replay_mark_lane_in_flight(worker_clock::ExecutorLane::MainAsr, request.submitted_at);
            self.main_mailbox.submit_opening(request);
        }
    }

    fn scheduled_static15_enabled(&self) -> bool {
        self.scheduled_static15_enabled
    }

    fn submit_background(
        &self,
        generation: u64,
        start_sample: usize,
        end_sample: usize,
        audio: Vec<f32>,
    ) {
        if self.scheduled_static15_enabled
            && self.main_alive.load(std::sync::atomic::Ordering::Acquire)
        {
            self.main_mailbox.submit_background(BackgroundRequest {
                generation,
                start_sample,
                end_sample,
                audio,
            });
        }
    }

    fn clear_pending(&self) {
        self.probe_mailbox.clear_pending();
        let mut state = self.main_mailbox.state.lock();
        state.latest_opening = None;
    }

    fn cancel_recording(&self, generation: u64) {
        self.main_mailbox
            .push_control(MainAsrControl::Cancel { generation });
    }

    fn reset_probe_stream(&self) {
        self.probe_mailbox.clear_pending();
        self.probe_mailbox.push_control(ProbeControl::Reset);
    }

    fn try_recv(&self) -> Result<Option<ProbeResult>, String> {
        match self.result_rx.try_recv() {
            Ok(result) => Ok(Some(result)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                Err("KWS worker result channel disconnected".to_string())
            }
        }
    }

    fn try_recv_opening(&self) -> Result<Option<OpeningCommandResult>, String> {
        match self.opening_result_rx.try_recv() {
            Ok(result) => Ok(Some(result)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                Err("main ASR opening result channel disconnected".to_string())
            }
        }
    }

    fn transcribe_final(&self, generation: u64, audio: Vec<f32>) -> Result<String, String> {
        if !self.main_alive.load(std::sync::atomic::Ordering::Acquire) {
            return Err("main ASR worker is not running".to_string());
        }
        let (reply_tx, reply_rx) = channel();
        self.main_mailbox.push_final(FinalRequest::Dictation {
            generation,
            audio,
            reply: reply_tx,
        });
        reply_rx
            .recv_timeout(FINAL_REPLY_TIMEOUT)
            .map_err(|error| match error {
                RecvTimeoutError::Timeout => "main ASR final timed out".to_string(),
                RecvTimeoutError::Disconnected => "main ASR final reply dropped".to_string(),
            })?
    }

    fn transcribe_file(&self, audio: Vec<f32>) -> Result<FileTranscript, String> {
        if !self.main_alive.load(std::sync::atomic::Ordering::Acquire) {
            return Err("main ASR worker is not running".to_string());
        }
        let (reply_tx, reply_rx) = channel();
        self.main_mailbox.push_final(FinalRequest::File {
            audio,
            reply: reply_tx,
        });
        reply_rx
            .recv_timeout(FILE_REPLY_TIMEOUT)
            .map_err(|error| match error {
                RecvTimeoutError::Timeout => "main ASR file transcription timed out".to_string(),
                RecvTimeoutError::Disconnected => "main ASR file reply dropped".to_string(),
            })?
    }

    fn reload_model(&self) -> Result<bool, String> {
        if !self.main_alive.load(std::sync::atomic::Ordering::Acquire)
            || !self.probe_alive.load(std::sync::atomic::Ordering::Acquire)
        {
            return Err("ASR workers are not running".to_string());
        }
        let (main_reply_tx, main_reply_rx) = channel();
        self.main_mailbox.push_control(MainAsrControl::Reload {
            reply: main_reply_tx,
        });
        let main_changed = main_reply_rx
            .recv_timeout(Duration::from_secs(30))
            .map_err(|error| match error {
                RecvTimeoutError::Timeout => "main ASR reload timed out".to_string(),
                RecvTimeoutError::Disconnected => "main ASR reload reply dropped".to_string(),
            })??;
        let (probe_reply_tx, probe_reply_rx) = channel();
        self.probe_mailbox.push_control(ProbeControl::Reload {
            reply: probe_reply_tx,
        });
        let probe_changed = probe_reply_rx
            .recv_timeout(Duration::from_secs(30))
            .map_err(|error| match error {
                RecvTimeoutError::Timeout => "KWS reload timed out".to_string(),
                RecvTimeoutError::Disconnected => "KWS reload reply dropped".to_string(),
            })??;
        Ok(main_changed || probe_changed)
    }

    fn shutdown(&self) {
        self.clear_pending();
        self.probe_mailbox.push_control(ProbeControl::Shutdown);
        self.main_mailbox.push_control(MainAsrControl::Shutdown);
    }
}

fn trace_probe_lifecycle(
    request: &ProbeRequest,
    phase: &'static str,
    probe_ms: Option<u64>,
    ok: Option<bool>,
) {
    if should_trace_probe_lifecycle(request, phase, ok) {
        emit_command_probe(probe_lifecycle_payload(request, phase, probe_ms, ok));
    }
}

fn should_trace_probe_lifecycle(
    request: &ProbeRequest,
    phase: &'static str,
    ok: Option<bool>,
) -> bool {
    request.diagnostic_sample
        || ok == Some(false)
        || matches!(phase, "pending_replaced" | "pending_cleared")
}

fn probe_lifecycle_payload(
    request: &ProbeRequest,
    phase: &'static str,
    probe_ms: Option<u64>,
    ok: Option<bool>,
) -> serde_json::Value {
    json!({
        "event": "probe_lifecycle",
        "phase": phase,
        "lane": request.lane,
        "session_id": request.session_id,
        "generation": request.generation,
        "kws_stream_generation": request.kws_stream_generation,
        "probe_sequence": request.probe_sequence,
        "diagnostic_sample": request.diagnostic_sample,
        "total_samples": request.total_samples,
        "command_samples": request.audio.len(),
        "command_start": request.command_start,
        "stream_base_sample": request.stream_base_sample,
        "native_origin_sample": request.native_origin_sample,
        // Age since submission. At `inference_started` this is mailbox wait: the
        // single-slot mailbox cannot accumulate, so a growing value here means
        // inference is the bottleneck and older snapshots are being replaced.
        "age_ms": request.created_at.elapsed().as_millis() as u64,
        "probe_ms": probe_ms,
        "ok": ok,
    })
}

fn probe_result_is_current(
    result: &ProbeResult,
    generation: u64,
    session_id: Option<&str>,
    kws_stream_generation: u64,
    _native_origin_sample: Option<usize>,
    live_total_samples: usize,
    now: worker_clock::WorkerInstant,
) -> bool {
    result.request.generation == generation
        && session_id == Some(result.request.session_id.as_str())
        && result.request.kws_stream_generation == kws_stream_generation
        && result.native_origin_sample == result.request.command_start
        && now.duration_since(result.request.submitted_at) <= Duration::from_secs(2)
        && live_total_samples.saturating_sub(result.request.total_samples)
            <= SAMPLE_RATE as usize * 2
}

fn opening_result_is_current(
    result: &OpeningCommandResult,
    generation: u64,
    session_id: Option<&str>,
) -> bool {
    result.request.generation == generation
        && session_id == Some(result.request.session_id.as_str())
}

fn latch_probe_lane_failure(failed: &mut bool) -> bool {
    if *failed {
        false
    } else {
        *failed = true;
        true
    }
}

#[cfg(test)]
mod unified_probe_lane_tests {
    use super::*;

    fn background_request(start_sample: usize, end_sample: usize) -> BackgroundRequest {
        BackgroundRequest {
            generation: 7,
            start_sample,
            end_sample,
            audio: vec![0.25; end_sample - start_sample],
        }
    }

    fn request(marker: u64) -> ProbeRequest {
        ProbeRequest {
            generation: 7,
            kws_stream_generation: 3,
            session_id: "session-7".into(),
            submitted_at: worker_clock::WorkerInstant::ZERO,
            created_at: Instant::now(),
            audio: vec![marker as f32],
            speech_start_sample: None,
            command_start: 0,
            stream_base_sample: 0,
            native_origin_sample: 0,
            total_samples: 1,
            pending_control_prefix: false,
            lane: "test",
            probe_sequence: marker,
            diagnostic_sample: marker == 1,
        }
    }

    fn opening_request(marker: u64) -> OpeningCommandRequest {
        OpeningCommandRequest {
            generation: 7,
            session_id: "session-7".into(),
            submitted_at: worker_clock::WorkerInstant::ZERO,
            created_at: Instant::now(),
            audio: vec![marker as f32],
            speech_start_sample: Some(0),
            command_start: 0,
            total_samples: marker as usize,
            pause_ready: false,
            pending_prefix: false,
            fallback_after_kws_sequence: None,
        }
    }

    #[test]
    fn opening_command_latest_snapshot_replaces_pending_audio() {
        let mailbox = MainAsrMailbox::default();
        mailbox.submit_opening(opening_request(1));
        mailbox.submit_opening(opening_request(2));

        let MainAsrJob::Opening(request) = mailbox.recv() else {
            panic!("expected opening command");
        };
        assert_eq!(request.audio, vec![2.0]);
    }

    #[test]
    fn idle_wake_mailbox_merges_contiguous_arrivals_without_dropping_pcm() {
        let mailbox = ProbeMailbox::default();
        mailbox.submit_idle_wake(IdleWakeRequest {
            audio: vec![1.0, 2.0],
            start_sample: 0,
            total_samples: 2,
        });
        mailbox.submit_idle_wake(IdleWakeRequest {
            audio: vec![3.0, 4.0],
            start_sample: 2,
            total_samples: 4,
        });

        let ProbeJob::IdleWake(request) = mailbox.recv() else {
            panic!("expected idle wake request");
        };
        assert_eq!(request.audio, vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!(request.start_sample, 0);
        assert_eq!(request.total_samples, 4);
    }

    #[test]
    fn opening_command_precedes_static15_background() {
        let mailbox = MainAsrMailbox::default();
        mailbox.submit_background(BackgroundRequest {
            generation: 7,
            start_sample: 0,
            end_sample: 1,
            audio: vec![1.0],
        });
        mailbox.submit_opening(opening_request(2));

        assert!(matches!(mailbox.recv(), MainAsrJob::Opening(_)));
        assert!(matches!(mailbox.recv(), MainAsrJob::Background(_)));
    }

    #[test]
    fn failed_background_window_retries_before_later_window_can_commit() {
        let mut scheduled = ScheduledStatic15::default();
        let first = background_request(0, 2);
        assert!(commit_background_window(&mut scheduled, &first, |_| Err("blank".into())).is_err());
        assert_eq!(scheduled.committed_samples(), 0);

        // The capture worker holds this request after a failure. Recover it
        // before accepting window two, preserving a contiguous final tail.
        let mut deferred = Some(first);
        let retry = deferred.take().expect("failed window retained");
        commit_background_window(&mut scheduled, &retry, |_| Ok("first".into()))
            .expect("retry commits first window");
        let second = background_request(2, 4);
        commit_background_window(&mut scheduled, &second, |_| Ok("second".into()))
            .expect("later window commits after recovery");

        assert_eq!(scheduled.committed_samples(), 4);
        assert_eq!(scheduled.committed_text_counts().1, 2);
    }

    #[test]
    fn opening_command_is_cleared_by_final_and_cancel() {
        let mailbox = MainAsrMailbox::default();
        mailbox.submit_opening(opening_request(1));
        let (reply, _rx) = channel();
        mailbox.push_final(FinalRequest::Dictation {
            generation: 7,
            audio: vec![2.0],
            reply,
        });
        assert!(matches!(mailbox.recv(), MainAsrJob::Final(_)));
        assert!(mailbox.state.lock().latest_opening.is_none());

        mailbox.submit_opening(opening_request(3));
        mailbox.push_control(MainAsrControl::Cancel { generation: 7 });
        assert!(matches!(
            mailbox.recv(),
            MainAsrJob::Control(MainAsrControl::Cancel { .. })
        ));
        assert!(mailbox.state.lock().latest_opening.is_none());
    }

    #[test]
    fn latest_probe_replaces_older_pending_audio() {
        let mailbox = ProbeMailbox::default();
        mailbox.submit_latest(request(1));
        mailbox.submit_latest(request(2));

        let ProbeJob::Probe(probe) = mailbox.recv() else {
            panic!("expected probe");
        };
        assert_eq!(probe.audio, vec![2.0]);
    }

    #[test]
    fn fresh_confirmation_fallback_requires_full_lane_and_armed_prefix() {
        assert!(should_use_fresh_confirmation_fallback(
            "zephyr_tail_full_async",
            true
        ));
        assert!(!should_use_fresh_confirmation_fallback(
            "zephyr_tail_async",
            true
        ));
        assert!(!should_use_fresh_confirmation_fallback(
            "zephyr_tail_full_async",
            false
        ));
    }

    #[test]
    fn virtual_decode_barrier_preserves_latest_pending_probe() {
        let controller = ReplayCaptureController::default();
        controller.load(vec![0.0; 640]);
        let mut capture = ReplayCapture {
            controller: controller.clone(),
        };
        capture.resume().unwrap();
        let mailbox = ProbeMailbox::default();
        mailbox.submit_latest(request(1));
        let ProbeJob::Probe(in_flight) = mailbox.recv() else {
            panic!("expected in-flight probe");
        };

        controller.mark_lane_in_flight(worker_clock::ExecutorLane::Kws, in_flight.submitted_at);
        controller.schedule_lane_completion(LaneCompletion {
            lane: worker_clock::ExecutorLane::Kws,
            submitted_at: in_flight.submitted_at,
            measured_decode_duration: Duration::from_millis(10),
        });
        mailbox.submit_latest(request(2));
        mailbox.submit_latest(request(3));

        assert_eq!(capture.read_f32(320).unwrap().len(), 320);
        assert!(capture.read_f32(320).unwrap().is_empty());
        assert!(controller
            .lane_completion_ready(worker_clock::ExecutorLane::Kws, in_flight.submitted_at,));
        let ProbeJob::Probe(next) = mailbox.recv() else {
            panic!("expected pending probe after virtual completion");
        };
        assert_eq!(next.probe_sequence, 3);
        assert_eq!(capture.read_f32(320).unwrap().len(), 320);
    }

    #[test]
    fn lifecycle_control_preempts_pending_probe() {
        let mailbox = ProbeMailbox::default();
        mailbox.submit_latest(request(1));
        let (reply, _rx) = channel();
        mailbox.push_control(ProbeControl::Reload { reply });

        assert!(matches!(
            mailbox.recv(),
            ProbeJob::Control(ProbeControl::Reload { .. })
        ));
        assert!(matches!(mailbox.recv(), ProbeJob::Probe(_)));
    }

    #[test]
    fn final_request_clears_pending_background_and_dequeues_first() {
        let mailbox = MainAsrMailbox::default();
        mailbox.submit_background(BackgroundRequest {
            generation: 7,
            start_sample: 0,
            end_sample: 1,
            audio: vec![1.0],
        });
        let (reply, _rx) = channel();
        mailbox.push_final(FinalRequest::Dictation {
            generation: 7,
            audio: vec![2.0],
            reply,
        });

        assert!(matches!(
            mailbox.recv(),
            MainAsrJob::Final(FinalRequest::Dictation { .. })
        ));
        assert!(mailbox.state.lock().background_requests.is_empty());
    }

    #[test]
    fn generation_and_session_reject_stale_results() {
        let result = ProbeResult {
            request: request(1),
            transcript: Ok("zephyr stop".into()),
            probe_ms: 10,
            measured_decode_duration: Duration::from_millis(10),
            native_origin_sample: 0,
            native_control_intent: None,
            trigger_onset_sample: None,
            trigger_onset_skip_reason: None,
        };
        assert!(probe_result_is_current(
            &result,
            7,
            Some("session-7"),
            3,
            Some(0),
            1,
            worker_clock::WorkerInstant::ZERO,
        ));
        assert!(!probe_result_is_current(
            &result,
            8,
            Some("session-7"),
            3,
            Some(0),
            1,
            worker_clock::WorkerInstant::ZERO,
        ));
        assert!(!probe_result_is_current(
            &result,
            7,
            Some("session-8"),
            3,
            Some(0),
            1,
            worker_clock::WorkerInstant::ZERO,
        ));
        assert!(!probe_result_is_current(
            &result,
            7,
            Some("session-7"),
            4,
            Some(0),
            1,
            worker_clock::WorkerInstant::ZERO,
        ));

        let opening = OpeningCommandResult {
            request: opening_request(1),
            transcript: Ok("open safari".into()),
            decode_ms: 10,
            measured_decode_duration: Duration::from_millis(10),
        };
        assert!(opening_result_is_current(&opening, 7, Some("session-7")));
        assert!(!opening_result_is_current(&opening, 8, Some("session-7")));
        assert!(!opening_result_is_current(&opening, 7, Some("session-8")));

        assert_eq!(
            result.lane_completion(),
            LaneCompletion {
                lane: worker_clock::ExecutorLane::Kws,
                submitted_at: worker_clock::WorkerInstant::ZERO,
                measured_decode_duration: Duration::from_millis(10),
            }
        );
        assert_eq!(
            opening.lane_completion(),
            LaneCompletion {
                lane: worker_clock::ExecutorLane::MainAsr,
                submitted_at: worker_clock::WorkerInstant::ZERO,
                measured_decode_duration: Duration::from_millis(10),
            }
        );

        let mut schedule = worker_clock::ReplayEventSchedule::default();
        result.lane_completion().schedule_on(&mut schedule);
        assert_eq!(
            schedule.next_event(),
            Some(worker_clock::ReplayEvent {
                at: worker_clock::WorkerInstant::ZERO.saturating_add(Duration::from_millis(10)),
                kind: worker_clock::ReplayEventKind::LaneCompletion(
                    worker_clock::ExecutorLane::Kws,
                ),
            })
        );
    }

    #[test]
    fn probe_staleness_uses_worker_decision_time() {
        let result = ProbeResult {
            request: request(1),
            transcript: Ok("zephyr stop".into()),
            probe_ms: 10,
            measured_decode_duration: Duration::from_millis(10),
            native_origin_sample: 0,
            native_control_intent: None,
            trigger_onset_sample: None,
            trigger_onset_skip_reason: None,
        };
        let virtual_now =
            worker_clock::WorkerInstant::ZERO.saturating_add(Duration::from_millis(2_001));
        assert!(!probe_result_is_current(
            &result,
            7,
            Some("session-7"),
            3,
            Some(0),
            1,
            virtual_now,
        ));
    }

    #[test]
    fn rolling_snapshot_feeds_only_audio_after_last_processed_sample() {
        assert_eq!(
            incremental_probe_offset(160_000, 40_000, 124_000),
            Some(120_000)
        );
        assert_eq!(incremental_probe_offset(160_000, 164_000, 8_000), None);
        assert_eq!(
            incremental_probe_offset(160_000, 40_000, 100_000),
            Some(100_000)
        );
        assert_eq!(probe_reset_stream_base(40_000, 164_000, None), 164_000);
        assert_eq!(
            probe_reset_stream_base(40_000, 40_000, Some(120_000)),
            40_000
        );
    }

    #[test]
    fn dead_probe_worker_is_reported_instead_of_hanging_reload() {
        let main_mailbox = Arc::new(MainAsrMailbox::default());
        let probe_mailbox = Arc::new(ProbeMailbox::default());
        let (result_tx, result_rx) = channel();
        drop(result_tx);
        let (idle_wake_result_tx, idle_wake_result_rx) = channel();
        drop(idle_wake_result_tx);
        let (opening_result_tx, opening_result_rx) = channel();
        drop(opening_result_tx);
        let lane = AsrExecutor {
            main_mailbox,
            probe_mailbox,
            result_rx,
            idle_wake_result_rx,
            opening_result_rx,
            main_alive: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            probe_alive: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            scheduled_static15_enabled: false,
        };

        assert!(lane.reload_model().unwrap_err().contains("not running"));
        assert!(lane.try_recv().unwrap_err().contains("disconnected"));
    }

    #[test]
    fn probe_worker_failure_is_reported_only_once() {
        let mut failed = false;
        assert!(latch_probe_lane_failure(&mut failed));
        assert!(!latch_probe_lane_failure(&mut failed));
    }

    #[test]
    fn probe_lifecycle_is_session_correlated_and_content_free() {
        let request = request(3);
        let payload = probe_lifecycle_payload(&request, "queued", None, None);
        assert_eq!(payload["session_id"], "session-7");
        assert_eq!(payload["generation"], 7);
        assert_eq!(payload["total_samples"], 1);
        assert_eq!(payload["phase"], "queued");
        assert!(payload.get("recognized_text").is_none());
        assert!(payload.get("transcript").is_none());
    }

    #[test]
    fn routine_probe_lifecycle_is_sampled_but_anomalies_are_not() {
        let routine = request(3);
        assert!(!should_trace_probe_lifecycle(
            &routine,
            "inference_finished",
            Some(true)
        ));
        assert!(should_trace_probe_lifecycle(
            &routine,
            "inference_finished",
            Some(false)
        ));
        assert!(should_trace_probe_lifecycle(
            &routine,
            "pending_replaced",
            None
        ));
        assert!(should_trace_probe_lifecycle(
            &request(1),
            "inference_finished",
            Some(true)
        ));
    }

    // --- trigger onset location + audio-cut arithmetic (2026-07-27 fix) ---

    fn request_at(command_start: usize) -> ProbeRequest {
        let mut req = request(1);
        req.command_start = command_start;
        req.stream_base_sample = command_start;
        req
    }

    fn tok(text: &str, start: f32, end: f32) -> parakeet_rs::TimedToken {
        parakeet_rs::TimedToken {
            text: text.to_string(),
            start,
            end,
        }
    }

    #[test]
    fn finds_trigger_onset_and_offsets_by_command_start() {
        // " please zephyr stop" split into pieces the way both decoders emit
        // them: a leading-space piece opens each new word.
        let tokens = vec![
            tok(" please", 0.0, 0.4),
            tok(" zephyr", 0.48, 0.9),
            tok(" stop", 0.96, 1.2),
        ];
        let request = request_at(16_000); // 1.0s into the full recording buffer
        let onset = find_trigger_onset_sample(request.command_start, &tokens);
        // 0.48s @ 16kHz = 7680 samples, offset by command_start.
        assert_eq!(onset, Some(16_000 + 7_680));
    }

    #[test]
    fn no_trigger_shaped_word_returns_none() {
        let tokens = vec![tok(" please", 0.0, 0.4), tok(" stop", 0.48, 0.9)];
        let request = request_at(0);
        assert_eq!(
            find_trigger_onset_sample(request.command_start, &tokens),
            None
        );
    }

    #[test]
    fn empty_tokens_return_none() {
        let request = request_at(0);
        assert_eq!(find_trigger_onset_sample(request.command_start, &[]), None);
    }

    #[test]
    fn picks_the_last_trigger_shaped_word_not_the_first() {
        // "zephyr finds zephyr stop" — only the SECOND "zephyr" is the real
        // trigger (anchored right before the verb); an earlier incidental
        // occurrence must not win.
        let tokens = vec![
            tok(" zephyr", 0.0, 0.4),
            tok(" finds", 0.48, 0.9),
            tok(" zephyr", 0.96, 1.4),
            tok(" stop", 1.44, 1.7),
        ];
        let request = request_at(0);
        let onset = find_trigger_onset_sample(request.command_start, &tokens);
        assert_eq!(onset, Some((0.96 * SAMPLE_RATE as f32).round() as usize));
    }

    #[test]
    fn multi_piece_word_uses_first_piece_start() {
        // A trigger word split across subword pieces ("▁ze" + "pher", no
        // leading space on the continuation piece) must still be timed from
        // its FIRST piece, not its last.
        let tokens = vec![tok(" ze", 0.5, 0.6), tok("pher", 0.6, 0.9)];
        let request = request_at(0);
        let onset = find_trigger_onset_sample(request.command_start, &tokens);
        assert_eq!(onset, Some((0.5 * SAMPLE_RATE as f32).round() as usize));
    }

    #[test]
    fn sherpa_split_zephyr_pieces_cut_before_the_wake_word() {
        // Split Zephyr phones retain first emission time. This must resolve to
        // first frame, then apply validated 320 ms early pad.
        let tokens = vec![
            tok(" dictated", 3.52, 3.76),
            tok(" z", 3.84, 3.92),
            tok("ep", 3.84, 3.92),
            tok("h", 4.08, 4.16),
            tok("y", 4.08, 4.16),
            tok("r", 4.16, 4.24),
            tok(" st", 4.40, 4.48),
            tok("op", 4.40, 4.48),
        ];
        let request = request_at(32_000);
        let onset = find_trigger_onset_sample(request.command_start, &tokens);
        assert_eq!(onset, Some(32_000 + 61_440));
        assert!(matches!(
            resolve_trigger_audio_cut(onset, None, Some(1_000), 120_000, 5_120),
            TriggerAudioCut::Cut {
                onset_sample: 93_440,
                cut_at: 88_320,
            }
        ));
    }

    #[test]
    fn live_long_silence_regression_keeps_one_absolute_sample_coordinate() {
        // 2026-07-29 session-3: app-owned stream began at sample 275232
        // after a long blank; Sherpa emitted Zephyr at +0.40s. The guarded
        // cut must resolve to sample 276512, before command speech, rather
        // than the field-failure cut at 283519 that leaked "Exact".
        let tokens = vec![tok(" zephyr", 0.40, 1.04)];
        let onset = find_trigger_onset_sample(275_232, &tokens);
        assert_eq!(onset, Some(281_632));
        assert_eq!(
            resolve_trigger_audio_cut(onset, None, Some(0), 303_359, TRIGGER_CUT_GUARD_PAD_SAMPLES,),
            TriggerAudioCut::Cut {
                onset_sample: 281_632,
                cut_at: 276_512,
            }
        );
    }

    #[test]
    fn resolves_cut_with_early_biased_guard_pad() {
        let cut = resolve_trigger_audio_cut(
            Some(20_000),
            None,
            Some(1_000),
            30_000,
            TRIGGER_CUT_GUARD_PAD_SAMPLES,
        );
        match cut {
            TriggerAudioCut::Cut {
                onset_sample,
                cut_at,
            } => {
                assert_eq!(onset_sample, 20_000);
                assert_eq!(cut_at, 20_000 - TRIGGER_CUT_GUARD_PAD_SAMPLES);
            }
            TriggerAudioCut::Skipped { .. } | TriggerAudioCut::EmptyPreCommand { .. } => {
                panic!("expected a cut")
            }
        }
    }

    #[test]
    fn cut_never_goes_below_speech_start() {
        // Onset is only 500 samples after speech start; a full pad would cut
        // BEFORE speech start, which must clamp to the floor instead.
        let cut = resolve_trigger_audio_cut(Some(1_500), None, Some(1_000), 30_000, 1_920);
        match cut {
            TriggerAudioCut::Cut { cut_at, .. } => assert_eq!(cut_at, 1_000),
            TriggerAudioCut::Skipped { .. } | TriggerAudioCut::EmptyPreCommand { .. } => {
                panic!("expected a cut clamped to speech start")
            }
        }
    }

    #[test]
    fn skips_with_no_timestamps_reason_when_onset_missing_and_reason_unset() {
        let cut = resolve_trigger_audio_cut(None, None, Some(1_000), 30_000, 1_920);
        assert!(matches!(
            cut,
            TriggerAudioCut::Skipped {
                reason: "no_timestamps"
            }
        ));
    }

    #[test]
    fn skips_with_piece_not_found_reason_when_probe_reported_it() {
        let cut =
            resolve_trigger_audio_cut(None, Some("piece_not_found"), Some(1_000), 30_000, 1_920);
        assert!(matches!(
            cut,
            TriggerAudioCut::Skipped {
                reason: "piece_not_found"
            }
        ));
    }

    #[test]
    fn skips_as_implausible_when_onset_precedes_vad_preroll() {
        let cut = resolve_trigger_audio_cut(Some(500), None, Some(10_000), 30_000, 1_920);
        assert!(matches!(
            cut,
            TriggerAudioCut::Skipped {
                reason: "implausible"
            }
        ));
    }

    #[test]
    fn skips_as_implausible_when_onset_exceeds_buffer_len() {
        let cut = resolve_trigger_audio_cut(Some(40_000), None, Some(1_000), 30_000, 1_920);
        assert!(matches!(
            cut,
            TriggerAudioCut::Skipped {
                reason: "implausible"
            }
        ));
    }

    #[test]
    fn never_panics_with_no_speech_start_sample_and_defaults_floor_to_zero() {
        // speech_start_sample is None whenever VAD never confirmed speech
        // (e.g. the standalone-command window fallback) — the floor must
        // default to 0, not panic on `unwrap`, and an onset far enough past
        // it still produces a normal cut.
        let cut = resolve_trigger_audio_cut(Some(5_000), None, None, 30_000, 1_920);
        match cut {
            TriggerAudioCut::Cut { cut_at, .. } => assert_eq!(cut_at, 5_000 - 1_920),
            TriggerAudioCut::Skipped { .. } | TriggerAudioCut::EmptyPreCommand { .. } => {
                panic!("expected a cut")
            }
        }
    }

    #[test]
    fn command_only_audio_returns_typed_empty_pre_command() {
        let cut = resolve_trigger_audio_cut(Some(100), None, None, 30_000, 1_920);
        assert!(matches!(
            cut,
            TriggerAudioCut::EmptyPreCommand { onset_sample: 100 }
        ));
    }

    #[test]
    fn current_result_requires_generation_snapshot_origin_and_live_lag() {
        let mut result = ProbeResult {
            request: request(1),
            transcript: Ok("zephyr stop".into()),
            probe_ms: 10,
            measured_decode_duration: Duration::from_millis(10),
            native_origin_sample: 0,
            native_control_intent: None,
            trigger_onset_sample: Some(100),
            trigger_onset_skip_reason: None,
        };
        assert!(probe_result_is_current(
            &result,
            7,
            Some("session-7"),
            3,
            Some(0),
            32_000,
            worker_clock::WorkerInstant::ZERO,
        ));
        assert!(!probe_result_is_current(
            &result,
            7,
            Some("session-7"),
            2,
            Some(0),
            1,
            worker_clock::WorkerInstant::ZERO,
        ));
        // Fresh snapshots legitimately move scheduler origin while an older
        // result is in flight. Result must match its own snapshot start; latest
        // scheduler origin is not command authority.
        assert!(probe_result_is_current(
            &result,
            7,
            Some("session-7"),
            3,
            Some(1),
            1,
            worker_clock::WorkerInstant::ZERO,
        ));
        result.native_origin_sample = 1;
        assert!(!probe_result_is_current(
            &result,
            7,
            Some("session-7"),
            3,
            Some(1),
            1,
            worker_clock::WorkerInstant::ZERO,
        ));
        result.native_origin_sample = 0;
        assert!(!probe_result_is_current(
            &result,
            7,
            Some("session-7"),
            3,
            Some(0),
            SAMPLE_RATE as usize * 2 + 2,
            worker_clock::WorkerInstant::ZERO,
        ));
    }

    #[test]
    fn native_graph_result_maps_only_canonical_typed_intents() {
        use heardright_core::text_pipeline::ControlIntent;
        assert_eq!(
            native_control_intent("zephyr send"),
            Some(ControlIntent::Send)
        );
        assert_eq!(native_control_intent("zephyr zipper"), None);
    }

    #[test]
    fn reset_origin_is_first_sample_actually_fed_not_old_stream_base() {
        assert_eq!(probe_reset_stream_base(6_559, 32_000, Some(4_000)), 6_559);
        // A recovery snapshot forces a reset; timestamp origin is snapshot
        // start, never stale requested stream base.
        assert_eq!(probe_reset_stream_base(6_559, 32_000, None), 32_000);
    }
}
