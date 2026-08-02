//! Per-project Studio settings: the optional-cloud-analysis consent, hard
//! budget, upload policy, provider selection, and a (name-only) credential
//! reference, plus a couple of cheap read-only engine facts the Settings
//! surface shows alongside them.
//!
//! REV2 §15.6 requires explicit per-project consent, a hard budget limit, a
//! proxy-versus-source upload policy, and a retention/deletion action before
//! any cloud provider call can exist. Phase 8 itself (the actual provider
//! adapters) is deliberately not built yet (`STATUS.md`) — this module only
//! gives that future phase somewhere to read its configuration from, and
//! gives a user somewhere to consent, budget, and (later) point at a
//! credential today. `provider` is therefore restricted to `"disabled"`
//! until a real adapter exists; the frontend must not offer a provider that
//! does nothing.
//!
//! Persisted at `<project>/.cutright-studio/settings.json` — a Studio-owned
//! sidecar, the same location pattern `project_identity.rs` uses for
//! `identity.json`. Nothing here is global machine state: every read/write
//! is scoped to the caller's own project root.
//!
//! ## Why this file can never hold a credential value
//!
//! A pasted API key ends up in React state, in Tauri's IPC log if debug
//! logging is on, in a crash report, in a screen recording or screenshot of
//! this very settings screen, and in this JSON file itself if it were ever
//! stored — none of which the operator can fully control after the fact.
//! The only thing Studio persists is the *name* of an environment variable
//! the operator sets in the engine's own process environment
//! (`credential_env_var`); `env_var_present` below reports only whether that
//! variable is currently set, never its value, a prefix, or a masked
//! preview.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

const SCHEMA_VERSION: u32 = 1;
const SETTINGS_REL: &str = ".cutright-studio/settings.json";
/// §15.6 cloud analysis is not built (STATUS.md `phase_8_cloud_awaiting_approval`).
/// The only provider identifier a project may currently persist.
const KNOWN_PROVIDERS: &[&str] = &["disabled"];
const MAX_ENV_VAR_NAME_LEN: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UploadPolicy {
    /// Send a downscaled/transcoded proxy, never the source master. Default.
    Proxy,
    /// Send the original source media. Requires the operator to opt in
    /// explicitly; never the default.
    Source,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSettings {
    pub schema_version: u32,
    /// Explicit per-project consent (REV2 §15.6). OFF by default and never
    /// inferred from any other setting.
    pub consent: bool,
    /// Hard budget limit in US dollars. Must be zero or positive; see
    /// `validate`.
    pub hard_budget_usd: f64,
    pub upload_policy: UploadPolicy,
    /// Provider identifier. Restricted to `KNOWN_PROVIDERS` — see module
    /// docs for why this cannot yet be "gemini" or "twelvelabs".
    pub provider: String,
    /// NAME of an environment variable the operator has set in the
    /// engine's own process environment. Never a credential value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_env_var: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl Default for CloudSettings {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            consent: false,
            hard_budget_usd: 0.0,
            upload_policy: UploadPolicy::Proxy,
            provider: "disabled".into(),
            credential_env_var: None,
            updated_at: None,
        }
    }
}

impl CloudSettings {
    /// Every field-level rule the write path enforces, checked once here so
    /// `write` cannot persist a value the read path or the frontend would
    /// have to work around. Kept intentionally strict on
    /// `credential_env_var`: an environment-variable NAME is short,
    /// upper-cased, and made of `[A-Z0-9_]` — a pasted API key value is
    /// none of those things (mixed case, punctuation, often 30+ chars), so
    /// this doubles as a defensive check against accidentally storing one.
    pub fn validate(&self) -> Result<(), String> {
        if !self.hard_budget_usd.is_finite() || self.hard_budget_usd < 0.0 {
            return Err("hard_budget_usd: must be zero or a positive number".into());
        }
        if !KNOWN_PROVIDERS.contains(&self.provider.as_str()) {
            return Err(format!(
                "provider: {:?} has no adapter configured; only {:?} is valid until a provider ships",
                self.provider, KNOWN_PROVIDERS
            ));
        }
        if let Some(name) = &self.credential_env_var {
            if name.is_empty() {
                return Err(
                    "credential_env_var: must be omitted (null), not an empty string".into(),
                );
            }
            if name.len() > MAX_ENV_VAR_NAME_LEN {
                return Err(format!(
                    "credential_env_var: must be {MAX_ENV_VAR_NAME_LEN} characters or fewer"
                ));
            }
            let first = name.chars().next().unwrap();
            let looks_like_env_name = (first.is_ascii_uppercase() || first == '_')
                && name
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
            if !looks_like_env_name {
                return Err(
                    "credential_env_var: must look like an environment variable name (A-Z, 0-9, _ — starting with a letter or underscore), not a credential value".into(),
                );
            }
        }
        Ok(())
    }
}

/// Read-only engine facts the Settings surface shows: the resolved
/// FFmpeg/FFprobe toolchain identity (REV2 §10.3) and its probed
/// capabilities. Deliberately not the full `videoctl doctor` report — that
/// spawns a dozen-plus probe processes (temp-dir smoke, sidecar
/// materialize, software encode, etc.) and this is meant to be cheap to
/// show every time Settings opens. Toolchain resolution already spawns the
/// two `-version` calls `doctor`'s own `core.ffmpeg.execute` /
/// `core.ffprobe.execute` checks perform, so `resolved: true` here is a
/// real (if partial) readiness signal, not a placeholder; `note` says so
/// explicitly rather than implying full coverage.
#[derive(Debug, Clone, Serialize)]
pub struct EngineCapabilities {
    pub has_zscale: bool,
    pub has_h264_videotoolbox: bool,
    pub has_prores_ks: bool,
    pub has_lut3d: bool,
    pub has_colortemperature: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineStatus {
    pub resolved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toolchain_identity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ffmpeg_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ffmpeg_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ffprobe_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<EngineCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub note: &'static str,
}

const ENGINE_STATUS_NOTE: &str =
    "toolchain identity only — run `videoctl doctor --profile all` for a full readiness report";

pub(crate) fn engine_status() -> EngineStatus {
    match video_media::resolve_toolchain() {
        Ok(toolchain) => EngineStatus {
            resolved: true,
            toolchain_identity: Some(toolchain.identity()),
            ffmpeg_version: Some(toolchain.version.clone()),
            ffmpeg_path: Some(toolchain.ffmpeg.to_string_lossy().into_owned()),
            ffprobe_path: Some(toolchain.ffprobe.to_string_lossy().into_owned()),
            capabilities: Some(EngineCapabilities {
                has_zscale: toolchain.capabilities.has_zscale,
                has_h264_videotoolbox: toolchain.capabilities.has_h264_videotoolbox,
                has_prores_ks: toolchain.capabilities.has_prores_ks,
                has_lut3d: toolchain.capabilities.has_lut3d,
                has_colortemperature: toolchain.capabilities.has_colortemperature,
            }),
            error: None,
            note: ENGINE_STATUS_NOTE,
        },
        Err(error) => EngineStatus {
            resolved: false,
            toolchain_identity: None,
            ffmpeg_version: None,
            ffmpeg_path: None,
            ffprobe_path: None,
            capabilities: None,
            error: Some(error.to_string()),
            note: ENGINE_STATUS_NOTE,
        },
    }
}

/// Whether `name` names an environment variable that is currently set in
/// this process's environment — presence only. Never reads or returns the
/// variable's value. `name` is validated with the same env-var-name shape
/// `CloudSettings::validate` uses, so this cannot be used to probe an
/// arbitrary string.
pub(crate) fn env_var_present(name: &str) -> Result<bool, String> {
    let probe = CloudSettings {
        credential_env_var: Some(name.to_string()),
        ..CloudSettings::default()
    };
    probe.validate()?;
    Ok(std::env::var_os(name).is_some())
}

fn settings_path(root: &Path) -> std::path::PathBuf {
    root.join(SETTINGS_REL)
}

pub(crate) fn read(root: &Path) -> Result<CloudSettings, String> {
    let path = settings_path(root);
    if !path.is_file() {
        return Ok(CloudSettings::default());
    }
    let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))
}

pub(crate) fn write(root: &Path, mut settings: CloudSettings) -> Result<CloudSettings, String> {
    settings.validate()?;
    settings.schema_version = SCHEMA_VERSION;
    settings.updated_at = Some(Utc::now());
    write_atomic(&settings_path(root), &settings)?;
    Ok(settings)
}

/// The retention/deletion action REV2 §15.6 requires: reset this project's
/// cloud settings to the (consent-off) default and remove any cloud
/// analysis cache directory. `analysis/cloud/` does not exist yet — Phase 8
/// is not built — so removing it today is a no-op that becomes real the
/// moment that phase starts writing there, without this command needing to
/// change.
pub(crate) fn delete(root: &Path) -> Result<CloudSettings, String> {
    let cache_dir = root.join("analysis/cloud");
    if cache_dir.is_dir() {
        fs::remove_dir_all(&cache_dir)
            .map_err(|error| format!("{}: {error}", cache_dir.display()))?;
    }
    write(root, CloudSettings::default())
}

fn write_atomic(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    let temp = parent.join(format!(".settings.tmp-{}", std::process::id()));
    {
        let mut file =
            fs::File::create(&temp).map_err(|error| format!("{}: {error}", temp.display()))?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("{}: {error}", temp.display()))?;
    }
    fs::rename(&temp, path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    fn scratch_root() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "cutright-studio-settings-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn default_settings_have_consent_off() {
        let settings = CloudSettings::default();
        assert!(!settings.consent);
        assert_eq!(settings.hard_budget_usd, 0.0);
        assert_eq!(settings.provider, "disabled");
        assert!(settings.credential_env_var.is_none());
    }

    #[test]
    fn reading_an_unwritten_project_returns_defaults_with_consent_off() {
        let root = scratch_root();
        let settings = read(&root).unwrap();
        assert!(!settings.consent);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn write_then_read_round_trips_and_stamps_updated_at() {
        let root = scratch_root();
        let written = write(
            &root,
            CloudSettings {
                consent: true,
                hard_budget_usd: 12.5,
                upload_policy: UploadPolicy::Source,
                credential_env_var: Some("CUTRIGHT_GEMINI_KEY".into()),
                ..CloudSettings::default()
            },
        )
        .unwrap();
        assert!(written.updated_at.is_some());
        let read_back = read(&root).unwrap();
        assert!(read_back.consent);
        assert_eq!(read_back.hard_budget_usd, 12.5);
        assert_eq!(read_back.upload_policy, UploadPolicy::Source);
        assert_eq!(
            read_back.credential_env_var.as_deref(),
            Some("CUTRIGHT_GEMINI_KEY")
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn negative_budget_is_rejected() {
        let root = scratch_root();
        let result = write(
            &root,
            CloudSettings {
                hard_budget_usd: -5.0,
                ..CloudSettings::default()
            },
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("hard_budget_usd"));
        // The rejected write must not have persisted anything.
        assert!(!settings_path(&root).is_file());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn non_finite_budget_is_rejected() {
        let root = scratch_root();
        let result = write(
            &root,
            CloudSettings {
                hard_budget_usd: f64::NAN,
                ..CloudSettings::default()
            },
        );
        assert!(result.is_err());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unknown_provider_is_rejected() {
        let root = scratch_root();
        let result = write(
            &root,
            CloudSettings {
                provider: "gemini".into(),
                ..CloudSettings::default()
            },
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("provider"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn credential_env_var_must_look_like_a_name_not_a_value() {
        let root = scratch_root();
        // A plausible pasted API key: lowercase, punctuated, long.
        let result = write(
            &root,
            CloudSettings {
                credential_env_var: Some("AIzaSyD-fake-not-a-real-key-example12345".into()),
                ..CloudSettings::default()
            },
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("credential_env_var"));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn credential_env_var_accepts_a_real_env_var_name() {
        let root = scratch_root();
        let result = write(
            &root,
            CloudSettings {
                credential_env_var: Some("CUTRIGHT_GEMINI_KEY".into()),
                ..CloudSettings::default()
            },
        );
        assert!(result.is_ok());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn env_var_present_reports_presence_only() {
        // SAFETY: test-local, single-threaded within this test's own scope
        // for this specific var name (unique per invocation would be
        // stronger, but `set_var` is only unsafe w.r.t. concurrent readers
        // of the same key from other threads, and no other test in this
        // module touches CUTRIGHT_STUDIO_TEST_PROBE_VAR).
        unsafe {
            std::env::set_var("CUTRIGHT_STUDIO_TEST_PROBE_VAR", "irrelevant-value");
        }
        assert!(env_var_present("CUTRIGHT_STUDIO_TEST_PROBE_VAR").unwrap());
        assert!(!env_var_present("CUTRIGHT_STUDIO_TEST_PROBE_VAR_UNSET").unwrap());
        unsafe {
            std::env::remove_var("CUTRIGHT_STUDIO_TEST_PROBE_VAR");
        }
    }

    #[test]
    fn env_var_present_rejects_a_malformed_name() {
        assert!(env_var_present("not-a-valid-name").is_err());
    }

    #[test]
    fn delete_resets_settings_to_defaults_and_removes_any_cloud_cache() {
        let root = scratch_root();
        write(
            &root,
            CloudSettings {
                consent: true,
                hard_budget_usd: 40.0,
                ..CloudSettings::default()
            },
        )
        .unwrap();
        let cache_dir = root.join("analysis/cloud");
        fs::create_dir_all(&cache_dir).unwrap();
        fs::write(cache_dir.join("cached.json"), b"{}").unwrap();

        let reset = delete(&root).unwrap();
        assert!(!reset.consent);
        assert_eq!(reset.hard_budget_usd, 0.0);
        assert!(!cache_dir.is_dir());
        let read_back = read(&root).unwrap();
        assert!(!read_back.consent);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn delete_is_a_no_op_success_when_there_is_no_cloud_cache_yet() {
        let root = scratch_root();
        let reset = delete(&root).unwrap();
        assert!(!reset.consent);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn engine_status_resolves_the_real_system_toolchain_or_reports_why_not() {
        let status = engine_status();
        assert_eq!(status.note, ENGINE_STATUS_NOTE);
        if status.resolved {
            assert!(status.toolchain_identity.is_some());
            assert!(status.ffmpeg_version.is_some());
            assert!(status.capabilities.is_some());
        } else {
            assert!(status.error.is_some());
        }
    }
}
