mod receipts;

pub use receipts::{verify_receipts, ReceiptCheck, ReceiptVerificationReport};

mod analysis;
mod audio_profile;
mod benchmark;
mod candidates;
pub mod caption_profile;
mod color_profile;
mod cut_plan;
pub mod effects;
mod evidence;
mod export;
mod final_render;
mod finish;
mod ingest;
mod io;
mod package;
mod project_init;
mod qa;
mod qa_probes;
mod reframe;
mod reframe_track;
mod remap;
mod rough_render;
mod shorts;
mod shorts_scoring;
mod snapshot;
mod timeline;
mod transcription;

pub use analysis::analyze_local;
pub use audio_profile::{AudioProfile, LoudnessGateResult, AUDIO_PROFILE_SCHEMA_VERSION};
pub use benchmark::bench_transcribe;
pub use candidates::{build_candidates, build_candidates_with_policy, count_fillers};
pub use caption_profile::{
    build_default_caption_document, default_fallback_chain, default_primary_font, default_profile,
    CaptionDocument, CaptionProfile, CAPTION_MODEL_SCHEMA_VERSION,
};
pub use color_profile::{
    export_preset_settings, ColorProfile, ExportPresetSettings, ARCHIVE_EXPORT_PRESET,
    COLOR_PROFILE_SCHEMA_VERSION,
};
pub use cut_plan::build_cut_plan;
pub use effects::{
    render_effect_preview, EffectPreviewFixture, EffectPreviewOutcome, EffectRegistry,
    EffectRegistryEntry, EffectRegistryError, EffectRenderer, MotionProfile, ReducedMotionBehavior,
    SafeZoneRef, EFFECT_REGISTRY_SCHEMA_VERSION,
};
pub use evidence::evidence_build;
pub use export::export_otio;
pub use final_render::{render_final, render_master};
pub use finish::{audio_finish, finish_validate, render_slot};
pub use ingest::{ingest_sources, IngestResult, IngestedSource};
pub use package::package_social;
pub use project_init::{
    init_project, migrate_project, InitResult, MigratedArtifact, MigrationReport, SkippedArtifact,
};
pub use qa::qa_run;
pub use reframe::reframe_plan;
pub use remap::{
    read_variant_selection, remap_transcript_for_variant, remap_transcript_with_variant,
    select_variant, SelectionRecord,
};
pub use rough_render::render_edit;
pub use shorts::propose_shorts;
pub use snapshot::{
    project_snapshot, BenchSnapshot, FinalSnapshot, PipelineStages, ProjectSnapshot,
    SourceSnapshot, SourceStages, VariantSnapshot,
};
pub use timeline::{compile_timeline, validate_edit};
pub use transcription::transcribe_project;

use serde::Serialize;
use std::path::PathBuf;
use thiserror::Error;
use video_media::ProbeError;
use video_media::{AudioError, RenderError};
use video_providers::ProviderError;

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("project path is not a directory: {0}")]
    NotDirectory(PathBuf),
    #[error("project manifest has unsupported schema version {0}")]
    UnsupportedSchema(u32),
    #[error("project manifest is invalid: {0}")]
    InvalidManifest(String),
    #[error("ingest requires at least one source file")]
    NoSources,
    #[error("source file is not inside an immutable registration: {0}")]
    SourceChanged(PathBuf),
    #[error("source path cannot be read: {0}")]
    InvalidSource(PathBuf),
    #[error(transparent)]
    Probe(#[from] ProbeError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error(transparent)]
    Render(#[from] RenderError),
    #[error(transparent)]
    Audio(#[from] AudioError),
    #[error(transparent)]
    AudioFinish(#[from] video_media::AudioFinishError),
    #[error("pipeline state is invalid: {0}")]
    InvalidState(String),
}

/// A generic pipeline-stage result envelope shared by every stage function
/// across the split modules (init, ingest, transcription, analysis,
/// candidates, cut-plan, timeline, render, remap, finish, package, export,
/// qa, evidence, shorts). Kept in the crate root rather than duplicated or
/// owned by a single stage module because every stage module returns it.
#[derive(Debug, Serialize)]
pub struct PipelineArtifact {
    pub status: &'static str,
    pub path: PathBuf,
    pub count: usize,
}

/// Shared test fixtures used by more than one stage module's test suite.
/// Kept in the crate root rather than duplicated per-module.
#[cfg(test)]
pub(crate) mod test_support {
    use video_core::Word;

    pub(crate) fn word(start_ms: i64, end_ms: i64) -> Word {
        Word {
            id: format!("w_{start_ms}"),
            source_word_id: None,
            text: "word".into(),
            start_ms,
            end_ms,
            confidence: 1.0,
            speaker: None,
            kind: "word".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::write_json_atomic;
    use crate::test_support::word;
    use crate::{
        build_candidates_with_policy, build_cut_plan, compile_timeline, finish_validate,
        init_project, remap_transcript_for_variant,
    };
    use std::fs;
    use video_core::models::SourceEntry;
    use video_core::models::SCHEMA_VERSION;
    use video_core::{FillerPolicy, SourceManifest, Transcript, VadSignal};

    /// End-to-end (media-tool-free) run through candidates → cut plan →
    /// timeline → transcript remap → finish validate, asserting that each
    /// stage that actually ran left a verifiable `StageReceipt` beside its
    /// artifact (hardening plan §10.4 requirement 5), that `verify_receipts`
    /// reports every one of them as passing, and that tampering with a
    /// bound output afterward makes verification fail.
    #[test]
    fn stage_pipeline_writes_verifiable_receipts_for_every_stage_that_ran() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        write_json_atomic(
            &temp.path().join("sources/manifest.json"),
            &SourceManifest {
                schema_version: SCHEMA_VERSION,
                sources: vec![SourceEntry {
                    source_id: "source-a".into(),
                    path: "sources/source-a.mov".into(),
                    blake3: "fixture".into(),
                    duration_ms: Some(10_000),
                    width: Some(1920),
                    height: Some(1080),
                    rotation_degrees: Some(0),
                    is_hdr: Some(false),
                    timebase: None,
                }],
            },
        )
        .unwrap();
        write_json_atomic(
            &temp.path().join("analysis/vad-source-a.json"),
            &VadSignal {
                schema_version: SCHEMA_VERSION,
                source_id: "source-a".into(),
                sample_rate: 16_000,
                provider: "heardright-silero".into(),
                regions: vec![video_core::VadRegion {
                    start_ms: 0,
                    end_ms: 900,
                    mean_probability: 0.9,
                }],
            },
        )
        .unwrap();
        write_json_atomic(
            &temp.path().join("analysis/transcripts/source-a.json"),
            &Transcript {
                schema_version: SCHEMA_VERSION,
                provider: "fixture".into(),
                source_id: "source-a".into(),
                language: "en".into(),
                words: vec![word(0, 100), word(150, 500)],
                events: Vec::new(),
            },
        )
        .unwrap();

        build_candidates_with_policy(temp.path(), FillerPolicy::SuggestOnly, false).unwrap();
        assert!(receipts::receipt_path_for(&temp.path().join("edit/candidates.json")).is_file());

        build_cut_plan(temp.path(), "natural", false).unwrap();
        let cut_plan_path = temp.path().join("edit/cut-plan-natural.json");
        assert!(receipts::receipt_path_for(&cut_plan_path).is_file());

        compile_timeline(temp.path(), "natural", false).unwrap();
        let timeline_path = temp.path().join("edit/timeline-natural.json");
        assert!(receipts::receipt_path_for(&timeline_path).is_file());

        remap_transcript_for_variant(temp.path(), "natural", false).unwrap();
        let output_transcript_path = temp.path().join("edit/output-transcript-natural.json");
        assert!(receipts::receipt_path_for(&output_transcript_path).is_file());

        finish_validate(temp.path(), Some("natural"), false).unwrap();
        let finish_plan_path = temp.path().join("finish/natural/finish-plan.json");
        assert!(receipts::receipt_path_for(&finish_plan_path).is_file());

        let report = verify_receipts(temp.path()).unwrap();
        assert_eq!(report.status, "pass");
        assert!(report.checked >= 5);
        assert!(report.results.iter().all(|result| result.status == "pass"));

        // Tamper with a bound output after the fact: the receipt binding it
        // must fail verification.
        fs::write(&cut_plan_path, b"{\"tampered\":true}").unwrap();
        let report = verify_receipts(temp.path()).unwrap();
        assert_eq!(report.status, "fail");
        assert!(report
            .results
            .iter()
            .any(|result| result.status == "fail" && !result.failures.is_empty()));
    }
}
