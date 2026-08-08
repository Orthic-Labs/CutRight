// apps/studio/src-tauri/src/project_index.rs
//
// Book 6 task CR-V2-B6-007 — Lane A Home / rebuildable project library.
//
// Owns the disposable project index described by
// `schemas/studio/project-index.schema.v1.json`. The index is rebuilt from
// project packages and app-local history; deleting the file loses no
// project truth.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const SCHEMA: &str = "cutright.studio.project_index/v1";

#[derive(Debug)]
pub enum IndexError {
    Io(std::io::Error),
    Json(serde_json::Error),
    InvalidSchema(String),
}

impl fmt::Display for IndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::Json(e) => write!(f, "json error: {e}"),
            Self::InvalidSchema(s) => write!(f, "invalid schema: {s}"),
        }
    }
}

impl std::error::Error for IndexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::InvalidSchema(_) => None,
        }
    }
}

impl From<std::io::Error> for IndexError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for IndexError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LaneId {
    RecordedFootage,
    Repurpose,
    Explainer,
    AnchoredCreative,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Idle,
    Running,
    Ready,
    NeedsReview,
    Failed,
    Stale,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectIndexRow {
    pub project_instance_id: String,
    pub package_path: String,
    pub title: String,
    pub lane: LaneId,
    pub active_revision: String,
    pub run_status: RunStatus,
    pub ready_count: u32,
    pub needs_review_count: u32,
    pub failed_count: u32,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIndex {
    pub schema: String,
    pub version: u32,
    pub rows: Vec<ProjectIndexRow>,
    #[serde(default)]
    pub watch_folder_import_enabled: bool,
}

impl Default for ProjectIndex {
    fn default() -> Self {
        Self {
            schema: SCHEMA.into(),
            version: 1,
            rows: Vec::new(),
            watch_folder_import_enabled: false,
        }
    }
}

/// Resolve the canonical path of the project index file inside the app's
/// local data directory. Pure function; no IO.
pub fn default_index_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("studio").join("project-index.v1.json")
}

/// Load the index from disk. If the file is missing or corrupt, an empty
/// default index is returned. Deletion of the file is *not* an error.
pub fn load_or_default(path: &Path) -> Result<ProjectIndex, IndexError> {
    if !path.exists() {
        return Ok(ProjectIndex::default());
    }
    let text = fs::read_to_string(path)?;
    match serde_json::from_str::<ProjectIndex>(&text) {
        Ok(idx) if idx.schema == SCHEMA => Ok(idx),
        Ok(_) => Err(IndexError::InvalidSchema(path.display().to_string())),
        Err(_) => Err(IndexError::InvalidSchema(path.display().to_string())),
    }
}

/// Persist the index atomically (write to tmp, then rename). The parent
/// directory is created on demand.
pub fn save(path: &Path, index: &ProjectIndex) -> Result<(), IndexError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(index)?;
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Rebuild the index from a list of registered package paths. Existing
/// rows whose `package_path` is not in `package_paths` are kept (with
/// `run_status = Missing`) so the user can see them in the index even if
/// the package is currently unmounted. Pass `history` to inject
/// app-local recent rows that aren't currently registered.
pub fn rebuild(
    rows_in: Vec<ProjectIndexRow>,
    package_paths: &[String],
    history: Vec<ProjectIndexRow>,
) -> ProjectIndex {
    let mut by_path: BTreeMap<String, ProjectIndexRow> = BTreeMap::new();
    for row in rows_in {
        by_path.insert(row.package_path.clone(), row);
    }
    for row in history {
        by_path.entry(row.package_path.clone()).or_insert(row);
    }
    let mut rows: Vec<ProjectIndexRow> = by_path
        .into_values()
        .map(|mut row| {
            if !package_paths.iter().any(|p| p == &row.package_path) {
                row.run_status = RunStatus::Missing;
            }
            row
        })
        .collect();
    // Deterministic sort: most recent first, ties break on id.
    rows.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| a.project_instance_id.cmp(&b.project_instance_id))
    });
    ProjectIndex {
        schema: SCHEMA.into(),
        version: 1,
        rows,
        watch_folder_import_enabled: false,
    }
}

/// Repair a single row by absolute package path. If the row is new, it is
/// inserted at the top of the index.
pub fn repair(path: &Path, row: ProjectIndexRow) -> Result<ProjectIndex, IndexError> {
    let mut index = load_or_default(path)?;
    if let Some(existing) = index
        .rows
        .iter_mut()
        .find(|r| r.package_path == row.package_path)
    {
        *existing = row;
    } else {
        index.rows.insert(0, row);
    }
    save(path, &index)?;
    Ok(index)
}

/// Remove a row from the index by project instance id. The underlying
/// project package is untouched.
pub fn remove_from_list(
    path: &Path,
    project_instance_id: &str,
) -> Result<ProjectIndex, IndexError> {
    let mut index = load_or_default(path)?;
    index
        .rows
        .retain(|r| r.project_instance_id != project_instance_id);
    save(path, &index)?;
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn row(id: &str, path: &str, updated: &str) -> ProjectIndexRow {
        ProjectIndexRow {
            project_instance_id: id.into(),
            package_path: path.into(),
            title: format!("Project {id}"),
            lane: LaneId::RecordedFootage,
            active_revision: "rev_001".into(),
            run_status: RunStatus::Ready,
            ready_count: 1,
            needs_review_count: 0,
            failed_count: 0,
            updated_at: updated.into(),
            thumbnail_hash: None,
        }
    }

    #[test]
    fn load_missing_returns_default() {
        let dir = tempdir().unwrap();
        let p = default_index_path(dir.path());
        let idx = load_or_default(&p).unwrap();
        assert_eq!(idx.schema, SCHEMA);
        assert!(idx.rows.is_empty());
        assert!(!idx.watch_folder_import_enabled);
    }

    #[test]
    fn rebuild_marks_missing_packages() {
        let rows = vec![
            row("p1", "/a", "2026-08-07T10:00:00Z"),
            row("p2", "/b", "2026-08-07T09:00:00Z"),
        ];
        let idx = rebuild(rows, &["/a".into()], Vec::new());
        assert_eq!(idx.rows.len(), 2);
        let by_path: BTreeMap<&str, &ProjectIndexRow> = idx
            .rows
            .iter()
            .map(|r| (r.package_path.as_str(), r))
            .collect();
        assert_eq!(by_path["/a"].run_status, RunStatus::Ready);
        assert_eq!(by_path["/b"].run_status, RunStatus::Missing);
    }

    #[test]
    fn repair_inserts_new_row_at_top() {
        let dir = tempdir().unwrap();
        let p = default_index_path(dir.path());
        let r = row("p1", "/a", "2026-08-07T10:00:00Z");
        let idx = repair(&p, r).unwrap();
        assert_eq!(idx.rows.len(), 1);
        assert_eq!(idx.rows[0].project_instance_id, "p1");
    }

    #[test]
    fn remove_from_list_preserves_other_rows() {
        let dir = tempdir().unwrap();
        let p = default_index_path(dir.path());
        let r1 = row("p1", "/a", "2026-08-07T10:00:00Z");
        let r2 = row("p2", "/b", "2026-08-07T11:00:00Z");
        let mut idx = repair(&p, r1).unwrap();
        idx = repair(&p, r2).unwrap();
        assert_eq!(idx.rows.len(), 2);
        let idx = remove_from_list(&p, "p1").unwrap();
        assert_eq!(idx.rows.len(), 1);
        assert_eq!(idx.rows[0].project_instance_id, "p2");
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempdir().unwrap();
        let p = default_index_path(dir.path());
        let r = row("p1", "/a", "2026-08-07T10:00:00Z");
        let mut idx = ProjectIndex::default();
        idx.rows.push(r);
        save(&p, &idx).unwrap();
        let loaded = load_or_default(&p).unwrap();
        assert_eq!(loaded.rows.len(), 1);
        assert_eq!(loaded.rows[0].project_instance_id, "p1");
    }
}
