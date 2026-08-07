//! Decision records for the v2 feedback loop.
//!
//! Every record is immutable, hash-bound, and appended to a chain. A stale
//! or mismatched subject hash is **retained** and flagged `stale_subject`
//! but is **excluded from learning**. A malformed record is retained but
//! flagged `malformed`. **No record is silently dropped.**

use blake3::Hasher;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Empty hash (64 hex zeros) used as the prev_hash of the first record.
pub const fn hash_chain_zero() -> &'static str {
    "0000000000000000000000000000000000000000000000000000000000000000"
}

/// Subjects that a decision can target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionTarget {
    Segment,
    Beat,
    Take,
    Boundary,
    Caption,
    Graphic,
    Effect,
    Audio,
    Crop,
    Final,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionAction {
    Approve,
    Reject,
    Replace,
    Trim,
    Extend,
    Reorder,
    Mute,
    Revoice,
    Restyle,
    Regenerate,
    Reframe,
    Rerender,
    Note,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionReason {
    TakeChoice,
    BoundaryChoice,
    FillerChoice,
    PauseChoice,
    HookChoice,
    CtaChoice,
    BeatOrder,
    CropChoice,
    CaptionChoice,
    GraphicChoice,
    EffectDensity,
    BRollChoice,
    SfxChoice,
    MusicChoice,
    ColorChoice,
    AudioChoice,
    IdentityChoice,
    FinalVerdict,
    UnknownReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionAxis {
    Take,
    Boundary,
    Filler,
    Pause,
    Hook,
    Cta,
    BeatOrder,
    Crop,
    Caption,
    Graphic,
    Motion,
    BRoll,
    Sfx,
    Music,
    Color,
    Audio,
    Identity,
    Final,
    UnsupportedAxis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatKey {
    pub content_type: String,
    pub platform: String,
    pub variant: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserOrigin {
    UserReviewed,
    UserRejected,
    UserReplaced,
    UserNoted,
    ModelSuggested,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionOrigin {
    ExternalSession,
    StudioReview,
    StudioAutonomous,
    Headless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewMode {
    Reviewed,
    ReviewLight,
    Autonomous,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub schema_version: String,
    pub decision_id: String,
    pub prev_hash: String,
    pub record_hash: String,
    pub project_instance_id: String,
    pub project_revision: String,
    pub subject_hash: String,
    pub decision_target: DecisionTarget,
    pub decision_action: DecisionAction,
    pub decision_reason: DecisionReason,
    pub decision_axis: DecisionAxis,
    pub delta: serde_json::Value,
    pub format_key: FormatKey,
    pub pack_set_id: String,
    pub pack_set_fingerprint: String,
    pub app_version: String,
    pub user_origin: UserOrigin,
    pub session_origin: SessionOrigin,
    pub asset_hash: Option<String>,
    pub effect_id: Option<String>,
    pub final_hash: Option<String>,
    pub review_mode: ReviewMode,
    pub sample_count: u32,
    pub confidence: f64,
    pub stale_subject: bool,
    pub malformed: bool,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Compute the BLAKE3 hash of a canonical decision record. `record_hash` is
/// excluded from the digest.
pub fn compute_record_hash(record: &DecisionRecord) -> String {
    let mut copy = record.clone();
    copy.record_hash = String::new();
    let bytes = serde_json::to_vec(&copy).expect("decision record serialises");
    let mut hasher = Hasher::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest.as_bytes() {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

/// Append a record to a chain. The caller supplies the previous record hash.
/// `stale_subject` indicates the subject hash is stale or mismatched; the
/// record is **retained** but excluded from learning.
pub fn append_record(prev_hash: &str, mut record: DecisionRecord) -> DecisionRecord {
    record.prev_hash = prev_hash.to_string();
    record.record_hash = compute_record_hash(&record);
    record
}

/// Returns true when the record's hash does not match its computed hash.
pub fn record_hash_mismatch(record: &DecisionRecord) -> bool {
    let computed = compute_record_hash(record);
    computed != record.record_hash
}
