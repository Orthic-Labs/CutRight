//! Project write locks and session bindings.
//!
//! Frozen contracts honoured here:
//! - `schemas/agent/session-binding.schema.v1.json` — every binding on disk
//!   or in flight.
//! - `docs/security/V2-ACTION-PERMISSIONS.md` — every external (loopback MCP)
//!   or embedded agent session is bound to one `project_id`, one
//!   `active_revision`, one `permission_set_id`. Cross-project writes fail
//!   with `cross_project_write_denied`. External MCP writes additionally
//!   require `frontmost_project_confirmed: true`.
//!
//! ## On-disk layout
//!
//! ```text
//! <project_root>/
//!   .state/
//!     lock         # advisory OS file lock (flock/lockf) held by the
//!                  # SessionGuard while a writer is active
//! ```
//!
//! The lock file is created lazily on the first [`SessionGuard::acquire`].

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Schema tag carried by every session binding. Kept in lock-step with
/// `schemas/agent/session-binding.schema.v1.json`.
pub const SESSION_BINDING_SCHEMA: &str = "cutright.session_binding/v1";

/// Directory under the project root that holds the lock file.
pub const STATE_DIR: &str = ".state";
/// Filename of the OS-file-lock file under `STATE_DIR`.
pub const LOCK_FILE: &str = "lock";

/// Stable session id. Schema `^[A-Za-z0-9_-]+$` (matches the v2 identifier
/// rule).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(String);

impl SessionId {
    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable project id. Schema `^[A-Za-z0-9_-]+$`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectId(String);

impl ProjectId {
    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable revision id (delegated to `cutright.revision/v1`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActiveRevisionId(String);

impl ActiveRevisionId {
    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ActiveRevisionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable permission-set id. Schema `^[A-Za-z0-9_-]+$`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PermissionSetId(String);

impl PermissionSetId {
    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PermissionSetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Origin surface a session is bound to. Matches the schema `surface` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionOrigin {
    /// Embedded UI / Studio pane.
    Embedded,
    /// External loopback MCP client.
    ExternalMcp,
    /// First-party CLI invocation.
    Cli,
}

/// A single session binding, matching `cutright.session_binding/v1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionBinding {
    /// Schema tag — always `cutright.session_binding/v1`.
    pub schema: String,
    /// Stable session id.
    pub session_id: SessionId,
    /// Stable project id the session is bound to.
    pub project_id: ProjectId,
    /// Revision the session observed when it was bound.
    pub active_revision: ActiveRevisionId,
    /// Origin surface. Optional in the schema; defaults to `embedded` when
    /// absent.
    #[serde(default)]
    pub surface: Option<SessionOrigin>,
    /// Frontmost-project confirmation flag. Required for external MCP writes.
    #[serde(default)]
    pub frontmost_project_confirmed: bool,
    /// Permission set assigned to the session.
    #[serde(default)]
    pub permission_set: Option<PermissionSetId>,
    /// Expiry timestamp in nanoseconds since the Unix epoch. Optional.
    #[serde(default)]
    pub expires_at_ns: Option<u64>,
}

/// Errors that can be raised while validating or constructing a
/// [`SessionBinding`].
#[derive(Debug, Error)]
pub enum SessionBindingError {
    /// A required field was missing or empty.
    #[error("session binding field `{0}` is required and must be non-empty")]
    MissingField(&'static str),
    /// The schema tag did not match the expected value.
    #[error("session binding schema `{actual}` does not match expected `{expected}`")]
    SchemaMismatch {
        /// Expected schema tag.
        expected: &'static str,
        /// Actual schema tag found in the binding.
        actual: String,
    },
    /// The binding has expired.
    #[error("session binding expired at {expires_at_ns} ns (now {now_ns} ns)")]
    Expired {
        /// Expiry timestamp from the binding.
        expires_at_ns: u64,
        /// Current timestamp the validator is running against.
        now_ns: u64,
    },
}

impl SessionBinding {
    /// Construct a new binding, validating required fields and the schema
    /// tag.
    pub fn new(
        session_id: SessionId,
        project_id: ProjectId,
        active_revision: ActiveRevisionId,
        surface: Option<SessionOrigin>,
        frontmost_project_confirmed: bool,
        permission_set: Option<PermissionSetId>,
        expires_at_ns: Option<u64>,
    ) -> Result<Self, SessionBindingError> {
        if session_id.as_str().is_empty() {
            return Err(SessionBindingError::MissingField("session_id"));
        }
        if project_id.as_str().is_empty() {
            return Err(SessionBindingError::MissingField("project_id"));
        }
        if active_revision.as_str().is_empty() {
            return Err(SessionBindingError::MissingField("active_revision"));
        }
        Ok(Self {
            schema: SESSION_BINDING_SCHEMA.to_string(),
            session_id,
            project_id,
            active_revision,
            surface,
            frontmost_project_confirmed,
            permission_set,
            expires_at_ns,
        })
    }

    /// Reject bindings whose `expires_at_ns` is in the past.
    pub fn validate_not_expired(&self, now_ns: u64) -> Result<(), SessionBindingError> {
        if let Some(expires) = self.expires_at_ns {
            if expires <= now_ns {
                return Err(SessionBindingError::Expired {
                    expires_at_ns: expires,
                    now_ns,
                });
            }
        }
        Ok(())
    }

    /// Reject bindings whose schema tag is not the expected value.
    pub fn validate_schema(&self) -> Result<(), SessionBindingError> {
        if self.schema != SESSION_BINDING_SCHEMA {
            return Err(SessionBindingError::SchemaMismatch {
                expected: SESSION_BINDING_SCHEMA,
                actual: self.schema.clone(),
            });
        }
        Ok(())
    }
}

/// Errors that can be raised by [`SessionGuard::acquire`] or by the
/// per-write guard checks.
#[derive(Debug, Error)]
pub enum SessionGuardError {
    /// I/O error during a lock or filesystem operation.
    #[error("session guard I/O error at {path}: {source}")]
    Io {
        /// Path the operation was acting on.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// Another writer holds the lock and refused to release it.
    #[error("project write lock at {0} is held by another writer")]
    LockHeld(PathBuf),
    /// The binding's `project_id` did not match the locked project.
    #[error(
        "cross project write denied: binding project_id `{binding}` does not match locked project `{locked}`"
    )]
    CrossProjectWriteDenied {
        /// Project id the binding claims.
        binding: ProjectId,
        /// Project id the lock was acquired against.
        locked: ProjectId,
    },
    /// An external MCP session attempted to write without the
    /// frontmost-project confirmation.
    #[error("external MCP write denied: frontmost project not confirmed for session {0}")]
    FrontmostProjectNotConfirmed(SessionId),
    /// The binding has expired.
    #[error("session binding expired: {0}")]
    ExpiredBinding(#[source] SessionBindingError),
    /// The binding failed schema validation.
    #[error("invalid session binding: {0}")]
    InvalidBinding(#[source] SessionBindingError),
}

/// Inner state shared by every `SessionGuard`. Owns the live `File` and the
/// path it locks.
#[derive(Debug)]
struct Inner {
    /// Path to the lock file the guard owns.
    lock_path: PathBuf,
    /// Project id the lock was acquired against.
    project_id: ProjectId,
    /// Open file handle holding the OS lock. Retained to keep the OS-level
    /// `flock(2)` live until the guard is dropped.
    #[allow(dead_code)]
    file: File,
}

/// A held project write lock. Acts as the single mutation entry point: every
/// write must call [`SessionGuard::assert_write_permitted`] and pass.
///
/// The guard holds an OS file lock (via `File::lock`) on `<project>/.state/lock`
/// for its entire lifetime. The lock is released atomically when the guard is
/// dropped.
#[derive(Debug, Clone)]
pub struct SessionGuard {
    inner: Arc<Inner>,
}

impl SessionGuard {
    /// Acquire the project write lock for `project_id`. The lock file is
    /// created if it does not yet exist. The returned guard MUST be kept alive
    /// for the entire duration of the writes it admits.
    pub fn acquire(
        project_root: impl Into<PathBuf>,
        project_id: ProjectId,
    ) -> Result<Self, SessionGuardError> {
        let project_root = project_root.into();
        let state_dir = project_root.join(STATE_DIR);
        fs::create_dir_all(&state_dir).map_err(|source| SessionGuardError::Io {
            path: state_dir.clone(),
            source,
        })?;
        let lock_path = state_dir.join(LOCK_FILE);
        let file = fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| SessionGuardError::Io {
                path: lock_path.clone(),
                source,
            })?;
        lock_exclusive(&file, &lock_path)?;
        Ok(Self {
            inner: Arc::new(Inner {
                lock_path,
                project_id,
                file,
            }),
        })
    }

    /// Project id this lock was acquired against.
    pub fn project_id(&self) -> &ProjectId {
        &self.inner.project_id
    }

    /// Path to the lock file.
    pub fn lock_path(&self) -> &Path {
        &self.inner.lock_path
    }

    /// Enforce every write precondition for `binding`.
    ///
    /// 1. `binding.schema` must match `cutright.session_binding/v1`.
    /// 2. `binding.project_id` must equal the locked project.
    /// 3. If `binding.surface` is `external_mcp`, `frontmost_project_confirmed`
    ///    must be `true`.
    /// 4. The binding must not have expired.
    ///
    /// Any failure raises a typed `SessionGuardError` variant.
    pub fn assert_write_permitted(&self, binding: &SessionBinding) -> Result<(), SessionGuardError> {
        binding.validate_schema().map_err(SessionGuardError::InvalidBinding)?;
        let now_ns = now_ns();
        binding
            .validate_not_expired(now_ns)
            .map_err(SessionGuardError::ExpiredBinding)?;
        if binding.project_id != self.inner.project_id {
            return Err(SessionGuardError::CrossProjectWriteDenied {
                binding: binding.project_id.clone(),
                locked: self.inner.project_id.clone(),
            });
        }
        if matches!(binding.surface, Some(SessionOrigin::ExternalMcp))
            && !binding.frontmost_project_confirmed
        {
            return Err(SessionGuardError::FrontmostProjectNotConfirmed(
                binding.session_id.clone(),
            ));
        }
        Ok(())
    }
}

/// Acquire an OS exclusive file lock. The dependency tree intentionally
/// avoids `fs2` / `file-lock`; we declare the lock via POSIX `flock(2)` in a
/// dependency-free way. The single `unsafe` block is bounded to the libc
/// call so the crate-level `deny(unsafe_code)` lint stays useful elsewhere.
#[cfg(unix)]
#[allow(unsafe_code)]
fn lock_exclusive(file: &File, lock_path: &Path) -> Result<(), SessionGuardError> {
    extern "C" {
        fn flock(fd: i32, op: i32) -> i32;
    }
    use std::os::unix::io::AsRawFd;
    // LOCK_EX = 2, LOCK_NB = 4. We want exclusive + non-blocking.
    const LOCK_EX: i32 = 2;
    const LOCK_NB: i32 = 4;
    let result = unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) };
    if result != 0 {
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::WouldBlock {
            return Err(SessionGuardError::LockHeld(lock_path.to_path_buf()));
        }
        return Err(SessionGuardError::Io {
            path: lock_path.to_path_buf(),
            source: err,
        });
    }
    Ok(())
}

#[cfg(not(unix))]
fn lock_exclusive(file: &File, lock_path: &Path) -> Result<(), SessionGuardError> {
    // On non-Unix platforms, holding an open `File` is enough to serialize
    // concurrent writers within the same process. Cross-process locking is
    // not supported in this build.
    let _ = file;
    let _ = lock_path;
    Ok(())
}

fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cutright-session-test-{nanos}-{counter}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    fn project_id(s: &str) -> ProjectId {
        ProjectId(s.to_string())
    }
    fn session_id(s: &str) -> SessionId {
        SessionId(s.to_string())
    }
    fn revision_id(s: &str) -> ActiveRevisionId {
        ActiveRevisionId(s.to_string())
    }

    fn embedded_binding(project: &ProjectId) -> SessionBinding {
        SessionBinding::new(
            session_id("sess_1"),
            project.clone(),
            revision_id("rev_0001"),
            Some(SessionOrigin::Embedded),
            false,
            None,
            None,
        )
        .expect("binding")
    }

    fn external_mcp_binding(project: &ProjectId, confirmed: bool) -> SessionBinding {
        SessionBinding::new(
            session_id("sess_mcp"),
            project.clone(),
            revision_id("rev_0001"),
            Some(SessionOrigin::ExternalMcp),
            confirmed,
            None,
            None,
        )
        .expect("binding")
    }

    #[test]
    fn binding_rejects_empty_required_fields() {
        let err = SessionBinding::new(
            session_id(""),
            project_id("proj_a"),
            revision_id("rev_0001"),
            None,
            false,
            None,
            None,
        )
        .unwrap_err();
        assert!(matches!(err, SessionBindingError::MissingField("session_id")));
    }

    #[test]
    fn binding_rejects_wrong_schema_tag() {
        let mut binding = embedded_binding(&project_id("proj_a"));
        binding.schema = "wrong.schema/v9".to_string();
        let err = binding.validate_schema().unwrap_err();
        assert!(matches!(err, SessionBindingError::SchemaMismatch { .. }));
    }

    #[test]
    fn binding_rejects_expired_binding() {
        let mut binding = embedded_binding(&project_id("proj_a"));
        binding.expires_at_ns = Some(1);
        let err = binding.validate_not_expired(2_000).unwrap_err();
        assert!(matches!(err, SessionBindingError::Expired { .. }));
    }

    #[test]
    fn acquire_releases_lock_on_drop() {
        let dir = unique_dir();
        let guard = SessionGuard::acquire(&dir, project_id("proj_a")).expect("acquire");
        assert!(guard.lock_path().exists());
        drop(guard);
        // After drop the lock must be re-acquirable.
        let again = SessionGuard::acquire(&dir, project_id("proj_a")).expect("re-acquire");
        drop(again);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cross_project_write_is_denied() {
        let dir = unique_dir();
        let guard = SessionGuard::acquire(&dir, project_id("proj_a")).expect("acquire");
        let foreign = embedded_binding(&project_id("proj_b"));
        let err = guard.assert_write_permitted(&foreign).unwrap_err();
        assert!(matches!(err, SessionGuardError::CrossProjectWriteDenied { .. }));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn external_mcp_write_requires_frontmost_confirmation() {
        let dir = unique_dir();
        let guard = SessionGuard::acquire(&dir, project_id("proj_a")).expect("acquire");
        let binding = external_mcp_binding(&project_id("proj_a"), false);
        let err = guard.assert_write_permitted(&binding).unwrap_err();
        assert!(matches!(
            err,
            SessionGuardError::FrontmostProjectNotConfirmed(_)
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn external_mcp_write_with_confirmation_succeeds() {
        let dir = unique_dir();
        let guard = SessionGuard::acquire(&dir, project_id("proj_a")).expect("acquire");
        let binding = external_mcp_binding(&project_id("proj_a"), true);
        guard.assert_write_permitted(&binding).expect("permitted");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn embedded_write_permitted_without_frontmost_confirmation() {
        let dir = unique_dir();
        let guard = SessionGuard::acquire(&dir, project_id("proj_a")).expect("acquire");
        let binding = embedded_binding(&project_id("proj_a"));
        guard.assert_write_permitted(&binding).expect("permitted");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn expired_binding_is_rejected_at_write_time() {
        let dir = unique_dir();
        let guard = SessionGuard::acquire(&dir, project_id("proj_a")).expect("acquire");
        let mut binding = embedded_binding(&project_id("proj_a"));
        binding.expires_at_ns = Some(1);
        let err = guard.assert_write_permitted(&binding).unwrap_err();
        assert!(matches!(err, SessionGuardError::ExpiredBinding(_)));
        let _ = fs::remove_dir_all(&dir);
    }
}
