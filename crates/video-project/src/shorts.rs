//! Shorts proposal orchestration (REV2 plan §15.4 Phase 6). This module is
//! purely I/O: it reads the artifacts this project already has (the
//! editorial candidate manifest, transcripts, VAD signals, the project
//! manifest's output presets), hands them to the deterministic pipeline in
//! [`crate::shorts_scoring`], and writes the result plus a stage receipt.
//! All scoring logic lives in `shorts_scoring.rs` so it can be unit-tested
//! without a project on disk.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::io::*;
use crate::receipts;
use crate::shorts_scoring::{
    build_shorts, load_or_init_shorts_profile, shorts_profile_path, SHORTS_SCHEMA_VERSION,
};
use crate::PipelineArtifact;
use crate::ProjectError;
use video_core::{CandidateManifest, SourceManifest, VadSignal};

/// Proposes up to `count` short-form candidates from this project's
/// editorial candidates, transcripts, and VAD data (REV2 plan §15.4).
/// Never selects one automatically: the output artifact's `status` stays
/// `"proposed"` and `selected_id` stays `null` — a human picks.
pub fn propose_shorts(
    project_path: &Path,
    count: u8,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let candidates: CandidateManifest = read_json(&project_path.join("edit/candidates.json"))?;
    let transcripts = load_transcripts(project_path)?;
    let manifest = read_project_manifest(&project_path.join("project.json"))?;
    let sources = read_json::<SourceManifest>(&project_path.join("sources/manifest.json"))?;

    let mut vad_by_source: BTreeMap<String, VadSignal> = BTreeMap::new();
    let mut vad_input_paths: Vec<PathBuf> = Vec::new();
    for source in &sources.sources {
        let vad_path = project_path.join(format!("analysis/vad-{}.json", source.source_id));
        if let Some(signal) = read_json_if_file::<VadSignal>(&vad_path) {
            vad_by_source.insert(source.source_id.clone(), signal);
            vad_input_paths.push(vad_path);
        }
    }

    let profile_path = shorts_profile_path(project_path);
    let profile = load_or_init_shorts_profile(project_path)?;

    let result = build_shorts(
        &candidates.candidates,
        &transcripts,
        &vad_by_source,
        &manifest.outputs,
        &profile,
        count as usize,
    );

    let path = project_path.join("edit/shorts.json");
    if !dry_run {
        let document = serde_json::json!({
            "schema_version": SHORTS_SCHEMA_VERSION,
            "status": "proposed",
            "strategy": "semantic_segmentation_hook_payoff_proof_value_v1",
            "profile_version": profile.profile_version,
            "profile_path": relative_artifact_path(project_path, &profile_path),
            "platform": profile.platform.id,
            "candidates_considered": result.candidates_considered,
            "variants": result.variants,
            "rejected": result.rejected,
            // Explicit human selection (REV2 plan §15.4 item 9): the
            // pipeline never fills this in. A separate, later action writes
            // the id a human picked; this stage only proposes.
            "selected_id": serde_json::Value::Null,
        });
        write_json_atomic(&path, &document)?;

        let candidates_path = project_path.join("edit/candidates.json");
        let project_json_path = project_path.join("project.json");
        let sources_path = project_path.join("sources/manifest.json");
        let transcript_paths = transcript_file_paths(project_path)?;

        let mut inputs: Vec<&Path> = vec![
            candidates_path.as_path(),
            project_json_path.as_path(),
            sources_path.as_path(),
            profile_path.as_path(),
        ];
        inputs.extend(transcript_paths.iter().map(PathBuf::as_path));
        inputs.extend(vad_input_paths.iter().map(PathBuf::as_path));

        receipts::write_stage_receipt(
            &receipts::receipt_path_for(&path),
            "shorts.propose",
            &inputs,
            &serde_json::json!({
                "count": count,
                "profile_version": profile.profile_version,
                "diversity_similarity_threshold": profile.diversity_similarity_threshold,
                "max_group_gap_ms": profile.max_group_gap_ms,
            }),
            BTreeMap::new(),
            &[path.as_path()],
        )?;
    }

    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "proposed" },
        path,
        count: result.variants.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_project;
    use video_core::models::{Candidate, SourceEntry, SCHEMA_VERSION};
    use video_core::{Transcript, Word};

    fn word(id: &str, start_ms: i64, end_ms: i64, text: &str, confidence: f32) -> Word {
        Word {
            id: id.into(),
            source_word_id: None,
            text: text.into(),
            start_ms,
            end_ms,
            confidence,
            speaker: None,
            kind: "word".into(),
        }
    }

    /// Builds a minimal but real project on disk: `init_project` for the
    /// manifest, one registered source, one transcript, and (optionally)
    /// one accepted editorial candidate long enough to survive duration
    /// fit and clean enough to pass standalone validation.
    fn setup(with_candidates: bool) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        init_project(dir.path(), false).expect("init project");

        let sources = SourceManifest {
            schema_version: SCHEMA_VERSION,
            sources: vec![SourceEntry {
                source_id: "cam-a".into(),
                path: "cam-a.mp4".into(),
                blake3: "fixture".into(),
                duration_ms: Some(20_000),
                width: Some(1920),
                height: Some(1080),
                rotation_degrees: None,
                is_hdr: None,
                timebase: None,
            }],
        };
        write_json_atomic(&dir.path().join("sources/manifest.json"), &sources).unwrap();

        let transcript = Transcript {
            schema_version: SCHEMA_VERSION,
            provider: "fixture".into(),
            source_id: "cam-a".into(),
            language: "en".into(),
            words: vec![
                word("w1", 0, 400, "Here", 0.95),
                word("w2", 400, 700, "is", 0.95),
                word("w3", 700, 1_000, "the", 0.9),
                word("w4", 1_000, 20_000, "mistake", 0.9),
            ],
            events: Vec::new(),
        };
        // `init_project` creates `analysis/transcripts/` as a directory, so
        // `transcript_file_paths` reads from there, not the singular
        // no-multi-source fallback path.
        write_json_atomic(
            &dir.path().join("analysis/transcripts/cam-a.json"),
            &transcript,
        )
        .unwrap();

        if with_candidates {
            let candidates = CandidateManifest {
                schema_version: SCHEMA_VERSION,
                candidates: vec![Candidate {
                    id: "candidate-001".into(),
                    source_id: "cam-a".into(),
                    start_ms: 0,
                    end_ms: 20_000,
                    text: "Here is the mistake everyone makes with their savings account.".into(),
                    beat_label: "hook".into(),
                    take_rank: 1,
                    drop_reason: None,
                    filler_count: 0,
                }],
            };
            write_json_atomic(&dir.path().join("edit/candidates.json"), &candidates).unwrap();
        } else {
            let candidates = CandidateManifest {
                schema_version: SCHEMA_VERSION,
                candidates: Vec::new(),
            };
            write_json_atomic(&dir.path().join("edit/candidates.json"), &candidates).unwrap();
        }

        dir
    }

    #[test]
    fn propose_shorts_writes_artifact_and_receipt() {
        let dir = setup(true);
        let result = propose_shorts(dir.path(), 4, false).unwrap();
        assert_eq!(result.status, "proposed");
        assert!(result.path.is_file());
        assert!(receipts::receipt_path_for(&result.path).is_file());

        let document: serde_json::Value = read_json(&result.path).unwrap();
        assert_eq!(document["status"], "proposed");
        assert_eq!(document["selected_id"], serde_json::Value::Null);
        assert!(document["variants"].as_array().unwrap().len() <= 4);

        // The versioned scoring profile was initialized and persisted
        // beside the project, not silently hard-coded.
        assert!(shorts_profile_path(dir.path()).is_file());
    }

    #[test]
    fn propose_shorts_dry_run_writes_nothing() {
        let dir = setup(true);
        let result = propose_shorts(dir.path(), 4, true).unwrap();
        assert_eq!(result.status, "dry-run");
        assert!(!result.path.is_file());
    }

    #[test]
    fn propose_shorts_with_no_candidates_yields_no_variants() {
        let dir = setup(false);
        let result = propose_shorts(dir.path(), 4, false).unwrap();
        assert_eq!(result.count, 0);
        let document: serde_json::Value = read_json(&result.path).unwrap();
        assert!(document["variants"].as_array().unwrap().is_empty());
    }
}
