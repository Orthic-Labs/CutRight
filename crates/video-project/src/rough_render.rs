use crate::io::*;
use crate::receipts;
use crate::PipelineArtifact;
use crate::ProjectError;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use video_core::{CutPlan, SourceManifest};
use video_media::{
    render_segments, render_source_segments, resolve_toolchain, RenderSegment, SourceRenderSegment,
};

pub fn render_edit(
    project_path: &Path,
    variant: &str,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let plan_path = project_path.join(format!("edit/cut-plan-{variant}.json"));
    let plan: CutPlan = read_json(&plan_path)?;
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    let output = project_path.join(format!("render/rough-cuts/{variant}.mp4"));
    if !dry_run {
        if sources.sources.len() == 1 {
            let source = sources.sources.first().expect("sources are nonempty");
            render_segments(
                Path::new(&source.path),
                &plan
                    .segments
                    .iter()
                    .map(|segment| RenderSegment {
                        start_ms: segment.source_start_ms,
                        end_ms: segment.source_end_ms,
                    })
                    .collect::<Vec<_>>(),
                &output,
            )?;
            write_rough_cut_receipt(&plan_path, &[Path::new(&source.path)], &output, &plan)?;
        } else {
            let inputs = sources
                .sources
                .iter()
                .map(|source| PathBuf::from(&source.path))
                .collect::<Vec<_>>();
            let segments = plan
                .segments
                .iter()
                .map(|segment| {
                    let input_index = sources
                        .sources
                        .iter()
                        .position(|source| source.source_id == segment.source_id)
                        .ok_or_else(|| {
                            ProjectError::InvalidState(format!(
                                "cut segment {} references a missing source",
                                segment.id
                            ))
                        })?;
                    Ok(SourceRenderSegment {
                        input_index,
                        start_ms: segment.source_start_ms,
                        end_ms: segment.source_end_ms,
                    })
                })
                .collect::<Result<Vec<_>, ProjectError>>()?;
            render_source_segments(&inputs, &segments, &output)?;
            let input_paths: Vec<&Path> = inputs.iter().map(PathBuf::as_path).collect();
            write_rough_cut_receipt(&plan_path, &input_paths, &output, &plan)?;
        }
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path: output,
        count: plan.segments.len(),
    })
}

/// Writes the `render.rough_cut` stage receipt (hardening plan §10.4) beside
/// the rendered rough cut, binding the exact cut plan and source file(s) that
/// produced it plus the resolved ffmpeg toolchain identity.
fn write_rough_cut_receipt(
    plan_path: &Path,
    source_paths: &[&Path],
    output: &Path,
    plan: &CutPlan,
) -> Result<(), ProjectError> {
    let mut inputs: Vec<&Path> = vec![plan_path];
    inputs.extend_from_slice(source_paths);
    let mut toolchains = BTreeMap::new();
    if let Ok(toolchain) = resolve_toolchain() {
        toolchains.insert("ffmpeg".to_string(), toolchain.identity());
    }
    receipts::write_stage_receipt(
        &receipts::receipt_path_for(output),
        "render.rough_cut",
        &inputs,
        &serde_json::json!({
            "variant": plan.variant,
            "segment_count": plan.segments.len(),
        }),
        toolchains,
        &[output],
    )?;
    Ok(())
}
