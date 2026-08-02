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
    // §6.1/§13.2: every artifact this final binds is resolved for THIS
    // variant and checked explicitly, unconditionally (dry-run included) —
    // the same pattern already applied to the rough cut above. A missing
    // variant artifact is a clear, variant-named error, never a silent
    // fallback to a different variant's captions or timeline.
    let captions = variant_captions_path(project_path, &variant);
    require_variant_artifact(project_path, &captions, &variant, "render.final")?;
    let timeline_path = variant_timeline_path(project_path, &variant);
    require_variant_artifact(project_path, &timeline_path, &variant, "render.final")?;
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
                timeline_path: &timeline_path,
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
    timeline_path: &'a Path,
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
        timeline_path,
        output,
        reframed,
    } = args;
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
        "timeline_path": relative_artifact_path(project_path, timeline_path),
        "timeline_hash": format!("blake3:{}", hash_file(timeline_path)?),
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

    /// Fakes the variant-scoped rough cut, captions, and timeline artifacts
    /// `render_final` requires for `variant` (media-tool-free: dry-run never
    /// shells out to ffmpeg, but the artifact-existence checks now run
    /// unconditionally, so they must be present even for a dry run).
    fn fake_variant_artifacts(project_path: &Path, variant: &str) {
        fs::write(
            project_path.join(format!("render/rough-cuts/{variant}.mp4")),
            variant.as_bytes(),
        )
        .unwrap();
        fs::write(
            project_path.join(format!("edit/captions-{variant}.srt")),
            b"1\n00:00:00,000 --> 00:00:01,000\nfixture\n\n",
        )
        .unwrap();
        write_json_atomic(
            &project_path.join(format!("edit/timeline-{variant}.json")),
            &serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "timebase": { "fps_num": 30, "fps_den": 1 },
                "tracks": [{ "id": "main", "track_type": "av", "segments": [] }]
            }),
        )
        .unwrap();
    }

    #[test]
    fn render_final_resolves_selected_variant_and_falls_back_to_natural() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        fake_variant_artifacts(temp.path(), "natural");
        fake_variant_artifacts(temp.path(), "tight");

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

    /// REV2 plan §6.1 regression — the P0 bug this fix closes: render
    /// `tight` fully (its own rough cut, captions, timeline), never build
    /// `natural`'s captions/timeline, then resolve a final for `natural`
    /// (the default when nothing is explicitly selected). Before the fix,
    /// `variant_captions_path`/`variant_timeline_path` silently fell back to
    /// a shared generic alias that `tight`'s own build had just overwritten
    /// — so this call would succeed and quietly bind `tight`'s captions and
    /// timeline into a final whose provenance claimed `"variant": "natural"`.
    /// It must ERROR instead, naming the missing variant, even though
    /// `natural`'s own rough cut exists (proving the failure is specifically
    /// about captions/timeline resolution, not the earlier rough-cut check).
    #[test]
    fn render_final_errors_instead_of_borrowing_a_different_variants_captions_or_timeline() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        // tight is fully built...
        fake_variant_artifacts(temp.path(), "tight");
        // ...but natural only has a rough cut — no captions, no timeline.
        fs::write(
            temp.path().join("render/rough-cuts/natural.mp4"),
            b"natural",
        )
        .unwrap();

        // No selection recorded -> resolves natural by default.
        let error = render_final(temp.path(), "youtube", None, true).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("natural"),
            "error must name the missing variant: {message}"
        );
        assert!(
            !temp
                .path()
                .join("render/finals/youtube.provenance.json")
                .is_file(),
            "no provenance should be written for a rejected mixed-variant resolution"
        );
    }

    /// Generates a tiny real (ffmpeg-encoded) mp4 so a full, non-dry-run
    /// `render_final` can run without a real source ingest.
    fn generate_fixture_mp4(path: &Path, seconds: &str) {
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=black:s=320x180:r=30",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=1000:sample_rate=48000",
                "-t",
                seconds,
                "-c:v",
                "libx264",
                "-c:a",
                "aac",
            ])
            .arg(path)
            .status()
            .expect("start fixture ffmpeg");
        assert!(status.success(), "fixture ffmpeg encode failed");
    }

    /// REV2 plan §6.1/§13.2 regression, real render: every artifact a final
    /// binds must belong to the SAME variant recorded in its own
    /// provenance. Builds `tight` and `natural` each with their own real
    /// rough cut, captions, and timeline, renders a real final for each,
    /// then asserts each final's provenance points at (and hashes match)
    /// only its own variant's captions/timeline — never the other
    /// variant's, even though both exist on disk simultaneously.
    #[test]
    fn final_provenance_variant_always_matches_the_artifacts_it_binds() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();

        // Each variant's rough cut, captions, and timeline exist
        // simultaneously and have DIFFERENT content, so a provenance that
        // accidentally bound the other variant's captions/timeline hash
        // would be detectable.
        for (variant, seconds, caption_text) in [
            ("tight", "1", "tight caption"),
            ("natural", "2", "natural caption"),
        ] {
            generate_fixture_mp4(
                &temp.path().join(format!("render/rough-cuts/{variant}.mp4")),
                seconds,
            );
            fs::write(
                temp.path().join(format!("edit/captions-{variant}.srt")),
                format!("1\n00:00:00,000 --> 00:00:01,000\n{caption_text}\n\n"),
            )
            .unwrap();
            write_json_atomic(
                &temp.path().join(format!("edit/timeline-{variant}.json")),
                &serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "timebase": { "fps_num": 30, "fps_den": 1 },
                    "tracks": [{ "id": "main", "track_type": "av", "segments": [] }]
                }),
            )
            .unwrap();
        }

        let provenance_path = temp.path().join("render/finals/youtube.provenance.json");

        // Render `tight` first and snapshot its provenance before `natural`
        // overwrites the shared per-preset provenance file.
        render_final(temp.path(), "youtube", Some("tight"), false).unwrap();
        let tight_provenance: serde_json::Value = read_json(&provenance_path).unwrap();

        render_final(temp.path(), "youtube", Some("natural"), false).unwrap();
        let natural_provenance: serde_json::Value = read_json(&provenance_path).unwrap();

        let tight_captions_hash = format!(
            "blake3:{}",
            hash_file(&temp.path().join("edit/captions-tight.srt")).unwrap()
        );
        let natural_captions_hash = format!(
            "blake3:{}",
            hash_file(&temp.path().join("edit/captions-natural.srt")).unwrap()
        );
        assert_ne!(tight_captions_hash, natural_captions_hash);

        assert_eq!(tight_provenance["variant"], "tight");
        assert_eq!(tight_provenance["captions_path"], "edit/captions-tight.srt");
        assert_eq!(tight_provenance["captions_hash"], tight_captions_hash);
        assert_eq!(
            tight_provenance["timeline_path"],
            "edit/timeline-tight.json"
        );

        assert_eq!(natural_provenance["variant"], "natural");
        assert_eq!(
            natural_provenance["captions_path"],
            "edit/captions-natural.srt"
        );
        assert_eq!(natural_provenance["captions_hash"], natural_captions_hash);
        assert_eq!(
            natural_provenance["timeline_path"],
            "edit/timeline-natural.json"
        );

        // Neither provenance ever names, or hashes to, the OTHER variant's
        // captions — the exact binding the P0 bug allowed to drift.
        assert_ne!(tight_provenance["captions_hash"], natural_captions_hash);
        assert_ne!(natural_provenance["captions_hash"], tight_captions_hash);
    }
}
