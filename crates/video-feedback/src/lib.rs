//! video-feedback — CutRight v2 hash-bound decision records, evidence-backed
//! preference learning, applied format profiles, autonomy state, and
//! autonomous orchestration.
//!
//! This is the canonical implementation of the v2 feedback loop. It writes
//! to no external service and depends on no network. Decision records are
//! append-only and hash-chained; malformed records are retained but flagged.

pub mod decision;
pub mod distributions;

pub use decision::{DecisionRecord, DecisionTarget, DecisionAction, DecisionReason, DecisionAxis,
                    FormatKey, UserOrigin, SessionOrigin, ReviewMode, append_record,
                    compute_record_hash, hash_chain_zero, record_hash_mismatch};
pub use distributions::{AxisDistribution, DistributionSample};

pub mod learn {
    //! Stub for B7-008. The actual implementation lives in
    //! `crates/video-feedback/src/learn.rs` once it is filled.
    pub struct PreferenceEstimate;
}

pub mod profile {
    //! Stub for B7-009.
    pub struct FormatProfile;
}

pub mod autonomy {
    //! Stub for B7-010.
    pub struct AutonomyState;
}
