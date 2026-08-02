//! Registered-source manifest reads and atomic JSON writes shared by the
//! project-scope and source-integrity commands. Moved out of `main.rs` per
//! REV2 §14.5 — pure move, no behavior change.

use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub(crate) struct SourcesManifest {
    pub(crate) sources: Vec<RegisteredSource>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegisteredSource {
    pub(crate) source_id: String,
    pub(crate) path: String,
    pub(crate) blake3: String,
}

pub(crate) fn read_sources(root: &Path) -> Result<SourcesManifest, String> {
    let path = root.join("sources/manifest.json");
    if !path.exists() {
        return Ok(SourcesManifest {
            sources: Vec::new(),
        });
    }
    let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))
}

pub(crate) fn write_json_atomic(
    root: &Path,
    rel: &str,
    value: &serde_json::Value,
) -> Result<(), String> {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    let temporary = path.with_extension(format!("json.tmp-{}", std::process::id()));
    {
        use std::io::Write;
        let mut file = fs::File::create(&temporary)
            .map_err(|error| format!("{}: {error}", temporary.display()))?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("{}: {error}", temporary.display()))?;
    }
    fs::rename(&temporary, &path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(())
}
