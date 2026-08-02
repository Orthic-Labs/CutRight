use crate::io::*;
use crate::receipts;
use crate::ProjectError;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use video_core::{
    models::{SourceEntry, SCHEMA_VERSION},
    SourceManifest,
};
use video_media::{probe, resolve_toolchain};

#[derive(Debug, Serialize)]
pub struct IngestResult {
    pub status: &'static str,
    pub project_path: PathBuf,
    pub manifest_path: PathBuf,
    pub sources: Vec<IngestedSource>,
}

#[derive(Debug, Serialize)]
pub struct IngestedSource {
    pub status: &'static str,
    pub entry: SourceEntry,
}

pub fn ingest_sources(
    project_path: &Path,
    source_paths: &[PathBuf],
    dry_run: bool,
) -> Result<IngestResult, ProjectError> {
    if source_paths.is_empty() {
        return Err(ProjectError::NoSources);
    }
    let project_manifest_path = project_path.join("project.json");
    read_project_manifest(&project_manifest_path)?;
    let manifest_path = project_path.join("sources/manifest.json");
    let mut manifest: SourceManifest = read_json(&manifest_path)?;
    if manifest.schema_version != SCHEMA_VERSION {
        return Err(ProjectError::UnsupportedSchema(manifest.schema_version));
    }

    let mut sources = Vec::with_capacity(source_paths.len());
    let mut newly_ingested_paths: Vec<PathBuf> = Vec::new();
    for source_path in source_paths {
        let canonical_path = fs::canonicalize(source_path)
            .map_err(|_| ProjectError::InvalidSource(source_path.clone()))?;
        let digest = hash_file(&canonical_path)?;
        let hash = format!("blake3:{digest}");
        let path_string = canonical_path.to_string_lossy().into_owned();
        if let Some(existing) = manifest
            .sources
            .iter()
            .find(|source| source.path == path_string)
        {
            if existing.blake3 != hash {
                return Err(ProjectError::SourceChanged(canonical_path));
            }
            sources.push(IngestedSource {
                status: "existing",
                entry: existing.clone(),
            });
            continue;
        }

        let metadata = probe(&canonical_path)?;
        let entry = SourceEntry {
            source_id: format!("source-{}", &digest[..12]),
            path: path_string,
            blake3: hash,
            duration_ms: metadata.duration_ms,
            width: metadata.width,
            height: metadata.height,
            rotation_degrees: metadata.rotation_degrees,
            is_hdr: metadata.is_hdr,
            timebase: metadata.timebase,
        };
        manifest.sources.push(entry.clone());
        newly_ingested_paths.push(canonical_path);
        sources.push(IngestedSource {
            status: "ingested",
            entry,
        });
    }

    if !dry_run {
        write_json_atomic(&manifest_path, &manifest)?;
        if !newly_ingested_paths.is_empty() {
            let inputs: Vec<&Path> = newly_ingested_paths.iter().map(PathBuf::as_path).collect();
            let mut toolchains = BTreeMap::new();
            if let Ok(toolchain) = resolve_toolchain() {
                toolchains.insert("ffmpeg".to_string(), toolchain.identity());
            }
            receipts::write_stage_receipt(
                &receipts::receipt_path_for(&manifest_path),
                "ingest.sources",
                &inputs,
                &serde_json::json!({ "newly_ingested_count": newly_ingested_paths.len() }),
                toolchains,
                &[manifest_path.as_path()],
            )?;
        }
    }
    Ok(IngestResult {
        status: if dry_run {
            "dry-run"
        } else if sources.iter().all(|source| source.status == "existing") {
            "existing"
        } else {
            "ingested"
        },
        project_path: project_path.to_path_buf(),
        manifest_path,
        sources,
    })
}
