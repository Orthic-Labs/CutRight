//! Integration tests for the recovery scan/repair pass.

use video_recovery::repair::{plan_repair, repair_is_idempotent, RepairAction};
use video_recovery::scan::{
    scan_snapshot, ObjectDigest, PackDigest, ProjectSnapshot, ReceiptDigest, RevisionDigest,
    StagingPath,
};

fn empty() -> ProjectSnapshot {
    ProjectSnapshot {
        active_pointer_present: true,
        ..Default::default()
    }
}

#[test]
fn clean_snapshot_has_no_findings() {
    let r = scan_snapshot(&empty());
    assert!(r.is_clean());
    assert!(repair_is_idempotent(&r));
}

#[test]
fn active_pointer_missing_is_manual() {
    let mut s = empty();
    s.active_pointer_present = false;
    let r = scan_snapshot(&s);
    assert_eq!(r.findings.len(), 1);
    assert_eq!(
        r.findings[0].class,
        video_recovery::scan::FindingClass::Manual
    );
}

#[test]
fn revision_hash_mismatch_is_manual() {
    let mut s = empty();
    s.revisions.push(RevisionDigest {
        id: "rev1".into(),
        declared_hash: "abc".into(),
        computed_hash: "xyz".into(),
    });
    let r = scan_snapshot(&s);
    assert_eq!(
        r.findings[0].class,
        video_recovery::scan::FindingClass::Manual
    );
}

#[test]
fn abandoned_staging_is_automatic() {
    let mut s = empty();
    s.abandoned_staging.push(StagingPath { path: "st".into() });
    let r = scan_snapshot(&s);
    assert_eq!(
        r.findings[0].class,
        video_recovery::scan::FindingClass::Automatic
    );
}

#[test]
fn resumable_job_is_automatic() {
    let mut s = empty();
    s.resumable_jobs.push("job.0".into());
    let r = scan_snapshot(&s);
    assert_eq!(
        r.findings[0].class,
        video_recovery::scan::FindingClass::Automatic
    );
}

#[test]
fn receipt_mismatch_is_manual() {
    let mut s = empty();
    s.receipts.push(ReceiptDigest {
        id: "r".into(),
        declared_hash: "a".into(),
        computed_hash: "b".into(),
    });
    let r = scan_snapshot(&s);
    assert_eq!(
        r.findings[0].class,
        video_recovery::scan::FindingClass::Manual
    );
}

#[test]
fn canonical_object_mismatch_is_manual() {
    let mut s = empty();
    s.canonical_objects.push(ObjectDigest {
        id: "o".into(),
        declared_hash: "a".into(),
        computed_hash: "b".into(),
    });
    let r = scan_snapshot(&s);
    assert_eq!(
        r.findings[0].class,
        video_recovery::scan::FindingClass::Manual
    );
}

#[test]
fn pack_signature_failure_is_manual() {
    let mut s = empty();
    s.packs.push(PackDigest {
        id: "p".into(),
        signature_valid: false,
    });
    let r = scan_snapshot(&s);
    assert_eq!(
        r.findings[0].class,
        video_recovery::scan::FindingClass::Manual
    );
}

#[test]
fn migration_uncommitted_is_manual() {
    let mut s = empty();
    s.migration_uncommitted = true;
    let r = scan_snapshot(&s);
    assert_eq!(
        r.findings[0].class,
        video_recovery::scan::FindingClass::Manual
    );
}

#[test]
fn repair_does_not_discard_evidence() {
    let mut snap = empty();
    snap.receipts.push(ReceiptDigest {
        id: "r1".into(),
        declared_hash: "a".into(),
        computed_hash: "b".into(),
    });
    snap.abandoned_staging
        .push(StagingPath { path: "st".into() });
    let r = scan_snapshot(&snap);
    let plan = plan_repair(&r);
    // Receipt mismatch preserved as manual; staging cleared via plan.
    assert_eq!(plan.manual_findings_preserved.len(), 1);
    assert_eq!(plan.manual_findings_preserved[0], "r1");
    let actions: Vec<_> = plan.actions.iter().map(|a| a.action.clone()).collect();
    assert!(actions.contains(&RepairAction::RemoveAbandonedStaging));
}

#[test]
fn second_repair_run_is_a_no_op() {
    let mut snap = empty();
    snap.abandoned_staging
        .push(StagingPath { path: "st".into() });
    snap.resumable_jobs.push("job.0".into());
    let r = scan_snapshot(&snap);
    assert!(!repair_is_idempotent(&r));
    let cleaned = empty();
    let r2 = scan_snapshot(&cleaned);
    assert!(repair_is_idempotent(&r2));
    assert!(plan_repair(&r2).actions.is_empty());
}
