//! Offline pack repair integration tests.

use video_runtime::offline_repair::{offline_repair, OfflineRepairAction, OfflineRepairRequest};

#[test]
fn corrupt_payload_is_rejected() {
    let r = offline_repair(&OfflineRepairRequest {
        pack_id: "p1".into(),
        requested_action: OfflineRepairAction::Repair,
        source: "corrupt".into(),
        payload_signature_valid: false,
        current_active: Some("p0".into()),
    });
    assert_eq!(r.action, OfflineRepairAction::Noop);
    assert!(r.keep_existing);
}

#[test]
fn interrupted_repair_keeps_old_pack_active() {
    let r = offline_repair(&OfflineRepairRequest {
        pack_id: "p1".into(),
        requested_action: OfflineRepairAction::Repair,
        source: "local_verified_bundle".into(),
        payload_signature_valid: false,
        current_active: Some("p0".into()),
    });
    // Repair with an invalid signature => Noop, and we must keep the old
    // pack in place.
    assert_eq!(r.action, OfflineRepairAction::Noop);
    assert!(r.keep_existing);
}

#[test]
fn no_network_control_or_url_appears() {
    let r = offline_repair(&OfflineRepairRequest {
        pack_id: "p1".into(),
        requested_action: OfflineRepairAction::Verify,
        source: "https://example.com/p1.pack".into(),
        payload_signature_valid: true,
        current_active: None,
    });
    let s = format!("{:?}", r);
    assert!(
        !s.contains("https://") || r.action == OfflineRepairAction::Noop,
        "non-local URL must be rejected: {}",
        s
    );
    assert!(!s.to_lowercase().contains("download"));
    assert_eq!(r.action, OfflineRepairAction::Noop);
}

#[test]
fn activate_requires_local_bundle_and_valid_signature() {
    let r = offline_repair(&OfflineRepairRequest {
        pack_id: "p1".into(),
        requested_action: OfflineRepairAction::Activate,
        source: "local_verified_bundle".into(),
        payload_signature_valid: true,
        current_active: None,
    });
    assert_eq!(r.action, OfflineRepairAction::Activate);
}
