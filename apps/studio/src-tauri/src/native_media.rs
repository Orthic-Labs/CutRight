//! Optional Mac accelerator commands; final FFmpeg rendering stays outside.
use crate::security_scoped_bookmarks::authorize_path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use video_media::native::*;

pub(crate) struct NativeMediaState(pub Mutex<Option<Arc<MacMediaWorker>>>);

impl Default for NativeMediaState {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

#[cfg(target_os = "macos")]
fn signing_team(path: &std::path::Path) -> Result<String, String> {
    let output = std::process::Command::new("/usr/bin/codesign")
        .args(["--display", "--verbose=4"])
        .arg(path)
        .output()
        .map_err(|error| format!("read code-signature identity: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "code-signature identity unavailable: {}",
            path.display()
        ));
    }
    let details = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    details
        .lines()
        .find_map(|line| line.strip_prefix("TeamIdentifier="))
        .filter(|team| !team.is_empty() && *team != "not set")
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            format!(
                "signed executable has no TeamIdentifier: {}",
                path.display()
            )
        })
}

#[cfg(target_os = "macos")]
fn packaged_sidecar() -> Result<Option<std::path::PathBuf>, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let Some(macos_dir) = executable.parent() else {
        return Ok(None);
    };
    let Some(contents_dir) = macos_dir.parent() else {
        return Ok(None);
    };
    if contents_dir.file_name().and_then(|name| name.to_str()) != Some("Contents")
        || contents_dir
            .parent()
            .and_then(|app| app.extension())
            .and_then(|extension| extension.to_str())
            != Some("app")
    {
        return Ok(None);
    }
    let sidecar = macos_dir.join("cutright-macos-media");
    if !sidecar.is_file() {
        return Err(format!(
            "signed macOS media sidecar is missing: {}",
            sidecar.display()
        ));
    }
    let sidecar = sidecar
        .canonicalize()
        .map_err(|error| format!("resolve signed macOS media sidecar: {error}"))?;
    let canonical_macos_dir = macos_dir
        .canonicalize()
        .map_err(|error| format!("resolve app MacOS directory: {error}"))?;
    if sidecar.parent() != Some(canonical_macos_dir.as_path()) {
        return Err("signed macOS media sidecar escaped app bundle".into());
    }
    let signature = std::process::Command::new("/usr/bin/codesign")
        .args(["--verify", "--strict"])
        .arg(&sidecar)
        .status()
        .map_err(|error| format!("verify macOS media sidecar signature: {error}"))?;
    if !signature.success() {
        return Err("signed macOS media sidecar failed code-signature verification".into());
    }
    if signing_team(&sidecar)? != signing_team(&executable)? {
        return Err("macOS media sidecar signing identity does not match app".into());
    }
    Ok(Some(sidecar))
}

fn studio_worker() -> Result<MacMediaWorker, String> {
    #[cfg(target_os = "macos")]
    if let Some(sidecar) = packaged_sidecar()? {
        return Ok(MacMediaWorker::with_worker(sidecar));
    }
    MacMediaWorker::new().map_err(|error| error.to_string())
}

fn worker(state: &tauri::State<'_, NativeMediaState>) -> Result<Arc<MacMediaWorker>, String> {
    let mut slot = state.0.lock().map_err(|_| "native_media_state_poisoned")?;
    if slot.is_none() {
        *slot = Some(Arc::new(studio_worker()?));
    }
    Ok(slot.as_ref().expect("initialized").clone())
}

fn ctx(id: String) -> NativeRequestContext {
    NativeRequestContext {
        request_id: id,
        timeout: Duration::from_secs(15),
    }
}

#[tauri::command]
pub(crate) fn native_media_capabilities(
    state: tauri::State<'_, NativeMediaState>,
) -> Result<MacMediaCapabilities, String> {
    worker(&state)?.capabilities().map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn native_media_inspect_asset(
    state: tauri::State<'_, NativeMediaState>,
    request_id: String,
    scope_token: u64,
    source: String,
) -> Result<NativeAssetInfo, String> {
    authorize_path(scope_token, std::path::Path::new(&source), false)?;
    worker(&state)?
        .inspect_asset(&ctx(request_id), std::path::Path::new(&source))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn native_media_analyze_frames(
    state: tauri::State<'_, NativeMediaState>,
    request_id: String,
    scope_token: u64,
    mut request: AnalyzeFramesRequest,
) -> Result<Vec<NativeFrameAnalysis>, String> {
    let mut roots = Vec::new();
    for frame in &request.frames {
        let root = authorize_path(scope_token, &frame.source_path, false)?;
        if !roots.contains(&root) {
            roots.push(root);
        }
    }
    request.allowed_roots = roots;
    worker(&state)?
        .analyze_frames(&ctx(request_id), &request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn native_media_render_caption(
    state: tauri::State<'_, NativeMediaState>,
    request_id: String,
    scope_token: u64,
    mut request: NativeCaptionRequest,
) -> Result<NativeRenderArtifact, String> {
    request.allowed_roots = vec![authorize_path(scope_token, &request.output_path, true)?];
    worker(&state)?
        .render_caption(&ctx(request_id), &request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn native_media_render_preview(
    state: tauri::State<'_, NativeMediaState>,
    request_id: String,
    input_scope_token: u64,
    output_scope_token: u64,
    mut request: NativePreviewRequest,
) -> Result<NativeRenderArtifact, String> {
    let input_root = authorize_path(input_scope_token, &request.input_path, false)?;
    let output_root = authorize_path(output_scope_token, &request.output_path, true)?;
    request.allowed_roots = vec![input_root];
    if !request.allowed_roots.contains(&output_root) {
        request.allowed_roots.push(output_root);
    }
    worker(&state)?
        .render_preview(&ctx(request_id), &request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn native_media_audio_features(
    state: tauri::State<'_, NativeMediaState>,
    request_id: String,
    scope_token: u64,
    mut request: NativeAudioRequest,
) -> Result<NativeAudioFeatures, String> {
    request.allowed_roots = vec![authorize_path(scope_token, &request.source_path, false)?];
    worker(&state)?
        .audio_features(&ctx(request_id), &request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn native_media_cancel(
    state: tauri::State<'_, NativeMediaState>,
    request_id: String,
) -> Result<(), String> {
    let worker = state
        .0
        .lock()
        .map_err(|_| "native_media_state_poisoned")?
        .clone();
    worker
        .map_or(Ok(()), |worker| worker.cancel(&request_id))
        .map_err(|e| e.to_string())
}
