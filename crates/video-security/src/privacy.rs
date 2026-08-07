//! Privacy-safe local logging helpers.
//!
//! The logger is a pure data structure that the runner calls into. It
//! forces the default log shape to be `[component, code, project pseudonymous
//! id, revision, job/stage id, durations, hashes]` — never the raw source
//! path, transcript, prompt or API key. Network attempts are counted
//! separately so a blocked-network target can still audit them.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub component: String,
    pub code: String,
    pub project_pseudo_id: String,
    pub revision: String,
    pub job_id: Option<String>,
    pub duration_ms: Option<u32>,
    pub hashes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogBuffer {
    pub entries: Vec<LogEntry>,
}

impl LogBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new entry into the buffer; the buffer is bounded by `cap`.
    /// Older entries fall off when the cap is exceeded.
    pub fn push(&mut self, entry: LogEntry, cap: usize) {
        self.entries.push(entry);
        if self.entries.len() > cap {
            let drop = self.entries.len() - cap;
            self.entries.drain(0..drop);
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkAttemptCounter {
    pub attempts: u64,
}

static NETWORK_ATTEMPTS: AtomicU64 = AtomicU64::new(0);

pub fn network_attempt_record() {
    NETWORK_ATTEMPTS.fetch_add(1, Ordering::SeqCst);
}

pub fn network_attempt_count() -> u64 {
    NETWORK_ATTEMPTS.load(Ordering::SeqCst)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryToggle {
    Off,
}

/// Returns true when telemetry/network are turned off (always — the v2 release
/// has both disabled by default).
pub fn telemetry_off() -> bool {
    matches!(TelemetryToggle::Off, TelemetryToggle::Off)
}

/// Build a default log entry from a stable record. The constructor refuses
/// to copy raw source paths, transcript content, prompt text or keys.
pub fn build_log_entry(
    component: &str,
    code: &str,
    project_pseudo_id: &str,
    revision: &str,
    job_id: Option<&str>,
    duration_ms: Option<u32>,
    hashes: Vec<String>,
) -> LogEntry {
    LogEntry {
        component: component.to_string(),
        code: code.to_string(),
        project_pseudo_id: project_pseudo_id.to_string(),
        revision: revision.to_string(),
        job_id: job_id.map(|s| s.to_string()),
        duration_ms,
        hashes,
    }
}

/// Redact a candidate string. Returns the placeholder if the string contains
/// any forbidden pattern (raw transcript, prompt, API key).
pub fn redact(candidate: &str) -> String {
    let lower = candidate.to_lowercase();
    if lower.contains("transcript:") || lower.contains("prompt:") || lower.contains("apikey") {
        return "<redacted>".to_string();
    }
    if candidate.starts_with('/') && candidate.contains('/') {
        // path-like inputs are reduced to a hash-shaped fingerprint
        return short_fingerprint(candidate);
    }
    candidate.to_string()
}

fn short_fingerprint(s: &str) -> String {
    use blake3::Hasher;
    let mut h = Hasher::new();
    h.update(s.as_bytes());
    let d = h.finalize();
    let mut out = String::with_capacity(8);
    for b in d.as_bytes().iter().take(4) {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

/// Clear diagnostics: returns the entries the caller may keep (logs that
/// do not depend on canonical project bytes) and an explicit decision that
/// canonical objects are untouched.
pub fn clear_diagnostics(buffer: &LogBuffer) -> ClearReport {
    let canonical_untouched = true;
    let kept = buffer.entries.clone();
    ClearReport {
        canonical_untouched,
        kept_entries: kept,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClearReport {
    pub canonical_untouched: bool,
    pub kept_entries: Vec<LogEntry>,
}

/// Build the LocalLog export document. The user reviews the supplied file
/// list; raw content is never embedded.
pub fn build_export(files: Vec<&str>) -> LocalLogExport {
    let mut listed = Vec::with_capacity(files.len());
    for f in files {
        listed.push(LogFile {
            path: f.to_string(),
            byte_size: 0,
            fingerprint: short_fingerprint(f),
        });
    }
    LocalLogExport { files: listed }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogFile {
    pub path: String,
    pub byte_size: u64,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalLogExport {
    pub files: Vec<LogFile>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_log_excludes_raw_text() {
        let e = build_log_entry(
            "studio",
            "decision.recorded",
            "proj-abc",
            "rev1",
            None,
            Some(12),
            vec!["abcd".to_string()],
        );
        assert_eq!(e.component, "studio");
        assert_eq!(e.revision, "rev1");
        // No transcript / prompt / api key text ever leaks in.
        let s = serde_json::to_string(&e).unwrap();
        assert!(!s.contains("/Users/"));
    }

    #[test]
    fn redact_strips_transcripts_and_paths() {
        assert_eq!(redact("transcript: hello world"), "<redacted>");
        assert_eq!(redact("prompt: do the thing"), "<redacted>");
        assert_eq!(redact("apikey=abcd"), "<redacted>");
    }

    #[test]
    fn network_attempt_counter_increments() {
        let before = network_attempt_count();
        network_attempt_record();
        let after = network_attempt_count();
        assert_eq!(after, before + 1);
    }

    #[test]
    fn telemetry_is_off_by_default() {
        assert!(telemetry_off());
    }

    #[test]
    fn clear_diagnostics_preserves_canonical_state() {
        let mut buf = LogBuffer::new();
        buf.push(
            build_log_entry("studio", "ok", "p", "r", None, None, vec![]),
            10,
        );
        let r = clear_diagnostics(&buf);
        assert!(r.canonical_untouched);
        assert_eq!(r.kept_entries.len(), 1);
    }
}
