//! Immutable project revision storage (`cutright.revision/v1`).
//!
//! Frozen contracts honoured here:
//! - `schemas/core/revision.schema.v1.json` — every revision on disk.
//! - `docs/architecture/V2-IDENTITY-TIME-REVISION.md` — acyclic parent graph,
//!   0..2 parents, content-addressed `revision_id`.
//! - `docs/architecture/V2-TRANSACTIONS-UNDO.md` — staged apply produces a new
//!   immutable revision; the active pointer is only advanced after the
//!   revision commit + receipt emit succeed.
//!
//! ## On-disk layout
//!
//! ```text
//! <project_root>/
//!   .state/
//!     revisions/<revision_id>/
//!       revision.json      # cutright.revision/v1 metadata
//!       state.json         # the staged state this revision commits
//!     active_pointer       # single line: revision_id of the active revision
//! ```
//!
//! ## Atomicity
//!
//! Every directory (`<revision_id>/`) and the active pointer is written via
//! temp-dir + rename. A reader either sees the previous revision or the new
//! one — never a half-written tree. The active pointer is a single file
//! renamed atomically over the previous one (POSIX `rename(2)` is atomic on
//! the same filesystem).

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Schema tag carried by every revision metadata file. Kept in lock-step with
/// `schemas/core/revision.schema.v1.json`.
pub const REVISION_SCHEMA: &str = "cutright.revision/v1";

/// Directory holding all revision material under the project root.
pub const STATE_DIR: &str = ".state";
/// Subdirectory under `STATE_DIR` that holds every revision subdirectory.
pub const REVISIONS_DIR: &str = "revisions";
/// Filename of the single-file active pointer under `STATE_DIR`.
pub const ACTIVE_POINTER_FILE: &str = "active_pointer";

/// Opaque, content-addressed revision identifier. Matches the schema regex
/// `^[A-Za-z0-9_-]+$`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RevisionId(String);

impl RevisionId {
    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Construct a `RevisionId` from a raw string after validating the schema
    /// regex. Returns `RevisionError::InvalidId` on a mismatch.
    pub fn parse(value: &str) -> Result<Self, RevisionError> {
        if value.is_empty()
            || !value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(RevisionError::InvalidId(value.to_string()));
        }
        Ok(Self(value.to_string()))
    }

    /// Wrap an already-validated string in a `RevisionId`. Caller is
    /// responsible for ensuring the value matches the schema regex. Used by
    /// the store when it has just produced a BLAKE3 digest.
    pub fn from_blake3_hex(hex: &str) -> Self {
        debug_assert!(
            hex.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
            "BLAKE3 hex must match the schema regex"
        );
        Self(hex.to_string())
    }
}

impl std::fmt::Display for RevisionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for RevisionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// The staged state that a revision commits. Modeled as arbitrary JSON so that
/// it can carry timelines, tracks, clips, sources, evidence, settings, etc.
/// The store is intentionally state-shape agnostic; lanes that own schema
/// (P-A, P-B) project their own typed views on top of this.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StagedState(pub serde_json::Value);

impl StagedState {
    /// Build a staged state from any `Serialize` value.
    pub fn from_value<T: Serialize>(value: &T) -> Result<Self, RevisionError> {
        serde_json::to_value(value).map(Self).map_err(RevisionError::Serialize)
    }

    /// Borrow the underlying JSON value.
    pub fn as_value(&self) -> &serde_json::Value {
        &self.0
    }
}

/// A committed, immutable revision.
///
/// Matches `schemas/core/revision.schema.v1.json` and is the only revision
/// shape the store will persist or load.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Revision {
    /// Schema tag — always `cutright.revision/v1`.
    pub schema: String,
    /// Content-addressed identifier (BLAKE3 over the staged state).
    pub revision_id: RevisionId,
    /// 0..2 parent revision IDs that this revision descends from.
    pub parents: Vec<RevisionId>,
    /// Wall-clock timestamp the revision was committed, in nanoseconds.
    pub created_at_ns: u64,
    /// Optional human-meaningful pointer label (e.g. `timeline_main`). The
    /// schema makes this optional; the store preserves it for round-trip but
    /// does not interpret it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_pointer: Option<String>,
    /// BLAKE3 fingerprint of the frozen public surface this revision was
    /// produced against. Detects cross-version mixing.
    pub compatibility_fp: String,
}

/// Errors that can be returned by [`RevisionStore`].
#[derive(Debug, Error)]
pub enum RevisionError {
    /// I/O error during a file or directory operation.
    #[error("revision store I/O error at {path}: {source}")]
    Io {
        /// Path the operation was acting on.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// JSON serialization or deserialization failed.
    #[error("could not serialize revision state: {0}")]
    Serialize(#[source] serde_json::Error),
    /// A revision ID string did not match the schema regex.
    #[error("invalid revision id `{0}` (must match `^[A-Za-z0-9_-]+$`)")]
    InvalidId(String),
    /// A requested revision was not present in the store.
    #[error("revision `{0}` not found")]
    NotFound(RevisionId),
    /// A parent revision cited by a new revision was not present in the
    /// store.
    #[error("missing parent revision `{0}` for new revision `{1}`")]
    MissingParent(RevisionId, RevisionId),
    /// A `set_active_pointer` call named a revision that does not exist.
    #[error("cannot point active to missing revision `{0}`")]
    UnknownActivePointer(RevisionId),
    /// A concurrent or repeated `put` detected a hash collision on the same
    /// (state, parents[]) tuple. Treated as a non-fatal boundary condition
    /// that the caller may retry.
    #[error("revision `{0}` already exists")]
    AlreadyExists(RevisionId),
    /// A revision's metadata file was structurally valid but its `revision_id`
    /// did not match the expected on-disk path (corruption / tampering).
    #[error("revision id mismatch in `{path}`: expected `{expected}`, got `{actual}`")]
    RevisionIdMismatch {
        /// Path to the metadata file that disagreed with the directory name.
        path: PathBuf,
        /// Revision id the directory said it should contain.
        expected: String,
        /// Revision id the metadata actually declared.
        actual: String,
    },
}

impl RevisionError {
    fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

/// Immutable, content-addressed revision store.
///
/// Constructed against a project root path. The store creates the layout
/// directories lazily on the first write.
#[derive(Debug, Clone)]
pub struct RevisionStore {
    project_root: PathBuf,
}

impl RevisionStore {
    /// Build a store that lives under `project_root`. Does not touch the
    /// filesystem until a write is performed.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    /// Project root this store was constructed against.
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Path to the `.state` directory under the project root.
    pub fn state_dir(&self) -> PathBuf {
        self.project_root.join(STATE_DIR)
    }

    /// Path to the directory holding every revision subdirectory.
    pub fn revisions_dir(&self) -> PathBuf {
        self.state_dir().join(REVISIONS_DIR)
    }

    /// Path to the active pointer file (`.state/active_pointer`).
    pub fn active_pointer_path(&self) -> PathBuf {
        self.state_dir().join(ACTIVE_POINTER_FILE)
    }

    /// Path to the metadata file for a given revision.
    pub fn revision_metadata_path(&self, id: &RevisionId) -> PathBuf {
        self.revisions_dir().join(id.as_str()).join("revision.json")
    }

    /// Path to the staged-state file for a given revision.
    pub fn revision_state_path(&self, id: &RevisionId) -> PathBuf {
        self.revisions_dir().join(id.as_str()).join("state.json")
    }

    /// Commit a new revision.
    ///
    /// The `revision_id` is computed as the BLAKE3 digest of the schema tag
    /// concatenated with the canonical JSON of the staged state. The two
    /// parent slots (if any) must already exist in the store. The new
    /// revision is written atomically via temp-dir + rename.
    pub fn put(
        &self,
        parent_a: Option<&RevisionId>,
        parent_b: Option<&RevisionId>,
        staged_state: &StagedState,
        compatibility_fp: &str,
    ) -> Result<RevisionId, RevisionError> {
        for parent in parent_a.iter().chain(parent_b.iter()) {
            if !self.exists(parent) {
                let pending = RevisionId::from_blake3_hex("__pending__");
                return Err(RevisionError::MissingParent((*parent).clone(), pending));
            }
        }

        let state_bytes = serde_json::to_vec(staged_state.as_value())
            .map_err(RevisionError::Serialize)?;
        let mut hasher = blake3::Hasher::new();
        hasher.update(REVISION_SCHEMA.as_bytes());
        hasher.update(b"\n");
        hasher.update(&state_bytes);
        let digest = hasher.finalize();
        let hex = digest.to_hex().to_string();
        let revision_id = RevisionId::from_blake3_hex(&hex);

        let metadata_path = self.revision_metadata_path(&revision_id);
        if metadata_path.exists() {
            return Err(RevisionError::AlreadyExists(revision_id));
        }

        let parents = [parent_a.cloned(), parent_b.cloned()]
            .into_iter()
            .flatten()
            .collect();
        let revision = Revision {
            schema: REVISION_SCHEMA.to_string(),
            revision_id: revision_id.clone(),
            parents,
            created_at_ns: now_ns(),
            active_pointer: None,
            compatibility_fp: compatibility_fp.to_string(),
        };

        let revision_dir = self.revisions_dir().join(revision_id.as_str());
        let temp_dir = unique_temp_dir(&revision_dir);
        fs::create_dir_all(&temp_dir).map_err(|source| RevisionError::io(&temp_dir, source))?;

        let metadata_tmp = temp_dir.join("revision.json");
        let metadata_bytes = serde_json::to_vec_pretty(&revision)
            .map_err(RevisionError::Serialize)?;
        write_atomic(&metadata_tmp, &metadata_bytes)?;

        let state_tmp = temp_dir.join("state.json");
        write_atomic(&state_tmp, &state_bytes)?;

        // Atomic rename of the whole revision directory.
        fs::rename(&temp_dir, &revision_dir).map_err(|source| {
            // Best-effort cleanup of the temp dir if rename failed.
            let _ = fs::remove_dir_all(&temp_dir);
            RevisionError::io(&revision_dir, source)
        })?;

        Ok(revision_id)
    }

    /// Load a revision's metadata. Returns `RevisionError::NotFound` if the
    /// revision directory is absent, and `RevisionError::RevisionIdMismatch`
    /// if the directory is present but the metadata's `revision_id` does not
    /// match the on-disk path (corruption / tampering).
    pub fn get(&self, id: &RevisionId) -> Result<Revision, RevisionError> {
        let metadata_path = self.revision_metadata_path(id);
        let bytes = fs::read(&metadata_path).map_err(|source| {
            if source.kind() == io::ErrorKind::NotFound {
                RevisionError::NotFound(id.clone())
            } else {
                RevisionError::io(&metadata_path, source)
            }
        })?;
        let revision: Revision = serde_json::from_slice(&bytes).map_err(RevisionError::Serialize)?;
        if revision.revision_id != *id {
            return Err(RevisionError::RevisionIdMismatch {
                path: metadata_path,
                expected: id.to_string(),
                actual: revision.revision_id.to_string(),
            });
        }
        Ok(revision)
    }

    /// Load the staged state that a revision committed.
    pub fn get_state(&self, id: &RevisionId) -> Result<StagedState, RevisionError> {
        // Ensure the metadata round-trips first so we surface corruption
        // before trusting the staged state file.
        let _ = self.get(id)?;
        let state_path = self.revision_state_path(id);
        let bytes = fs::read(&state_path).map_err(|source| RevisionError::io(state_path, source))?;
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(RevisionError::Serialize)?;
        Ok(StagedState(value))
    }

    /// List every revision currently in the store, in on-disk order.
    pub fn list(&self) -> Result<Vec<RevisionId>, RevisionError> {
        let revisions_dir = self.revisions_dir();
        if !revisions_dir.exists() {
            return Ok(Vec::new());
        }
        let read = fs::read_dir(&revisions_dir).map_err(|source| RevisionError::io(&revisions_dir, source))?;
        let mut ids = Vec::new();
        for entry in read {
            let entry = entry.map_err(|source| RevisionError::io(&revisions_dir, source))?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            // Skip any temp directories that may have been left behind by a
            // crashed write — they always carry the `.tmp-` suffix.
            if name.starts_with(".tmp-") {
                continue;
            }
            if let Ok(id) = RevisionId::parse(&name) {
                ids.push(id);
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// Whether a revision id is present in the store.
    pub fn exists(&self, id: &RevisionId) -> bool {
        self.revision_metadata_path(id).exists()
    }

    /// Atomically advance the active pointer to `revision_id`. The named
    /// revision must already exist; otherwise [`RevisionError::UnknownActivePointer`]
    /// is returned.
    pub fn set_active_pointer(&self, id: &RevisionId) -> Result<(), RevisionError> {
        if !self.exists(id) {
            return Err(RevisionError::UnknownActivePointer(id.clone()));
        }
        let state_dir = self.state_dir();
        fs::create_dir_all(&state_dir).map_err(|source| RevisionError::io(&state_dir, source))?;
        let target = self.active_pointer_path();
        let temp = unique_temp_file(&target);
        {
            let mut file = fs::File::create(&temp).map_err(|source| RevisionError::io(&temp, source))?;
            file.write_all(id.as_str().as_bytes())
                .map_err(|source| RevisionError::io(&temp, source))?;
            file.write_all(b"\n")
                .map_err(|source| RevisionError::io(&temp, source))?;
            file.sync_all().map_err(|source| RevisionError::io(&temp, source))?;
        }
        fs::rename(&temp, &target).map_err(|source| {
            let _ = fs::remove_file(&temp);
            RevisionError::io(&target, source)
        })?;
        Ok(())
    }

    /// Read the current active pointer, if any.
    pub fn active_pointer(&self) -> Result<Option<RevisionId>, RevisionError> {
        let path = self.active_pointer_path();
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&path).map_err(|source| RevisionError::io(&path, source))?;
        let trimmed = std::str::from_utf8(bytes.trim_ascii())
            .map_err(|source| RevisionError::io(&path, io::Error::other(source)))?;
        if trimmed.is_empty() {
            return Ok(None);
        }
        Ok(Some(RevisionId::parse(trimmed)?))
    }
}

fn write_atomic(target: &Path, bytes: &[u8]) -> Result<(), RevisionError> {
    let temp = unique_temp_file(target);
    {
        let mut file = fs::File::create(&temp).map_err(|source| RevisionError::io(&temp, source))?;
        file.write_all(bytes)
            .map_err(|source| RevisionError::io(&temp, source))?;
        file.sync_all().map_err(|source| RevisionError::io(&temp, source))?;
    }
    fs::rename(&temp, target).map_err(|source| {
        let _ = fs::remove_file(&temp);
        RevisionError::io(target, source)
    })
}

fn unique_temp_dir(target: &Path) -> PathBuf {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let stem = target
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "revision".to_string());
    let nonce = now_ns();
    parent.join(format!(".tmp-{stem}-{nonce:x}"))
}

fn unique_temp_file(target: &Path) -> PathBuf {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let stem = target
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let nonce = now_ns();
    parent.join(format!(".tmp-{stem}-{nonce:x}"))
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
    use std::sync::Arc;
    use std::thread;

    fn unique_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cutright-revision-store-{nanos}-{counter}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    fn staged(label: &str) -> StagedState {
        let value = serde_json::json!({"label": label, "version": 1});
        StagedState::from_value(&value).expect("serialize staged state")
    }

    #[test]
    fn put_then_get_round_trips() {
        let dir = unique_dir();
        let store = RevisionStore::new(&dir);
        let id = store
            .put(None, None, &staged("alpha"), "fp-test")
            .expect("put");
        let revision = store.get(&id).expect("get");
        assert_eq!(revision.schema, REVISION_SCHEMA);
        assert_eq!(revision.revision_id, id);
        assert!(revision.parents.is_empty());
        assert_eq!(revision.compatibility_fp, "fp-test");

        let state = store.get_state(&id).expect("state");
        assert_eq!(state.as_value()["label"], "alpha");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_parent_is_rejected() {
        let dir = unique_dir();
        let store = RevisionStore::new(&dir);
        let phantom = RevisionId::parse("rev_doesnotexist").unwrap();
        let result = store.put(Some(&phantom), None, &staged("beta"), "fp-test");
        assert!(matches!(result, Err(RevisionError::MissingParent(_, _))));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn duplicate_put_is_rejected_as_already_exists() {
        let dir = unique_dir();
        let store = RevisionStore::new(&dir);
        let state = staged("dup");
        let id = store.put(None, None, &state, "fp-test").expect("first put");
        let second = store.put(None, None, &state, "fp-test");
        assert!(matches!(second, Err(RevisionError::AlreadyExists(ref found)) if found == &id));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_returns_every_committed_revision() {
        let dir = unique_dir();
        let store = RevisionStore::new(&dir);
        let a = store.put(None, None, &staged("a"), "fp").expect("a");
        let b = store.put(Some(&a), None, &staged("b"), "fp").expect("b");
        let c = store.put(Some(&b), None, &staged("c"), "fp").expect("c");
        let mut ids = store.list().expect("list");
        ids.sort();
        let mut expected = vec![a, b, c];
        expected.sort();
        assert_eq!(ids, expected);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn set_active_pointer_round_trips() {
        let dir = unique_dir();
        let store = RevisionStore::new(&dir);
        let id = store.put(None, None, &staged("first"), "fp").expect("put");
        store.set_active_pointer(&id).expect("set active");
        let pointed = store.active_pointer().expect("active pointer").expect("some");
        assert_eq!(pointed, id);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn set_active_pointer_rejects_unknown_revision() {
        let dir = unique_dir();
        let store = RevisionStore::new(&dir);
        let phantom = RevisionId::parse("rev_missing").unwrap();
        let result = store.set_active_pointer(&phantom);
        assert!(matches!(result, Err(RevisionError::UnknownActivePointer(_))));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupted_metadata_with_wrong_id_is_detected() {
        let dir = unique_dir();
        let store = RevisionStore::new(&dir);
        let id = store.put(None, None, &staged("corrupt"), "fp").expect("put");
        let metadata_path = store.revision_metadata_path(&id);
        let bytes = fs::read(&metadata_path).expect("read metadata");
        // Rewrite the metadata with a different `revision_id` but keep the
        // path. The store must reject this as `RevisionIdMismatch`.
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("parse");
        value["revision_id"] = serde_json::Value::String("rev_other".to_string());
        let rewritten = serde_json::to_vec_pretty(&value).expect("rewrite");
        fs::write(&metadata_path, rewritten).expect("write tampered");
        let result = store.get(&id);
        assert!(matches!(result, Err(RevisionError::RevisionIdMismatch { .. })));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn concurrent_puts_of_distinct_states_both_succeed() {
        let dir = unique_dir();
        let store = Arc::new(RevisionStore::new(&dir));
        let mut handles = Vec::new();
        for i in 0..8u32 {
            let store = Arc::clone(&store);
            handles.push(thread::spawn(move || {
                let state = staged(&format!("thread-{i}"));
                store.put(None, None, &state, "fp").expect("concurrent put")
            }));
        }
        let mut ids: Vec<RevisionId> = handles
            .into_iter()
            .map(|h| h.join().expect("thread join"))
            .collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 8, "every thread's put should produce a distinct id");
        // Active pointer un-set until something calls set_active_pointer.
        assert!(store.active_pointer().expect("active pointer").is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn revision_id_parser_accepts_and_rejects_regex() {
        assert!(RevisionId::parse("rev_0001").is_ok());
        assert!(RevisionId::parse("a-b-c-x_y_z").is_ok());
        assert!(RevisionId::parse("").is_err());
        assert!(RevisionId::parse("has space").is_err());
        assert!(RevisionId::parse("with/slash").is_err());
    }
}
