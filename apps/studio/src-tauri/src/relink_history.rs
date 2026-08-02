//! Source-relink history ledger.
//!
//! Records every `relink_source` attempt (REV2 §12.6), matched or not, so a
//! rejected (hash mismatch) attempt is still auditable — it is not silently
//! dropped just because the manifest was not mutated.

use crate::decision_contract::feedback_ledger_path;
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

/// One append-only entry recording a `relink_source` attempt (REV2 §12.6).
/// Written whether or not the relink was applied, so a rejected (hash
/// mismatch) attempt is still auditable — it is not silently dropped just
/// because the manifest was not mutated.
#[derive(Debug, Clone, Serialize)]
pub struct RelinkHistoryRecord {
    pub ts: String,
    pub source_id: String,
    pub requested_path: String,
    pub canonical_path: String,
    pub expected_blake3: String,
    pub actual_blake3: Option<String>,
    pub matches: bool,
    pub applied: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

pub fn relink_history_path(root: &Path, create_parent: bool) -> Result<PathBuf, String> {
    feedback_ledger_path(root, "relink-history.jsonl", create_parent)
}

/// Append one relink attempt to `feedback/relink-history.jsonl`, next to the
/// decision ledger. Uses the same single-O_APPEND-write-then-sync pattern as
/// `decision_contract::append_record` so concurrent appends cannot interleave.
pub fn append_relink_record(root: &Path, record: &RelinkHistoryRecord) -> Result<(), String> {
    let path = relink_history_path(root, true)?;
    let mut line = serde_json::to_string(record).map_err(|e| e.to_string())?;
    line.push('\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    file.write_all(line.as_bytes())
        .and_then(|_| file.sync_data())
        .map_err(|e| format!("{}: {e}", path.display()))
}
