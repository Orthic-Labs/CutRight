use super::relative_artifact_path;
use crate::read_variant_selection;
use crate::ProjectError;
use std::path::{Path, PathBuf};

pub(crate) fn validate_variant(variant: &str) -> Result<(), ProjectError> {
    match variant {
        "tight" | "natural" => Ok(()),
        _ => Err(ProjectError::InvalidState(format!(
            "unknown edit variant {variant}; use tight or natural"
        ))),
    }
}

/// Resolve the variant a downstream command should operate on. An explicit
/// variant wins; otherwise the reviewed-base selection is used; otherwise fall
/// back to `natural` for backward compatibility with legacy projects.
pub(crate) fn resolve_variant(
    project_path: &Path,
    variant: Option<&str>,
) -> Result<String, ProjectError> {
    if let Some(variant) = variant {
        validate_variant(variant)?;
        return Ok(variant.to_string());
    }
    if let Some(selection) = read_variant_selection(project_path)? {
        validate_variant(&selection.variant)?;
        return Ok(selection.variant);
    }
    Ok("natural".to_string())
}

/// The variant-scoped path for an edit/render/reframe/finish artifact.
/// Deliberately NEVER falls back to a legacy generic alias (REV2 plan §6.1):
/// a project that mixed a `tight` build with a `natural` build used to leave
/// stale/wrong-variant content sitting at the generic path, and any stage
/// that consumed it via a silent fallback would ship a mixed-variant
/// artifact graph without any error. Callers that need the file to exist
/// call [`require_variant_artifact`] first for a clear, variant-named error
/// instead of letting a missing file surface as an opaque read failure or,
/// worse, resolve to a different variant's data.
pub(crate) fn variant_plan_path(project_path: &Path, variant: &str) -> PathBuf {
    project_path.join(format!("edit/cut-plan-{variant}.json"))
}

pub(crate) fn variant_timeline_path(project_path: &Path, variant: &str) -> PathBuf {
    project_path.join(format!("edit/timeline-{variant}.json"))
}

pub(crate) fn variant_captions_path(project_path: &Path, variant: &str) -> PathBuf {
    project_path.join(format!("edit/captions-{variant}.srt"))
}

pub(crate) fn variant_reframe_path(project_path: &Path, variant: &str) -> PathBuf {
    project_path.join(format!("analysis/reframe/{variant}/reframe-plan.json"))
}

pub(crate) fn variant_finish_path(project_path: &Path, variant: &str) -> PathBuf {
    project_path.join(format!("finish/{variant}/finish-plan.json"))
}

/// Require a variant-scoped artifact to exist on disk before a variant-strict
/// stage reads it. Fails with an error that names the variant, the stage,
/// and the exact expected path — never a silent fallback to a different
/// variant's file (REV2 plan §6.1/§13.2).
pub(crate) fn require_variant_artifact(
    project_path: &Path,
    path: &Path,
    variant: &str,
    stage: &str,
) -> Result<(), ProjectError> {
    if path.is_file() {
        Ok(())
    } else {
        Err(ProjectError::InvalidState(format!(
            "{stage} requires the variant {variant} artifact {}; it does not exist \
             (a different variant's artifact is never substituted for it)",
            relative_artifact_path(project_path, path)
        )))
    }
}
