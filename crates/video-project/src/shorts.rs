use crate::io::*;
use crate::PipelineArtifact;
use crate::ProjectError;
use std::path::Path;
use video_core::{models::SCHEMA_VERSION, CandidateManifest};

pub fn propose_shorts(
    project_path: &Path,
    count: u8,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let candidates: CandidateManifest = read_json(&project_path.join("edit/candidates.json"))?;
    let mut ranked = candidates
        .candidates
        .iter()
        .filter(|candidate| candidate.drop_reason.is_none())
        .cloned()
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        let left_score = (
            left.end_ms - left.start_ms,
            std::cmp::Reverse(left.take_rank),
        );
        let right_score = (
            right.end_ms - right.start_ms,
            std::cmp::Reverse(right.take_rank),
        );
        right_score.cmp(&left_score)
    });
    let mut source_ids = std::collections::HashSet::new();
    let mut selected = ranked
        .iter()
        .filter(|candidate| source_ids.insert(candidate.source_id.clone()))
        .take(count as usize)
        .cloned()
        .collect::<Vec<_>>();
    if selected.len() < count as usize {
        let selected_ids = selected
            .iter()
            .map(|candidate| candidate.id.clone())
            .collect::<std::collections::HashSet<_>>();
        let remaining = ranked
            .into_iter()
            .filter(|candidate| !selected_ids.contains(&candidate.id))
            .take(count as usize - selected.len())
            .collect::<Vec<_>>();
        selected.extend(remaining);
    }
    let path = project_path.join("edit/shorts.json");
    if !dry_run {
        write_json_atomic(
            &path,
            &serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "status": "proposed",
                "strategy": "duration_then_take_rank_with_source_diversity",
                "variants": selected
            }),
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: selected.len(),
    })
}
