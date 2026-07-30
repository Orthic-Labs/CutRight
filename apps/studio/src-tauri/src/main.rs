use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

mod decision_contract;

use decision_contract::{DecisionIntent, DecisionRecord, DecisionReplay};

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

fn grant_project_assets(app: &AppHandle, root: &Path) -> Result<(), String> {
    let scope = app.asset_protocol_scope();
    scope
        .allow_directory(root, true)
        .map_err(|error| format!("asset scope for {}: {error}", root.display()))?;

    for source in read_sources(root)?.sources {
        let source_path = fs::canonicalize(&source.path)
            .map_err(|error| format!("source {}: {error}", source.source_id))?;
        scope
            .allow_file(&source_path)
            .map_err(|error| format!("asset scope for {}: {error}", source_path.display()))?;
    }
    Ok(())
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
    grant_project_assets(&app, &root)?;
    Ok(Some(root.to_string_lossy().into_owned()))
}

#[tauri::command]
fn read_snapshot(app: AppHandle, path: String) -> Result<video_project::ProjectSnapshot, String> {
    let root = canonical_project_root(&path)?;
    grant_project_assets(&app, &root)?;
    video_project::project_snapshot(&root).map_err(|error| error.to_string())
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
    let mut file = fs::File::open(&canonical).map_err(|error| named_error("new_path", error))?;
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut file, &mut hasher).map_err(|error| named_error("new_path", error))?;
    let actual_blake3 = format!("blake3:{}", hasher.finalize().to_hex());
    let matches = actual_blake3 == expected_blake3;
    entry["path"] = serde_json::Value::String(canonical.to_string_lossy().into_owned());
    write_json_atomic(&root, manifest_rel, &manifest)?;
    Ok(SourceCheck {
        source_id,
        path: canonical.to_string_lossy().into_owned(),
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
        root
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
    fn relink_source_reports_a_content_mismatch() {
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
        fs::remove_dir_all(root).unwrap();
    }
}
