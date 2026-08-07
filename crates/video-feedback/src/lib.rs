//! video-feedback — CutRight v2 hash-bound decision records, evidence-backed
//! preference learning, applied format profiles, autonomy state, and
//! autonomous orchestration.
//!
//! This is the canonical implementation of the v2 feedback loop. It writes
//! to no external service and depends on no network. Decision records are
//! append-only and hash-chained; malformed records are retained but flagged.

pub mod autonomy;
pub mod decision;
pub mod distributions;
pub mod learn;
pub mod profile;

pub use autonomy::{
    advance, demote, has_regression_trigger, initial_state, transitions,
    AutonomyAdvancementPredicate, AutonomyDemotionPredicate, AutonomyMetrics, AutonomyMode,
    AutonomyState, AutonomyTransition, AutonomyTransitionReason,
};
pub use decision::{
    append_record, compute_record_hash, hash_chain_zero, record_hash_mismatch, DecisionAction,
    DecisionAxis, DecisionReason, DecisionRecord, DecisionTarget, FormatKey, ReviewMode,
    SessionOrigin, UserOrigin,
};
pub use distributions::{AxisDistribution, DistributionSample};
pub use learn::{
    compute_preference, compute_recommendation, estimate_is_supported, EstimateScope,
    InsufficientReason, PreferenceEstimate, Recommendation, ScopedEstimate,
};
pub use profile::{
    apply_profile, approve_profile, profile_compatibility_mismatch, FormatProfile,
    FormatProfileValues, ProfileApprovedBy, ProfileCompatibility, ProfileCompatibilityMismatch,
    ProfileVersion,
};
