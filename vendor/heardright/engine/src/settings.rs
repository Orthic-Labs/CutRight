use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use heardright_core::settings::{
    migrate, SettingsBlob, DEFAULT_AI_POLISH_MODE, DEFAULT_ASR_BACKEND,
    DEFAULT_SCREENSHOT_DESTINATION, DEFAULT_TRANSCRIPT_CLEANUP,
};

const APP_DATA_DIR_NAME: &str = "HeardRightNext";
static SETTINGS_OVERRIDE: OnceLock<Mutex<Option<SettingsBlob>>> = OnceLock::new();

const CALIBRATION_ROUTE_FILE: &str = "recognition-route.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedRecognitionRoute {
    pub id: String,
    pub encoder_compute: String,
    pub decoder_compute: String,
    pub provider: String,
    #[serde(default)]
    pub adapter_index: Option<i32>,
    #[serde(default)]
    pub adapter_name: Option<String>,
}

fn calibration_route_path() -> PathBuf {
    app_data_root().join(CALIBRATION_ROUTE_FILE)
}

pub fn persisted_recognition_route() -> Option<PersistedRecognitionRoute> {
    let bytes = fs::read(calibration_route_path()).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Persist only a route that completed the fixed onboarding read. Write+rename
/// ensures normal dictation never sees a partially-written selection.
pub fn persist_recognition_route(route: &PersistedRecognitionRoute) -> Result<(), String> {
    let path = calibration_route_path();
    let parent = path
        .parent()
        .ok_or_else(|| "recognition route has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("create recognition route dir: {error}"))?;
    let bytes =
        serde_json::to_vec(route).map_err(|error| format!("encode recognition route: {error}"))?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes).map_err(|error| format!("write recognition route: {error}"))?;
    fs::rename(&temporary, &path).map_err(|error| format!("activate recognition route: {error}"))
}

/// Calibration runs in an isolated process. These overrides let each candidate
/// use its exact compute route without mutating normal dictation settings.
pub fn apply_calibration_route_environment(route: &PersistedRecognitionRoute) {
    std::env::set_var("HR_ONBOARDING_CALIBRATION", "1");
    std::env::set_var("HR_CALIBRATION_ENCODER_COMPUTE", &route.encoder_compute);
    std::env::set_var("HR_CALIBRATION_DECODER_COMPUTE", &route.decoder_compute);
    if let Some(index) = route.adapter_index {
        std::env::set_var("HR_DML_DEVICE_ID", index.to_string());
    } else {
        std::env::remove_var("HR_DML_DEVICE_ID");
    }
}

pub fn onboarding_calibration_active() -> bool {
    std::env::var("HR_ONBOARDING_CALIBRATION").as_deref() == Ok("1")
}

/// Apply a calibrated DirectML adapter before normal model creation. CoreML
/// routes are read directly by `routed_compute`; DirectML needs its adapter in
/// the provider builder's process environment.
pub fn apply_persisted_recognition_route_environment() {
    if let Some(route) = persisted_recognition_route() {
        if route.provider == "dml" {
            if let Some(index) = route.adapter_index {
                std::env::set_var("HR_DML_DEVICE_ID", index.to_string());
            } else {
                std::env::remove_var("HR_DML_DEVICE_ID");
            }
        }
    }
}

fn calibration_compute_override(role: &str) -> Option<String> {
    let override_key = match role {
        "encoder" => "HR_CALIBRATION_ENCODER_COMPUTE",
        "decoder" => "HR_CALIBRATION_DECODER_COMPUTE",
        _ => return None,
    };
    std::env::var(override_key)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn persisted_compute(role: &str) -> Option<String> {
    persisted_recognition_route().map(|route| match role {
        "encoder" => route.encoder_compute,
        "decoder" => route.decoder_compute,
        _ => unreachable!(),
    })
}

pub fn replace_runtime_config(config: SettingsBlob) {
    let mut blob = config;
    migrate(&mut blob);
    let cell = SETTINGS_OVERRIDE.get_or_init(|| Mutex::new(None));
    if let Ok(mut slot) = cell.lock() {
        *slot = Some(blob);
    }
}

pub fn app_data_root() -> PathBuf {
    if let Some(path) = std::env::var_os("HR_APP_DATA_DIR").filter(|path| !path.is_empty()) {
        return PathBuf::from(path);
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(root) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(root).join(APP_DATA_DIR_NAME);
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join(APP_DATA_DIR_NAME);
        }
    }
    if let Some(root) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(root).join(APP_DATA_DIR_NAME);
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join(APP_DATA_DIR_NAME);
    }
    PathBuf::from(".").join(APP_DATA_DIR_NAME)
}

pub fn wake_lab_marker_present() -> bool {
    app_data_root().join("owner-idle-wake.enabled").is_file()
}

fn settings_path() -> PathBuf {
    app_data_root().join("settings.json")
}

fn load_settings() -> SettingsBlob {
    if let Some(blob) = SETTINGS_OVERRIDE
        .get()
        .and_then(|cell| cell.lock().ok().and_then(|slot| slot.clone()))
    {
        return blob;
    }
    let path = settings_path();
    let Ok(bytes) = fs::read(path) else {
        let mut blob = SettingsBlob::default();
        migrate(&mut blob);
        return blob;
    };
    match serde_json::from_slice::<SettingsBlob>(&bytes) {
        Ok(mut blob) => {
            migrate(&mut blob);
            blob
        }
        Err(_) => {
            let mut blob = SettingsBlob::default();
            migrate(&mut blob);
            blob
        }
    }
}

/// Pro entitlement as pushed by the shell from the keyring license. Production
/// default is fail-closed (false) so a missing/Free license blocks Pro-only
/// voice commands. Under `cfg!(test)`/engine-test-mode the default is `true` so
/// the existing engine command tests don't each need to push a license; a test
/// that wants the Free path pushes `is_pro: Some(false)` via replace_runtime_config.
pub fn is_pro() -> bool {
    match load_settings().is_pro {
        Some(value) => value,
        None => {
            cfg!(test)
                || std::env::var("HEARDRIGHT_ENGINE_TEST_MODE")
                    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false)
        }
    }
}

pub fn diagnostics_enabled() -> bool {
    load_settings()
        .diagnostics_enabled
        .unwrap_or(heardright_core::settings::DEFAULT_DIAGNOSTICS_ENABLED)
}

fn compute_profile(value: Option<String>, fallback: &str, allow_inherit: bool) -> String {
    match value.as_deref() {
        Some("neural_engine") => "neural_engine",
        Some("cpu_gpu") => "cpu_gpu",
        Some("cpu_only") => "cpu_only",
        Some("inherit") if allow_inherit => "inherit",
        _ => fallback,
    }
    .to_string()
}

pub fn asr_compute_profile() -> String {
    compute_profile(load_settings().asr_compute_profile, "automatic", false)
}

pub fn asr_encoder_compute() -> String {
    if let Some(value) = calibration_compute_override("encoder") {
        return compute_profile(Some(value), "inherit", true);
    }
    let configured = load_settings().asr_encoder_compute;
    if configured
        .as_deref()
        .is_some_and(|value| value != "inherit")
    {
        return compute_profile(configured, "inherit", true);
    }
    if let Some(value) = persisted_compute("encoder") {
        return compute_profile(Some(value), "inherit", true);
    }
    compute_profile(configured, "inherit", true)
}

pub fn asr_decoder_compute() -> String {
    if let Some(value) = calibration_compute_override("decoder") {
        return compute_profile(Some(value), "inherit", true);
    }
    let configured = load_settings().asr_decoder_compute;
    if configured
        .as_deref()
        .is_some_and(|value| value != "inherit")
    {
        return compute_profile(configured, "inherit", true);
    }
    if let Some(value) = persisted_compute("decoder") {
        return compute_profile(Some(value), "inherit", true);
    }
    compute_profile(configured, "inherit", true)
}

pub fn asr_detailed_telemetry() -> bool {
    !onboarding_calibration_active() && load_settings().telemetry_usage.unwrap_or(false)
}

pub fn diagnostic_audio_capture() -> bool {
    load_settings().diagnostic_audio_capture.unwrap_or(false)
}

pub fn diagnostic_unredacted_logs() -> bool {
    load_settings().telemetry_usage.unwrap_or(false)
}

pub fn screenshot_destination() -> String {
    match load_settings()
        .screenshot_destination
        .unwrap_or_else(|| DEFAULT_SCREENSHOT_DESTINATION.to_string())
        .as_str()
    {
        "disk" => "disk".to_string(),
        "both" => "both".to_string(),
        _ => DEFAULT_SCREENSHOT_DESTINATION.to_string(),
    }
}

pub fn snippets() -> HashMap<String, String> {
    load_settings().snippets.unwrap_or_default()
}

/// Deterministic text replacements ("always fix X to Y"), applied at the
/// local polish layer. Distinct from vocabulary and snippets.
pub fn replacements() -> HashMap<String, String> {
    load_settings().replacements.unwrap_or_default()
}

/// Saved mic selection string (e.g. `id:2:USB Mic`), or None for system default.
/// The worker resolves this to a device index via `resolve_mic_selection`.
pub fn input_device() -> Option<String> {
    load_settings().input_device
}

pub fn asr_backend() -> String {
    match load_settings()
        .asr_backend
        .unwrap_or_else(|| DEFAULT_ASR_BACKEND.to_string())
        .as_str()
    {
        "parakeet-unified" | "parakeet" | "parakeet-rnnt" | "rnnt" => {
            DEFAULT_ASR_BACKEND.to_string()
        }
        "parakeet-tdt" => "parakeet-tdt".to_string(),
        "whisper" | "whisper-cli" => "whisper".to_string(),
        _ => DEFAULT_ASR_BACKEND.to_string(),
    }
}

pub fn transcript_cleanup() -> String {
    match load_settings()
        .transcript_cleanup
        .unwrap_or_else(|| DEFAULT_TRANSCRIPT_CLEANUP.to_string())
        .as_str()
    {
        "clean" => "clean".to_string(),
        "aggressive" => "aggressive".to_string(),
        _ => DEFAULT_TRANSCRIPT_CLEANUP.to_string(),
    }
}

pub fn ai_polish_mode() -> String {
    match load_settings()
        .ai_polish_mode
        .unwrap_or_else(|| DEFAULT_AI_POLISH_MODE.to_string())
        .as_str()
    {
        "app" => "app".to_string(),
        _ => DEFAULT_AI_POLISH_MODE.to_string(),
    }
}

/// Recognition language. `"auto"` = Parakeet auto-detect (free). A specific code
/// = lock Whisper to that language (Pro; the shell already sanitized this to
/// "auto" for non-Pro before pushing). Precedence: `HR_DICTATION_LANG` env
/// (dev/power-user) → pushed setting → "auto".
pub fn dictation_language() -> String {
    // The live pushed config / on-disk settings WIN over the spawn-time
    // HR_DICTATION_LANG env. The supervisor bakes that env once at spawn, so if
    // env won, a language change in Settings (pushed via ReplaceEngineConfig)
    // would never reach the running engine — it would stay on the boot language
    // forever. The env is only a last-resort seed for running the engine binary
    // standalone (no shell pushing config).
    if let Some(v) = load_settings()
        .dictation_language
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())
    {
        return v;
    }
    std::env::var("HR_DICTATION_LANG")
        .ok()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "auto".to_string())
}
