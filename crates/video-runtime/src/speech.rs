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
    fn require(
        &self,
        pack: PackId,
        resource: ResourceId,
    ) -> Result<VerifiedResource, ResolverError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ResolverError {
    #[error("pack not registered: {0}")]
    PackNotRegistered(String),
    #[error("resource not present in pack: {pack}/{resource}")]
    ResourceNotPresent { pack: String, resource: String },
    #[error("hash mismatch for {path}: expected {expected}, got {actual}")]
    HashMismatch {
        path: String,
        expected: String,
        actual: String,
    },
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
        Self {
            max_bytes,
            captured: Vec::new(),
        }
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
    pub fn new() -> Self {
        Self { cancelled: false }
    }
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
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

/// Open a session only after every runtime input resolves from a verified
/// speech pack. No path discovery or fallback occurs at this boundary.
pub fn open_session(
    resolver: &impl PackResourceResolver,
    identity: &AdapterIdentity,
    _options: &SessionOptions,
) -> Result<SessionHandle, AdapterError> {
    if identity.vendored_commit.is_empty() {
        return Err(AdapterError::EmptyVendoredCommit);
    }
    let resources = [
        "bin/heardright-engine",
        "bin/libsherpa-onnx-c-api.dylib",
        "bin/libonnxruntime.dylib",
        "bin/models/parakeet-tdt-v3/encoder.int8.onnx",
        "bin/models/parakeet-tdt-v3/decoder.int8.onnx",
        "bin/models/parakeet-tdt-v3/joiner.int8.onnx",
        "bin/models/parakeet-tdt-v3/tokens.txt",
        "bin/vad/silero_vad_16k_op15.onnx",
    ]
    .into_iter()
    .map(|resource| resolver.require(PackId("speech".into()), ResourceId(resource.into())))
    .collect::<Result<Vec<_>, _>>()?;
    Ok(SessionHandle {
        identity: identity.clone(),
        resources,
    })
}

#[derive(Debug, Clone)]
pub struct SessionHandle {
    pub identity: AdapterIdentity,
    pub resources: Vec<VerifiedResource>,
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("empty vendored commit in adapter identity")]
    EmptyVendoredCommit,
    #[error(transparent)]
    Resolver(#[from] ResolverError),
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Resolver;

    impl PackResourceResolver for Resolver {
        fn require(
            &self,
            pack: PackId,
            resource: ResourceId,
        ) -> Result<VerifiedResource, ResolverError> {
            Ok(VerifiedResource {
                pack,
                path: PathBuf::from(&resource.0),
                resource,
                sha256: "sha256".into(),
                blake3: "blake3".into(),
                license: "verified".into(),
            })
        }
    }

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

    #[test]
    fn session_requires_all_signed_pack_resources() {
        let identity = AdapterIdentity {
            vendored_commit: "b60bff947f12ffa9d25e94ad27e8ff30db006a24".into(),
            pack_hashes: BTreeMap::new(),
            build_target: "universal-apple-darwin".into(),
        };
        let session = open_session(&Resolver, &identity, &SessionOptions::default()).unwrap();
        assert_eq!(session.resources.len(), 8);
        assert!(session
            .resources
            .iter()
            .all(|resource| resource.pack.0 == "speech"));
    }
}
