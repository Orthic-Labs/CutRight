//! Project repair pass.
//!
//! Given a [`ScanReport`], the repair pass filters the automatic findings
//! and emits a [`RepairReport`] describing each fix that *would* be applied
//! (or that *was* applied, depending on `apply`). No canonical object or
//! migration is touched here; only derivable/index/staging state is.

use serde::{Deserialize, Serialize};

use crate::scan::{FindingClass, FindingKind, ScanReport};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairAction {
    RebuildIndex,
    RemoveAbandonedStaging,
    ResumeJob,
    /// No automatic fix is available; surface to the user.
    NeedsUserDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairReport {
    pub actions: Vec<RepairActionRow>,
    pub manual_findings_preserved: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairActionRow {
    pub kind: FindingKind,
    pub action: RepairAction,
    pub path: String,
}

/// Plan a repair without applying anything.
pub fn plan_repair(report: &ScanReport) -> RepairReport {
    let mut actions = Vec::new();
    let mut manual = Vec::new();
    for f in &report.findings {
        if f.class == FindingClass::Automatic {
            actions.push(RepairActionRow {
                kind: f.kind.clone(),
                action: action_for(&f.kind),
                path: f.path.clone(),
            });
        } else {
            manual.push(f.path.clone());
        }
    }
    RepairReport {
        actions,
        manual_findings_preserved: manual,
    }
}

/// Plan + apply repair. For B7-013 we simply count how many automatic
/// actions would fire; the actual file mutation is owned by the harness
/// (studiod binary) which calls into here for policy decisions.
pub fn apply_repair(report: &ScanReport) -> RepairReport {
    plan_repair(report)
}

/// Returns true when a second repair run is a no-op against the supplied
/// finding list (every finding has been resolved or moved to manual).
pub fn repair_is_idempotent(report: &ScanReport) -> bool {
    !report.findings.iter().any(|f| {
        f.class == FindingClass::Automatic
            && matches!(
                f.kind,
                FindingKind::AbandonedStaging | FindingKind::ResumeableJob
            )
    })
}

fn action_for(kind: &FindingKind) -> RepairAction {
    match kind {
        FindingKind::AbandonedStaging => RepairAction::RemoveAbandonedStaging,
        FindingKind::ResumeableJob => RepairAction::ResumeJob,
        FindingKind::MissingActivePointer
        | FindingKind::RevisionHashMismatch
        | FindingKind::ReceiptHashMismatch
        | FindingKind::ReceiptTreeIncomplete
        | FindingKind::CanonicalObjectMismatch
        | FindingKind::PackMissing
        | FindingKind::MigrationUncommitted => RepairAction::NeedsUserDecision,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::{ObjectDigest, PackDigest, ReceiptDigest, RevisionDigest, ScanFinding};

    fn empty_finding() -> ScanReport {
        ScanReport::default()
    }

    fn with_automatic() -> ScanReport {
        let mut r = empty_finding();
        r.findings.push(ScanFinding {
            kind: FindingKind::AbandonedStaging,
            class: FindingClass::Automatic,
            path: "staging/job1".to_string(),
            message: "...".to_string(),
        });
        r.findings.push(ScanFinding {
            kind: FindingKind::ResumeableJob,
            class: FindingClass::Automatic,
            path: "job/42".to_string(),
            message: "...".to_string(),
        });
        r.findings.push(ScanFinding {
            kind: FindingKind::ReceiptHashMismatch,
            class: FindingClass::Manual,
            path: "r/1".to_string(),
            message: "...".to_string(),
        });
        r
    }

    #[test]
    fn idempotent_when_no_automatic_left() {
        let r = empty_finding();
        assert!(repair_is_idempotent(&r));
    }

    #[test]
    fn not_idempotent_while_automatic_findings_remain() {
        let r = with_automatic();
        assert!(!repair_is_idempotent(&r));
        let plan = plan_repair(&r);
        assert_eq!(plan.actions.len(), 2);
        assert_eq!(plan.manual_findings_preserved.len(), 1);
    }

    #[test]
    fn manual_findings_are_preserved_unchanged() {
        let r = with_automatic();
        let plan = plan_repair(&r);
        assert_eq!(plan.manual_findings_preserved, vec!["r/1".to_string()]);
    }

    #[test]
    fn revision_hash_mismatch_is_manual() {
        let mut s = crate::scan::ProjectSnapshot {
            active_pointer_present: true,
            ..Default::default()
        };
        s.revisions.push(RevisionDigest {
            id: "rev1".into(),
            declared_hash: "abc".into(),
            computed_hash: "xyz".into(),
        });
        let r = crate::scan::scan_snapshot(&s);
        assert_eq!(r.findings.len(), 1);
        assert_eq!(r.findings[0].class, FindingClass::Manual);
    }

    #[test]
    fn pack_signature_failure_is_manual() {
        let mut s = crate::scan::ProjectSnapshot {
            active_pointer_present: true,
            ..Default::default()
        };
        s.packs.push(PackDigest {
            id: "pack1".into(),
            signature_valid: false,
        });
        let r = crate::scan::scan_snapshot(&s);
        assert_eq!(r.findings[0].class, FindingClass::Manual);
    }

    #[test]
    fn receipt_mismatch_classified_manual() {
        let mut s = crate::scan::ProjectSnapshot {
            active_pointer_present: true,
            ..Default::default()
        };
        s.receipts.push(ReceiptDigest {
            id: "r1".into(),
            declared_hash: "a".into(),
            computed_hash: "b".into(),
        });
        s.canonical_objects.push(ObjectDigest {
            id: "o1".into(),
            declared_hash: "x".into(),
            computed_hash: "y".into(),
        });
        let r = crate::scan::scan_snapshot(&s);
        assert_eq!(r.findings.len(), 2);
        for f in &r.findings {
            assert_eq!(f.class, FindingClass::Manual);
        }
    }
}
