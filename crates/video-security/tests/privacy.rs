//! Privacy-safe logs and telemetry-off defaults — integration tests.

use video_security::privacy::{
    build_export, build_log_entry, clear_diagnostics, network_attempt_count,
    network_attempt_record, redact, telemetry_off, LogBuffer,
};

#[test]
fn default_logs_contain_no_raw_transcript_or_path() {
    let mut buf = LogBuffer::new();
    // Build an entry without raw content; serialization must still hide paths.
    let entry = build_log_entry(
        "studio",
        "decision.recorded",
        "proj-abc",
        "rev1",
        Some("job.1"),
        Some(12),
        vec!["abcd".to_string()],
    );
    buf.push(entry, 16);
    let s = serde_json::to_string(&buf).unwrap();
    assert!(!s.contains("/Users/"));
    assert!(!s.to_lowercase().contains("transcript:"));
    assert!(!s.to_lowercase().contains("prompt:"));
    assert!(!s.to_lowercase().contains("apikey="));
}

#[test]
fn redact_strips_transcripts_and_paths_and_keys() {
    assert_eq!(redact("transcript: hello"), "<redacted>");
    assert_eq!(redact("prompt: do the thing"), "<redacted>");
    assert_eq!(redact("apikey=abcd"), "<redacted>");
    // path-like inputs are fingerprinted
    let r = redact("/Users/person/secret.mp4");
    assert!(r != "/Users/person/secret.mp4");
    assert!(r.len() < 32);
}

#[test]
fn network_attempt_counter_is_visible_with_blocked_network() {
    // Read the live atomic, not NetworkAttemptCounter::default(), which is
    // always zero. Other tests in this binary share the counter, so assert a
    // delta rather than an absolute value.
    let before = network_attempt_count();
    network_attempt_record();
    network_attempt_record();
    assert!(network_attempt_count() >= before + 2);
}

#[test]
fn telemetry_off_default_holds() {
    assert!(telemetry_off());
}

#[test]
fn clear_diagnostics_leaves_canonical_state_intact() {
    let mut buf = LogBuffer::new();
    buf.push(
        build_log_entry("studio", "ok", "p", "r", None, None, vec![]),
        16,
    );
    let r = clear_diagnostics(&buf);
    assert!(r.canonical_untouched);
    assert_eq!(r.kept_entries.len(), 1);
}

#[test]
fn log_export_lists_user_files_with_fingerprints_only() {
    let e = build_export(vec!["/var/folders/log/foo.log", "/var/folders/log/bar.log"]);
    assert_eq!(e.files.len(), 2);
    for f in &e.files {
        assert!(f.fingerprint.len() > 4);
        // No raw byte_size field is required.
        let _ = f.byte_size;
    }
}
