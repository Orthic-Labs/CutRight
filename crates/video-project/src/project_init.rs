use crate::io::*;
use crate::ProjectError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use video_core::{
    models::SCHEMA_VERSION, OutputPreset, ProjectManifest, ReviewMode, SourceManifest, SourcePolicy,
};

const PROJECT_DIRS: &[&str] = &[
    "brief",
    "sources",
    "cache/audio",
    "cache/proxies",
    "cache/frames",
    "cache/waveforms",
    "cache/provider-responses",
    "analysis/cloud-analysis",
    "analysis/bench/transcribe",
    "analysis/transcripts",
    "edit/variants",
    "finish/slots",
    "render/proxies",
    "render/rough-cuts",
    "render/slots",
    "render/previews",
    "render/finals",
    "qa",
    "feedback",
    "exports/youtube",
    "exports/vertical",
    "exports/captions",
    "exports/interchange",
];

#[derive(Debug, Serialize)]
pub struct InitResult {
    pub status: &'static str,
    pub project_path: PathBuf,
    pub created_paths: Vec<PathBuf>,
    pub manifest_path: PathBuf,
}

/// One artifact relocated by [`migrate_project`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MigratedArtifact {
    pub from: String,
    pub to: String,
    pub backup: String,
}

/// One legacy artifact left in place by [`migrate_project`], with a reason.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkippedArtifact {
    pub path: String,
    pub reason: String,
}

/// Result of a legacy-to-variant layout migration (§6.7).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MigrationReport {
    pub schema_version: u32,
    pub status: String,
    pub migrated_at: DateTime<Utc>,
    pub migrated: Vec<MigratedArtifact>,
    pub skipped: Vec<SkippedArtifact>,
    pub backup_dir: Option<PathBuf>,
}

pub fn init_project(path: &Path, dry_run: bool) -> Result<InitResult, ProjectError> {
    if path.exists() && !path.is_dir() {
        return Err(ProjectError::NotDirectory(path.to_path_buf()));
    }

    let project_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("video-project")
        .to_string();
    let manifest_path = path.join("project.json");
    let mut created_paths = Vec::new();

    if dry_run {
        return Ok(InitResult {
            status: "dry-run",
            project_path: path.to_path_buf(),
            created_paths: PROJECT_DIRS.iter().map(|dir| path.join(dir)).collect(),
            manifest_path,
        });
    }

    fs::create_dir_all(path)?;
    for dir in PROJECT_DIRS {
        let directory = path.join(dir);
        if !directory.exists() {
            fs::create_dir_all(&directory)?;
            created_paths.push(directory);
        }
    }

    if manifest_path.exists() {
        read_project_manifest(&manifest_path)?;
    } else {
        // §12.7: identity is random, not folder-derived — two projects named
        // "reel" used to share a project_id and therefore a decision ledger
        // identity. The folder name is kept as the human title instead.
        let instance_id = fresh_instance_id();
        let manifest = ProjectManifest {
            schema_version: SCHEMA_VERSION,
            project_id: instance_id.clone(),
            project_instance_id: instance_id,
            title: project_name.clone(),
            kind: "mixed_creator_content".into(),
            created_at: Utc::now(),
            review_mode: ReviewMode::Reviewed,
            source_policy: SourcePolicy::Immutable,
            outputs: vec![
                OutputPreset {
                    id: "youtube".into(),
                    aspect: "16:9".into(),
                    width: 1920,
                    height: 1080,
                },
                OutputPreset {
                    id: "reels".into(),
                    aspect: "9:16".into(),
                    width: 1080,
                    height: 1920,
                },
                OutputPreset {
                    id: "tiktok".into(),
                    aspect: "9:16".into(),
                    width: 1080,
                    height: 1920,
                },
            ],
        };
        write_json_atomic(&manifest_path, &manifest)?;
        created_paths.push(manifest_path.clone());
    }

    let sources_manifest = path.join("sources/manifest.json");
    if !sources_manifest.exists() {
        write_json_atomic(
            &sources_manifest,
            &SourceManifest {
                schema_version: SCHEMA_VERSION,
                sources: Vec::new(),
            },
        )?;
        created_paths.push(sources_manifest);
    }

    Ok(InitResult {
        status: if created_paths.is_empty() {
            "existing"
        } else {
            "created"
        },
        project_path: path.to_path_buf(),
        created_paths,
        manifest_path,
    })
}

/// Legacy generic artifacts and their `natural` variant destinations. Legacy
/// renders were natural-based, so generic state is attributed to `natural`.
const MIGRATION_TARGETS: &[(&str, &str)] = &[
    ("edit/cut-plan.json", "edit/cut-plan-natural.json"),
    ("edit/timeline.json", "edit/timeline-natural.json"),
    (
        "analysis/reframe-plan.json",
        "analysis/reframe/natural/reframe-plan.json",
    ),
    ("finish/finish-plan.json", "finish/natural/finish-plan.json"),
];

/// Migrate a legacy project layout into variant-scoped locations (§6.7).
/// Generic artifacts are copied into their `natural` variant paths and backed up
/// under `migrations/backup-<timestamp>/`, then removed. Idempotent: once the
/// generic artifacts are gone there is nothing left to move.
pub fn migrate_project(project_path: &Path) -> Result<MigrationReport, ProjectError> {
    let project_path = project_path.canonicalize()?;
    let manifest_path = project_path.join("project.json");
    let mut manifest = read_project_manifest(&manifest_path)?;
    // §12.7: a pre-migration manifest keeps its original project_id for
    // backward compatibility and gains an immutable instance id. Never
    // regenerated once present — not on rename, not on relink.
    if manifest.project_instance_id.is_empty() {
        manifest.project_instance_id = fresh_instance_id();
        if manifest.title.is_empty() {
            manifest.title = project_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string();
        }
        write_json_atomic(&manifest_path, &manifest)?;
    }
    let mut migrated = Vec::new();
    let mut skipped = Vec::new();
    let mut backup_dir: Option<PathBuf> = None;

    for &(from_rel, to_rel) in MIGRATION_TARGETS {
        let from = project_path.join(from_rel);
        let to = project_path.join(to_rel);
        if !from.is_file() {
            skipped.push(SkippedArtifact {
                path: from_rel.to_string(),
                reason: "missing".to_string(),
            });
            continue;
        }
        if to.is_file() {
            skipped.push(SkippedArtifact {
                path: from_rel.to_string(),
                reason: "target-exists".to_string(),
            });
            continue;
        }
        let backup_root = backup_dir.clone().unwrap_or_else(|| {
            let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
            project_path.join(format!("migrations/backup-{stamp}"))
        });
        let backup_path = backup_root.join(from_rel);
        if let Some(parent) = backup_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&from, &backup_path)?;
        fs::copy(&from, &to)?;
        fs::remove_file(&from)?;
        backup_dir = Some(backup_root);
        migrated.push(MigratedArtifact {
            from: from_rel.to_string(),
            to: to_rel.to_string(),
            backup: relative_artifact_path(&project_path, &backup_path),
        });
    }

    let status = if migrated.is_empty() {
        "already-current"
    } else {
        "migrated"
    };
    let report = MigrationReport {
        schema_version: SCHEMA_VERSION,
        status: status.to_string(),
        migrated_at: Utc::now(),
        migrated,
        skipped,
        backup_dir,
    };
    write_json_atomic(
        &project_path.join("migrations/migration-report.json"),
        &report,
    )?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_is_idempotent_and_keeps_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let first = init_project(temp.path(), false).unwrap();
        let manifest_before = fs::read(temp.path().join("project.json")).unwrap();
        let second = init_project(temp.path(), false).unwrap();
        let manifest_after = fs::read(temp.path().join("project.json")).unwrap();
        assert_eq!(first.status, "created");
        assert_eq!(second.status, "existing");
        assert_eq!(manifest_before, manifest_after);
        assert!(temp.path().join("analysis/bench/transcribe").is_dir());
    }

    #[test]
    fn identically_named_projects_get_distinct_immutable_identities() {
        // §12.7: the old folder-name-derived project_id made these collide,
        // so two unrelated projects shared one decision-ledger identity.
        let first_root = tempfile::tempdir().unwrap();
        let second_root = tempfile::tempdir().unwrap();
        let first = first_root.path().join("reel");
        let second = second_root.path().join("reel");
        init_project(&first, false).unwrap();
        init_project(&second, false).unwrap();

        let first: ProjectManifest = read_json(&first.join("project.json")).unwrap();
        let second: ProjectManifest = read_json(&second.join("project.json")).unwrap();

        assert_ne!(first.project_instance_id, second.project_instance_id);
        assert_ne!(first.project_id, second.project_id);
        assert!(first.project_instance_id.starts_with("pin_"));
        // The folder name survives as the human title, not as identity.
        assert_eq!(first.title, "reel");
        assert_eq!(second.title, "reel");
    }

    #[test]
    fn migration_backfills_an_instance_id_once_and_never_regenerates_it() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        let manifest_path = temp.path().join("project.json");

        // Simulate a pre-§12.7 manifest: legacy folder-derived id, no instance id.
        let mut legacy: ProjectManifest = read_json(&manifest_path).unwrap();
        legacy.project_id = "project-legacy".into();
        legacy.project_instance_id = String::new();
        legacy.title = String::new();
        write_json_atomic(&manifest_path, &legacy).unwrap();

        migrate_project(temp.path()).unwrap();
        let migrated: ProjectManifest = read_json(&manifest_path).unwrap();
        assert_eq!(migrated.project_id, "project-legacy");
        assert!(migrated.project_instance_id.starts_with("pin_"));

        migrate_project(temp.path()).unwrap();
        let again: ProjectManifest = read_json(&manifest_path).unwrap();
        assert_eq!(again.project_instance_id, migrated.project_instance_id);
    }

    #[test]
    fn init_rejects_a_newer_manifest_schema() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path()).unwrap();
        let manifest = serde_json::json!({ "schema_version": 99 });
        fs::write(
            temp.path().join("project.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let error = init_project(temp.path(), false).unwrap_err();
        assert!(matches!(error, ProjectError::UnsupportedSchema(99)));
    }

    #[test]
    fn migrate_project_moves_legacy_artifacts_and_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        fs::write(temp.path().join("edit/cut-plan.json"), b"cut-plan").unwrap();
        fs::write(temp.path().join("edit/timeline.json"), b"timeline").unwrap();
        fs::write(temp.path().join("analysis/reframe-plan.json"), b"reframe").unwrap();
        fs::write(temp.path().join("finish/finish-plan.json"), b"finish").unwrap();

        let report = migrate_project(temp.path()).unwrap();
        assert_eq!(report.status, "migrated");
        assert_eq!(report.migrated.len(), 4);
        assert!(report.backup_dir.is_some());
        assert_eq!(
            fs::read(temp.path().join("edit/cut-plan-natural.json")).unwrap(),
            b"cut-plan"
        );
        assert_eq!(
            fs::read(temp.path().join("edit/timeline-natural.json")).unwrap(),
            b"timeline"
        );
        assert_eq!(
            fs::read(
                temp.path()
                    .join("analysis/reframe/natural/reframe-plan.json")
            )
            .unwrap(),
            b"reframe"
        );
        assert_eq!(
            fs::read(temp.path().join("finish/natural/finish-plan.json")).unwrap(),
            b"finish"
        );
        assert!(!temp.path().join("edit/cut-plan.json").exists());
        assert!(!temp.path().join("edit/timeline.json").exists());
        assert!(!temp.path().join("analysis/reframe-plan.json").exists());
        assert!(!temp.path().join("finish/finish-plan.json").exists());

        let again = migrate_project(temp.path()).unwrap();
        assert_eq!(again.status, "already-current");
        assert!(again.migrated.is_empty());
        assert_eq!(
            fs::read(temp.path().join("edit/cut-plan-natural.json")).unwrap(),
            b"cut-plan"
        );
    }
}
