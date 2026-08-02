use crate::ProjectError;
use serde::Serialize;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use video_core::{models::SCHEMA_VERSION, ProjectManifest};

pub(crate) fn read_json_if_file<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    path.is_file().then(|| read_json(path).ok()).flatten()
}

pub(crate) fn read_value_if_file(path: &Path) -> Option<serde_json::Value> {
    read_json_if_file(path)
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
}
