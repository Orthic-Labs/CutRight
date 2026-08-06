use std::fs;
use std::path::PathBuf;

fn source(path: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(root.join(path)).unwrap_or_else(|error| panic!("read {path}: {error}"))
}

#[test]
fn engine_has_independent_main_asr_and_kws_executors() {
    let worker = source("src/worker.rs");
    let lane = source("src/worker_sections/unified_probe_lane.rs");

    assert!(worker.contains("unified_probe_lane.rs"));
    assert!(lane.contains("struct AsrExecutor"));
    assert!(lane.contains("struct MainAsrMailbox"));
    assert!(lane.contains("struct ProbeMailbox"));
    assert!(lane.contains("latest_probe: Option<ProbeRequest>"));
    assert!(lane.contains(".name(\"hr-main-asr-worker\""));
    assert!(lane.contains(".name(\"hr-kws-worker\""));
    assert_eq!(lane.matches("AsrRuntime::load").count(), 2);
}

#[test]
fn capture_worker_owns_no_asr_runtime_outside_the_executor() {
    let startup = source("src/worker_sections/section02.rs");
    let executor = source("src/worker_sections/unified_probe_lane.rs");

    assert!(
        !startup.contains("AsrRuntime::load"),
        "capture worker must not load a second ASR runtime"
    );
    assert_eq!(executor.matches("AsrRuntime::load").count(), 2);
    assert!(executor.contains("let mut probe_model = match AsrRuntime::load_probe"));
    assert!(executor.contains(".name(\"hr-main-asr-worker\""));
    assert!(executor.contains(".name(\"hr-kws-worker\""));
}

#[test]
fn final_requests_have_mailbox_priority_over_scheduled_background_work() {
    let executor = source("src/worker_sections/unified_probe_lane.rs");

    let final_branch = executor
        .find("if let Some(final_request)")
        .expect("final-priority mailbox branch");
    let background_branch = executor
        .find("if let Some(background_request)")
        .expect("background mailbox branch");
    assert!(
        final_branch < background_branch,
        "final request must dequeue before speculative background work"
    );
    assert!(
        executor.contains("state.background_requests.clear()"),
        "submitting final work must discard queued speculative work"
    );
}

#[test]
fn capture_loop_never_runs_command_or_trigger_asr_inline() {
    let streaming = source("src/worker_sections/worker_streaming.rs");

    assert!(!streaming.contains("run_control_tail_probe(\n                    &mut model"));
    assert!(!streaming.contains("transcribe_probe_buffer(&mut model"));
    assert!(streaming.contains("asr_executor.submit_probe"));
}

#[test]
fn scheduled_static15_background_work_runs_off_capture_on_every_platform() {
    let streaming = source("src/worker_sections/worker_streaming.rs");
    let executor = source("src/worker_sections/unified_probe_lane.rs");

    assert!(!streaming.contains("process_ready"));
    assert!(streaming.contains("asr_executor.submit_background"));
    assert!(executor.contains("ScheduledStatic15::default()"));
    assert!(executor.contains("commit_background_window"));
    assert!(executor.contains("finish_recording_transcript"));
    assert!(executor.contains("deferred_background.take()"));
    assert!(!executor.contains("background_failed"));
}

#[test]
fn opening_commands_treat_blank_short_probes_as_recoverable() {
    let worker = source("src/worker_sections/section03.rs");
    let helper_start = worker
        .find("fn transcribe_opening_buffer")
        .expect("opening-command ASR helper");
    let helper = &worker[helper_start..];
    let helper_end = helper
        .find("fn warm_main_asr")
        .expect("opening-command ASR helper boundary");
    let helper = &helper[..helper_end];
    assert!(helper.contains("BlankPolicy::Probe"));
    assert!(helper.contains("apply_utterance_bias"));

    let executor = source("src/worker_sections/unified_probe_lane.rs");
    let branch_start = executor
        .find("MainAsrJob::Opening(request)")
        .expect("opening-command executor branch");
    let branch = &executor[branch_start..];
    let branch_end = branch
        .find("MainAsrJob::Background(request)")
        .expect("opening-command executor branch boundary");
    let branch = &branch[..branch_end];
    assert!(branch.contains("transcribe_opening_buffer"));
    assert!(!branch.contains("transcribe_buffer("));
}

#[test]
fn final_decode_takes_recording_buffer_ownership_without_a_full_clone() {
    for path in [
        "src/worker_sections/worker_commands.rs",
        "src/worker_sections/worker_streaming.rs",
    ] {
        let source = source(path);
        assert!(source.contains("let final_audio = std::mem::take(&mut buffer);"));
        assert!(source.contains("transcribe_final(final_generation, final_audio)"));
        assert!(!source.contains("transcribe_final(final_generation, buffer.clone())"));
    }
}

#[test]
fn worker_event_pump_startup_fails_closed() {
    let run = source("src/ipc_sections/section01.rs");
    let dispatch = source("src/ipc_sections/section02.rs");
    let pump = source("src/ipc_sections/section03.rs");

    assert!(run.contains("worker event pump failed before ready"));
    assert!(dispatch.contains("-> Result<(), String>"));
    assert!(dispatch.contains("return error_frame(request_id, &trace_id, None, &message)"));
    assert!(pump.contains("-> Result<(), String>"));
    assert!(pump.contains("spawn worker event pump:"));
    assert!(pump.contains(".map_err(|error| format!(\"spawn worker event pump: {error}\"))"));
}

#[test]
fn recording_start_revalidates_the_executor_model() {
    let commands = source("src/worker_sections/worker_commands.rs");
    let start = commands
        .find("Ok(WorkerCmd::StartRecording")
        .expect("start recording handler");
    let handler = &commands[start..];
    let end = handler
        .find("Ok(WorkerCmd::StopRecording")
        .expect("start recording handler boundary");
    let handler = &handler[..end];

    assert!(handler.contains("asr_executor"));
    assert!(handler.contains(".reload_model()"));
}

#[test]
fn manual_stop_preserves_send_intent_in_worker_telemetry() {
    let runtime = source("src/runtime_sections/section02.rs");
    assert!(
        runtime.contains("w.send(WorkerCmd::StopRecording { send_enter })"),
        "runtime must pass its authoritative Send intent into the worker"
    );

    let commands = source("src/worker_sections/worker_commands.rs");
    let start = commands
        .find("Ok(WorkerCmd::StopRecording { send_enter })")
        .expect("stop recording handler");
    let handler = &commands[start..];
    let end = handler
        .find("Ok(WorkerCmd::Cancel)")
        .expect("stop recording handler boundary");
    let handler = &handler[..end];
    assert!(
        handler.contains(
            "trace_recording_stop(\n                    Some(&session_id),\n                    \"manual_stop\",\n                    buffer.len(),\n                    heard_voice,\n                    speech_start_sample,\n                    None,\n                    send_enter,"
        ),
        "manual-stop telemetry must log the same Send intent"
    );
}

#[test]
fn autostop_enters_runtime_transcribing_before_final_asr_completes() {
    let ipc = source("src/ipc_sections/section03.rs");
    let start = ipc
        .find("WorkerEvent::AutoStopBegin")
        .expect("auto-stop begin handler");
    let handler = &ipc[start..];
    let end = handler
        .find("WorkerEvent::RunawayDiscard")
        .expect("auto-stop begin handler boundary");
    let handler = &handler[..end];

    let begin_stop = handler
        .find("runtime.lock().begin_stop(&session_id, send_enter, false)")
        .expect("auto-stop must start runtime timing before final ASR");
    let transcribing_event = handler
        .find("EngineSchemaName::TranscribingStarted")
        .expect("auto-stop must notify shell");
    assert!(begin_stop < transcribing_event);
}

#[test]
fn command_probe_jsonl_is_not_duplicated_through_sidecar_stderr() {
    let worker = source("src/worker_sections/section03.rs");
    let start = worker
        .find("fn emit_command_probe")
        .expect("command-probe sink");
    let sink = &worker[start..];
    let end = sink
        .find("// M6 (perf audit")
        .expect("command-probe sink boundary");
    let sink = &sink[..end];

    assert!(sink.contains("append_command_probe_event(&redacted)"));
    assert!(!sink.contains("tracing::"));
}

#[test]
fn macos_and_windows_use_the_same_dedicated_probe_runtime() {
    let lane = source("src/worker_sections/unified_probe_lane.rs");
    assert!(lane.contains("let mut probe_model = match AsrRuntime::load_probe"));
    assert!(lane.contains(".name(\"hr-kws-worker\""));
    assert!(!lane.contains(
        "#[cfg(target_os = \"windows\")]\n                            let probe_model = &mut model;"
    ));
}

#[test]
fn incremental_sherpa_kws_receives_raw_pcm() {
    let lane = source("src/worker_sections/unified_probe_lane.rs");
    let start = lane
        .find("fn transcribe_probe_buffer_timed")
        .expect("timed KWS helper");
    let helper = &lane[start..];
    let end = helper
        .find("#[derive(Debug, PartialEq)]")
        .expect("helper boundary");
    let helper = &helper[..end];

    let raw_branch = helper
        .find("if model.requires_timed_control_probe()")
        .expect("Sherpa raw branch");
    let conditioning = helper
        .find("condition_for_asr")
        .expect("non-Sherpa conditioning branch");
    assert!(
        raw_branch < conditioning,
        "Sherpa must receive raw continuous PCM before generic per-call conditioning"
    );
}

#[test]
fn wake_commands_require_native_sherpa_intent_not_main_asr_text() {
    let probe_results = source("src/worker_sections/worker_probe_results.rs");
    let authority_start = probe_results
        .find("let control = probe_result.native_control_intent")
        .expect("control authority branch");
    let authority = &probe_results[authority_start..];
    let authority_end = authority
        .find("let saw_control_candidate")
        .expect("control authority boundary");
    let authority = &authority[..authority_end];

    assert!(authority.contains("probe_result.native_control_intent"));
    assert!(
        !authority.contains("parse_control_command(trimmed)"),
        "main-ASR text must not authorize a wake command"
    );
    assert!(
        !authority.contains("parse_pending_control_tail("),
        "fuzzy main-ASR text must not authorize a wake command"
    );
}

#[test]
fn main_asr_cannot_arm_or_mutate_persistent_kws_confirmation() {
    let opening = source("src/worker_sections/worker_opening_results.rs");
    let helpers = source("src/worker_sections/section03.rs");
    let lane = source("src/worker_sections/unified_probe_lane.rs");

    let legacy_fallback = ["fallback_armed_sherpa_", "confirmation"].concat();
    assert!(!opening.contains(&legacy_fallback));
    assert!(!opening.contains("pending_control_prefix_since"));
    assert!(!opening.contains("confirmed_main_asr_control"));
    assert!(!opening.contains("classify_streaming"));
    assert!(!opening.contains("opening_action_confirmation"));
    assert!(opening.contains("main_asr_dictation_only"));

    let fallback_start = helpers
        .find("fn main_asr_control_fallback_due")
        .expect("fallback gate");
    let fallback = &helpers[fallback_start..][..helpers[fallback_start..]
        .find("fn confirmed_main_asr_control")
        .unwrap()];
    assert!(fallback.contains("false"));
    assert!(!fallback.contains("parse_control_command"));

    let cascade_call = lane
        .find("probe_model.transcribe_probe_cascade_result")
        .expect("persistent cascade call");
    let call = &lane[cascade_call..][..lane[cascade_call..].find("                    )").unwrap()];
    assert!(call.contains("request.pending_control_prefix"), "{call}");

    let cadence_start = helpers.find("fn tail_probe_due").expect("tail cadence");
    let cadence = &helpers[cadence_start..];
    let cadence_end = cadence
        .find("fn tail_probe_active")
        .expect("cadence boundary");
    assert!(cadence[..cadence_end].contains("TAIL_CONFIRM_CHECK_MS"));
    assert!(cadence[..cadence_end].contains("TAIL_CONFIRM_NEW_AUDIO_SAMPLES"));
    assert!(cadence[..cadence_end].contains("TAIL_WAKE_CHECK_MS"));

    let sherpa = source("src/sherpa_kws.rs");
    assert!(sherpa.contains("wake_decode_due"));
    assert!(sherpa.contains("WAKE_RESCAN_STEP_SAMPLES"));
}

#[test]
fn vad_has_no_kws_stream_base_state() {
    for path in [
        "src/worker_sections/section02.rs",
        "src/worker_sections/section03.rs",
        "src/worker_sections/worker_commands.rs",
        "src/worker_sections/worker_opening_results.rs",
        "src/worker_sections/worker_probe_results.rs",
        "src/worker_sections/worker_streaming.rs",
        "src/worker_sections/section05.rs",
    ] {
        assert!(
            !source(path).contains("kws_stream_base_sample"),
            "{path} must not retain VAD-derived KWS timing state"
        );
    }
}

#[test]
fn final_main_asr_cannot_parse_or_arm_wake_commands() {
    let runtime = source("src/runtime_sections/finalize_transcript.rs");
    let phase_start = runtime
        .find("fn finalize_phase1_capture")
        .expect("final transcription phase");
    let phase = &runtime[phase_start..];
    let phase_end = phase
        .find("fn finalize_phase2_process")
        .expect("final transcription phase boundary");
    let phase = &phase[..phase_end];

    assert!(phase.contains("self.pending_send_enter"));
    assert!(
        !phase.contains("parse_control_command"),
        "only Sherpa may supply wake-command authority"
    );
    assert!(
        !phase.contains("ControlIntent"),
        "final main ASR must not select STOP, SEND, or CANCEL"
    );
}

#[test]
fn bundled_kws_keeps_text_decoder_confusions_out_of_acoustic_graph() {
    let keywords = source("../src-tauri/resources/kws/keywords.txt");
    let rows: Vec<_> = keywords.lines().filter(|line| !line.is_empty()).collect();

    assert_eq!(rows.len(), 3, "acoustic graph must remain canonical-only");
    assert!(rows.iter().all(|line| line.starts_with("Z EH1 F ER0 ")));
    assert!(
        rows.iter().all(|line| line.contains(" :2 #0.15 @")),
        "field-calibrated score/threshold must remain locked"
    );
    assert!(rows.iter().any(|line| line.ends_with("@ZEPHYR_STOP")));
    assert!(rows.iter().any(|line| line.ends_with("@ZEPHYR_SEND")));
    assert!(rows.iter().any(|line| line.ends_with("@ZEPHYR_CANCEL")));
}

#[cfg(target_os = "macos")]
#[test]
fn bundled_macos_kws_runtime_matches_ttl_patch_manifest() {
    let manifest_text = source("../src-tauri/resources/kws/manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("valid KWS manifest");
    let patch_hash = "f8700cbb7efbab01363c3c4f901a5300fe21d53d8d4e307c435d2c1d2bd1a707";
    let dylib_hash = "1f16f3dc70afaa25e774f36113365f7ace836c927ea485350124ac3f3d836ecf";
    assert_eq!(manifest["native_patch"]["sha256"], patch_hash);
    assert_eq!(manifest["native_patch"]["active_keyword_ttl_frames"], 38);
    assert_eq!(manifest["native_patch"]["active_keyword_ttl_ms"], 1520);
    assert_eq!(manifest["macos_runtime"]["native_patch_sha256"], patch_hash);
    assert_eq!(
        manifest["macos_runtime"]["libsherpa-onnx-c-api.dylib_sha256"],
        dylib_hash
    );
    assert_eq!(
        manifest["windows_runtime"]["native_patch_sha256"],
        patch_hash
    );

    let runtime = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../src-tauri/resources/runtime/libsherpa-onnx-c-api.dylib");
    let output = std::process::Command::new("shasum")
        .args(["-a", "256", runtime.to_str().expect("runtime path")])
        .output()
        .expect("run shasum");
    assert!(output.status.success(), "shasum failed: {:?}", output);
    let actual = String::from_utf8(output.stdout)
        .expect("sha output")
        .split_whitespace()
        .next()
        .expect("sha value")
        .to_string();
    assert_eq!(actual, dylib_hash);
}

#[cfg(target_os = "windows")]
#[test]
fn bundled_windows_kws_runtime_matches_complete_patch_manifest() {
    let manifest_text = source("../src-tauri/resources/kws/manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("valid KWS manifest");
    let patch_hash = "f8700cbb7efbab01363c3c4f901a5300fe21d53d8d4e307c435d2c1d2bd1a707";
    let dll_hash = "9c9f0e25ab2c9ecf089dec934d70464963f98637fd5fed6ebd5fefa12b90cb7c";
    assert_eq!(
        manifest["windows_runtime"]["build"],
        "complete_patch_verified"
    );
    assert_eq!(
        manifest["windows_runtime"]["native_patch_sha256"],
        patch_hash
    );
    assert_eq!(
        manifest["windows_runtime"]["sherpa-onnx-c-api.dll_bytes"],
        2_883_072
    );
    assert_eq!(
        manifest["windows_runtime"]["sherpa-onnx-c-api.dll_sha256"],
        dll_hash
    );

    let runtime = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../src-tauri/resources/runtime/sherpa-onnx-c-api.dll");
    let output = std::process::Command::new("certutil")
        .args([
            "-hashfile",
            runtime.to_str().expect("runtime path"),
            "SHA256",
        ])
        .output()
        .expect("run certutil");
    assert!(output.status.success(), "certutil failed: {:?}", output);
    let hash_output = String::from_utf8(output.stdout).expect("hash output");
    let actual = hash_output
        .lines()
        .map(str::trim)
        .find(|line| line.len() == 64 && line.chars().all(|ch| ch.is_ascii_hexdigit()))
        .expect("SHA-256 line");
    assert_eq!(actual.to_ascii_lowercase(), dll_hash);
}

#[test]
fn model_reload_swaps_only_after_a_successful_warmup() {
    let helpers = source("src/worker_sections/section01.rs");
    let start = helpers
        .find("fn reload_asr_if_changed")
        .expect("reload helper");
    let reload = &helpers[start..];
    let end = reload.find("fn asr_reload_key").expect("reload boundary");
    let reload = &reload[..end];

    assert!(!reload.contains("let _ = transcribe_buffer"));
    let warmup = reload.find("warm_main_asr").expect("warmup call");
    let swap = reload.find("*model = m").expect("model swap");
    assert!(
        warmup < swap,
        "warmup must succeed before model replacement"
    );
    assert!(reload[warmup..swap].contains("map_err"));
}

#[test]
fn dead_asr_executor_disables_future_streaming_submissions() {
    let streaming = source("src/worker_sections/worker_streaming.rs");
    let submit = streaming
        .find("asr_executor.submit_probe")
        .expect("probe submission");
    let gate_start = streaming[..submit]
        .rfind("if !auto_stop_fired")
        .expect("probe submission gate");
    assert!(streaming[gate_start..submit].contains("!asr_executor_failed"));
}

#[test]
fn final_and_probe_bias_live_on_independent_models() {
    let executor = source("src/worker_sections/unified_probe_lane.rs");
    let helpers = source("src/worker_sections/section03.rs");
    let asr = source("src/asr_sections/section02.rs");

    assert!(executor.contains("apply_probe_context_bias(&mut probe_model)"));
    assert!(!executor.contains("let probe_model = &mut model"));
    assert!(helpers.contains("crate::asr::apply_utterance_bias(model);"));
    assert!(
        !asr.contains("LAST_SCREEN_HASH"),
        "final bias restoration must not be skipped by global screen hash"
    );
}

#[test]
fn kws_timed_decode_never_enters_the_main_inference_gate() {
    let asr = include_str!("../src/asr_sections/section01.rs");
    let lane = include_str!("../src/worker_sections/unified_probe_lane.rs");
    assert!(lane.contains("return model.transcribe_probe_result(buffer)"));
    assert!(asr.contains("AsrRuntime::SherpaKws(model)"));
    let probe_method = asr
        .split("pub(crate) fn transcribe_probe_result")
        .nth(1)
        .expect("explicit gate-free KWS method")
        .split("pub(crate) fn transcribe_result_under_lease")
        .next()
        .expect("bounded KWS method");
    assert!(!probe_method.contains("with_inference_lease"));
    assert!(!probe_method.contains("inference_gate"));
}

#[test]
fn failed_kws_timing_drops_stream_identity_before_retry() {
    let lane = source("src/worker_sections/unified_probe_lane.rs");
    let start = lane
        .find("if transcript.is_ok()")
        .expect("probe result bookkeeping");
    let bookkeeping = &lane[start..];
    let end = bookkeeping
        .find("let probe_ms")
        .expect("bookkeeping boundary");
    let bookkeeping = &bookkeeping[..end];

    assert!(bookkeeping.contains("probe_session_id.clear()"));
    assert!(bookkeeping.contains("probe_processed_total = 0"));
    assert!(bookkeeping.contains("probe_stream_base_sample = 0"));
}

#[test]
fn main_runtime_changes_bias_only_while_its_decode_lease_is_held() {
    let executor = source("src/worker_sections/unified_probe_lane.rs");
    let helpers = source("src/worker_sections/section03.rs");

    fn assert_atomic_sequence(source: &str, owner: &str, bias: &str, decode: &str) {
        let start = source
            .find(&format!("with_inference_lease(\"{owner}\""))
            .expect("lease owner");
        let closure = &source[start..];
        let end = closure.find("\n        })").expect("lease closure end");
        let closure = &closure[..end];
        let bias = closure.find(bias).expect("bias mutation inside lease");
        let decode = closure.find(decode).expect("decode inside lease");
        assert!(bias < decode, "bias must be applied before decode");
    }

    assert_atomic_sequence(
        &helpers,
        "final_asr",
        "apply_utterance_bias(model)",
        "transcribe_under_lease(conditioned)",
    );
    assert!(
        !executor.contains("with_inference_lease(\"command_trigger_probe\""),
        "CPU KWS must not wait on main-ASR device work"
    );
    assert_atomic_sequence(
        &helpers,
        "command_trigger_probe",
        "apply_probe_context_bias(model)",
        "transcribe_under_lease(conditioned)",
    );
    assert_atomic_sequence(
        &helpers,
        "file_asr",
        "apply_utterance_bias(model)",
        "transcribe_file_under_lease(conditioned)",
    );
}
