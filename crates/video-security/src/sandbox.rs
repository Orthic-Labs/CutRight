//! Sandboxed worker execution.
//!
//! The dispatcher constructs a [`WorkerGrant`] describing exactly which
//! paths a worker may read, which directory it may write, which environment
//! variables are allowed, which limits apply and whether the worker is
//! permitted to use the network. The harness on the target owns the
//! actual enforcement; this module only validates that the requested
//! grant obeys the v2 release policy.
//!
//! The acceptance criteria:
//!  * Path escape, process escape and decompression-bomb fixtures must
//!    produce [`SandboxError::PathOutsideScope`] /
//!    [`SandboxError::PathTraversesScope`] / [`SandboxError::BombDetected`].
//!  * A worker cannot read outside the granted files/packs/temp scope.
//!  * Targets without a required sandbox guarantee must report
//!    [`SandboxError::Unsupported`] and never claim supported.

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkPolicy {
    Denied,
    Allowed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub wall_clock_seconds: u32,
    pub max_output_bytes: u64,
    pub max_temp_bytes: u64,
    pub max_decompressed_ratio: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            wall_clock_seconds: 600,
            max_output_bytes: 8 * 1024 * 1024 * 1024,
            max_temp_bytes: 16 * 1024 * 1024 * 1024,
            max_decompressed_ratio: 64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerGrant {
    pub worker_id: String,
    pub executable_hash: String,
    pub readable_files: Vec<PathBuf>,
    pub writable_dir: PathBuf,
    pub env_allowlist: Vec<String>,
    pub limits: ResourceLimits,
    pub network: NetworkPolicy,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SandboxError {
    #[error("requested path {0:?} is outside the granted scope")]
    PathOutsideScope(PathBuf),
    #[error("path {0:?} traverses outside the granted scope (parent escape)")]
    PathTraversesScope(PathBuf),
    #[error("path {0:?} contains a symlink that points outside scope")]
    SymlinkEscape(PathBuf),
    #[error("requested path is absolute {0:?} but no root scope provided")]
    AbsoluteWithoutRoot(PathBuf),
    #[error("network access is denied by release policy")]
    NetworkDenied,
    #[error("requested limit {field} value {value} exceeds policy")]
    LimitTooHigh { field: String, value: u64 },
    #[error("decompression ratio {0} exceeds policy floor {1}")]
    BombDetected(u32, u32),
    #[error("sandbox is unsupported on this target")]
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerRole {
    MediaDecoder,
    Model,
    Helper,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantRequest {
    pub worker_id: String,
    pub role: WorkerRole,
    pub executable_hash: String,
    pub readable_paths: Vec<PathBuf>,
    pub writable_dir: PathBuf,
    pub env_allowlist: Vec<String>,
    pub limits: ResourceLimits,
}

/// Validate a grant request against the policy floor.
///
/// Returns the [`WorkerGrant`] on success or a typed [`SandboxError`] on
/// failure. `root_scope` pins the working directory for relative paths.
pub fn validate_grant(
    request: &GrantRequest,
    root_scope: &Path,
    network: NetworkPolicy,
) -> Result<WorkerGrant, SandboxError> {
    for path in &request.readable_paths {
        check_readable(path, root_scope)?;
    }
    check_writable(&request.writable_dir, root_scope)?;
    validate_limits(&request.limits)?;
    Ok(WorkerGrant {
        worker_id: request.worker_id.clone(),
        executable_hash: request.executable_hash.clone(),
        readable_files: request.readable_paths.clone(),
        writable_dir: request.writable_dir.clone(),
        env_allowlist: request.env_allowlist.clone(),
        limits: request.limits.clone(),
        network,
    })
}

fn validate_limits(limits: &ResourceLimits) -> Result<(), SandboxError> {
    let floor = ResourceLimits::default();
    if u64::from(limits.max_decompressed_ratio) > u64::from(floor.max_decompressed_ratio) {
        return Err(SandboxError::LimitTooHigh {
            field: "max_decompressed_ratio".to_string(),
            value: u64::from(limits.max_decompressed_ratio),
        });
    }
    Ok(())
}

fn check_readable(path: &Path, root_scope: &Path) -> Result<(), SandboxError> {
    if path.is_absolute() && root_scope.as_os_str().is_empty() {
        return Err(SandboxError::AbsoluteWithoutRoot(path.to_path_buf()));
    }
    for c in path.components() {
        match c {
            Component::ParentDir => {
                return Err(SandboxError::PathTraversesScope(path.to_path_buf()));
            }
            Component::Prefix(_) if root_scope.as_os_str().is_empty() => {
                return Err(SandboxError::PathOutsideScope(path.to_path_buf()));
            }
            _ => {}
        }
    }
    Ok(())
}

fn check_writable(path: &Path, root_scope: &Path) -> Result<(), SandboxError> {
    check_readable(path, root_scope)
}

/// Classify an observed decompressed-to-compressed ratio against the
/// release policy floor.
pub fn classify_decompression(
    compressed_bytes: u64,
    decompressed_bytes: u64,
    limits: &ResourceLimits,
) -> Result<(), SandboxError> {
    if compressed_bytes == 0 {
        return Ok(());
    }
    let ratio = (decompressed_bytes / compressed_bytes) as u32;
    if ratio > limits.max_decompressed_ratio {
        Err(SandboxError::BombDetected(
            ratio,
            limits.max_decompressed_ratio,
        ))
    } else {
        Ok(())
    }
}

/// Returns true when the target supports a v2 sandbox guarantee. The
/// harness passes the platform-specific claim; the result is propagated to
/// release claims so that unsupported targets are excluded.
pub fn target_supports_sandbox(target_claim: bool) -> Result<(), SandboxError> {
    if target_claim {
        Ok(())
    } else {
        Err(SandboxError::Unsupported)
    }
}

/// Build a default grant for a media-decoder worker with hard-coded
/// network denial. The harness still has to enforce the grant locally.
pub fn media_decoder_grant(
    worker_id: &str,
    executable_hash: &str,
    readable_paths: Vec<PathBuf>,
    writable_dir: PathBuf,
) -> WorkerGrant {
    WorkerGrant {
        worker_id: worker_id.to_string(),
        executable_hash: executable_hash.to_string(),
        readable_files: readable_paths,
        writable_dir,
        env_allowlist: vec!["PATH".to_string()],
        limits: ResourceLimits::default(),
        network: NetworkPolicy::Denied,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        PathBuf::from("/var/folders/cutright/scope")
    }

    #[test]
    fn parent_dir_traversal_is_rejected() {
        let req = GrantRequest {
            worker_id: "w".into(),
            role: WorkerRole::MediaDecoder,
            executable_hash: "abcd".into(),
            readable_paths: vec![PathBuf::from("../../../etc/passwd")],
            writable_dir: PathBuf::from("out"),
            env_allowlist: vec!["PATH".into()],
            limits: ResourceLimits::default(),
        };
        let err = validate_grant(&req, &root(), NetworkPolicy::Denied).unwrap_err();
        assert!(matches!(err, SandboxError::PathTraversesScope(_)));
    }

    #[test]
    fn absolute_path_without_root_is_rejected() {
        let req = GrantRequest {
            worker_id: "w".into(),
            role: WorkerRole::Model,
            executable_hash: "abcd".into(),
            readable_paths: vec![PathBuf::from("/etc/passwd")],
            writable_dir: PathBuf::from("out"),
            env_allowlist: vec!["PATH".into()],
            limits: ResourceLimits::default(),
        };
        let err = validate_grant(&req, &PathBuf::new(), NetworkPolicy::Denied).unwrap_err();
        assert!(matches!(err, SandboxError::AbsoluteWithoutRoot(_)));
    }

    #[test]
    fn network_is_denied_by_default_for_media_workers() {
        let g = media_decoder_grant(
            "decoder.0",
            "abcd",
            vec![PathBuf::from("source.mp4")],
            PathBuf::from("out"),
        );
        assert_eq!(g.network, NetworkPolicy::Denied);
    }

    #[test]
    fn high_decompression_ratio_is_a_bomb() {
        let limits = ResourceLimits::default();
        let err = classify_decompression(10, 10_000, &limits).unwrap_err();
        assert!(matches!(err, SandboxError::BombDetected(1000, 64)));
    }

    #[test]
    fn unsupported_target_is_unsupported() {
        let err = target_supports_sandbox(false).unwrap_err();
        assert_eq!(err, SandboxError::Unsupported);
    }
}
