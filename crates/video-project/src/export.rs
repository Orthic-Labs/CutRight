use crate::io::*;
use crate::PipelineArtifact;
use crate::ProjectError;
use std::path::Path;
use video_core::{SourceManifest, Timeline};

pub fn export_otio(
    project_path: &Path,
    variant: Option<&str>,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let variant = resolve_variant(project_path, variant)?;
    let timeline: Timeline = read_json(&variant_timeline_path(project_path, &variant))?;
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    let rate = timeline.timebase.fps_num as f64 / timeline.timebase.fps_den as f64;
    let children = timeline.tracks[0]
        .segments
        .iter()
        .map(|segment| {
            let source = sources
                .sources
                .iter()
                .find(|source| source.source_id == segment.source_id)
                .ok_or_else(|| ProjectError::InvalidState(format!("missing source {}", segment.source_id)))?;
            let source_duration = segment.source_end_ms - segment.source_start_ms;
            // Frame math uses the timeline's explicit working timebase (§6.6).
            Ok(serde_json::json!({
                "OTIO_SCHEMA": "Clip.2",
                "name": segment.id,
                "media_reference": {
                    "OTIO_SCHEMA": "ExternalReference.1",
                    "target_url": format!("file://{}", percent_encode_file_url_path(&source.path))
                },
                "source_range": {
                    "OTIO_SCHEMA": "TimeRange.1",
                    "start_time": {"OTIO_SCHEMA": "RationalTime.1", "value": ms_to_frames_f64(segment.source_start_ms, &timeline.timebase), "rate": rate},
                    "duration": {"OTIO_SCHEMA": "RationalTime.1", "value": ms_to_frames_f64(source_duration, &timeline.timebase), "rate": rate}
                }
            }))
        })
        .collect::<Result<Vec<_>, ProjectError>>()?;
    let path = project_path.join(format!("deliverables/otio/{variant}.otio"));
    if !dry_run {
        let value = serde_json::json!({
            "OTIO_SCHEMA": "Timeline.1",
            "name": "CutRight",
            "variant": variant,
            "tracks": {
                "OTIO_SCHEMA": "Stack.1",
                "children": [{ "OTIO_SCHEMA": "Track.1", "kind": "Video", "children": children }]
            }
        });
        write_json_atomic(&path, &value)?;
        // Compatibility alias for the legacy generic interchange path.
        write_json_atomic(
            &project_path.join("exports/interchange/timeline.otio.json"),
            &value,
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
    use video_core::models::SourceEntry;
    use video_core::models::SCHEMA_VERSION;
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
    fn otio_export_has_standard_timeline_clip_and_media_reference_schemas() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        write_json_atomic(&temp.path().join("edit/timeline.json"), &sample_timeline()).unwrap();
        write_json_atomic(
            &temp.path().join("sources/manifest.json"),
            &SourceManifest {
                schema_version: SCHEMA_VERSION,
                sources: vec![SourceEntry {
                    source_id: "source-001".into(),
                    path: "/captures/cam one.mov".into(),
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

        let result = export_otio(temp.path(), None, false).unwrap();
        let otio: serde_json::Value = read_json(&result.path).unwrap();
        assert_eq!(otio["OTIO_SCHEMA"], "Timeline.1");
        let clip = &otio["tracks"]["children"][0]["children"][0];
        assert_eq!(clip["OTIO_SCHEMA"], "Clip.2");
        assert_eq!(
            clip["media_reference"]["OTIO_SCHEMA"],
            "ExternalReference.1"
        );
        assert_eq!(clip["source_range"]["OTIO_SCHEMA"], "TimeRange.1");
        assert_eq!(
            clip["source_range"]["start_time"]["OTIO_SCHEMA"],
            "RationalTime.1"
        );
        assert_eq!(
            clip["media_reference"]["target_url"],
            "file:///captures/cam%20one.mov"
        );
    }

    #[test]
    fn otio_export_percent_encodes_reserved_and_unicode_source_paths() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        write_json_atomic(&temp.path().join("edit/timeline.json"), &sample_timeline()).unwrap();
        write_json_atomic(
            &temp.path().join("sources/manifest.json"),
            &SourceManifest {
                schema_version: SCHEMA_VERSION,
                sources: vec![SourceEntry {
                    source_id: "source-001".into(),
                    path: "/captures/take #3 café 日本語 100%.mov".into(),
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

        let result = export_otio(temp.path(), None, false).unwrap();
        let otio: serde_json::Value = read_json(&result.path).unwrap();
        let clip = &otio["tracks"]["children"][0]["children"][0];
        assert_eq!(
            clip["media_reference"]["target_url"],
            "file:///captures/take%20%233%20caf%C3%A9%20%E6%97%A5%E6%9C%AC%E8%AA%9E%20100%25.mov"
        );
    }
}
