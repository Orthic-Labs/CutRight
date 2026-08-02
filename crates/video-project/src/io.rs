use crate::read_variant_selection;
use crate::ProjectError;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use video_core::{
    models::SCHEMA_VERSION, ProjectManifest, SourceManifest, Timebase, Transcript, Word,
};

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

pub(crate) fn read_json_if_file<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    path.is_file().then(|| read_json(path).ok()).flatten()
}

pub(crate) fn read_value_if_file(path: &Path) -> Option<serde_json::Value> {
    read_json_if_file(path)
}

pub(crate) fn relative_artifact_path(project_path: &Path, path: &Path) -> String {
    path.strip_prefix(project_path)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

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

/// Prefer the variant-scoped artifact, falling back to the legacy generic alias
/// when the variant file does not exist yet.
pub(crate) fn variant_or_generic(
    project_path: &Path,
    variant_rel: &str,
    generic_rel: &str,
) -> PathBuf {
    let variant_path = project_path.join(variant_rel);
    if variant_path.is_file() {
        variant_path
    } else {
        project_path.join(generic_rel)
    }
}

pub(crate) fn variant_plan_path(project_path: &Path, variant: &str) -> PathBuf {
    variant_or_generic(
        project_path,
        &format!("edit/cut-plan-{variant}.json"),
        "edit/cut-plan.json",
    )
}

pub(crate) fn variant_timeline_path(project_path: &Path, variant: &str) -> PathBuf {
    variant_or_generic(
        project_path,
        &format!("edit/timeline-{variant}.json"),
        "edit/timeline.json",
    )
}

pub(crate) fn variant_captions_path(project_path: &Path, variant: &str) -> PathBuf {
    variant_or_generic(
        project_path,
        &format!("edit/captions-{variant}.srt"),
        "edit/captions.srt",
    )
}

pub(crate) fn variant_reframe_path(project_path: &Path, variant: &str) -> PathBuf {
    variant_or_generic(
        project_path,
        &format!("analysis/reframe/{variant}/reframe-plan.json"),
        "analysis/reframe-plan.json",
    )
}

pub(crate) fn variant_finish_path(project_path: &Path, variant: &str) -> PathBuf {
    variant_or_generic(
        project_path,
        &format!("finish/{variant}/finish-plan.json"),
        "finish/finish-plan.json",
    )
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

pub(crate) fn group_words(words: &[Word], gap_threshold_ms: i64) -> Vec<Vec<Word>> {
    let mut groups: Vec<Vec<Word>> = Vec::new();
    for word in words.iter().filter(|word| word.end_ms > word.start_ms) {
        if groups
            .last()
            .and_then(|group| group.last())
            .is_some_and(|last| word.start_ms - last.end_ms > gap_threshold_ms)
        {
            groups.push(vec![word.clone()]);
        } else if let Some(group) = groups.last_mut() {
            group.push(word.clone());
        } else {
            groups.push(vec![word.clone()]);
        }
    }
    groups
}

pub(crate) fn write_srt(path: &Path, words: &[Word]) -> Result<(), ProjectError> {
    let mut body = String::new();
    for (index, group) in group_words(words, 1_000).into_iter().enumerate() {
        let start = group.first().expect("nonempty caption").start_ms;
        let end = group
            .last()
            .expect("nonempty caption")
            .end_ms
            .max(start + 80);
        body.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            index + 1,
            srt_time(start),
            srt_time(end),
            group
                .iter()
                .map(|word| word.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    write_bytes_atomic(path, body.as_bytes())?;
    Ok(())
}

pub(crate) fn srt_time(milliseconds: i64) -> String {
    let total = milliseconds.max(0) as u64;
    format!(
        "{:02}:{:02}:{:02},{:03}",
        total / 3_600_000,
        (total / 60_000) % 60,
        (total / 1_000) % 60,
        total % 1_000
    )
}

/// RFC 3986 percent-encoding for a filesystem path embedded in a `file://`
/// URL (§13.6). Keeps `/` as the path separator and the unreserved set
/// (ALPHA / DIGIT / "-" "." "_" "~") literal; every other byte — spaces,
/// `#`, `%`, and non-ASCII UTF-8 bytes — is escaped as uppercase `%XX` so
/// the URL is unambiguous and round-trips through OTIO-consuming tools.
pub(crate) fn percent_encode_file_url_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

pub(crate) fn hash_file(path: &Path) -> Result<String, ProjectError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, ProjectError> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

pub(crate) fn read_project_manifest(path: &Path) -> Result<ProjectManifest, ProjectError> {
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    let version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| ProjectError::InvalidManifest("missing schema_version".into()))?;
    if version != SCHEMA_VERSION as u64 {
        return Err(ProjectError::UnsupportedSchema(version as u32));
    }
    serde_json::from_value(value).map_err(|error| ProjectError::InvalidManifest(error.to_string()))
}

/// Monotonic counter combined with pid, thread id, and a nanosecond
/// timestamp to build a temp file name that is unique even when several
/// writers race to the same destination directory concurrently (REV2 plan
/// §10.6: PID-only temp names collide under concurrency).
static TEMP_FILE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// A fresh, immutable project instance id (§12.7). Derived the same way the
/// atomic temp names are — pid, thread id, nanosecond clock, and a monotonic
/// counter — so two projects created in the same second, on the same machine,
/// with the same folder name still get distinct identities. Generated exactly
/// once per project and never regenerated.
pub(crate) fn fresh_instance_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let sequence = TEMP_FILE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let seed = format!(
        "{}-{:?}-{nanos}-{sequence}",
        std::process::id(),
        std::thread::current().id()
    );
    format!("pin_{}", &blake3::hash(seed.as_bytes()).to_hex()[..32])
}

pub(crate) fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), ProjectError> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    write_bytes_atomic(path, &bytes)
}

/// Atomic, concurrency-safe file write (REV2 plan §10.6): a uniquely named
/// temp file is created in the destination directory with `create_new` (so
/// two writers can never silently clobber each other's temp file), the
/// file's contents and metadata are fsynced, the temp file is atomically
/// renamed onto the destination, and the parent directory is fsynced where
/// the platform supports it so the rename itself is durable.
pub(crate) fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), ProjectError> {
    let parent = path.parent().ok_or_else(|| {
        ProjectError::InvalidState(format!("{} has no parent directory", path.display()))
    })?;
    fs::create_dir_all(parent)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            ProjectError::InvalidState(format!("{} has no file name", path.display()))
        })?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let sequence = TEMP_FILE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".{file_name}.tmp-{}-{:?}-{nanos}-{sequence}",
        std::process::id(),
        std::thread::current().id()
    ));
    let write_result = (|| -> Result<(), ProjectError> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    let rename_result = fs::rename(&temporary, path);
    if rename_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    rename_result?;
    // Parent-directory fsync is not supported on every platform (notably
    // Windows); a best-effort sync is still worth attempting where it is.
    if let Ok(dir) = fs::File::open(parent) {
        let _ = dir.sync_all();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_source_bytes_with_blake3() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.bin");
        fs::write(&source, b"cutright").unwrap();
        assert_eq!(
            hash_file(&source).unwrap(),
            blake3::hash(b"cutright").to_hex().to_string()
        );
    }

    #[test]
    fn percent_encode_file_url_path_covers_reserved_and_unicode_bytes() {
        // §13.6 fixtures: spaces, Unicode, `#`, `%`, and non-ASCII paths must
        // round-trip through a real percent-encoder, not a single `' '` ->
        // `%20` substitution.
        assert_eq!(
            percent_encode_file_url_path("/captures/cam one.mov"),
            "/captures/cam%20one.mov"
        );
        assert_eq!(
            percent_encode_file_url_path("/captures/café münchen.mov"),
            "/captures/caf%C3%A9%20m%C3%BCnchen.mov"
        );
        assert_eq!(
            percent_encode_file_url_path("/captures/take#3 100%.mov"),
            "/captures/take%233%20100%25.mov"
        );
        assert_eq!(
            percent_encode_file_url_path("/captures/日本語.mov"),
            "/captures/%E6%97%A5%E6%9C%AC%E8%AA%9E.mov"
        );
        // Unreserved characters and the path separator stay literal.
        assert_eq!(
            percent_encode_file_url_path("/a-b/c_d.e~f/g.mov"),
            "/a-b/c_d.e~f/g.mov"
        );
    }
}
