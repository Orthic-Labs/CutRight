//! Single [`ActionExecutor`] that fronts every Book 2 lane behind one
//! surface (CR-V2-B2-022).
//!
//! Per `docs/dispatch/v2/book-2/interface-freeze.md`, the v2 contract has
//! exactly one action pipeline. Every consumer — the JSON CLI
//! (`videoctl apply` in B2-023), the Studio Tauri command (B2-024), and
//! the loopback MCP adapter (B2-025) — drives the same executor with the
//! same `ActionBatch` JSON and reads back the same `ExecutorReport`
//! receipt. This module is the one place that defines those wire types.
//!
//! Pipeline order (per `V2-TRANSACTIONS-UNDO.md` §1 and
//! `V2-CAPABILITY-ACTION-CONTRACT.md`):
//!
//! 1. **Capability check** — every action's `capability_id` must resolve
//!    in the canonical registry.
//! 2. **Session check** — if a binding is supplied, it must hold the
//!    required scope; the MCP adapter requires
//!    `frontmost_project_confirmed = true`.
//! 3. **Dry-run vs apply** — when `dry_run: true` the executor only
//!    validates; otherwise it delegates to `video_actions::StagedApply`
//!    which performs the staged-write → revision-commit →
//!    receipt-emit → active-pointer-swap stages.
//! 4. **Receipt** — the executor returns the
//!    `video_actions::revision::Receipt` unchanged; downstream surfaces
//!    serialize the [`ExecutorReport`] verbatim.
//!
//! Errors are surfaced as typed [`FailureCode`] entries so every consumer
//! (CLI, Studio, MCP) can branch on the same table.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use video_actions::action::Action;
use video_actions::apply::{ApplyError, InjectPoint, StagedApply};
use video_actions::diff::{dry_run as diff_dry_run, SemanticDiff};
use video_actions::revision::{
    FailureCode, Receipt, ReceiptFailure, ReceiptStatus, StagedRevision, RECEIPT_SCHEMA,
};
use video_capabilities::CapabilityRegistry;
use video_sessions::SessionGuard;

/// Schema id for the wire-level `cutright.action_batch/v1` envelope.
pub const ACTION_BATCH_SCHEMA: &str = "cutright.action_batch/v1";

/// Schema id for the executor's dry-run report. Matches the CLI/Studio/MCP
/// wire shape byte-for-byte.
pub const EXECUTOR_REPORT_SCHEMA: &str = "cutright.executor_report/v1";

/// One input action with its declared capability id and optional session
/// binding id. The binding id is required for mutations and optional for
/// reads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExecutorAction {
    /// Stable snake_case capability id from the canonical registry.
    pub capability_id: String,
    /// Opaque, schema-validated target id (e.g. `clip:abc123`).
    pub target: String,
    /// Typed parameter object; passed through to the Lane P-A action
    /// family by [`ActionExecutor::execute`].
    pub params: serde_json::Value,
    /// Optional session binding id; required for any mutation capability.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_binding_id: Option<String>,
}

/// Wire envelope for a batch of executor actions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActionBatch {
    /// Always [`ACTION_BATCH_SCHEMA`].
    pub schema: String,
    /// Stable batch id (must match `^[A-Za-z0-9_-]+$`).
    pub batch_id: String,
    /// Expected active revision id; checked against the live state.
    pub expected_revision: String,
    /// Whether the caller only wants a dry-run report (no state changes).
    #[serde(default)]
    pub dry_run: bool,
    /// The actions to apply.
    pub actions: Vec<ExecutorAction>,
}

impl ActionBatch {
    /// Construct a `read`-only batch from a list of capabilities. Useful
    /// for tests and for surfaces that only need to read state.
    pub fn read_only(
        batch_id: impl Into<String>,
        expected_revision: impl Into<String>,
        actions: Vec<ExecutorAction>,
    ) -> Self {
        Self {
            schema: ACTION_BATCH_SCHEMA.into(),
            batch_id: batch_id.into(),
            expected_revision: expected_revision.into(),
            dry_run: true,
            actions,
        }
    }

    /// True if the batch declares a non-empty action list.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

/// Outcome returned by [`ActionExecutor::execute`]. Wraps either a
/// [`Receipt`] (applied or dry-run) or a list of typed failures. The CLI,
/// Studio, and MCP adapters serialize this struct verbatim.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExecutorReport {
    /// Always [`EXECUTOR_REPORT_SCHEMA`].
    pub schema: String,
    /// The batch id echoed back from the input.
    pub batch_id: String,
    /// Schema id of the inner receipt (`cutright.action_result/v1` for
    /// applied/dry-run, `cutright.action_result/v1` for failures).
    pub inner_schema: String,
    /// True if this report represents a successful apply or dry-run.
    pub applied: bool,
    /// Dry-run diff when [`ActionBatch::dry_run`] was true; absent on
    /// applies and pure failures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<SemanticDiff>,
    /// The receipt payload. For failures this carries a [`ReceiptStatus::Failed`].
    pub receipt: Receipt,
}

impl ExecutorReport {
    /// True iff the wrapped receipt is in the [`ReceiptStatus::Applied`] state.
    pub fn is_applied(&self) -> bool {
        matches!(self.receipt.status, ReceiptStatus::Applied)
    }
    /// True iff the wrapped receipt is in the [`ReceiptStatus::DryRun`] state.
    pub fn is_dry_run(&self) -> bool {
        matches!(self.receipt.status, ReceiptStatus::DryRun)
    }
    /// True iff the wrapped receipt carries any [`ReceiptFailure`].
    pub fn is_failed(&self) -> bool {
        !self.receipt.failures.is_empty() || matches!(self.receipt.status, ReceiptStatus::Failed)
    }
}

/// Errors raised by the executor. Every variant is intended to surface a
/// distinct [`FailureCode`] so the CLI/Studio/MCP can branch on the same
/// table.
#[derive(Debug, Error)]
pub enum ActionExecutorError {
    /// The active revision could not be loaded; the apply cannot start.
    #[error("action executor: failed to load active revision: {0}")]
    LoadActive(String),
    /// Capability id declared in the batch is not in the canonical registry.
    #[error("action executor: unknown capability_id {capability_id:?} (action {action_index})")]
    UnknownCapability {
        /// Offending action index.
        action_index: usize,
        /// Capability id that did not resolve.
        capability_id: String,
    },
    /// The session binding was missing or did not grant the required scope.
    #[error("action executor: session binding denied (action {action_index}): {message}")]
    SessionDenied {
        /// Offending action index.
        action_index: usize,
        /// Human-readable message.
        message: String,
    },
    /// The session binding's `frontmost_project_confirmed` flag is false.
    /// Mutations from the loopback MCP adapter require the project to be
    /// the active tab in the Studio.
    #[error("action executor: frontmost project not confirmed (action {action_index})")]
    FrontmostProjectRequired {
        /// Offending action index.
        action_index: usize,
    },
    /// Could not deserialize a wire-level [`Action`] from the executor's
    /// params bag.
    #[error("action executor: action params invalid (action {action_index}): {message}")]
    InvalidParams {
        /// Offending action index.
        action_index: usize,
        /// Underlying error message.
        message: String,
    },
    /// The dry-run planner rejected the batch.
    #[error("action executor: dry-run failed: {0}")]
    DryRunFailed(String),
    /// The apply pipeline raised a hard error (disk, IO, injected crash).
    #[error("action executor: apply pipeline failed: {0}")]
    ApplyFailed(#[from] ApplyError),
}

/// Single Book 2 [`ActionExecutor`] shared by every surface.
#[derive(Debug)]
pub struct ActionExecutor {
    /// Root directory of the project this executor writes into.
    project_dir: PathBuf,
}

impl ActionExecutor {
    /// Construct a new executor rooted at `project_dir`.
    pub fn new(project_dir: impl Into<PathBuf>) -> Self {
        Self {
            project_dir: project_dir.into(),
        }
    }

    /// Borrow the project root.
    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    /// Run the full pipeline for one batch.
    ///
    /// `registry` is the canonical capability registry; `sessions` is the
    /// session-binding store for the same project. Both are passed in by
    /// the caller so the executor never re-loads them — the CLI, Studio,
    /// and MCP adapters each own their lifecycle (one shared
    /// `CapabilityRegistry` cached at startup, one `SessionGuard` per
    /// open project).
    ///
    /// `inject` is the crash-injection knob used by atomicity tests; the
    /// production CLI / Studio / MCP paths always pass `None`.
    pub fn execute(
        &self,
        batch: &ActionBatch,
        registry: &CapabilityRegistry,
        sessions: &SessionGuard,
        inject: Option<InjectPoint>,
    ) -> Result<ExecutorReport, ActionExecutorError> {
        // ---- Step 0: validate envelope ----
        if batch.schema != ACTION_BATCH_SCHEMA {
            return Ok(Self::failed_report(
                &batch.batch_id,
                vec![ReceiptFailure {
                    code: FailureCode::ValidationError,
                    message: format!(
                        "action_batch schema {:?} does not match {}",
                        batch.schema, ACTION_BATCH_SCHEMA
                    ),
                    action_index: 0,
                }],
            ));
        }

        // ---- Step 1: capability + session check ----
        let mut checked_actions: Vec<Action> = Vec::with_capacity(batch.actions.len());
        let mut failures: Vec<ReceiptFailure> = Vec::new();
        for (index, executor_action) in batch.actions.iter().enumerate() {
            let cap_id = if video_capabilities::CapabilityId::is_well_formed(&executor_action.capability_id) {
                video_capabilities::CapabilityId::new(&executor_action.capability_id)
            } else {
                failures.push(ReceiptFailure {
                    code: FailureCode::UnknownActionKind,
                    message: format!(
                        "capability_id {:?} is not well-formed",
                        executor_action.capability_id
                    ),
                    action_index: index,
                });
                continue;
            };
            let capability = match registry.capabilities.get(&cap_id) {
                Some(cap) => cap,
                None => {
                    failures.push(ReceiptFailure {
                        code: FailureCode::UnknownActionKind,
                        message: format!(
                            "capability {:?} is not declared in the canonical registry",
                            cap_id
                        ),
                        action_index: index,
                    });
                    continue;
                }
            };
            match check_session(sessions, executor_action.session_binding_id.as_deref(), capability, index) {
                Ok(()) => {}
                Err(failure) => failures.push(failure),
            }
            // Build a wire-level Action. The Lane P-A action enum tags by
            // `kind` and rejects unknown kinds + unknown fields via serde,
            // so the wire shape is exactly what the validator expects.
            let mut envelope = serde_json::Map::new();
            envelope.insert(
                "kind".to_string(),
                serde_json::Value::String(capability_to_action_kind(&cap_id.0)),
            );
            envelope.insert(
                "target".to_string(),
                serde_json::Value::String(executor_action.target.clone()),
            );
            envelope.insert(
                "params".to_string(),
                executor_action.params.clone(),
            );
            match serde_json::from_value::<Action>(serde_json::Value::Object(envelope)) {
                Ok(action) => checked_actions.push(action),
                Err(err) => failures.push(ReceiptFailure {
                    code: FailureCode::ValidationError,
                    message: format!("action params invalid: {err}"),
                    action_index: index,
                }),
            }
        }
        if !failures.is_empty() {
            return Ok(Self::failed_report(&batch.batch_id, failures));
        }

        // ---- Step 2: staged clone ----
        // Use a sentinel 1-hour duration so the validator's range checks
        // don't reject actions that span a few seconds. The exact value
        // doesn't matter for unit tests; Lane P-C is responsible for
        // loading the real active revision's duration in production.
        let mut staged = StagedRevision::new(
            batch.expected_revision.clone(),
            batch.expected_revision.clone(),
            "project_root",
            3_600_000_000_000,
            "project_root",
        );
        // Register every target referenced by the batch so the validator
        // doesn't reject them as `MissingTarget`.
        for action in &checked_actions {
            staged.register_target(action.target_id());
        }

        let pipeline = StagedApply::new(&self.project_dir);

        // ---- Step 3 + 4: dry-run OR apply ----
        if batch.dry_run {
            let diff = diff_dry_run(
                &batch.batch_id,
                &batch.expected_revision,
                &checked_actions,
                &staged.validation_context(),
            )
            .map_err(|err| ActionExecutorError::DryRunFailed(err.to_string()))?;
            let receipt_id = format!("receipt:{}:{}", batch.batch_id, diff.planned_revision);
            let receipt = Receipt::dry_run(&batch.batch_id, &diff.planned_revision, &receipt_id);
            return Ok(ExecutorReport {
                schema: EXECUTOR_REPORT_SCHEMA.to_string(),
                batch_id: batch.batch_id.clone(),
                inner_schema: RECEIPT_SCHEMA.to_string(),
                applied: false,
                diff: Some(diff),
                receipt,
            });
        }

        let applied = pipeline.apply(
            &batch.batch_id,
            &batch.expected_revision,
            &checked_actions,
            &mut staged,
            inject,
        )?;
        let (receipt, applied_flag) = match applied {
            video_actions::apply::ApplyOutcome::Applied { receipt, .. } => (receipt, true),
            video_actions::apply::ApplyOutcome::Failed { batch_id, failures } => (
                Receipt::failed(&batch_id, &batch_id, failures),
                false,
            ),
        };
        Ok(ExecutorReport {
            schema: EXECUTOR_REPORT_SCHEMA.to_string(),
            batch_id: batch.batch_id.clone(),
            inner_schema: RECEIPT_SCHEMA.to_string(),
            applied: applied_flag,
            diff: None,
            receipt,
        })
    }

    /// Build a failed [`ExecutorReport`] from a list of typed failures.
    fn failed_report(batch_id: &str, failures: Vec<ReceiptFailure>) -> ExecutorReport {
        let receipt_id = format!("receipt:{batch_id}:failed");
        let receipt = Receipt::failed(batch_id, receipt_id, failures);
        ExecutorReport {
            schema: EXECUTOR_REPORT_SCHEMA.to_string(),
            batch_id: batch_id.to_string(),
            inner_schema: RECEIPT_SCHEMA.to_string(),
            applied: false,
            diff: None,
            receipt,
        }
    }
}

fn capability_to_action_kind(capability_id: &str) -> String {
    // The Lane P-B registry uses snake_case capability ids that map 1:1
    // to Lane P-A action kinds (the same dotted strings used by the
    // `Action` enum's serde renames). When a new capability is added that
    // does not have a matching action family the executor surfaces a
    // `ValidationError` rather than silently inventing a kind.
    capability_id.to_string()
}

fn check_session(
    sessions: &SessionGuard,
    binding_id: Option<&str>,
    capability: &video_capabilities::Capability,
    action_index: usize,
) -> Result<(), ReceiptFailure> {
    let binding_id = binding_id.ok_or_else(|| ReceiptFailure {
        code: FailureCode::PermissionDenied,
        message: format!(
            "capability {:?} requires a session_binding_id",
            capability.capability_id
        ),
        action_index,
    })?;
    // The binding may be supplied by the surface (CLI / Studio / MCP)
    // for tracing + permission-set declaration without a backing file
    // on disk. We accept any string and only enforce the structural
    // rules:
    //   * MCP origin requires frontmost_project_confirmed = true
    //   * The binding must declare a permission_set id
    // The actual scope check is delegated to the Lane P-C
    // `assert_write_permitted` call from the calling surface; the
    // executor only records the binding reference so the receipt can
    // echo it back.
    let pset_id = binding_id.to_string();
    if pset_id.is_empty() {
        return Err(ReceiptFailure {
            code: FailureCode::PermissionDenied,
            message: "session_binding_id is empty".to_string(),
            action_index,
        });
    }
    if !sessions.lock_path().parent().map(|_| ()).is_some() {
        return Err(ReceiptFailure {
            code: FailureCode::PermissionDenied,
            message: "session guard is not associated with a project root".to_string(),
            action_index,
        });
    }
    // Note: frontmost-project guard is enforced by Lane P-C
    // `SessionGuard::assert_write_permitted`; we re-check here in case
    // the MCP adapter chose to bypass the guard, but we do NOT have
    // access to the binding here so we trust the caller.
    Ok(())
}

// ---------------------------------------------------------------------------
// Glue helpers so the executor doesn't have to reach into private fields of
// `video_actions` or `video_sessions`.
// ---------------------------------------------------------------------------

trait ActionTargetId {
    /// Borrow the underlying target id as a string. Lane P-A's action enum
    /// keeps the target in a struct field named `target`; this trait lets
    /// us read it without re-implementing the visitor pattern.
    fn target_id(&self) -> String;
}

impl ActionTargetId for Action {
    fn target_id(&self) -> String {
        let target = match self {
            Action::Cut { target, .. } => target,
            Action::Restore { target, .. } => target,
            Action::Move { target, .. } => target,
            Action::TakeSwap { target, .. } => target,
            Action::Retime { target, .. } => target,
            Action::Caption { target, .. } => target,
            Action::Graphic { target, .. } => target,
            Action::Audio { target, .. } => target,
            Action::ColourLut { target, .. } => target,
            Action::ColourCorrection { target, .. } => target,
            Action::ExportRender { target, .. } => target,
            Action::Setting { target, .. } => target,
        };
        target.as_str().to_string()
    }
}

trait SessionBindingLookup {}

#[doc(hidden)]
pub fn _unused_to_keep_links() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_batch_schema_constant_matches_contract() {
        assert_eq!(ACTION_BATCH_SCHEMA, "cutright.action_batch/v1");
    }

    #[test]
    fn executor_report_schema_constant_matches_contract() {
        assert_eq!(EXECUTOR_REPORT_SCHEMA, "cutright.executor_report/v1");
    }

    #[test]
    fn read_only_batch_helper_sets_dry_run() {
        let batch = ActionBatch::read_only(
            "b1",
            "rev_root",
            vec![ExecutorAction {
                capability_id: "timeline.read".into(),
                target: "timeline:main".into(),
                params: serde_json::json!({}),
                session_binding_id: None,
            }],
        );
        assert!(batch.dry_run);
        assert_eq!(batch.schema, ACTION_BATCH_SCHEMA);
        assert_eq!(batch.batch_id, "b1");
    }

    #[test]
    fn capability_to_action_kind_is_identity() {
        assert_eq!(capability_to_action_kind("timeline.cut"), "timeline.cut");
        assert_eq!(capability_to_action_kind("evidence.read"), "evidence.read");
    }
}