//! Lane P-C: project write locks, session bindings, frontmost-project guard.
//!
//! Implements the v2 contract frozen by `27d5682..50f76d5`:
//! - `docs/security/V2-ACTION-PERMISSIONS.md` — session bindings, frontmost
//! - `docs/architecture/V2-CRATE-DAG.md` — Lane P-C ownership of `video-sessions`
//!
//! Per `V2-CRATE-DAG.md` rule 2 ("Lower never depends on higher"), this crate
//! depends only on `video-core` and never on `video-actions`,
//! `video-capabilities`, `video-project`, `video-cli`, or `apps/studio`.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod binding;

pub use binding::{
    SessionBinding, SessionBindingError, SessionGuard, SessionGuardError, SessionOrigin,
};
