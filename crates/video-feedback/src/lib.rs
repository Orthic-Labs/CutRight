//! video-feedback — CutRight v2 hash-bound decision records, evidence-backed
//! preference learning, applied format profiles, autonomy state, and
//! autonomous orchestration.
//!
//! This is the canonical implementation of the v2 feedback loop. It writes
//! to no external service and depends on no network. Decision records are
//! append-only and hash-chained; malformed records are retained but flagged.

pub mod decision;
pub mod distributions;
pub mod learn;
pub mod profile;
pub mod autonomy;

pub use decision::{DecisionRecord, DecisionTarget, DecisionAction, DecisionReason, DecisionAxis,
                    FormatKey, UserOrigin, SessionOrigin, ReviewMode, append_record,
                    compute_record_hash, hash_chain_zero, record_hash_mismatch};
pub use distributions::{AxisDistribution, DistributionSample};
pub use learn::{PreferenceEstimate, InsufficientReason, ScopedEstimate, Recommendation,
                  EstimateScope, compute_preference, compute_recommendation,
                  estimate_is_supported};
pub use profile::{FormatProfile, FormatProfileValues, ProfileCompatibility, ProfileVersion,
                   ProfileApprovedBy, ProfileCompatibilityMismatch, approve_profile,
                   apply_profile, profile_compatibility_mismatch};
pub use autonomy::{AutonomyState, AutonomyMode, AutonomyMetrics, AutonomyTransition,
                     AutonomyTransitionReason, AutonomyAdvancementPredicate,
                     AutonomyDemotionPredicate, initial_state, advance, demote, transitions,
                     has_regression_trigger};
