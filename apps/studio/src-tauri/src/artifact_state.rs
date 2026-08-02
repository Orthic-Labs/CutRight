//! Explicit state for optional, filesystem-backed project artifacts.
//!
//! `video_project::project_snapshot` (in `crates/video-project`, which this
//! app does not own) reads several optional artifacts — the QA report, the
//! transcription benchmark, the reframe plan, each variant's cut plan —
//! through a path that collapses "file does not exist" and "file exists but
//! failed to parse" into the same `None`. REV2 plan §12.1 requires Studio to
//! tell those apart. Since the crate itself is out of scope here, this module
//! re-reads the same well-known artifact paths independently and reports the
//! true state, which `main.rs` attaches to the snapshot payload alongside the
//! crate's original (still-lossy) fields for compatibility.

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// The true state of an on-disk JSON artifact.
///
/// - `Missing`: the file does not exist. Nothing has gone wrong.
/// - `Ready`: the file exists and parsed successfully.
/// - `Invalid`: the file exists but could not be read or parsed — this is
///   corruption, and must never be presented to a user as "not generated".
/// - `Stale`: the file exists and parsed successfully, but a caller-supplied
///   check determined it no longer reflects the current project state (for
///   example, a cut plan edited after its rough cut was last rendered).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", content = "data", rename_all = "snake_case")]
pub enum ArtifactState<T> {
    Missing,
    Ready(T),
    Invalid { path: PathBuf, error: String },
    Stale { path: PathBuf, reason: String },
}

impl<T> ArtifactState<T> {
    /// Borrow the value if this state is `Ready`. Used by this module's own
    /// tests; kept `pub` as the natural accessor for any future call site
    /// that needs to inspect a resolved artifact rather than just its state.
    #[allow(dead_code)]
    pub fn ready(&self) -> Option<&T> {
        match self {
            ArtifactState::Ready(value) => Some(value),
            _ => None,
        }
    }
}

/// Read and parse a JSON artifact, distinguishing a missing file from an
/// unreadable/malformed one. `stale_reason` runs only after a successful
/// parse and, if it returns `Some`, downgrades `Ready` to `Stale` with that
/// explanation; it never runs for a file that does not exist or failed to
/// parse.
pub fn load_json_checked<T: DeserializeOwned>(
    path: &Path,
    stale_reason: impl FnOnce(&T) -> Option<String>,
) -> ArtifactState<T> {
    if !path.is_file() {
        return ArtifactState::Missing;
    }
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return ArtifactState::Invalid {
                path: path.to_path_buf(),
                error: error.to_string(),
            }
        }
    };
    match serde_json::from_slice::<T>(&bytes) {
        Ok(value) => match stale_reason(&value) {
            Some(reason) => ArtifactState::Stale {
                path: path.to_path_buf(),
                reason,
            },
            None => ArtifactState::Ready(value),
        },
        Err(error) => ArtifactState::Invalid {
            path: path.to_path_buf(),
            error: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn scratch_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "cutright-artifact-state-{}-{}-{name}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn missing_file_is_missing_not_invalid() {
        let path = scratch_path("missing.json");
        let state = load_json_checked::<serde_json::Value>(&path, |_| None);
        assert!(matches!(state, ArtifactState::Missing));
    }

    #[test]
    fn malformed_json_is_invalid_with_the_parse_error() {
        let path = scratch_path("malformed.json");
        fs::write(&path, b"{not json").unwrap();
        let state = load_json_checked::<serde_json::Value>(&path, |_| None);
        match state {
            ArtifactState::Invalid { path: p, error } => {
                assert_eq!(p, path);
                assert!(!error.is_empty());
            }
            other => panic!("expected Invalid, got {other:?}"),
        }
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn valid_json_is_ready() {
        let path = scratch_path("ready.json");
        fs::write(&path, br#"{"ok":true}"#).unwrap();
        let state = load_json_checked::<serde_json::Value>(&path, |_| None);
        assert_eq!(state.ready().unwrap()["ok"], serde_json::json!(true));
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn stale_reason_downgrades_a_valid_parse() {
        let path = scratch_path("stale.json");
        fs::write(&path, br#"{"ok":true}"#).unwrap();
        let state = load_json_checked::<serde_json::Value>(&path, |_| {
            Some("superseded by a newer render".to_string())
        });
        match state {
            ArtifactState::Stale { path: p, reason } => {
                assert_eq!(p, path);
                assert_eq!(reason, "superseded by a newer render");
            }
            other => panic!("expected Stale, got {other:?}"),
        }
        fs::remove_file(&path).unwrap();
    }
}
