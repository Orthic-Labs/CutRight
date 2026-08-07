//! CutRight-owned adapter around the vendored HeardRight crates.
//!
//! This module removes every HeardRight install/path/environment-variable
//! discovery; all resources resolve through the injected
//! `PackResourceResolver`. The vendored commit and pack hashes are
//! recorded in the cap-ledger and surfaced through the adapter identity.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

/// Opaque pack identifier (mirrors `runtime/manifests/*.source.json`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackId(pub String);

/// Opaque resource identifier within a pack.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceId(pub String);

/// A verified resource: bytes are hash-pinned and the licence ledger row
/// resolves to a known disposition.
#[derive(Debug, Clone)]
pub struct VerifiedResource {
    pub pack: PackId,
    pub resource: ResourceId,
    pub path: PathBuf,
    pub sha256: String,
    pub blake3: String,
    pub license: String,
}

/// Resolves pack resources to verified on-disk paths. The implementation
/// MUST NOT support install discovery, environment variables, PATH lookup,
/// or internet download.
pub trait PackResourceResolver {
    fn require(&self, pack: PackId, resource: ResourceId) -> Result<VerifiedResource, ResolverError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ResolverError {
    #[error("pack not registered: {0}")]
    PackNotRegistered(String),
    #[error("resource not present in pack: {pack}/{resource}")]
    ResourceNotPresent { pack: String, resource: String },
    #[error("hash mismatch for {path}: expected {expected}, got {actual}")]
    HashMismatch { path: String, expected: String, actual: String },
    #[error("license not in ledger: {0}")]
    UnknownLicense(String),
}

/// CutRight adapter identity. Surface this in every speech session log so
/// the run receipt can be cross-referenced against the vendored commit.
#[derive(Debug, Clone)]
pub struct AdapterIdentity {
    pub vendored_commit: String,
    pub pack_hashes: BTreeMap<PackId, String>,
    pub build_target: String,
}

/// Bounded stderr capture from a sidecar boundary.
#[derive(Debug, Clone, Default)]
pub struct BoundedStderr {
    pub max_bytes: usize,
    pub captured: Vec<u8>,
}

impl BoundedStderr {
    pub fn new(max_bytes: usize) -> Self {
        Self { max_bytes, captured: Vec::new() }
    }
    pub fn push(&mut self, chunk: &[u8]) {
        let remaining = self.max_bytes.saturating_sub(self.capd_len());
        if remaining == 0 {
            return;
        }
        let take = chunk.len().min(remaining);
        self.captured.extend_from_slice(&chunk[..take]);
    }
    fn capd_len(&self) -> usize {
        self.captured.len()
    }
}

/// Cancellation token routed into the vendored engine.
#[derive(Debug, Clone)]
pub struct Cancellation {
    pub cancelled: bool,
}

impl Cancellation {
    pub fn new() -> Self { Self { cancelled: false } }
    pub fn cancel(&mut self) { self.cancelled = true; }
}

/// Supervised session options.
#[derive(Debug, Clone)]
pub struct SessionOptions {
    pub timeout: Duration,
    pub stderr: BoundedStderr,
    pub cancellation: Cancellation,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            stderr: BoundedStderr::new(64 * 1024),
            cancellation: Cancellation::new(),
        }
    }
}

/// Stub the session entry point so the cap-ledger can record the
/// hand-shake. The actual session dispatch lives in the vendored
/// crate; this adapter only wraps the boundary.
pub fn open_session(
    identity: &AdapterIdentity,
    _options: &SessionOptions,
) -> Result<SessionHandle, AdapterError> {
    if identity.vendored_commit.is_empty() {
        return Err(AdapterError::EmptyVendoredCommit);
    }
    Ok(SessionHandle { identity: identity.clone() })
}

#[derive(Debug, Clone)]
pub struct SessionHandle {
    pub identity: AdapterIdentity,
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("empty vendored commit in adapter identity")]
    EmptyVendoredCommit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_stderr_caps_at_max_bytes() {
        let mut s = BoundedStderr::new(8);
        s.push(b"abcdef");
        s.push(b"ghijkl");
        assert_eq!(s.captured.len(), 8);
    }

    #[test]
    fn cancellation_round_trip() {
        let mut c = Cancellation::new();
        assert!(!c.cancelled);
        c.cancel();
        assert!(c.cancelled);
    }

    #[test]
    fn session_options_have_bounded_stderr() {
        let o = SessionOptions::default();
        assert!(o.timeout > Duration::from_secs(0));
        assert_eq!(o.stderr.max_bytes, 64 * 1024);
    }
}
