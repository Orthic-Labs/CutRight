//! `heardright_core` — pure, GUI-free logic for the Heard Right app.
//!
//! Anything in here must stay free of `tauri` / GUI / OS-UI dependencies so it
//! unit-tests in a plain console harness. (The app crate links Common Controls
//! v6 through tauri — `comctl32!TaskDialogIndirect` — which the loader can't
//! resolve in a bare `cargo test` binary, so the app crate's own tests don't
//! launch. Test pure logic here.)

#![allow(
    clippy::result_large_err,
    clippy::large_enum_variant,
    clippy::too_many_arguments,
    clippy::doc_lazy_continuation,
    clippy::manual_clamp
)] // intentional: large variants live on cold error paths, a few arg-heavy platform fns, doc-comment style

pub mod audio_conditioning;
pub mod command;
pub mod command_recognition;
pub mod controller;
pub mod credential;
pub mod delivery;
pub mod engine;
pub mod external_url;
pub mod history;
pub mod mic_selection;
pub mod pill;
pub mod settings;
pub mod state;
pub mod text_pipeline;
pub mod vocab_learn;
pub mod vocabulary;
pub mod wake;

use serde_json::Value;

const DIAGNOSTIC_REDACTION_MARKER: &str = "[redacted:diagnostics]";

/// Remove user content, target-window metadata, and secret-shaped values from
/// an ordinary local diagnostic event before it reaches disk or stderr.
///
/// This is unconditional: privacy is HeardRight's baseline, not an optional
/// mode. AI-payload logging is a separate, explicit opt-in path and does not
/// call this function.
pub fn redact_diagnostic_event(mut value: Value) -> Value {
    redact_diagnostic_value(&mut value);
    value
}

/// Redact provider credentials from locally retained AI payloads while keeping
/// ordinary transcript fields readable. This stays separate from ordinary
/// diagnostics, whose content fields are always blanket-redacted.
pub fn redact_payload_value(value: Value, max_text_chars: usize) -> Value {
    match value {
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| {
                    let value = if payload_secret_key(&key) {
                        Value::String("[redacted]".into())
                    } else {
                        redact_payload_value(value, max_text_chars)
                    };
                    (key, value)
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|value| redact_payload_value(value, max_text_chars))
                .collect(),
        ),
        Value::String(value) => Value::String(redact_payload_text(&value, max_text_chars)),
        value => value,
    }
}

/// Redact secret-shaped tokens in either plain provider output or JSON text.
pub fn redact_payload_text(value: &str, max_text_chars: usize) -> String {
    let value: String = value.chars().take(max_text_chars).collect();
    if let Ok(parsed) = serde_json::from_str::<Value>(&value) {
        return serde_json::to_string(&redact_payload_value(parsed, max_text_chars))
            .unwrap_or_else(|_| redact_payload_secret_tokens(&value));
    }
    redact_payload_secret_tokens(&value)
}

fn payload_secret_key(key: &str) -> bool {
    [
        "authorization",
        "api_key",
        "apikey",
        "access_token",
        "refresh_token",
        "id_token",
        "client_secret",
        "password",
        "password_hash",
        "credential",
    ]
    .contains(&key.to_ascii_lowercase().as_str())
}

fn redact_payload_secret_tokens(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0;
    let mut redact_next = false;
    let bytes = value.as_bytes();
    while cursor < bytes.len() {
        if !is_payload_secret_token_char(bytes[cursor]) {
            let Some(character) = value[cursor..].chars().next() else {
                break;
            };
            output.push(character);
            cursor += character.len_utf8();
            continue;
        }
        let start = cursor;
        while cursor < bytes.len() && is_payload_secret_token_char(bytes[cursor]) {
            cursor += 1;
        }
        let token = &value[start..cursor];
        if redact_next || looks_like_payload_secret(token) {
            output.push_str("[redacted]");
        } else {
            output.push_str(token);
        }
        redact_next = token.eq_ignore_ascii_case("bearer");
    }
    output
}

fn is_payload_secret_token_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
}

fn looks_like_payload_secret(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let prefixes = [
        "sk-",
        "gsk_",
        "ghp_",
        "github_pat_",
        "xoxb-",
        "xoxp-",
        "xoxa-",
        "xoxr-",
        "xoxs-",
        // `AIza` is case-sensitive in issued keys; compare lowercase after
        // normalizing so mixed-case copies cannot bypass redaction.
        "aiza",
    ];
    if value.len() >= 16 && prefixes.iter().any(|prefix| lower.starts_with(prefix)) {
        return true;
    }
    if value.len() == 20
        && value.starts_with("AKIA")
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return true;
    }
    let mut parts = value.split('.');
    let Some(header) = parts.next() else {
        return false;
    };
    let Some(payload) = parts.next() else {
        return false;
    };
    let Some(signature) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && header.starts_with("eyJ")
        && header.len() >= 8
        && payload.len() >= 8
        && signature.len() >= 8
}

fn redact_diagnostic_value(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if key.eq_ignore_ascii_case("message") {
                    // `message` is the operational payload of every local
                    // telemetry event (`key=value` diagnostics, error strings).
                    // Blanket redaction makes the entire local log useless, so
                    // scrub it token-wise instead — same privacy posture as the
                    // sidecar stderr scrubber. Emitters must never place raw
                    // user content in `message`; content belongs under the
                    // blanket-redacted keys below.
                    if let Value::String(text) = child {
                        *text = scrub_diagnostic_message(text);
                    } else {
                        redact_diagnostic_value(child);
                    }
                } else if sensitive_diagnostic_key(key) {
                    *child = Value::String(DIAGNOSTIC_REDACTION_MARKER.to_string());
                } else {
                    redact_diagnostic_value(child);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_diagnostic_value(item);
            }
        }
        Value::String(text) if secret_shaped(text) => {
            *text = DIAGNOSTIC_REDACTION_MARKER.to_string();
        }
        _ => {}
    }
}

fn sensitive_diagnostic_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    matches!(
        key.as_str(),
        "authorization"
            | "body"
            | "completion"
            | "content"
            | "cookie"
            | "last_transcript"
            | "phrase"
            | "process_name"
            | "query"
            | "raw_text"
            | "recognized_text"
            | "selected_text"
            | "target"
            | "text"
            | "transcript"
            | "window"
            | "window_title"
    ) || [
        "api_key",
        "clipboard",
        "password",
        "prompt",
        "refresh_token",
        "secret",
        "token",
        "transcript",
        "window",
    ]
    .iter()
    .any(|fragment| key.contains(fragment))
}

/// Token-wise scrub for the operational `message` field: keep `key=value`
/// diagnostics and error text, redact tokens that name or carry sensitive
/// material. Mirrors `sidecar_log::redact_log_token` in the app crate.
fn scrub_diagnostic_message(text: &str) -> String {
    const SENSITIVE_MESSAGE_FRAGMENTS: &[&str] = &[
        "authorization",
        "bearer",
        "api_key",
        "api-key",
        "apikey",
        "token",
        "transcript",
        "prompt",
        "content",
        "password",
        "secret",
        "clipboard",
        "phrase",
        "window_title",
        "title=",
        "text=",
    ];
    let mut scrubbed = String::with_capacity(text.len());
    let mut redact_next = false;
    for token in text.split_inclusive(char::is_whitespace) {
        let trimmed_len = token.trim_end_matches(char::is_whitespace).len();
        let (body, suffix) = token.split_at(trimmed_len);
        let lower = body.to_ascii_lowercase();
        let sensitive = SENSITIVE_MESSAGE_FRAGMENTS
            .iter()
            .any(|fragment| lower.contains(fragment))
            || secret_shaped(body);
        if redact_next || sensitive {
            scrubbed.push_str(DIAGNOSTIC_REDACTION_MARKER);
        } else {
            scrubbed.push_str(body);
        }
        scrubbed.push_str(suffix);
        redact_next = lower == "bearer" || lower.ends_with("=bearer") || lower.ends_with(":bearer");
    }
    scrubbed
}

fn secret_shaped(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    if lower.contains("bearer ")
        || lower.contains("authorization=")
        || lower.contains("api_key=")
        || lower.contains("api-key=")
        || lower.contains("access_token=")
        || lower.contains("refresh_token=")
        || lower.contains("secret=")
    {
        return true;
    }

    value
        .split(|character: char| {
            character.is_whitespace()
                || matches!(
                    character,
                    '"' | '\'' | ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}'
                )
        })
        .any(|token| {
            let token = token.trim_matches(|character: char| matches!(character, ':' | '='));
            let lower = token.to_ascii_lowercase();
            (token.len() >= 12
                && ["sk-", "gsk_", "csk_", "rk_", "xoxb-", "xoxp-"]
                    .iter()
                    .any(|prefix| lower.starts_with(prefix)))
                || (token.len() >= 24
                    && token.starts_with("eyJ")
                    && token.bytes().filter(|byte| *byte == b'.').count() >= 2)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_redaction_covers_content_and_target_fields_recursively() {
        let v = serde_json::json!({
            "state": "pasted",
            "delivery_id": "d1",
            "transcript": "secret spoken words",
            "message": "requested=hide resolved=hidden visible_before=true",
            "target": {
                "process_name": "PrivateApp",
                "window_title": "Confidential document"
            },
            "nested": {
                "selected_text": "private selection",
                "prompt": "private prompt"
            }
        });
        let r = redact_diagnostic_event(v);
        assert_eq!(r["transcript"], "[redacted:diagnostics]");
        // Operational message payloads survive so local logs stay debuggable.
        assert_eq!(
            r["message"],
            "requested=hide resolved=hidden visible_before=true"
        );
        assert_eq!(r["target"], "[redacted:diagnostics]");
        assert_eq!(r["nested"]["selected_text"], "[redacted:diagnostics]");
        assert_eq!(r["nested"]["prompt"], "[redacted:diagnostics]");
        assert_eq!(r["state"], "pasted");
        assert_eq!(r["delivery_id"], "d1");
    }

    #[test]
    fn diagnostic_redaction_handles_null_sensitive_fields_without_panic() {
        let v = serde_json::json!({
            "state": "error",
            "last_transcript": null
        });
        let r = redact_diagnostic_event(v);
        assert_eq!(r["last_transcript"], "[redacted:diagnostics]");
        assert_eq!(r["state"], "error");
    }

    #[test]
    fn diagnostic_redaction_removes_secret_shaped_values_under_unknown_keys() {
        let v = serde_json::json!({
            "event": "provider_failure",
            "detail": "request failed with gsk_live_1234567890abcdef",
            "context": ["safe status", "Bearer eyJhbGciOiJIUzI1NiJ9.payload.signature"]
        });
        let r = redact_diagnostic_event(v);
        assert_eq!(r["detail"], "[redacted:diagnostics]");
        assert_eq!(r["context"][0], "safe status");
        assert_eq!(r["context"][1], "[redacted:diagnostics]");
        assert_eq!(r["event"], "provider_failure");
    }

    #[test]
    fn diagnostic_message_scrub_keeps_operational_tokens_and_redacts_sensitive_ones() {
        let v = serde_json::json!({
            "event": "pill_compositor_recompose_result",
            "message": "reason=recording_start phase=deferred active=true show_ok=true visible_after=true",
        });
        let r = redact_diagnostic_event(v);
        assert_eq!(
            r["message"],
            "reason=recording_start phase=deferred active=true show_ok=true visible_after=true"
        );

        let v = serde_json::json!({
            "event": "provider_failure",
            "message": "retry=2 api_key=sk-live-123456789012 transcript=private Bearer eyJhbGciOiJIUzI1NiJ9.p.s done",
        });
        let r = redact_diagnostic_event(v);
        let message = r["message"].as_str().unwrap();
        assert!(message.contains("retry=2"));
        assert!(message.contains("done"));
        assert!(!message.contains("sk-live-123456789012"));
        assert!(!message.contains("transcript=private"));
        assert!(!message.contains("eyJhbGciOiJIUzI1NiJ9"));
    }

    #[test]
    fn payload_redaction_covers_bearer_jwt_aiza_and_common_secret_values() {
        let value = redact_payload_value(
            serde_json::json!({
                "transcript": "Keep this dictated sentence intact.",
                "unknown": "Bearer arbitrary-opaque-secret-value",
                "jwt": "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3OCJ9.signature-value-123456",
                "google": "AIzaSyDUMMY123456789012345678901234",
                "provider": "gsk_common-api-key-value-123456",
                "pin": "4821",
                "token_count": 37,
                "secret_message": "dictated secret recipe",
                "path": "/Users/adrian/support.json",
            }),
            16 * 1024,
        );
        assert_eq!(value["transcript"], "Keep this dictated sentence intact.");
        for key in ["unknown", "jwt", "google", "provider"] {
            assert!(value[key].as_str().unwrap().contains("[redacted]"));
        }
        assert_eq!(value["pin"], "4821");
        assert_eq!(value["token_count"], 37);
        assert_eq!(value["secret_message"], "dictated secret recipe");
        assert_eq!(value["path"], "/Users/adrian/support.json");
    }
}
