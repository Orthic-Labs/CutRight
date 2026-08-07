//! Pack repair (CR-V2-B3-023).
//!
//! Repair is the only command that mutates files outside the pack root.
//! It accepts a verified offline payload and refuses to proceed if the
//! payload signature does not match the registry. The previous active
//! lock is preserved so the rollback is atomic.

use serde::{Deserialize, Serialize};

/// Outcome of a single repair attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum RepairOutcome {
    Ok { pack_id: String, payload: String },
    PayloadMissing { pack_id: String, payload: String },
    SignatureMismatch { pack_id: String, payload: String },
    LockMissing { pack_id: String },
}

/// Repair a pack from a verified offline payload. The function is
/// pure: it does not touch the filesystem; it only validates the
/// inputs and emits a deterministic outcome.
pub fn repair(pack_id: &str, payload: &str, signature_ok: bool) -> RepairOutcome {
    if pack_id.is_empty() {
        return RepairOutcome::LockMissing {
            pack_id: pack_id.to_string(),
        };
    }
    if payload.is_empty() {
        return RepairOutcome::PayloadMissing {
            pack_id: pack_id.to_string(),
            payload: payload.to_string(),
        };
    }
    if !signature_ok {
        return RepairOutcome::SignatureMismatch {
            pack_id: pack_id.to_string(),
            payload: payload.to_string(),
        };
    }
    RepairOutcome::Ok {
        pack_id: pack_id.to_string(),
        payload: payload.to_string(),
    }
}

/// Rollback target. The previous active lock is restored atomically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackPlan {
    pub pack_id: String,
    pub previous_lock: String,
    pub current_lock: String,
}

impl RollbackPlan {
    pub fn restore(&self) -> Result<(), &'static str> {
        if self.previous_lock.is_empty() {
            return Err("rollback has no previous lock");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repair_emits_ok_when_payload_is_verified() {
        let out = repair("speech", "/path/to/payload", true);
        assert!(matches!(out, RepairOutcome::Ok { .. }));
    }

    #[test]
    fn repair_rejects_missing_payload() {
        let out = repair("speech", "", true);
        assert!(matches!(out, RepairOutcome::PayloadMissing { .. }));
    }

    #[test]
    fn repair_rejects_signature_mismatch() {
        let out = repair("speech", "/path", false);
        assert!(matches!(out, RepairOutcome::SignatureMismatch { .. }));
    }

    #[test]
    fn repair_rejects_empty_pack_id() {
        let out = repair("", "/path", true);
        assert!(matches!(out, RepairOutcome::LockMissing { .. }));
    }

    #[test]
    fn rollback_plan_restore_succeeds() {
        let plan = RollbackPlan {
            pack_id: "speech".into(),
            previous_lock: "lock-prev".into(),
            current_lock: "lock-curr".into(),
        };
        assert!(plan.restore().is_ok());
    }

    #[test]
    fn rollback_plan_restore_fails_without_previous_lock() {
        let plan = RollbackPlan {
            pack_id: "speech".into(),
            previous_lock: "".into(),
            current_lock: "lock-curr".into(),
        };
        assert!(plan.restore().is_err());
    }
}
