//! Content-addressed materialization for embedded sidecar binaries
//! (hardening plan §10.2).
//!
//! Embedded worker binaries (caption-card, vision-anchor, and any future
//! sidecar) are compiled into the host crate with `include_bytes!` and then
//! written out to a temp path before being spawned. Naming that temp path by
//! crate version alone is unsound: editing the worker's source without
//! bumping the crate version leaves a stale binary on disk that keeps
//! running. Naming it by the content hash of the embedded bytes instead
//! means a changed worker always lands at a new path, and an unchanged
//! worker always reuses the same path — with the on-disk bytes verified
//! against that hash before every reuse.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Directory under the system temp directory that holds every
/// content-addressed sidecar binary.
const WORKERS_DIR: &str = "cutright-workers";

#[derive(Debug, thiserror::Error)]
pub enum ContentStoreError {
    #[error("failed to materialize sidecar worker at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(
        "sidecar worker at {path} does not match its content-addressed digest {expected} (tampered or corrupted on disk)"
    )]
    Tampered { path: PathBuf, expected: String },
}

/// Materialize `bytes` (an embedded sidecar binary) at
/// `$TMPDIR/cutright-workers/<blake3-hex>/<name>` and return that path.
///
/// - If nothing exists at the content-addressed path yet, it is written and
///   (on unix) marked executable.
/// - If a file already exists there, its bytes are re-hashed and compared
///   against the digest encoded in the path before being trusted — a
///   mismatch is reported as [`ContentStoreError::Tampered`] rather than
///   silently overwritten, so a corrupted or tampered binary is never
///   executed.
/// - A worker whose embedded bytes changed (a source edit with no crate
///   version bump) hashes to a different digest and therefore lands at a
///   different path automatically, so it can never collide with the stale
///   binary.
pub fn materialize_worker(bytes: &[u8], name: &str) -> Result<PathBuf, ContentStoreError> {
    let digest = blake3::hash(bytes).to_hex().to_string();
    let dir = std::env::temp_dir().join(WORKERS_DIR).join(&digest);
    let path = dir.join(name);

    if path.is_file() {
        let existing = fs::read(&path).map_err(|source| ContentStoreError::Io {
            path: path.clone(),
            source,
        })?;
        let existing_digest = blake3::hash(&existing).to_hex().to_string();
        if existing_digest != digest {
            return Err(ContentStoreError::Tampered {
                path,
                expected: digest,
            });
        }
        return Ok(path);
    }

    fs::create_dir_all(&dir).map_err(|source| ContentStoreError::Io {
        path: dir.clone(),
        source,
    })?;
    fs::write(&path, bytes).map_err(|source| ContentStoreError::Io {
        path: path.clone(),
        source,
    })?;
    set_executable(&path)?;
    Ok(path)
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<(), ContentStoreError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|source| {
        ContentStoreError::Io {
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<(), ContentStoreError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materializes_new_content_at_its_hash_path() {
        let path = materialize_worker(b"worker-v1", "demo-worker").expect("materialize");
        assert!(path.is_file());
        assert_eq!(fs::read(&path).expect("read"), b"worker-v1");
        assert!(
            path.parent()
                .expect("parent")
                .file_name()
                .expect("hash dir name")
                .to_string_lossy()
                .len()
                >= 32
        );
        let _ = fs::remove_dir_all(path.parent().expect("parent"));
    }

    #[test]
    fn reuses_identical_bytes_without_rewriting() {
        let path_a = materialize_worker(b"stable-bytes", "demo-worker").expect("first write");
        let modified_at = fs::metadata(&path_a).expect("metadata").modified().ok();
        let path_b = materialize_worker(b"stable-bytes", "demo-worker").expect("second call");
        assert_eq!(path_a, path_b);
        if let Some(before) = modified_at {
            let after = fs::metadata(&path_b).expect("metadata").modified().ok();
            assert_eq!(after, Some(before));
        }
        let _ = fs::remove_dir_all(path_a.parent().expect("parent"));
    }

    #[test]
    fn rematerializes_when_embedded_bytes_change() {
        let path_v1 = materialize_worker(b"payload-v1", "demo-worker").expect("v1");
        let path_v2 = materialize_worker(b"payload-v2", "demo-worker").expect("v2");
        assert_ne!(path_v1, path_v2, "changed bytes must land at a new path");
        assert_eq!(fs::read(&path_v2).expect("read v2"), b"payload-v2");
        let _ = fs::remove_dir_all(path_v1.parent().expect("parent"));
        let _ = fs::remove_dir_all(path_v2.parent().expect("parent"));
    }

    #[test]
    fn rejects_tampered_bytes_at_the_content_addressed_path() {
        let path = materialize_worker(b"trusted-payload", "demo-worker").expect("materialize");
        fs::write(&path, b"tampered-payload").expect("simulate tamper");
        let result = materialize_worker(b"trusted-payload", "demo-worker");
        assert!(matches!(result, Err(ContentStoreError::Tampered { .. })));
        let _ = fs::remove_dir_all(path.parent().expect("parent"));
    }
}
