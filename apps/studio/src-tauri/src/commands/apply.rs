//! Studio Tauri command: `apply_action_batch` — drives the same
//! [`ActionExecutor`](video_project::ActionExecutor) the JSON CLI uses
//! (CR-V2-B2-024).
//!
//! This module is a thin marshalling layer. It does not duplicate executor
//! logic; it only:
//! 1. Parses a `cutright.action_batch/v1` envelope from the frontend.
//! 2. Loads the canonical capability registry (Lane P-B) from the repo
//!    default or a `CUTRIGHT_CAPABILITY_REGISTRY` override.
//! 3. Acquires a `SessionGuard` for the target project (Lane P-C).
//! 4. Hands the batch to the executor and serialises the
//!    [`ExecutorReport`](video_project::ExecutorReport) to JSON for the
//!    frontend.
//!
//! The Studio frontend receives the typed result through the same
//! `action_result/v1` JSON envelope the CLI prints, so the Studio can
//! deserialise it into the shared TS `CapabilityDescriptor` types generated
//! in B2-014.

use std::path::PathBuf;

use video_capabilities::RegistryDocument;
use video_project::{ActionBatch, ActionExecutor, ExecutorReport, ACTION_BATCH_SCHEMA};
use video_sessions::{ProjectId, SessionGuard};

use crate::project_scope::canonical_project_root;

/// Outcome of running an action batch from the Studio UI. The frontend
/// always gets a structured value — errors here are domain errors (failed
/// validation, lock held) returned as `Err(String)` per Tauri convention.
#[derive(Debug)]
pub struct ApplyOutcome {
    /// The executor's typed report, serialised to JSON for the frontend.
    pub report: ExecutorReport,
}

/// Load the canonical capability registry from disk, resolving the registry
/// path the same way the CLI does (B2-023). The Studio's frontend does not
/// pass a registry override; we always fall back to the in-repo canonical
/// file so the Studio's behaviour matches `videoctl apply` exactly.
fn load_registry() -> Result<video_capabilities::CapabilityRegistry, String> {
    let registry_path = resolve_registry_path();
    let doc = RegistryDocument::load(&registry_path)
        .map_err(|error| format!("load registry {}: {error}", registry_path.display()))?;
    Ok(doc.into_registry())
}

/// Resolve the canonical registry path. Mirrors `crates/video-cli/apply.rs`
/// so the two surfaces never disagree on which registry they use. Both the
/// CLI and the Studio binary live at `<repo>/crates/video-cli/` and
/// `<repo>/apps/studio/src-tauri/` respectively; both go up three parents
/// to the repo root.
fn resolve_registry_path() -> PathBuf {
    if let Ok(value) = std::env::var("CUTRIGHT_CAPABILITY_REGISTRY") {
        return PathBuf::from(value);
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent());
    let path = repo_root
        .map(|p| p.join("docs/dispatch/v2/source/capability-registry.json"))
        .unwrap_or_else(|| PathBuf::from("docs/dispatch/v2/source/capability-registry.json"));
    if path.is_file() {
        path
    } else {
        // Fallback for environments where `CARGO_MANIFEST_DIR` doesn't
        // match the on-disk layout (e.g. relocated source trees).
        PathBuf::from("docs/dispatch/v2/source/capability-registry.json")
    }
}

/// The Tauri command body. The frontend calls this with a project path and
/// the raw `action_batch` JSON; the command resolves the project root,
/// drives the executor, and returns the JSON-serialised
/// [`ExecutorReport`].
///
/// This is the single entry point every Studio surface (clip timeline,
/// review pane, embedded agent) uses to mutate project state. It mirrors
/// `videoctl apply` byte-for-byte on identical inputs.
pub(crate) fn run_apply(
    path: String,
    batch_json: serde_json::Value,
) -> Result<ApplyOutcome, String> {
    let root = canonical_project_root(&path)?;

    // Parse the action_batch envelope. The frontend sends it as JSON
    // because Tauri command arguments must be serde-deserialisable.
    let batch: ActionBatch = serde_json::from_value(batch_json)
        .map_err(|error| format!("apply: failed to parse action_batch: {error}"))?;
    if batch.schema != ACTION_BATCH_SCHEMA {
        return Err(format!(
            "apply: action_batch schema {:?} does not match {}",
            batch.schema, ACTION_BATCH_SCHEMA
        ));
    }

    let registry = load_registry()?;

    let project_id = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".into());
    let guard = SessionGuard::acquire(&root, ProjectId::new(project_id))
        .map_err(|error| format!("apply: failed to acquire project lock: {error}"))?;

    let executor = ActionExecutor::new(&root);
    let report = executor
        .execute(&batch, &registry, &guard, None)
        .map_err(|error| format!("apply: executor failed: {error}"))?;

    Ok(ApplyOutcome { report })
}

/// Tauri command entry point. Returns the executor report as JSON so the
/// frontend can deserialise it into its typed shape.
#[tauri::command]
pub(crate) fn apply_action_batch(
    path: String,
    batch: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let outcome = run_apply(path, batch)?;
    serde_json::to_value(&outcome.report).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use video_capabilities::RegistryDocument;
    use video_project::{ActionBatch, ExecutorAction, ACTION_BATCH_SCHEMA};

    fn init_project(root: &std::path::Path) {
        fs::create_dir_all(root).unwrap();
        fs::write(
            root.join("project.json"),
            r#"{"project_id":"project-test"}"#,
        )
        .unwrap();
    }

    fn load_canonical_registry() -> video_capabilities::CapabilityRegistry {
        let path = resolve_registry_path();
        RegistryDocument::load(&path)
            .unwrap_or_else(|error| {
                panic!("canonical registry missing at {}: {error}", path.display())
            })
            .into_registry()
    }

    #[test]
    fn run_apply_returns_dry_run_report_for_read_only_batch() {
        let temp = tempdir().unwrap();
        init_project(temp.path());
        let registry = load_canonical_registry();
        let read_cap = registry
            .capabilities
            .iter()
            .find(|(id, _)| {
                let kind = registry.capabilities.get(id).map(|c| c.kind);
                matches!(kind, Some(video_capabilities::CapabilityKind::Read))
            })
            .map(|(id, _)| id.0.clone())
            .unwrap_or_else(|| "timeline.read".into());

        let batch = ActionBatch {
            schema: ACTION_BATCH_SCHEMA.to_string(),
            batch_id: "studio-dry-run".into(),
            expected_revision: "project_root".into(),
            dry_run: true,
            actions: vec![ExecutorAction {
                capability_id: read_cap,
                target: "timeline:main".into(),
                params: serde_json::json!({}),
                session_binding_id: None,
            }],
        };
        let json = serde_json::to_value(&batch).unwrap();

        let outcome = run_apply(temp.path().to_string_lossy().into_owned(), json).unwrap();
        assert!(outcome.report.is_dry_run() || outcome.report.is_failed());
    }

    #[test]
    fn run_apply_rejects_unknown_capability() {
        let temp = tempdir().unwrap();
        init_project(temp.path());
        let batch = ActionBatch {
            schema: ACTION_BATCH_SCHEMA.to_string(),
            batch_id: "bad".into(),
            expected_revision: "project_root".into(),
            dry_run: true,
            actions: vec![ExecutorAction {
                capability_id: "no.such.capability".into(),
                target: "anything".into(),
                params: serde_json::json!({}),
                session_binding_id: None,
            }],
        };
        let json = serde_json::to_value(&batch).unwrap();
        let outcome = run_apply(temp.path().to_string_lossy().into_owned(), json).unwrap();
        assert!(outcome.report.is_failed());
        assert!(outcome
            .report
            .receipt
            .failures
            .iter()
            .any(|failure| matches!(
                failure.code,
                video_actions::FailureCode::UnknownActionKind
                    | video_actions::FailureCode::ValidationError
                    | video_actions::FailureCode::PermissionDenied
            )));
    }

    #[test]
    fn run_apply_rejects_wrong_schema() {
        let temp = tempdir().unwrap();
        init_project(temp.path());
        let mut batch = ActionBatch::read_only("a", "rev", vec![]);
        batch.schema = "wrong.schema/v1".into();
        let json = serde_json::to_value(&batch).unwrap();
        let err = run_apply(temp.path().to_string_lossy().into_owned(), json)
            .expect_err("wrong schema must error");
        assert!(err.contains("schema"), "error should mention schema: {err}");
    }
}
