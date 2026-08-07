//! Lane P-A: typed `Action` enum, validation, dry-run, atomic apply, and undo.
//!
//! Implements the v2 contract frozen by `27d5682..50f76d5`:
//! - `docs/architecture/V2-CAPABILITY-ACTION-CONTRACT.md` — action kinds
//! - `docs/architecture/V2-IDENTITY-TIME-REVISION.md` — stable IDs, rational time
//! - `docs/architecture/V2-TRANSACTIONS-UNDO.md` — staged apply, inverse actions
//! - `docs/architecture/V2-SEMANTIC-DIFF.md` — semantic dry-run diff
//! - `docs/architecture/V2-CRATE-DAG.md` — Lane P-A ownership of `video-actions`
//!
//! Per `V2-CRATE-DAG.md` rule 2 ("Lower never depends on higher"), this crate
//! depends only on `video-core` and never on `video-state`, `video-capabilities`,
//! `video-project`, `video-cli`, or `apps/studio`.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod action;
pub mod apply;
pub mod diff;
pub mod revision;
pub mod validation;

pub use action::{
    Action, AudioParams, CaptionParams, ColourCorrectionParams, ColourLutParams, CutParams,
    ExportRenderParams, GraphicParams, MoveParams, ParamError, RetimeParams, RestoreParams,
    SettingParams, TakeSwapParams, TargetRef, TargetRefError, ACTION_KINDS,
};
pub use apply::{ApplyError, ApplyOutcome, DryRunOutcome, InjectPoint, RecoveryState, StagedApply};
pub use diff::{dry_run, DiffEntry, DiffError, DiffRange, SemanticDiff, StableDiffKey, DRY_RUN_SCHEMA};
pub use revision::{
    FailureCode, Receipt, ReceiptFailure, ReceiptStatus, Revision, RevisionError, StagedRevision,
    RECEIPT_SCHEMA, REVISION_SCHEMA,
};
pub use validation::{
    validate_batch, DefaultValidator, ValidationContext, ValidationError, ValidationFailure,
    Validator,
};
