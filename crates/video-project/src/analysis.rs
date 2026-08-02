use crate::io::*;
use crate::receipts;
use crate::PipelineArtifact;
use crate::ProjectError;
use std::collections::BTreeMap;
use std::path::Path;
use video_core::{models::SCHEMA_VERSION, providers::VadRequest, SourceManifest};
use video_media::extract_audio_f32;
use video_providers::HeardRightProvider;

pub fn analyze_local(project_path: &Path, dry_run: bool) -> Result<PipelineArtifact, ProjectError> {
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    if dry_run {
        return Ok(PipelineArtifact {
            status: "dry-run",
            path: project_path.join("analysis"),
            count: sources.sources.len(),
        });
    }
    // HeardRight is the single local-audio boundary: it owns the Silero model
    // and runtime and returns VAD regions; CutRight supplies media and policy.
    let provider = HeardRightProvider::discover()?;
    let mut region_count = 0;
    for source in &sources.sources {
        let audio_path = project_path.join(format!("cache/audio/{}-16k.f32", source.source_id));
        extract_audio_f32(Path::new(&source.path), &audio_path, 16_000)?;
        let (signal, provenance) = provider
            .analyze_file_vad_with_provenance(&VadRequest {
                source_id: source.source_id.clone(),
                audio_path: audio_path.clone(),
                sample_rate: 16_000,
                threshold: 0.5,
            })
            .map_err(|error| ProjectError::InvalidState(error.to_string()))?;
        region_count += signal.regions.len();
        let vad_path = project_path.join(format!("analysis/vad-{}.json", source.source_id));
        write_json_atomic(&vad_path, &signal)?;
        // §10.7: VAD carries the same provenance ASR already does — which
        // model, which runtime, which thresholds, and against exactly which
        // decoded audio. Stored beside the signal rather than inside it so the
        // VadSignal artifact shape stays schema-stable.
        write_json_atomic(
            &project_path.join(format!("analysis/vad-{}.provenance.json", source.source_id)),
            &serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "source_id": source.source_id,
                "source_blake3": source.blake3,
                "provider": signal.provider,
                "decoded_audio_blake3": provenance.decoded_audio_blake3,
                "model_revision": provenance.model_revision,
                "runtime_backend": provenance.runtime_backend,
                "threshold": provenance.threshold,
                "min_speech_ms": provenance.min_speech_ms,
                "min_silence_ms": provenance.min_silence_ms,
                "sample_rate": provenance.sample_rate,
                "decode_policy": provenance.decode_policy,
                "request_blake3": provenance.request_blake3,
                "warnings": provenance.warnings,
            }),
        )?;
        let mut toolchains = BTreeMap::new();
        toolchains.insert(
            "heardright".to_string(),
            format!(
                "{}:{}",
                provenance.model_revision, provenance.runtime_backend
            ),
        );
        receipts::write_stage_receipt(
            &receipts::receipt_path_for(&vad_path),
            "analyze.vad",
            &[audio_path.as_path()],
            &serde_json::json!({
                "source_id": source.source_id,
                "threshold": provenance.threshold,
                "min_speech_ms": provenance.min_speech_ms,
                "min_silence_ms": provenance.min_silence_ms,
                "sample_rate": provenance.sample_rate,
                "decode_policy": provenance.decode_policy,
            }),
            toolchains,
            &[vad_path.as_path()],
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path: project_path.join("analysis"),
        count: region_count,
    })
}
