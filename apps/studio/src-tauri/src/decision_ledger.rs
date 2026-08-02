//! JSONL decision ledger: persistence and replay for [`DecisionRecord`]s built
//! by [`crate::decision_contract`]. Split out of `decision_contract.rs` per
//! REV2 §14.7 — pure move, no behavior change.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::decision_contract::{build_record, err, hash_file, DecisionIntent, DecisionRecord};

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordStatus {
    Current,
    StaleArtifact,
    MissingArtifact,
    Superseded,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionWithStatus {
    #[serde(flatten)]
    pub record: DecisionRecord,
    pub status: RecordStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct MalformedLine {
    pub line_number: usize,
    pub content: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionReplay {
    pub records: Vec<DecisionWithStatus>,
    pub malformed_lines: Vec<MalformedLine>,
}

pub fn decisions_path(root: &Path, create_parent: bool) -> Result<PathBuf, String> {
    feedback_ledger_path(root, "decisions.jsonl", create_parent)
}

pub(crate) fn feedback_ledger_path(
    root: &Path,
    file_name: &str,
    create_parent: bool,
) -> Result<PathBuf, String> {
    let feedback = root.join("feedback");
    if create_parent && !feedback.exists() {
        fs::create_dir(&feedback).map_err(|e| format!("{}: {e}", feedback.display()))?;
    }
    if feedback.exists() {
        let canonical =
            fs::canonicalize(&feedback).map_err(|e| format!("{}: {e}", feedback.display()))?;
        if !canonical.starts_with(root) {
            return Err(err("path", "feedback directory escapes the project"));
        }
    }
    let target = feedback.join(file_name);
    if target.exists() {
        let canonical =
            fs::canonicalize(&target).map_err(|e| format!("{}: {e}", target.display()))?;
        if !canonical.starts_with(root) {
            return Err(err("path", "ledger file escapes the project"));
        }
        return Ok(canonical);
    }
    Ok(target)
}

/// Append one record as a single buffer to an O_APPEND file, then sync. A
/// single write to an O_APPEND handle is atomic for records of this size, so
/// concurrent appends cannot interleave a JSON body with a stray newline.
fn append_record(root: &Path, record: &DecisionRecord) -> Result<(), String> {
    let path = decisions_path(root, true)?;
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

fn find_by_client_request_id(root: &Path, id: &str) -> Option<DecisionRecord> {
    let path = decisions_path(root, false).ok()?;
    if !path.exists() {
        return None;
    }
    let file = File::open(&path).ok()?;
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(record) = serde_json::from_str::<DecisionRecord>(&line) {
            if record.client_request_id == id {
                return Some(record);
            }
        }
    }
    None
}

/// Idempotently apply an intent: a retry with the same `client_request_id`
/// returns the already-persisted record instead of appending a duplicate.
pub fn apply_intent(
    root: &Path,
    intent: &DecisionIntent,
    app_version: &str,
    now: DateTime<Utc>,
) -> Result<DecisionRecord, String> {
    let id = intent.client_request_id.trim();
    if !id.is_empty() {
        if let Some(existing) = find_by_client_request_id(root, id) {
            return Ok(existing);
        }
    }
    let record = build_record(root, intent, app_version, now)?;
    append_record(root, &record)?;
    Ok(record)
}

fn classify(root: &Path, record: &DecisionRecord) -> RecordStatus {
    let subject_path = root.join(&record.subject);
    match hash_file(&subject_path) {
        Ok((hash, _)) => match &record.subject_blake3 {
            Some(expected) if expected == &hash => RecordStatus::Current,
            _ => RecordStatus::StaleArtifact,
        },
        Err(_) => RecordStatus::MissingArtifact,
    }
}

/// Replay the ledger preserving stale and missing history. Schema-invalid lines
/// are reported as malformed; a once-valid record whose artifact changed stays
/// visible with a status rather than being silently discarded.
pub fn replay(root: &Path) -> Result<DecisionReplay, String> {
    let path = decisions_path(root, false)?;
    if !path.exists() {
        return Ok(DecisionReplay {
            records: Vec::new(),
            malformed_lines: Vec::new(),
        });
    }
    let file = File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut records = Vec::new();
    let mut malformed_lines = Vec::new();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|e| format!("{}: {e}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<DecisionRecord>(&line) {
            Ok(record) => {
                let status = classify(root, &record);
                records.push(DecisionWithStatus { record, status });
            }
            Err(error) => malformed_lines.push(MalformedLine {
                line_number: index + 1,
                content: line,
                error: error.to_string(),
            }),
        }
    }

    // A later verdict on the same subject supersedes earlier ones.
    let mut last_by_subject: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for (index, entry) in records.iter().enumerate() {
        if matches!(
            entry.record.kind.as_str(),
            "variant_verdict" | "final_verdict"
        ) {
            last_by_subject.insert(entry.record.subject.clone(), index);
        }
    }
    for (index, entry) in records.iter_mut().enumerate() {
        if matches!(
            entry.record.kind.as_str(),
            "variant_verdict" | "final_verdict"
        ) && last_by_subject.get(&entry.record.subject).copied() != Some(index)
            && entry.status == RecordStatus::Current
        {
            entry.status = RecordStatus::Superseded;
        }
    }

    Ok(DecisionReplay {
        records,
        malformed_lines,
    })
}
