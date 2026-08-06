/// Apply a partial settings update (PATCH-style) to `blob`. Only allowlisted
/// keys are written; unknown keys and invalid values return `Err`. Pure — no
/// lock, no disk. The IPC layer wraps this with locking + persistence.
///
/// All-or-nothing: validates every key against a clone first, then commits
/// only if every key passes. A single invalid value leaves `blob` unchanged.
pub fn apply_patch_to(blob: &mut SettingsBlob, patch: &Value) -> Result<(), String> {
    let obj = patch
        .as_object()
        .ok_or_else(|| "patch must be a JSON object".to_string())?;
    let mut staged = blob.clone();
    fn bool_value(v: &Value, key: &str) -> Result<bool, String> {
        v.as_bool().ok_or_else(|| format!("{key} must be bool"))
    }
    for (k, v) in obj {
        match k.as_str() {
            "close_behavior" => {
                let s = v
                    .as_str()
                    .ok_or("close_behavior must be string")?
                    .to_string();
                // Spec: three close behaviors. `destroy` is the world-class
                // default for new installs — the hub is ephemeral, the engine
                // is the only long-lived resident. `minimize` keeps the old
                // hide-to-tray behavior. `quit` exits the whole app.
                if !["minimize", "quit", "destroy"].contains(&s.as_str()) {
                    return Err(format!("invalid close_behavior: {s}"));
                }
                staged.close_behavior = Some(s);
            }
            "app_theme" => {
                let s = v.as_str().ok_or("app_theme must be string")?.to_string();
                if !["auto", "ember", "ember-light"].contains(&s.as_str()) {
                    return Err(format!("invalid app_theme: {s}"));
                }
                staged.app_theme = Some(s);
            }
            "app_font" => {
                let s = v.as_str().ok_or("app_font must be string")?.to_string();
                if !["hanken", "inter", "source-serif", "geist", "jetbrains"].contains(&s.as_str())
                {
                    return Err(format!("invalid app_font: {s}"));
                }
                staged.app_font = Some(s);
            }
            "audio_conditioning" => {
                staged.audio_conditioning = Some(bool_value(v, "audio_conditioning")?)
            }
            "hotkey_toggle" => staged.hotkey_toggle = v.as_str().map(String::from),
            "hotkey_cancel" => staged.hotkey_cancel = v.as_str().map(String::from),
            "hotkey_open_hub" => staged.hotkey_open_hub = v.as_str().map(String::from),
            "input_device" => staged.input_device = v.as_str().map(String::from),
            "sound_effects_on" => {
                staged.sound_effects_on = Some(bool_value(v, "sound_effects_on")?)
            }
            "sound_effects_volume" => {
                let n = v.as_u64().ok_or("sound_effects_volume must be u64")?;
                if n > 100 {
                    return Err(format!("sound_effects_volume out of range: {n}"));
                }
                staged.sound_effects_volume = Some(n as u8);
            }
            "pill_visibility" => {
                let s = v
                    .as_str()
                    .ok_or("pill_visibility must be string")?
                    .to_string();
                // Spec: two states — `always` (visible at rest) and
                // `recording_only` (visible only while recording). The
                // historical `during_dictation` alias is accepted so an older
                // install that wrote that value still round-trips.
                if !["always", "recording_only", "during_dictation"].contains(&s.as_str()) {
                    return Err(format!("invalid pill_visibility: {s}"));
                }
                staged.pill_visibility = Some(s);
            }
            "asr_backend" => {
                let s = v.as_str().ok_or("asr_backend must be string")?.to_string();
                if ![
                    "parakeet",
                    "parakeet-unified",
                    "parakeet-tdt",
                    "parakeet-rnnt",
                    "rnnt",
                    "whisper",
                    "whisper-cli",
                ]
                .contains(&s.as_str())
                {
                    return Err(format!("invalid asr_backend: {s}"));
                }
                staged.asr_backend = Some(s);
            }
            "dictation_language" => {
                // "auto" = Parakeet auto-detect; otherwise a short BCP-47-ish code
                // (2-3 letters, optional region) that locks Whisper to that language.
                // The Pro gate is enforced at the shell's config push, not here.
                let s = v
                    .as_str()
                    .ok_or("dictation_language must be string")?
                    .trim()
                    .to_lowercase();
                let valid = s == "auto"
                    || (s.len() >= 2
                        && s.len() <= 8
                        && s.chars().all(|c| c.is_ascii_alphabetic() || c == '-'));
                if !valid {
                    return Err(format!("invalid dictation_language: {s}"));
                }
                staged.dictation_language = Some(s);
            }
            "transcript_cleanup" => {
                let s = v
                    .as_str()
                    .ok_or("transcript_cleanup must be string")?
                    .to_string();
                if !["clean", "aggressive"].contains(&s.as_str()) {
                    return Err(format!("invalid transcript_cleanup: {s}"));
                }
                staged.transcript_cleanup = Some(s);
            }
            "ai_polish_enabled" => {
                staged.ai_polish_enabled = Some(bool_value(v, "ai_polish_enabled")?)
            }
            "ai_polish_on_device" => {
                staged.ai_polish_on_device = Some(bool_value(v, "ai_polish_on_device")?)
            }
            "ai_polish_mode" => {
                let s = v
                    .as_str()
                    .ok_or("ai_polish_mode must be string")?
                    .to_string();
                if s != "app" {
                    return Err(format!("invalid ai_polish_mode: {s}"));
                }
                staged.ai_polish_mode = Some(s);
            }
            "delivery_method" => {
                let s = v
                    .as_str()
                    .ok_or("delivery_method must be string")?
                    .to_string();
                if !["paste", "keystroke"].contains(&s.as_str()) {
                    return Err(format!("invalid delivery_method: {s}"));
                }
                staged.delivery_method = Some(s);
            }
            "screenshot_destination" => {
                let s = v
                    .as_str()
                    .ok_or("screenshot_destination must be string")?
                    .to_string();
                if !["clipboard", "disk", "both"].contains(&s.as_str()) {
                    return Err(format!("invalid screenshot_destination: {s}"));
                }
                staged.screenshot_destination = Some(s);
            }
            "screenshot_prompt_selected" => {
                staged.screenshot_prompt_selected =
                    Some(bool_value(v, "screenshot_prompt_selected")?)
            }
            "cancel_hint_seen" => {
                staged.cancel_hint_seen = Some(bool_value(v, "cancel_hint_seen")?)
            }
            "diagnostics_enabled" => {
                staged.diagnostics_enabled = Some(bool_value(v, "diagnostics_enabled")?)
            }
            "developer_mode" => staged.developer_mode = Some(bool_value(v, "developer_mode")?),
            "asr_compute_profile" => {
                let value = v.as_str().ok_or("asr_compute_profile must be string")?;
                if !["automatic", "neural_engine", "cpu_gpu", "cpu_only"].contains(&value) {
                    return Err(format!("invalid asr_compute_profile: {value}"));
                }
                staged.asr_compute_profile = Some(value.to_string());
            }
            "asr_encoder_compute" | "asr_decoder_compute" => {
                let value = v.as_str().ok_or_else(|| format!("{k} must be string"))?;
                if !["inherit", "neural_engine", "cpu_gpu", "cpu_only"].contains(&value) {
                    return Err(format!("invalid {k}: {value}"));
                }
                if k == "asr_encoder_compute" {
                    staged.asr_encoder_compute = Some(value.to_string());
                } else {
                    staged.asr_decoder_compute = Some(value.to_string());
                }
            }
            "asr_detailed_telemetry" => {
                staged.asr_detailed_telemetry = Some(bool_value(v, "asr_detailed_telemetry")?)
            }
            "diagnostic_audio_capture" => {
                staged.diagnostic_audio_capture = Some(bool_value(v, "diagnostic_audio_capture")?)
            }
            "diagnostic_unredacted_logs" => {
                staged.diagnostic_unredacted_logs =
                    Some(bool_value(v, "diagnostic_unredacted_logs")?)
            }
            "history_retention" => {
                staged.history_retention = Some(
                    serde_json::from_value::<HistoryRetention>(v.clone())
                        .map_err(|_| "invalid history_retention".to_string())?,
                )
            }
            "telemetry_usage" => staged.telemetry_usage = Some(bool_value(v, "telemetry_usage")?),
            "auto_update_install" => {
                staged.auto_update_install = Some(bool_value(v, "auto_update_install")?)
            }
            "snippets" => {
                let obj = v.as_object().ok_or("snippets must be object")?;
                let mut snippets = HashMap::new();
                for (key, value) in obj {
                    let replacement = value
                        .as_str()
                        .ok_or("snippet values must be strings")?
                        .to_string();
                    let key = key.trim().trim_start_matches('/').to_ascii_lowercase();
                    if key.is_empty() {
                        return Err("snippet key must not be empty".to_string());
                    }
                    if key.len() > 64 {
                        return Err(format!("snippet key too long: {key}"));
                    }
                    snippets.insert(key, replacement);
                }
                staged.snippets = Some(snippets);
            }
            "replacements" => {
                let obj = v.as_object().ok_or("replacements must be object")?;
                let mut replacements = HashMap::new();
                for (key, value) in obj {
                    let to = value
                        .as_str()
                        .ok_or("replacement values must be strings")?
                        .to_string();
                    let key = key.trim().to_string();
                    if key.is_empty() {
                        return Err("replacement key must not be empty".to_string());
                    }
                    if key.len() > 64 {
                        return Err(format!("replacement key too long: {key}"));
                    }
                    if to.len() > 256 {
                        return Err(format!("replacement value too long for: {key}"));
                    }
                    replacements.insert(key, to);
                }
                staged.replacements = Some(replacements);
            }
            "push_to_talk" => staged.push_to_talk = Some(bool_value(v, "push_to_talk")?),
            "push_to_talk_key" => {
                let s = v
                    .as_str()
                    .ok_or("push_to_talk_key must be string")?
                    .trim()
                    .to_ascii_lowercase();
                // Locked set for now. Adding more keys (e.g. scroll_lock,
                // grave) is an additive change — extend this list and the
                // VK resolution in recording_cancel_hook together.
                if !["ralt", "rctrl"].contains(&s.as_str()) {
                    return Err(format!("invalid push_to_talk_key: {s}"));
                }
                staged.push_to_talk_key = Some(s);
            }
            "middle_mouse_recording" => {
                staged.middle_mouse_recording = Some(bool_value(v, "middle_mouse_recording")?)
            }
            "onboarding_complete" => {
                staged.onboarding_complete = Some(bool_value(v, "onboarding_complete")?)
            }
            "onboarding_version" => {
                let version = v.as_u64().ok_or("onboarding_version must be a number")?;
                staged.onboarding_version = Some(version.min(u32::MAX as u64) as u32);
            }
            "legal_acceptance_version" => {
                let version = v
                    .as_str()
                    .ok_or("legal_acceptance_version must be string")?
                    .trim();
                if version != required_legal_acceptance_version() {
                    return Err("legal_acceptance_version is not current".to_string());
                }
                staged.legal_acceptance_version = Some(version.to_string());
            }
            "legal_accepted_at" => {
                let accepted_at = v.as_str().ok_or("legal_accepted_at must be string")?.trim();
                if !legal_accepted_at_is_canonical_utc(accepted_at) {
                    return Err(
                        "legal_accepted_at must be a canonical UTC RFC3339 timestamp".to_string(),
                    );
                }
                staged.legal_accepted_at = Some(accepted_at.to_string());
            }
            "legal_eligibility_basis" => {
                let basis = v
                    .as_str()
                    .ok_or("legal_eligibility_basis must be string")?
                    .trim();
                if !["individual", "enterprise"].contains(&basis) {
                    return Err("legal_eligibility_basis is invalid".to_string());
                }
                staged.legal_eligibility_basis = Some(basis.to_string());
            }
            "legal_material_sha256" => {
                let hash = v
                    .as_str()
                    .ok_or("legal_material_sha256 must be string")?
                    .trim()
                    .to_ascii_lowercase();
                if hash != required_legal_material_sha256() {
                    return Err("legal_material_sha256 is not current".to_string());
                }
                staged.legal_material_sha256 = Some(hash);
            }
            other => {
                return Err(format!("settings key not in allowlist: {other}"));
            }
        }
    }
    *blob = staged;
    Ok(())
}
