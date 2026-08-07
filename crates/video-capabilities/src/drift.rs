//! Capability registry drift detector (CR-V2-B2-016).
//!
//! Walks the in-tree source set declared by Lane P-B
//! (`crates/video-capabilities/`, `crates/video-cli/`,
//! `bindings/ts/`, `bindings/mcp/`, `apps/studio/src-tauri/`) and extracts
//! every literal `capability_id` referenced from hand-written code. The
//! resulting set is compared to the canonical registry declared at
//! `docs/dispatch/v2/source/capability-registry.json`.
//!
//! Drift is reported in three categories:
//!
//! 1. **Unknown references** — a literal in code that is not in the
//!    registry (e.g. a typo, or a capability that was deleted from the
//!    canonical registry without removing its callers).
//! 2. **Unreferenced capabilities** — a canonical capability that no source
//!    file mentions. Allowed but warned about (this is how we tell when a
//!    capability has been added but never wired up).
//! 3. **Schema drift** — a generated artifact (`generated.rs`,
//!    `bindings/ts/capabilities.ts`, `bindings/mcp/tool-registry.json`) is
//!    out of sync with the canonical registry; the detector prints the
//!    stale byte hash so the regeneration step is obvious.
//!
//! The drift detector is invoked by `tools/v2-evals/registry_drift.py` and
//! also by the Book 2 gate (`B2-027`).

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::registry::{Capability, RegistryDocument, REGISTRY_SCHEMA};

/// Drift report produced by [`detect_drift`].
#[derive(Debug)]
pub struct DriftReport {
    /// Capability ids declared in the canonical registry, sorted.
    pub canonical: BTreeSet<String>,
    /// Capability ids referenced from any scanned source file.
    pub referenced: BTreeSet<String>,
    /// `referenced - canonical` — references to capabilities that don't
    /// exist. **Hard failure**: the gate must exit non-zero.
    pub unknown_references: BTreeSet<String>,
    /// `canonical - referenced` — capabilities no source file mentions.
    /// **Soft warning**: reported but does not fail the gate.
    pub unreferenced_capabilities: BTreeSet<String>,
    /// Path -> SHA-256 of the file, for any generated artifact whose
    /// expected hash does not match the on-disk hash.
    pub generated_artifacts_drift: Vec<GeneratedArtifactDrift>,
}

/// One stale generated artifact.
#[derive(Debug, Clone)]
pub struct GeneratedArtifactDrift {
    /// File path relative to the repo root.
    pub path: PathBuf,
    /// What the artifact would be if regenerated from the canonical
    /// registry (its on-disk content hash).
    pub expected_hash: String,
    /// What is on disk right now.
    pub actual_hash: String,
}

impl DriftReport {
    /// True if the tree has no unknown references AND every generated
    /// artifact is in sync. Soft warnings (unreferenced capabilities) do
    /// not flip this.
    pub fn is_clean(&self) -> bool {
        self.unknown_references.is_empty() && self.generated_artifacts_drift.is_empty()
    }
}

impl fmt::Display for DriftReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Drift report for canonical capability registry")?;
        writeln!(f, "  canonical count: {}", self.canonical.len())?;
        writeln!(f, "  referenced count: {}", self.referenced.len())?;
        writeln!(f, "  unknown references: {:?}", self.unknown_references)?;
        writeln!(f, "  unreferenced (warning): {:?}", self.unreferenced_capabilities)?;
        for drift in &self.generated_artifacts_drift {
            writeln!(
                f,
                "  generated artifact drift: {} (expected {}, got {})",
                drift.path.display(),
                drift.expected_hash,
                drift.actual_hash
            )?;
        }
        Ok(())
    }
}

/// Subset of the report that the JSON gate script consumes. We intentionally
/// keep this separate from [`DriftReport`] so the public type can stay
/// `Debug`-only.
#[derive(Debug, serde::Serialize)]
pub struct DriftReportJson {
    /// `cutright.capability_drift_report/v1`
    pub schema: &'static str,
    /// Number of canonical capabilities.
    pub canonical_count: usize,
    /// Number of capabilities referenced from in-tree code.
    pub referenced_count: usize,
    /// Capabilities referenced in code but absent from the registry.
    pub unknown_references: Vec<String>,
    /// Capabilities in the registry but not referenced anywhere.
    pub unreferenced_capabilities: Vec<String>,
    /// Generated artifacts that are out of sync.
    pub generated_artifacts_drift: Vec<DriftedArtifactJson>,
    /// True iff [`DriftReport::is_clean`] is true.
    pub clean: bool,
}

/// Serializable view of [`GeneratedArtifactDrift`].
#[derive(Debug, serde::Serialize)]
pub struct DriftedArtifactJson {
    /// File path relative to the repo root.
    pub path: String,
    /// Hash of the expected (regenerated) artifact.
    pub expected_hash: String,
    /// Hash of the actual on-disk artifact.
    pub actual_hash: String,
}

impl From<&DriftReport> for DriftReportJson {
    fn from(report: &DriftReport) -> Self {
        Self {
            schema: "cutright.capability_drift_report/v1",
            canonical_count: report.canonical.len(),
            referenced_count: report.referenced.len(),
            unknown_references: report.unknown_references.iter().cloned().collect(),
            unreferenced_capabilities: report.unreferenced_capabilities.iter().cloned().collect(),
            generated_artifacts_drift: report
                .generated_artifacts_drift
                .iter()
                .map(|d| DriftedArtifactJson {
                    path: d.path.display().to_string(),
                    expected_hash: d.expected_hash.clone(),
                    actual_hash: d.actual_hash.clone(),
                })
                .collect(),
            clean: report.is_clean(),
        }
    }
}

/// Walk `repo_root` looking for `capability_id` string literals in any
/// source file under the scanned set, returning the union of all matches.
///
/// `source_paths` is the list of directories to walk (typically
/// `crates/video-capabilities/`, `crates/video-cli/`, `bindings/`,
/// `apps/studio/src-tauri/`). Hidden directories (`.git`, `target`, etc.)
/// are skipped.
pub fn scan_references(_repo_root: &Path, source_paths: &[PathBuf]) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    for root in source_paths {
        walk_for_references(root, &mut refs);
    }
    // The canonical registry itself is part of the tree; drop its literals
    // so the detector doesn't count the source-of-truth as a self-reference.
    refs.remove(REGISTRY_SCHEMA);
    refs
}

fn walk_for_references(root: &Path, refs: &mut BTreeSet<String>) {
    let walker = match fs::read_dir(root) {
        Ok(walker) => walker,
        Err(_) => return,
    };
    for entry in walker.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            walk_for_references(&path, refs);
        } else if is_scannable(&path) {
            collect_references_from_file(&path, refs);
        }
    }
}

fn is_scannable(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("rs" | "ts" | "tsx" | "js" | "json" | "md")
    )
}

fn collect_references_from_file(path: &Path, refs: &mut BTreeSet<String>) {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return,
    };
    for line in text.lines() {
        for token in extract_capability_id_literals(line) {
            refs.insert(token);
        }
    }
}

/// Recognise `capability_id = "..."`, `capability_id: "..."`, and
/// `capability_id: '...'` literals in a single source line. Conservative:
/// we only match the canonical snake_case shape.
pub fn extract_capability_id_literals(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let needle = b"capability_id";
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            // Look for an `=` or `:` followed by optional whitespace and a
            // string opener.
            let mut j = i + needle.len();
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && (bytes[j] == b'=' || bytes[j] == b':') {
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < bytes.len() && (bytes[j] == b'"' || bytes[j] == b'\'') {
                    let quote = bytes[j];
                    j += 1;
                    let start = j;
                    while j < bytes.len() && bytes[j] != quote {
                        j += 1;
                    }
                    if j < bytes.len() {
                        let literal = &line[start..j];
                        if is_capability_id_shape(literal) {
                            out.push(literal.to_string());
                        }
                        // Skip past the closing quote so the next iteration
                        // doesn't re-match a substring inside the literal.
                        j += 1;
                        i = j;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    out
}

fn is_capability_id_shape(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
        && s.chars().next().is_some_and(|c| c.is_ascii_lowercase())
}

/// Verify that a set of generated artifacts is in sync with the canonical
/// registry. `artifacts` lists `(path, expected_contents)` pairs; on drift
/// the path is reported with the actual hash.
pub fn check_generated_artifacts(
    artifacts: &[(PathBuf, String)],
) -> Result<Vec<GeneratedArtifactDrift>, Box<dyn Error>> {
    let mut drift = Vec::new();
    for (path, expected) in artifacts {
        let actual = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!(
                    "failed to read generated artifact {}: {error}",
                    path.display()
                )
                .into())
            }
        };
        if actual != *expected {
            drift.push(GeneratedArtifactDrift {
                path: path.to_path_buf(),
                expected_hash: short_hash(expected.as_bytes()),
                actual_hash: short_hash(actual.as_bytes()),
            });
        }
    }
    Ok(drift)
}

fn short_hash(bytes: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(bytes);
    let hash = hasher.finalize();
    let hex = hash.to_hex();
    let trimmed = hex.as_str();
    String::from(&trimmed[..16.min(trimmed.len())])
}

/// Run the full drift detection pipeline: load the canonical registry,
/// scan the tree, check generated artifacts, build a report.
pub fn detect_drift(
    repo_root: &Path,
    registry_path: &Path,
    source_paths: &[PathBuf],
    generated_artifacts: &[(PathBuf, String)],
) -> Result<DriftReport, Box<dyn Error>> {
    let doc = RegistryDocument::load(registry_path)?;
    let canonical = canonical_ids(&doc);
    let referenced = scan_references(repo_root, source_paths);
    let unknown: BTreeSet<String> = referenced.difference(&canonical).cloned().collect();
    let unreferenced: BTreeSet<String> = canonical.difference(&referenced).cloned().collect();
    let artifacts_drift = check_generated_artifacts(generated_artifacts)?;
    Ok(DriftReport {
        canonical,
        referenced,
        unknown_references: unknown,
        unreferenced_capabilities: unreferenced,
        generated_artifacts_drift: artifacts_drift,
    })
}

/// Sorted set of capability ids declared by the registry.
fn canonical_ids(doc: &RegistryDocument) -> BTreeSet<String> {
    doc.capabilities
        .iter()
        .map(|c: &Capability| c.capability_id.0.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_capability_id_literals_handles_common_shapes() {
        let line = r#"capability_id = "timeline.read", capability_id: 'asset.plan', capability_id = "BAD-ID","#;
        let tokens = extract_capability_id_literals(line);
        assert_eq!(tokens, vec!["timeline.read", "asset.plan"]);
    }

    #[test]
    fn extract_capability_id_literals_ignores_unknown_shapes() {
        let line = r#"capability_id = "Timeline.read", capability_id = "1abc", capability_id = """#;
        let tokens = extract_capability_id_literals(line);
        assert!(tokens.is_empty(), "expected no tokens, got {tokens:?}");
    }

    #[test]
    fn is_capability_id_shape_is_strict() {
        assert!(is_capability_id_shape("timeline.read"));
        assert!(is_capability_id_shape("pack.manage"));
        assert!(!is_capability_id_shape("Timeline.read"));
        assert!(!is_capability_id_shape("1abc"));
        assert!(!is_capability_id_shape(""));
    }
}