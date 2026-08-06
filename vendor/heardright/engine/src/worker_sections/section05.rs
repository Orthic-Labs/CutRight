#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_diagnostics_are_numeric_only_and_identify_a_live_audio_kws_path() {
        let mut diagnostics = WakeDiagnostics::new(0.25, true);
        diagnostics.observe_audio(&[0.25, -0.25, 0.0, 0.0]);
        diagnostics.observe_decode(7);
        let value = diagnostics.payload("observing", "no_match", "info", "wake_observing");
        assert_eq!(value["event"], "wake_diagnostic");
        assert_eq!(value["mic_opened"], true);
        assert_eq!(value["audio_samples"], 4);
        assert_eq!(value["kws_decode_attempts"], 1);
        assert_eq!(value["score_available"], false);
        assert!(value.get("audio").is_none());
        assert!(value.get("transcript").is_none());
    }

    #[test]
    fn quiet_final_audio_still_reaches_decoder() {
        let audio = vec![0.001; SAMPLE_RATE as usize];
        let mut calls = 0;
        let result = transcribe_conditioned(&audio, BlankPolicy::Final, |_| {
            calls += 1;
            Ok("decoded".to_string())
        });
        assert_eq!(result, Ok("decoded".to_string()));
        assert_eq!(calls, 1);
    }

    #[test]
    fn blank_final_returns_audible_error_without_retry() {
        let audio = vec![0.001; SAMPLE_RATE as usize];
        let mut calls = 0;
        let result = transcribe_conditioned(&audio, BlankPolicy::Final, |_| {
            calls += 1;
            Ok(String::new())
        });
        assert_eq!(
            result,
            Err(crate::asr::AUDIBLE_BLANK_TRANSCRIPTION_ERROR.to_string())
        );
        assert_eq!(calls, 1);
    }

    #[test]
    fn silence_policy_discards_only_before_voice_then_stops_after_voice() {
        assert_eq!(
            silence_decision(false, Duration::from_millis(29_999)),
            SilenceDecision::KeepRecording
        );
        assert_eq!(
            silence_decision(false, Duration::from_millis(30_000)),
            SilenceDecision::DiscardInitialSilence
        );
        assert_eq!(
            silence_decision(true, Duration::from_millis(29_999)),
            SilenceDecision::KeepRecording
        );
        assert_eq!(
            silence_decision(true, Duration::from_millis(30_000)),
            SilenceDecision::KeepRecording
        );
        assert_eq!(
            silence_decision(true, Duration::from_millis(59_999)),
            SilenceDecision::KeepRecording
        );
        assert_eq!(
            silence_decision(true, Duration::from_millis(60_000)),
            SilenceDecision::StopAfterPostSpeechSilence
        );
    }

    #[test]
    fn capture_ingress_rejects_non_finite_pcm_before_any_policy() {
        let healthy = vec![0.0, 0.25, -0.25];
        let healthy_history = validate_capture_pcm(&healthy, None).unwrap().history();
        assert!(validate_capture_pcm(&[0.0, f32::NAN], None).is_err());
        assert!(validate_capture_pcm(&[f32::INFINITY], None).is_err());
        let constant = vec![0.0; CAPTURE_LIVENESS_MIN_SAMPLES];
        assert!(validate_capture_pcm(&constant, None).is_err());
        let clipped = vec![1.0; CAPTURE_LIVENESS_MIN_SAMPLES];
        assert!(validate_capture_pcm(&clipped, None).is_err());
        let repeated = validate_capture_pcm(&healthy, Some(&healthy_history)).unwrap();
        assert!(repeated.discarded_repeat());
        let repeated_history = repeated.history();
        assert!(validate_capture_pcm(&healthy, Some(&repeated_history)).is_err());

        let changed = vec![0.0, 0.5, -0.5];
        let changed_history = validate_capture_pcm(&changed, Some(&repeated_history))
            .unwrap()
            .history();
        assert!(validate_capture_pcm(&healthy, Some(&changed_history)).is_ok());
    }

    #[test]
    fn opening_command_rolls_after_silero_without_waiting_for_pause() {
        assert_eq!(CMD_RETRY_MS, 50);
        assert_eq!(CMD_NEW_AUDIO_PROBE_SAMPLES, SAMPLE_RATE as usize / 20);
        let now = Instant::now();
        assert!(opening_command_due(
            true,
            false,
            false,
            SAMPLE_RATE as usize / 2,
            0,
            now - Duration::from_millis(CMD_RETRY_MS),
        ));
        assert!(!opening_command_due(
            false,
            false,
            false,
            SAMPLE_RATE as usize / 2,
            0,
            now - Duration::from_millis(CMD_RETRY_MS),
        ));
    }

    #[test]
    fn opening_command_fires_immediately_when_silero_closes_the_phrase() {
        let now = Instant::now();
        assert!(opening_command_due(
            true,
            false,
            true,
            SAMPLE_RATE as usize,
            SAMPLE_RATE as usize,
            now,
        ));
    }

    #[test]
    fn opening_pause_probe_blocks_duplicate_while_in_flight() {
        let mut checked = false;
        mark_pause_probe_submitted(&mut checked, true);
        assert!(checked);
    }

    #[test]
    fn opening_prefix_result_reopens_pause_probe_for_fresh_audio() {
        let mut checked = true;
        let mut pending = None;
        mark_opening_prefix_result(&mut checked, &mut pending);
        assert!(!checked);
        assert!(pending.is_some());
    }

    #[test]
    fn opening_command_requires_fresh_audio_and_stays_bounded() {
        let now = Instant::now();
        assert!(!opening_command_due(
            true,
            false,
            false,
            CMD_MIN_PROBE_SAMPLES,
            CMD_MIN_PROBE_SAMPLES,
            now - Duration::from_millis(CMD_RETRY_MS),
        ));
        assert!(!opening_command_due(
            true,
            false,
            false,
            CMD_WINDOW_SAMPLES + 1,
            0,
            now - Duration::from_millis(CMD_RETRY_MS),
        ));
        assert!(opening_command_due(
            true,
            true,
            false,
            CMD_WINDOW_SAMPLES + CMD_NEW_AUDIO_PROBE_SAMPLES,
            CMD_WINDOW_SAMPLES,
            now - Duration::from_millis(CMD_RETRY_MS),
        ));
    }

    #[test]
    fn opening_prefix_retry_requires_fresh_post_pause_audio() {
        let now = Instant::now();
        assert!(!opening_command_due(
            true,
            true,
            true,
            SAMPLE_RATE as usize,
            SAMPLE_RATE as usize,
            now,
        ));
        assert!(opening_command_due(
            true,
            true,
            true,
            SAMPLE_RATE as usize + CMD_NEW_AUDIO_PROBE_SAMPLES,
            SAMPLE_RATE as usize,
            now,
        ));
    }

    #[test]
    fn ambiguous_opening_needs_same_action_on_later_pause_ready_probe() {
        let mut pending = None;

        // Field sequence: early ASR probes changed identity before one
        // pause-ready ambiguous result. No single result may stop recording.
        assert!(!opening_action_confirmation(
            &mut pending,
            "yeah",
            1_000,
            true,
            true,
        ));
        assert!(!opening_action_confirmation(
            &mut pending,
            "same",
            2_000,
            true,
            true,
        ));
        assert!(!opening_action_confirmation(
            &mut pending,
            "save",
            3_000,
            true,
            true,
        ));

        // Matching identity on a later probe can confirm only once the phrase
        // is pause-ready; equal sample counts are not a second observation.
        assert!(!opening_action_confirmation(
            &mut pending,
            "save",
            3_000,
            true,
            true,
        ));
        assert!(!opening_action_confirmation(
            &mut pending,
            "save",
            4_000,
            false,
            true,
        ));
        assert!(opening_action_confirmation(
            &mut pending,
            "save",
            5_000,
            true,
            true,
        ));
    }

    #[test]
    fn complete_opening_needs_same_action_on_later_probe() {
        let mut pending = None;

        // Acoustic hypotheses can still change even when each result is a
        // grammar-complete command. One result must not stop recording.
        assert!(!opening_action_confirmation(
            &mut pending,
            "key:ctrl+s",
            1_000,
            true,
            false,
        ));
        assert!(opening_action_confirmation(
            &mut pending,
            "key:ctrl+s",
            2_000,
            true,
            false,
        ));
    }

    #[test]
    fn blank_audible_probe_waits_for_more_audio_without_final_error() {
        let audio = vec![0.25; SAMPLE_RATE as usize];
        let mut calls = 0;
        let result = transcribe_conditioned(&audio, BlankPolicy::Probe, |_| {
            calls += 1;
            Ok(String::new())
        });
        assert_eq!(result, Ok(String::new()));
        assert_eq!(calls, 1);
    }

    #[test]
    fn command_probe_keeps_pre_speech_prefix() {
        let speech_start = SAMPLE_RATE as usize;
        let start = speech_start.saturating_sub(CMD_PREROLL_SAMPLES);
        assert_eq!(command_probe_start_sample(Some(start), 0), start);
        // With no speech start, fall back to the LAST CMD_WINDOW_SAMPLES of
        // audio so the standalone probe sees real speech (zephyr tail already
        // does this — without it, the pre-decode voice gate fails on the
        // leading silence in the full buffer; field bug 2026-07-06).
        assert_eq!(command_probe_start_sample(None, CMD_WINDOW_SAMPLES), 0);
        assert_eq!(
            command_probe_start_sample(None, CMD_WINDOW_SAMPLES + 4000),
            4000
        );
    }

    #[test]
    fn zephyr_tail_hit_is_decided_from_short_tail_text() {
        use heardright_core::text_pipeline::{has_trailing_control_wake, parse_control_command};
        assert!(parse_control_command("please send this zephyr send").is_some());
        assert!(parse_control_command("zephyr stop").is_some());
        assert!(parse_control_command("please stop by later").is_none());
        assert!(has_trailing_control_wake("please send this zefyr"));
        assert!(!has_trailing_control_wake("the zephyr wind"));
    }

    #[test]
    fn capture_liveness_flags_a_dead_stream_but_never_a_quiet_room() {
        // Dead CoreAudio stream: bit-exact zeros, which is what the 2026-07-23
        // resumed warm stream delivered for 13.3 s.
        let dead = vec![0.0f32; SAMPLE_RATE as usize / 12];
        assert!(capture_stream_looks_dead(&dead));

        // Quiet room on a good mic: a real noise floor around 1e-5. This is the
        // case an RMS threshold would punish and exact-zero matching does not.
        let quiet: Vec<f32> = (0..SAMPLE_RATE as usize / 12)
            .map(|i| if i % 2 == 0 { 1.2e-5 } else { -0.9e-5 })
            .collect();
        assert!(!capture_stream_looks_dead(&quiet));
        assert!(pcm_zero_ratio(&quiet) < CAPTURE_LIVENESS_ZERO_RATIO);

        // Mostly-silent audio with a genuine signal is alive.
        let mut sparse = vec![0.0f32; SAMPLE_RATE as usize / 12];
        for (i, sample) in sparse.iter_mut().enumerate() {
            if i % 20 == 0 {
                *sample = 0.05;
            }
        }
        assert!(!capture_stream_looks_dead(&sparse));

        // A short or partial read is never called dead.
        assert!(!capture_stream_looks_dead(&[0.0; 4]));
        assert!(!capture_stream_looks_dead(&[]));
        assert_eq!(pcm_zero_ratio(&[]), 0.0);
    }

    #[test]
    fn main_asr_cannot_arm_or_parse_kws_controls() {
        assert!(!main_asr_control_fallback_due(false, true, true, false));
        assert!(!main_asr_control_fallback_due(false, false, true, true));
        assert!(!main_asr_control_fallback_due(false, true, false, true));
        assert!(!main_asr_control_fallback_due(true, true, true, true));
        assert!(!main_asr_control_fallback_due(false, true, true, true));
        assert!(confirmed_main_asr_control("Zephyr Sand", true).is_none());
        assert!(confirmed_main_asr_control("Zephyr Stop", true).is_none());
        assert!(confirmed_main_asr_control("Zephyr Cancel", true).is_none());
        assert!(confirmed_main_asr_control("Zephyr Sand", false).is_none());
        assert!(confirmed_main_asr_control("review them", true).is_none());
    }

    #[test]
    fn tail_probe_policy_uses_full_context_on_every_platform() {
        let total = SAMPLE_RATE as usize * 10;
        assert_eq!(
            tail_probe_start_sample(total, TailProbeLane::Full),
            total - TAIL_WINDOW_SAMPLES
        );

        assert_eq!(
            tail_probe_start_sample(total, TailProbeLane::Fast),
            total - FAST_TAIL_WINDOW_SAMPLES
        );
        assert_eq!(FAST_TAIL_WINDOW_SAMPLES, SAMPLE_RATE as usize * 3);
    }

    #[test]
    fn windows_full_tail_fallback_requires_a_control_verb_and_fresh_audio() {
        // The measured 1.75-second Windows lane can retain the intent verb but
        // crop or distort the wake word. That requests the reliable 3s lane,
        // but never hot-loops on the same samples.
        assert!(should_run_full_tail_fallback(
            "enemies ever send.",
            false,
            TAIL_PREFIX_NEW_AUDIO_SAMPLES,
        ));
        assert!(should_run_full_tail_fallback(
            "and sephyr stop.",
            false,
            TAIL_PREFIX_NEW_AUDIO_SAMPLES,
        ));
        assert!(!should_run_full_tail_fallback(
            "enemies ever send.",
            false,
            TAIL_PREFIX_NEW_AUDIO_SAMPLES - 1,
        ));
        assert!(!should_run_full_tail_fallback(
            "enemies ever submit.",
            false,
            TAIL_PREFIX_NEW_AUDIO_SAMPLES,
        ));
        // Council revision: silence alone must never launch a blocking full
        // decode; the fallback requires transcript evidence of a control verb.
        assert!(!should_run_full_tail_fallback(
            "ordinary dictation",
            false,
            usize::MAX,
        ));
    }

    #[test]
    fn windows_full_tail_fallback_retries_an_armed_candidate() {
        assert!(should_run_full_tail_fallback(
            "ordinary dictation",
            true,
            TAIL_PREFIX_NEW_AUDIO_SAMPLES,
        ));
        assert!(!should_run_full_tail_fallback(
            "ordinary dictation",
            false,
            TAIL_PREFIX_NEW_AUDIO_SAMPLES,
        ));
    }

    #[test]
    fn candidate_stays_fast_only_while_sherpa_still_sees_wake_or_verb() {
        assert!(should_keep_control_candidate_armed(false, true));
        assert!(!should_keep_control_candidate_armed(false, false));
        assert!(!should_keep_control_candidate_armed(true, true));
    }

    #[test]
    fn missed_vad_does_not_close_command_probing() {
        assert!(!pause_can_close_command_probe(false, true));
        assert!(!pause_can_close_command_probe(true, false));
        assert!(pause_can_close_command_probe(true, true));
    }

    #[test]
    fn low_energy_first_speech_is_still_sent_to_silero() {
        let chunk = vec![0.0001; 320];
        let mut last_voice_at = Instant::now();
        let mut heard_voice = false;
        let mut checked_this_pause = true;
        let mut speech_start_sample = None;
        let mut observed = false;

        observe_voice_chunk(
            &chunk,
            &mut last_voice_at,
            &mut heard_voice,
            &mut checked_this_pause,
            |samples| {
                observed = true;
                assert_eq!(samples, chunk);
                true
            },
            &mut speech_start_sample,
            8_000,
        );

        assert!(observed, "Silero must own first-speech detection");
        assert!(heard_voice);
        assert!(!checked_this_pause);
        assert_eq!(speech_start_sample, Some(8_000 - CMD_PREROLL_SAMPLES));
    }

    #[test]
    fn silero_silence_after_speech_does_not_refresh_voice_clock() {
        let chunk = vec![0.0; 320];
        let original_voice_at = Instant::now() - Duration::from_secs(2);
        let mut last_voice_at = original_voice_at;
        let mut heard_voice = true;
        let mut checked_this_pause = true;
        let mut speech_start_sample = Some(4_000);
        let mut observed = false;

        observe_voice_chunk(
            &chunk,
            &mut last_voice_at,
            &mut heard_voice,
            &mut checked_this_pause,
            |_| {
                observed = true;
                false
            },
            &mut speech_start_sample,
            12_000,
        );

        assert!(
            observed,
            "Silero must continue observing after first speech"
        );
        assert!(heard_voice, "silence must not erase prior speech evidence");
        assert_eq!(last_voice_at, original_voice_at);
        assert!(checked_this_pause);
        assert_eq!(speech_start_sample, Some(4_000));
    }

    #[test]
    fn speech_after_command_pause_does_not_move_kws_context() {
        let chunk = vec![0.1; 320];
        let mut last_voice_at = Instant::now() - Duration::from_millis(PAUSE_GATE_MS + 1);
        let mut heard_voice = true;
        let mut checked_this_pause = true;
        let mut speech_start_sample = Some(1_000);

        observe_voice_chunk(
            &chunk,
            &mut last_voice_at,
            &mut heard_voice,
            &mut checked_this_pause,
            |_| true,
            &mut speech_start_sample,
            40_000,
        );

        assert_eq!(speech_start_sample, Some(1_000));
    }

    #[test]
    fn speech_below_command_pause_keeps_current_sherpa_stream() {
        let chunk = vec![0.1; 320];
        let mut last_voice_at =
            Instant::now() - Duration::from_millis(PAUSE_GATE_MS.saturating_sub(1));
        let mut heard_voice = true;
        let mut checked_this_pause = true;
        let mut speech_start_sample = Some(1_000);

        observe_voice_chunk(
            &chunk,
            &mut last_voice_at,
            &mut heard_voice,
            &mut checked_this_pause,
            |_| true,
            &mut speech_start_sample,
            40_000,
        );

        assert_eq!(speech_start_sample, Some(1_000));
    }

    #[test]
    fn redacted_command_probe_keeps_counts_without_recognized_words() {
        let payload = command_probe_payload(CommandProbeLog {
            lane: "zephyr_tail_async",
            session_id: Some("session-1"),
            probe_ms: 2,
            recognized_text: "Zephyr send",
            classifier_result: "control_complete",
            speech_start_sample: Some(0),
            command_start: 0,
            command_samples: 16_000,
            total_samples: 16_000,
            pending_prefix: false,
            error: None,
        });
        let redacted = heardright_core::redact_diagnostic_event(payload);
        assert_eq!(redacted["recognized_chars"], 11);
        assert_eq!(redacted["recognized_words"], 2);
        assert_eq!(redacted["recognized_empty"], false);
        assert_eq!(redacted["recognized_text"], "[redacted:diagnostics]");
        assert!(!redacted.to_string().contains("Zephyr"));
    }

    #[test]
    fn capture_reopen_policy_reuses_an_unchanged_saved_device() {
        assert!(!capture_should_reopen(false, false));
        assert!(capture_should_reopen(true, false));
        assert!(capture_should_reopen(false, true));
    }

    #[test]
    fn first_buffer_health_reopens_every_non_live_start_state() {
        use heardright_capture::{CaptureErrorKind, CaptureFirstBuffer};

        assert!(capture_first_buffer_requires_reopen(
            &CaptureFirstBuffer::NoCallbacks
        ));
        assert!(capture_first_buffer_requires_reopen(
            &CaptureFirstBuffer::NoSamples
        ));
        assert!(capture_first_buffer_requires_reopen(
            &CaptureFirstBuffer::StreamError(CaptureErrorKind::StreamInvalidated)
        ));
        assert!(capture_first_buffer_requires_reopen(
            &CaptureFirstBuffer::Data(vec![0.0; CAPTURE_LIVENESS_MIN_SAMPLES])
        ));
        assert!(capture_first_buffer_requires_reopen(
            &CaptureFirstBuffer::Data(vec![0.00001; CAPTURE_LIVENESS_MIN_SAMPLES - 1])
        ));
        assert!(!capture_first_buffer_requires_reopen(
            &CaptureFirstBuffer::Data(vec![0.00001; CAPTURE_LIVENESS_MIN_SAMPLES])
        ));
    }

    #[test]
    fn capture_route_signature_detects_native_format_changes() {
        let devices = [DeviceInfo {
            id: 4,
            name: "MacBook Microphone".to_string(),
            native_rate: 48_000,
            channels: 1,
            sample_format: "F32".to_string(),
            is_default: true,
            transport: CaptureTransport::BuiltIn,
            form_factor: heardright_capture::CaptureFormFactor::Microphone,
            platform_id: Some("built-in".into()),
        }];
        let (_, baseline) = resolve_capture_route(None, &devices);
        let mut changed = devices.clone();
        changed[0].native_rate = 24_000;
        let (_, call_route) = resolve_capture_route(None, &changed);
        assert_ne!(baseline, call_route);
    }

    #[test]
    fn active_capture_route_change_requires_a_stream_rebuild() {
        let before = CaptureRouteSignature {
            device_id: 0,
            name: "Laptop Microphone".into(),
            native_rate: 48_000,
            channels: 2,
            sample_format: "F32".into(),
            transport: CaptureTransport::BuiltIn,
            form_factor: heardright_capture::CaptureFormFactor::Microphone,
        };
        let during_call = CaptureRouteSignature {
            device_id: 1,
            name: "Headset Microphone (Hands-Free)".into(),
            native_rate: 16_000,
            channels: 1,
            sample_format: "F32".into(),
            transport: CaptureTransport::Bluetooth,
            form_factor: heardright_capture::CaptureFormFactor::Headset,
        };
        assert!(capture_route_changed(&Some(before), &Some(during_call)));
    }

    #[test]
    fn bluetooth_capture_gets_route_specific_settle_budget() {
        let bluetooth = CaptureRouteSignature {
            device_id: 1,
            name: "Headset".into(),
            native_rate: 16_000,
            channels: 1,
            sample_format: "F32".into(),
            transport: CaptureTransport::Bluetooth,
            form_factor: heardright_capture::CaptureFormFactor::Headset,
        };
        assert_eq!(capture_settle_budget_ms(Some(&bluetooth), false), 1_500);
        assert_eq!(capture_settle_budget_ms(Some(&bluetooth), true), 1_500);
        assert_eq!(capture_settle_budget_ms(None, false), 250);
        assert_eq!(capture_settle_budget_ms(None, true), 500);
    }

    #[test]
    fn dummy_worker_reload_replies_ready() {
        let worker = spawn_dummy_worker(Arc::new(Mutex::new(FocusTracker::new()))).unwrap();
        assert!(!worker.reload_model().unwrap());
        let _ = worker.send(WorkerCmd::Shutdown);
    }

    #[test]
    fn duration_cap_trips_at_exactly_the_configured_ceiling() {
        assert!(!recording_duration_cap_exceeded(MAX_RECORDING_SAMPLES - 1));
        assert!(recording_duration_cap_exceeded(MAX_RECORDING_SAMPLES));
        assert!(recording_duration_cap_exceeded(MAX_RECORDING_SAMPLES + 1));
        // Sanity: cap is 30 minutes at 16kHz, not some other unit slip.
        assert_eq!(MAX_RECORDING_SAMPLES, 30 * 60 * SAMPLE_RATE as usize);
    }

    #[test]
    fn probe_gate_stashes_stop_without_dropping_it() {
        let (tx, rx) = channel::<WorkerCmd>();
        let mut stash: Option<WorkerCmd> = None;

        // Nothing queued yet — gate stays clear, nothing stashed.
        assert!(probe_gate_clear(&rx, &mut stash));
        assert!(stash.is_none());

        // Stop queued — gate closes, and the command is captured (not lost).
        tx.send(WorkerCmd::StopRecording { send_enter: true })
            .unwrap();
        assert!(!probe_gate_clear(&rx, &mut stash));
        assert!(stop_or_cancel_stashed(&stash));

        // A second peek must not pull another item off the channel (only one
        // command is ever in flight through the stash at a time).
        tx.send(WorkerCmd::Cancel).unwrap();
        assert!(!probe_gate_clear(&rx, &mut stash));
        assert!(matches!(
            stash,
            Some(WorkerCmd::StopRecording { send_enter: true })
        ));
        // The Cancel is still sitting in the channel, unconsumed.
        assert!(matches!(rx.try_recv(), Ok(WorkerCmd::Cancel)));
    }

    #[test]
    fn probe_gate_does_not_block_on_non_stop_commands() {
        let (tx, rx) = channel::<WorkerCmd>();
        let mut stash: Option<WorkerCmd> = None;
        tx.send(WorkerCmd::ReloadModel { reply: None }).unwrap();
        // A queued-but-unrelated command is stashed (not dropped) but does not
        // close the probe gate.
        assert!(probe_gate_clear(&rx, &mut stash));
        assert!(matches!(stash, Some(WorkerCmd::ReloadModel { .. })));
    }
}

#[cfg(all(test, any(target_os = "macos", target_os = "windows")))]
mod trigger_virtual_time_gate2 {
    use super::replay_effect_sink::{ReplayEffect, ReplayEffectSink};
    use super::*;
    use serde_json::{json, Value};
    use std::fs::File;
    use std::io::{BufWriter, Write};
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    #[derive(Clone, Debug)]
    struct ReplayRow {
        id: String,
        audio_path: PathBuf,
        expected: Option<&'static str>,
        corpus: &'static str,
    }

    fn intent(value: &str) -> &'static str {
        match value.trim().to_ascii_uppercase().as_str() {
            "STOP" | "ZEPHYR_STOP" => "STOP",
            "SEND" | "ZEPHYR_SEND" => "SEND",
            "CANCEL" | "ZEPHYR_CANCEL" => "CANCEL",
            other => panic!("unsupported trigger intent {other}"),
        }
    }

    #[cfg(target_os = "macos")]
    const DEFAULT_CORPUS_ROOT: &str = "/Volumes/D/kwstest";
    #[cfg(target_os = "windows")]
    const DEFAULT_CORPUS_ROOT: &str = r"\\192.168.1.7\d\kwstest";

    #[cfg(target_os = "macos")]
    fn localize_network_path(value: &str) -> PathBuf {
        let normalized = value.replace('\\', "/");
        let suffix = normalized
            .strip_prefix("//192.168.1.7/d/kwstest/")
            .unwrap_or_else(|| panic!("unexpected corpus path {value}"));
        PathBuf::from(DEFAULT_CORPUS_ROOT).join(suffix)
    }

    #[cfg(target_os = "windows")]
    fn localize_network_path(value: &str) -> PathBuf {
        let normalized = value.replace('/', "\\");
        let prefix = format!("{}\\", DEFAULT_CORPUS_ROOT);
        assert!(
            normalized.starts_with(&prefix),
            "unexpected corpus path {value}"
        );
        PathBuf::from(normalized)
    }

    #[cfg(target_os = "macos")]
    fn owner_audio_path(clip: &Value) -> PathBuf {
        clip["mac_path"]
            .as_str()
            .map(PathBuf::from)
            .unwrap_or_else(|| localize_network_path(clip["path"].as_str().expect("owner path")))
    }

    #[cfg(target_os = "windows")]
    fn owner_audio_path(clip: &Value) -> PathBuf {
        localize_network_path(clip["path"].as_str().expect("owner path"))
    }

    fn default_output_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(".audit/2026-08-04-event-driven-trigger-runtime")
    }

    fn load_rows(root: &Path) -> Vec<ReplayRow> {
        let mut rows = Vec::new();
        let positive: Value = serde_json::from_slice(
            &std::fs::read(root.join("canonical/positive14/qualification-raw.json"))
                .expect("read canonical positive truth"),
        )
        .expect("parse canonical positive truth");
        for clip in positive["results"][0]["clips"]
            .as_array()
            .expect("canonical positive clips")
        {
            let id = clip["audio"]["id"].as_str().expect("positive id");
            rows.push(ReplayRow {
                id: id.to_string(),
                audio_path: root
                    .join("canonical/positive14/audio")
                    .join(format!("{id}.wav")),
                expected: Some(intent(
                    clip["parser_result"]["intent"]
                        .as_str()
                        .expect("positive intent"),
                )),
                corpus: "canonical_positive14",
            });
        }

        let negative: Vec<Value> = serde_json::from_slice(
            &std::fs::read(root.join("canonical/negative238/candidate_manifest.json"))
                .expect("read canonical negative truth"),
        )
        .expect("parse canonical negative truth");
        for clip in negative {
            rows.push(ReplayRow {
                id: clip["id"].as_str().expect("negative id").to_string(),
                audio_path: localize_network_path(clip["path"].as_str().expect("negative path")),
                expected: None,
                corpus: "canonical_negative238",
            });
        }

        for (relative, corpus) in [
            ("owner/windows/owner-kws-manifest.json", "owner_windows"),
            ("owner/mac/owner-kws-manifest.json", "owner_mac"),
        ] {
            let manifest: Value = serde_json::from_slice(
                &std::fs::read(root.join(relative)).expect("read owner truth"),
            )
            .expect("parse owner truth");
            for clip in manifest["rows"].as_array().expect("owner rows") {
                let audio_path = owner_audio_path(clip);
                rows.push(ReplayRow {
                    id: clip["id"].as_str().expect("owner id").to_string(),
                    audio_path,
                    expected: clip["expected_detection"].as_str().map(intent),
                    corpus,
                });
            }
        }
        rows
    }

    fn read_wav(path: &Path) -> Vec<f32> {
        let mut reader = hound::WavReader::open(path)
            .unwrap_or_else(|error| panic!("open {}: {error}", path.display()));
        let spec = reader.spec();
        assert_eq!(
            spec.sample_rate,
            SAMPLE_RATE,
            "{} sample rate",
            path.display()
        );
        assert_eq!(spec.channels, 1, "{} channel count", path.display());
        match (spec.sample_format, spec.bits_per_sample) {
            (hound::SampleFormat::Int, bits) if bits <= 16 => reader
                .samples::<i16>()
                .map(|sample| sample.expect("i16 WAV sample") as f32 / 32_768.0)
                .collect(),
            (hound::SampleFormat::Int, bits) => {
                let scale = (1_u64 << (bits - 1)) as f32;
                reader
                    .samples::<i32>()
                    .map(|sample| sample.expect("i32 WAV sample") as f32 / scale)
                    .collect()
            }
            (hound::SampleFormat::Float, 32) => reader
                .samples::<f32>()
                .map(|sample| sample.expect("f32 WAV sample"))
                .collect(),
            other => panic!("unsupported WAV format {other:?}: {}", path.display()),
        }
    }

    fn scripted_control_cmd(event: CaptureEvent) -> Result<Option<WorkerCmd>, String> {
        match event {
            CaptureEvent::Control { name, .. } => match name.trim().to_ascii_lowercase().as_str() {
                "stop" => Ok(Some(WorkerCmd::StopRecording { send_enter: false })),
                "send" => Ok(Some(WorkerCmd::StopRecording { send_enter: true })),
                "cancel" => Ok(Some(WorkerCmd::Cancel)),
                other => Err(format!("unsupported scripted replay control {other}")),
            },
            CaptureEvent::CaptureError { message, .. } => {
                Err(format!("scripted replay capture error: {message}"))
            }
            CaptureEvent::Disconnect { .. } => Err("scripted replay disconnect".into()),
            CaptureEvent::Audio(_) | CaptureEvent::Eof { .. } => Ok(None),
        }
    }

    fn dispatch_scripted_controls(controller: &ReplayCaptureController, handle: &WorkerHandle) {
        while let Some(event) = controller.pop_scripted_control() {
            if let Some(cmd) = scripted_control_cmd(event).expect("valid scripted replay control") {
                handle.send(cmd).expect("dispatch scripted replay control");
            }
        }
    }

    fn drain_action_events(
        handle: &WorkerHandle,
        session_id: &str,
        effects: &mut ReplayEffectSink,
    ) -> Vec<&'static str> {
        let mut actions = Vec::new();
        let events = handle.event_rx.lock();
        loop {
            match events.try_recv() {
                Ok(WorkerEvent::AutoStopBegin {
                    session_id: event_session,
                    send_enter,
                }) if event_session == session_id => {
                    let (action, effect) = if send_enter {
                        ("SEND", ReplayEffect::Send)
                    } else {
                        ("STOP", ReplayEffect::Stop)
                    };
                    effects
                        .record_terminal(session_id, effect)
                        .unwrap_or_else(|duplicate| {
                            panic!("duplicate replay effect: {duplicate:?}")
                        });
                    actions.push(action);
                }
                Ok(WorkerEvent::RunawayDiscard {
                    session_id: event_session,
                }) if event_session == session_id => {
                    effects
                        .record_terminal(session_id, ReplayEffect::Cancel)
                        .unwrap_or_else(|duplicate| {
                            panic!("duplicate replay effect: {duplicate:?}")
                        });
                    actions.push("CANCEL");
                }
                Ok(WorkerEvent::CaptureFailed {
                    session_id: event_session,
                    message,
                }) if event_session == session_id => {
                    panic!("{session_id}: capture failed: {message}")
                }
                Ok(WorkerEvent::TranscriptFinal {
                    session_id: event_session,
                    result: Err(message),
                    ..
                }) if event_session == session_id => {
                    panic!("{session_id}: recording failed: {message}")
                }
                Ok(WorkerEvent::StartFailed { message }) => {
                    panic!("worker failed during replay: {message}")
                }
                Ok(_) => {}
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    panic!("worker event channel disconnected")
                }
            }
        }
        actions
    }

    #[test]
    fn scripted_controls_map_to_real_worker_commands() {
        for (name, expected) in [("stop", "stop"), ("send", "send"), ("cancel", "cancel")] {
            let cmd = scripted_control_cmd(CaptureEvent::Control {
                at_sample: 320,
                name: name.into(),
            })
            .unwrap()
            .unwrap();
            assert!(match (expected, cmd) {
                ("stop", WorkerCmd::StopRecording { send_enter: false })
                | ("send", WorkerCmd::StopRecording { send_enter: true })
                | ("cancel", WorkerCmd::Cancel) => true,
                _ => false,
            });
        }
    }

    #[test]
    fn gate2_defaults_use_host_corpus_without_copying() {
        #[cfg(target_os = "macos")]
        assert_eq!(
            localize_network_path(r"\\192.168.1.7\d\kwstest\owner\mac\sample.wav"),
            PathBuf::from("/Volumes/D/kwstest/owner/mac/sample.wav")
        );
        #[cfg(target_os = "windows")]
        assert_eq!(
            localize_network_path(r"\\192.168.1.7\d\kwstest\owner\windows\sample.wav"),
            PathBuf::from(r"\\192.168.1.7\d\kwstest\owner\windows\sample.wav")
        );
    }

    #[test]
    #[ignore = "explicit 373-row production-worker virtual-time trigger corpus"]
    fn production_worker_gate2_replays_full_trigger_corpus() {
        assert_eq!(
            std::env::var("HR_KWS_GATE2").ok().as_deref(),
            Some("1"),
            "set HR_KWS_GATE2=1 for explicit Gate 2 run"
        );
        let corpus_root = PathBuf::from(
            std::env::var("HR_KWS_CORPUS_ROOT").unwrap_or_else(|_| DEFAULT_CORPUS_ROOT.to_string()),
        );
        let output_root = std::env::var("HR_KWS_RUNTIME_OUTPUT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_output_root());
        std::fs::create_dir_all(&output_root).expect("create runtime output root");
        let positives_only = std::env::var("HR_KWS_RUNTIME_POSITIVES_ONLY").as_deref() == Ok("1");
        let row_filter = std::env::var("HR_KWS_RUNTIME_ROW_ID").ok();
        let mut rows = load_rows(&corpus_root);
        if positives_only {
            rows.retain(|row| row.expected.is_some());
        }
        if let Some(row_id) = row_filter.as_deref() {
            rows.retain(|row| row.id == row_id);
        }
        let positives = rows.iter().filter(|row| row.expected.is_some()).count();
        if row_filter.is_none() {
            assert_eq!(
                (rows.len(), positives),
                if positives_only { (56, 56) } else { (373, 56) }
            );
        } else {
            assert!(!rows.is_empty());
        }
        assert!(rows.iter().all(|row| row.audio_path.is_file()));

        std::env::set_var(
            "HR_SHERPA_KWS_DIR",
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/resources/kws"),
        );
        std::env::set_var("HR_APP_DATA_DIR", output_root.join("app-data"));
        std::env::set_var("HR_SCHEDULED_STATIC15", "0");
        std::env::remove_var("HR_DISABLE_KWS");
        #[cfg(target_os = "windows")]
        let _ort = unsafe {
            libloading::Library::new(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../src-tauri/resources/runtime/onnxruntime.dll"),
            )
        }
        .expect("preload bundled ONNX Runtime");

        let controller = install_trigger_replay();
        let worker = spawn_worker_with_clock(
            output_root.join("unused-main-model"),
            AsrEp::Cpu,
            Arc::new(Mutex::new(FocusTracker::new())),
            controller.virtual_clock(),
        )
        .expect("spawn production worker for trigger replay");
        let rows_path = output_root.join("rows.jsonl");
        let mut output = BufWriter::new(File::create(&rows_path).expect("create rows output"));
        let started = Instant::now();
        let mut passed = 0usize;
        let mut misses = Vec::new();
        let mut false_actions = Vec::new();
        let mut wrong_actions = Vec::new();
        let mut max_probe_ms = 0u64;
        let mut effects = ReplayEffectSink::default();

        for (index, row) in rows.iter().enumerate() {
            let audio = read_wav(&row.audio_path);
            let row_timeout =
                Duration::from_secs_f64(audio.len() as f64 / SAMPLE_RATE as f64 + 60.0);
            controller.load(audio);
            let session_id = format!("virtual-time-gate2-{index:03}-{}", row.id);
            worker
                .send(WorkerCmd::StartRecording {
                    session_id: session_id.clone(),
                })
                .unwrap_or_else(|error| panic!("{} start: {error}", row.id));

            let row_started = Instant::now();
            let mut actions = Vec::new();
            let row_max_probe_ms = loop {
                dispatch_scripted_controls(&controller, &worker);
                actions.extend(drain_action_events(&worker, &session_id, &mut effects));
                if !actions.is_empty() {
                    break controller.finished().unwrap_or_default();
                }
                if let Some(max_probe_ms) = controller.finished() {
                    worker
                        .send(WorkerCmd::Cancel)
                        .expect("cancel negative replay");
                    worker.reload_model().expect("negative replay barrier");
                    actions.extend(drain_action_events(&worker, &session_id, &mut effects));
                    break max_probe_ms;
                }
                assert!(
                    row_started.elapsed() < row_timeout,
                    "{} replay timed out: {}",
                    row.id,
                    controller.debug_state()
                );
                std::thread::sleep(Duration::from_millis(1));
            };

            if !actions.is_empty() {
                worker.reload_model().expect("positive replay barrier");
                actions.extend(drain_action_events(&worker, &session_id, &mut effects));
            }
            max_probe_ms = max_probe_ms.max(row_max_probe_ms);
            let actual = match actions.as_slice() {
                [] => None,
                [single] => Some(*single),
                _ => panic!("{} emitted multiple actions: {actions:?}", row.id),
            };
            let expected_effects = match actual {
                Some("STOP") => vec![ReplayEffect::Stop],
                Some("SEND") => vec![ReplayEffect::Send],
                Some("CANCEL") => vec![ReplayEffect::Cancel],
                None => Vec::new(),
                Some(other) => unreachable!("unknown replay action {other}"),
            };
            effects
                .assert_exactly_once(&session_id, &expected_effects)
                .unwrap_or_else(|error| panic!("{}: {error}", row.id));
            let pass = actual == row.expected;
            if pass {
                passed += 1;
            } else {
                match (row.expected, actual) {
                    (Some(_), None) => misses.push(row.id.clone()),
                    (None, Some(_)) => false_actions.push(row.id.clone()),
                    (Some(_), Some(_)) => wrong_actions.push(row.id.clone()),
                    (None, None) => unreachable!(),
                }
            }
            serde_json::to_writer(
                &mut output,
                &json!({
                    "row_id": row.id,
                    "corpus": row.corpus,
                    "audio_path": row.audio_path,
                    "expected": row.expected,
                    "actual": actual,
                    "pass": pass,
                    "max_probe_ms": row_max_probe_ms,
                }),
            )
            .expect("write row result");
            output.write_all(b"\n").expect("finish row result");
            output.flush().expect("checkpoint row result");
        }

        worker
            .send(WorkerCmd::Shutdown)
            .expect("shutdown replay worker");
        TRIGGER_REPLAY_ACTIVE.store(false, std::sync::atomic::Ordering::Release);
        *REPLAY_CAPTURE
            .get_or_init(|| std::sync::Mutex::new(None))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;

        let summary = json!({
            "schema": "heardright.kws_runtime_gate2.trigger_only.result.v1",
            "evidence_class": "gate2_acceptance",
            "rows": rows.len(),
            "positives": positives,
            "negatives": rows.len() - positives,
            "passed": passed,
            "misses": misses,
            "false_actions": false_actions,
            "wrong_actions": wrong_actions,
            "max_probe_ms": max_probe_ms,
            "wall_ms": started.elapsed().as_millis() as u64,
        });
        std::fs::write(
            output_root.join("summary.json"),
            serde_json::to_vec_pretty(&summary).expect("serialize summary"),
        )
        .expect("write summary");
        assert_eq!(passed, rows.len(), "{summary}");
    }
}
