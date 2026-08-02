use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_dialog::DialogExt;

mod artifact_state;
mod decision_contract;
mod project_identity;

use decision_contract::{DecisionIntent, DecisionRecord, DecisionReplay, RelinkHistoryRecord};

#[derive(Debug, Deserialize)]
struct SourcesManifest {
    sources: Vec<RegisteredSource>,
}

#[derive(Debug, Deserialize)]
struct RegisteredSource {
    source_id: String,
    path: String,
    blake3: String,
}

#[derive(Debug, Serialize)]
struct SourceCheck {
    source_id: String,
    path: String,
    expected_blake3: String,
    actual_blake3: Option<String>,
    matches: bool,
    error: Option<String>,
}

fn named_error(field: &str, message: impl std::fmt::Display) -> String {
    format!("{field}: {message}")
}

fn canonical_project_root(path: &str) -> Result<PathBuf, String> {
    let root = fs::canonicalize(path).map_err(|error| named_error("path", error))?;
    if !root.is_dir() {
        return Err(named_error("path", "must be a project directory"));
    }
    let manifest = root.join("project.json");
    if !manifest.is_file() {
        return Err(named_error("path", "project.json is missing"));
    }
    Ok(root)
}

fn read_sources(root: &Path) -> Result<SourcesManifest, String> {
    let path = root.join("sources/manifest.json");
    if !path.exists() {
        return Ok(SourcesManifest {
            sources: Vec::new(),
        });
    }
    let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))
}

fn write_json_atomic(root: &Path, rel: &str, value: &serde_json::Value) -> Result<(), String> {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    let temporary = path.with_extension(format!("json.tmp-{}", std::process::id()));
    {
        use std::io::Write;
        let mut file = fs::File::create(&temporary)
            .map_err(|error| format!("{}: {error}", temporary.display()))?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("{}: {error}", temporary.display()))?;
    }
    fs::rename(&temporary, &path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(())
}

fn is_regular_file(path: &Path) -> bool {
    fs::metadata(path)
        .map(|meta| meta.is_file())
        .unwrap_or(false)
}

fn blake3_of(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut file, &mut hasher)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

/// Outcome of granting scope to one registered source's media file.
#[derive(Debug, Clone, Serialize)]
struct SourceGrant {
    source_id: String,
    path: String,
    /// The asset-protocol scope was extended to cover this exact file.
    granted: bool,
    /// The file's current BLAKE3 matches the hash it was registered with.
    /// `false` does not block `granted` — a source can still be played back
    /// while flagged unverified, per REV2 §12.4 ("manifest hash match or an
    /// explicit unverified state before playback"); the frontend is
    /// responsible for surfacing this rather than silently trusting it.
    verified: bool,
    error: Option<String>,
}

/// Grant the asset protocol access to exactly the files the current project
/// state needs: registered source media, produced rough-cut/final MP4s, and
/// per-source poster/waveform evidence. Replaces the previous
/// `allow_directory(root, true)`, which handed a shared/untrusted project
/// package the ability to grant arbitrary local paths merely by editing
/// `sources/manifest.json` (REV2 §12.4). Evidence artifacts under the
/// project root are safe to grant on existence alone (they are only ever
/// written there by the pipeline); external source media additionally
/// requires the path to resolve to a regular file and to probe as supported
/// media before scope is extended to it.
fn grant_project_assets<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: &video_project::ProjectSnapshot,
) -> Result<Vec<SourceGrant>, String> {
    let scope = app.asset_protocol_scope();

    let mut evidence_paths: Vec<PathBuf> = Vec::new();
    for variant in &snapshot.variants {
        if let Some(mp4) = &variant.mp4 {
            evidence_paths.push(mp4.clone());
        }
    }
    for final_snapshot in &snapshot.finals {
        evidence_paths.push(final_snapshot.mp4.clone());
    }
    for entry in &snapshot.sources {
        if let Some(poster) = &entry.poster_jpg {
            evidence_paths.push(poster.clone());
        }
        if let Some(waveform) = &entry.waveform_png {
            evidence_paths.push(waveform.clone());
        }
    }
    for path in &evidence_paths {
        if is_regular_file(path) {
            scope
                .allow_file(path)
                .map_err(|error| format!("asset scope for {}: {error}", path.display()))?;
        }
    }

    let mut source_grants = Vec::with_capacity(snapshot.sources.len());
    for entry in &snapshot.sources {
        let source = &entry.source;
        let requested = Path::new(&source.path);
        let canonical = match fs::canonicalize(requested) {
            Ok(path) => path,
            Err(error) => {
                source_grants.push(SourceGrant {
                    source_id: source.source_id.clone(),
                    path: source.path.clone(),
                    granted: false,
                    verified: false,
                    error: Some(error.to_string()),
                });
                continue;
            }
        };
        if !is_regular_file(&canonical) {
            source_grants.push(SourceGrant {
                source_id: source.source_id.clone(),
                path: canonical.to_string_lossy().into_owned(),
                granted: false,
                verified: false,
                error: Some("registered source is not a regular file".into()),
            });
            continue;
        }
        if let Err(error) = video_media::probe(&canonical) {
            source_grants.push(SourceGrant {
                source_id: source.source_id.clone(),
                path: canonical.to_string_lossy().into_owned(),
                granted: false,
                verified: false,
                error: Some(format!("unsupported media: {error}")),
            });
            continue;
        }
        let verified = blake3_of(&canonical)
            .map(|actual| actual == source.blake3)
            .unwrap_or(false);
        match scope.allow_file(&canonical) {
            Ok(()) => source_grants.push(SourceGrant {
                source_id: source.source_id.clone(),
                path: canonical.to_string_lossy().into_owned(),
                granted: true,
                verified,
                error: None,
            }),
            Err(error) => source_grants.push(SourceGrant {
                source_id: source.source_id.clone(),
                path: canonical.to_string_lossy().into_owned(),
                granted: false,
                verified,
                error: Some(error.to_string()),
            }),
        }
    }

    Ok(source_grants)
}

#[tauri::command]
fn pick_project(app: AppHandle) -> Result<Option<String>, String> {
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

/// Best-effort staleness check: a cut plan that was edited after its rough
/// cut was last rendered no longer describes what is on disk.
fn stale_cut_plan_reason(plan_path: &Path, mp4_path: &Path) -> Option<String> {
    let plan_mtime = fs::metadata(plan_path)
        .and_then(|meta| meta.modified())
        .ok()?;
    let mp4_mtime = fs::metadata(mp4_path)
        .and_then(|meta| meta.modified())
        .ok()?;
    (plan_mtime > mp4_mtime)
        .then(|| "cut plan was modified after the rough cut was last rendered".to_string())
}

/// Best-effort staleness check: a QA report generated before the newest
/// final render no longer covers what would ship.
fn stale_qa_reason(root: &Path, snapshot: &video_project::ProjectSnapshot) -> Option<String> {
    let qa_mtime = fs::metadata(root.join("qa/report.json"))
        .and_then(|meta| meta.modified())
        .ok()?;
    let newest_final = snapshot
        .finals
        .iter()
        .filter_map(|final_snapshot| {
            fs::metadata(&final_snapshot.mp4)
                .and_then(|meta| meta.modified())
                .ok()
        })
        .max()?;
    (newest_final > qa_mtime)
        .then(|| "a final was rendered after this QA report was generated".to_string())
}

fn file_signature(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let since_epoch = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(format!("{}:{}", metadata.len(), since_epoch.as_nanos()))
}

/// A hash over the canonical review inputs and artifact receipts (REV2
/// §12.3): the project identity plus a cheap size+mtime signature of every
/// variant, final, and the QA/bench reports that currently exist. This is
/// deliberately not a full content hash of every rendered MP4 on every
/// snapshot read — that would make opening a project with large renders
/// noticeably slower — but it changes whenever an artifact a reviewer would
/// look at is added, replaced, or removed, which `generated_at` alone (a
/// timestamp of the read, not of the data) cannot signal.
fn project_revision(root: &Path, snapshot: &video_project::ProjectSnapshot) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(snapshot.manifest.project_id.as_bytes());
    for variant in &snapshot.variants {
        hasher.update(variant.id.as_bytes());
        if let Some(mp4) = &variant.mp4 {
            if let Some(signature) = file_signature(mp4) {
                hasher.update(signature.as_bytes());
            }
        }
    }
    for final_snapshot in &snapshot.finals {
        hasher.update(final_snapshot.preset.as_bytes());
        if let Some(signature) = file_signature(&final_snapshot.mp4) {
            hasher.update(signature.as_bytes());
        }
    }
    for path in [
        root.join("qa/report.json"),
        root.join("analysis/bench/transcribe/report.json"),
    ] {
        if let Some(signature) = file_signature(&path) {
            hasher.update(signature.as_bytes());
        }
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}

fn reframe_plan_path(root: &Path) -> PathBuf {
    let primary = root.join("analysis/reframe-plan.json");
    if primary.is_file() {
        primary
    } else {
        root.join("analysis/reframe/natural/reframe-plan.json")
    }
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
fn read_snapshot(app: AppHandle, path: String) -> Result<serde_json::Value, String> {
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
fn read_transcript(path: String, variant: String) -> Result<serde_json::Value, String> {
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
        fs::read(&transcript).map_err(|error| format!("{}: {error}", transcript.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", transcript.display()))
}

/// Accept a minimal review intent and persist the authoritative record the
/// backend constructs. Returns the persisted record so the frontend stores
/// exactly what was written.
#[tauri::command]
fn append_decision(
    app: AppHandle,
    path: String,
    intent: DecisionIntent,
) -> Result<DecisionRecord, String> {
    let root = canonical_project_root(&path)?;
    let app_version = app.package_info().version.to_string();
    decision_contract::apply_intent(&root, &intent, &app_version, Utc::now())
}

#[tauri::command]
fn read_decisions(path: String) -> Result<DecisionReplay, String> {
    let root = canonical_project_root(&path)?;
    decision_contract::replay(&root)
}

#[tauri::command]
fn verify_sources(app: AppHandle, path: String) -> Result<Vec<SourceCheck>, String> {
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

/// Record the reviewed-base selection that gates final rendering. Wraps
/// `video_project::select_variant` with `selected_by = "studio"`.
#[tauri::command]
fn select_variant(path: String, variant: String) -> Result<video_project::SelectionRecord, String> {
    let root = canonical_project_root(&path)?;
    video_project::select_variant(&root, &variant, "studio").map_err(|error| error.to_string())
}

/// Read the current reviewed-base selection, if any.
#[tauri::command]
fn read_variant_selection(path: String) -> Result<Option<video_project::SelectionRecord>, String> {
    let root = canonical_project_root(&path)?;
    video_project::read_variant_selection(&root).map_err(|error| error.to_string())
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
fn relink_source(path: String, source_id: String, new_path: String) -> Result<SourceCheck, String> {
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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            pick_project,
            read_snapshot,
            read_transcript,
            append_decision,
            read_decisions,
            verify_sources,
            select_variant,
            read_variant_selection,
            relink_source
        ])
        .run(tauri::generate_context!())
        .expect("CutRight Studio failed to start");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision_contract::{
        apply_intent, build_record, replay, DecisionIntent, DecisionVerdict, RecordStatus,
        ReviewReason, ReviewTarget, SCHEMA_VERSION,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_TEST: AtomicUsize = AtomicUsize::new(0);

    fn project() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "cutright-studio-test-{}-{}",
            std::process::id(),
            NEXT_TEST.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("feedback")).unwrap();
        fs::create_dir_all(root.join("render/rough-cuts")).unwrap();
        fs::create_dir_all(root.join("render/finals")).unwrap();
        fs::create_dir_all(root.join("qa")).unwrap();
        fs::write(
            root.join("project.json"),
            r#"{"project_id":"project-test"}"#,
        )
        .unwrap();
        fs::write(root.join("render/rough-cuts/natural.mp4"), b"natural-bytes").unwrap();
        fs::write(root.join("render/rough-cuts/tight.mp4"), b"tight-bytes").unwrap();
        fs::write(root.join("render/finals/youtube.mp4"), b"youtube-bytes").unwrap();
        fs::write(root.join("qa/report.json"), br#"{"status":"pass"}"#).unwrap();
        // Canonicalize so later `starts_with(root)` containment checks agree
        // with the canonicalized paths those checks compare against (macOS
        // resolves `$TMPDIR` through a `/var` -> `/private/var` symlink, so
        // the two would otherwise disagree on this platform).
        fs::canonicalize(&root).unwrap()
    }

    fn variant_intent(variant: &str, request_id: &str) -> DecisionIntent {
        DecisionIntent {
            schema_version: SCHEMA_VERSION,
            client_request_id: request_id.into(),
            target: ReviewTarget::Variant {
                variant: variant.into(),
            },
            verdict: DecisionVerdict::Approved,
            reason: ReviewReason::Pacing,
            note: None,
            playhead_ms: 1000,
            word_id: None,
            source_word_id: None,
        }
    }

    #[test]
    fn variant_approval_appends_and_replays() {
        let root = project();
        let record = apply_intent(
            &root,
            &variant_intent("natural", "req-1"),
            "0.1.0",
            Utc::now(),
        )
        .unwrap();
        assert_eq!(record.kind, "variant_verdict");
        assert_eq!(record.subject, "render/rough-cuts/natural.mp4");
        assert_eq!(record.variant.as_deref(), Some("natural"));

        let replay = replay(&root).unwrap();
        assert_eq!(replay.records.len(), 1);
        assert!(replay.malformed_lines.is_empty());
        assert_eq!(replay.records[0].status, RecordStatus::Current);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn final_approval_appends_and_replays() {
        let root = project();
        let intent = DecisionIntent {
            schema_version: SCHEMA_VERSION,
            client_request_id: "req-final".into(),
            target: ReviewTarget::Final {
                preset: "youtube".into(),
            },
            verdict: DecisionVerdict::Approved,
            reason: ReviewReason::LooksRight,
            note: None,
            playhead_ms: 0,
            word_id: None,
            source_word_id: None,
        };
        let record = apply_intent(&root, &intent, "0.1.0", Utc::now()).unwrap();
        assert_eq!(record.kind, "final_verdict");
        assert_eq!(record.subject, "render/finals/youtube.mp4");
        assert_eq!(record.preset.as_deref(), Some("youtube"));

        let replay = replay(&root).unwrap();
        assert_eq!(replay.records.len(), 1);
        assert_eq!(replay.records[0].status, RecordStatus::Current);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn other_reason_retains_its_note() {
        let root = project();
        let mut intent = variant_intent("natural", "req-note");
        intent.reason = ReviewReason::Other;
        intent.note = Some("  pacing felt rushed near the hook  ".into());
        let record = apply_intent(&root, &intent, "0.1.0", Utc::now()).unwrap();
        assert_eq!(record.reason, "other");
        assert_eq!(
            record.note.as_deref(),
            Some("pacing felt rushed near the hook")
        );

        let replay = replay(&root).unwrap();
        assert_eq!(
            replay.records[0].record.note.as_deref(),
            Some("pacing felt rushed near the hook")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_a_reason_that_does_not_belong_to_the_target() {
        let root = project();
        let mut intent = variant_intent("natural", "req-bad-reason");
        intent.reason = ReviewReason::LooksRight; // final-only reason
        let error = apply_intent(&root, &intent, "0.1.0", Utc::now()).unwrap_err();
        assert!(error.starts_with("reason:"), "got: {error}");

        let mut intent = variant_intent("natural", "req-bad-note");
        intent.note = Some("stray note".into()); // note without other
        let error = apply_intent(&root, &intent, "0.1.0", Utc::now()).unwrap_err();
        assert!(error.starts_with("note:"), "got: {error}");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn subject_is_canonical_and_cannot_be_injected() {
        // The intent carries no subject field at all; the backend derives it
        // from the target, so absolute or traversal subjects are unrepresentable.
        let root = project();
        let record = build_record(
            &root,
            &variant_intent("natural", "req-subject"),
            "0.1.0",
            Utc::now(),
        )
        .unwrap();
        assert_eq!(record.subject, "render/rough-cuts/natural.mp4");
        assert!(!Path::new(&record.subject).is_absolute());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_client_request_id_is_idempotent() {
        let root = project();
        let first = apply_intent(
            &root,
            &variant_intent("natural", "req-dup"),
            "0.1.0",
            Utc::now(),
        )
        .unwrap();
        let second = apply_intent(
            &root,
            &variant_intent("natural", "req-dup"),
            "0.1.0",
            Utc::now(),
        )
        .unwrap();
        assert_eq!(first.decision_id, second.decision_id);
        let replay = replay(&root).unwrap();
        assert_eq!(replay.records.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn concurrent_appends_produce_complete_json_lines() {
        let root = project();
        let mut handles = Vec::new();
        for i in 0..8 {
            let root = root.clone();
            handles.push(std::thread::spawn(move || {
                let intent = variant_intent("natural", &format!("req-concurrent-{i}"));
                apply_intent(&root, &intent, "0.1.0", Utc::now()).unwrap();
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        let replay = replay(&root).unwrap();
        assert_eq!(replay.records.len(), 8);
        assert!(replay.malformed_lines.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stale_artifact_remains_in_replay() {
        let root = project();
        apply_intent(
            &root,
            &variant_intent("natural", "req-stale"),
            "0.1.0",
            Utc::now(),
        )
        .unwrap();
        // Re-render the rough cut so its bytes (and hash) change.
        fs::write(root.join("render/rough-cuts/natural.mp4"), b"re-rendered").unwrap();
        let replay = replay(&root).unwrap();
        assert_eq!(replay.records.len(), 1);
        assert_eq!(replay.records[0].status, RecordStatus::StaleArtifact);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_artifact_is_reported_not_dropped() {
        let root = project();
        apply_intent(
            &root,
            &variant_intent("tight", "req-missing"),
            "0.1.0",
            Utc::now(),
        )
        .unwrap();
        fs::remove_file(root.join("render/rough-cuts/tight.mp4")).unwrap();
        let replay = replay(&root).unwrap();
        assert_eq!(replay.records.len(), 1);
        assert_eq!(replay.records[0].status, RecordStatus::MissingArtifact);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_newer_verdict_supersedes_the_older_one() {
        let root = project();
        apply_intent(
            &root,
            &variant_intent("natural", "req-v1"),
            "0.1.0",
            Utc::now(),
        )
        .unwrap();
        let mut reject = variant_intent("natural", "req-v2");
        reject.verdict = DecisionVerdict::Rejected;
        reject.reason = ReviewReason::Energy;
        apply_intent(&root, &reject, "0.1.0", Utc::now()).unwrap();
        let replay = replay(&root).unwrap();
        assert_eq!(replay.records.len(), 2);
        assert_eq!(replay.records[0].status, RecordStatus::Superseded);
        assert_eq!(replay.records[1].status, RecordStatus::Current);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_tail_is_reported() {
        let root = project();
        apply_intent(
            &root,
            &variant_intent("natural", "req-ok"),
            "0.1.0",
            Utc::now(),
        )
        .unwrap();
        let decisions = root.join("feedback/decisions.jsonl");
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&decisions)
            .unwrap();
        std::io::Write::write_all(&mut file, b"not json\n").unwrap();
        let replay = replay(&root).unwrap();
        assert_eq!(replay.records.len(), 1);
        assert_eq!(replay.malformed_lines.len(), 1);
        assert_eq!(replay.malformed_lines[0].content, "not json");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn backend_derived_provenance_is_present() {
        let root = project();
        fs::create_dir_all(root.join("analysis/bench/transcribe")).unwrap();
        fs::write(
            root.join("analysis/bench/transcribe/report.json"),
            br#"{"decision":"primary"}"#,
        )
        .unwrap();
        let record = apply_intent(
            &root,
            &variant_intent("natural", "req-prov"),
            "9.9.9",
            Utc::now(),
        )
        .unwrap();
        assert_eq!(record.app_version, "9.9.9");
        assert_eq!(record.project_id, "project-test");
        assert!(record.bench_resolved);
        assert!(record.bench_report_blake3.is_some());
        assert!(record.subject_blake3.is_some());
        assert!(record.subject_size.is_some());
        assert!(record.project_revision.is_some());
        assert!(record.decision_id.starts_with("d_"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unresolved_benchmark_is_recorded_as_unresolved() {
        let root = project();
        let record = apply_intent(
            &root,
            &variant_intent("natural", "req-bench"),
            "0.1.0",
            Utc::now(),
        )
        .unwrap();
        assert!(!record.bench_resolved);
        assert!(record.bench_report_blake3.is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn frontend_intent_fixture_round_trips_through_rust() {
        // The exact JSON shape the Studio frontend sends over IPC.
        let json = r#"{
            "schema_version": 1,
            "client_request_id": "req-frontend",
            "target": { "target_kind": "variant", "variant": "natural" },
            "verdict": "approved",
            "reason": "pacing",
            "playhead_ms": 1500,
            "word_id": "ow_000003",
            "source_word_id": "source-a:w_000003"
        }"#;
        let intent: DecisionIntent = serde_json::from_str(json).unwrap();
        let root = project();
        let record = apply_intent(&root, &intent, "0.1.0", Utc::now()).unwrap();
        assert_eq!(record.word_id.as_deref(), Some("ow_000003"));
        assert_eq!(record.source_word_id.as_deref(), Some("source-a:w_000003"));
        assert_eq!(record.verdict, "approved");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_misformatted_word_ids() {
        let root = project();
        let mut intent = variant_intent("natural", "req-wordid");
        intent.word_id = Some("ow_3".into());
        let error = apply_intent(&root, &intent, "0.1.0", Utc::now()).unwrap_err();
        assert!(error.starts_with("word_id:"), "got: {error}");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_traversal_project_roots() {
        let root = project();
        let traversal = root.join("../../etc");
        let traversal = traversal.to_string_lossy().into_owned();
        assert!(canonical_project_root(&traversal).is_err());
        assert!(read_decisions(traversal).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_symlinked_feedback_directory_that_escapes_the_project() {
        use std::os::unix::fs::symlink;

        let root = project();
        let outside = std::env::temp_dir().join(format!(
            "cutright-studio-outside-{}",
            NEXT_TEST.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&outside).unwrap();
        fs::remove_dir(root.join("feedback")).unwrap();
        symlink(&outside, root.join("feedback")).unwrap();

        assert!(apply_intent(
            &root,
            &variant_intent("natural", "req-sym"),
            "0.1.0",
            Utc::now()
        )
        .is_err());

        fs::remove_file(root.join("feedback")).unwrap();
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn select_variant_writes_a_hash_bound_record_and_reads_back() {
        let root = project();
        let record = select_variant(root.to_string_lossy().into_owned(), "natural".into()).unwrap();
        assert_eq!(record.variant, "natural");
        assert_eq!(record.rough_cut_path, "render/rough-cuts/natural.mp4");
        assert_eq!(record.selected_by, "studio");
        assert_eq!(record.rough_cut_size, b"natural-bytes".len() as u64);
        let expected = format!("blake3:{}", blake3::hash(b"natural-bytes").to_hex());
        assert_eq!(record.rough_cut_blake3, expected);

        let read = read_variant_selection(root.to_string_lossy().into_owned()).unwrap();
        let read = read.expect("selection should persist");
        assert_eq!(read.variant, "natural");
        assert_eq!(read.rough_cut_blake3, expected);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn select_variant_rejects_an_invalid_or_missing_rough_cut() {
        let root = project();
        let error = select_variant(root.to_string_lossy().into_owned(), "wide".into()).unwrap_err();
        assert!(error.contains("variant"), "got: {error}");

        fs::remove_file(root.join("render/rough-cuts/tight.mp4")).unwrap();
        let error =
            select_variant(root.to_string_lossy().into_owned(), "tight".into()).unwrap_err();
        assert!(error.contains("tight"), "got: {error}");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relink_source_updates_the_manifest_path_and_reports_match() {
        let root = project();
        let media = root.join("relinked.mov");
        fs::write(&media, b"source-bytes").unwrap();
        let hash = format!("blake3:{}", blake3::hash(b"source-bytes").to_hex());
        fs::create_dir_all(root.join("sources")).unwrap();
        fs::write(
            root.join("sources/manifest.json"),
            serde_json::json!({
                "schema_version": 1,
                "sources": [{ "source_id": "source-a", "path": "/missing/old.mov", "blake3": hash }]
            })
            .to_string(),
        )
        .unwrap();

        let check = relink_source(
            root.to_string_lossy().into_owned(),
            "source-a".into(),
            media.to_string_lossy().into_owned(),
        )
        .unwrap();
        assert!(check.matches);
        assert_eq!(check.expected_blake3, hash);
        assert_eq!(check.actual_blake3.as_deref(), Some(hash.as_str()));

        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("sources/manifest.json")).unwrap()).unwrap();
        assert_eq!(
            manifest["sources"][0]["path"].as_str().unwrap(),
            fs::canonicalize(&media).unwrap().to_string_lossy()
        );
        // The immutable identity hash is preserved, not overwritten.
        assert_eq!(manifest["sources"][0]["blake3"].as_str().unwrap(), hash);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relink_source_reports_a_content_mismatch_without_mutating_the_manifest() {
        let root = project();
        let media = root.join("different.mov");
        fs::write(&media, b"different-bytes").unwrap();
        let registered = format!("blake3:{}", blake3::hash(b"original").to_hex());
        fs::create_dir_all(root.join("sources")).unwrap();
        fs::write(
            root.join("sources/manifest.json"),
            serde_json::json!({
                "schema_version": 1,
                "sources": [{ "source_id": "source-a", "path": "/missing/old.mov", "blake3": registered }]
            })
            .to_string(),
        )
        .unwrap();

        let check = relink_source(
            root.to_string_lossy().into_owned(),
            "source-a".into(),
            media.to_string_lossy().into_owned(),
        )
        .unwrap();
        assert!(!check.matches);
        assert_eq!(check.expected_blake3, registered);

        // A rejected relink must not touch the manifest: the old (missing)
        // path stays registered, never silently swapped for unverified bytes.
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("sources/manifest.json")).unwrap()).unwrap();
        assert_eq!(
            manifest["sources"][0]["path"].as_str().unwrap(),
            "/missing/old.mov"
        );

        // The rejected attempt is still recorded in the append-only history.
        let history = fs::read_to_string(root.join("feedback/relink-history.jsonl")).unwrap();
        let record: serde_json::Value =
            serde_json::from_str(history.lines().next().unwrap()).unwrap();
        assert_eq!(record["applied"], serde_json::json!(false));
        assert_eq!(record["matches"], serde_json::json!(false));
        assert_eq!(record["source_id"], serde_json::json!("source-a"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relink_source_records_a_successful_attempt_in_the_history_ledger() {
        let root = project();
        let media = root.join("relinked.mov");
        fs::write(&media, b"source-bytes").unwrap();
        let hash = format!("blake3:{}", blake3::hash(b"source-bytes").to_hex());
        fs::create_dir_all(root.join("sources")).unwrap();
        fs::write(
            root.join("sources/manifest.json"),
            serde_json::json!({
                "schema_version": 1,
                "sources": [{ "source_id": "source-a", "path": "/missing/old.mov", "blake3": hash }]
            })
            .to_string(),
        )
        .unwrap();

        relink_source(
            root.to_string_lossy().into_owned(),
            "source-a".into(),
            media.to_string_lossy().into_owned(),
        )
        .unwrap();

        let history = fs::read_to_string(root.join("feedback/relink-history.jsonl")).unwrap();
        let record: serde_json::Value =
            serde_json::from_str(history.lines().next().unwrap()).unwrap();
        assert_eq!(record["applied"], serde_json::json!(true));
        assert_eq!(record["matches"], serde_json::json!(true));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relink_source_never_creates_a_source_id_that_was_not_already_registered() {
        let root = project();
        let media = root.join("relinked.mov");
        fs::write(&media, b"source-bytes").unwrap();
        fs::create_dir_all(root.join("sources")).unwrap();
        fs::write(
            root.join("sources/manifest.json"),
            serde_json::json!({ "schema_version": 1, "sources": [] }).to_string(),
        )
        .unwrap();

        let error = relink_source(
            root.to_string_lossy().into_owned(),
            "never-registered".into(),
            media.to_string_lossy().into_owned(),
        )
        .unwrap_err();
        assert!(error.starts_with("source_id:"), "got: {error}");

        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("sources/manifest.json")).unwrap()).unwrap();
        assert!(manifest["sources"].as_array().unwrap().is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    /// A full `project.json` that satisfies `video_project::project_snapshot`'s
    /// strict `ProjectManifest` deserialization, on top of the lighter fixture
    /// `project()` builds for the decision-contract tests above.
    fn full_project() -> PathBuf {
        let root = project();
        fs::write(
            root.join("project.json"),
            serde_json::json!({
                "schema_version": 1,
                "project_id": "project-test",
                "kind": "mixed_creator_content",
                "created_at": Utc::now().to_rfc3339(),
                "review_mode": "reviewed",
                "source_policy": "immutable",
                "outputs": [{ "id": "youtube", "aspect": "16:9", "width": 1920, "height": 1080 }],
            })
            .to_string(),
        )
        .unwrap();
        root
    }

    fn scratch_sibling(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "cutright-studio-sibling-{}-{}-{name}",
            std::process::id(),
            NEXT_TEST.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, b"not part of any project").unwrap();
        path
    }

    /// REV2 §12.5: an allowed project preview loads through the asset
    /// protocol, and a sibling/outside file is denied. Exercises the real
    /// `tauri::scope::fs::Scope` the packaged app enforces (via
    /// `tauri::test`'s mock runtime), not a browser-side QA mock.
    #[test]
    fn packaged_asset_scope_allows_project_media_and_denies_a_sibling_file() {
        let root = full_project();
        let outside = scratch_sibling("outside.mp4");

        let snapshot = video_project::project_snapshot(&root).unwrap();
        let app = tauri::test::mock_app();
        let handle = app.handle();
        grant_project_assets(handle, &snapshot).unwrap();
        let scope = handle.asset_protocol_scope();

        let allowed_rough_cut = root.join("render/rough-cuts/natural.mp4");
        let allowed_final = root.join("render/finals/youtube.mp4");
        assert!(
            scope.is_allowed(&allowed_rough_cut),
            "expected the rough cut to be granted"
        );
        assert!(
            scope.is_allowed(&allowed_final),
            "expected the final to be granted"
        );
        assert!(
            !scope.is_allowed(&outside),
            "a file outside the project must never be granted"
        );

        fs::remove_file(&outside).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn source_grants_require_a_regular_file_and_a_supported_media_probe() {
        let root = full_project();
        let bogus = root.join("not-media.txt");
        fs::write(&bogus, b"plain text, not a video").unwrap();
        let hash = blake3_of(&bogus).unwrap();
        fs::create_dir_all(root.join("sources")).unwrap();
        fs::write(
            root.join("sources/manifest.json"),
            serde_json::json!({
                "schema_version": 1,
                "sources": [{ "source_id": "source-a", "path": bogus.to_string_lossy(), "blake3": hash }]
            })
            .to_string(),
        )
        .unwrap();

        let snapshot = video_project::project_snapshot(&root).unwrap();
        let app = tauri::test::mock_app();
        let handle = app.handle();
        let grants = grant_project_assets(handle, &snapshot).unwrap();

        let grant = grants.iter().find(|g| g.source_id == "source-a").unwrap();
        assert!(
            !grant.granted,
            "a non-media file must not be granted playback scope"
        );
        assert!(!handle.asset_protocol_scope().is_allowed(&bogus));
        fs::remove_dir_all(root).unwrap();
    }
}
