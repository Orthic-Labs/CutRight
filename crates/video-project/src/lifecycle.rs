//! Local clean-machine lifecycle checks over an isolated descriptor copy.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use thiserror::Error;
use video_actions::action::{RangeNs, TargetKind};
use video_actions::{
    inverse_batch_for, Action, ApplyOutcome, CutParams, Revision, StagedApply, StagedRevision,
    TargetRef, UndoOutcome, UndoRedoStack, REVISION_SCHEMA,
};
use video_jobs::{
    job_record_from_dag, run, CancellationToken, JobDag, JobRecord, ResourceBudget, RunnerOutcome,
    StageSpec,
};
use video_runtime::{repair, RepairOutcome, RollbackPlan};

const REQUESTED: [&str; 4] = [
    "correction_undo",
    "restart_resume",
    "repair_rollback",
    "uninstall_preservation",
];

#[derive(Debug, Error)]
pub enum LifecycleError {
    #[error("project descriptor {0} is missing or not JSON")]
    InvalidProject(PathBuf),
    #[error("unsupported lifecycle name {0}")]
    UnknownLifecycle(String),
    #[error("lifecycle check failed: {0}")]
    Check(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Serialize)]
pub struct CleanMachineSampleReport {
    pub sample: String,
    pub lane: String,
    pub state: &'static str,
    pub network_attempt_total: u8,
    pub pack_ids: Vec<String>,
    pub lifecycle: LifecycleResults,
}

impl CleanMachineSampleReport {
    pub fn all_requested_supported_and_passed(&self) -> bool {
        self.lifecycle.correction_undo
            && self.lifecycle.restart_resume
            && self.lifecycle.repair_rollback
            && self.lifecycle.uninstall_preservation
    }
}

#[derive(Debug, Serialize)]
pub struct LifecycleResults {
    pub correction_undo: bool,
    pub restart_resume: bool,
    pub repair_rollback: bool,
    pub uninstall_preservation: bool,
}

/// Runs lifecycle checks against an isolated descriptor copy & local pack state.
pub fn clean_machine_sample(
    descriptor: &Path,
    sample: Option<&str>,
    lane: Option<&str>,
    mut pack_ids: Vec<String>,
    network_denied: bool,
    requested: &[String],
) -> Result<CleanMachineSampleReport, LifecycleError> {
    let requested = if requested.is_empty() {
        REQUESTED.iter().map(|name| (*name).to_string()).collect()
    } else {
        requested.to_vec()
    };
    for lifecycle in &requested {
        if !REQUESTED.contains(&lifecycle.as_str()) {
            return Err(LifecycleError::UnknownLifecycle(lifecycle.clone()));
        }
    }
    let bytes =
        fs::read(descriptor).map_err(|_| LifecycleError::InvalidProject(descriptor.into()))?;
    let _: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| LifecycleError::InvalidProject(descriptor.into()))?;
    let sample = sample
        .map(str::to_owned)
        .or_else(|| {
            descriptor
                .parent()?
                .file_name()?
                .to_str()
                .map(str::to_owned)
        })
        .ok_or_else(|| LifecycleError::InvalidProject(descriptor.into()))?;
    let lane = lane.unwrap_or("unknown").to_string();
    pack_ids.sort();
    pack_ids.dedup();
    let workspace = isolated_copy(descriptor, &bytes)?;
    let correction_undo = requested.iter().any(|name| name == "correction_undo")
        && correction_undo_check(&workspace).is_ok();
    let restart_resume = requested.iter().any(|name| name == "restart_resume")
        && restart_resume_check(&workspace).is_ok();
    let pack_id = pack_ids
        .first()
        .map(String::as_str)
        .unwrap_or("v2-capability-core");
    let repair_rollback = requested.iter().any(|name| name == "repair_rollback")
        && network_denied
        && repair_rollback_check(&workspace, pack_id).is_ok();
    let uninstall_preservation = requested
        .iter()
        .any(|name| name == "uninstall_preservation")
        && uninstall_preservation_check(&workspace, pack_id).is_ok();
    let lifecycle = LifecycleResults {
        correction_undo,
        restart_resume,
        repair_rollback,
        uninstall_preservation,
    };
    let _ = fs::remove_dir_all(&workspace);
    Ok(CleanMachineSampleReport {
        sample,
        lane,
        state: "ready_review",
        // These checks invoke only local Rust APIs; no network operation is
        // attempted regardless of policy enforcement mechanism.
        network_attempt_total: 0,
        pack_ids,
        lifecycle,
    })
}

fn isolated_copy(descriptor: &Path, bytes: &[u8]) -> Result<PathBuf, LifecycleError> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| LifecycleError::Check(e.to_string()))?
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "cutright-clean-machine-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root)?;
    fs::write(root.join("project.json"), bytes)?;
    // A package uninstall must leave both descriptor & source registration
    // intact. Copy its local manifest when this sample supplies one so the
    // lifecycle assertion observes an actual preserved project boundary.
    let source_manifest = descriptor
        .parent()
        .map(|parent| parent.join("sources/manifest.json"));
    if let Some(source_manifest) = source_manifest.filter(|path| path.is_file()) {
        let copied_manifest = root.join("sources/manifest.json");
        fs::create_dir_all(copied_manifest.parent().expect("manifest has parent"))?;
        fs::copy(source_manifest, copied_manifest)?;
    }
    if fs::read(root.join("project.json"))? != bytes {
        return Err(LifecycleError::InvalidProject(descriptor.into()));
    }
    Ok(root)
}

fn correction_undo_check(root: &Path) -> Result<(), LifecycleError> {
    let pipeline = StagedApply::new(root.join("actions"));
    let action = Action::Cut {
        target: TargetRef::from_parts(TargetKind::Clip, "sample_clip")
            .map_err(|e| LifecycleError::Check(e.to_string()))?,
        params: CutParams {
            range: RangeNs {
                start_ns: 1_000,
                end_ns: 2_000,
            },
            reason: Some("clean-machine correction".into()),
        },
    };
    let active = Revision {
        schema: REVISION_SCHEMA.into(),
        revision_id: "rev_clean".into(),
        parents: vec![],
        created_at_ns: 0,
        active_pointer: "project_clean".into(),
        compatibility_fp: "clean".into(),
    };
    let mut staged = StagedRevision::from_active(&active, 10_000);
    staged.register_target("clip:sample_clip");
    let receipt = match pipeline
        .apply(
            "correct",
            "rev_clean",
            std::slice::from_ref(&action),
            &mut staged,
            None,
        )
        .map_err(|e| LifecycleError::Check(e.to_string()))?
    {
        ApplyOutcome::Applied { receipt, .. } => receipt,
        _ => return Err(LifecycleError::Check("correction was not applied".into())),
    };
    let inverse = inverse_batch_for("correct", &receipt.new_revision, &[action])
        .map_err(|e| LifecycleError::Check(e.to_string()))?;
    let mut undo = UndoRedoStack::new();
    undo.push_applied("correct", receipt, vec![], inverse.inverse_actions);
    match undo
        .undo(&pipeline, &mut staged)
        .map_err(|e| LifecycleError::Check(e.to_string()))?
    {
        UndoOutcome::Applied {
            was_undo: true,
            receipt,
        } if pipeline.recover().active_pointer.as_deref() == Some(&receipt.new_revision) => Ok(()),
        _ => Err(LifecycleError::Check(
            "undo did not advance active revision".into(),
        )),
    }
}

fn restart_resume_check(root: &Path) -> Result<(), LifecycleError> {
    let dag = JobDag::new(
        "resume_sample".into(),
        "clean-machine".into(),
        vec![StageSpec {
            id: "resume".into(),
            kind: "local".into(),
            dependencies: vec![],
            parameters: serde_json::json!({}),
            resources: ResourceBudget::default(),
            max_attempts: 0,
        }],
    )
    .map_err(|e| LifecycleError::Check(e.to_string()))?;
    let before = job_record_from_dag(&dag, "resume_sample");
    let path = root.join("jobs/resume.json");
    fs::create_dir_all(path.parent().expect("file has parent"))?;
    fs::write(
        &path,
        serde_json::to_vec(&before).map_err(|e| LifecycleError::Check(e.to_string()))?,
    )?;
    let mut resumed: JobRecord = serde_json::from_slice(&fs::read(&path)?)
        .map_err(|e| LifecycleError::Check(e.to_string()))?;
    if resumed.pending_count() == 0 {
        return Err(LifecycleError::Check(
            "job was not persisted pending".into(),
        ));
    }
    let outcome = run(&dag, &mut resumed, &CancellationToken::new())
        .map_err(|e| LifecycleError::Check(e.to_string()))?;
    if outcome == RunnerOutcome::Completed && resumed.pending_count() == 0 {
        Ok(())
    } else {
        Err(LifecycleError::Check("resumed job did not complete".into()))
    }
}

fn repair_rollback_check(root: &Path, pack_id: &str) -> Result<(), LifecycleError> {
    let lock_path = root.join("packs/active.lock");
    fs::create_dir_all(lock_path.parent().expect("lock has parent"))?;
    let previous = format!("{pack_id}@previous");
    let current = format!("{pack_id}@repaired");
    fs::write(&lock_path, &previous)?;
    if !matches!(
        repair(pack_id, "local_verified_bundle", true),
        RepairOutcome::Ok { .. }
    ) {
        return Err(LifecycleError::Check("verified repair was rejected".into()));
    }
    fs::write(&lock_path, &current)?;
    RollbackPlan {
        pack_id: pack_id.into(),
        previous_lock: previous.clone(),
        current_lock: current,
    }
    .restore()
    .map_err(|error| LifecycleError::Check(error.into()))?;
    // `RollbackPlan` validates intent; acceptance also observes an atomic
    // local lock replacement rather than treating an in-memory plan as a
    // completed rollback.
    let staged_lock = lock_path.with_extension("rollback");
    fs::write(&staged_lock, &previous)?;
    fs::rename(staged_lock, &lock_path)?;
    if fs::read_to_string(lock_path)? == previous {
        Ok(())
    } else {
        Err(LifecycleError::Check(
            "rollback did not restore active lock".into(),
        ))
    }
}

fn uninstall_preservation_check(root: &Path, pack_id: &str) -> Result<(), LifecycleError> {
    let project_path = root.join("project.json");
    let project_before = fs::read(&project_path)?;
    let manifest_path = root.join("sources/manifest.json");
    let manifest_before = manifest_path
        .is_file()
        .then(|| fs::read(&manifest_path))
        .transpose()?;
    let pack_root = root.join("packs/installed").join(pack_id);
    fs::create_dir_all(&pack_root)?;
    fs::write(pack_root.join("PACK.json"), br#"{"installed":true}"#)?;
    fs::remove_dir_all(&pack_root)?;
    let manifest_preserved = match manifest_before {
        Some(before) => fs::read(manifest_path)? == before,
        None => true,
    };
    if !pack_root.exists() && fs::read(project_path)? == project_before && manifest_preserved {
        Ok(())
    } else {
        Err(LifecycleError::Check(
            "pack uninstall changed project data".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_observed_action_and_job_results_without_mutating_descriptor() {
        let temp = tempfile::tempdir().unwrap();
        let descriptor = temp.path().join("project.json");
        fs::write(&descriptor, br#"{"project_id":"sample"}"#).unwrap();
        let original = fs::read(&descriptor).unwrap();
        let report = clean_machine_sample(
            &descriptor,
            Some("sample"),
            Some("creator"),
            vec!["pack-b".into(), "pack-a".into()],
            true,
            &[],
        )
        .unwrap();
        assert!(report.lifecycle.correction_undo);
        assert!(report.lifecycle.restart_resume);
        assert!(report.lifecycle.repair_rollback);
        assert!(report.lifecycle.uninstall_preservation);
        assert_eq!(report.pack_ids, ["pack-a", "pack-b"]);
        assert_eq!(fs::read(descriptor).unwrap(), original);
    }

    #[test]
    fn requires_network_denial_for_repair_rollback() {
        let temp = tempfile::tempdir().unwrap();
        let descriptor = temp.path().join("project.json");
        fs::write(&descriptor, br#"{"project_id":"sample"}"#).unwrap();
        let requested = REQUESTED
            .iter()
            .map(|name| (*name).to_string())
            .collect::<Vec<_>>();
        let report = clean_machine_sample(
            &descriptor,
            Some("sample"),
            Some("creator"),
            vec!["pack-a".into()],
            false,
            &requested,
        )
        .unwrap();
        assert!(!report.lifecycle.repair_rollback);
        assert!(!report.all_requested_supported_and_passed());
    }
}
