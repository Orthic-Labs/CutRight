//! Authoritative review-decision contract.
//!
//! The frontend sends a minimal [`DecisionIntent`]; this module constructs the
//! persisted [`DecisionRecord`], binding it to the exact artifact reviewed.
//! Callers never supply subjects, timestamps, hashes, project identity, or
//! application version — those are resolved here so a malicious or stale client
//! cannot inject absolute or traversal paths or forge provenance.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

pub const SCHEMA_VERSION: u32 = 1;

static DECISION_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn err(field: &str, message: impl std::fmt::Display) -> String {
    format!("{field}: {message}")
}

/// What a review target resolves to on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "target_kind", rename_all = "snake_case")]
pub enum ReviewTarget {
    Variant { variant: String },
    Final { preset: String },
    Segment { variant: String, segment_id: String },
    QaReport { preset: Option<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionVerdict {
    Approved,
    Rejected,
    Acknowledged,
}

impl DecisionVerdict {
    fn as_str(self) -> &'static str {
        match self {
            DecisionVerdict::Approved => "approved",
            DecisionVerdict::Rejected => "rejected",
            DecisionVerdict::Acknowledged => "acknowledged",
        }
    }
}

/// One reason vocabulary shared by all targets; [`ReviewReason::valid_for`]
/// enforces which reasons each target accepts. The frontend renders only the
/// subset valid for the active target.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewReason {
    Pacing,
    WordEdges,
    Energy,
    Length,
    LooksRight,
    Captions,
    Loudness,
    Framing,
    Color,
    Audio,
    ClippedWord,
    TooTight,
    TooLoose,
    BadBoundary,
    WrongTake,
    Reviewed,
    Other,
}

impl ReviewReason {
    fn as_str(self) -> &'static str {
        match self {
            ReviewReason::Pacing => "pacing",
            ReviewReason::WordEdges => "word_edges",
            ReviewReason::Energy => "energy",
            ReviewReason::Length => "length",
            ReviewReason::LooksRight => "looks_right",
            ReviewReason::Captions => "captions",
            ReviewReason::Loudness => "loudness",
            ReviewReason::Framing => "framing",
            ReviewReason::Color => "color",
            ReviewReason::Audio => "audio",
            ReviewReason::ClippedWord => "clipped_word",
            ReviewReason::TooTight => "too_tight",
            ReviewReason::TooLoose => "too_loose",
            ReviewReason::BadBoundary => "bad_boundary",
            ReviewReason::WrongTake => "wrong_take",
            ReviewReason::Reviewed => "reviewed",
            ReviewReason::Other => "other",
        }
    }

    pub fn valid_for(self, target: &ReviewTarget) -> bool {
        use ReviewReason::*;
        match target {
            ReviewTarget::Variant { .. } => {
                matches!(self, Pacing | WordEdges | Energy | Length | Other)
            }
            ReviewTarget::Final { .. } => matches!(
                self,
                LooksRight | Captions | Loudness | Framing | Color | Audio | Other
            ),
            ReviewTarget::Segment { .. } => matches!(
                self,
                ClippedWord | TooTight | TooLoose | BadBoundary | WrongTake | Other
            ),
            ReviewTarget::QaReport { .. } => matches!(self, Reviewed),
        }
    }
}

/// The minimal, typed intent the Studio frontend submits.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DecisionIntent {
    pub schema_version: u32,
    pub client_request_id: String,
    pub target: ReviewTarget,
    pub verdict: DecisionVerdict,
    pub reason: ReviewReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub playhead_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub word_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_word_id: Option<String>,
}

/// The authoritative, persisted record. Written as one JSON line.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DecisionRecord {
    pub decision_id: String,
    pub schema_version: u32,
    pub client_request_id: String,
    pub ts: String,
    pub project_id: String,
    pub kind: String,
    pub verdict: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub subject: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_blake3: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segment_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub word_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_word_id: Option<String>,
    pub playhead_ms: i64,
    pub bench_resolved: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bench_report_blake3: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_revision: Option<String>,
    pub app_version: String,
}

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

pub fn is_word_id(value: &str) -> bool {
    value
        .strip_prefix("ow_")
        .is_some_and(|digits| digits.len() == 6 && digits.bytes().all(|b| b.is_ascii_digit()))
}

pub fn is_source_word_id(value: &str) -> bool {
    let Some((source, word)) = value.rsplit_once(':') else {
        return false;
    };
    !source.is_empty()
        && word.starts_with("w_")
        && word.len() == 8
        && word[2..].bytes().all(|b| b.is_ascii_digit())
}

fn hash_file(path: &Path) -> Result<(String, u64), String> {
    let mut file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hasher = blake3::Hasher::new();
    let size = io::copy(&mut file, &mut hasher).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((format!("blake3:{}", hasher.finalize().to_hex()), size))
}

fn read_project_id(root: &Path) -> Result<String, String> {
    let path = root.join("project.json");
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?)
            .map_err(|e| format!("{}: {e}", path.display()))?;
    value
        .get("project_id")
        .and_then(serde_json::Value::as_str)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| err("project_id", "missing from project.json"))
}

/// Benchmark resolution state and the report hash that binds it.
fn bench_state(root: &Path) -> (bool, Option<String>) {
    let report = root.join("analysis/bench/transcribe/report.json");
    if !report.is_file() {
        return (false, None);
    }
    let hash = hash_file(&report).ok().map(|(h, _)| h);
    let resolved = fs::read(&report)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|value| {
            value
                .get("decision")
                .and_then(|d| d.as_str())
                .map(str::to_owned)
        })
        .map(|decision| decision != "unresolved")
        .unwrap_or(false);
    (resolved, hash)
}

struct ResolvedTarget {
    kind: String,
    subject: String,
    variant: Option<String>,
    segment_id: Option<String>,
    preset: Option<String>,
}

/// Resolve the canonical project-relative subject and derived scope fields from
/// the target. The subject path is constructed here, never taken from the client.
fn resolve_target(target: &ReviewTarget) -> Result<ResolvedTarget, String> {
    Ok(match target {
        ReviewTarget::Variant { variant } => {
            if variant.is_empty() {
                return Err(err("variant", "must not be empty"));
            }
            ResolvedTarget {
                kind: "variant_verdict".into(),
                subject: format!("render/rough-cuts/{variant}.mp4"),
                variant: Some(variant.clone()),
                segment_id: None,
                preset: None,
            }
        }
        ReviewTarget::Final { preset } => {
            if preset.is_empty() {
                return Err(err("preset", "must not be empty"));
            }
            ResolvedTarget {
                kind: "final_verdict".into(),
                subject: format!("render/finals/{preset}.mp4"),
                variant: None,
                segment_id: None,
                preset: Some(preset.clone()),
            }
        }
        ReviewTarget::Segment {
            variant,
            segment_id,
        } => {
            if variant.is_empty() {
                return Err(err("variant", "must not be empty"));
            }
            if segment_id.is_empty() {
                return Err(err("segment_id", "must not be empty"));
            }
            ResolvedTarget {
                kind: "segment_flag".into(),
                subject: format!("render/rough-cuts/{variant}.mp4"),
                variant: Some(variant.clone()),
                segment_id: Some(segment_id.clone()),
                preset: None,
            }
        }
        ReviewTarget::QaReport { preset } => ResolvedTarget {
            kind: "qa_ack".into(),
            subject: "qa/report.json".into(),
            variant: None,
            segment_id: None,
            preset: preset.clone(),
        },
    })
}

fn validate_verdict_for(kind: &str, verdict: DecisionVerdict) -> Result<(), String> {
    let ok = match kind {
        "variant_verdict" | "final_verdict" => {
            matches!(
                verdict,
                DecisionVerdict::Approved | DecisionVerdict::Rejected
            )
        }
        "segment_flag" => matches!(verdict, DecisionVerdict::Rejected),
        "qa_ack" => matches!(verdict, DecisionVerdict::Acknowledged),
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(err("verdict", format!("not valid for {kind}")))
    }
}

fn new_decision_id(now: &DateTime<Utc>, client_request_id: &str) -> String {
    let seq = DECISION_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut hasher = blake3::Hasher::new();
    hasher.update(now.to_rfc3339().as_bytes());
    hasher.update(client_request_id.as_bytes());
    hasher.update(std::process::id().to_le_bytes().as_slice());
    hasher.update(seq.to_le_bytes().as_slice());
    format!("d_{}", hasher.finalize().to_hex())
}

/// Construct the authoritative record from an intent, resolving all
/// machine-generated fields against the project on disk.
pub fn build_record(
    root: &Path,
    intent: &DecisionIntent,
    app_version: &str,
    now: DateTime<Utc>,
) -> Result<DecisionRecord, String> {
    if intent.schema_version != SCHEMA_VERSION {
        return Err(err("schema_version", format!("must be {SCHEMA_VERSION}")));
    }
    if intent.client_request_id.trim().is_empty() {
        return Err(err("client_request_id", "must not be empty"));
    }
    if intent.playhead_ms < 0 {
        return Err(err("playhead_ms", "must be non-negative"));
    }
    if !intent.reason.valid_for(&intent.target) {
        return Err(err(
            "reason",
            format!("{} not allowed for this target", intent.reason.as_str()),
        ));
    }
    match &intent.note {
        Some(note) => {
            if intent.reason != ReviewReason::Other {
                return Err(err("note", "is only allowed when reason is other"));
            }
            let trimmed = note.trim();
            if trimmed.is_empty() {
                return Err(err("note", "must not be empty"));
            }
            if trimmed.chars().count() > 200 {
                return Err(err("note", "must be 200 characters or fewer"));
            }
        }
        None => {
            if intent.reason == ReviewReason::Other {
                return Err(err("note", "is required when reason is other"));
            }
        }
    }
    if let Some(word_id) = &intent.word_id {
        if !is_word_id(word_id) {
            return Err(err("word_id", "must match ow_000000"));
        }
    }
    if let Some(source_word_id) = &intent.source_word_id {
        if !is_source_word_id(source_word_id) {
            return Err(err("source_word_id", "must match source-id:w_000000"));
        }
    }

    let resolved = resolve_target(&intent.target)?;
    validate_verdict_for(&resolved.kind, intent.verdict)?;

    let subject_path = root.join(&resolved.subject);
    if !subject_path.is_file() {
        return Err(err(
            "subject",
            format!("reviewed artifact does not exist: {}", resolved.subject),
        ));
    }
    let (subject_blake3, subject_size) = hash_file(&subject_path)?;

    let project_id = read_project_id(root)?;
    let (bench_resolved, bench_report_blake3) = bench_state(root);

    let mut revision = blake3::Hasher::new();
    revision.update(project_id.as_bytes());
    revision.update(subject_blake3.as_bytes());
    if let Some(bench_hash) = &bench_report_blake3 {
        revision.update(bench_hash.as_bytes());
    }
    let project_revision = format!("blake3:{}", revision.finalize().to_hex());

    Ok(DecisionRecord {
        decision_id: new_decision_id(&now, &intent.client_request_id),
        schema_version: SCHEMA_VERSION,
        client_request_id: intent.client_request_id.trim().to_owned(),
        ts: now.to_rfc3339(),
        project_id,
        kind: resolved.kind,
        verdict: intent.verdict.as_str().into(),
        reason: intent.reason.as_str().into(),
        note: intent.note.as_ref().map(|note| note.trim().to_owned()),
        subject: resolved.subject,
        subject_blake3: Some(subject_blake3),
        subject_size: Some(subject_size),
        variant: resolved.variant,
        segment_id: resolved.segment_id,
        preset: resolved.preset,
        word_id: intent.word_id.clone(),
        source_word_id: intent.source_word_id.clone(),
        playhead_ms: intent.playhead_ms,
        bench_resolved,
        bench_report_blake3,
        project_revision: Some(project_revision),
        app_version: app_version.to_owned(),
    })
}

pub fn decisions_path(root: &Path, create_parent: bool) -> Result<PathBuf, String> {
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
    let target = feedback.join("decisions.jsonl");
    if target.exists() {
        let canonical =
            fs::canonicalize(&target).map_err(|e| format!("{}: {e}", target.display()))?;
        if !canonical.starts_with(root) {
            return Err(err("path", "decisions file escapes the project"));
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
