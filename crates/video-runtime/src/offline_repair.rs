//! Offline pack repair helpers.
//!
//! The crate exposes [`offline_repair`] which produces an
//! [`OfflineRepairOutcome`] describing what would change on the next pack
//! swap. Network and remote sources are not allowed.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfflineRepairAction {
    Verify,
    Repair,
    Activate,
    Rollback,
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineRepairOutcome {
    pub action: OfflineRepairAction,
    pub keep_existing: bool,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfflineRepairRequest {
    pub pack_id: String,
    pub requested_action: OfflineRepairAction,
    pub source: String,
    pub payload_signature_valid: bool,
    pub current_active: Option<String>,
}

pub fn offline_repair(req: &OfflineRepairRequest) -> OfflineRepairOutcome {
    if req.source != "local_verified_bundle" {
        return OfflineRepairOutcome {
            action: OfflineRepairAction::Noop,
            keep_existing: true,
            source: req.source.clone(),
            message: "refusing non-local source; offline v2 requires a local verified bundle"
                .to_string(),
        };
    }
    let action = match req.requested_action {
        OfflineRepairAction::Noop => OfflineRepairAction::Noop,
        OfflineRepairAction::Verify => {
            if req.payload_signature_valid {
                OfflineRepairAction::Verify
            } else {
                OfflineRepairAction::Noop
            }
        }
        OfflineRepairAction::Repair | OfflineRepairAction::Activate => {
            if req.payload_signature_valid {
                req.requested_action.clone()
            } else {
                OfflineRepairAction::Noop
            }
        }
        OfflineRepairAction::Rollback => OfflineRepairAction::Rollback,
    };
    let keep_existing = matches!(
        action,
        OfflineRepairAction::Noop | OfflineRepairAction::Rollback
    ) && req.current_active.is_some();
    OfflineRepairOutcome {
        action,
        keep_existing,
        source: req.source.clone(),
        message: format!("offline repair acted on pack {}", req.pack_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_non_local_source() {
        let r = offline_repair(&OfflineRepairRequest {
            pack_id: "p1".into(),
            requested_action: OfflineRepairAction::Verify,
            source: "remote".into(),
            payload_signature_valid: true,
            current_active: Some("p0".into()),
        });
        assert_eq!(r.action, OfflineRepairAction::Noop);
        assert!(r.keep_existing);
    }

    #[test]
    fn verify_with_valid_signature() {
        let r = offline_repair(&OfflineRepairRequest {
            pack_id: "p1".into(),
            requested_action: OfflineRepairAction::Verify,
            source: "local_verified_bundle".into(),
            payload_signature_valid: true,
            current_active: None,
        });
        assert_eq!(r.action, OfflineRepairAction::Verify);
    }

    #[test]
    fn rollback_keeps_existing_active() {
        let r = offline_repair(&OfflineRepairRequest {
            pack_id: "p1".into(),
            requested_action: OfflineRepairAction::Rollback,
            source: "local_verified_bundle".into(),
            payload_signature_valid: true,
            current_active: Some("p0".into()),
        });
        assert_eq!(r.action, OfflineRepairAction::Rollback);
        assert!(r.keep_existing);
        assert_eq!(r.source, "local_verified_bundle");
    }
}
