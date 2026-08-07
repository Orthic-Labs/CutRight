//! Crash-recovery scanner.
//!
//! The scanner inspects a project state and emits a list of [`ScanFinding`]s.
//! Each finding carries its classification as [`FindingClass`] (automatic
//! vs manual) and enough context for the repair pass to either apply a
//! fix or surface the issue to the user.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingClass {
    /// A derivable/index/staging fault the repair pass may fix automatically.
    Automatic,
    /// A canonical/source/migration fault that requires user decision.
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    /// Project pointer is missing or unreadable.
    MissingActivePointer,
    /// Revision record hash mismatch.
    RevisionHashMismatch,
    /// Abandoned staging directory present.
    AbandonedStaging,
    /// Job state present but resumable.
    ResumeableJob,
    /// Receipt hash mismatch.
    ReceiptHashMismatch,
    /// Receipt tree incomplete.
    ReceiptTreeIncomplete,
    /// Object hash mismatch against registered canonical.
    CanonicalObjectMismatch,
    /// Active pack missing or signature invalid.
    PackMissing,
    /// Migration active but uncommitted.
    MigrationUncommitted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanFinding {
    pub kind: FindingKind,
    pub class: FindingClass,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ScanReport {
    pub findings: Vec<ScanFinding>,
}

impl ScanReport {
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }
    pub fn automatic_findings(&self) -> impl Iterator<Item = &ScanFinding> {
        self.findings
            .iter()
            .filter(|f| f.class == FindingClass::Automatic)
    }
    pub fn manual_findings(&self) -> impl Iterator<Item = &ScanFinding> {
        self.findings
            .iter()
            .filter(|f| f.class == FindingClass::Manual)
    }
}

/// View of a project used by the scanner; callers extract it from the
/// project store (state crate) and pass it here.
#[derive(Debug, Clone, Default)]
pub struct ProjectSnapshot {
    pub active_pointer_present: bool,
    pub revisions: Vec<RevisionDigest>,
    pub abandoned_staging: Vec<StagingPath>,
    pub resumable_jobs: Vec<String>,
    pub receipts: Vec<ReceiptDigest>,
    pub canonical_objects: Vec<ObjectDigest>,
    pub packs: Vec<PackDigest>,
    pub migration_uncommitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionDigest {
    pub id: String,
    pub declared_hash: String,
    pub computed_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagingPath {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptDigest {
    pub id: String,
    pub declared_hash: String,
    pub computed_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectDigest {
    pub id: String,
    pub declared_hash: String,
    pub computed_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackDigest {
    pub id: String,
    pub signature_valid: bool,
}

/// Run the scanner against the supplied snapshot.
pub fn scan_snapshot(snapshot: &ProjectSnapshot) -> ScanReport {
    let mut findings = Vec::new();

    if !snapshot.active_pointer_present {
        findings.push(ScanFinding {
            kind: FindingKind::MissingActivePointer,
            class: FindingClass::Manual,
            path: "<project>/active".to_string(),
            message: "no active project pointer; user must relink or restore"
                .to_string(),
        });
    }

    for r in &snapshot.revisions {
        if r.declared_hash != r.computed_hash {
            findings.push(ScanFinding {
                kind: FindingKind::RevisionHashMismatch,
                class: FindingClass::Manual,
                path: r.id.clone(),
                message: "revision bytes do not match declared hash".to_string(),
            });
        }
    }

    for staging in &snapshot.abandoned_staging {
        findings.push(ScanFinding {
            kind: FindingKind::AbandonedStaging,
            class: FindingClass::Automatic,
            path: staging.path.clone(),
            message: "staging directory present without a corresponding job"
                .to_string(),
        });
    }

    for job in &snapshot.resumable_jobs {
        findings.push(ScanFinding {
            kind: FindingKind::ResumeableJob,
            class: FindingClass::Automatic,
            path: job.clone(),
            message: "interrupted job present; resumable".to_string(),
        });
    }

    for r in &snapshot.receipts {
        if r.declared_hash != r.computed_hash {
            findings.push(ScanFinding {
                kind: FindingKind::ReceiptHashMismatch,
                class: FindingClass::Manual,
                path: r.id.clone(),
                message: "receipt bytes do not match declared hash".to_string(),
            });
        }
    }

    for o in &snapshot.canonical_objects {
        if o.declared_hash != o.computed_hash {
            findings.push(ScanFinding {
                kind: FindingKind::CanonicalObjectMismatch,
                class: FindingClass::Manual,
                path: o.id.clone(),
                message: "canonical object bytes do not match declared hash"
                    .to_string(),
            });
        }
    }

    for p in &snapshot.packs {
        if !p.signature_valid {
            findings.push(ScanFinding {
                kind: FindingKind::PackMissing,
                class: FindingClass::Manual,
                path: p.id.clone(),
                message: "active pack missing or signature invalid".to_string(),
            });
        }
    }

    if snapshot.migration_uncommitted {
        findings.push(ScanFinding {
            kind: FindingKind::MigrationUncommitted,
            class: FindingClass::Manual,
            path: "<project>/migration".to_string(),
            message: "migration started but never committed".to_string(),
        });
    }

    ScanReport { findings }
}
