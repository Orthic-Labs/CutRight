use crate::io::*;
use crate::receipts;
use crate::PipelineArtifact;
use crate::ProjectError;
use std::collections::BTreeMap;
use std::path::Path;
use video_core::{
    models::SCHEMA_VERSION, CutPlan, SourceManifest, Timeline, TimelineSegment, Track,
};

pub fn validate_edit(
    project_path: &Path,
    variant: Option<&str>,
) -> Result<PipelineArtifact, ProjectError> {
    let variant = resolve_variant(project_path, variant)?;
    let plan_path = variant_plan_path(project_path, &variant);
    require_variant_artifact(project_path, &plan_path, &variant, "edit.validate")?;
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    let plan: CutPlan = read_json(&plan_path)?;
    let mut durations = std::collections::HashMap::new();
    for source in sources.sources {
        durations.insert(source.source_id, source.duration_ms);
    }
    // Validate every source range independently. Ranges must be non-negative,
    // non-empty, and in-range for their source.
    for segment in &plan.segments {
        let duration = durations.get(&segment.source_id).ok_or_else(|| {
            ProjectError::InvalidState(format!("missing source {}", segment.source_id))
        })?;
        if segment.source_start_ms < 0
            || segment.source_end_ms <= segment.source_start_ms
            || duration.is_some_and(|end| segment.source_end_ms > end)
        {
            return Err(ProjectError::InvalidState(format!(
                "invalid segment {} range {}..{}",
                segment.id, segment.source_start_ms, segment.source_end_ms
            )));
        }
    }
    // Detect overlaps per source, sorting by source time solely for that check.
    // Output order is allowed to differ from source order (§6.5): a plan may
    // reorder non-overlapping source intervals freely.
    let mut by_source: std::collections::HashMap<String, Vec<(i64, i64, String)>> =
        std::collections::HashMap::new();
    for segment in &plan.segments {
        by_source
            .entry(segment.source_id.clone())
            .or_default()
            .push((
                segment.source_start_ms,
                segment.source_end_ms,
                segment.id.clone(),
            ));
    }
    for (_, mut intervals) in by_source {
        intervals.sort_by_key(|(start, _, _)| *start);
        let mut last_end: Option<i64> = None;
        for (start, end, id) in intervals {
            if last_end.is_some_and(|prev_end| start < prev_end) {
                return Err(ProjectError::InvalidState(format!(
                    "overlapping segment {id}"
                )));
            }
            last_end = Some(end);
        }
    }
    Ok(PipelineArtifact {
        status: "valid",
        path: plan_path,
        count: plan.segments.len(),
    })
}

pub fn compile_timeline(
    project_path: &Path,
    variant: &str,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    validate_variant(variant)?;
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    let cut_plan_path = project_path.join(format!("edit/cut-plan-{variant}.json"));
    require_variant_artifact(project_path, &cut_plan_path, variant, "edit.timeline")?;
    let plan: CutPlan = read_json(&cut_plan_path)?;
    // Carry the explicit working/output timebase onto the timeline (§6.6) rather
    // than silently inheriting the first source's rate.
    let timebase = working_timebase(project_path, &sources);
    let mut cursor = 0;
    let mut segments = Vec::new();
    for segment in &plan.segments {
        let duration = segment.source_end_ms - segment.source_start_ms;
        segments.push(TimelineSegment {
            id: segment.id.clone(),
            source_id: segment.source_id.clone(),
            source_start_ms: segment.source_start_ms,
            source_end_ms: segment.source_end_ms,
            output_start_ms: cursor,
            output_end_ms: cursor + duration,
            speed: 1.0,
            reason: segment.reason.clone(),
        });
        cursor += duration;
    }
    let timeline = Timeline {
        schema_version: SCHEMA_VERSION,
        timebase,
        tracks: vec![Track {
            id: "main".into(),
            track_type: "av".into(),
            segments,
        }],
    };
    let path = project_path.join(format!("edit/timeline-{variant}.json"));
    if !dry_run {
        // §6.1: variant-scoped path only — no generic `edit/timeline.json`
        // alias. That alias used to be overwritten on every call regardless
        // of variant, so building `tight` then `natural` left the generic
        // file holding `natural`'s timeline even for a downstream stage
        // still working against `tight`.
        write_json_atomic(&path, &timeline)?;
        receipts::write_stage_receipt(
            &receipts::receipt_path_for(&path),
            "edit.timeline",
            &[cut_plan_path.as_path()],
            &serde_json::json!({ "variant": variant, "timebase": timeline.timebase }),
            BTreeMap::new(),
            &[path.as_path()],
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: timeline.tracks[0].segments.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_project;
    use std::fs;
    use video_core::models::SourceEntry;
    use video_core::{CutSegment, Timebase};

    #[test]
    fn compile_timeline_is_variant_scoped_and_writes_no_generic_alias() {
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
                    timebase: Some(Timebase {
                        fps_num: 30,
                        fps_den: 1,
                    }),
                }],
            },
        )
        .unwrap();
        let plan = |variant: &str, count: usize| CutPlan {
            schema_version: SCHEMA_VERSION,
            variant: variant.into(),
            gap_threshold_ms: 0,
            head_margin_ms: 0,
            tail_margin_ms: 0,
            segments: (0..count as i64)
                .map(|i| CutSegment {
                    id: format!("segment-{i:03}"),
                    source_id: "source-a".into(),
                    source_start_ms: i * 1000,
                    source_end_ms: i * 1000 + 500,
                    reason: "fixture".into(),
                })
                .collect(),
        };
        write_json_atomic(
            &temp.path().join("edit/cut-plan-tight.json"),
            &plan("tight", 3),
        )
        .unwrap();
        write_json_atomic(
            &temp.path().join("edit/cut-plan-natural.json"),
            &plan("natural", 2),
        )
        .unwrap();

        let tight = compile_timeline(temp.path(), "tight", false).unwrap();
        assert_eq!(tight.path, temp.path().join("edit/timeline-tight.json"));
        let tight_bytes_before = fs::read(&tight.path).unwrap();
        let tight_timeline: Timeline = read_json(&tight.path).unwrap();
        assert_eq!(tight_timeline.tracks[0].segments.len(), 3);
        // §6.1: no generic `edit/timeline.json` alias is written.
        assert!(!temp.path().join("edit/timeline.json").is_file());

        compile_timeline(temp.path(), "natural", false).unwrap();
        let natural_timeline: Timeline =
            read_json(&temp.path().join("edit/timeline-natural.json")).unwrap();
        assert_eq!(natural_timeline.tracks[0].segments.len(), 2);
        // The canonical tight timeline is byte-identical after compiling
        // natural — building one variant must never mutate another's state.
        let tight_bytes_after = fs::read(&tight.path).unwrap();
        assert_eq!(tight_bytes_before, tight_bytes_after);
        assert!(!temp.path().join("edit/timeline.json").is_file());
    }

    #[test]
    fn validate_edit_accepts_reordered_but_non_overlapping_plan() {
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
        // Output order is reversed relative to source order, but the source
        // intervals do not overlap.
        let reordered = CutPlan {
            schema_version: SCHEMA_VERSION,
            variant: "natural".into(),
            gap_threshold_ms: 0,
            head_margin_ms: 0,
            tail_margin_ms: 0,
            segments: vec![
                CutSegment {
                    id: "segment-001".into(),
                    source_id: "source-a".into(),
                    source_start_ms: 5_000,
                    source_end_ms: 6_000,
                    reason: "fixture".into(),
                },
                CutSegment {
                    id: "segment-002".into(),
                    source_id: "source-a".into(),
                    source_start_ms: 0,
                    source_end_ms: 1_000,
                    reason: "fixture".into(),
                },
            ],
        };
        write_json_atomic(&temp.path().join("edit/cut-plan-natural.json"), &reordered).unwrap();
        let result = validate_edit(temp.path(), None).unwrap();
        assert_eq!(result.status, "valid");
        assert_eq!(result.count, 2);

        let overlapping = CutPlan {
            schema_version: SCHEMA_VERSION,
            variant: "natural".into(),
            gap_threshold_ms: 0,
            head_margin_ms: 0,
            tail_margin_ms: 0,
            segments: vec![
                CutSegment {
                    id: "segment-001".into(),
                    source_id: "source-a".into(),
                    source_start_ms: 0,
                    source_end_ms: 2_000,
                    reason: "fixture".into(),
                },
                CutSegment {
                    id: "segment-002".into(),
                    source_id: "source-a".into(),
                    source_start_ms: 1_000,
                    source_end_ms: 3_000,
                    reason: "fixture".into(),
                },
            ],
        };
        write_json_atomic(
            &temp.path().join("edit/cut-plan-natural.json"),
            &overlapping,
        )
        .unwrap();
        assert!(validate_edit(temp.path(), None).is_err());
    }

    #[test]
    fn compile_timeline_uses_the_explicit_working_timebase() {
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
                    timebase: Some(Timebase {
                        fps_num: 30,
                        fps_den: 1,
                    }),
                }],
            },
        )
        .unwrap();
        // The project declares an explicit 24/1 working timebase that must win
        // over the 30/1 source rate.
        let mut manifest: serde_json::Value = read_json(&temp.path().join("project.json")).unwrap();
        manifest["working_timebase"] = serde_json::json!({ "fps_num": 24, "fps_den": 1 });
        fs::write(
            temp.path().join("project.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        write_json_atomic(
            &temp.path().join("edit/cut-plan-natural.json"),
            &CutPlan {
                schema_version: SCHEMA_VERSION,
                variant: "natural".into(),
                gap_threshold_ms: 0,
                head_margin_ms: 0,
                tail_margin_ms: 0,
                segments: vec![CutSegment {
                    id: "segment-001".into(),
                    source_id: "source-a".into(),
                    source_start_ms: 0,
                    source_end_ms: 1_000,
                    reason: "fixture".into(),
                }],
            },
        )
        .unwrap();

        compile_timeline(temp.path(), "natural", false).unwrap();
        let timeline: Timeline =
            read_json(&temp.path().join("edit/timeline-natural.json")).unwrap();
        assert_eq!(
            timeline.timebase,
            Timebase {
                fps_num: 24,
                fps_den: 1,
            }
        );
    }
}
