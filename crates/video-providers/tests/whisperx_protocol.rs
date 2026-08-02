//! Black-box process-boundary tests for the WhisperX provider (hardening
//! plan §10.9): nonzero exit, invalid response, and temp-file cleanup, all
//! driven through the crate's real public API (`WhisperXProvider::discover`
//! and `TranscriptionProvider::transcribe`) against a fake "python"
//! executable — never a real WhisperX install.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use video_core::providers::{
    ProviderError as CoreProviderError, TranscriptionProvider, TranscriptionRequest,
};
use video_providers::WhisperXProvider;

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

/// `WhisperXProvider` invokes `python <script> <source> <output>` as one
/// command. For these tests the fake "python" is itself the whole behavior;
/// the "script" only needs to exist to satisfy discovery.
fn set_fake_whisperx(root: &std::path::Path, python_body: &str) -> PathBuf {
    let python = root.join("fake-python");
    fs::write(&python, format!("#!/bin/sh\n{python_body}\n")).expect("write fake python");
    fs::set_permissions(&python, fs::Permissions::from_mode(0o700))
        .expect("make fake python executable");
    let script = root.join("fake-script.py");
    fs::write(&script, "# unused by the fake interpreter\n").expect("write fake script");
    std::env::set_var("CUTRIGHT_WHISPERX_PYTHON", &python);
    std::env::set_var("CUTRIGHT_WHISPERX_SCRIPT", &script);
    std::env::set_var("CUTRIGHT_WHISPERX_TIMEOUT_SECS", "10");
    script
}

fn clear_env() {
    std::env::remove_var("CUTRIGHT_WHISPERX_PYTHON");
    std::env::remove_var("CUTRIGHT_WHISPERX_SCRIPT");
    std::env::remove_var("CUTRIGHT_WHISPERX_TIMEOUT_SECS");
}

fn transcription_request(source_path: PathBuf) -> TranscriptionRequest {
    TranscriptionRequest {
        source_id: "source-a".into(),
        source_path,
        language_hint: None,
    }
}

/// Every temp file this crate's WhisperX path creates matches this prefix
/// (`whisperx.rs::TempFileGuard::new`). Used to prove no leftover file
/// remains after a call, success or failure.
fn leftover_whisperx_temp_files() -> Vec<PathBuf> {
    let dir = std::env::temp_dir();
    let mut leftovers = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("cutright-whisperx-") {
                    leftovers.push(entry.path());
                }
            }
        }
    }
    leftovers
}

#[test]
fn nonzero_exit_surfaces_stderr_as_a_rejected_error() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = unique_temp_dir("cutright-wx-nonzero");
    set_fake_whisperx(&root, "echo 'alignment blew up' 1>&2\nexit 1");

    let provider = WhisperXProvider::discover().expect("discover fake whisperx");
    let error = provider
        .transcribe(&transcription_request(root.join("source.wav")))
        .expect_err("nonzero exit must be rejected");
    match error {
        CoreProviderError::Rejected { reason, .. } => {
            assert!(reason.contains("alignment blew up"), "reason: {reason}");
            assert!(reason.contains("exit_code"), "reason: {reason}");
        }
        other => panic!("expected Rejected, got {other:?}"),
    }
    assert!(
        leftover_whisperx_temp_files().is_empty(),
        "temp output file must be cleaned up after a nonzero exit"
    );

    clear_env();
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn invalid_json_response_is_rejected() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = unique_temp_dir("cutright-wx-invalid-json");
    // $3 is the output path WhisperXProvider asks the interpreter to write
    // ($1 is the script path, $2 the source path).
    set_fake_whisperx(&root, "printf 'not json at all' > \"$3\"\nexit 0");

    let provider = WhisperXProvider::discover().expect("discover fake whisperx");
    let error = provider
        .transcribe(&transcription_request(root.join("source.wav")))
        .expect_err("invalid JSON output must be rejected");
    match error {
        CoreProviderError::Rejected { reason, .. } => {
            assert!(
                reason.contains("invalid WhisperX response"),
                "reason: {reason}"
            );
        }
        other => panic!("expected Rejected, got {other:?}"),
    }
    assert!(
        leftover_whisperx_temp_files().is_empty(),
        "temp output file must be cleaned up after an invalid response"
    );

    clear_env();
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn invalid_word_shape_is_rejected() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = unique_temp_dir("cutright-wx-invalid-words");
    // Valid JSON, but not the expected [{s,e,w}, ...] word list shape.
    set_fake_whisperx(&root, "printf '{\"not\":\"a word list\"}' > \"$3\"\nexit 0");

    let provider = WhisperXProvider::discover().expect("discover fake whisperx");
    let error = provider
        .transcribe(&transcription_request(root.join("source.wav")))
        .expect_err("wrong response shape must be rejected");
    match error {
        CoreProviderError::Rejected { reason, .. } => {
            assert!(
                reason.contains("invalid WhisperX word response"),
                "reason: {reason}"
            );
        }
        other => panic!("expected Rejected, got {other:?}"),
    }

    clear_env();
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn temp_output_file_is_removed_after_a_successful_transcription() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = unique_temp_dir("cutright-wx-success-cleanup");
    set_fake_whisperx(
        &root,
        "printf '[{\"s\":0.0,\"e\":0.5,\"w\":\"hello\"}]' > \"$3\"\nexit 0",
    );

    let provider = WhisperXProvider::discover().expect("discover fake whisperx");
    let output = provider
        .transcribe(&transcription_request(root.join("source.wav")))
        .expect("fake whisperx succeeds");
    assert_eq!(output.transcript.words.len(), 1);
    assert_eq!(output.transcript.words[0].text, "hello");
    assert!(
        leftover_whisperx_temp_files().is_empty(),
        "temp output file must be cleaned up even after success"
    );

    clear_env();
    fs::remove_dir_all(root).expect("cleanup");
}
