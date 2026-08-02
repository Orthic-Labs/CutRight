use crate::io::*;
use crate::receipts;
use crate::PipelineArtifact;
use crate::ProjectError;
use chrono::Utc;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;
use video_core::{models::SCHEMA_VERSION, OutputPreset, Timeline};
use video_media::{
    render_preset_with_captions, render_preset_with_captions_and_reframe, resolve_toolchain,
    ReframeAnchor,
};

#[derive(Debug, Deserialize)]
struct ReframePlan {
    approved: bool,
    anchors: Vec<ReframePlanAnchor>,
}

#[derive(Debug, Deserialize)]
struct ReframePlanAnchor {
    output_start_ms: i64,
    output_end_ms: i64,
    source_id: String,
    center_x: f64,
    center_y: f64,
    approved: bool,
}

pub fn render_final(
    project_path: &Path,
    preset: &str,
    variant: Option<&str>,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let variant = resolve_variant(project_path, variant)?;
    let manifest = read_project_manifest(&project_path.join("project.json"))?;
    let output_preset = manifest
        .outputs
        .iter()
        .find(|candidate| candidate.id == preset)
        .ok_or_else(|| ProjectError::InvalidState(format!("unknown output preset {preset}")))?;
    let input = project_path.join(format!("render/rough-cuts/{variant}.mp4"));
    if !input.is_file() {
        return Err(ProjectError::InvalidState(format!(
            "final rendering requires the selected rough cut: render/rough-cuts/{variant}.mp4"
        )));
    }
    let captions = variant_captions_path(project_path, &variant);
    let output = project_path.join(format!("render/finals/{preset}.mp4"));
    let reframe_anchors = if output_preset.aspect == "9:16" {
        Some(load_approved_reframe_anchors(project_path, &variant)?)
    } else {
        None
    };
    if !dry_run {
        if let Some(anchors) = reframe_anchors.as_deref() {
            render_preset_with_captions_and_reframe(
                &input,
                &captions,
                &output,
                output_preset.width,
                output_preset.height,
                true,
                Some(anchors),
            )?;
        } else {
            render_preset_with_captions(
                &input,
                &captions,
                &output,
                output_preset.width,
                output_preset.height,
                false,
            )?;
        }
        write_final_provenance(
            project_path,
            FinalProvenanceInput {
                preset,
                variant: &variant,
                output_preset,
                input: &input,
                captions: &captions,
                output: &output,
                reframed: reframe_anchors.is_some(),
            },
        )?;
        let mut final_inputs = vec![input.as_path(), captions.as_path()];
        let reframe_path = variant_reframe_path(project_path, &variant);
        if reframe_anchors.is_some() && reframe_path.is_file() {
            final_inputs.push(reframe_path.as_path());
        }
        let mut toolchains = BTreeMap::new();
        if let Ok(toolchain) = resolve_toolchain() {
            toolchains.insert("ffmpeg".to_string(), toolchain.identity());
        }
        receipts::write_stage_receipt(
            &receipts::receipt_path_for(&output),
            "render.final",
            &final_inputs,
            &serde_json::json!({
                "preset": preset,
                "variant": variant,
                "width": output_preset.width,
                "height": output_preset.height,
                "aspect": output_preset.aspect,
                "reframed": reframe_anchors.is_some(),
            }),
            toolchains,
            &[output.as_path()],
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path: output,
        count: 1,
    })
}

/// Grouped arguments for [`write_final_provenance`] — a plain data bag, not
/// a persisted or public shape.
struct FinalProvenanceInput<'a> {
    preset: &'a str,
    variant: &'a str,
    output_preset: &'a OutputPreset,
    input: &'a Path,
    captions: &'a Path,
    output: &'a Path,
    reframed: bool,
}

/// Binds a final render to the exact variant, captions, and timeline it was
/// produced from (§13.2/§13.5): `qa_run` reads this to reject a
/// mixed-variant artifact graph, and `package_social` reads it to resolve
/// each deliverable's own caption artifact instead of assuming a single
/// generic one.
fn write_final_provenance(
    project_path: &Path,
    args: FinalProvenanceInput<'_>,
) -> Result<(), ProjectError> {
    let FinalProvenanceInput {
        preset,
        variant,
        output_preset,
        input,
        captions,
        output,
        reframed,
    } = args;
    let timeline_path = variant_timeline_path(project_path, variant);
    let mut provenance = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "preset": preset,
        "variant": variant,
        "aspect": output_preset.aspect,
        "width": output_preset.width,
        "height": output_preset.height,
        "rough_cut_path": relative_artifact_path(project_path, input),
        "rough_cut_hash": format!("blake3:{}", hash_file(input)?),
        "captions_path": relative_artifact_path(project_path, captions),
        "captions_hash": format!("blake3:{}", hash_file(captions)?),
        "timeline_path": relative_artifact_path(project_path, &timeline_path),
        "timeline_hash": format!("blake3:{}", hash_file(&timeline_path)?),
        "output_path": relative_artifact_path(project_path, output),
        "output_hash": format!("blake3:{}", hash_file(output)?),
        "created_at": Utc::now(),
    });
    if reframed {
        let reframe_path = variant_reframe_path(project_path, variant);
        if reframe_path.is_file() {
            provenance["reframe_plan_path"] =
                serde_json::Value::String(relative_artifact_path(project_path, &reframe_path));
            provenance["reframe_plan_hash"] =
                serde_json::Value::String(format!("blake3:{}", hash_file(&reframe_path)?));
        }
    }
    write_json_atomic(
        &project_path.join(format!("render/finals/{preset}.provenance.json")),
        &provenance,
    )
}

fn load_approved_reframe_anchors(
    project_path: &Path,
    variant: &str,
) -> Result<Vec<ReframeAnchor>, ProjectError> {
    let path = variant_reframe_path(project_path, variant);
    let plan: ReframePlan = read_json(&path).map_err(|_| {
        ProjectError::InvalidState(format!(
            "vertical final rendering requires an approved reframe plan for variant {variant}"
        ))
    })?;
    if !plan.approved
        || plan.anchors.is_empty()
        || plan.anchors.iter().any(|anchor| !anchor.approved)
    {
        return Err(ProjectError::InvalidState(
            "vertical final rendering requires every reframe anchor to be explicitly approved"
                .into(),
        ));
    }
    let timeline: Timeline = read_json(&variant_timeline_path(project_path, variant))?;
    let segments = &timeline
        .tracks
        .first()
        .ok_or_else(|| ProjectError::InvalidState("timeline has no main track".into()))?
        .segments;
    if plan.anchors.len() != segments.len()
        || plan.anchors.iter().zip(segments).any(|(anchor, segment)| {
            anchor.source_id != segment.source_id
                || anchor.output_start_ms != segment.output_start_ms
                || anchor.output_end_ms != segment.output_end_ms
                || !(0.0..=1.0).contains(&anchor.center_x)
                || !(0.0..=1.0).contains(&anchor.center_y)
        })
    {
        return Err(ProjectError::InvalidState(
            "reframe anchors must exactly cover the output timeline with normalized centers".into(),
        ));
    }
    Ok(plan
        .anchors
        .into_iter()
        .map(|anchor| ReframeAnchor {
            output_start_ms: anchor.output_start_ms,
            center_x: anchor.center_x,
            center_y: anchor.center_y,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{init_project, select_variant};
    use std::fs;

    #[test]
    fn render_final_resolves_selected_variant_and_falls_back_to_natural() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        fs::write(
            temp.path().join("render/rough-cuts/natural.mp4"),
            b"natural",
        )
        .unwrap();
        fs::write(temp.path().join("render/rough-cuts/tight.mp4"), b"tight").unwrap();

        // No selection -> resolves natural -> dry-run ok (youtube is 16:9).
        let ok = render_final(temp.path(), "youtube", None, true).unwrap();
        assert_eq!(ok.path, temp.path().join("render/finals/youtube.mp4"));

        // Select tight, then remove tight.mp4: resolution follows the selection
        // (and fails), proving it did not silently fall back to natural.
        select_variant(temp.path(), "tight", "cli").unwrap();
        fs::remove_file(temp.path().join("render/rough-cuts/tight.mp4")).unwrap();
        assert!(render_final(temp.path(), "youtube", None, true).is_err());

        // An explicit variant overrides the selection.
        assert!(render_final(temp.path(), "youtube", Some("natural"), true).is_ok());

        // Removing the selection falls back to natural.
        fs::remove_file(temp.path().join("feedback/variant-selection.json")).unwrap();
        assert!(render_final(temp.path(), "youtube", None, true).is_ok());
    }
}
