//! Cross-module Studio backend tests: decision-ledger application/replay,
//! variant selection, source relinking, and asset-scope grants, exercised
//! together against a shared project fixture. Moved out of `main.rs` per
//! REV2 §14.5 — pure move, no behavior change.

use crate::commands::{read_decisions, read_variant_selection, select_variant};
use crate::decision_contract::{
    apply_intent, build_record, replay, DecisionIntent, DecisionVerdict, RecordStatus,
    ReviewReason, ReviewTarget, SCHEMA_VERSION,
};
use crate::project_scope::{blake3_of, canonical_project_root, grant_project_assets};
use crate::source_integrity::relink_source;
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::Manager;

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
    let error = select_variant(root.to_string_lossy().into_owned(), "tight".into()).unwrap_err();
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
    let record: serde_json::Value = serde_json::from_str(history.lines().next().unwrap()).unwrap();
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
    let record: serde_json::Value = serde_json::from_str(history.lines().next().unwrap()).unwrap();
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
