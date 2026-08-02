use crate::io::*;
use crate::receipts;
use crate::PipelineArtifact;
use crate::ProjectError;
use std::collections::BTreeMap;
use std::path::Path;
use video_core::{
    models::{SourceEntry, SCHEMA_VERSION},
    SourceManifest, Timeline, Transcript,
};
use video_media::{extract_frame, render_waveform, render_waveform_range};

/// Builds final decision evidence from the SELECTED VARIANT TIMELINE's
/// actual cut boundaries (§13.1), not from pre-selection candidates. Every
/// join between two adjacent timeline segments is a real cut that shipped;
/// for each one this records the source frames immediately before/at/after
/// the cut, the output frames around the join in the actual rendered
/// artifact, source and output waveform snippets, the nearest transcript
/// word id on each side, the removed-gap duration, and the source/output
/// time mapping — plus the render artifact's own hash, so the evidence is
/// bound to the exact bytes it was extracted from.
pub fn evidence_build(
    project_path: &Path,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let variant = resolve_variant(project_path, None)?;
    let timeline_path = variant_timeline_path(project_path, &variant);
    let timeline: Timeline = read_json(&timeline_path)?;
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    let segments = &timeline
        .tracks
        .first()
        .ok_or_else(|| ProjectError::InvalidState("timeline has no main track".into()))?
        .segments;
    let rough_cut = project_path.join(format!("render/rough-cuts/{variant}.mp4"));
    let path = project_path.join("analysis/evidence/manifest.json");
    let join_count = segments.len().saturating_sub(1);
    if !dry_run {
        if !rough_cut.is_file() {
            return Err(ProjectError::InvalidState(format!(
                "evidence build requires the rendered rough cut: render/rough-cuts/{variant}.mp4"
            )));
        }
        let transcripts = load_transcripts(project_path)?;
        let render_artifact_hash = format!("blake3:{}", hash_file(&rough_cut)?);
        let boundary_dir = project_path.join("analysis/evidence/boundaries");
        let waveform_dir = project_path.join("analysis/evidence/waveforms");
        let mut cuts = Vec::new();
        for (index, pair) in segments.windows(2).enumerate() {
            let (left, right) = (&pair[0], &pair[1]);
            let cut_id = format!("cut-{:03}", index + 1);
            let left_source = find_source(&sources, &left.source_id)?;
            let right_source = find_source(&sources, &right.source_id)?;

            let source_before = boundary_dir.join(format!("{cut_id}-source-before.jpg"));
            let source_at = boundary_dir.join(format!("{cut_id}-source-at.jpg"));
            let source_after = boundary_dir.join(format!("{cut_id}-source-after.jpg"));
            extract_frame(
                Path::new(&left_source.path),
                left.source_end_ms.saturating_sub(200).max(0),
                &source_before,
            )?;
            extract_frame(Path::new(&left_source.path), left.source_end_ms, &source_at)?;
            extract_frame(
                Path::new(&right_source.path),
                right.source_start_ms,
                &source_after,
            )?;

            let output_before = boundary_dir.join(format!("{cut_id}-output-before.jpg"));
            let output_after = boundary_dir.join(format!("{cut_id}-output-after.jpg"));
            extract_frame(
                &rough_cut,
                left.output_end_ms.saturating_sub(200).max(0),
                &output_before,
            )?;
            extract_frame(
                &rough_cut,
                left.output_end_ms.saturating_add(200),
                &output_after,
            )?;

            let source_waveform = waveform_dir.join(format!("{cut_id}-source.png"));
            render_waveform_range(
                Path::new(&left_source.path),
                left.source_end_ms.saturating_sub(750).max(0),
                left.source_end_ms.saturating_add(750),
                &source_waveform,
            )?;
            let output_waveform = waveform_dir.join(format!("{cut_id}-output.png"));
            render_waveform_range(
                &rough_cut,
                left.output_end_ms.saturating_sub(750).max(0),
                left.output_end_ms.saturating_add(750),
                &output_waveform,
            )?;

            let previous_word_id =
                nearest_word_before(&transcripts, &left.source_id, left.source_end_ms);
            let next_word_id =
                nearest_word_after(&transcripts, &right.source_id, right.source_start_ms);
            let removed_gap_ms = if left.source_id == right.source_id {
                Some((right.source_start_ms - left.source_end_ms).max(0))
            } else {
                None
            };

            cuts.push(serde_json::json!({
                "id": cut_id,
                "join_output_ms": left.output_end_ms,
                "removed_gap_ms": removed_gap_ms,
                "previous_word_id": previous_word_id,
                "next_word_id": next_word_id,
                "left": {
                    "segment_id": left.id,
                    "source_id": left.source_id,
                    "source_start_ms": left.source_start_ms,
                    "source_end_ms": left.source_end_ms,
                    "output_start_ms": left.output_start_ms,
                    "output_end_ms": left.output_end_ms,
                },
                "right": {
                    "segment_id": right.id,
                    "source_id": right.source_id,
                    "source_start_ms": right.source_start_ms,
                    "source_end_ms": right.source_end_ms,
                    "output_start_ms": right.output_start_ms,
                    "output_end_ms": right.output_end_ms,
                },
                "source_frames": {
                    "before": relative_artifact_path(project_path, &source_before),
                    "at": relative_artifact_path(project_path, &source_at),
                    "after": relative_artifact_path(project_path, &source_after),
                },
                "output_frames": {
                    "before": relative_artifact_path(project_path, &output_before),
                    "after": relative_artifact_path(project_path, &output_after),
                },
                "source_waveform": relative_artifact_path(project_path, &source_waveform),
                "output_waveform": relative_artifact_path(project_path, &output_waveform),
            }));
        }
        let full_output_waveform = waveform_dir.join(format!("{variant}-output.png"));
        render_waveform(&rough_cut, &full_output_waveform)?;
        write_json_atomic(
            &path,
            &serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "variant": variant,
                "render_artifact_path": relative_artifact_path(project_path, &rough_cut),
                "render_artifact_hash": render_artifact_hash,
                "full_output_waveform": relative_artifact_path(project_path, &full_output_waveform),
                "cuts": cuts,
            }),
        )?;
        receipts::write_stage_receipt(
            &receipts::receipt_path_for(&path),
            "evidence.build",
            &[timeline_path.as_path(), rough_cut.as_path()],
            &serde_json::json!({ "variant": variant, "cut_count": join_count }),
            BTreeMap::new(),
            &[path.as_path()],
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: join_count,
    })
}

fn find_source<'a>(
    sources: &'a SourceManifest,
    source_id: &str,
) -> Result<&'a SourceEntry, ProjectError> {
    sources
        .sources
        .iter()
        .find(|source| source.source_id == source_id)
        .ok_or_else(|| {
            ProjectError::InvalidState(format!("evidence references missing source {source_id}"))
        })
}

/// The nearest transcript word on `source_id` that ends at or before
/// `at_ms` — the last word spoken before a cut point in that source.
fn nearest_word_before(transcripts: &[Transcript], source_id: &str, at_ms: i64) -> Option<String> {
    transcripts
        .iter()
        .filter(|transcript| transcript.source_id == source_id)
        .flat_map(|transcript| transcript.words.iter())
        .filter(|word| word.end_ms <= at_ms)
        .max_by_key(|word| word.end_ms)
        .map(|word| word.id.clone())
}

/// The nearest transcript word on `source_id` that starts at or after
/// `at_ms` — the first word spoken after a cut point in that source.
fn nearest_word_after(transcripts: &[Transcript], source_id: &str, at_ms: i64) -> Option<String> {
    transcripts
        .iter()
        .filter(|transcript| transcript.source_id == source_id)
        .flat_map(|transcript| transcript.words.iter())
        .filter(|word| word.start_ms >= at_ms)
        .min_by_key(|word| word.start_ms)
        .map(|word| word.id.clone())
}
