use crate::io::*;
use crate::receipts;
use crate::PipelineArtifact;
use crate::ProjectError;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use video_core::{models::SCHEMA_VERSION, SourceManifest, Timeline};
use video_media::extract_frame;

#[derive(Debug, Deserialize)]
struct VisionAnchorResponse {
    found: bool,
    center_x: f64,
    center_y: f64,
    confidence: f64,
}

pub fn reframe_plan(
    project_path: &Path,
    variant: Option<&str>,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let variant = resolve_variant(project_path, variant)?;
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    if sources.sources.is_empty() {
        return Err(ProjectError::NoSources);
    }
    let timeline_path = variant_timeline_path(project_path, &variant);
    let timeline: Timeline = read_json(&timeline_path).map_err(|_| {
        ProjectError::InvalidState(format!(
            "reframe planning requires edit/timeline-{variant}.json; run `videoctl edit render <project> --variant {variant}` first"
        ))
    })?;
    let timeline_segments = &timeline
        .tracks
        .first()
        .ok_or_else(|| ProjectError::InvalidState("timeline has no main track".into()))?
        .segments;
    let path = project_path.join(format!("analysis/reframe/{variant}/reframe-plan.json"));
    if !dry_run {
        let worker = vision_anchor_worker()?;
        let mut anchors = Vec::with_capacity(timeline_segments.len());
        for segment in timeline_segments {
            let source = sources
                .sources
                .iter()
                .find(|source| source.source_id == segment.source_id)
                .ok_or_else(|| {
                    ProjectError::InvalidState(format!(
                        "reframe segment {} references a missing source",
                        segment.id
                    ))
                })?;
            let frame = project_path.join(format!("cache/frames/reframe-{}.jpg", segment.id));
            extract_frame(
                Path::new(&source.path),
                segment.source_start_ms + (segment.source_end_ms - segment.source_start_ms) / 2,
                &frame,
            )?;
            let vision = detect_vision_anchor(&worker, &frame)?;
            anchors.push(serde_json::json!({
                "source_id": segment.source_id,
                "output_start_ms": segment.output_start_ms,
                "output_end_ms": segment.output_end_ms,
                "center_x": vision.center_x,
                "center_y": vision.center_y,
                "strategy": if vision.found { "vision_face" } else { "manual_anchor_required" },
                "confidence": vision.confidence,
                "approved": false
            }));
        }
        let plan = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "kind": "timeline_reframe_plan",
            "variant": variant,
            "target_aspect": "9:16",
            "approved": false,
            "requires_review": true,
            "anchors": anchors
        });
        write_json_atomic(&path, &plan)?;
        // Compatibility alias for consumers not yet variant-aware.
        write_json_atomic(&project_path.join("analysis/reframe-plan.json"), &plan)?;
        let mut toolchains = BTreeMap::new();
        if let Ok(worker_hash) = hash_file(&worker) {
            toolchains.insert("vision_anchor_worker".to_string(), worker_hash);
        }
        receipts::write_stage_receipt(
            &receipts::receipt_path_for(&path),
            "reframe.plan",
            &[timeline_path.as_path()],
            &serde_json::json!({ "variant": variant, "target_aspect": "9:16" }),
            toolchains,
            &[path.as_path()],
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: timeline_segments.len(),
    })
}

/// Materialize the embedded Vision anchor worker by CONTENT HASH (§10.2).
///
/// The old version-named temp file meant an edited worker with no crate
/// version bump kept running the stale binary that happened to be on disk.
/// Keying on the bytes makes a changed worker land at a different path, and
/// on-disk bytes are verified before execution.
fn vision_anchor_worker() -> Result<PathBuf, ProjectError> {
    video_core::content_store::materialize_worker(
        include_bytes!(env!("CUTRIGHT_VISION_ANCHOR")),
        "vision-anchor",
    )
    .map_err(|error| ProjectError::InvalidState(error.to_string()))
}

fn detect_vision_anchor(worker: &Path, frame: &Path) -> Result<VisionAnchorResponse, ProjectError> {
    let request = serde_json::json!({ "image_path": frame });
    let mut child = Command::new(worker)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .as_mut()
        .expect("piped vision stdin")
        .write_all(&serde_json::to_vec(&request)?)?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(ProjectError::InvalidState(format!(
            "Vision reframe anchor failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let anchor: VisionAnchorResponse = serde_json::from_slice(&output.stdout)?;
    if !(0.0..=1.0).contains(&anchor.center_x) || !(0.0..=1.0).contains(&anchor.center_y) {
        return Err(ProjectError::InvalidState(
            "Vision reframe anchor returned invalid normalized coordinates".into(),
        ));
    }
    Ok(anchor)
}
