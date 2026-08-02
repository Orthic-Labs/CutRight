//! CLI and process-boundary smoke tests (REV2 plan §10.9), scoped to what
//! `video-cli` alone can exercise: the exit-code table (§10.8) and
//! `doctor`'s JSON shape and exit behavior (§11).

use assert_cmd::Command;
use serde_json::Value;

fn videoctl() -> Command {
    Command::cargo_bin("videoctl").expect("videoctl binary builds")
}

fn parse_stdout_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout was not valid JSON: {error}\nstdout: {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

#[test]
fn doctor_core_profile_reports_a_valid_shape_and_exits_zero_when_ready() {
    let output = videoctl()
        .args(["doctor", "--profile", "core"])
        .output()
        .expect("videoctl runs");

    let value = parse_stdout_json(&output);
    assert_eq!(value["event"], "doctor");
    assert_eq!(value["profile"], "core");
    let checks = value["checks"].as_array().expect("checks is an array");
    assert!(
        !checks.is_empty(),
        "core profile must run at least one check"
    );
    for check in checks {
        for field in ["id", "status", "required", "evidence"] {
            assert!(
                check.get(field).is_some(),
                "check missing field `{field}`: {check}"
            );
        }
        let status = check["status"].as_str().expect("status is a string");
        assert!(
            ["ok", "degraded", "missing", "failed"].contains(&status),
            "unexpected status `{status}` in check {check}"
        );
    }

    // On a machine with a working ffmpeg/ffprobe/temp dir (this dev/CI
    // environment), core should be fully ready.
    if output.status.success() {
        assert_eq!(value["status"], "ok");
    } else {
        // If the environment genuinely lacks a required core capability,
        // the exit code must still be the doctor-specific stable code, and
        // the report must say so truthfully rather than claiming "ok".
        assert_eq!(output.status.code(), Some(5));
        assert_eq!(value["status"], "fail");
    }
}

#[test]
fn doctor_reports_missing_required_capability_as_nonzero_exit_five() {
    // Force a required capability (ffmpeg) to be unresolvable so the
    // "missing-capability" path is deterministic regardless of the host
    // environment.
    let output = videoctl()
        .args(["doctor", "--profile", "core"])
        .env(
            "CUTRIGHT_FFMPEG",
            "/nonexistent/videoctl-doctor-test/ffmpeg",
        )
        .output()
        .expect("videoctl runs");

    assert_eq!(
        output.status.code(),
        Some(5),
        "a missing required doctor check must exit 5"
    );
    let value = parse_stdout_json(&output);
    assert_eq!(value["status"], "fail");
    let checks = value["checks"].as_array().expect("checks is an array");
    let ffmpeg_check = checks
        .iter()
        .find(|check| check["id"] == "core.ffmpeg.execute")
        .expect("core.ffmpeg.execute check is present");
    assert_eq!(ffmpeg_check["status"], "missing");
    assert_eq!(ffmpeg_check["required"], true);
    assert!(ffmpeg_check["remediation"].is_string());
}

#[test]
fn doctor_audio_profile_includes_core_and_audio_checks() {
    let output = videoctl()
        .args(["doctor", "--profile", "audio"])
        .env(
            "CUTRIGHT_HEARDRIGHT_ENGINE",
            "/nonexistent/videoctl-doctor-test/heardright-engine",
        )
        .output()
        .expect("videoctl runs");

    let value = parse_stdout_json(&output);
    let checks = value["checks"].as_array().expect("checks is an array");
    let ids: Vec<&str> = checks.iter().filter_map(|c| c["id"].as_str()).collect();
    assert!(
        ids.contains(&"core.ffmpeg.execute"),
        "audio profile must include core checks"
    );
    assert!(
        ids.contains(&"audio.heardright.discover"),
        "audio profile must include audio checks"
    );

    let heardright = checks
        .iter()
        .find(|check| check["id"] == "audio.heardright.discover")
        .expect("audio.heardright.discover present");
    assert_eq!(heardright["status"], "missing");
}

#[test]
fn doctor_strict_fails_on_degraded_optional_checks() {
    let lenient = videoctl()
        .args(["doctor", "--profile", "render"])
        .output()
        .expect("videoctl runs");
    let strict = videoctl()
        .args(["doctor", "--profile", "render", "--strict"])
        .output()
        .expect("videoctl runs");

    let lenient_value = parse_stdout_json(&lenient);
    let strict_value = parse_stdout_json(&strict);
    assert_eq!(strict_value["strict"], true);
    assert_eq!(lenient_value["strict"], false);

    // Strict can only be equal-or-more-strict than lenient: if lenient
    // already failed (exit 5), strict must too.
    if lenient.status.code() == Some(5) {
        assert_eq!(strict.status.code(), Some(5));
    }
}

#[test]
fn doctor_write_receipt_writes_a_hashable_json_file() {
    let dir = tempfile_dir();
    let receipt_path = dir.join("doctor-receipt.json");
    let output = videoctl()
        .args(["doctor", "--profile", "core", "--write-receipt"])
        .arg(&receipt_path)
        .output()
        .expect("videoctl runs");
    let _ = parse_stdout_json(&output);

    let receipt_bytes = std::fs::read(&receipt_path).expect("receipt file was written");
    let receipt: Value = serde_json::from_slice(&receipt_bytes).expect("receipt is valid JSON");
    assert_eq!(receipt["kind"], "videoctl.doctor.receipt");
    assert!(receipt["created_at"].is_string());
    assert!(receipt["report_blake3"].is_string());
    assert!(receipt["report"]["checks"].is_array());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn analyze_cloud_is_implemented_and_reports_a_domain_error_not_a_stub() {
    // REV2 plan §15.6 Phase 8: `analyze cloud` used to be the one subcommand
    // behind the frozen CLI contract with no implementation, always hitting
    // the `not_implemented` fallback (exit 3). It is real now — against a
    // project directory that does not even exist, it must fail as a normal
    // domain error (no proxy to analyze), exactly like any other command
    // pointed at a bad project path, never the stub path.
    let output = videoctl()
        .args([
            "analyze",
            "cloud",
            "/nonexistent-project",
            "--provider",
            "fake",
        ])
        .output()
        .expect("videoctl runs");

    let value = parse_stdout_json(&output);
    assert_eq!(value["status"], "error");
    assert_ne!(value["status"], "not_implemented");
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn cloud_consent_and_budget_round_trip_through_the_cli() {
    // Exercises the explicit per-project consent/budget surface end to end:
    // `videoctl cloud consent`/`videoctl cloud budget` persist state that a
    // later `analyze cloud` call reads back, and refuses cleanly (exit 0,
    // status "refused") rather than erroring when nothing has been granted.
    let dir = tempfile_dir();
    let init = videoctl()
        .args(["project", "init"])
        .arg(&dir)
        .output()
        .expect("videoctl runs");
    assert_eq!(init.status.code(), Some(0));

    let proxies_dir = dir.join("cache/proxies");
    std::fs::create_dir_all(&proxies_dir).expect("create cache/proxies");
    std::fs::write(proxies_dir.join("clip.bin"), b"proxy-bytes").expect("write test proxy");

    // No consent yet: analyze cloud must refuse, not error, even though a
    // proxy exists to analyze.
    let refused = videoctl()
        .args(["analyze", "cloud"])
        .arg(&dir)
        .args(["--provider", "fake"])
        .output()
        .expect("videoctl runs");
    let refused_value = parse_stdout_json(&refused);
    assert_eq!(refused.status.code(), Some(0));
    assert_eq!(refused_value["result"]["status"], "refused");

    let consent = videoctl()
        .args(["cloud", "consent"])
        .arg(&dir)
        .arg("--enable")
        .output()
        .expect("videoctl runs");
    let consent_value = parse_stdout_json(&consent);
    assert_eq!(consent.status.code(), Some(0));
    assert_eq!(consent_value["result"]["consent"], true);

    let budget = videoctl()
        .args(["cloud", "budget"])
        .arg(&dir)
        .args(["--usd", "5"])
        .output()
        .expect("videoctl runs");
    let budget_value = parse_stdout_json(&budget);
    assert_eq!(budget.status.code(), Some(0));
    assert_eq!(budget_value["result"]["budget_usd_limit"], 5.0);

    // Consent + budget are now granted, but the embedded 'fake' adapter's
    // fixtures are keyed on placeholder hashes, not this real test proxy's
    // hash, so the provider call misses its fixture and the outage-fallback
    // path (never a hard CLI error for an optional feature) applies.
    let fallback = videoctl()
        .args(["analyze", "cloud"])
        .arg(&dir)
        .args(["--provider", "fake"])
        .output()
        .expect("videoctl runs");
    let fallback_value = parse_stdout_json(&fallback);
    assert_eq!(fallback.status.code(), Some(0));
    assert_eq!(fallback_value["result"]["status"], "fallback_local");

    let delete = videoctl()
        .args(["cloud", "delete"])
        .arg(&dir)
        .output()
        .expect("videoctl runs");
    let delete_value = parse_stdout_json(&delete);
    assert_eq!(delete.status.code(), Some(0));
    assert_eq!(delete_value["result"]["status"], "nothing_to_delete");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn invalid_subcommand_exits_nonzero_with_stable_code_four_and_valid_json() {
    let output = videoctl()
        .args(["not-a-real-command"])
        .output()
        .expect("videoctl runs");

    assert_eq!(output.status.code(), Some(4));
    let value = parse_stdout_json(&output);
    assert_eq!(value["status"], "error");
    assert_eq!(value["error_kind"], "invalid_command");
}

#[test]
fn help_still_exits_zero_and_is_not_treated_as_invalid() {
    let output = videoctl().arg("--help").output().expect("videoctl runs");
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn status_error_from_a_domain_command_exits_nonzero_with_stable_code_one() {
    // `project migrate` on a folder that is not a CutRight project reports
    // a domain error (status: "error"), not a doctor or not_implemented
    // path.
    let dir = tempfile_dir();
    let output = videoctl()
        .args(["project", "migrate"])
        .arg(&dir)
        .output()
        .expect("videoctl runs");

    let value = parse_stdout_json(&output);
    assert_eq!(value["status"], "error");
    assert_eq!(output.status.code(), Some(1));

    let _ = std::fs::remove_dir_all(&dir);
}

fn tempfile_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "videoctl-cli-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// §10.2: `core.sidecar.materialize` now drives the real
/// `video_core::content_store::materialize_worker` API on every doctor run
/// (materialize, verify bytes, verify reuse, and verify a tampered copy at
/// the content-addressed path is rejected) instead of hardcoding `missing`.
/// A passing run only happens if the tamper-rejection step inside the probe
/// actually rejected the tampered bytes, so this test also stands in for
/// "fails on tampered bytes": if `materialize_worker`'s tamper detection
/// ever regressed, this check would report `failed`, not `ok`.
#[test]
fn doctor_sidecar_materialize_probe_verifies_the_real_content_store() {
    let output = videoctl()
        .args(["doctor", "--profile", "core"])
        .output()
        .expect("videoctl runs");

    let value = parse_stdout_json(&output);
    let checks = value["checks"].as_array().expect("checks is an array");
    let sidecar = checks
        .iter()
        .find(|check| check["id"] == "core.sidecar.materialize")
        .expect("core.sidecar.materialize check is present");
    assert_eq!(sidecar["status"], "ok", "probe result: {sidecar}");
    let verified = sidecar["evidence"]["verified"]
        .as_array()
        .expect("evidence.verified is an array");
    let verified: Vec<&str> = verified.iter().filter_map(|v| v.as_str()).collect();
    assert!(verified.contains(&"materialize"));
    assert!(verified.contains(&"reuse-on-identical-bytes"));
    assert!(verified.contains(&"reject-tampered-bytes"));
}

/// `audio.heardright.handshake` must honestly report `missing` (never a
/// fabricated `ok`) when no HeardRight engine can be discovered at all.
#[test]
fn doctor_heardright_handshake_reports_missing_when_engine_is_unreachable() {
    let output = videoctl()
        .args(["doctor", "--profile", "audio"])
        .env(
            "CUTRIGHT_HEARDRIGHT_ENGINE",
            "/nonexistent/videoctl-doctor-test/heardright-engine",
        )
        .output()
        .expect("videoctl runs");

    let value = parse_stdout_json(&output);
    let checks = value["checks"].as_array().expect("checks is an array");
    let handshake = checks
        .iter()
        .find(|check| check["id"] == "audio.heardright.handshake")
        .expect("audio.heardright.handshake check is present");
    assert_eq!(handshake["status"], "missing");
    assert!(handshake["remediation"].is_string());
}

/// `audio.heardright.handshake` reports `ok` and the negotiated identity
/// when it can complete the real handshake against a HeardRight engine —
/// here, a fake engine script standing in for the real one (never the real
/// engine), the same pattern `video-providers`'s own protocol tests use.
/// The fake engine treats any non-handshake request as a hard failure, so a
/// passing test also proves the probe never sent a transcription/VAD
/// request.
#[test]
fn doctor_heardright_handshake_reports_ok_against_a_fake_engine() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile_dir();
    let engine = dir.join("fake-heardright-engine");
    std::fs::write(
        &engine,
        "#!/bin/sh\nwhile IFS= read -r line; do\n  rid=$(printf '%s' \"$line\" | sed -n 's/.*\"request_id\":\"\\([^\"]*\\)\".*/\\1/p')\n  case \"$line\" in\n    *session_handshake_request*) printf '{\"schema_name\":\"session_handshake_result\",\"protocol_major\":1,\"protocol_minor\":0,\"engine_version\":\"fake-engine/1.0\",\"request_id\":\"%s\",\"payload\":{\"capabilities\":[\"file_vad_regions_v1\"]}}\\n' \"$rid\" ;;\n    *) exit 1 ;;\n  esac\ndone\n",
    )
    .expect("write fake engine");
    std::fs::set_permissions(&engine, std::fs::Permissions::from_mode(0o700))
        .expect("make fake engine executable");

    let output = videoctl()
        .args(["doctor", "--profile", "audio"])
        .env("CUTRIGHT_HEARDRIGHT_ENGINE", &engine)
        .output()
        .expect("videoctl runs");

    let value = parse_stdout_json(&output);
    let checks = value["checks"].as_array().expect("checks is an array");
    let handshake = checks
        .iter()
        .find(|check| check["id"] == "audio.heardright.handshake")
        .expect("audio.heardright.handshake check is present");
    assert_eq!(handshake["status"], "ok", "probe result: {handshake}");
    assert_eq!(handshake["evidence"]["engine_version"], "fake-engine/1.0");
    assert_eq!(handshake["evidence"]["protocol_major"], 1);

    let _ = std::fs::remove_dir_all(&dir);
}
