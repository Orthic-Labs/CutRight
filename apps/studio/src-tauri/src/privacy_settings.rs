//! Privacy settings stored in the Studio.
//!
//! The Tauri side of the application keeps these on disk for the user
//! review. The defaults match the v2 release policy: telemetry off,
//! network denied, no opt-in for raw transcript export.

use serde::{Deserialize, Serialize};

use video_security::privacy::{telemetry_off, NetworkAttemptCounter};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct PrivacySettings {
    pub telemetry_enabled: bool,
    pub network_allowed: bool,
    pub raw_transcript_export_allowed: bool,
    pub raw_prompt_export_allowed: bool,
    pub project_pseudonym_salt: String,
    pub network_attempts: NetworkAttemptCounter,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        // v2 release defaults match the policy floors: everything is off.
        assert!(telemetry_off());
        Self {
            telemetry_enabled: false,
            network_allowed: false,
            raw_transcript_export_allowed: false,
            raw_prompt_export_allowed: false,
            project_pseudonym_salt: "v2-salt".to_string(),
            network_attempts: NetworkAttemptCounter::default(),
        }
    }
}

impl PrivacySettings {
    #[allow(dead_code)]
    pub fn is_network_blocked(&self) -> bool {
        !self.network_allowed
    }

    #[allow(dead_code)]
    pub fn is_telemetry_off(&self) -> bool {
        !self.telemetry_enabled
    }

    /// Apply privacy-safe transformations to a project id so logs only see
    /// the pseudonym.
    #[allow(dead_code)]
    pub fn pseudonym_for(&self, raw_project_id: &str) -> String {
        use blake3::Hasher;
        let mut h = Hasher::new();
        h.update(self.project_pseudonym_salt.as_bytes());
        h.update(b"::");
        h.update(raw_project_id.as_bytes());
        let d = h.finalize();
        let mut out = String::with_capacity(16);
        for b in d.as_bytes().iter().take(8) {
            out.push_str(&format!("{:02x}", b));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_block_network_and_telemetry() {
        let s = PrivacySettings::default();
        assert!(s.is_network_blocked());
        assert!(s.is_telemetry_off());
        assert!(!s.raw_transcript_export_allowed);
        assert!(!s.raw_prompt_export_allowed);
    }

    #[test]
    fn pseudonym_is_deterministic_per_salt() {
        let s = PrivacySettings::default();
        let a = s.pseudonym_for("p1");
        let b = s.pseudonym_for("p1");
        assert_eq!(a, b);
        // The raw project id never appears in the pseudonym.
        assert!(!a.contains("p1"));
    }

    #[test]
    fn different_salts_give_different_pseudonyms() {
        let s1 = PrivacySettings {
            project_pseudonym_salt: "salt-1".to_string(),
            ..PrivacySettings::default()
        };
        let s2 = PrivacySettings {
            project_pseudonym_salt: "salt-2".to_string(),
            ..PrivacySettings::default()
        };
        assert_ne!(s1.pseudonym_for("p1"), s2.pseudonym_for("p1"));
    }
}
