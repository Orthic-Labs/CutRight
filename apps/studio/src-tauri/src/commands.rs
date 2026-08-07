//! Tauri command bodies: project selection, snapshot/transcript reads, and
//! the decision-ledger and variant-selection commands. Moved out of
//! `main.rs` per REV2 §14.5 — pure move, no behavior change.

pub(crate) mod apply;

use crate::artifact_state;
use crate::decision_contract::{self, DecisionIntent, DecisionRecord, DecisionReplay};
use crate::decision_store::read_sources;
use crate::project_identity;
use crate::project_scope::{
    canonical_project_root, grant_project_assets, named_error, project_revision, reframe_plan_path,
    stale_cut_plan_reason, stale_qa_reason,
};
// Aliased (not `self`) because `write_cloud_settings` below takes a
// parameter named `settings`, which would otherwise shadow the module.
use crate::settings::{self as cloud_settings, CloudSettings, EngineStatus};
use chrono::Utc;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub(crate) fn pick_project(app: AppHandle) -> Result<Option<String>, String> {
    let Some(path) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let path = path
        .into_path()
        .map_err(|error| named_error("path", error))?;
    let root = canonical_project_root(path.to_string_lossy().as_ref())?;
    // Asset scope is granted by `read_snapshot`, which the frontend always
    // calls immediately after this resolves the picked path — granting here
    // too would just re-run the same probes and hashing for no benefit.
    Ok(Some(root.to_string_lossy().into_owned()))
}

/// Read-only, Studio-facing project snapshot. Wraps
/// `video_project::project_snapshot` (which this app does not own) to add:
/// exact-file asset scope grants (§12.4), explicit artifact state for the
/// optional JSON artifacts that crate silently collapses to `None` on parse
/// failure (§12.1), a `project_revision` (§12.3), and the project's
/// immutable Studio-side identity (§12.7). The crate's original `qa`,
/// `bench`, `reframe_plan`, and per-variant `cut_plan`/`segment_count` fields
/// are left in place for compatibility; the `*_artifact` and `integrity`
/// fields alongside them carry the corrected, non-lossy state.
#[tauri::command]
pub(crate) fn read_snapshot(app: AppHandle, path: String) -> Result<serde_json::Value, String> {
    let root = canonical_project_root(&path)?;
    let snapshot = video_project::project_snapshot(&root).map_err(|error| error.to_string())?;
    let source_grants = grant_project_assets(&app, &snapshot)?;
    let identity = project_identity::resolve(&root, Some(&snapshot.manifest.project_id))?;
    let revision = project_revision(&root, &snapshot);

    let mut value = serde_json::to_value(&snapshot).map_err(|error| error.to_string())?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| "snapshot: unexpected shape".to_string())?;

    object.insert(
        "qa_artifact".into(),
        serde_json::to_value(artifact_state::load_json_checked::<serde_json::Value>(
            &root.join("qa/report.json"),
            |_| stale_qa_reason(&root, &snapshot),
        ))
        .map_err(|error| error.to_string())?,
    );
    object.insert(
        "bench_artifact".into(),
        serde_json::to_value(artifact_state::load_json_checked::<serde_json::Value>(
            &root.join("analysis/bench/transcribe/report.json"),
            |_| None,
        ))
        .map_err(|error| error.to_string())?,
    );
    object.insert(
        "reframe_plan_artifact".into(),
        serde_json::to_value(artifact_state::load_json_checked::<serde_json::Value>(
            &reframe_plan_path(&root),
            |_| None,
        ))
        .map_err(|error| error.to_string())?,
    );

    if let Some(variants) = object
        .get_mut("variants")
        .and_then(serde_json::Value::as_array_mut)
    {
        for variant in variants.iter_mut() {
            let Some(id) = variant
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
            else {
                continue;
            };
            let plan_path = root.join(format!("edit/cut-plan-{id}.json"));
            let mp4_path = root.join(format!("render/rough-cuts/{id}.mp4"));
            let state = artifact_state::load_json_checked::<serde_json::Value>(&plan_path, |_| {
                stale_cut_plan_reason(&plan_path, &mp4_path)
            });
            if let Some(variant_object) = variant.as_object_mut() {
                variant_object.insert(
                    "cut_plan_artifact".into(),
                    serde_json::to_value(&state).map_err(|error| error.to_string())?,
                );
            }
        }
    }

    if let Some(sources) = object
        .get_mut("sources")
        .and_then(serde_json::Value::as_array_mut)
    {
        for source in sources.iter_mut() {
            let grant = source
                .get("source_id")
                .and_then(serde_json::Value::as_str)
                .and_then(|id| source_grants.iter().find(|grant| grant.source_id == id));
            if let Some(source_object) = source.as_object_mut() {
                source_object.insert(
                    "integrity".into(),
                    serde_json::to_value(grant).map_err(|error| error.to_string())?,
                );
            }
        }
    }

    object.insert(
        "project_instance_id".into(),
        serde_json::Value::String(identity.project_instance_id),
    );
    object.insert(
        "project_revision".into(),
        serde_json::Value::String(revision),
    );

    Ok(value)
}

#[tauri::command]
pub(crate) fn read_transcript(path: String, variant: String) -> Result<serde_json::Value, String> {
    let root = canonical_project_root(&path)?;
    let transcript = match variant.as_str() {
        "tight" | "natural" => root.join(format!("edit/output-transcript-{variant}.json")),
        source_id => {
            let source_exists = read_sources(&root)?
                .sources
                .iter()
                .any(|source| source.source_id == source_id);
            if !source_exists {
                return Err(named_error(
                    "variant",
                    "must be tight, natural, or a registered source id",
                ));
            }
            root.join("analysis/transcripts")
                .join(format!("{source_id}.json"))
        }
    };
    let bytes =
        std::fs::read(&transcript).map_err(|error| format!("{}: {error}", transcript.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", transcript.display()))
}

/// Accept a minimal review intent and persist the authoritative record the
/// backend constructs. Returns the persisted record so the frontend stores
/// exactly what was written.
#[tauri::command]
pub(crate) fn append_decision(
    app: AppHandle,
    path: String,
    intent: DecisionIntent,
) -> Result<DecisionRecord, String> {
    let root = canonical_project_root(&path)?;
    let app_version = app.package_info().version.to_string();
    decision_contract::apply_intent(&root, &intent, &app_version, Utc::now())
}

#[tauri::command]
pub(crate) fn read_decisions(path: String) -> Result<DecisionReplay, String> {
    let root = canonical_project_root(&path)?;
    decision_contract::replay(&root)
}

/// Record the reviewed-base selection that gates final rendering. Wraps
/// `video_project::select_variant` with `selected_by = "studio"`.
#[tauri::command]
pub(crate) fn select_variant(
    path: String,
    variant: String,
) -> Result<video_project::SelectionRecord, String> {
    let root = canonical_project_root(&path)?;
    video_project::select_variant(&root, &variant, "studio").map_err(|error| error.to_string())
}

/// Read the current reviewed-base selection, if any.
#[tauri::command]
pub(crate) fn read_variant_selection(
    path: String,
) -> Result<Option<video_project::SelectionRecord>, String> {
    let root = canonical_project_root(&path)?;
    video_project::read_variant_selection(&root).map_err(|error| error.to_string())
}

/// Read this project's cloud-analysis settings (REV2 §15.6), or the
/// consent-off defaults if none have been saved yet.
#[tauri::command]
pub(crate) fn read_cloud_settings(path: String) -> Result<CloudSettings, String> {
    let root = canonical_project_root(&path)?;
    cloud_settings::read(&root)
}

/// Validate and persist this project's cloud-analysis settings. Returns the
/// persisted record (with `updated_at` stamped) so the frontend stores
/// exactly what was written, matching `append_decision`'s pattern.
#[tauri::command]
pub(crate) fn write_cloud_settings(
    path: String,
    settings: CloudSettings,
) -> Result<CloudSettings, String> {
    let root = canonical_project_root(&path)?;
    cloud_settings::write(&root, settings)
}

/// The REV2 §15.6 retention/deletion action: reset this project's cloud
/// settings to defaults (consent off) and remove any cloud analysis cache.
#[tauri::command]
pub(crate) fn delete_cloud_data(path: String) -> Result<CloudSettings, String> {
    let root = canonical_project_root(&path)?;
    cloud_settings::delete(&root)
}

/// Whether `name` is currently set in the engine's own process environment
/// — presence only, never the value. See `settings.rs` module docs.
#[tauri::command]
pub(crate) fn credential_env_var_present(name: String) -> Result<bool, String> {
    cloud_settings::env_var_present(&name)
}

/// Cheap, read-only engine identity facts (resolved FFmpeg/FFprobe
/// toolchain) for the Settings surface. Takes no project path — this is
/// machine/engine state, not per-project state.
#[tauri::command]
pub(crate) fn read_engine_status() -> EngineStatus {
    cloud_settings::engine_status()
}
