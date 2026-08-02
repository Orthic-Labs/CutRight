//! Project-root resolution, asset-protocol scoping, and the derived
//! staleness/revision signals that `commands::read_snapshot` attaches to a
//! project snapshot. Moved out of `main.rs` per REV2 §14.5 — pure move, no
//! behavior change.

use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, Runtime};

pub(crate) fn named_error(field: &str, message: impl std::fmt::Display) -> String {
    format!("{field}: {message}")
}

pub(crate) fn canonical_project_root(path: &str) -> Result<PathBuf, String> {
    let root = fs::canonicalize(path).map_err(|error| named_error("path", error))?;
    if !root.is_dir() {
        return Err(named_error("path", "must be a project directory"));
    }
    let manifest = root.join("project.json");
    if !manifest.is_file() {
        return Err(named_error("path", "project.json is missing"));
    }
    Ok(root)
}

pub(crate) fn is_regular_file(path: &Path) -> bool {
    fs::metadata(path)
        .map(|meta| meta.is_file())
        .unwrap_or(false)
}

pub(crate) fn blake3_of(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut hasher = blake3::Hasher::new();
    std::io::copy(&mut file, &mut hasher)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

/// Outcome of granting scope to one registered source's media file.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct SourceGrant {
    pub(crate) source_id: String,
    pub(crate) path: String,
    /// The asset-protocol scope was extended to cover this exact file.
    pub(crate) granted: bool,
    /// The file's current BLAKE3 matches the hash it was registered with.
    /// `false` does not block `granted` — a source can still be played back
    /// while flagged unverified, per REV2 §12.4 ("manifest hash match or an
    /// explicit unverified state before playback"); the frontend is
    /// responsible for surfacing this rather than silently trusting it.
    pub(crate) verified: bool,
    pub(crate) error: Option<String>,
}

/// Grant the asset protocol access to exactly the files the current project
/// state needs: registered source media, produced rough-cut/final MP4s, and
/// per-source poster/waveform evidence. Replaces the previous
/// `allow_directory(root, true)`, which handed a shared/untrusted project
/// package the ability to grant arbitrary local paths merely by editing
/// `sources/manifest.json` (REV2 §12.4). Evidence artifacts under the
/// project root are safe to grant on existence alone (they are only ever
/// written there by the pipeline); external source media additionally
/// requires the path to resolve to a regular file and to probe as supported
/// media before scope is extended to it.
pub(crate) fn grant_project_assets<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: &video_project::ProjectSnapshot,
) -> Result<Vec<SourceGrant>, String> {
    let scope = app.asset_protocol_scope();

    let mut evidence_paths: Vec<PathBuf> = Vec::new();
    for variant in &snapshot.variants {
        if let Some(mp4) = &variant.mp4 {
            evidence_paths.push(mp4.clone());
        }
    }
    for final_snapshot in &snapshot.finals {
        evidence_paths.push(final_snapshot.mp4.clone());
    }
    for entry in &snapshot.sources {
        if let Some(poster) = &entry.poster_jpg {
            evidence_paths.push(poster.clone());
        }
        if let Some(waveform) = &entry.waveform_png {
            evidence_paths.push(waveform.clone());
        }
    }
    for path in &evidence_paths {
        if is_regular_file(path) {
            scope
                .allow_file(path)
                .map_err(|error| format!("asset scope for {}: {error}", path.display()))?;
        }
    }

    let mut source_grants = Vec::with_capacity(snapshot.sources.len());
    for entry in &snapshot.sources {
        let source = &entry.source;
        let requested = Path::new(&source.path);
        let canonical = match fs::canonicalize(requested) {
            Ok(path) => path,
            Err(error) => {
                source_grants.push(SourceGrant {
                    source_id: source.source_id.clone(),
                    path: source.path.clone(),
                    granted: false,
                    verified: false,
                    error: Some(error.to_string()),
                });
                continue;
            }
        };
        if !is_regular_file(&canonical) {
            source_grants.push(SourceGrant {
                source_id: source.source_id.clone(),
                path: canonical.to_string_lossy().into_owned(),
                granted: false,
                verified: false,
                error: Some("registered source is not a regular file".into()),
            });
            continue;
        }
        if let Err(error) = video_media::probe(&canonical) {
            source_grants.push(SourceGrant {
                source_id: source.source_id.clone(),
                path: canonical.to_string_lossy().into_owned(),
                granted: false,
                verified: false,
                error: Some(format!("unsupported media: {error}")),
            });
            continue;
        }
        let verified = blake3_of(&canonical)
            .map(|actual| actual == source.blake3)
            .unwrap_or(false);
        match scope.allow_file(&canonical) {
            Ok(()) => source_grants.push(SourceGrant {
                source_id: source.source_id.clone(),
                path: canonical.to_string_lossy().into_owned(),
                granted: true,
                verified,
                error: None,
            }),
            Err(error) => source_grants.push(SourceGrant {
                source_id: source.source_id.clone(),
                path: canonical.to_string_lossy().into_owned(),
                granted: false,
                verified,
                error: Some(error.to_string()),
            }),
        }
    }

    Ok(source_grants)
}

/// Best-effort staleness check: a cut plan that was edited after its rough
/// cut was last rendered no longer describes what is on disk.
pub(crate) fn stale_cut_plan_reason(plan_path: &Path, mp4_path: &Path) -> Option<String> {
    let plan_mtime = fs::metadata(plan_path)
        .and_then(|meta| meta.modified())
        .ok()?;
    let mp4_mtime = fs::metadata(mp4_path)
        .and_then(|meta| meta.modified())
        .ok()?;
    (plan_mtime > mp4_mtime)
        .then(|| "cut plan was modified after the rough cut was last rendered".to_string())
}

/// Best-effort staleness check: a QA report generated before the newest
/// final render no longer covers what would ship.
pub(crate) fn stale_qa_reason(
    root: &Path,
    snapshot: &video_project::ProjectSnapshot,
) -> Option<String> {
    let qa_mtime = fs::metadata(root.join("qa/report.json"))
        .and_then(|meta| meta.modified())
        .ok()?;
    let newest_final = snapshot
        .finals
        .iter()
        .filter_map(|final_snapshot| {
            fs::metadata(&final_snapshot.mp4)
                .and_then(|meta| meta.modified())
                .ok()
        })
        .max()?;
    (newest_final > qa_mtime)
        .then(|| "a final was rendered after this QA report was generated".to_string())
}

fn file_signature(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let since_epoch = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(format!("{}:{}", metadata.len(), since_epoch.as_nanos()))
}

/// A hash over the canonical review inputs and artifact receipts (REV2
/// §12.3): the project identity plus a cheap size+mtime signature of every
/// variant, final, and the QA/bench reports that currently exist. This is
/// deliberately not a full content hash of every rendered MP4 on every
/// snapshot read — that would make opening a project with large renders
/// noticeably slower — but it changes whenever an artifact a reviewer would
/// look at is added, replaced, or removed, which `generated_at` alone (a
/// timestamp of the read, not of the data) cannot signal.
pub(crate) fn project_revision(root: &Path, snapshot: &video_project::ProjectSnapshot) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(snapshot.manifest.project_id.as_bytes());
    for variant in &snapshot.variants {
        hasher.update(variant.id.as_bytes());
        if let Some(mp4) = &variant.mp4 {
            if let Some(signature) = file_signature(mp4) {
                hasher.update(signature.as_bytes());
            }
        }
    }
    for final_snapshot in &snapshot.finals {
        hasher.update(final_snapshot.preset.as_bytes());
        if let Some(signature) = file_signature(&final_snapshot.mp4) {
            hasher.update(signature.as_bytes());
        }
    }
    for path in [
        root.join("qa/report.json"),
        root.join("analysis/bench/transcribe/report.json"),
    ] {
        if let Some(signature) = file_signature(&path) {
            hasher.update(signature.as_bytes());
        }
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}

pub(crate) fn reframe_plan_path(root: &Path) -> PathBuf {
    let primary = root.join("analysis/reframe-plan.json");
    if primary.is_file() {
        primary
    } else {
        root.join("analysis/reframe/natural/reframe-plan.json")
    }
}
