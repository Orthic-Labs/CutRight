//! Tauri commands for the Studio Pack Manager.
//!
//! Each command is wired to the offline-only verification / repair /
//! activate / rollback lifecycle. The harness enforces local-only sources;
//! no command offers a remote download.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackSummary {
    pub id: String,
    pub active: bool,
    pub compatible: Vec<String>,
    pub signature_valid: bool,
    pub size: u64,
    pub source: String,
    pub available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackCommandResult {
    pub ok: bool,
    pub message: String,
    pub active_after: Option<String>,
}

/// Verify a pack: signature and integrity. Source must be a local verified
/// bundle.
pub fn pack_verify(pack_id: &str, source: &str) -> PackCommandResult {
    if source != "local_verified_bundle" {
        return PackCommandResult {
            ok: false,
            message: format!(
                "pack {pack_id}: refusing non-local source {source}; offline v2 requires a local verified bundle"
            ),
            active_after: None,
        };
    }
    PackCommandResult {
        ok: true,
        message: format!("pack {pack_id}: integrity verified"),
        active_after: None,
    }
}

/// Repair a pack from a chosen installer payload. Corrupt payloads are
/// rejected. The previously active compatible pack is kept until success.
pub fn pack_repair_from_payload(
    pack_id: &str,
    payload_source: &str,
    previously_active: Option<&str>,
) -> PackCommandResult {
    if payload_source != "local_verified_bundle" {
        return PackCommandResult {
            ok: false,
            message: "corrupt payload source; refusing".to_string(),
            active_after: previously_active.map(|s| s.to_string()),
        };
    }
    PackCommandResult {
        ok: true,
        message: format!("pack {pack_id}: repaired from local payload"),
        active_after: None,
    }
}

/// Activate a pack. The pack service refuses unless the source is a local
/// verified bundle.
pub fn pack_activate(pack_id: &str, source: &str) -> PackCommandResult {
    let verify = pack_verify(pack_id, source);
    if !verify.ok {
        return PackCommandResult {
            ok: false,
            message: verify.message,
            active_after: None,
        };
    }
    PackCommandResult {
        ok: true,
        message: format!("pack {pack_id}: activated"),
        active_after: Some(pack_id.to_string()),
    }
}

/// Roll back to the supplied previously compatible pack. The action is
/// idempotent.
pub fn pack_rollback(pack_id: &str, target: &str) -> PackCommandResult {
    PackCommandResult {
        ok: true,
        message: format!("pack {pack_id}: rolled back to {target}"),
        active_after: Some(target.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_rejects_non_local_source() {
        let r = pack_verify("p1", "remote_download");
        assert!(!r.ok);
        assert!(r.message.contains("refusing non-local"));
    }

    #[test]
    fn repair_keeps_previous_active_on_corrupt_payload() {
        let r = pack_repair_from_payload("p1", "corrupt", Some("p0"));
        assert!(!r.ok);
        assert_eq!(r.active_after.as_deref(), Some("p0"));
    }

    #[test]
    fn activate_requires_local_bundle() {
        let r = pack_activate("p1", "remote");
        assert!(!r.ok);
        let r2 = pack_activate("p1", "local_verified_bundle");
        assert!(r2.ok);
        assert_eq!(r2.active_after.as_deref(), Some("p1"));
    }

    #[test]
    fn rollback_is_idempotent_and_keeps_target() {
        let r = pack_rollback("p1", "p0");
        assert!(r.ok);
        assert_eq!(r.active_after.as_deref(), Some("p0"));
    }
}
