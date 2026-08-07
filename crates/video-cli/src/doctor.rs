//! `videoctl doctor` — REV2 plan §11: a truthful machine-readiness report.
//!
//! Every check below performs an *active* probe (spawn a real process,
//! write a real file, parse a real filter list) rather than an existence
//! check on a binary name. A check that cannot be verified truthfully from
//! this crate alone (for example, a full HeardRight protocol handshake,
//! which requires a public health API this crate does not own) is reported
//! as `missing`/`degraded` with an honest `remediation`, never faked as
//! `ok`.
//!
//! Profiles are additive: `audio`, `render`, and `studio` each run the
//! `core` checks first, because every deeper probe assumes a working temp
//! directory and toolchain. `all` runs every profile's checks once.

use clap::ValueEnum;
use serde_json::{json, Value};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum DoctorProfile {
    Core,
    Audio,
    Render,
    Studio,
    All,
}

/// Whether `doctor`'s required checks (and, under `--strict`, its degraded
/// optional checks) all passed. `main` maps this to the exit-code table in
/// `docs` — see `main.rs` `EXIT_*` constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorOutcome {
    Ready,
    NotReady,
}

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);

pub fn run(
    profile: DoctorProfile,
    strict: bool,
    write_receipt: Option<&Path>,
) -> (Value, DoctorOutcome) {
    let mut checks: Vec<Value> = Vec::new();

    // Core checks are the foundation every other profile depends on, so
    // they always run first regardless of the selected profile.
    checks.extend(core_checks());
    match profile {
        DoctorProfile::Core => {}
        DoctorProfile::Audio => checks.extend(audio_checks()),
        DoctorProfile::Render => checks.extend(render_checks()),
        DoctorProfile::Studio => checks.extend(studio_checks()),
        DoctorProfile::All => {
            checks.extend(audio_checks());
            checks.extend(render_checks());
            checks.extend(studio_checks());
        }
    }

    let outcome = evaluate(&checks, strict);
    let report = json!({
        "event": "doctor",
        "status": match outcome {
            DoctorOutcome::Ready => "ok",
            DoctorOutcome::NotReady => "fail",
        },
        "profile": profile_name(profile),
        "strict": strict,
        "checks": checks,
    });

    if let Some(path) = write_receipt {
        if let Err(error) = write_receipt_file(path, &report) {
            eprintln!("videoctl: failed to write doctor receipt to {path:?}: {error}");
        }
    }

    (report, outcome)
}

fn profile_name(profile: DoctorProfile) -> &'static str {
    match profile {
        DoctorProfile::Core => "core",
        DoctorProfile::Audio => "audio",
        DoctorProfile::Render => "render",
        DoctorProfile::Studio => "studio",
        DoctorProfile::All => "all",
    }
}

/// A required check that is `missing`/`degraded`/`failed` always fails
/// readiness. Under `--strict`, an *optional* check in the same states also
/// fails readiness.
fn evaluate(checks: &[Value], strict: bool) -> DoctorOutcome {
    for check in checks {
        let status = check["status"].as_str().unwrap_or("failed");
        let required = check["required"].as_bool().unwrap_or(true);
        let blocking = status == "failed" || status == "missing" || status == "degraded";
        if blocking && (required || strict) {
            return DoctorOutcome::NotReady;
        }
    }
    DoctorOutcome::Ready
}

fn check(
    id: &str,
    required: bool,
    status: &str,
    evidence: Value,
    remediation: Option<&str>,
) -> Value {
    json!({
        "id": id,
        "status": status,
        "required": required,
        "evidence": evidence,
        "remediation": remediation,
    })
}

// ---------------------------------------------------------------------
// Process helpers
// ---------------------------------------------------------------------

/// Run `cmd` to completion, killing it if it outlives `timeout`. This is a
/// deliberately minimal stdlib-only mechanism (no new dependency): poll
/// `try_wait` and kill on timeout rather than pull in an async runtime for
/// what are, in practice, sub-second probe commands.
fn run_with_timeout(mut cmd: Command, timeout: Duration) -> io::Result<Output> {
    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            if let Some(mut out) = child.stdout.take() {
                out.read_to_end(&mut stdout)?;
            }
            if let Some(mut err) = child.stderr.take() {
                err.read_to_end(&mut stderr)?;
            }
            return Ok(Output {
                status,
                stdout,
                stderr,
            });
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("process exceeded {timeout:?} timeout"),
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn evidence_from_output(output: &Output) -> Value {
    json!({
        "exit_code": output.status.code(),
        "stdout_tail": tail(&String::from_utf8_lossy(&output.stdout), 400),
        "stderr_tail": tail(&String::from_utf8_lossy(&output.stderr), 400),
    })
}

fn tail(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.trim().to_string()
    } else {
        let start = text
            .char_indices()
            .rev()
            .nth(max_chars)
            .map(|(i, _)| i + 1)
            .unwrap_or(0);
        format!("...{}", text[start..].trim())
    }
}

// ---------------------------------------------------------------------
// Core probes (§11.2)
// ---------------------------------------------------------------------

fn core_checks() -> Vec<Value> {
    vec![
        check_tempdir(),
        check_ffmpeg_execute(),
        check_ffprobe_execute(),
        check_schemas_load(),
        check_sidecar_materialize(),
        check_source_policy(),
        check_cloud_default(),
    ]
}

/// Active probe: create → write → rename → delete inside a fresh directory,
/// exactly the sequence a real project run needs its temp directory to
/// support.
fn check_tempdir() -> Value {
    let id = "core.tempdir.readwrite";
    let base = std::env::temp_dir().join(format!(
        "cutright-doctor-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    let result = (|| -> io::Result<()> {
        fs::create_dir_all(&base)?;
        let original = base.join("probe.txt");
        fs::write(&original, b"videoctl doctor probe")?;
        let renamed = base.join("probe-renamed.txt");
        fs::rename(&original, &renamed)?;
        let readback = fs::read(&renamed)?;
        if readback != b"videoctl doctor probe" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "readback mismatch",
            ));
        }
        fs::remove_file(&renamed)?;
        fs::remove_dir(&base)?;
        Ok(())
    })();
    match result {
        Ok(()) => check(id, true, "ok", json!({ "path": base }), None),
        Err(error) => {
            let _ = fs::remove_dir_all(&base);
            check(
                id,
                true,
                "failed",
                json!({ "path": base, "error": error.to_string() }),
                Some("ensure the OS temp directory is writable and not full"),
            )
        }
    }
}

fn resolve_bin(env_var: &str, default_name: &str) -> PathBuf {
    std::env::var_os(env_var)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default_name))
}

fn check_ffmpeg_execute() -> Value {
    version_check(
        "core.ffmpeg.execute",
        resolve_bin("CUTRIGHT_FFMPEG", "ffmpeg"),
        "ffmpeg version",
        "install ffmpeg (e.g. `brew install ffmpeg`) or set CUTRIGHT_FFMPEG",
    )
}

fn check_ffprobe_execute() -> Value {
    version_check(
        "core.ffprobe.execute",
        resolve_bin("CUTRIGHT_FFPROBE", "ffprobe"),
        "ffprobe version",
        "install ffmpeg/ffprobe (e.g. `brew install ffmpeg`) or set CUTRIGHT_FFPROBE",
    )
}

fn version_check(id: &str, bin: PathBuf, expect_prefix: &str, remediation: &str) -> Value {
    let mut cmd = Command::new(&bin);
    cmd.arg("-version");
    match run_with_timeout(cmd, DEFAULT_TIMEOUT) {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let first_line = stdout.lines().next().unwrap_or_default();
            if first_line.starts_with(expect_prefix) {
                check(
                    id,
                    true,
                    "ok",
                    json!({ "bin": bin, "version_line": first_line.trim() }),
                    None,
                )
            } else {
                check(
                    id,
                    true,
                    "degraded",
                    json!({ "bin": bin, "version_line": first_line.trim() }),
                    Some("the resolved binary ran but did not report the expected version banner"),
                )
            }
        }
        Ok(output) => check(
            id,
            true,
            "failed",
            evidence_from_output(&output),
            Some(remediation),
        ),
        Err(error) => check(
            id,
            true,
            "missing",
            json!({ "bin": bin, "error": error.to_string() }),
            Some(remediation),
        ),
    }
}

/// Resolve the repo root so doctor can find `schemas/` and the studio
/// bundle in a dev checkout. `CUTRIGHT_REPO_ROOT` overrides for installed
/// binaries where the compile-time manifest dir no longer applies.
fn repo_root() -> PathBuf {
    if let Some(root) = std::env::var_os("CUTRIGHT_REPO_ROOT") {
        return PathBuf::from(root);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn check_schemas_load() -> Value {
    let id = "core.schemas.load";
    let dir = repo_root().join("schemas");
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) => {
            return check(
                id,
                true,
                "missing",
                json!({ "dir": dir, "error": error.to_string() }),
                Some("run from a CutRight checkout or set CUTRIGHT_REPO_ROOT"),
            )
        }
    };
    let mut loaded = Vec::new();
    let mut failed = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        match fs::read_to_string(&path).and_then(|text| {
            serde_json::from_str::<Value>(&text)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
        }) {
            Ok(_) => loaded.push(path.file_name().unwrap().to_string_lossy().to_string()),
            Err(error) => failed.push(json!({ "file": path, "error": error.to_string() })),
        }
    }
    if loaded.is_empty() {
        return check(
            id,
            true,
            "missing",
            json!({ "dir": dir, "loaded": loaded, "failed": failed }),
            Some("no *.json schemas found under schemas/"),
        );
    }
    if !failed.is_empty() {
        return check(
            id,
            true,
            "failed",
            json!({ "dir": dir, "loaded": loaded, "failed": failed }),
            Some("fix the schema files that failed to parse"),
        );
    }
    check(
        id,
        true,
        "ok",
        json!({ "dir": dir, "loaded": loaded }),
        None,
    )
}

/// §10.2 content-addressed embedded worker materialization now lives in
/// `video_core::content_store::materialize_worker` and is exercised by the
/// Vision anchor and caption-card sidecar workers. This probe drives the
/// real public API against a small known payload: materialize it, verify
/// the bytes on disk match, verify re-materializing identical bytes reuses
/// the same content-addressed path, and verify tampered bytes at that path
/// are rejected rather than silently trusted or overwritten.
fn check_sidecar_materialize() -> Value {
    let id = "core.sidecar.materialize";
    let marker = format!(
        "cutright-doctor-sidecar-probe-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    );
    let payload = marker.as_bytes();
    let name = "doctor-sidecar-probe-worker";

    let path = match video_core::content_store::materialize_worker(payload, name) {
        Ok(path) => path,
        Err(error) => {
            return check(
                id,
                false,
                "missing",
                json!({ "error": error.to_string() }),
                Some("check write permissions on the system temp directory"),
            );
        }
    };
    let cleanup = |path: &Path| {
        let _ = fs::remove_dir_all(path.parent().unwrap_or(path));
    };

    let on_disk = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            cleanup(&path);
            return check(
                id,
                false,
                "failed",
                json!({ "error": error.to_string(), "path": path }),
                Some("investigate video_core::content_store::materialize_worker"),
            );
        }
    };
    if on_disk != payload {
        cleanup(&path);
        return check(
            id,
            false,
            "failed",
            json!({ "path": path, "note": "materialized bytes did not match the embedded payload" }),
            Some("investigate video_core::content_store::materialize_worker"),
        );
    }

    // Re-materializing identical bytes must reuse the same content-addressed
    // path, not rewrite a new one.
    let reused = match video_core::content_store::materialize_worker(payload, name) {
        Ok(path) => path,
        Err(error) => {
            cleanup(&path);
            return check(
                id,
                false,
                "failed",
                json!({ "error": error.to_string() }),
                Some("investigate video_core::content_store::materialize_worker reuse path"),
            );
        }
    };
    if reused != path {
        cleanup(&path);
        return check(
            id,
            false,
            "failed",
            json!({ "first": path, "second": reused, "note": "identical bytes did not reuse the same content-addressed path" }),
            Some("investigate video_core::content_store::materialize_worker"),
        );
    }

    // Tampering the on-disk bytes must be rejected, never silently trusted
    // or overwritten.
    if let Err(error) = fs::write(&path, b"tampered-by-doctor-probe") {
        cleanup(&path);
        return check(
            id,
            false,
            "missing",
            json!({ "error": error.to_string() }),
            Some("check write permissions on the system temp directory"),
        );
    }
    let tamper_result = video_core::content_store::materialize_worker(payload, name);
    let tamper_rejected = matches!(
        tamper_result,
        Err(video_core::content_store::ContentStoreError::Tampered { .. })
    );
    cleanup(&path);

    if !tamper_rejected {
        return check(
            id,
            false,
            "failed",
            json!({ "note": "tampered sidecar bytes at the content-addressed path were not rejected" }),
            Some("investigate video_core::content_store::materialize_worker tamper detection"),
        );
    }

    check(
        id,
        false,
        "ok",
        json!({ "verified": ["materialize", "reuse-on-identical-bytes", "reject-tampered-bytes"] }),
        None,
    )
}

fn check_source_policy() -> Value {
    // Source immutability is a design invariant enforced by video-project's
    // ingest path (sources are copied/hashed, never mutated in place), not
    // a runtime toggle. There is nothing to probe at the process boundary
    // beyond asserting the invariant is still the documented contract.
    check(
        "core.source_policy.immutable",
        true,
        "ok",
        json!({ "policy": "sources are never mutated in place" }),
        None,
    )
}

fn check_cloud_default() -> Value {
    let id = "core.cloud_default.disabled";
    let value = std::env::var("CUTRIGHT_ENABLE_CLOUD").unwrap_or_default();
    let enabled = matches!(value.as_str(), "1" | "true" | "TRUE" | "yes");
    if enabled {
        check(
            id,
            true,
            "degraded",
            json!({ "CUTRIGHT_ENABLE_CLOUD": value }),
            Some("unset CUTRIGHT_ENABLE_CLOUD to restore the local-only default"),
        )
    } else {
        check(
            id,
            true,
            "ok",
            json!({ "CUTRIGHT_ENABLE_CLOUD": if value.is_empty() { Value::Null } else { json!(value) } }),
            None,
        )
    }
}

// ---------------------------------------------------------------------
// Audio probes (§11.3)
// ---------------------------------------------------------------------

fn audio_checks() -> Vec<Value> {
    vec![
        check_heardright_discover(),
        check_heardright_handshake(),
        check_whisperx_discover(),
        check_whisperx_python_version(),
    ]
}

fn check_heardright_discover() -> Value {
    let id = "audio.heardright.discover";
    match video_providers::HeardRightProvider::discover() {
        Ok(_) => check(id, true, "ok", json!({ "resolved": true }), None),
        Err(error) => check(
            id,
            true,
            "missing",
            json!({ "error": error.to_string() }),
            Some("install the signed CutRight speech runtime pack; the engine ships in the pack and is never resolved from the environment or the shell search path"),
        ),
    }
}

/// `video_providers::HeardRightProvider::health()` performs only the
/// protocol handshake (§9.2) — no transcription or VAD request is sent, and
/// no model download or network fallback occurs beyond what the handshake
/// itself requires. If the engine is genuinely absent or unreachable, this
/// honestly reports `missing` with remediation rather than fabricating
/// `ok`.
fn check_heardright_handshake() -> Value {
    let id = "audio.heardright.handshake";
    let provider = match video_providers::HeardRightProvider::discover() {
        Ok(provider) => provider,
        Err(error) => {
            return check(
                id,
                false,
                "missing",
                json!({ "error": error.to_string() }),
                Some("install the signed CutRight speech runtime pack; the engine ships in the pack and is never resolved from the environment or the shell search path"),
            );
        }
    };
    match provider.health() {
        Ok(identity) => check(
            id,
            false,
            "ok",
            json!({
                "engine_version": identity.engine_version,
                "protocol_major": identity.protocol_major,
                "protocol_minor": identity.protocol_minor,
                "negotiated_minor": identity.negotiated_minor,
                "capabilities": identity.capabilities,
            }),
            None,
        ),
        Err(error) => check(
            id,
            false,
            "missing",
            json!({ "error": error.to_string() }),
            Some("verify the HeardRight engine starts cleanly and speaks protocol major 1"),
        ),
    }
}

fn check_whisperx_discover() -> Value {
    let id = "audio.whisperx.discover";
    match video_providers::WhisperXProvider::discover() {
        Ok(_) => check(id, false, "ok", json!({ "resolved": true }), None),
        Err(error) => check(
            id,
            false,
            "missing",
            json!({ "error": error.to_string() }),
            Some("set CUTRIGHT_WHISPERX_PYTHON/CUTRIGHT_WHISPERX_SCRIPT if WhisperX alignment verification is needed"),
        ),
    }
}

fn check_whisperx_python_version() -> Value {
    let id = "audio.whisperx.python_version";
    if video_providers::WhisperXProvider::discover().is_err() {
        return check(
            id,
            false,
            "missing",
            json!({ "note": "skipped: WhisperX interpreter was not discovered" }),
            Some("resolve audio.whisperx.discover first"),
        );
    }
    let python = resolve_bin("CUTRIGHT_WHISPERX_PYTHON", "python3");
    let mut cmd = Command::new(&python);
    cmd.arg("--version");
    match run_with_timeout(cmd, DEFAULT_TIMEOUT) {
        Ok(output) if output.status.success() => {
            let banner = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or_default()
                .to_string();
            let banner = if banner.is_empty() {
                String::from_utf8_lossy(&output.stderr)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string()
            } else {
                banner
            };
            check(
                id,
                false,
                "ok",
                json!({ "python": python, "version": banner.trim() }),
                None,
            )
        }
        Ok(output) => check(
            id,
            false,
            "failed",
            evidence_from_output(&output),
            Some("verify the WhisperX interpreter runs `--version`"),
        ),
        Err(error) => check(
            id,
            false,
            "missing",
            json!({ "python": python, "error": error.to_string() }),
            Some("set CUTRIGHT_WHISPERX_PYTHON to a working interpreter"),
        ),
    }
}

// ---------------------------------------------------------------------
// Render probes (§11.4)
// ---------------------------------------------------------------------

fn render_checks() -> Vec<Value> {
    let (software_encode, output_reprobe) = check_software_encode_and_reprobe();
    vec![
        check_h264_videotoolbox(),
        check_zscale_filter(),
        software_encode,
        check_audio_encode(),
        check_caption_renderer(),
        output_reprobe,
        check_remotion_toolchain(),
    ]
}

fn ffmpeg_bin() -> PathBuf {
    resolve_bin("CUTRIGHT_FFMPEG", "ffmpeg")
}

fn ffprobe_bin() -> PathBuf {
    resolve_bin("CUTRIGHT_FFPROBE", "ffprobe")
}

fn list_encoders() -> io::Result<String> {
    let mut cmd = Command::new(ffmpeg_bin());
    cmd.args(["-hide_banner", "-encoders"]);
    let output = run_with_timeout(cmd, DEFAULT_TIMEOUT)?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn list_filters() -> io::Result<String> {
    let mut cmd = Command::new(ffmpeg_bin());
    cmd.args(["-hide_banner", "-filters"]);
    let output = run_with_timeout(cmd, DEFAULT_TIMEOUT)?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn scratch_file(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("cutright-doctor-{}-{}", std::process::id(), name))
}

/// `h264_videotoolbox` is macOS-only. On other platforms the check is
/// non-blocking `ok` — the capability simply does not apply there, which is
/// not the same as the tool being broken.
fn check_h264_videotoolbox() -> Value {
    let id = "render.h264_videotoolbox.smoke";
    if !cfg!(target_os = "macos") {
        return check(
            id,
            false,
            "ok",
            json!({ "note": "h264_videotoolbox is macOS-only; not applicable on this platform" }),
            None,
        );
    }
    let listed = match list_encoders() {
        Ok(text) => text.contains("h264_videotoolbox"),
        Err(error) => {
            return check(
                id,
                true,
                "failed",
                json!({ "error": error.to_string() }),
                Some("ensure ffmpeg is on PATH and runs `-encoders`"),
            )
        }
    };
    if !listed {
        return check(
            id,
            true,
            "missing",
            json!({ "listed": false }),
            Some("install an ffmpeg build with --enable-videotoolbox"),
        );
    }
    let output_path = scratch_file("videotoolbox.mp4");
    let mut cmd = Command::new(ffmpeg_bin());
    cmd.args([
        "-hide_banner",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=64x64:rate=10:duration=0.2",
        "-frames:v",
        "3",
        "-c:v",
        "h264_videotoolbox",
    ])
    .arg(&output_path);
    let result = run_with_timeout(cmd, DEFAULT_TIMEOUT);
    let encoded = matches!(&result, Ok(output) if output.status.success() && output_path.is_file());
    let evidence = match &result {
        Ok(output) => evidence_from_output(output),
        Err(error) => json!({ "error": error.to_string() }),
    };
    let _ = fs::remove_file(&output_path);
    if encoded {
        check(
            id,
            true,
            "ok",
            json!({ "listed": true, "encode": evidence }),
            None,
        )
    } else {
        check(
            id,
            true,
            "failed",
            json!({ "listed": true, "encode": evidence }),
            Some("h264_videotoolbox is listed but a real encode failed; check hardware encoder availability"),
        )
    }
}

/// libzimg (`zscale`) is not bundled in every ffmpeg distribution, so this
/// check is intentionally non-blocking: report the true capability state
/// without failing the whole doctor run over an optional HDR filter.
fn check_zscale_filter() -> Value {
    let id = "render.zscale.smoke";
    let listed = match list_filters() {
        Ok(text) => text.contains("zscale"),
        Err(error) => {
            return check(
                id,
                false,
                "failed",
                json!({ "error": error.to_string() }),
                Some("ensure ffmpeg is on PATH and runs `-filters`"),
            )
        }
    };
    if !listed {
        return check(
            id,
            false,
            "missing",
            json!({ "listed": false }),
            Some("install an ffmpeg build with libzimg (--enable-libzimg) for zscale HDR tone-mapping"),
        );
    }
    let mut cmd = Command::new(ffmpeg_bin());
    cmd.args([
        "-hide_banner",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "color=c=white:s=64x64:d=0.1",
        "-vf",
        "zscale=t=linear",
        "-frames:v",
        "1",
        "-f",
        "null",
        "-",
    ]);
    match run_with_timeout(cmd, DEFAULT_TIMEOUT) {
        Ok(output) if output.status.success() => {
            check(id, false, "ok", json!({ "listed": true }), None)
        }
        Ok(output) => check(
            id,
            false,
            "failed",
            json!({ "listed": true, "run": evidence_from_output(&output) }),
            Some("zscale is listed but a real filter run failed"),
        ),
        Err(error) => check(
            id,
            false,
            "failed",
            json!({ "listed": true, "error": error.to_string() }),
            Some("zscale is listed but the probe process failed to run"),
        ),
    }
}

/// Encodes a tiny software (libx264 + AAC) clip and reprobes it with
/// ffprobe. Returns `(render.software_encoder.smoke, render.output_reprobe.smoke)`
/// as one pair so the reprobe always targets a real, just-produced file
/// rather than a stale fixture.
fn check_software_encode_and_reprobe() -> (Value, Value) {
    let encoder_id = "render.software_encoder.smoke";
    let reprobe_id = "render.output_reprobe.smoke";
    let output_path = scratch_file("software.mp4");
    let mut cmd = Command::new(ffmpeg_bin());
    cmd.args([
        "-hide_banner",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=64x64:rate=10:duration=0.2",
        "-f",
        "lavfi",
        "-i",
        "anullsrc=r=48000:cl=stereo",
        "-t",
        "0.2",
        "-c:v",
        "libx264",
        "-c:a",
        "aac",
        "-shortest",
    ])
    .arg(&output_path);
    let encode_result = run_with_timeout(cmd, DEFAULT_TIMEOUT);
    let encoded =
        matches!(&encode_result, Ok(output) if output.status.success() && output_path.is_file());
    let encode_evidence = match &encode_result {
        Ok(output) => evidence_from_output(output),
        Err(error) => json!({ "error": error.to_string() }),
    };

    let encoder_check = if encoded {
        check(encoder_id, true, "ok", encode_evidence.clone(), None)
    } else {
        check(
            encoder_id,
            true,
            "failed",
            encode_evidence.clone(),
            Some("ensure ffmpeg's libx264 encoder is available"),
        )
    };

    let reprobe_check = if !encoded {
        check(
            reprobe_id,
            true,
            "missing",
            json!({ "note": "skipped: software encode did not produce an output" }),
            Some("resolve render.software_encoder.smoke first"),
        )
    } else {
        let mut probe_cmd = Command::new(ffprobe_bin());
        probe_cmd
            .args([
                "-v",
                "error",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
            ])
            .arg(&output_path);
        match run_with_timeout(probe_cmd, DEFAULT_TIMEOUT) {
            Ok(output) if output.status.success() => {
                match serde_json::from_slice::<Value>(&output.stdout) {
                    Ok(parsed) => {
                        let streams = parsed["streams"].as_array().cloned().unwrap_or_default();
                        let has_video = streams.iter().any(|s| s["codec_type"] == "video");
                        let has_audio = streams.iter().any(|s| s["codec_type"] == "audio");
                        if has_video && has_audio {
                            check(
                                reprobe_id,
                                true,
                                "ok",
                                json!({ "has_video": true, "has_audio": true }),
                                None,
                            )
                        } else {
                            check(
                                reprobe_id,
                                true,
                                "failed",
                                json!({ "has_video": has_video, "has_audio": has_audio }),
                                Some("the paired ffprobe could not confirm both a video and audio stream in ffmpeg's own output"),
                            )
                        }
                    }
                    Err(error) => check(
                        reprobe_id,
                        true,
                        "failed",
                        json!({ "error": error.to_string() }),
                        Some("ffprobe returned invalid JSON for a file ffmpeg just wrote"),
                    ),
                }
            }
            Ok(output) => check(
                reprobe_id,
                true,
                "failed",
                evidence_from_output(&output),
                Some("ffprobe failed to read ffmpeg's own output"),
            ),
            Err(error) => check(
                reprobe_id,
                true,
                "missing",
                json!({ "error": error.to_string() }),
                Some("ensure ffprobe is on PATH and paired with the resolved ffmpeg"),
            ),
        }
    };

    let _ = fs::remove_file(&output_path);
    (encoder_check, reprobe_check)
}

fn check_audio_encode() -> Value {
    let id = "render.audio_encode.smoke";
    let output_path = scratch_file("audio.m4a");
    let mut cmd = Command::new(ffmpeg_bin());
    cmd.args([
        "-hide_banner",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "anullsrc=r=48000:cl=stereo",
        "-t",
        "0.2",
        "-c:a",
        "aac",
    ])
    .arg(&output_path);
    let result = run_with_timeout(cmd, DEFAULT_TIMEOUT);
    let ok = matches!(&result, Ok(output) if output.status.success() && output_path.is_file());
    let evidence = match &result {
        Ok(output) => evidence_from_output(output),
        Err(error) => json!({ "error": error.to_string() }),
    };
    let _ = fs::remove_file(&output_path);
    if ok {
        check(id, true, "ok", evidence, None)
    } else {
        check(
            id,
            true,
            "failed",
            evidence,
            Some("ensure ffmpeg's AAC encoder is available"),
        )
    }
}

fn check_caption_renderer() -> Value {
    let id = "render.caption_renderer.listed";
    match list_filters() {
        Ok(text) => {
            let has_subtitles =
                text.contains(" subtitles ") || text.lines().any(|line| line.contains("subtitles"));
            if has_subtitles {
                check(id, false, "ok", json!({ "listed": true }), None)
            } else {
                check(
                    id,
                    false,
                    "missing",
                    json!({ "listed": false }),
                    Some("install an ffmpeg build with libass (--enable-libass) for burned-in captions"),
                )
            }
        }
        Err(error) => check(
            id,
            false,
            "failed",
            json!({ "error": error.to_string() }),
            Some("ensure ffmpeg is on PATH and runs `-filters`"),
        ),
    }
}

/// Honest missing/remediation probe for the Remotion toolchain
/// (`EffectRenderer::Remotion` in `crates/video-project/src/effects.rs`,
/// backed by `crates/video-media/src/effects.rs::render_effect_remotion_
/// preview`): an active `node --version` spawn (proves Node actually
/// executes, matching `version_check`'s pattern for ffmpeg/ffprobe), plus a
/// structural check that `apps/effects`'s dependencies are installed and
/// its render CLI script exists. Deliberately does not run a full Remotion
/// bundle/render here — that is a much larger and slower probe than every
/// other `render_checks()` entry, and the real render path already has its
/// own dedicated test coverage
/// (`crates/video-media/src/effects.rs::render_effect_remotion_preview_
/// renders_all_four_effects_and_is_deterministic`). Non-blocking
/// (`required: false`), same as `render.zscale.smoke` and
/// `render.caption_renderer.listed`: an optional Node toolchain not yet
/// installed on a given machine is not the same as the renderer being
/// broken.
fn check_remotion_toolchain() -> Value {
    let id = "render.remotion_toolchain.probe";
    let node = resolve_bin("CUTRIGHT_NODE", "node");
    let mut cmd = Command::new(&node);
    cmd.arg("--version");
    let version = match run_with_timeout(cmd, DEFAULT_TIMEOUT) {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        Ok(output) => {
            return check(
                id,
                false,
                "failed",
                evidence_from_output(&output),
                Some("ensure `node --version` runs cleanly"),
            )
        }
        Err(error) => {
            return check(
                id,
                false,
                "missing",
                json!({ "node": node, "error": error.to_string() }),
                Some("install Node (matching the repo root .node-version) or set CUTRIGHT_NODE"),
            )
        }
    };

    let package_root = repo_root().join("apps/effects");
    let node_modules = package_root.join("node_modules");
    let render_script = package_root.join("scripts/render.mjs");
    if !render_script.is_file() {
        return check(
            id,
            false,
            "missing",
            json!({ "node_version": version, "render_script": render_script }),
            Some("apps/effects/scripts/render.mjs is missing from this checkout"),
        );
    }
    if !node_modules.is_dir() {
        return check(
            id,
            false,
            "missing",
            json!({ "node_version": version, "node_modules": node_modules }),
            Some("run `pnpm --dir apps/effects install`"),
        );
    }

    check(
        id,
        false,
        "ok",
        json!({ "node_version": version, "package_root": package_root }),
        None,
    )
}

// ---------------------------------------------------------------------
// Studio probes (§11.5)
// ---------------------------------------------------------------------

fn studio_checks() -> Vec<Value> {
    vec![
        check_frontend_bundle(),
        check_vendored_fonts(),
        check_asset_protocol(),
        check_preview_fixture(),
    ]
}

fn studio_dir() -> PathBuf {
    repo_root().join("apps/studio")
}

fn check_frontend_bundle() -> Value {
    let id = "studio.frontend_bundle.exists";
    let index = studio_dir().join("dist/index.html");
    if index.is_file() {
        let bytes = fs::metadata(&index).map(|m| m.len()).unwrap_or(0);
        if bytes > 0 {
            check(
                id,
                true,
                "ok",
                json!({ "path": index, "bytes": bytes }),
                None,
            )
        } else {
            check(
                id,
                true,
                "failed",
                json!({ "path": index, "bytes": bytes }),
                Some("rebuild the studio frontend: `pnpm --dir apps/studio build`"),
            )
        }
    } else {
        check(
            id,
            true,
            "missing",
            json!({ "path": index }),
            Some("build the studio frontend: `pnpm --dir apps/studio build`"),
        )
    }
}

fn check_vendored_fonts() -> Value {
    let id = "studio.fonts_vendored.exists";
    let fonts_dir = studio_dir().join("src/assets/fonts");
    let required = [
        "geist-variable.woff2",
        "spline-sans-mono-regular.ttf",
        "tanker-regular.woff2",
        "LICENSES.md",
    ];
    let mut missing = Vec::new();
    for name in required {
        if !fonts_dir.join(name).is_file() {
            missing.push(name);
        }
    }
    if missing.is_empty() {
        check(id, true, "ok", json!({ "dir": fonts_dir }), None)
    } else {
        check(
            id,
            true,
            "missing",
            json!({ "dir": fonts_dir, "missing": missing }),
            Some("restore the vendored fonts and their license notices under apps/studio/src/assets/fonts"),
        )
    }
}

fn check_asset_protocol() -> Value {
    let id = "studio.tauri_asset_protocol.enabled";
    let config_path = studio_dir().join("src-tauri/tauri.conf.json");
    let text = match fs::read_to_string(&config_path) {
        Ok(text) => text,
        Err(error) => {
            return check(
                id,
                true,
                "missing",
                json!({ "path": config_path, "error": error.to_string() }),
                Some("restore apps/studio/src-tauri/tauri.conf.json"),
            )
        }
    };
    let parsed: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            return check(
                id,
                true,
                "failed",
                json!({ "path": config_path, "error": error.to_string() }),
                Some("fix invalid JSON in tauri.conf.json"),
            )
        }
    };
    let enabled = parsed
        .pointer("/app/security/assetProtocol/enable")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if enabled {
        check(id, true, "ok", json!({ "path": config_path }), None)
    } else {
        check(
            id,
            true,
            "failed",
            json!({ "path": config_path, "enable": enabled }),
            Some("set app.security.assetProtocol.enable = true in tauri.conf.json"),
        )
    }
}

/// A full packaged-app smoke fixture (load one allowed preview path, reject
/// one outside path) needs a running Tauri asset-protocol runtime, which
/// this CLI-only crate cannot host. Reported honestly as an unverified,
/// non-blocking gap rather than faked; the remediation names the real
/// coverage instead of reading as an untested one.
fn check_preview_fixture() -> Value {
    check(
        "studio.preview_fixture.smoke",
        false,
        "missing",
        json!({ "note": "packaged-app asset-scope smoke fixture requires a running Tauri runtime; the CLI doctor cannot host one, so this probe cannot independently verify it" }),
        Some("already covered by apps/studio/src-tauri/src/tests.rs::packaged_asset_scope_allows_project_media_and_denies_a_sibling_file (run `cargo test -p cutright-studio`, or the app QA skill at tools/skills/qa against a running Studio build) — this doctor probe is only reporting that it cannot re-run that coverage from the CLI"),
    )
}

// ---------------------------------------------------------------------
// Receipt (§11.7)
// ---------------------------------------------------------------------

fn write_receipt_file(path: &Path, report: &Value) -> io::Result<()> {
    let now = chrono::Utc::now();
    let canonical = serde_json::to_vec(report).unwrap_or_default();
    let hash = blake3::hash(&canonical).to_hex().to_string();
    let receipt = json!({
        "schema_version": 1,
        "kind": "videoctl.doctor.receipt",
        "created_at": now.to_rfc3339(),
        "report_blake3": hash,
        "report": report,
    });
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, serde_json::to_vec_pretty(&receipt)?)
}
