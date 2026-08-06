fn worker_main(
    models_dir: PathBuf,
    ep: AsrEp,
    focus: Arc<Mutex<FocusTracker>>,
    cmd_rx: Receiver<WorkerCmd>,
    event_tx: Sender<WorkerEvent>,
    ready_tx: Sender<Result<(), String>>,
    decision_clock: Arc<dyn worker_clock::WorkerClock>,
) {
    crate::app_launch::prewarm();
    let asr_executor = match AsrExecutor::spawn(models_dir.clone(), ep) {
        Ok(executor) => executor,
        Err(message) => {
            tracing::error!("ASR executor failed: {message}");
            let _ = ready_tx.send(Err(message.clone()));
            let _ = event_tx.send(WorkerEvent::StartFailed { message });
            return;
        }
    };
    // Warm Harper off the first-dictation path after ASR is resident.
    let polish_warm = Instant::now();
    crate::text_polish::warm();
    tracing::info!(
        "worker polish warmup ok in {} ms",
        polish_warm.elapsed().as_millis()
    );
    let mut speech_vad = crate::vad::SpeechVad::new();

    // Model prewarm must not cross the microphone privacy boundary. Open capture
    // only after an explicit StartRecording command; its blocking seed absorbs
    // device spin-up without clipping the leading word.
    let initial_saved_device = crate::settings::input_device();
    let mut last_saved_device = initial_saved_device;
    let (mut capture, mut last_capture_route) = (None, None);
    let _ = ready_tx.send(Ok(()));
    let mut recording = false;
    let mut wake_listening = false;
    let mut wake_start_pending = false;
    let mut wake_threshold = 0.25f32;
    let mut wake_fire_count = 0u64;
    let mut wake_diagnostics: Option<WakeDiagnostics> = None;
    let mut idle_sample_clock = 0usize;
    let mut idle_audio_window: std::collections::VecDeque<f32> =
        std::collections::VecDeque::with_capacity(SAMPLE_RATE as usize * 4);
    let mut idle_audio_origin = 0usize;
    let mut wake_handoff_audio: Vec<f32> = Vec::with_capacity(SAMPLE_RATE as usize * 2);
    let mut recording_session: Option<String> = None;
    let mut buffer: Vec<f32> = Vec::with_capacity(SAMPLE_RATE as usize * 30);
    let mut previous_capture_block: Option<CapturePcmHistory> = None;
    let mut scheduled_submitted_samples = 0usize;
    let mut voice_ema: f32 = 0.0;
    let mut last_level_emit = Instant::now() - Duration::from_millis(120);
    // Streaming auto-fire state: `last_voice_at` = when speech last occurred;
    // `checked_this_pause` = a terminal or in-flight check for current silence.
    // An incomplete command prefix reopens it only after fresh audio arrives, so
    // the decoder can settle without repeatedly transcribing one idle snapshot.
    // `auto_stop_fired` = guard so a session auto-stops at most once.
    let mut last_voice_at = decision_clock.now();
    let mut checked_this_pause = false;
    let mut auto_stop_fired = false;
    let mut pending_command_prefix_since: Option<worker_clock::WorkerInstant> = None;
    let mut heard_voice = false;
    let mut speech_start_sample: Option<usize> = None;
    let mut kws_stream_generation = 0u64;
    let mut kws_native_origin_sample: Option<usize> = None;
    let mut kws_probe_count = 0u64;
    #[cfg(target_os = "windows")]
    let mut last_full_tail_probe_samples = 0usize;
    let mut pending_control_prefix_since: Option<worker_clock::WorkerInstant> = None;
    let mut pending_main_asr_control_result: Option<OpeningCommandResult> = None;
    // Every standalone action needs a second opening probe with the same
    // normalized action before it can stop recording.
    let mut pending_opening_action: Option<PendingOpeningAction> = None;
    let mut last_cmd_check: Option<worker_clock::WorkerInstant> = None;
    let mut last_command_probe_samples = 0usize;
    // M13: throttles `focus.lock().refresh_current()` in worker_streaming.rs.
    // Starts already-elapsed so the very first streaming iteration still
    // refreshes promptly.
    let mut last_focus_refresh: Option<worker_clock::WorkerInstant> = None;
    // F4(b) (Sol audit 2026-07-16): mpsc has no "peek and put back", so a
    // command pulled via `try_recv` (to gate a probe launch — see
    // `stash_pending_command` below) is stashed here instead of dropped.
    // worker_commands.rs's dispatcher checks this FIRST, before calling
    // `recv_timeout` again, so a stashed command is handled on the very next
    // loop iteration and no command is ever lost or reordered.
    let mut pending_cmd_peek: Option<WorkerCmd> = None;
    let mut recording_generation = 0u64;
    let mut asr_executor_failed = false;

    loop {
        let mut audio_arrived_this_loop = false;
        include!("worker_commands.rs");
        include!("worker_opening_results.rs");
        include!("worker_probe_results.rs");
        include!("worker_streaming.rs");
    }
}
