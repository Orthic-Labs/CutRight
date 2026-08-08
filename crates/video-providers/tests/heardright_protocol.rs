//! Black-box process-boundary tests for the HeardRight client protocol
//! (hardening plan §10.9): request correlation, malformed JSON, timeout,
//! early EOF, and the restart-once policy, all driven through the crate's
//! real public API against fake engine scripts — never HeardRight's real
//! engine.
//!
//! Per the v2 standalone boundary (§9.3) the release code no longer resolves
//! the engine through environment overrides or bare-name lookup: the engine
//! ships in the signed speech runtime pack. These tests therefore construct
//! the provider through the explicit pack-style seam
//! `HeardRightProvider::with_engine(<fake engine path>)` and use only the
//! timeout env overrides (`CUTRIGHT_HEARDRIGHT_REQUEST_TIMEOUT_SECS` /
//! `CUTRIGHT_HEARDRIGHT_HANDSHAKE_TIMEOUT_SECS`). Those timeout variables
//! are process-global, so every test holds `ENV_LOCK` for its duration to
//! avoid cross-test interference.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use video_core::providers::VadRequest;
use video_providers::{HeardRightProvider, ProviderError};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("{prefix}-{unique}"));
    fs::create_dir_all(&root).expect("create test directory");
    root
}

/// Write a fake HeardRight engine as a POSIX shell script and return its
/// path, so the caller can construct a provider through the explicit
/// pack-style seam (`HeardRightProvider::with_engine`) and inspect side
/// files (e.g. a start-count log) and clean up afterward.
fn write_fake_engine(root: &Path, script_body: &str) -> PathBuf {
    let engine = root.join("fake-engine");
    fs::write(&engine, format!("#!/bin/sh\n{script_body}\n")).expect("write fake engine");
    fs::set_permissions(&engine, fs::Permissions::from_mode(0o700))
        .expect("make fake engine executable");
    engine
}

fn clear_env() {
    std::env::remove_var("CUTRIGHT_HEARDRIGHT_REQUEST_TIMEOUT_SECS");
    std::env::remove_var("CUTRIGHT_HEARDRIGHT_HANDSHAKE_TIMEOUT_SECS");
}

const HANDSHAKE_REPLY: &str = "printf '{\"schema_name\":\"session_handshake_result\",\"protocol_major\":1,\"protocol_minor\":0,\"engine_version\":\"fake-engine/1.0\",\"request_id\":\"%s\",\"payload\":{\"capabilities\":[]}}\\n' \"$rid\"";

fn vad_request(root: &Path) -> VadRequest {
    let audio_path = root.join("source-a-16k.f32");
    fs::write(&audio_path, [0u8; 16]).expect("write fake decoded audio");
    VadRequest {
        source_id: "source-a".into(),
        audio_path,
        sample_rate: 16_000,
        threshold: 0.5,
    }
}

#[test]
fn response_with_mismatched_request_id_is_rejected_as_correlation_error() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = unique_temp_dir("cutright-hs-correlation");
    // The engine answers every non-handshake request with a *fixed*,
    // deliberately wrong request_id, never the caller's own.
    let engine = write_fake_engine(
        &root,
        &format!(
            "while IFS= read -r line; do\n  rid=$(printf '%s' \"$line\" | sed -n 's/.*\"request_id\":\"\\([^\"]*\\)\".*/\\1/p')\n  case \"$line\" in\n    *session_handshake_request*) {HANDSHAKE_REPLY} ;;\n    *) printf '{{\"schema_name\":\"file_vad_result\",\"request_id\":\"not-the-callers-id\",\"payload\":{{\"sample_rate\":16000,\"regions\":[]}}}}\\n' ;;\n  esac\ndone"
        ),
    );

    let provider = HeardRightProvider::with_engine(engine);
    let error = provider
        .analyze_file_vad(&vad_request(&root))
        .expect_err("mismatched request_id must be rejected");
    assert!(
        matches!(error, ProviderError::Correlation { .. }),
        "expected Correlation, got {error:?}"
    );

    clear_env();
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn malformed_json_response_surfaces_as_a_json_error() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = unique_temp_dir("cutright-hs-malformed-json");
    let engine = write_fake_engine(
        &root,
        &format!(
            "while IFS= read -r line; do\n  rid=$(printf '%s' \"$line\" | sed -n 's/.*\"request_id\":\"\\([^\"]*\\)\".*/\\1/p')\n  case \"$line\" in\n    *session_handshake_request*) {HANDSHAKE_REPLY} ;;\n    *) printf 'this is not json\\n' ;;\n  esac\ndone"
        ),
    );

    let provider = HeardRightProvider::with_engine(engine);
    let error = provider
        .analyze_file_vad(&vad_request(&root))
        .expect_err("malformed JSON must be rejected");
    assert!(
        matches!(error, ProviderError::Json(_)),
        "expected Json, got {error:?}"
    );

    clear_env();
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn a_hanging_request_times_out_then_recovers_after_one_restart() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("CUTRIGHT_HEARDRIGHT_REQUEST_TIMEOUT_SECS", "1");
    let root = unique_temp_dir("cutright-hs-timeout");
    let starts = root.join("starts.log");
    // First spawn: handshakes fine, then hangs forever on the real request
    // (never responds, never exits) — must time out rather than block. On
    // restart, the fresh (second) spawn behaves normally.
    let engine = write_fake_engine(
        &root,
        &format!(
            "echo start >> '{}'\nwhile IFS= read -r line; do\n  rid=$(printf '%s' \"$line\" | sed -n 's/.*\"request_id\":\"\\([^\"]*\\)\".*/\\1/p')\n  case \"$line\" in\n    *session_handshake_request*) {HANDSHAKE_REPLY} ;;\n    *) if [ \"$(wc -l < '{}')\" -eq 1 ]; then sleep 30; else printf '{{\"schema_name\":\"file_vad_result\",\"request_id\":\"%s\",\"payload\":{{\"sample_rate\":16000,\"regions\":[]}}}}\\n' \"$rid\"; fi ;;\n  esac\ndone",
            starts.display(),
            starts.display()
        ),
    );

    let provider = HeardRightProvider::with_engine(engine);
    let result = provider.analyze_file_vad(&vad_request(&root));
    assert!(
        result.is_ok(),
        "expected recovery after one restart, got {result:?}"
    );
    let spawn_count = fs::read_to_string(&starts)
        .expect("read starts")
        .lines()
        .count();
    assert_eq!(spawn_count, 2, "expected exactly one restart (two spawns)");

    clear_env();
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn engine_exiting_mid_request_recovers_after_one_restart() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("CUTRIGHT_HEARDRIGHT_REQUEST_TIMEOUT_SECS", "5");
    let root = unique_temp_dir("cutright-hs-early-eof");
    let starts = root.join("starts.log");
    // First spawn: handshakes fine, then exits immediately on the real
    // request (stdout closes -> early EOF). Second spawn (after restart)
    // behaves normally.
    let engine = write_fake_engine(
        &root,
        &format!(
            "echo start >> '{}'\nwhile IFS= read -r line; do\n  rid=$(printf '%s' \"$line\" | sed -n 's/.*\"request_id\":\"\\([^\"]*\\)\".*/\\1/p')\n  case \"$line\" in\n    *session_handshake_request*) {HANDSHAKE_REPLY} ;;\n    *) if [ \"$(wc -l < '{}')\" -eq 1 ]; then exit 0; else printf '{{\"schema_name\":\"file_vad_result\",\"request_id\":\"%s\",\"payload\":{{\"sample_rate\":16000,\"regions\":[]}}}}\\n' \"$rid\"; fi ;;\n  esac\ndone",
            starts.display(),
            starts.display()
        ),
    );

    let provider = HeardRightProvider::with_engine(engine);
    let result = provider.analyze_file_vad(&vad_request(&root));
    assert!(
        result.is_ok(),
        "expected recovery after one restart, got {result:?}"
    );
    let spawn_count = fs::read_to_string(&starts)
        .expect("read starts")
        .lines()
        .count();
    assert_eq!(spawn_count, 2, "expected exactly one restart (two spawns)");

    clear_env();
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn restart_budget_is_exactly_one_when_the_engine_keeps_failing() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::env::set_var("CUTRIGHT_HEARDRIGHT_REQUEST_TIMEOUT_SECS", "5");
    let root = unique_temp_dir("cutright-hs-restart-budget");
    let starts = root.join("starts.log");
    // Every spawn handshakes fine, then always exits immediately on the
    // real request. A single request() call must therefore spawn exactly
    // twice (initial + one restart) and then return the failure — never an
    // unbounded retry loop.
    let engine = write_fake_engine(
        &root,
        &format!(
            "echo start >> '{}'\nwhile IFS= read -r line; do\n  rid=$(printf '%s' \"$line\" | sed -n 's/.*\"request_id\":\"\\([^\"]*\\)\".*/\\1/p')\n  case \"$line\" in\n    *session_handshake_request*) {HANDSHAKE_REPLY} ;;\n    *) exit 0 ;;\n  esac\ndone",
            starts.display()
        ),
    );

    let provider = HeardRightProvider::with_engine(engine);
    let error = provider
        .analyze_file_vad(&vad_request(&root))
        .expect_err("an always-crashing engine must fail after the restart budget");
    assert!(
        matches!(error, ProviderError::UnexpectedExit { .. }),
        "expected UnexpectedExit, got {error:?}"
    );
    let spawn_count = fs::read_to_string(&starts)
        .expect("read starts")
        .lines()
        .count();
    assert_eq!(
        spawn_count, 2,
        "expected exactly initial spawn + one restart, no more"
    );

    clear_env();
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn engine_protocol_major_mismatch_is_rejected_without_starting_work() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = unique_temp_dir("cutright-hs-major-mismatch");
    // Handshake reports an incompatible major version.
    let engine = write_fake_engine(
        &root,
        "while IFS= read -r line; do\n  rid=$(printf '%s' \"$line\" | sed -n 's/.*\"request_id\":\"\\([^\"]*\\)\".*/\\1/p')\n  case \"$line\" in\n    *session_handshake_request*) printf '{\"schema_name\":\"session_handshake_result\",\"protocol_major\":99,\"protocol_minor\":0,\"engine_version\":\"fake-engine/99\",\"request_id\":\"%s\",\"payload\":{}}\\n' \"$rid\" ;;\n    *) printf '{}\\n' ;;\n  esac\ndone",
    );

    let provider = HeardRightProvider::with_engine(engine);
    let error = provider
        .analyze_file_vad(&vad_request(&root))
        .expect_err("protocol major mismatch must be rejected");
    assert!(
        matches!(
            error,
            ProviderError::ProtocolMajorMismatch { engine_major: 99 }
        ),
        "expected ProtocolMajorMismatch, got {error:?}"
    );

    clear_env();
    fs::remove_dir_all(root).expect("cleanup");
}
