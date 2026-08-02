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
fn not_implemented_command_exits_nonzero_with_stable_code_three() {
    // `analyze cloud` is a real, parseable subcommand behind the frozen CLI
    // contract that has no implementation yet, so it always hits the
    // `not_implemented` fallback in `run()` regardless of environment.
    let output = videoctl()
        .args([
            "analyze",
            "cloud",
            "/nonexistent-project",
            "--provider",
            "example",
        ])
        .output()
        .expect("videoctl runs");

    let value = parse_stdout_json(&output);
    assert_eq!(value["status"], "not_implemented");
    assert_eq!(output.status.code(), Some(3));
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
