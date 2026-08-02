use crate::color_profile::ColorProfile;
use crate::io::*;
use crate::receipts;
use crate::PipelineArtifact;
use crate::ProjectError;
use chrono::Utc;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use video_core::models::SCHEMA_VERSION;
use video_media::resolve_toolchain;

/// Packages every configured deliverable preset (§13.3/§13.5). Each
/// deliverable resolves its OWN captions from the variant its final was
/// actually rendered from (via that final's provenance) instead of copying
/// one generic SRT into every package by assumption, and packaging refuses
/// any preset whose QA report has not passed — the taste gate (§8) sits
/// upstream of QA via the `human.final_verdict` check, so a package can
/// only be built from deliverables Adrian has already signed off on. Every
/// package emits one manifest (`exports/package-manifest.json`) binding
/// artifact paths, hashes, sizes, selected variant, preset/profile
/// versions, QA report hash, caption artifact hash, creation time, and
/// toolchain identity — copying files alone is not a release package.
pub fn package_social(
    project_path: &Path,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let manifest = read_project_manifest(&project_path.join("project.json"))?;
    let path = project_path.join("exports/package-manifest.json");
    if dry_run {
        return Ok(PipelineArtifact {
            status: "dry-run",
            path,
            count: manifest.outputs.len(),
        });
    }
    let mut deliverables = Vec::new();
    let mut package_inputs: Vec<PathBuf> = Vec::new();
    for preset in &manifest.outputs {
        let final_video = project_path.join(format!("render/finals/{}.mp4", preset.id));
        let provenance_path =
            project_path.join(format!("render/finals/{}.provenance.json", preset.id));
        if !final_video.is_file() || !provenance_path.is_file() {
            return Err(ProjectError::InvalidState(format!(
                "social packaging requires a rendered final and provenance for preset {}",
                preset.id
            )));
        }
        let provenance: serde_json::Value = read_json(&provenance_path)?;
        let variant = provenance
            .get("variant")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                ProjectError::InvalidState(format!("{} provenance is missing a variant", preset.id))
            })?
            .to_string();
        // §13.3: this preset's OWN captions, resolved from the variant it
        // was actually rendered from — never a generic file assumed shared.
        let captions = variant_captions_path(project_path, &variant);
        if !captions.is_file() {
            return Err(ProjectError::InvalidState(format!(
                "social packaging requires captions for preset {} (variant {variant})",
                preset.id
            )));
        }
        let qa_report_path = project_path.join(format!("qa/{}.report.json", preset.id));
        let qa_report: serde_json::Value = read_json(&qa_report_path).map_err(|_| {
            ProjectError::InvalidState(format!(
                "social packaging requires a QA report for preset {}: run `videoctl qa <project> --preset {}` first",
                preset.id, preset.id
            ))
        })?;
        if qa_report.get("status").and_then(serde_json::Value::as_str) != Some("pass") {
            return Err(ProjectError::InvalidState(format!(
                "social packaging refuses preset {} whose QA report did not pass",
                preset.id
            )));
        }

        let export_dir = project_path.join(format!("exports/{}", preset.id));
        let video_export = export_dir.join(format!("{}.mp4", preset.id));
        let caption_export = export_dir.join(format!("{}.srt", preset.id));
        fs::create_dir_all(&export_dir)?;
        fs::copy(&final_video, &video_export)?;
        fs::copy(&captions, &caption_export)?;

        deliverables.push(serde_json::json!({
            "preset": preset.id,
            "aspect": preset.aspect,
            "selected_variant": variant,
            "video": {
                "path": relative_artifact_path(project_path, &video_export),
                "hash": format!("blake3:{}", hash_file(&video_export)?),
                "size_bytes": fs::metadata(&video_export)?.len(),
            },
            "captions": {
                "path": relative_artifact_path(project_path, &caption_export),
                "hash": format!("blake3:{}", hash_file(&caption_export)?),
                "size_bytes": fs::metadata(&caption_export)?.len(),
            },
            "preset_version": SCHEMA_VERSION,
            "caption_profile_version": SCHEMA_VERSION,
            "qa_report_path": relative_artifact_path(project_path, &qa_report_path),
            "qa_report_hash": format!("blake3:{}", hash_file(&qa_report_path)?),
        }));
        package_inputs.push(final_video);
        package_inputs.push(qa_report_path);
    }

    // §15.2 Export: bind the master/archive artifact (when it exists) and
    // the color profile version that produced it into the SAME manifest —
    // extending it rather than replacing any of the deliverable
    // hashes/QA binding built above.
    let color_profile = ColorProfile::load_or_default(project_path)?;
    let master_video = project_path.join("render/finals/master.mov");
    let master_provenance_path = project_path.join("render/finals/master.provenance.json");
    let master = if master_video.is_file() && master_provenance_path.is_file() {
        package_inputs.push(master_video.clone());
        package_inputs.push(master_provenance_path.clone());
        Some(serde_json::json!({
            "video": {
                "path": relative_artifact_path(project_path, &master_video),
                "hash": format!("blake3:{}", hash_file(&master_video)?),
                "size_bytes": fs::metadata(&master_video)?.len(),
            },
            "provenance_path": relative_artifact_path(project_path, &master_provenance_path),
            "provenance_hash": format!("blake3:{}", hash_file(&master_provenance_path)?),
        }))
    } else {
        None
    };

    let package = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "created_at": Utc::now(),
        "toolchain_identity": {
            "videoctl_version": env!("CARGO_PKG_VERSION"),
            "ffmpeg": std::env::var("CUTRIGHT_FFMPEG").unwrap_or_else(|_| "ffmpeg".into()),
        },
        "color_profile_id": color_profile.id,
        "color_profile_version": color_profile.schema_version,
        "master": master,
        "deliverables": deliverables,
    });
    write_json_atomic(&path, &package)?;
    let input_refs: Vec<&Path> = package_inputs.iter().map(PathBuf::as_path).collect();
    let mut toolchains = BTreeMap::new();
    if let Ok(toolchain) = resolve_toolchain() {
        toolchains.insert("ffmpeg".to_string(), toolchain.identity());
    }
    receipts::write_stage_receipt(
        &receipts::receipt_path_for(&path),
        "package.social",
        &input_refs,
        &serde_json::json!({ "preset_count": manifest.outputs.len() }),
        toolchains,
        &[path.as_path()],
    )?;
    Ok(PipelineArtifact {
        status: "created",
        path,
        count: deliverables.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_project;

    /// Fakes exactly what one deliverable preset needs for `package_social`
    /// to accept it (final video + provenance naming a variant + captions
    /// for that variant + a passing QA report) without shelling out to
    /// ffmpeg — `package_social` itself never calls ffmpeg, only copies and
    /// hashes files.
    fn fake_deliverable(project_path: &Path, preset_id: &str) {
        fs::write(
            project_path.join(format!("render/finals/{preset_id}.mp4")),
            format!("{preset_id}-video").as_bytes(),
        )
        .unwrap();
        write_json_atomic(
            &project_path.join(format!("render/finals/{preset_id}.provenance.json")),
            &serde_json::json!({ "variant": "natural" }),
        )
        .unwrap();
        fs::write(
            project_path.join("edit/captions-natural.srt"),
            b"1\n00:00:00,000 --> 00:00:01,000\nfixture\n\n",
        )
        .unwrap();
        write_json_atomic(
            &project_path.join(format!("qa/{preset_id}.report.json")),
            &serde_json::json!({ "status": "pass" }),
        )
        .unwrap();
    }

    /// REV2 plan §15.2 Export regression: `package_social` extends the
    /// existing manifest with the color profile version and, when a master
    /// artifact exists, its own hashed entry — without breaking any
    /// existing deliverable hash/QA binding.
    #[test]
    fn package_manifest_includes_color_profile_and_optional_master() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        for preset_id in ["youtube", "reels", "tiktok"] {
            fake_deliverable(temp.path(), preset_id);
        }

        // No master rendered yet: the manifest still builds, with
        // "master": null and existing deliverable binding intact.
        let result = package_social(temp.path(), false).unwrap();
        assert_eq!(result.count, 3);
        let package: serde_json::Value = read_json(&result.path).unwrap();
        assert_eq!(package["color_profile_id"], "default-sdr-v1");
        assert_eq!(package["color_profile_version"], SCHEMA_VERSION);
        assert!(package["master"].is_null());
        assert_eq!(package["deliverables"].as_array().unwrap().len(), 3);
        assert_eq!(package["deliverables"][0]["preset"], "youtube");

        // Now fake a rendered master + provenance and repackage: the
        // manifest's master entry appears, hashed, without touching the
        // deliverables that were already correct.
        fs::write(
            temp.path().join("render/finals/master.mov"),
            b"master-video-bytes",
        )
        .unwrap();
        write_json_atomic(
            &temp.path().join("render/finals/master.provenance.json"),
            &serde_json::json!({ "output_metadata_verified": true }),
        )
        .unwrap();

        let result = package_social(temp.path(), false).unwrap();
        let package: serde_json::Value = read_json(&result.path).unwrap();
        assert!(!package["master"].is_null());
        assert_eq!(
            package["master"]["video"]["path"],
            "render/finals/master.mov"
        );
        assert_eq!(package["deliverables"].as_array().unwrap().len(), 3);
    }
}
