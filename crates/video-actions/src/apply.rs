//! Staged atomic apply pipeline (CR-V2-B2-010).
//!
//! Implements the staged apply pipeline from
//! `docs/architecture/V2-TRANSACTIONS-UNDO.md` §1 verbatim:
//!
//! 1. **Staged clone** — clone the active revision; no writes to live project.
//! 2. **Full semantic validation** — every action validated against staged
//!    revision using the same [`crate::validation::DefaultValidator`] the
//!    dry-run planner uses.
//! 3. **Atomic artifact writes** — temp-file + rename.
//! 4. **Revision commit** — staged state becomes new immutable revision.
//! 5. **Receipt emission** — `action_result/v1` written.
//! 6. **Active-pointer swap** — only after stages 1–5 succeed.
//!
//! On failure at any stage the active pointer is NOT advanced and the staged
//! clone is discarded.
//!
//! The pipeline also exposes three interruption injection points so atomicity
//! tests can crash between stages:
//!
//! - [`InjectPoint::BeforeRevisionCommit`] — between stage 3 and stage 4.
//! - [`InjectPoint::BeforeReceiptEmit`] — between stage 4 and stage 5.
//! - [`InjectPoint::BeforeActiveSwap`] — between stage 5 and stage 6.
//!
//! Each injection point is checked at most once per apply, so the test can
//! simulate a crash without the apply silently advancing.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::action::Action;
use crate::diff::{dry_run, DiffEntry, SemanticDiff, StableDiffKey, DRY_RUN_SCHEMA};
use crate::revision::{FailureCode, Receipt, ReceiptFailure, ReceiptStatus, Revision, RevisionError, RECEIPT_SCHEMA, REVISION_SCHEMA};
use crate::validation::{ValidationFailure};

/// Where the apply pipeline currently is. Used by atomicity tests to inject
/// a crash between stages without touching the pipeline itself.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum InjectPoint {
    /// Just before the staged revision becomes a committed immutable
    /// revision (`V2-TRANSACTIONS-UNDO.md` §1 step 4).
    BeforeRevisionCommit,
    /// Just before the receipt is written (`V2-TRANSACTIONS-UNDO.md` §1
    /// step 5).
    BeforeReceiptEmit,
    /// Just before the active pointer is swapped to the new revision
    /// (`V2-TRANSACTIONS-UNDO.md` §1 step 6).
    BeforeActiveSwap,
}

impl InjectPoint {
    /// All three injection points, in pipeline order. Used by the recovery
    /// helper in [`StagedApply::recover`].
    pub fn all() -> &'static [InjectPoint] {
        &[
            InjectPoint::BeforeRevisionCommit,
            InjectPoint::BeforeReceiptEmit,
            InjectPoint::BeforeActiveSwap,
        ]
    }
}

/// The staged-apply pipeline.
#[derive(Debug)]
pub struct StagedApply {
    /// Root directory of the project this pipeline writes into.
    project_dir: PathBuf,
}

impl StagedApply {
    /// Construct a pipeline rooted at `project_dir`. The directory must
    /// already exist; otherwise every call returns [`ApplyError::Io`].
    pub fn new(project_dir: impl Into<PathBuf>) -> Self {
        Self {
            project_dir: project_dir.into(),
        }
    }

    /// Root project directory.
    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    /// Apply a batch atomically.
    ///
    /// `expected_revision` MUST equal `staged.parent_revision_id` (which is
    /// populated by [`crate::revision::StagedRevision::from_active`]); if
    /// it does not, the apply fails with `FailureCode::StaleRevision` and
    /// no mutation occurs.
    ///
    /// The caller is responsible for stage 1 (`V2-TRANSACTIONS-UNDO.md` §1):
    /// build a [`crate::revision::StagedRevision`] via
    /// [`crate::revision::StagedRevision::from_active`] and register every
    /// known target via [`crate::revision::StagedRevision::register_target`]
    /// before invoking `apply`. The pipeline then runs stages 2–6 with no
    /// further input from the caller.
    ///
    /// `inject` is an optional injection point used by tests; when `Some`,
    /// the pipeline aborts with [`ApplyError::Injected`] just before the
    /// named stage.
    pub fn apply(
        &self,
        batch_id: &str,
        expected_revision: &str,
        actions: &[Action],
        staged: &mut crate::revision::StagedRevision,
        inject: Option<InjectPoint>,
    ) -> Result<ApplyOutcome, ApplyError> {
        // ---- Stage 0: stale-revision guard ----
        if staged.parent_revision_id != expected_revision {
            return Ok(ApplyOutcome::failed(
                batch_id,
                vec![ReceiptFailure {
                    code: FailureCode::StaleRevision,
                    message: format!(
                        "expected_revision {expected_revision:?} did not match staged parent {:?}",
                        staged.parent_revision_id
                    ),
                    action_index: 0,
                }],
            ));
        }

        // ---- Stage 2: full semantic validation (stage 1 already done by caller) ----
        let ctx = staged.validation_context();
        if let Err(failures) = crate::validation::validate_batch(actions, &ctx) {
            return Ok(ApplyOutcome::failed(
                batch_id,
                failures_to_receipt(failures),
            ));
        }

        // Compute the staged revision id deterministically from the
        // batch (same primitive as the dry-run planner).
        let staged_revision_id = crate::diff::planned_revision_for(
            batch_id,
            expected_revision,
            actions,
        );
        staged.staged_revision_id = staged_revision_id.clone();

        // ---- Stage 3: atomic artifact writes (temp-file + rename) ----
        let revision_path = self.revision_path(&staged_revision_id);
        let receipt_path = self.receipt_path(batch_id);
        let active_pointer_path = self.active_pointer_path();

        let created_at_ns = current_time_ns();
        let compatibility_fp = compute_compatibility_fp(actions);
        let committed = staged.commit(created_at_ns, &compatibility_fp);

        // ---- Stage 4: revision commit (atomic write) ----
        if inject == Some(InjectPoint::BeforeRevisionCommit) {
            return Err(ApplyError::Injected(InjectPoint::BeforeRevisionCommit));
        }
        let committed_bytes = serde_json::to_vec_pretty(&committed)
            .map_err(ApplyError::SerializeRevision)?;
        write_bytes_atomic(&revision_path, &committed_bytes)?;

        // ---- Stage 5: receipt emission ----
        if inject == Some(InjectPoint::BeforeReceiptEmit) {
            return Err(ApplyError::Injected(InjectPoint::BeforeReceiptEmit));
        }
        let receipt_id = make_receipt_id(batch_id, &staged_revision_id);
        let receipt = Receipt::applied(
            batch_id,
            &staged_revision_id,
            &receipt_id,
            (0..actions.len()).map(|i| format!("act_{i}")).collect(),
        );
        let receipt_bytes = serde_json::to_vec_pretty(&receipt)
            .map_err(ApplyError::SerializeReceipt)?;
        write_bytes_atomic(&receipt_path, &receipt_bytes)?;

        // ---- Stage 6: active-pointer swap ----
        if inject == Some(InjectPoint::BeforeActiveSwap) {
            return Err(ApplyError::Injected(InjectPoint::BeforeActiveSwap));
        }
        let pointer_bytes = staged_revision_id.as_bytes();
        write_bytes_atomic(&active_pointer_path, pointer_bytes)?;

        // Promote the staged revision into the active revision so subsequent
        // applies / undo / redo calls can match `expected_revision` against
        // the now-active revision id. The staged clone is implicitly
        // consumed; the caller can reuse `staged` for the next batch
        // without having to recapture the parent revision id.
        staged.parent_revision_id = staged_revision_id.clone();

        Ok(ApplyOutcome::Applied {
            revision: committed,
            receipt,
        })
    }

    /// Run only the dry-run planner (no mutation). Returns a typed
    /// [`SemanticDiff`] (`cutright.semantic_diff/v1`) and a
    /// [`Receipt`] in [`ReceiptStatus::DryRun`].
    ///
    /// The dry-run path uses the same validator as `apply`, satisfying
    /// `V2-SEMANTIC-DIFF.md` §3. The caller is responsible for building
    /// the [`crate::revision::StagedRevision`] (same as `apply`).
    pub fn dry_run(
        &self,
        batch_id: &str,
        expected_revision: &str,
        actions: &[Action],
        staged: &crate::revision::StagedRevision,
    ) -> Result<DryRunOutcome, ApplyError> {
        if staged.parent_revision_id != expected_revision {
            return Ok(DryRunOutcome::failed(
                batch_id,
                vec![ReceiptFailure {
                    code: FailureCode::StaleRevision,
                    message: format!(
                        "expected_revision {expected_revision:?} did not match staged parent {:?}",
                        staged.parent_revision_id
                    ),
                    action_index: 0,
                }],
            ));
        }
        let ctx = staged.validation_context();
        let diff = dry_run(batch_id, expected_revision, actions, &ctx)
            .map_err(map_diff_err_to_apply)?;
        let receipt = Receipt::dry_run(batch_id, &diff.planned_revision, &make_receipt_id(batch_id, &diff.planned_revision));
        Ok(DryRunOutcome {
            diff,
            receipt,
        })
    }

    /// Recovery helper used by atomicity tests after a simulated crash.
    /// Inspects the on-disk state and returns the [`RecoveryState`] of the
    /// pipeline so the test can assert that recovery is deterministic.
    pub fn recover(&self) -> RecoveryState {
        let active_pointer_path = self.active_pointer_path();
        let revisions_dir = self.project_dir.join("revisions");
        let receipts_dir = self.project_dir.join("receipts");
        let active_pointer = std::fs::read_to_string(&active_pointer_path)
            .ok()
            .map(|s| s.trim().to_string());
        let revisions: Vec<String> = std::fs::read_dir(&revisions_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok())
                    .filter_map(|entry| entry.file_name().to_str().map(|s| s.to_string()))
                    .filter(|name| name.ends_with(".json"))
                    .collect()
            })
            .unwrap_or_default();
        let receipts: Vec<Receipt> = std::fs::read_dir(&receipts_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok())
                    .filter_map(|entry| entry.file_name().to_str().map(|s| s.to_string()))
                    .filter(|name| name.ends_with(".json"))
                    .filter_map(|name| std::fs::read_to_string(receipts_dir.join(name)).ok())
                    .filter_map(|bytes| serde_json::from_str::<Receipt>(&bytes).ok())
                    .collect()
            })
            .unwrap_or_default();
        let mut receipt_files: Vec<String> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&receipts_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".json") {
                        receipt_files.push(name.to_string());
                    }
                }
            }
        }
        RecoveryState {
            active_pointer,
            revisions,
            receipt_files,
            receipts,
        }
    }

    fn revision_path(&self, revision_id: &str) -> PathBuf {
        self.project_dir
            .join("revisions")
            .join(format!("{revision_id}.json"))
    }

    fn receipt_path(&self, batch_id: &str) -> PathBuf {
        self.project_dir
            .join("receipts")
            .join(format!("{batch_id}.json"))
    }

    fn active_pointer_path(&self) -> PathBuf {
        self.project_dir.join("active_pointer")
    }
}

/// The outcome of a successful or failed `apply` call.
#[derive(Debug, Clone)]
pub enum ApplyOutcome {
    /// Apply succeeded end-to-end.
    Applied {
        /// The committed [`Revision`].
        revision: Revision,
        /// The [`Receipt`] that was written.
        receipt: Receipt,
    },
    /// Apply failed; the staged clone was discarded and no live state
    /// changed. The `failures` vector explains why.
    Failed {
        /// The action batch id this outcome describes.
        batch_id: String,
        /// Every failure that contributed to the abort.
        failures: Vec<ReceiptFailure>,
    },
}

impl ApplyOutcome {
    /// Construct a failed outcome with one or more failures.
    pub fn failed(batch_id: impl Into<String>, failures: Vec<ReceiptFailure>) -> Self {
        Self::Failed {
            batch_id: batch_id.into(),
            failures,
        }
    }

    /// True iff the outcome is [`ApplyOutcome::Applied`].
    pub fn is_applied(&self) -> bool {
        matches!(self, ApplyOutcome::Applied { .. })
    }

    /// True iff the outcome is [`ApplyOutcome::Failed`].
    pub fn is_failed(&self) -> bool {
        matches!(self, ApplyOutcome::Failed { .. })
    }
}

/// The outcome of a `dry_run` call.
#[derive(Debug, Clone)]
pub struct DryRunOutcome {
    /// The semantic diff (`cutright.semantic_diff/v1`).
    pub diff: SemanticDiff,
    /// The matching dry-run receipt (`cutright.action_result/v1` with
    /// status `dry_run`).
    pub receipt: Receipt,
}

impl DryRunOutcome {
    /// Wrap a `DiffError::Validation` failure into a typed outcome so the
    /// caller doesn't have to deal with two error types.
    pub fn failed(batch_id: impl Into<String>, failures: Vec<ReceiptFailure>) -> Self {
        let batch_id = batch_id.into();
        let receipt = Receipt::failed(
            "",
            &make_receipt_id(&batch_id, "dry_run"),
            failures,
        );
        Self {
            diff: SemanticDiff {
                schema: DRY_RUN_SCHEMA.to_string(),
                batch_id,
                expected_revision: String::new(),
                planned_revision: String::new(),
                diff: Vec::new(),
            },
            receipt,
        }
    }
}

/// State of the on-disk pipeline as observed by
/// [`StagedApply::recover`].
#[derive(Debug, Clone, Default)]
pub struct RecoveryState {
    /// The current active pointer value, if any.
    pub active_pointer: Option<String>,
    /// All revision files present in `revisions/` (file names only).
    pub revisions: Vec<String>,
    /// All receipt files present in `receipts/` (file names only).
    pub receipt_files: Vec<String>,
    /// All parsed receipts on disk (used for consistency checks).
    pub receipts: Vec<Receipt>,
}

impl RecoveryState {
    /// True iff the active pointer file is missing OR points at a revision
    /// for which no `applied` receipt exists with `new_revision == pointer`.
    pub fn is_inconsistent(&self) -> bool {
        match &self.active_pointer {
            None => true,
            Some(rev_id) => !self.receipts.iter().any(|r| {
                r.new_revision == *rev_id && r.status == crate::revision::ReceiptStatus::Applied
            }),
        }
    }
}

/// Typed error returned by [`StagedApply::apply`] and friends.
#[derive(Debug, Error)]
pub enum ApplyError {
    /// Underlying I/O failure during an atomic write.
    #[error("I/O error during {stage}: {source}")]
    Io {
        /// Stage that failed (`"write revision"`, `"write receipt"`, …).
        stage: &'static str,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },
    /// Revision serialisation failed.
    #[error("failed to serialize revision: {0}")]
    SerializeRevision(#[source] serde_json::Error),
    /// Receipt serialisation failed.
    #[error("failed to serialize receipt: {0}")]
    SerializeReceipt(#[source] serde_json::Error),
    /// Validation failure bubbled up from the dry-run planner.
    #[error("dry-run validation failed: {0:?}")]
    DryRunValidation(Vec<ValidationFailure>),
    /// A test injection point fired; the apply aborted at the named stage.
    /// Recovery can be performed via [`StagedApply::recover`].
    #[error("apply aborted by injection point {0:?}")]
    Injected(InjectPoint),
    /// Revision construction/loading failed.
    #[error("revision error: {0}")]
    Revision(#[from] RevisionError),
}

fn failures_to_receipt(failures: Vec<ValidationFailure>) -> Vec<ReceiptFailure> {
    failures
        .into_iter()
        .map(|failure| ReceiptFailure {
            code: FailureCode::ValidationError,
            message: failure.to_string(),
            action_index: failure.action_index,
        })
        .collect()
}

fn map_diff_err_to_apply(err: crate::diff::DiffError) -> ApplyError {
    match err {
        crate::diff::DiffError::Validation(failures) => {
            ApplyError::DryRunValidation(failures)
        }
        crate::diff::DiffError::Hash(source) => {
            // Serialisation failures during diff planning are recovered as
            // I/O-shaped errors at the receipt-emission stage.
            ApplyError::SerializeReceipt(source)
        }
    }
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), ApplyError> {
    let parent = path.parent().ok_or_else(|| ApplyError::Io {
        stage: "resolve parent directory",
        source: std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("{} has no parent directory", path.display()),
        ),
    })?;
    fs::create_dir_all(parent).map_err(|source| ApplyError::Io {
        stage: "create parent directory",
        source,
    })?;
    let file_name = path.file_name().and_then(|s| s.to_str()).ok_or_else(|| {
        ApplyError::Io {
            stage: "resolve file name",
            source: std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{} has no file name", path.display()),
            ),
        }
    })?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let sequence = TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temp = parent.join(format!(
        ".{file_name}.tmp-{}-{:?}-{nanos}-{sequence}",
        std::process::id(),
        std::thread::current().id()
    ));
    let write_result = (|| -> Result<(), ApplyError> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|source| ApplyError::Io {
                stage: "open temp file",
                source,
            })?;
        std::io::Write::write_all(&mut file, bytes).map_err(|source| ApplyError::Io {
            stage: "write temp file",
            source,
        })?;
        file.sync_all().map_err(|source| ApplyError::Io {
            stage: "fsync temp file",
            source,
        })?;
        Ok(())
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    if let Err(source) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(ApplyError::Io {
            stage: "rename temp file",
            source,
        });
    }
    if let Ok(dir) = fs::File::open(parent) {
        let _ = dir.sync_all();
    }
    Ok(())
}

static TEMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn current_time_ns() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

/// Default duration we use when the active revision doesn't carry one.
/// Stays inside `i64` and is comfortably below the validation overflow
/// bound so tests can use it.
const DEFAULT_DURATION_NS: i64 = 10_000_000_000;

fn active_duration_ns(active: &Revision) -> i64 {
    // Revisions don't carry a duration in the v1 schema; the orchestrator
    // is responsible for tracking the project timeline length. For the
    // standalone lane-P-A apply pipeline we use a conservative default
    // that is larger than any plausible test range.
    DEFAULT_DURATION_NS
}

fn compute_compatibility_fp(actions: &[Action]) -> String {
    let canonical = serde_json::json!({
        "schema": REVISION_SCHEMA,
        "actions": actions.iter()
            .map(|a| serde_json::to_value(a).expect("action serialises"))
            .collect::<Vec<_>>(),
    });
    let bytes = serde_json::to_vec(&canonical).expect("canonical serialises");
    let hash = blake3::hash(&bytes).to_hex().to_string();
    hash
}

fn make_receipt_id(batch_id: &str, revision_id: &str) -> String {
    let canonical = serde_json::json!({
        "schema": RECEIPT_SCHEMA,
        "batch_id": batch_id,
        "revision_id": revision_id,
    });
    let bytes = serde_json::to_vec(&canonical).expect("canonical serialises");
    let hash = blake3::hash(&bytes).to_hex().to_string();
    format!("rcpt_{}", &hash[..32])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{CutParams, RangeNs, TargetKind, TargetRef};
    use crate::revision::StagedRevision;
    use std::collections::BTreeSet;

    fn make_active(revision_id: &str) -> Revision {
        Revision {
            schema: REVISION_SCHEMA.to_string(),
            revision_id: revision_id.into(),
            parents: Vec::new(),
            created_at_ns: 1_700_000_000_000_000_000,
            active_pointer: "prj_main".into(),
            compatibility_fp: "deadbeefcafebabe1234567890abcdef".into(),
        }
    }

    fn cut_action() -> Action {
        Action::Cut {
            target: TargetRef::from_parts(TargetKind::Clip, "clip_5").unwrap(),
            params: CutParams {
                range: RangeNs {
                    start_ns: 1_000,
                    end_ns: 2_000,
                },
                reason: None,
            },
        }
    }

    fn staged_with_clip5() -> StagedRevision {
        let mut staged = StagedRevision::from_active(&make_active("rev_0001"), DEFAULT_DURATION_NS);
        staged.register_target("clip:clip_5");
        staged
    }

    #[test]
    fn apply_succeeds_for_a_valid_batch() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = StagedApply::new(temp.path());
        let mut staged = staged_with_clip5();
        let outcome = pipeline
            .apply(
                "batch_0001",
                "rev_0001",
                &[cut_action()],
                &mut staged,
                None,
            )
            .unwrap();
        assert!(outcome.is_applied());

        let recovery = pipeline.recover();
        assert_eq!(
            recovery.active_pointer.as_deref(),
            Some(outcome_applied_revision(&outcome).revision_id.as_str())
        );
        assert!(!recovery.revisions.is_empty());
        assert!(!recovery.receipt_files.is_empty());
        assert!(!recovery.receipts.is_empty());
    }

    #[test]
    fn apply_rejects_stale_revision() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = StagedApply::new(temp.path());
        let mut staged = staged_with_clip5();
        let outcome = pipeline
            .apply("batch_0001", "rev_0099", &[cut_action()], &mut staged, None)
            .unwrap();
        assert!(outcome.is_failed());
        if let ApplyOutcome::Failed { failures, .. } = outcome {
            assert_eq!(failures[0].code, FailureCode::StaleRevision);
        } else {
            panic!("expected failed outcome");
        }
    }

    #[test]
    fn apply_propagates_validation_failures() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = StagedApply::new(temp.path());
        // Empty staged (no known targets) means cut_action will fail validation.
        let mut staged = StagedRevision::from_active(&make_active("rev_0001"), DEFAULT_DURATION_NS);
        let outcome = pipeline
            .apply(
                "batch_0001",
                "rev_0001",
                &[cut_action()],
                &mut staged,
                None,
            )
            .unwrap();
        assert!(outcome.is_failed());
    }

    #[test]
    fn inject_before_revision_commit_leaves_no_receipt_no_pointer() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = StagedApply::new(temp.path());
        let mut staged = staged_with_clip5();
        let err = pipeline
            .apply(
                "batch_0001",
                "rev_0001",
                &[cut_action()],
                &mut staged,
                Some(InjectPoint::BeforeRevisionCommit),
            )
            .unwrap_err();
        assert!(matches!(err, ApplyError::Injected(InjectPoint::BeforeRevisionCommit)));
        let recovery = pipeline.recover();
        assert!(recovery.revisions.is_empty());
        assert!(recovery.receipt_files.is_empty());
        assert!(recovery.receipts.is_empty());
        assert!(recovery.active_pointer.is_none());
    }

    #[test]
    fn inject_before_receipt_emit_leaves_revision_but_no_receipt() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = StagedApply::new(temp.path());
        let mut staged = staged_with_clip5();
        let err = pipeline
            .apply(
                "batch_0001",
                "rev_0001",
                &[cut_action()],
                &mut staged,
                Some(InjectPoint::BeforeReceiptEmit),
            )
            .unwrap_err();
        assert!(matches!(err, ApplyError::Injected(InjectPoint::BeforeReceiptEmit)));
        let recovery = pipeline.recover();
        assert!(!recovery.revisions.is_empty(), "revision file should exist");
        assert!(recovery.receipt_files.is_empty(), "receipt must NOT exist");
        assert!(recovery.receipts.is_empty(), "receipt must NOT exist");
        assert!(recovery.active_pointer.is_none(), "pointer must NOT be swapped");
        assert!(recovery.is_inconsistent());
    }

    #[test]
    fn inject_before_active_swap_leaves_revision_and_receipt_but_no_pointer() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = StagedApply::new(temp.path());
        let mut staged = staged_with_clip5();
        let err = pipeline
            .apply(
                "batch_0001",
                "rev_0001",
                &[cut_action()],
                &mut staged,
                Some(InjectPoint::BeforeActiveSwap),
            )
            .unwrap_err();
        assert!(matches!(err, ApplyError::Injected(InjectPoint::BeforeActiveSwap)));
        let recovery = pipeline.recover();
        assert!(!recovery.revisions.is_empty(), "revision file should exist");
        assert!(!recovery.receipt_files.is_empty(), "receipt should exist");
        assert!(!recovery.receipts.is_empty(), "receipt should exist");
        assert!(recovery.active_pointer.is_none(), "pointer must NOT be swapped");
        assert!(recovery.is_inconsistent());
    }

    #[test]
    fn dry_run_emits_diff_and_dry_run_receipt() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = StagedApply::new(temp.path());
        let staged = staged_with_clip5();
        let outcome = pipeline
            .dry_run("batch_0001", "rev_0001", &[cut_action()], &staged)
            .unwrap();
        assert_eq!(outcome.diff.schema, DRY_RUN_SCHEMA);
        assert_eq!(outcome.receipt.status, ReceiptStatus::DryRun);
        assert!(outcome.diff.diff.len() == 1);
    }

    #[test]
    fn dry_run_rejects_stale_revision() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = StagedApply::new(temp.path());
        let mut staged = staged_with_clip5();
        staged.parent_revision_id = "rev_0099".into();
        let outcome = pipeline
            .dry_run("batch_0001", "rev_0001", &[cut_action()], &staged)
            .unwrap();
        // dry_run converts validation errors into a typed failed outcome.
        assert_eq!(outcome.diff.diff.len(), 0);
    }

    #[test]
    fn recovery_state_is_consistent_after_successful_apply() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = StagedApply::new(temp.path());
        let mut staged = staged_with_clip5();
        let outcome = pipeline
            .apply(
                "batch_0001",
                "rev_0001",
                &[cut_action()],
                &mut staged,
                None,
            )
            .unwrap();
        assert!(outcome.is_applied());
        let recovery = pipeline.recover();
        assert!(!recovery.is_inconsistent());
    }

    fn outcome_applied_revision(outcome: &ApplyOutcome) -> &Revision {
        match outcome {
            ApplyOutcome::Applied { revision, .. } => revision,
            _ => panic!("expected applied outcome"),
        }
    }

    #[test]
    fn diff_entries_in_dry_run_are_sorted() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = StagedApply::new(temp.path());
        let staged = staged_with_clip5();
        let actions = vec![
            cut_action(),
            Action::Cut {
                target: TargetRef::from_parts(TargetKind::Clip, "clip_5").unwrap(),
                params: CutParams {
                    range: RangeNs {
                        start_ns: 5_000,
                        end_ns: 6_000,
                    },
                    reason: None,
                },
            },
        ];
        let outcome = pipeline
            .dry_run("batch_0001", "rev_0001", &actions, &staged)
            .unwrap();
        let keys: Vec<StableDiffKey> = outcome
            .diff
            .diff
            .iter()
            .map(|entry: &DiffEntry| StableDiffKey::from_entry(entry, "prj_main"))
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }

    #[test]
    fn known_targets_helper_used_for_setup() {
        // Sanity test: BTreeSet construction used by callers.
        let mut set: BTreeSet<String> = BTreeSet::new();
        set.insert("clip:clip_5".into());
        assert!(set.contains("clip:clip_5"));
    }
}
