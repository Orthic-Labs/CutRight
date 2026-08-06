use std::fs;
use std::path::PathBuf;

fn source(relative: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(root.join(relative)).unwrap()
}

#[test]
fn active_trigger_policy_uses_current_low_latency_kws_cadence() {
    let constants = source("src/worker_sections/section01.rs");
    assert!(constants.contains("const TAIL_CHECK_MS: u64 = 20;"));
    assert!(constants.contains("const TAIL_ACTIVE_AFTER_SPEECH_MS: u64 = 2_000;"));
    assert!(!constants.contains("CONTROL_PREFIX_FOLLOWUP_PREROLL_SAMPLES"));

    let worker = source("src/worker_sections/section03.rs");
    assert!(!worker.contains("stable_fuzzy_wake"));
    assert!(!worker.contains("control_prefix_followup_start_sample"));
    assert!(!worker.contains("tail_pause_probe_due"));
}

#[test]
fn acoustic_graph_stays_canonical_while_text_parser_handles_asr_variants() {
    let graph = source("../src-tauri/resources/kws/keywords.txt");
    assert_eq!(
        graph.lines().filter(|line| !line.trim().is_empty()).count(),
        3
    );
    assert!(!graph.contains("STAH"));
    assert!(!graph.contains("CANCER"));
}

#[test]
fn native_active_keyword_ttl_is_tracked_while_wrapper_stays_single_stream() {
    let wrapper = source("src/sherpa_kws.rs");
    assert!(!wrapper.contains("prefix_spotter"));
    assert!(!wrapper.contains("recent_audio"));
    assert!(!wrapper.contains("recreate_prefix_stream"));

    let patch = source("native/sherpa/patches/0001-expire-active-keyword-hypotheses.patch");
    assert!(patch.contains("kMaxKeywordActiveFrames = 38"));
    assert!(patch.contains("bool KeywordPathExpired"));
    assert!(patch.contains("current_frame - first_frame > kMaxKeywordActiveFrames"));
    let prefilter = &patch[..patch.find("// Due to merging paths").expect("beam expansion")];
    assert!(patch.contains("void RemoveIf(Predicate predicate)"));
    assert!(prefilter.contains("cur[b].RemoveIf"));
    assert!(prefilter.contains("return KeywordPathExpired(hyp, current_frame)"));
    assert!(prefilter.contains("if (cur[b].Size() == 0)"));
    assert!(!prefilter.contains("active_hyps"));
    assert!(!prefilter.contains("GetMostProbable(false)"));
    assert!(patch.contains("diff --git a/sherpa-onnx/csrc/keyword-spotter-transducer-impl.h"));
    assert!(patch.contains("-        Reset(s);"));
    assert!(!patch.contains("+        Reset(s);"));

    let readme = source("native/sherpa/README.md");
    assert!(readme.contains("142807252687d81b40d6315f23470a1512a00de3"));
    assert!(readme.contains("f8700cbb7efbab01363c3c4f901a5300fe21d53d8d4e307c435d2c1d2bd1a707"));
    assert!(readme.contains("38` at 40 ms/frame retains every active keyword context for 1.52 s"));
}

#[test]
fn production_supervisor_does_not_force_disable_kws() {
    let supervisor = source("../src-tauri/src/engine_supervisor_sections/section02.rs");
    assert!(!supervisor.contains("HR_DISABLE_KWS"));
}

#[test]
fn probe_only_bias_matches_mac_policy() {
    let asr = source("src/asr_sections/section02.rs");
    assert!(asr.contains("const DEFAULT_CONTEXT_BIAS_SCORE: f32 = 1.0;"));
    assert!(asr.contains("const PROBE_CONTEXT_BIAS_SCORE: f32 = 5.0;"));
}
