use crate::io::*;
use crate::receipts;
use crate::render_final;
use crate::PipelineArtifact;
use crate::ProjectError;
use std::collections::BTreeMap;
use std::path::Path;
use video_core::{models::SCHEMA_VERSION, Timeline};

pub fn finish_validate(
    project_path: &Path,
    variant: Option<&str>,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let variant = resolve_variant(project_path, variant)?;
    let timeline_path = variant_timeline_path(project_path, &variant);
    require_variant_artifact(project_path, &timeline_path, &variant, "finish.validate")?;
    let timeline: Timeline = read_json(&timeline_path)?;
    let manifest = read_project_manifest(&project_path.join("project.json"))?;
    if timeline
        .tracks
        .iter()
        .all(|track| track.segments.is_empty())
    {
        return Err(ProjectError::InvalidState(
            "timeline has no segments".into(),
        ));
    }
    let path = project_path.join(format!("finish/{variant}/finish-plan.json"));
    if !dry_run {
        let plan = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "variant": variant,
            "base_timeline": relative_artifact_path(project_path, &timeline_path),
            "slots": manifest.outputs.iter().map(|preset| serde_json::json!({
                "id": format!("final-{}", preset.id),
                "kind": "final_delivery",
                "renderer": "render.final",
                "effect_id": "delivery.render_final.v1",
                "preset": preset.id,
                "width": preset.width,
                "height": preset.height,
                "requires_reframe_approval": preset.aspect == "9:16",
                "output_start_ms": 0,
                "output_end_ms": timeline.tracks[0].segments.last().map(|segment| segment.output_end_ms).unwrap_or(0)
            })).collect::<Vec<_>>()
        });
        // §6.1: variant-scoped path only — no generic `finish/finish-plan.json`
        // alias that a stale cross-variant write could contaminate.
        write_json_atomic(&path, &plan)?;
        receipts::write_stage_receipt(
            &receipts::receipt_path_for(&path),
            "finish.validate",
            &[timeline_path.as_path()],
            &serde_json::json!({ "variant": variant }),
            BTreeMap::new(),
            &[path.as_path()],
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "valid" },
        path,
        count: timeline
            .tracks
            .iter()
            .map(|track| track.segments.len())
            .sum(),
    })
}

pub fn render_slot(
    project_path: &Path,
    slot_id: &str,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let variant = resolve_variant(project_path, None)?;
    let finish_path = variant_finish_path(project_path, &variant);
    require_variant_artifact(project_path, &finish_path, &variant, "finish.render_slot")?;
    let finish: serde_json::Value = read_json(&finish_path)?;
    // Prefer the variant the finish plan was built from; fall back to resolution.
    let plan_variant = finish
        .get("variant")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .unwrap_or(variant);
    let slot = finish
        .get("slots")
        .and_then(serde_json::Value::as_array)
        .and_then(|slots| {
            slots
                .iter()
                .find(|slot| slot.get("id").and_then(serde_json::Value::as_str) == Some(slot_id))
        })
        .ok_or_else(|| ProjectError::InvalidState(format!("unknown finish slot {slot_id}")))?;
    let renderer = slot
        .get("renderer")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            ProjectError::InvalidState(format!("finish slot {slot_id} has no renderer"))
        })?;
    match renderer {
        "render.final" => {
            let preset = slot
                .get("preset")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    ProjectError::InvalidState(format!("finish slot {slot_id} has no preset"))
                })?;
            render_final(project_path, preset, Some(&plan_variant), dry_run)
        }
        other => Err(ProjectError::InvalidState(format!(
            "finish slot {slot_id} has unsupported renderer {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_project;
    use video_core::{Timebase, TimelineSegment, Track};

    fn sample_timeline() -> Timeline {
        Timeline {
            schema_version: SCHEMA_VERSION,
            timebase: Timebase {
                fps_num: 30,
                fps_den: 1,
            },
            tracks: vec![Track {
                id: "main".into(),
                track_type: "video".into(),
                segments: vec![TimelineSegment {
                    id: "segment-001".into(),
                    source_id: "source-001".into(),
                    source_start_ms: 1_000,
                    source_end_ms: 3_000,
                    output_start_ms: 0,
                    output_end_ms: 2_000,
                    speed: 1.0,
                    reason: "fixture".into(),
                }],
            }],
        }
    }

    #[test]
    fn finish_validation_creates_one_delivery_slot_per_preset() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        write_json_atomic(
            &temp.path().join("edit/timeline-natural.json"),
            &sample_timeline(),
        )
        .unwrap();

        let result = finish_validate(temp.path(), None, false).unwrap();
        assert_eq!(result.count, 1);
        let plan: video_core::FinishPlan = read_json(&result.path).unwrap();
        assert_eq!(plan.slots.len(), 3);
        assert_eq!(plan.slots[0].id, "final-youtube");
        assert_eq!(plan.slots[0].renderer, "render.final");
        assert_eq!(plan.slots[0].effect_id, "delivery.render_final.v1");
    }
}
