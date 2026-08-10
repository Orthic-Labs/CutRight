//! Signed media-pack resolution shared by `videoctl doctor`.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct MediaTools {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub mode: &'static str,
}

fn pair(root: &Path, mode: &'static str) -> Option<MediaTools> {
    let ffmpeg = root.join("ffmpeg");
    let ffprobe = root.join("ffprobe");
    (ffmpeg.is_file() && ffprobe.is_file()).then_some(MediaTools {
        ffmpeg,
        ffprobe,
        mode,
    })
}

fn signed_pack_roots(executable_dir: &Path) -> [PathBuf; 3] {
    [
        executable_dir.join("packs/media/bin"),
        executable_dir.join("../packs/media/bin"),
        executable_dir.join("../Resources/packs/media/bin"),
    ]
}

pub fn resolve() -> Result<MediaTools, String> {
    #[cfg(debug_assertions)]
    if let Some(ffmpeg) = std::env::var_os("CUTRIGHT_FFMPEG").map(PathBuf::from) {
        let ffprobe = std::env::var_os("CUTRIGHT_FFPROBE")
            .map(PathBuf::from)
            .or_else(|| ffmpeg.parent().map(|parent| parent.join("ffprobe")))
            .ok_or_else(|| "CUTRIGHT_FFPROBE is absent".to_string())?;
        if ffmpeg.is_file() && ffprobe.is_file() {
            return Ok(MediaTools {
                ffmpeg,
                ffprobe,
                mode: "development-override",
            });
        }
        return Err("development media override is incomplete".into());
    }

    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    if let Some(root) = executable.parent() {
        for candidate in signed_pack_roots(root) {
            if let Some(tools) = pair(&candidate, "signed-pack") {
                return Ok(tools);
            }
        }
    }

    #[cfg(debug_assertions)]
    if let Some(tools) = pair(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../runtime/packs/media/bin"),
        "signed-pack-development-root",
    ) {
        return Ok(tools);
    }

    Err("runtime_pack_unavailable: signed media pack is absent".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_app_resources_are_a_signed_pack_candidate() {
        let roots = signed_pack_roots(Path::new(
            "/Applications/CutRight Studio.app/Contents/MacOS",
        ));
        assert!(roots.contains(&PathBuf::from(
            "/Applications/CutRight Studio.app/Contents/MacOS/../Resources/packs/media/bin"
        )));
    }
}
