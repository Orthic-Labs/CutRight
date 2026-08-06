use std::fs;
use std::path::PathBuf;

fn source(relative: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(root.join(relative)).unwrap()
}

#[test]
fn owner_capture_covers_every_completed_recording_path() {
    let manual = source("src/worker_sections/worker_commands.rs");
    assert!(
        manual.contains("owner_diagnostics::capture_audio_parts(&session_id, &buffer, &[]);"),
        "manual Stop must retain exact PCM"
    );

    let zephyr_and_standalone = source("src/worker_sections/worker_probe_results.rs");
    assert!(
        zephyr_and_standalone
            .matches("owner_diagnostics::capture_audio_parts(&sid,")
            .count()
            >= 2,
        "Zephyr controls and standalone command completion need capture"
    );

    let silence = source("src/worker_sections/worker_streaming.rs");
    assert!(
        silence.contains("owner_diagnostics::capture_audio_parts(&sid,"),
        "silence/duration auto-stop must retain exact PCM"
    );
}

#[test]
fn owner_log_covers_raw_stripped_and_delivered_text() {
    let probe = source("src/worker_sections/worker_probe_results.rs");
    assert!(probe.contains("\"raw_transcript\""));
    assert!(probe.contains("\"stripped_transcript\""));

    let manual = source("src/worker_sections/worker_commands.rs");
    assert!(manual.contains("\"raw_transcript\""));

    let finalizer = source("src/runtime_sections/finalize_transcript.rs");
    assert!(finalizer.contains("\"delivered_transcript\""));
}

#[test]
fn owner_flag_also_enables_unredacted_probe_trace() {
    let settings = source("src/settings.rs");
    assert!(
        settings.contains("crate::owner_diagnostics::enabled()"),
        "one env flag must enable existing unredacted probe tracing"
    );
}
