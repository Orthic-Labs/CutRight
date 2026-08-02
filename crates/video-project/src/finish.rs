use crate::audio_profile::load_or_init_audio_profile;
use crate::io::*;
use crate::receipts;
use crate::render_final;
use crate::PipelineArtifact;
use crate::ProjectError;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use video_core::{
    models::SCHEMA_VERSION, SourceManifest, Timeline, TimelineSegment, VadRegion, VadSignal,
};
use video_media::{
    duck_track_under_speech, measure_loudness_and_clipping, measure_room_tone_step,
    process_dialogue_stem_with_receipt, render_waveform_range,
};

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

/// Optional user-supplied background/music stem a project may place at this
/// path before running `finish.audio`. When absent, ducking is a no-op
/// (there is nothing to duck).
fn music_input_path(project_path: &Path, variant: &str) -> PathBuf {
    project_path.join(format!("finish/{variant}/music-input.wav"))
}

fn audio_finish_dir(project_path: &Path, variant: &str) -> PathBuf {
    project_path.join(format!("audio/finish/{variant}"))
}

const ROOM_TONE_WINDOW_MS: i64 = 250;

/// Dialogue-only audio finishing (REV2 plan §15.2 "Audio"): runs the
/// versioned dialogue chain (high-pass -> gentle compression -> de-ess ->
/// limiter) over the variant's rendered rough cut, caching the processed
/// stem by content hash + profile version; measures integrated loudness,
/// true peak, and clipped samples; GATES the finish on the profile's
/// tolerance (a failing measurement is a hard stage failure, not just a
/// number in a report); ducks an optional music stem under VAD-derived
/// speech regions mapped into output time; and probes every timeline join
/// for an audible room-tone step, attaching waveform evidence to any join
/// that exceeds the profile's tolerance.
pub fn audio_finish(
    project_path: &Path,
    variant: Option<&str>,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let variant = resolve_variant(project_path, variant)?;
    let timeline_path = variant_timeline_path(project_path, &variant);
    require_variant_artifact(project_path, &timeline_path, &variant, "finish.audio")?;
    let timeline: Timeline = read_json(&timeline_path)?;
    let segments = &timeline
        .tracks
        .first()
        .ok_or_else(|| ProjectError::InvalidState("timeline has no main track".into()))?
        .segments;
    let join_count = segments.len().saturating_sub(1);
    let rough_cut = project_path.join(format!("render/rough-cuts/{variant}.mp4"));
    let path = audio_finish_dir(project_path, &variant).join("audio-finish.json");

    if dry_run {
        return Ok(PipelineArtifact {
            status: "dry-run",
            path,
            count: join_count,
        });
    }

    if !rough_cut.is_file() {
        return Err(ProjectError::InvalidState(format!(
            "audio finish requires the rendered rough cut: render/rough-cuts/{variant}.mp4"
        )));
    }

    let profile = load_or_init_audio_profile(project_path)?;
    let profile_path = crate::audio_profile::audio_profile_path(project_path);
    let dir = audio_finish_dir(project_path, &variant);

    // Dialogue-only cached processed stem: keyed on the rough cut's content
    // hash + the profile version, so a re-run with unchanged inputs and an
    // unchanged profile reuses the stem instead of reprocessing (§15.2).
    let rough_cut_hash = hash_file(&rough_cut)?;
    let cache_key = format!("{}-v{}", &rough_cut_hash[..16], profile.profile_version);
    let stem_path = dir.join(format!("dialogue-stem-{cache_key}.wav"));
    let stem_receipt_path = receipts::receipt_path_for(&stem_path);
    let stem_is_cached = stem_path.is_file() && stem_receipt_path.is_file();
    if !stem_is_cached {
        process_dialogue_stem_with_receipt(
            &rough_cut,
            &stem_path,
            &profile.dialogue_chain_params(),
            profile.profile_version,
        )?;
    }

    // Integrated loudness / true peak / clipped-sample measurement, then the
    // gate. Measuring alone is not the requirement — an out-of-tolerance or
    // clipped result is a hard finish failure.
    let measurement = measure_loudness_and_clipping(&stem_path)?;
    let gate = profile.evaluate_loudness_gate(&measurement);

    // Music ducking under speech, driven by VAD regions mapped from source
    // time into output time via the timeline's own segment mapping.
    let music_input = music_input_path(project_path, &variant);
    let ducking = if music_input.is_file() {
        let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
        let regions = output_speech_regions(project_path, segments, &sources);
        let ducked_path = dir.join("music-ducked.wav");
        duck_track_under_speech(
            &music_input,
            &ducked_path,
            &regions,
            profile.duck_reduction_db,
        )?;
        Some(serde_json::json!({
            "input": relative_artifact_path(project_path, &music_input),
            "output": relative_artifact_path(project_path, &ducked_path),
            "speech_region_count": regions.len(),
            "duck_reduction_db": profile.duck_reduction_db,
        }))
    } else {
        None
    };

    // Room-tone continuity: probe every join in the processed stem and
    // attach waveform evidence to any join whose noise-floor step exceeds
    // the profile's tolerance.
    let evidence_dir = dir.join("room-tone-evidence");
    let mut room_tone_problems = Vec::new();
    for (index, pair) in segments.windows(2).enumerate() {
        let (left, right) = (&pair[0], &pair[1]);
        let cut_id = format!("cut-{:03}", index + 1);
        let join_output_ms = left.output_end_ms;
        let step = measure_room_tone_step(&stem_path, join_output_ms, ROOM_TONE_WINDOW_MS)?;
        let removed_gap_ms = if left.source_id == right.source_id {
            Some((right.source_start_ms - left.source_end_ms).max(0))
        } else {
            None
        };
        let exceeds_tolerance = step
            .step_db
            .is_some_and(|step_db| step_db > profile.room_tone_step_tolerance_db);
        if exceeds_tolerance {
            let waveform_path = evidence_dir.join(format!("{cut_id}.png"));
            render_waveform_range(
                &stem_path,
                (join_output_ms - 750).max(0),
                join_output_ms.saturating_add(750),
                &waveform_path,
            )?;
            room_tone_problems.push(serde_json::json!({
                "id": cut_id,
                "join_output_ms": join_output_ms,
                "removed_gap_ms": removed_gap_ms,
                "before_mean_db": step.before_mean_db,
                "after_mean_db": step.after_mean_db,
                "step_db": step.step_db,
                "tolerance_db": profile.room_tone_step_tolerance_db,
                "waveform_evidence": relative_artifact_path(project_path, &waveform_path),
            }));
        }
    }

    let output = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "variant": variant,
        "profile_version": profile.profile_version,
        "profile_path": relative_artifact_path(project_path, &profile_path),
        "dialogue_stem": relative_artifact_path(project_path, &stem_path),
        "stem_cached": stem_is_cached,
        "measurement": {
            "integrated_lufs": measurement.integrated_lufs,
            "true_peak_dbtp": measurement.true_peak_dbtp,
            "clipped_samples": measurement.clipped_samples,
        },
        "gate": {
            "passed": gate.passed,
            "target_integrated_lufs": profile.target_integrated_lufs,
            "loudness_tolerance_lu": profile.loudness_tolerance_lu,
            "target_true_peak_dbtp": profile.target_true_peak_dbtp,
            "true_peak_tolerance_db": profile.true_peak_tolerance_db,
            "failures": gate.failures,
        },
        "ducking": ducking,
        "room_tone_problems": room_tone_problems,
    });
    write_json_atomic(&path, &output)?;
    receipts::write_stage_receipt(
        &receipts::receipt_path_for(&path),
        "finish.audio",
        &[
            timeline_path.as_path(),
            rough_cut.as_path(),
            profile_path.as_path(),
        ],
        &serde_json::json!({
            "variant": variant,
            "profile_version": profile.profile_version,
            "stem_cached": stem_is_cached,
        }),
        BTreeMap::new(),
        &[path.as_path()],
    )?;

    if !gate.passed {
        return Err(ProjectError::InvalidState(format!(
            "finish.audio loudness gate failed for variant {variant}: {}",
            gate.failures.join("; ")
        )));
    }

    Ok(PipelineArtifact {
        status: "created",
        path,
        count: join_count,
    })
}

/// Map each timeline segment's VAD speech regions from source time into
/// output time (linear per-segment mapping via `output_start_ms +
/// (source_ms - source_start_ms) / speed`), clipped to the segment's own
/// source range, so ducking a music/background track (which lives in
/// output/timeline time) lines up with where dialogue actually plays in the
/// finished edit. VAD for a referenced source that has not been analyzed
/// yet is skipped rather than failing the whole stage — ducking degrades
/// gracefully, it does not block the finish.
fn output_speech_regions(
    project_path: &Path,
    segments: &[TimelineSegment],
    sources: &SourceManifest,
) -> Vec<(i64, i64)> {
    let mut vad_by_source: BTreeMap<String, VadSignal> = BTreeMap::new();
    for source in &sources.sources {
        let vad_path = project_path.join(format!("analysis/vad-{}.json", source.source_id));
        if let Some(signal) = read_json_if_file::<VadSignal>(&vad_path) {
            vad_by_source.insert(source.source_id.clone(), signal);
        }
    }
    let mut regions = Vec::new();
    for segment in segments {
        let Some(signal) = vad_by_source.get(&segment.source_id) else {
            continue;
        };
        let speed = if segment.speed > 0.0 {
            segment.speed
        } else {
            1.0
        };
        for region in &signal.regions {
            if let Some((start_ms, end_ms)) = map_region_to_output(segment, region, speed) {
                regions.push((start_ms, end_ms));
            }
        }
    }
    regions.sort_by_key(|(start_ms, _)| *start_ms);
    regions
}

fn map_region_to_output(
    segment: &TimelineSegment,
    region: &VadRegion,
    speed: f64,
) -> Option<(i64, i64)> {
    let clipped_start = region.start_ms.max(segment.source_start_ms);
    let clipped_end = region.end_ms.min(segment.source_end_ms);
    if clipped_end <= clipped_start {
        return None;
    }
    let to_output = |source_ms: i64| -> i64 {
        segment.output_start_ms
            + ((source_ms - segment.source_start_ms) as f64 / speed).round() as i64
    };
    Some((to_output(clipped_start), to_output(clipped_end)))
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
    use crate::AudioProfile;
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

    // --- finish.audio (REV2 plan §15.2) ---

    fn vad_region(start_ms: i64, end_ms: i64) -> VadRegion {
        VadRegion {
            start_ms,
            end_ms,
            mean_probability: 0.9,
        }
    }

    #[test]
    fn map_region_to_output_clips_to_the_segment_source_range_and_offsets_into_output_time() {
        let segment = TimelineSegment {
            id: "segment-001".into(),
            source_id: "source-a".into(),
            source_start_ms: 1_000,
            source_end_ms: 3_000,
            output_start_ms: 5_000,
            output_end_ms: 7_000,
            speed: 1.0,
            reason: "fixture".into(),
        };
        // Fully inside the segment: a flat +4000ms output offset.
        let inside = map_region_to_output(&segment, &vad_region(1_200, 1_800), 1.0).unwrap();
        assert_eq!(inside, (5_200, 5_800));

        // Overlaps the segment's leading/trailing edge: clipped to the
        // segment's own source range before mapping.
        let overlapping = map_region_to_output(&segment, &vad_region(500, 1_500), 1.0).unwrap();
        assert_eq!(overlapping, (5_000, 5_500));

        // Entirely outside the segment's source range: no mapped region.
        assert!(map_region_to_output(&segment, &vad_region(3_500, 4_000), 1.0).is_none());
    }

    #[test]
    fn output_speech_regions_skips_sources_with_no_vad_on_disk() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        let segments = vec![TimelineSegment {
            id: "segment-001".into(),
            source_id: "source-no-vad".into(),
            source_start_ms: 0,
            source_end_ms: 1_000,
            output_start_ms: 0,
            output_end_ms: 1_000,
            speed: 1.0,
            reason: "fixture".into(),
        }];
        let sources = SourceManifest {
            schema_version: SCHEMA_VERSION,
            sources: vec![video_core::models::SourceEntry {
                source_id: "source-no-vad".into(),
                path: "sources/source-no-vad.mov".into(),
                blake3: "fixture".into(),
                duration_ms: Some(1_000),
                width: Some(1920),
                height: Some(1080),
                rotation_degrees: Some(0),
                is_hdr: Some(false),
                timebase: None,
            }],
        };
        // No VAD file written for source-no-vad: ducking degrades gracefully
        // rather than failing the stage.
        assert!(output_speech_regions(temp.path(), &segments, &sources).is_empty());
    }

    /// Builds a real `render/rough-cuts/<variant>.mp4` fixture: a short color
    /// video muxed with a two-part audio track (a quiet half then a loud
    /// half), so the join between the two timeline segments below lands
    /// exactly on a real, measurable noise-floor step.
    fn write_rough_cut_with_level_step(project_path: &Path, variant: &str) -> PathBuf {
        let dir = project_path.join("render/rough-cuts");
        std::fs::create_dir_all(&dir).unwrap();
        let rough_cut = dir.join(format!("{variant}.mp4"));
        let status = std::process::Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-y"])
            .args(["-f", "lavfi", "-i", "color=c=gray:s=64x64:r=10:d=4"])
            .args([
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=220:sample_rate=48000:duration=2,volume=0.02",
            ])
            .args([
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=220:sample_rate=48000:duration=2,volume=0.9",
            ])
            .args([
                "-filter_complex",
                "[1:a][2:a]concat=n=2:v=0:a=1[a]",
                "-map",
                "0:v",
                "-map",
                "[a]",
                "-shortest",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
            ])
            .arg(&rough_cut)
            .status()
            .expect("spawn ffmpeg to build rough-cut fixture");
        assert!(status.success(), "rough-cut fixture generation failed");
        rough_cut
    }

    /// A two-segment, single-source timeline whose join lands at output
    /// 2000ms — matching the level step baked into
    /// [`write_rough_cut_with_level_step`] — with a real removed gap so the
    /// evidence path exercises the same-source removed-gap branch too.
    fn timeline_with_one_join() -> Timeline {
        Timeline {
            schema_version: SCHEMA_VERSION,
            timebase: Timebase {
                fps_num: 30,
                fps_den: 1,
            },
            tracks: vec![Track {
                id: "main".into(),
                track_type: "video".into(),
                segments: vec![
                    TimelineSegment {
                        id: "segment-001".into(),
                        source_id: "source-a".into(),
                        source_start_ms: 0,
                        source_end_ms: 2_000,
                        output_start_ms: 0,
                        output_end_ms: 2_000,
                        speed: 1.0,
                        reason: "fixture".into(),
                    },
                    TimelineSegment {
                        id: "segment-002".into(),
                        source_id: "source-a".into(),
                        source_start_ms: 2_500,
                        source_end_ms: 4_500,
                        output_start_ms: 2_000,
                        output_end_ms: 4_000,
                        speed: 1.0,
                        reason: "fixture".into(),
                    },
                ],
            }],
        }
    }

    fn write_loose_profile(project_path: &Path) {
        // Wide open tolerance: the loudness gate passes no matter what the
        // fixture's actual measured LUFS/true-peak land on, so these tests
        // isolate room-tone/ducking/receipt behavior from exact loudness
        // tuning of a synthetic tone.
        let profile = AudioProfile {
            loudness_tolerance_lu: 100.0,
            true_peak_tolerance_db: 100.0,
            room_tone_step_tolerance_db: 3.0,
            ..AudioProfile::default()
        };
        write_json_atomic(
            &crate::audio_profile::audio_profile_path(project_path),
            &profile,
        )
        .unwrap();
    }

    fn write_impossible_profile(project_path: &Path) {
        // A target no real measurement can land inside: guarantees the gate
        // fails regardless of the fixture's actual measured loudness.
        let profile = AudioProfile {
            target_integrated_lufs: -14.0,
            loudness_tolerance_lu: 0.0001,
            ..AudioProfile::default()
        };
        write_json_atomic(
            &crate::audio_profile::audio_profile_path(project_path),
            &profile,
        )
        .unwrap();
    }

    #[test]
    fn audio_finish_flags_a_real_room_tone_step_with_waveform_evidence() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        write_loose_profile(temp.path());
        write_json_atomic(
            &temp.path().join("edit/timeline-natural.json"),
            &timeline_with_one_join(),
        )
        .unwrap();
        write_rough_cut_with_level_step(temp.path(), "natural");

        let result = audio_finish(temp.path(), Some("natural"), false).expect("gate passes");
        assert_eq!(result.status, "created");
        let output: serde_json::Value = read_json(&result.path).unwrap();
        assert_eq!(output["profile_version"], 1);
        assert_eq!(output["gate"]["passed"], true);
        let problems = output["room_tone_problems"].as_array().unwrap();
        assert_eq!(
            problems.len(),
            1,
            "expected the one real join to be flagged: {output}"
        );
        assert_eq!(problems[0]["removed_gap_ms"], 500);
        let evidence_path = temp
            .path()
            .join(problems[0]["waveform_evidence"].as_str().unwrap());
        assert!(evidence_path.is_file());
        assert!(receipts::receipt_path_for(&result.path).is_file());
    }

    #[test]
    fn audio_finish_fails_the_gate_out_of_tolerance_but_still_persists_evidence() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        write_impossible_profile(temp.path());
        write_json_atomic(
            &temp.path().join("edit/timeline-natural.json"),
            &timeline_with_one_join(),
        )
        .unwrap();
        write_rough_cut_with_level_step(temp.path(), "natural");

        let error = audio_finish(temp.path(), Some("natural"), false).unwrap_err();
        assert!(matches!(error, ProjectError::InvalidState(_)));
        let message = error.to_string();
        assert!(message.contains("loudness gate failed"), "{message}");

        // The artifact and its receipt are still written on a gate failure
        // — a failed gate is evidence, not a silent no-op.
        let path = audio_finish_dir(temp.path(), "natural").join("audio-finish.json");
        assert!(path.is_file());
        let output: serde_json::Value = read_json(&path).unwrap();
        assert_eq!(output["gate"]["passed"], false);
        assert!(!output["gate"]["failures"].as_array().unwrap().is_empty());
        assert!(receipts::receipt_path_for(&path).is_file());
    }

    #[test]
    fn audio_finish_ducks_music_only_when_a_music_input_is_present() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        write_loose_profile(temp.path());
        write_json_atomic(
            &temp.path().join("edit/timeline-natural.json"),
            &timeline_with_one_join(),
        )
        .unwrap();
        write_rough_cut_with_level_step(temp.path(), "natural");

        // Without a music input, ducking is a no-op.
        let result = audio_finish(temp.path(), Some("natural"), false).expect("gate passes");
        let output: serde_json::Value = read_json(&result.path).unwrap();
        assert!(output["ducking"].is_null());

        // Register the source + VAD speech regions, and place a music
        // stem: ducking must now run and report the mapped speech regions.
        write_json_atomic(
            &temp.path().join("sources/manifest.json"),
            &SourceManifest {
                schema_version: SCHEMA_VERSION,
                sources: vec![video_core::models::SourceEntry {
                    source_id: "source-a".into(),
                    path: "sources/source-a.mov".into(),
                    blake3: "fixture".into(),
                    duration_ms: Some(4_500),
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
                provider: "fixture".into(),
                regions: vec![vad_region(200, 900)],
            },
        )
        .unwrap();
        let music_input = music_input_path(temp.path(), "natural");
        std::fs::create_dir_all(music_input.parent().unwrap()).unwrap();
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
            ])
            .arg("sine=frequency=440:sample_rate=48000:duration=4,volume=0.3")
            .arg(&music_input)
            .status()
            .unwrap();
        assert!(status.success());

        let result = audio_finish(temp.path(), Some("natural"), false).expect("gate still passes");
        let output: serde_json::Value = read_json(&result.path).unwrap();
        let ducking = &output["ducking"];
        assert!(!ducking.is_null());
        assert_eq!(ducking["speech_region_count"], 1);
        let ducked_output = temp.path().join(ducking["output"].as_str().unwrap());
        assert!(ducked_output.is_file());
    }
}
