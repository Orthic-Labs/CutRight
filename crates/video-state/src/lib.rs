//! Lane P-C: immutable project revision storage, append-only logs, migrations.
//!
//! Implements the v2 contract frozen by `27d5682..50f76d5`:
//! - `docs/architecture/V2-IDENTITY-TIME-REVISION.md` — immutable revisions
//! - `docs/architecture/V2-TRANSACTIONS-UNDO.md` — staged apply, active pointer
//! - `docs/architecture/V2-CRATE-DAG.md` — Lane P-C ownership of `video-state`
//!
//! Per `V2-CRATE-DAG.md` rule 2 ("Lower never depends on higher"), this crate
//! depends only on `video-core` and never on `video-actions`,
//! `video-capabilities`, `video-project`, `video-cli`, or `apps/studio`.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod log;
pub mod revision_store;

pub use log::{LogError, LogKind, LogRecord, LogVerifier, LogVerifierOutcome, LogVerifierReport};
pub use revision_store::{Revision, RevisionError, RevisionId, RevisionStore, StagedState};
