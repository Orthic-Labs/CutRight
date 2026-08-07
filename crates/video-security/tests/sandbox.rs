//! Sandbox and media-limit integration tests.
//!
//! Verifies the fix-failure surface required by B7-012:
//! * Path escape, parent escape and decompression-bomb fixtures fail.
//! * Workers cannot read outside granted scopes.
//! * Unsupported targets are not claimed supported.

use std::path::PathBuf;

use video_security::media_limits::{validate_media, MediaLimits, MediaMetadata};
use video_security::sandbox::{
    classify_decompression, media_decoder_grant, target_supports_sandbox, validate_grant,
    GrantRequest, NetworkPolicy, ResourceLimits, WorkerGrant, WorkerRole,
};

fn root() -> PathBuf {
    PathBuf::from("/tmp/cutright-scope")
}

fn small_limits() -> ResourceLimits {
    ResourceLimits {
        wall_clock_seconds: 30,
        max_output_bytes: 1024,
        max_temp_bytes: 1024,
        max_decompressed_ratio: 16,
    }
}

#[test]
fn path_escape_outside_root_fails() {
    let req = GrantRequest {
        worker_id: "worker.0".to_string(),
        role: WorkerRole::Model,
        executable_hash: "abcd".to_string(),
        readable_paths: vec![PathBuf::from("/etc/passwd")],
        writable_dir: PathBuf::from("out"),
        env_allowlist: vec!["PATH".to_string()],
        limits: small_limits(),
    };
    let r = validate_grant(&req, &PathBuf::new(), NetworkPolicy::Denied);
    assert!(r.is_err());
}

#[test]
fn parent_dir_traversal_is_a_path_traverses_scope() {
    let req = GrantRequest {
        worker_id: "worker.0".to_string(),
        role: WorkerRole::Helper,
        executable_hash: "abcd".to_string(),
        readable_paths: vec![PathBuf::from("../escape.txt")],
        writable_dir: PathBuf::from("out"),
        env_allowlist: vec!["PATH".to_string()],
        limits: small_limits(),
    };
    let err = validate_grant(&req, &root(), NetworkPolicy::Denied).unwrap_err();
    assert!(format!("{}", err).contains("traverses"));
}

#[test]
fn decompression_bomb_is_detected() {
    let err = classify_decompression(1, 10_000, &ResourceLimits::default()).unwrap_err();
    assert!(format!("{}", err).contains("decompression"));
}

#[test]
fn media_limits_decompression_ratio_is_a_bomb() {
    let limits = MediaLimits::default();
    let m = MediaMetadata {
        width: 16,
        height: 16,
        duration_ms: 100,
        stream_count: 1,
        compressed_bytes: 16,
        decompressed_bytes: 16 * 1024 * 1024,
        metadata_size_bytes: 16,
    };
    let err = validate_media(&m, &limits).unwrap_err();
    assert!(format!("{}", err).contains("ratio"));
}

#[test]
fn worker_cannot_read_outside_scope_when_rooted() {
    let req = GrantRequest {
        worker_id: "worker.0".to_string(),
        role: WorkerRole::MediaDecoder,
        executable_hash: "abcd".to_string(),
        readable_paths: vec![PathBuf::from("/../escape.txt")],
        writable_dir: PathBuf::from("out"),
        env_allowlist: vec!["PATH".to_string()],
        limits: small_limits(),
    };
    let r = validate_grant(&req, &root(), NetworkPolicy::Denied);
    assert!(r.is_err());
}

#[test]
fn network_must_be_denied_by_default_for_media() {
    let g: WorkerGrant = media_decoder_grant(
        "decoder.0",
        "abcd",
        vec![PathBuf::from("source.mp4")],
        PathBuf::from("out"),
    );
    assert!(matches!(g.network, NetworkPolicy::Denied));
}

#[test]
fn unsupported_target_is_unsupported() {
    let err = target_supports_sandbox(false).unwrap_err();
    assert!(format!("{}", err).contains("unsupported"));
}

#[test]
fn valid_request_is_accepted() {
    let req = GrantRequest {
        worker_id: "decoder.0".to_string(),
        role: WorkerRole::MediaDecoder,
        executable_hash: "abcd".to_string(),
        readable_paths: vec![PathBuf::from("source.mp4")],
        writable_dir: PathBuf::from("out"),
        env_allowlist: vec!["PATH".to_string()],
        limits: small_limits(),
    };
    let grant = validate_grant(&req, &root(), NetworkPolicy::Denied).expect("grant");
    assert_eq!(grant.worker_id, "decoder.0");
}
