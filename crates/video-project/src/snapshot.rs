use crate::io::*;
use crate::ProjectError;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::{Path, PathBuf};
use video_core::{
    models::{SourceEntry, SCHEMA_VERSION},
    CandidateManifest, CutPlan, OutputPreset, ProjectManifest, SourceManifest,
};
use video_media::probe;

/// Read-only, Studio-facing summary of a project. Optional artifacts are omitted when their
/// producing pipeline stage has not run (or the artifact is unreadable).
#[derive(Debug, Serialize)]
pub struct ProjectSnapshot {
    pub schema_version: u32,
    pub project_path: PathBuf,
    pub manifest: ProjectManifest,
    pub generated_at: DateTime<Utc>,
    pub sources: Vec<SourceSnapshot>,
    pub stages: PipelineStages,
    pub variants: Vec<VariantSnapshot>,
    pub finals: Vec<FinalSnapshot>,
    pub qa: Option<serde_json::Value>,
    pub bench: Option<BenchSnapshot>,
    pub reframe_plan: Option<serde_json::Value>,
    pub decisions_path: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct SourceSnapshot {
    #[serde(flatten)]
    pub source: SourceEntry,
    pub file_present: bool,
    pub transcript: Option<PathBuf>,
    pub stages: SourceStages,
    pub waveform_png: Option<PathBuf>,
    pub poster_jpg: Option<PathBuf>,
}

#[derive(Debug, Serialize, Default)]
pub struct SourceStages {
    pub ingested: bool,
    pub transcribed: bool,
    pub analyzed: bool,
    pub in_candidates: bool,
    pub in_cut: bool,
}

#[derive(Debug, Serialize, Default)]
pub struct PipelineStages {
    pub ingested: bool,
    pub transcribed: bool,
    pub analyzed: bool,
    pub candidates: bool,
    pub rough_cut: bool,
    #[serde(rename = "final")]
    pub final_render: bool,
    pub qa: bool,
}

#[derive(Debug, Serialize)]
pub struct VariantSnapshot {
    pub id: String,
    pub mp4: Option<PathBuf>,
    pub mp4_mtime: Option<DateTime<Utc>>,
    pub fps: Option<f64>,
    pub cut_plan: Option<CutPlan>,
    pub output_transcript: Option<PathBuf>,
    pub srt: Option<PathBuf>,
    pub segment_count: Option<usize>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct FinalSnapshot {
    pub preset: String,
    pub aspect: String,
    pub width: u32,
    pub height: u32,
    pub mp4: PathBuf,
    pub mp4_mtime: Option<DateTime<Utc>>,
    pub fps: Option<f64>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct BenchSnapshot {
    pub decision: Option<String>,
    pub report: PathBuf,
}

/// Returns the filesystem-backed state Studio needs without mutating or hashing the project.
pub fn project_snapshot(project_path: &Path) -> Result<ProjectSnapshot, ProjectError> {
    let project_path = project_path.canonicalize()?;
    let manifest = read_project_manifest(&project_path.join("project.json"))?;
    let source_manifest_path = project_path.join("sources/manifest.json");
    let sources_manifest = read_json_if_file::<SourceManifest>(&source_manifest_path);
    let candidates =
        read_json_if_file::<CandidateManifest>(&project_path.join("edit/candidates.json"));
    let variant_ids = ["tight", "natural"];
    let variant_plans = variant_ids
        .iter()
        .map(|id| {
            (
                *id,
                read_json_if_file::<CutPlan>(
                    &project_path.join(format!("edit/cut-plan-{id}.json")),
                ),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();

    let sources: Vec<SourceSnapshot> = sources_manifest
        .as_ref()
        .map(|manifest| {
            manifest
                .sources
                .iter()
                .cloned()
                .map(|mut source| {
                    let source_path = absolute_path(&project_path, Path::new(&source.path));
                    source.path = source_path.to_string_lossy().into_owned();
                    let transcript = existing_path(
                        project_path
                            .join(format!("analysis/transcripts/{}.json", source.source_id)),
                    );
                    let analyzed = project_path
                        .join(format!("analysis/vad-{}.json", source.source_id))
                        .is_file();
                    let in_candidates = candidates.as_ref().is_some_and(|items| {
                        items
                            .candidates
                            .iter()
                            .any(|candidate| candidate.source_id == source.source_id)
                    });
                    let in_cut = variant_plans.values().flatten().any(|plan| {
                        plan.segments
                            .iter()
                            .any(|segment| segment.source_id == source.source_id)
                    });
                    SourceSnapshot {
                        file_present: source_path.is_file(),
                        transcript,
                        stages: SourceStages {
                            ingested: true,
                            transcribed: project_path
                                .join(format!("analysis/transcripts/{}.json", source.source_id))
                                .is_file(),
                            analyzed,
                            in_candidates,
                            in_cut,
                        },
                        waveform_png: existing_path(
                            project_path.join(format!("cache/waveforms/{}.png", source.source_id)),
                        ),
                        poster_jpg: existing_path(
                            project_path.join(format!("cache/frames/{}.jpg", source.source_id)),
                        ),
                        source,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let variants = variant_ids
        .iter()
        .map(|id| snapshot_variant(&project_path, id, variant_plans.get(id).cloned().flatten()))
        .collect::<Vec<_>>();
    let finals = manifest
        .outputs
        .iter()
        .filter_map(|preset| snapshot_final(&project_path, preset))
        .collect::<Vec<_>>();
    let stages = PipelineStages {
        ingested: !sources.is_empty(),
        transcribed: !sources.is_empty() && sources.iter().all(|source| source.stages.transcribed),
        analyzed: !sources.is_empty() && sources.iter().all(|source| source.stages.analyzed),
        candidates: candidates.is_some(),
        rough_cut: variants.iter().any(|variant| variant.mp4.is_some()),
        final_render: !finals.is_empty(),
        qa: project_path.join("qa/report.json").is_file(),
    };
    let bench_path = project_path.join("analysis/bench/transcribe/report.json");

    Ok(ProjectSnapshot {
        schema_version: SCHEMA_VERSION,
        project_path: project_path.clone(),
        manifest,
        generated_at: Utc::now(),
        sources,
        stages,
        variants,
        finals,
        qa: read_value_if_file(&project_path.join("qa/report.json")),
        bench: read_value_if_file(&bench_path).map(|report| BenchSnapshot {
            decision: report
                .get("decision")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            report: bench_path,
        }),
        reframe_plan: read_value_if_file(&project_path.join("analysis/reframe-plan.json")).or_else(
            || read_value_if_file(&project_path.join("analysis/reframe/natural/reframe-plan.json")),
        ),
        decisions_path: project_path.join("feedback/decisions.jsonl"),
    })
}

fn snapshot_variant(project_path: &Path, id: &str, cut_plan: Option<CutPlan>) -> VariantSnapshot {
    let mp4_path = project_path.join(format!("render/rough-cuts/{id}.mp4"));
    let metadata = mp4_path.is_file().then(|| probe(&mp4_path).ok()).flatten();
    VariantSnapshot {
        id: id.to_string(),
        mp4: existing_path(mp4_path.clone()),
        mp4_mtime: file_mtime(&mp4_path),
        fps: metadata
            .as_ref()
            .and_then(|value| value.timebase.as_ref())
            .map(fps),
        output_transcript: existing_path(
            project_path.join(format!("edit/output-transcript-{id}.json")),
        ),
        srt: existing_path(project_path.join(format!("edit/captions-{id}.srt"))),
        segment_count: cut_plan.as_ref().map(|plan| plan.segments.len()),
        duration_ms: metadata.and_then(|value| value.duration_ms),
        cut_plan,
    }
}

fn snapshot_final(project_path: &Path, preset: &OutputPreset) -> Option<FinalSnapshot> {
    let mp4 = project_path.join(format!("render/finals/{}.mp4", preset.id));
    if !mp4.is_file() {
        return None;
    }
    let metadata = probe(&mp4).ok();
    Some(FinalSnapshot {
        preset: preset.id.clone(),
        aspect: preset.aspect.clone(),
        width: preset.width,
        height: preset.height,
        mp4: mp4.clone(),
        mp4_mtime: file_mtime(&mp4),
        fps: metadata
            .as_ref()
            .and_then(|value| value.timebase.as_ref())
            .map(fps),
        duration_ms: metadata.and_then(|value| value.duration_ms),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_project;

    #[test]
    fn snapshot_of_manifest_only_project_has_no_completed_stages() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();

        let snapshot = project_snapshot(temp.path()).unwrap();

        assert!(snapshot.sources.is_empty());
        assert!(!snapshot.stages.ingested);
        assert!(!snapshot.stages.transcribed);
        assert!(!snapshot.stages.analyzed);
        assert!(!snapshot.stages.candidates);
        assert!(!snapshot.stages.rough_cut);
        assert!(!snapshot.stages.final_render);
        assert!(!snapshot.stages.qa);
        assert_eq!(
            snapshot.decisions_path,
            temp.path()
                .canonicalize()
                .unwrap()
                .join("feedback/decisions.jsonl")
        );
    }
}
