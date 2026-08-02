//! Authoritative review-decision contract.
//!
//! The frontend sends a minimal [`DecisionIntent`]; this module constructs the
//! persisted [`DecisionRecord`], binding it to the exact artifact reviewed.
//! Callers never supply subjects, timestamps, hashes, project identity, or
//! application version — those are resolved here so a malicious or stale client
//! cannot inject absolute or traversal paths or forge provenance.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

#[allow(unused_imports)] // re-exported for existing call sites (REV2 §14.7 pure move)
pub use crate::decision_ledger::{
    apply_intent, decisions_path, replay, DecisionReplay, DecisionWithStatus, MalformedLine,
    RecordStatus,
};

pub const SCHEMA_VERSION: u32 = 1;

static DECISION_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn err(field: &str, message: impl std::fmt::Display) -> String {
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
    /// Studio-owned immutable identity (REV2 §12.7), distinct from
    /// `project_id` (which `video_project::init_project` still derives from
    /// the folder name and can collide). Defaults to empty string when
    /// deserializing a decision written before this field existed.
    #[serde(default)]
    pub project_instance_id: String,
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

pub(crate) fn hash_file(path: &Path) -> Result<(String, u64), String> {
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
    let project_instance_id =
        crate::project_identity::resolve(root, Some(&project_id))?.project_instance_id;
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
        project_instance_id,
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
