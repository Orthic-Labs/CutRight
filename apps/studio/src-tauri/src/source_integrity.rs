//! Registered-source verification and relinking commands. Moved out of
//! `main.rs` per REV2 §14.5 — pure move, no behavior change.

use crate::decision_contract::{self, RelinkHistoryRecord};
use crate::decision_store::{read_sources, write_json_atomic};
use crate::project_scope::{blake3_of, canonical_project_root, named_error};
use chrono::Utc;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize)]
pub(crate) struct SourceCheck {
    pub(crate) source_id: String,
    pub(crate) path: String,
    pub(crate) expected_blake3: String,
    pub(crate) actual_blake3: Option<String>,
    pub(crate) matches: bool,
    pub(crate) error: Option<String>,
}

#[tauri::command]
pub(crate) fn verify_sources(app: AppHandle, path: String) -> Result<Vec<SourceCheck>, String> {
    let root = canonical_project_root(&path)?;
    let sources = read_sources(&root)?.sources;
    let total = sources.len();
    sources
        .into_iter()
        .enumerate()
        .map(|(index, source)| {
            let source_path = PathBuf::from(&source.path);
            match fs::File::open(&source_path) {
                Ok(mut file) => {
                    let mut hasher = blake3::Hasher::new();
                    std::io::copy(&mut file, &mut hasher)
                        .map_err(|error| format!("{}: {error}", source_path.display()))?;
                    let actual = format!("blake3:{}", hasher.finalize().to_hex());
                    let check = SourceCheck {
                        source_id: source.source_id,
                        path: source.path,
                        matches: actual == source.blake3,
                        expected_blake3: source.blake3,
                        actual_blake3: Some(actual),
                        error: None,
                    };
                    app.emit(
                        "source-verify-progress",
                        serde_json::json!({
                            "completed": index + 1,
                            "total": total,
                            "source_id": check.source_id,
                        }),
                    )
                    .map_err(|error| error.to_string())?;
                    Ok(check)
                }
                Err(error) => {
                    let check = SourceCheck {
                        source_id: source.source_id,
                        path: source.path,
                        expected_blake3: source.blake3,
                        actual_blake3: None,
                        matches: false,
                        error: Some(error.to_string()),
                    };
                    app.emit(
                        "source-verify-progress",
                        serde_json::json!({
                            "completed": index + 1,
                            "total": total,
                            "source_id": check.source_id,
                        }),
                    )
                    .map_err(|error| error.to_string())?;
                    Ok(check)
                }
            }
        })
        .collect()
}

/// Re-register a missing source at a new path. The new file is canonicalized and
/// BLAKE3-hashed; the manifest path is updated while the immutable registered
/// hash is preserved as the identity. `matches` reports whether the relinked
/// bytes still match that identity.
#[tauri::command]
/// A missing source can only be relinked to a file whose BLAKE3 matches the
/// hash it was registered with (REV2 §12.6). On a match the manifest path is
/// updated atomically; on a mismatch the manifest is left untouched and the
/// mismatch is reported instead — the caller gets the same `SourceCheck`
/// shape either way and decides what to do next. A relink is never allowed
/// to register a source id that was not already present: this always edits
/// an existing entry, never inserts one.
pub(crate) fn relink_source(
    path: String,
    source_id: String,
    new_path: String,
) -> Result<SourceCheck, String> {
    let root = canonical_project_root(&path)?;
    let manifest_rel = "sources/manifest.json";
    let manifest_path = root.join(manifest_rel);
    let mut manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(&manifest_path)
            .map_err(|error| format!("{}: {error}", manifest_path.display()))?,
    )
    .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
    let sources = manifest
        .get_mut("sources")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| named_error("sources", "manifest has no sources array"))?;
    let entry = sources
        .iter_mut()
        .find(|source| {
            source.get("source_id").and_then(serde_json::Value::as_str) == Some(source_id.as_str())
        })
        .ok_or_else(|| named_error("source_id", "no registered source with that id"))?;
    let expected_blake3 = entry
        .get("blake3")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let canonical = fs::canonicalize(&new_path).map_err(|error| named_error("new_path", error))?;
    if !canonical.is_file() {
        return Err(named_error("new_path", "must be an existing file"));
    }
    let actual_blake3 = blake3_of(&canonical)?;
    let matches = actual_blake3 == expected_blake3;
    let canonical_str = canonical.to_string_lossy().into_owned();

    if matches {
        entry["path"] = serde_json::Value::String(canonical_str.clone());
        write_json_atomic(&root, manifest_rel, &manifest)?;
    }

    decision_contract::append_relink_record(
        &root,
        &RelinkHistoryRecord {
            ts: Utc::now().to_rfc3339(),
            source_id: source_id.clone(),
            requested_path: new_path,
            canonical_path: canonical_str.clone(),
            expected_blake3: expected_blake3.clone(),
            actual_blake3: Some(actual_blake3.clone()),
            matches,
            applied: matches,
            reason: (!matches).then(|| "content_mismatch".to_string()),
        },
    )?;

    Ok(SourceCheck {
        source_id,
        path: canonical_str,
        expected_blake3,
        actual_blake3: Some(actual_blake3),
        matches,
        error: None,
    })
}
