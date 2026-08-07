//! Revision and receipt types (CR-V2-B2-010).
//!
//! - [`Revision`] matches `schemas/core/revision.schema.v1.json`.
//! - [`Receipt`] matches `schemas/actions/action-result.schema.v1.json`.
//! - [`StagedRevision`] is the working copy the staged apply pipeline
//!   operates on; it is immutable from the validator's perspective but
//!   mutable from the apply pipeline's perspective.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::validation::ValidationContext;

/// Schema id for a [`Revision`].
pub const REVISION_SCHEMA: &str = "cutright.revision/v1";

/// Schema id for a [`Receipt`].
pub const RECEIPT_SCHEMA: &str = "cutright.action_result/v1";

/// Typed failure code attached to a [`Receipt`] failure. Mirrors the enum in
/// `schemas/actions/action-result.schema.v1.json`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    /// `expected_revision` did not match the active revision.
    StaleRevision,
    /// The target id was not present in the staged revision.
    MissingTarget,
    /// The action range was out of order or out of bounds.
    InvalidRange,
    /// The action's permission set was insufficient.
    PermissionDenied,
    /// The action exceeded a resource limit (budget, time, etc.).
    ResourceLimit,
    /// Apply produced partial output; inverse rollback is required.
    PartialOutput,
    /// The action kind was not in the frozen vocabulary.
    UnknownActionKind,
    /// Generic semantic-validation failure (`validation_error`).
    ValidationError,
}

/// Failure entry attached to a receipt when the apply pipeline fails.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReceiptFailure {
    /// Stable failure code.
    pub code: FailureCode,
    /// Human-readable message.
    pub message: String,
    /// Index of the offending action within its batch.
    pub action_index: usize,
}

/// Wire schema for an `action_result/v1` receipt. Schema id is
/// [`RECEIPT_SCHEMA`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    /// Always [`RECEIPT_SCHEMA`].
    pub schema: String,
    /// The action batch id this receipt describes.
    pub batch_id: String,
    /// `applied`, `dry_run`, or `failed`.
    pub status: ReceiptStatus,
    /// The new revision id (only meaningful when status = `applied`).
    pub new_revision: String,
    /// Stable receipt id (`rcpt_<32-hex>`).
    pub receipt_id: String,
    /// Applied action ids (empty for failed receipts).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applied_actions: Vec<String>,
    /// Failures recorded during the apply.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<ReceiptFailure>,
}

/// Status of a [`Receipt`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptStatus {
    /// Apply succeeded end-to-end.
    Applied,
    /// Dry-run only; no mutation occurred.
    DryRun,
    /// Apply failed; no mutation took effect.
    Failed,
}

impl Receipt {
    /// Build a successful apply receipt.
    pub fn applied(
        batch_id: impl Into<String>,
        new_revision: impl Into<String>,
        receipt_id: impl Into<String>,
        applied_actions: Vec<String>,
    ) -> Self {
        Self {
            schema: RECEIPT_SCHEMA.to_string(),
            batch_id: batch_id.into(),
            status: ReceiptStatus::Applied,
            new_revision: new_revision.into(),
            receipt_id: receipt_id.into(),
            applied_actions,
            failures: Vec::new(),
        }
    }

    /// Build a dry-run receipt.
    pub fn dry_run(
        batch_id: impl Into<String>,
        planned_revision: impl Into<String>,
        receipt_id: impl Into<String>,
    ) -> Self {
        Self {
            schema: RECEIPT_SCHEMA.to_string(),
            batch_id: batch_id.into(),
            status: ReceiptStatus::DryRun,
            new_revision: planned_revision.into(),
            receipt_id: receipt_id.into(),
            applied_actions: Vec::new(),
            failures: Vec::new(),
        }
    }

    /// Build a failed receipt.
    pub fn failed(
        batch_id: impl Into<String>,
        receipt_id: impl Into<String>,
        failures: Vec<ReceiptFailure>,
    ) -> Self {
        Self {
            schema: RECEIPT_SCHEMA.to_string(),
            batch_id: batch_id.into(),
            status: ReceiptStatus::Failed,
            new_revision: String::new(),
            receipt_id: receipt_id.into(),
            applied_actions: Vec::new(),
            failures,
        }
    }
}

/// Wire schema for a `revision/v1` revision. Schema id is [`REVISION_SCHEMA`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    /// Always [`REVISION_SCHEMA`].
    pub schema: String,
    /// Stable revision id (`rev_<32-hex>`).
    pub revision_id: String,
    /// Parent revision ids (0..=2).
    pub parents: Vec<String>,
    /// Nanosecond unix timestamp.
    pub created_at_ns: i64,
    /// Active pointer string (typically the project or active timeline id).
    pub active_pointer: String,
    /// BLAKE3 hash of the frozen public surface (>= 16 hex chars).
    pub compatibility_fp: String,
}

/// Staged-revision working copy used by the apply pipeline.
///
/// `StagedRevision` is what `V2-TRANSACTIONS-UNDO.md` §1 calls the "staged
/// clone": the active revision is cloned, all writes go here, and the staged
/// clone only becomes the live revision after a successful atomic commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedRevision {
    /// Revision id of the parent (the active revision when the apply starts).
    pub parent_revision_id: String,
    /// Staged revision id (deterministic from parent + actions).
    pub staged_revision_id: String,
    /// Project this revision belongs to.
    pub project_id: String,
    /// Total timeline duration in nanoseconds.
    pub duration_ns: i64,
    /// Known target ids in this revision.
    pub known_targets: BTreeSet<String>,
    /// Per-target project mapping for cross-project checks.
    pub target_projects: BTreeMap<String, String>,
    /// Known project ids.
    pub known_project_ids: BTreeSet<String>,
    /// Active pointer (typically the project id).
    pub active_pointer: String,
    /// Per-action id assigned during planning (used for receipts/inverse).
    pub planned_action_ids: Vec<String>,
}

impl StagedRevision {
    /// Construct a staged revision with minimal state. Use [`from_active`]
    /// when promoting a live revision into a staged clone.
    pub fn new(
        parent_revision_id: impl Into<String>,
        staged_revision_id: impl Into<String>,
        project_id: impl Into<String>,
        duration_ns: i64,
        active_pointer: impl Into<String>,
    ) -> Self {
        Self {
            parent_revision_id: parent_revision_id.into(),
            staged_revision_id: staged_revision_id.into(),
            project_id: project_id.into(),
            duration_ns,
            known_targets: BTreeSet::new(),
            target_projects: BTreeMap::new(),
            known_project_ids: BTreeSet::new(),
            active_pointer: active_pointer.into(),
            planned_action_ids: Vec::new(),
        }
    }

    /// Promote a live revision into a staged clone (step 1 of
    /// `V2-TRANSACTIONS-UNDO.md` §1).
    pub fn from_active(active: &Revision, duration_ns: i64) -> Self {
        let mut staged = Self::new(
            &active.revision_id,
            &active.revision_id,
            active.active_pointer.clone(),
            duration_ns,
            &active.active_pointer,
        );
        staged
            .known_project_ids
            .insert(active.active_pointer.clone());
        staged
    }

    /// Register a known target id (and optionally its project).
    pub fn register_target(&mut self, target_id: impl Into<String>) {
        self.known_targets.insert(target_id.into());
    }

    /// Build a [`ValidationContext`] for the validators. The validator takes
    /// this view and never sees the mutable staged revision.
    pub fn validation_context(&self) -> ValidationContext {
        let mut ctx = ValidationContext::new(
            self.project_id.clone(),
            self.duration_ns,
            self.known_targets.clone(),
        );
        ctx.target_projects = self.target_projects.clone();
        ctx.known_project_ids = self.known_project_ids.clone();
        ctx
    }

    /// Commit the staged revision: produces an immutable [`Revision`]
    /// matching the `revision/v1` schema. Caller is responsible for writing
    /// it atomically and only after the active pointer is ready to be
    /// swapped (steps 4 + 6 of `V2-TRANSACTIONS-UNDO.md` §1).
    pub fn commit(
        &self,
        created_at_ns: i64,
        compatibility_fp: impl Into<String>,
    ) -> Revision {
        Revision {
            schema: REVISION_SCHEMA.to_string(),
            revision_id: self.staged_revision_id.clone(),
            parents: vec![self.parent_revision_id.clone()],
            created_at_ns,
            active_pointer: self.active_pointer.clone(),
            compatibility_fp: compatibility_fp.into(),
        }
    }
}

/// Typed error returned by revision construction and staged-revision helpers.
#[derive(Debug, Error)]
pub enum RevisionError {
    /// The active revision file could not be read or parsed.
    #[error("failed to load active revision {path}: {source}")]
    Load {
        /// Path of the revision file that failed to load.
        path: String,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The revision file was JSON but did not match the frozen schema.
    #[error("revision file {path} is malformed: {message}")]
    Malformed {
        /// Path of the malformed revision file.
        path: String,
        /// Description of the schema drift.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn active() -> Revision {
        Revision {
            schema: REVISION_SCHEMA.to_string(),
            revision_id: "rev_0001".into(),
            parents: Vec::new(),
            created_at_ns: 1_700_000_000_000_000_000,
            active_pointer: "prj_main".into(),
            compatibility_fp: "deadbeefcafebabe1234567890abcdef".into(),
        }
    }

    #[test]
    fn revision_round_trips() {
        let r = active();
        let value = serde_json::to_value(&r).unwrap();
        assert_eq!(value["schema"], "cutright.revision/v1");
        assert_eq!(value["revision_id"], "rev_0001");
        let decoded: Revision = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, r);
    }

    #[test]
    fn revision_rejects_unknown_fields() {
        let bogus = serde_json::json!({
            "schema": "cutright.revision/v1",
            "revision_id": "rev_0001",
            "parents": [],
            "created_at_ns": 0,
            "active_pointer": "p",
            "compatibility_fp": "deadbeefcafebabe1234567890abcdef",
            "rogue": true,
        });
        serde_json::from_value::<Revision>(bogus)
            .expect_err("unknown field must fail closed");
    }

    #[test]
    fn receipt_applied_round_trips() {
        let receipt = Receipt::applied(
            "batch_0001",
            "rev_0002",
            "rcpt_0001",
            vec!["act_0".to_string()],
        );
        let value = serde_json::to_value(&receipt).unwrap();
        assert_eq!(value["schema"], "cutright.action_result/v1");
        assert_eq!(value["status"], "applied");
        let decoded: Receipt = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, receipt);
    }

    #[test]
    fn receipt_dry_run_round_trips() {
        let receipt = Receipt::dry_run("batch_0001", "rev_0002", "rcpt_0001");
        let value = serde_json::to_value(&receipt).unwrap();
        assert_eq!(value["status"], "dry_run");
        let decoded: Receipt = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, receipt);
    }

    #[test]
    fn receipt_failed_round_trips() {
        let receipt = Receipt::failed(
            "batch_0001",
            "rcpt_0001",
            vec![ReceiptFailure {
                code: FailureCode::ValidationError,
                message: "bad".into(),
                action_index: 0,
            }],
        );
        let value = serde_json::to_value(&receipt).unwrap();
        assert_eq!(value["status"], "failed");
        assert_eq!(value["failures"][0]["code"], "validation_error");
        let decoded: Receipt = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, receipt);
    }

    #[test]
    fn staged_revision_from_active_inherits_state() {
        let active = active();
        let staged = StagedRevision::from_active(&active, 10_000_000_000);
        assert_eq!(staged.parent_revision_id, "rev_0001");
        assert_eq!(staged.active_pointer, "prj_main");
        assert!(staged.known_project_ids.contains("prj_main"));
    }

    #[test]
    fn staged_revision_commit_produces_valid_revision() {
        let active = active();
        let staged = StagedRevision::from_active(&active, 10_000_000_000);
        let committed = staged.commit(1_700_000_001_000_000_000, "newhash1234567890abcdef");
        assert_eq!(committed.schema, REVISION_SCHEMA);
        assert_eq!(committed.parents, vec!["rev_0001".to_string()]);
        assert_eq!(committed.active_pointer, "prj_main");
    }

    #[test]
    fn validation_context_reflects_staged_state() {
        let active = active();
        let mut staged = StagedRevision::from_active(&active, 10_000_000_000);
        staged.register_target("clip:clip_5");
        let ctx = staged.validation_context();
        assert!(ctx.knows_target(
            &crate::action::TargetRef::from_parts(crate::action::TargetKind::Clip, "clip_5")
                .unwrap()
        ));
    }
}
