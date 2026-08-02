mod atomic_io;
mod srt;
mod variant;

pub(crate) use atomic_io::{
    fresh_instance_id, hash_file, read_json, read_json_if_file, read_project_manifest,
    read_value_if_file, write_bytes_atomic, write_json_atomic,
};
pub(crate) use srt::*;
pub(crate) use variant::*;

use crate::ProjectError;
use chrono::{DateTime, Utc};
use std::fs;
use std::path::{Path, PathBuf};
use video_core::{SourceManifest, Timebase, Transcript};

pub(crate) fn existing_path(path: PathBuf) -> Option<PathBuf> {
    path.is_file().then_some(path)
}

pub(crate) fn absolute_path(project_path: &Path, path: &Path) -> PathBuf {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_path.join(path)
    };
    path.canonicalize().unwrap_or(path)
}

pub(crate) fn file_mtime(path: &Path) -> Option<DateTime<Utc>> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()
        .map(DateTime::<Utc>::from)
}

pub(crate) fn fps(timebase: &Timebase) -> f64 {
    f64::from(timebase.fps_num) / f64::from(timebase.fps_den)
}

pub(crate) fn relative_artifact_path(project_path: &Path, path: &Path) -> String {
    path.strip_prefix(project_path)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

/// The explicit working/output timebase (§6.6). A project-level
/// `working_timebase` in `project.json` wins; otherwise the first source's
/// timebase; otherwise a sensible NTSC default. Never silently inherits an
/// ambiguous project rate.
pub(crate) fn working_timebase(project_path: &Path, sources: &SourceManifest) -> Timebase {
    let declared = read_json::<serde_json::Value>(&project_path.join("project.json"))
        .ok()
        .and_then(|value| value.get("working_timebase").cloned())
        .and_then(|timebase| {
            let num = timebase
                .get("fps_num")
                .and_then(serde_json::Value::as_u64)?;
            let den = timebase
                .get("fps_den")
                .and_then(serde_json::Value::as_u64)?;
            (num > 0 && den > 0).then_some(Timebase {
                fps_num: num as u32,
                fps_den: den as u32,
            })
        });
    if let Some(timebase) = declared {
        return timebase;
    }
    sources
        .sources
        .first()
        .and_then(|source| source.timebase.clone())
        .unwrap_or(Timebase {
            fps_num: 30_000,
            fps_den: 1_001,
        })
}

/// Convert a millisecond duration to a (possibly fractional) frame count at the
/// given timebase. Used for interchange frame math so render/export never assume
/// the source fps.
pub(crate) fn ms_to_frames_f64(milliseconds: i64, timebase: &Timebase) -> f64 {
    milliseconds as f64 * timebase.fps_num as f64 / (1000.0 * timebase.fps_den as f64)
}

/// The exact set of transcript files [`load_transcripts`] reads, exposed
/// separately so stage receipts can bind the real input paths without
/// re-parsing every transcript.
pub(crate) fn transcript_file_paths(project_path: &Path) -> Result<Vec<PathBuf>, ProjectError> {
    let directory = project_path.join("analysis/transcripts");
    let mut paths = if directory.is_dir() {
        fs::read_dir(directory)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension().and_then(|ext| ext.to_str()) == Some("json")
                    && !path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.contains(".whisperx."))
            })
            .collect::<Vec<_>>()
    } else {
        vec![project_path.join("analysis/transcript.json")]
    };
    paths.sort();
    if paths.is_empty() {
        return Err(ProjectError::InvalidState(
            "transcribe must run before editing".into(),
        ));
    }
    Ok(paths)
}

pub(crate) fn load_transcripts(project_path: &Path) -> Result<Vec<Transcript>, ProjectError> {
    transcript_file_paths(project_path)?
        .into_iter()
        .map(|path| read_json(&path))
        .collect()
}
