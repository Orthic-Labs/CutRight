//! Autonomy state, advancement, and automatic demotion.
//!
//! Advancement: `thresholds_met && user_approval_present`.
//! Demotion: any regression trigger.
//! No code path self-approves advancement. The user approval timestamp is
//! written only by the Studio review action.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::decision::FormatKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyMode {
    Reviewed,
    ReviewLight,
    Autonomous,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutonomyMetrics {
    pub benchmark_pass: bool,
    pub user_approval_count: u32,
    pub regression_count: u32,
    pub critic_disagreement: u32,
    pub integrity_failures: u32,
    pub qa_failures: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyTransitionReason {
    ThresholdsMetUserApproval,
    RejectedFinal,
    UnresolvedEscalation,
    BenchmarkRegression,
    CriticDisagreement,
    IntegrityFailure,
    IncompatiblePackChange,
    FirstSeen,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutonomyTransition {
    pub from: AutonomyMode,
    pub to: AutonomyMode,
    pub reason: AutonomyTransitionReason,
    pub audit_id: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutonomyState {
    pub schema_version: String,
    pub autonomy_id: String,
    pub format: FormatKey,
    pub mode: AutonomyMode,
    pub compatible_pack_set: Vec<String>,
    pub benchmark_profile: String,
    pub sample_count: u32,
    pub metrics: AutonomyMetrics,
    pub demoted: bool,
    pub last_user_approval: Option<DateTime<Utc>>,
    pub transition_history: Vec<AutonomyTransition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomyAdvancementPredicate {
    pub min_user_approval_count: u32,
    pub require_benchmark_pass: bool,
    pub require_user_approval_timestamp: bool,
}

impl Default for AutonomyAdvancementPredicate {
    fn default() -> Self {
        Self {
            min_user_approval_count: 3,
            require_benchmark_pass: true,
            require_user_approval_timestamp: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomyDemotionPredicate {
    pub max_regression_count: u32,
    pub max_critic_disagreement: u32,
    pub max_integrity_failures: u32,
    pub max_qa_failures: u32,
}

impl Default for AutonomyDemotionPredicate {
    fn default() -> Self {
        Self {
            max_regression_count: 0,
            max_critic_disagreement: 0,
            max_integrity_failures: 0,
            max_qa_failures: 0,
        }
    }
}

pub fn initial_state(
    format: FormatKey,
    compatible_pack_set: Vec<String>,
    benchmark_profile: String,
) -> AutonomyState {
    let now = Utc::now();
    let audit_format = format.clone();
    AutonomyState {
        schema_version: "v2".to_string(),
        autonomy_id: compute_autonomy_id(&format, &compatible_pack_set, &benchmark_profile),
        format,
        mode: AutonomyMode::Reviewed,
        compatible_pack_set,
        benchmark_profile,
        sample_count: 0,
        metrics: AutonomyMetrics {
            benchmark_pass: false,
            user_approval_count: 0,
            regression_count: 0,
            critic_disagreement: 0,
            integrity_failures: 0,
            qa_failures: 0,
        },
        demoted: false,
        last_user_approval: None,
        transition_history: vec![AutonomyTransition {
            from: AutonomyMode::Reviewed,
            to: AutonomyMode::Reviewed,
            reason: AutonomyTransitionReason::FirstSeen,
            audit_id: compute_audit_id(&audit_format, "first_seen"),
            at: now,
        }],
    }
}

/// Advance a state. Returns the new state. The user approval timestamp must
/// be supplied by the caller; the function never self-approves.
pub fn advance(
    state: AutonomyState,
    predicate: AutonomyAdvancementPredicate,
    user_approval: DateTime<Utc>,
    audit_id_seed: &str,
) -> AutonomyState {
    let mut next = state.clone();
    let approved_ok = if predicate.require_user_approval_timestamp {
        true
    } else {
        false
    };
    let passes_benchmark = if predicate.require_benchmark_pass {
        state.metrics.benchmark_pass
    } else {
        true
    };
    let counts_ok = state.metrics.user_approval_count >= predicate.min_user_approval_count;
    if approved_ok && passes_benchmark && counts_ok {
        next.mode = AutonomyMode::Autonomous;
        next.last_user_approval = Some(user_approval);
        next.transition_history.push(AutonomyTransition {
            from: state.mode,
            to: AutonomyMode::Autonomous,
            reason: AutonomyTransitionReason::ThresholdsMetUserApproval,
            audit_id: compute_audit_id(&state.format, audit_id_seed),
            at: Utc::now(),
        });
    }
    next
}

/// Apply a demotion trigger. The mode returns to `Reviewed`.
pub fn demote(
    state: AutonomyState,
    reason: AutonomyTransitionReason,
    audit_id_seed: &str,
) -> AutonomyState {
    let mut next = state.clone();
    next.mode = AutonomyMode::Reviewed;
    next.demoted = true;
    next.transition_history.push(AutonomyTransition {
        from: state.mode,
        to: AutonomyMode::Reviewed,
        reason,
        audit_id: compute_audit_id(&state.format, audit_id_seed),
        at: Utc::now(),
    });
    next
}

/// Returns the audit history of the state.
pub fn transitions(state: &AutonomyState) -> &[AutonomyTransition] {
    &state.transition_history
}

/// Returns true when the state has any regression trigger.
pub fn has_regression_trigger(state: &AutonomyState, predicate: AutonomyDemotionPredicate) -> bool {
    state.metrics.regression_count > predicate.max_regression_count
        || state.metrics.critic_disagreement > predicate.max_critic_disagreement
        || state.metrics.integrity_failures > predicate.max_integrity_failures
        || state.metrics.qa_failures > predicate.max_qa_failures
}

fn compute_autonomy_id(format: &FormatKey, packs: &[String], profile: &str) -> String {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(format.content_type.as_bytes());
    hasher.update(&[0u8]);
    hasher.update(format.platform.as_bytes());
    hasher.update(&[0u8]);
    hasher.update(format.variant.as_bytes());
    hasher.update(&[0u8]);
    for p in packs {
        hasher.update(p.as_bytes());
        hasher.update(&[0u8]);
    }
    hasher.update(profile.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest.as_bytes() {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

fn compute_audit_id(format: &FormatKey, seed: &str) -> String {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(format.content_type.as_bytes());
    hasher.update(&[0u8]);
    hasher.update(seed.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest.as_bytes() {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}
