#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_safe() {
        assert_eq!(DEFAULT_CLOSE_BEHAVIOR, "destroy");
        assert_eq!(DEFAULT_APP_THEME, "ember");
        assert_eq!(DEFAULT_APP_FONT, "hanken");
        assert_eq!(DEFAULT_ASR_BACKEND, "parakeet-tdt");
        assert_eq!(DEFAULT_PILL_VISIBILITY, "always");
        assert_eq!(DEFAULT_TRANSCRIPT_CLEANUP, "aggressive");
        assert!(!DEFAULT_AI_POLISH_ENABLED);
        assert_eq!(DEFAULT_AI_POLISH_MODE, "app");
        assert!(DEFAULT_DIAGNOSTICS_ENABLED);
        assert_eq!(DEFAULT_HISTORY_RETENTION, HistoryRetention::Forever);
        assert!(!DEFAULT_TELEMETRY_USAGE);
    }

    #[test]
    fn patch_rejects_unknown_key() {
        let mut blob = SettingsBlob::default();
        assert!(apply_patch_to(&mut blob, &serde_json::json!({ "evil": "yes" })).is_err());
    }

    #[test]
    fn patch_rejects_out_of_range_volume() {
        let mut blob = SettingsBlob::default();
        assert!(apply_patch_to(
            &mut blob,
            &serde_json::json!({ "sound_effects_volume": 9999 })
        )
        .is_err());
    }

    #[test]
    fn patch_rejects_invalid_enum_values() {
        let mut blob = SettingsBlob::default();
        assert!(
            apply_patch_to(&mut blob, &serde_json::json!({ "close_behavior": "trash" })).is_err()
        );
        assert!(apply_patch_to(&mut blob, &serde_json::json!({ "app_theme": "quantum" })).is_err());
        assert!(
            apply_patch_to(&mut blob, &serde_json::json!({ "asr_backend": "wav2vec" })).is_err()
        );
        apply_patch_to(
            &mut blob,
            &serde_json::json!({ "asr_backend": "parakeet-unified" }),
        )
        .unwrap();
        assert_eq!(blob.asr_backend.as_deref(), Some("parakeet-unified"));
        apply_patch_to(&mut blob, &serde_json::json!({ "asr_backend": "whisper" })).unwrap();
        assert_eq!(blob.asr_backend.as_deref(), Some("whisper"));
        apply_patch_to(
            &mut blob,
            &serde_json::json!({ "asr_backend": "whisper-cli" }),
        )
        .unwrap();
        assert_eq!(blob.asr_backend.as_deref(), Some("whisper-cli"));
        assert!(
            apply_patch_to(&mut blob, &serde_json::json!({ "asr_backend": "vulkan" })).is_err()
        );
        assert!(apply_patch_to(
            &mut blob,
            &serde_json::json!({ "transcript_cleanup": "rewrite" })
        )
        .is_err());
        assert!(apply_patch_to(
            &mut blob,
            &serde_json::json!({ "transcript_cleanup": "minimal" })
        )
        .is_err());
        assert!(apply_patch_to(
            &mut blob,
            &serde_json::json!({ "transcript_cleanup": "maximal" })
        )
        .is_err());
        assert!(apply_patch_to(
            &mut blob,
            &serde_json::json!({ "ai_polish_mode": "summarize" })
        )
        .is_err());
        assert!(apply_patch_to(
            &mut blob,
            &serde_json::json!({ "ai_polish_mode": "simple" })
        )
        .is_err());
        assert!(apply_patch_to(
            &mut blob,
            &serde_json::json!({ "ai_polish_mode": "prompt" })
        )
        .is_err());
        assert!(apply_patch_to(
            &mut blob,
            &serde_json::json!({ "pill_visibility": "forever" })
        )
        .is_err());
        assert!(apply_patch_to(
            &mut blob,
            &serde_json::json!({ "history_retention": "two_weeks" })
        )
        .is_err());
    }

    #[test]
    fn patch_rejects_wrong_typed_booleans() {
        let mut blob = SettingsBlob::default();
        let r = apply_patch_to(
            &mut blob,
            &serde_json::json!({ "diagnostics_enabled": "true", "telemetry_usage": 1 }),
        );
        assert!(r.is_err());
        assert_eq!(blob.diagnostics_enabled, None);
        assert_eq!(blob.telemetry_usage, None);
    }

    #[test]
    fn patch_applies_valid_values() {
        let mut blob = SettingsBlob::default();
        apply_patch_to(
            &mut blob,
            &serde_json::json!({
                "diagnostics_enabled": false,
                "history_retention": "30_days",
                "sound_effects_volume": 80,
                "asr_backend": "parakeet-tdt",
                "transcript_cleanup": "aggressive",
                "ai_polish_enabled": true,
                "ai_polish_mode": "app",
                "app_theme": "ember-light",
                "app_font": "hanken",
                "pill_visibility": "during_dictation",
                "snippets": { "/email": "adrian@example.com" }
            }),
        )
        .unwrap();
        assert_eq!(blob.diagnostics_enabled, Some(false));
        assert_eq!(blob.history_retention, Some(HistoryRetention::ThirtyDays));
        assert_eq!(blob.sound_effects_volume, Some(80));
        assert_eq!(blob.asr_backend.as_deref(), Some("parakeet-tdt"));
        assert_eq!(blob.transcript_cleanup.as_deref(), Some("aggressive"));
        assert_eq!(blob.ai_polish_enabled, Some(true));
        assert_eq!(blob.ai_polish_mode.as_deref(), Some("app"));
        assert_eq!(blob.app_theme.as_deref(), Some("ember-light"));
        assert_eq!(blob.app_font.as_deref(), Some("hanken"));
        assert_eq!(blob.pill_visibility.as_deref(), Some("during_dictation"));
        assert_eq!(
            blob.snippets
                .as_ref()
                .and_then(|snippets| snippets.get("email"))
                .map(String::as_str),
            Some("adrian@example.com")
        );
    }

    #[test]
    fn patch_rejects_when_a_later_key_is_invalid_after_earlier_applied() {
        // All-or-nothing: the valid key must NOT be committed when a later
        // key fails validation.
        let mut blob = SettingsBlob::default();
        let r = apply_patch_to(
            &mut blob,
            &serde_json::json!({ "diagnostics_enabled": false, "recording_style": "mini" }),
        );
        assert!(r.is_err());
        assert_eq!(blob.diagnostics_enabled, None);
    }

    #[test]
    fn migrate_stamps_unversioned_blob_and_preserves_fields() {
        // A v0 (pre-versioning) blob with real user data must keep that data and
        // gain the current version — NOT get reset.
        let mut blob = SettingsBlob {
            diagnostics_enabled: Some(false),
            app_theme: Some("ink-copper".to_string()),
            schema_version: None,
            ..Default::default()
        };
        let changed = migrate(&mut blob);
        assert!(changed);
        assert_eq!(blob.schema_version, Some(CURRENT_SCHEMA_VERSION));
        assert_eq!(blob.diagnostics_enabled, Some(false));
        assert_eq!(blob.app_theme.as_deref(), Some("ink-copper"));
    }

    #[test]
    fn migrate_drops_retired_noop_settings_but_keeps_future_extras() {
        let json = serde_json::json!({
            "schema_version": 1,
            "privacy_mode": true,
            "recording_style": "classic",
            "launch_on_login": true,
            "telemetry_wake_samples": true,
            "future_key_xyz": 42
        });
        let mut blob: SettingsBlob = serde_json::from_value(json).unwrap();

        assert!(migrate(&mut blob));
        assert_eq!(blob.schema_version, Some(CURRENT_SCHEMA_VERSION));
        assert_eq!(blob.diagnostics_enabled, Some(false));
        assert!(!blob.extras.contains_key("privacy_mode"));
        assert_eq!(
            blob.extras.get("future_key_xyz"),
            Some(&serde_json::json!(42))
        );
        assert!(!blob.extras.contains_key("recording_style"));
        assert!(!blob.extras.contains_key("launch_on_login"));
        assert!(!blob.extras.contains_key("telemetry_wake_samples"));
    }

    #[test]
    fn migrate_canonicalizes_legacy_parakeet_alias_to_current_default() {
        let mut blob = SettingsBlob {
            schema_version: Some(2),
            asr_backend: Some("parakeet".to_string()),
            ..Default::default()
        };

        assert!(migrate(&mut blob));
        assert_eq!(blob.schema_version, Some(CURRENT_SCHEMA_VERSION));
        assert_eq!(blob.asr_backend.as_deref(), Some(DEFAULT_ASR_BACKEND));
    }

    #[test]
    fn migrate_retires_explicit_rnnt_choice() {
        let mut blob = SettingsBlob {
            schema_version: Some(4),
            asr_backend: Some("parakeet-rnnt".to_string()),
            ..Default::default()
        };

        assert!(migrate(&mut blob));
        assert_eq!(blob.schema_version, Some(CURRENT_SCHEMA_VERSION));
        assert_eq!(blob.asr_backend.as_deref(), Some(DEFAULT_ASR_BACKEND));
    }

    #[test]
    fn migrate_v6_to_v7_moves_tdt_default_to_unified() {
        let mut blob = SettingsBlob {
            schema_version: Some(6),
            asr_backend: Some("parakeet-tdt".to_string()),
            ..Default::default()
        };

        assert!(migrate(&mut blob));
        assert_eq!(blob.schema_version, Some(CURRENT_SCHEMA_VERSION));
        assert_eq!(blob.asr_backend.as_deref(), Some(DEFAULT_ASR_BACKEND));
    }

    #[test]
    fn migrate_v6_to_v7_preserves_whisper_choice() {
        let mut blob = SettingsBlob {
            schema_version: Some(6),
            asr_backend: Some("whisper".to_string()),
            ..Default::default()
        };

        assert!(migrate(&mut blob));
        assert_eq!(blob.schema_version, Some(CURRENT_SCHEMA_VERSION));
        assert_eq!(blob.asr_backend.as_deref(), Some("whisper"));
    }

    #[test]
    fn migrate_v7_to_v8_retires_prompt_as_persistent_ai_polish_mode() {
        let mut blob = SettingsBlob {
            schema_version: Some(7),
            ai_polish_mode: Some("prompt".to_string()),
            ..Default::default()
        };

        assert!(migrate(&mut blob));
        assert_eq!(blob.schema_version, Some(CURRENT_SCHEMA_VERSION));
        assert_eq!(blob.ai_polish_mode.as_deref(), Some(DEFAULT_AI_POLISH_MODE));
    }

    include!("section03_more.rs");
}
