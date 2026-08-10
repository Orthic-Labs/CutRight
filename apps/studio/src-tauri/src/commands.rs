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
pub(crate) fn rightkit_app_info() -> serde_json::Value {
    serde_json::json!({"schema_version":1,"app":"cutright","tier":"free","license":"Proprietary","offline":true,"telemetry":false,"updates":"disabled-until-configured"})
}

#[tauri::command]
pub(crate) fn rightkit_logs_write(
    path: String,
    events: Vec<serde_json::Value>,
) -> Result<(), String> {
    let root = canonical_project_root(&path)?;
    let store = rightkit_logs::JsonlStore::new(
        root.join(".cutright-tools/rightkit-events.jsonl"),
        1_048_576,
    );
    for event in events {
        store.append_value(&event).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn rightkit_logs_collect(path: String) -> Result<Vec<serde_json::Value>, String> {
    let root = canonical_project_root(&path)?;
    let file = root.join(".cutright-tools/rightkit-events.jsonl");
    if !file.exists() {
        return Ok(vec![]);
    }
    std::fs::read_to_string(file)
        .map_err(|e| e.to_string())
        .map(|text| {
            text.lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect()
        })
}

#[tauri::command]
pub(crate) fn rightkit_logs_clear(path: String) -> Result<(), String> {
    let root = canonical_project_root(&path)?;
    rightkit_logs::JsonlStore::new(
        root.join(".cutright-tools/rightkit-events.jsonl"),
        1_048_576,
    )
    .clear()
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod rightkit_tests {
    use super::*;
    #[test]
    fn app_info_is_free_offline_cutright() {
        let info = rightkit_app_info();
        assert_eq!(info["app"], "cutright");
        assert_eq!(info["tier"], "free");
        assert_eq!(info["offline"], true);
        assert_eq!(info["telemetry"], false);
    }
    #[test]
    fn logs_write_collect_clear_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("project.json"), "{}").unwrap();
        let path = dir.path().to_string_lossy().into_owned();
        rightkit_logs_write(path.clone(), vec![serde_json::json!({"event":"opened"})]).unwrap();
        let rows = rightkit_logs_collect(path.clone()).unwrap();
        assert_eq!(rows.len(), 1);
        rightkit_logs_clear(path.clone()).unwrap();
        assert!(rightkit_logs_collect(path).unwrap().is_empty());
    }
}

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

#[tauri::command]
pub(crate) fn finish_commit_variant(
    path: String,
    variant_id: String,
    locked_cut_hash: String,
    source_hashes: Vec<String>,
) -> Result<serde_json::Value, String> {
    const VARIANTS: [&str; 5] = ["balanced", "pullback", "punch", "push", "editor-takeover"];
    let valid_hash = |value: &str| {
        value.len() == 71
            && value.starts_with("blake3:")
            && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
    };
    if !VARIANTS.contains(&variant_id.as_str())
        || source_hashes.is_empty()
        || source_hashes.iter().any(|hash| !valid_hash(hash))
    {
        return Err("finish variant requires a known id and source hashes".into());
    }
    let root = canonical_project_root(&path)?;
    let current = project_revision(
        &root,
        &video_project::project_snapshot(&root).map_err(|e| e.to_string())?,
    );
    if current != locked_cut_hash {
        return Err("stale_locked_cut".into());
    }
    let value = serde_json::json!({"schemaVersion":1,"variantId":variant_id,"lockedCutHash":locked_cut_hash,"sourceHashes":source_hashes});
    let dir = root.join("finish");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tmp = dir.join("selection.json.tmp");
    let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    use std::io::Write as _;
    file.write_all(&serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, dir.join("selection.json")).map_err(|e| e.to_string())?;
    std::fs::File::open(&dir)
        .and_then(|directory| directory.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(value)
}

#[tauri::command]
pub(crate) fn finish_read_selection(path: String) -> Result<Option<serde_json::Value>, String> {
    let root = canonical_project_root(&path)?;
    let file = root.join("finish/selection.json");
    if !file.exists() {
        return Ok(None);
    }
    serde_json::from_slice(&std::fs::read(file).map_err(|e| e.to_string())?)
        .map(Some)
        .map_err(|e| e.to_string())
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
