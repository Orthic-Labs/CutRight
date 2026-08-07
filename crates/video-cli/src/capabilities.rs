//! `videoctl capabilities …` — read-only inspection of the canonical
//! capability registry owned by Lane P-B (CR-V2-B2-014).
//!
//! The CLI NEVER duplicates registry logic: it loads
//! `docs/dispatch/v2/source/capability-registry.json` through the shared
//! `video_capabilities::RegistryDocument` loader and prints a JSON document
//! to stdout. The Studio backend (B2-024) calls the same loader via the
//! `ActionExecutor`; the loopback MCP adapter (B2-025) reuses the same
//! `RegistryDocument`. There is exactly one source of truth for the v2
//! capability registry and this command is a thin transport over it.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Serialize;
use video_capabilities::{CapabilityRegistry, RegistryDocument};

use crate::cli::CapabilitiesCommand;

const CAPABILITY_LIST_SCHEMA: &str = "cutright.capability_list/v1";

/// Outcome of `videoctl capabilities list`. The struct is `Serialize` so the
/// JSON shape is locked to a single, byte-stable document.
#[derive(Debug, Serialize)]
pub struct CapabilityList {
    /// Always [`CAPABILITY_LIST_SCHEMA`].
    pub schema: &'static str,
    /// Stable registry id (mirrors `RegistryDocument::registry_id`).
    pub registry_id: String,
    /// Source path the registry was loaded from (relative to repo root).
    pub source_path: String,
    /// Number of capability entries (post-filter).
    pub count: usize,
    /// Capability entries (after applying the optional `--id` filter).
    pub capabilities: Vec<CapabilityEntry>,
}

/// A single entry in [`CapabilityList`]. Mirrors the relevant fields of
/// `video_capabilities::Capability` so callers don't have to depend on the
/// crate to read the JSON.
#[derive(Debug, Serialize)]
pub struct CapabilityEntry {
    /// Stable, snake_case capability id.
    pub id: String,
    /// Monotone version.
    pub version: u32,
    /// `read` or `mutation`.
    pub kind: String,
    /// Owning crate / component.
    pub owner_component: String,
    /// Permission-set id.
    pub permission_set: String,
    /// Output-shape hint.
    pub outputs: Outputs,
    /// Degradation status.
    pub degradation: String,
}

/// Output-shape projection.
#[derive(Debug, Serialize)]
pub struct Outputs {
    pub bounded: bool,
    pub windowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u32>,
}

/// Locate the canonical capability registry. Resolution order:
/// 1. `CUTRIGHT_CAPABILITY_REGISTRY` env var, if set.
/// 2. `<repo_root>/docs/dispatch/v2/source/capability-registry.json`,
///    where `<repo_root>` is the parent of `CARGO_MANIFEST_DIR`.
pub fn resolve_registry_path() -> PathBuf {
    if let Ok(value) = std::env::var("CUTRIGHT_CAPABILITY_REGISTRY") {
        return PathBuf::from(value);
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    repo_root.join("docs/dispatch/v2/source/capability-registry.json")
}

/// Run a [`CapabilitiesCommand`] and return its exit code.
pub fn run(command: &CapabilitiesCommand) -> ExitCode {
    match command {
        CapabilitiesCommand::List { id, json } => run_list(id.as_deref(), *json),
    }
}

fn run_list(filter: Option<&str>, _json: bool) -> ExitCode {
    let path = resolve_registry_path();
    let doc = match RegistryDocument::load(&path) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!(
                "failed to load capability registry at {}: {err}",
                path.display()
            );
            return ExitCode::FAILURE;
        }
    };
    let registry = doc.into_registry();
    let list = build_list(&registry, filter, &path);
    match serde_json::to_string_pretty(&list) {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("failed to serialize capability list: {err}");
            ExitCode::FAILURE
        }
    }
}

fn build_list(registry: &CapabilityRegistry, filter: Option<&str>, path: &Path) -> CapabilityList {
    let mut entries: Vec<CapabilityEntry> = registry
        .capabilities
        .values()
        .filter(|cap| filter.is_none_or(|needle| cap.capability_id.0 == needle))
        .map(|cap| CapabilityEntry {
            id: cap.capability_id.0.clone(),
            version: cap.version,
            kind: match cap.kind {
                video_capabilities::CapabilityKind::Read => "read".to_string(),
                video_capabilities::CapabilityKind::Mutation => "mutation".to_string(),
            },
            owner_component: cap.owner_component.clone(),
            permission_set: cap.permission_set.clone(),
            outputs: Outputs {
                bounded: cap.outputs.bounded,
                windowed: cap.outputs.windowed,
                max_items: cap.outputs.max_items,
            },
            degradation: cap
                .degradation
                .map(|d| match d {
                    video_capabilities::Degradation::Ok => "ok",
                    video_capabilities::Degradation::Degraded => "degraded",
                    video_capabilities::Degradation::Missing => "missing",
                })
                .unwrap_or("ok")
                .to_string(),
        })
        .collect();
    // Stable ordering so byte-for-byte output is deterministic.
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    let source_path = path
        .strip_prefix(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(|p| p.parent())
                .unwrap_or_else(|| std::path::Path::new(".")),
        )
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string_lossy().into_owned());
    CapabilityList {
        schema: CAPABILITY_LIST_SCHEMA,
        registry_id: registry.registry_id.clone(),
        source_path,
        count: entries.len(),
        capabilities: entries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use video_capabilities::RegistryDocument;

    fn fixture_path() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root")
            .join("docs/dispatch/v2/source/capability-registry.json")
    }

    #[test]
    fn resolve_registry_path_finds_canonical_document() {
        let path = resolve_registry_path();
        assert!(path.ends_with("docs/dispatch/v2/source/capability-registry.json"));
    }

    #[test]
    fn build_list_includes_every_canonical_capability() {
        let doc = RegistryDocument::load(fixture_path()).expect("canonical registry loads");
        let registry = doc.into_registry();
        let list = build_list(&registry, None, &fixture_path());
        assert_eq!(list.schema, CAPABILITY_LIST_SCHEMA);
        assert_eq!(list.count, registry.len());
        assert!(list.capabilities.iter().any(|c| c.id == "timeline.cut"));
        assert!(list.capabilities.iter().any(|c| c.id == "evidence.read"));
    }

    #[test]
    fn build_list_filters_by_id() {
        let doc = RegistryDocument::load(fixture_path()).expect("canonical registry loads");
        let registry = doc.into_registry();
        let list = build_list(&registry, Some("timeline.cut"), &fixture_path());
        assert_eq!(list.count, 1);
        assert_eq!(list.capabilities[0].id, "timeline.cut");
        assert_eq!(list.capabilities[0].kind, "mutation");
    }

    #[test]
    fn build_list_is_byte_stable() {
        let doc = RegistryDocument::load(fixture_path()).expect("canonical registry loads");
        let registry = doc.into_registry();
        let list_a = build_list(&registry, None, &fixture_path());
        let list_b = build_list(&registry, None, &fixture_path());
        let json_a = serde_json::to_string_pretty(&list_a).unwrap();
        let json_b = serde_json::to_string_pretty(&list_b).unwrap();
        assert_eq!(json_a, json_b);
    }
}
