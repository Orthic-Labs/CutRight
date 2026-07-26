use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Decision {
    schema_version: u32,
    ts: String,
    project_id: String,
    kind: String,
    verdict: Option<String>,
    reason: String,
    note: Option<String>,
    subject: String,
    variant: Option<String>,
    segment_id: Option<String>,
    word_id: Option<String>,
    source_word_id: Option<String>,
    playhead_ms: i64,
    bench_resolved: bool,
    snapshot_generated_at: String,
    app_version: String,
}

#[derive(Debug, Serialize)]
struct DecisionsResponse {
    decisions: Vec<Decision>,
    skipped: usize,
}

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

fn decisions_path(root: &Path, create_parent: bool) -> Result<PathBuf, String> {
    let feedback = root.join("feedback");
    if create_parent && !feedback.exists() {
        fs::create_dir(&feedback).map_err(|error| format!("{}: {error}", feedback.display()))?;
    }
    if feedback.exists() {
        let canonical_feedback = fs::canonicalize(&feedback)
            .map_err(|error| format!("{}: {error}", feedback.display()))?;
        if !canonical_feedback.starts_with(root) {
            return Err(named_error(
                "path",
                "feedback directory escapes the project",
            ));
        }
    }

    let target = feedback.join("decisions.jsonl");
    if target.exists() {
        let canonical_target =
            fs::canonicalize(&target).map_err(|error| format!("{}: {error}", target.display()))?;
        if !canonical_target.starts_with(root) {
            return Err(named_error("path", "decisions file escapes the project"));
        }
        return Ok(canonical_target);
    }
    Ok(target)
}

fn project_id(root: &Path) -> Result<String, String> {
    let path = root.join("project.json");
    let value: serde_json::Value = serde_json::from_slice(
        &fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?,
    )
    .map_err(|error| format!("{}: {error}", path.display()))?;
    value
        .get("project_id")
        .and_then(serde_json::Value::as_str)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| named_error("project_id", "missing from project.json"))
}

fn is_project_relative(path: &str) -> bool {
    !path.is_empty()
        && !Path::new(path).is_absolute()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn is_word_id(value: &str) -> bool {
    value
        .strip_prefix("ow_")
        .is_some_and(|digits| digits.len() == 6 && digits.bytes().all(|byte| byte.is_ascii_digit()))
}

fn is_source_word_id(value: &str) -> bool {
    let Some((source, word)) = value.rsplit_once(':') else {
        return false;
    };
    !source.is_empty()
        && word.starts_with("w_")
        && word.len() == 8
        && word[2..].bytes().all(|byte| byte.is_ascii_digit())
}

fn valid_reason(kind: &str, reason: &str) -> bool {
    matches!(
        (kind, reason),
        (
            "variant_verdict",
            "pacing" | "word_edges" | "energy" | "length" | "other"
        ) | (
            "final_verdict",
            "looks_right" | "captions" | "loudness" | "framing" | "other"
        ) | (
            "segment_flag",
            "clipped_word" | "too_tight" | "too_loose" | "bad_boundary" | "wrong_take" | "other"
        ) | ("qa_ack", "reviewed")
            | ("session_open", "opened")
    )
}

fn variant_ids(root: &Path) -> Result<Vec<String>, String> {
    let variants_dir = root.join("edit");
    if !variants_dir.exists() {
        return Ok(Vec::new());
    }
    let variants = fs::read_dir(&variants_dir)
        .map_err(|error| format!("{}: {error}", variants_dir.display()))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|file_type| file_type.is_file())
                .and_then(|_| entry.file_name().into_string().ok())
        })
        .filter_map(|file_name| {
            file_name
                .strip_prefix("cut-plan-")
                .and_then(|name| name.strip_suffix(".json"))
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    Ok(variants)
}

fn validate_decision(root: &Path, decision: &Decision) -> Result<(), String> {
    if decision.schema_version != 1 {
        return Err(named_error("schema_version", "must be 1"));
    }
    DateTime::parse_from_rfc3339(&decision.ts).map_err(|error| named_error("ts", error))?;
    DateTime::parse_from_rfc3339(&decision.snapshot_generated_at)
        .map_err(|error| named_error("snapshot_generated_at", error))?;
    if decision.project_id != project_id(root)? {
        return Err(named_error("project_id", "does not match project.json"));
    }
    if !matches!(
        decision.kind.as_str(),
        "variant_verdict" | "final_verdict" | "segment_flag" | "qa_ack" | "session_open"
    ) {
        return Err(named_error("kind", "unsupported value"));
    }
    if !valid_reason(&decision.kind, &decision.reason) {
        return Err(named_error("reason", "not allowed for kind"));
    }
    if decision.playhead_ms < 0 {
        return Err(named_error("playhead_ms", "must be non-negative"));
    }
    if let Some(note) = &decision.note {
        if decision.reason != "other" {
            return Err(named_error("note", "is only allowed when reason is other"));
        }
        if note.chars().count() > 200 {
            return Err(named_error("note", "must be 200 characters or fewer"));
        }
        if note.trim().is_empty() {
            return Err(named_error("note", "must not be empty"));
        }
    } else if decision.reason == "other" {
        return Err(named_error("note", "is required when reason is other"));
    }
    if let Some(word_id) = &decision.word_id {
        if !is_word_id(word_id) {
            return Err(named_error("word_id", "must match ow_000000"));
        }
    }
    if let Some(source_word_id) = &decision.source_word_id {
        if !is_source_word_id(source_word_id) {
            return Err(named_error(
                "source_word_id",
                "must match source-id:w_000000",
            ));
        }
    }

    match decision.kind.as_str() {
        "session_open" => {
            if decision.subject != "project" {
                return Err(named_error("subject", "must be project for session_open"));
            }
            if decision.verdict.is_some() {
                return Err(named_error("verdict", "must be null for session_open"));
            }
            if decision.variant.is_some() || decision.segment_id.is_some() {
                return Err(named_error(
                    "session_open",
                    "cannot be variant or segment scoped",
                ));
            }
        }
        "variant_verdict" | "final_verdict" => {
            if !matches!(decision.verdict.as_deref(), Some("approved" | "rejected")) {
                return Err(named_error("verdict", "must be approved or rejected"));
            }
            if !is_project_relative(&decision.subject) {
                return Err(named_error("subject", "must be project-relative"));
            }
            if decision.kind == "variant_verdict" && decision.variant.is_none() {
                return Err(named_error("variant", "is required for variant_verdict"));
            }
            if let Some(variant) = &decision.variant {
                if decision.subject != format!("render/rough-cuts/{variant}.mp4") {
                    return Err(named_error(
                        "subject",
                        "must match the variant rough-cut path",
                    ));
                }
            }
            if decision.kind == "final_verdict" && decision.variant.is_some() {
                return Err(named_error(
                    "variant",
                    "is only allowed for variant_verdict",
                ));
            }
            if decision.segment_id.is_some() {
                return Err(named_error(
                    "segment_id",
                    "is only allowed for segment_flag",
                ));
            }
        }
        "segment_flag" => {
            if !matches!(decision.verdict.as_deref(), Some("rejected")) {
                return Err(named_error("verdict", "must be rejected for segment_flag"));
            }
            if !is_project_relative(&decision.subject) {
                return Err(named_error("subject", "must be project-relative"));
            }
            if decision.segment_id.as_deref().is_none_or(str::is_empty) {
                return Err(named_error("segment_id", "is required for segment_flag"));
            }
            if decision.variant.is_some() {
                return Err(named_error(
                    "variant",
                    "is only allowed for variant_verdict",
                ));
            }
        }
        "qa_ack" => {
            if decision.verdict.as_deref() != Some("acknowledged") {
                return Err(named_error("verdict", "must be acknowledged for qa_ack"));
            }
            if decision.subject != "qa/report.json" {
                return Err(named_error("subject", "must be qa/report.json for qa_ack"));
            }
            if decision.variant.is_some() || decision.segment_id.is_some() {
                return Err(named_error("qa_ack", "cannot be variant or segment scoped"));
            }
        }
        _ => unreachable!(),
    }

    if let Some(variant) = &decision.variant {
        if !variant_ids(root)?.iter().any(|id| id == variant) {
            return Err(named_error("variant", "does not exist in this project"));
        }
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

#[tauri::command]
fn append_decision(path: String, decision: Decision) -> Result<(), String> {
    let root = canonical_project_root(&path)?;
    validate_decision(&root, &decision)?;
    let decisions = decisions_path(&root, true)?;
    let line = serde_json::to_string(&decision).map_err(|error| error.to_string())?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&decisions)
        .map_err(|error| format!("{}: {error}", decisions.display()))?;
    file.write_all(line.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .and_then(|_| file.sync_data())
        .map_err(|error| format!("{}: {error}", decisions.display()))
}

#[tauri::command]
fn read_decisions(path: String) -> Result<DecisionsResponse, String> {
    let root = canonical_project_root(&path)?;
    let decisions_path = decisions_path(&root, false)?;
    if !decisions_path.exists() {
        return Ok(DecisionsResponse {
            decisions: Vec::new(),
            skipped: 0,
        });
    }
    let file = fs::File::open(&decisions_path)
        .map_err(|error| format!("{}: {error}", decisions_path.display()))?;
    let mut decisions = Vec::new();
    let mut skipped = 0;
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|error| format!("{}: {error}", decisions_path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Decision>(&line) {
            Ok(decision) if validate_decision(&root, &decision).is_ok() => decisions.push(decision),
            _ => skipped += 1,
        }
    }
    Ok(DecisionsResponse { decisions, skipped })
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
            verify_sources
        ])
        .run(tauri::generate_context!())
        .expect("CutRight Studio failed to start");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_TEST: AtomicUsize = AtomicUsize::new(0);

    fn project() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "cutright-studio-test-{}-{}",
            std::process::id(),
            NEXT_TEST.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("feedback")).unwrap();
        fs::write(
            root.join("project.json"),
            r#"{"project_id":"project-test"}"#,
        )
        .unwrap();
        root
    }

    fn decision() -> Decision {
        Decision {
            schema_version: 1,
            ts: "2026-07-26T09:14:22.481Z".into(),
            project_id: "project-test".into(),
            kind: "session_open".into(),
            verdict: None,
            reason: "opened".into(),
            note: None,
            subject: "project".into(),
            variant: None,
            segment_id: None,
            word_id: None,
            source_word_id: None,
            playhead_ms: 0,
            bench_resolved: true,
            snapshot_generated_at: "2026-07-26T09:12:00Z".into(),
            app_version: "0.1.0".into(),
        }
    }

    #[test]
    fn session_open_has_a_fixed_subject_and_null_verdict() {
        let root = project();
        validate_decision(&root, &decision()).unwrap();
        let mut invalid = decision();
        invalid.subject = "render/rough-cuts/natural.mp4".into();
        assert!(validate_decision(&root, &invalid)
            .unwrap_err()
            .starts_with("subject:"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_traversal_for_read_and_write_roots() {
        let root = project();
        let traversal = root.join("../../etc");
        let traversal = traversal.to_string_lossy();
        assert!(canonical_project_root(&traversal).is_err());
        assert!(read_decisions(traversal.into_owned()).is_err());
        assert!(append_decision(
            root.join("../../etc").to_string_lossy().into_owned(),
            decision()
        )
        .is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_decisions_are_reported_not_replayed() {
        let root = project();
        fs::write(root.join("feedback/decisions.jsonl"), "not json\n").unwrap();
        let result = read_decisions(root.to_string_lossy().into_owned()).unwrap();
        assert!(result.decisions.is_empty());
        assert_eq!(result.skipped, 1);
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

        assert!(append_decision(root.to_string_lossy().into_owned(), decision()).is_err());

        fs::remove_file(root.join("feedback")).unwrap();
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
}
