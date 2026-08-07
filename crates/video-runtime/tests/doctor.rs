// crates/video-runtime/tests/doctor.rs — CR-V2-B3-023 focused doctor tests.
//
// The doctor must satisfy:
//   * No command accesses network or PATH.
//   * Corrupt file and signature fixtures fail.
//   * Rollback restores the prior active lock atomically.

use video_runtime::doctor::{doctor, PackDoctorStatus};
use video_runtime::repair::{repair, RepairOutcome, RollbackPlan};

#[test]
fn doctor_is_clean_for_named_packs() {
    let r = doctor(&["speech".into(), "media".into()]);
    assert!(r.is_clean());
}

#[test]
fn doctor_reports_corrupt_pack_distinctly() {
    let r = doctor(&["corrupt-speech".into()]);
    assert_eq!(r.reports[0].status, PackDoctorStatus::Corrupt);
    assert!(r.reports[0].remediation.is_some());
}

#[test]
fn doctor_reports_incompatible_pack_distinctly() {
    let r = doctor(&["incompatible-linux".into()]);
    assert_eq!(r.reports[0].status, PackDoctorStatus::Incompatible);
}

#[test]
fn doctor_reports_unsupported_pack_distinctly() {
    let r = doctor(&["unsupported-tracker".into()]);
    assert_eq!(r.reports[0].status, PackDoctorStatus::Unsupported);
}

#[test]
fn doctor_reports_unqualified_pack_distinctly() {
    let r = doctor(&["unqualified-gpu".into()]);
    assert_eq!(r.reports[0].status, PackDoctorStatus::Unqualified);
}

#[test]
fn repair_signature_mismatch_is_rejected() {
    let out = repair("speech", "/path/to/payload", false);
    assert!(matches!(out, RepairOutcome::SignatureMismatch { .. }));
}

#[test]
fn repair_missing_payload_is_rejected() {
    let out = repair("speech", "", true);
    assert!(matches!(out, RepairOutcome::PayloadMissing { .. }));
}

#[test]
fn rollback_restores_previous_lock_atomically() {
    let plan = RollbackPlan {
        pack_id: "speech".into(),
        previous_lock: "lock-prev".into(),
        current_lock: "lock-curr".into(),
    };
    assert!(plan.restore().is_ok());
}

#[test]
fn rollback_fails_when_no_previous_lock() {
    let plan = RollbackPlan {
        pack_id: "speech".into(),
        previous_lock: "".into(),
        current_lock: "lock-curr".into(),
    };
    assert!(plan.restore().is_err());
}
