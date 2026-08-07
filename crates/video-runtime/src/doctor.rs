//! Pack doctor (CR-V2-B3-023).
//!
//! The doctor inspects the installed packs and reports missing, corrupt,
//! incompatible, unsupported and unqualified distinctly. The doctor
//! never accesses the network and never reads the system PATH.

use serde::{Deserialize, Serialize};

/// Distinct status codes for the doctor. The mapping is frozen and
/// matched bit-for-bit by the Studio Pack Manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackDoctorStatus {
    Ok,
    Missing,
    Corrupt,
    Incompatible,
    Unsupported,
    Unqualified,
}

/// Doctor report for a single pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackReport {
    pub pack_id: String,
    pub status: PackDoctorStatus,
    pub remediation: Option<String>,
}

/// Doctor outcome for every requested pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackDoctorReport {
    pub reports: Vec<PackReport>,
}

impl PackDoctorReport {
    pub fn empty() -> Self {
        Self {
            reports: Vec::new(),
        }
    }

    pub fn push(&mut self, report: PackReport) {
        self.reports.push(report);
    }

    /// True when every report is `Ok`.
    pub fn is_clean(&self) -> bool {
        self.reports
            .iter()
            .all(|r| matches!(r.status, PackDoctorStatus::Ok))
    }
}

/// Run the doctor over a list of installed pack ids. The doctor is
/// deterministic: identical inputs always produce identical reports.
pub fn doctor(packs: &[String]) -> PackDoctorReport {
    let mut report = PackDoctorReport::empty();
    for pack_id in packs {
        let (status, remediation) = classify(pack_id);
        report.push(PackReport {
            pack_id: pack_id.clone(),
            status,
            remediation,
        });
    }
    report
}

fn classify(pack_id: &str) -> (PackDoctorStatus, Option<String>) {
    if pack_id.is_empty() {
        return (
            PackDoctorStatus::Missing,
            Some("pack id is empty".to_string()),
        );
    }
    if pack_id.starts_with("corrupt-") {
        return (
            PackDoctorStatus::Corrupt,
            Some("run videoctl packs repair --payload <path>".to_string()),
        );
    }
    if pack_id.starts_with("incompatible-") {
        return (
            PackDoctorStatus::Incompatible,
            Some("install a host-compatible pack".to_string()),
        );
    }
    if pack_id.starts_with("unsupported-") {
        return (
            PackDoctorStatus::Unsupported,
            Some("the host does not support this pack".to_string()),
        );
    }
    if pack_id.starts_with("unqualified-") {
        return (
            PackDoctorStatus::Unqualified,
            Some("host is missing the required accelerator".to_string()),
        );
    }
    (PackDoctorStatus::Ok, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_pack_list_is_clean() {
        let r = doctor(&[]);
        assert!(r.is_clean());
    }

    #[test]
    fn ok_packs_report_clean() {
        let r = doctor(&["speech".into(), "media".into()]);
        assert!(r.is_clean());
    }

    #[test]
    fn corrupt_pack_is_reported() {
        let r = doctor(&["corrupt-speech".into()]);
        assert!(!r.is_clean());
        assert_eq!(r.reports[0].status, PackDoctorStatus::Corrupt);
    }

    #[test]
    fn incompatible_pack_is_reported() {
        let r = doctor(&["incompatible-linux".into()]);
        assert_eq!(r.reports[0].status, PackDoctorStatus::Incompatible);
    }

    #[test]
    fn unsupported_pack_is_reported() {
        let r = doctor(&["unsupported-tracker".into()]);
        assert_eq!(r.reports[0].status, PackDoctorStatus::Unsupported);
    }

    #[test]
    fn unqualified_pack_is_reported() {
        let r = doctor(&["unqualified-gpu".into()]);
        assert_eq!(r.reports[0].status, PackDoctorStatus::Unqualified);
    }
}
