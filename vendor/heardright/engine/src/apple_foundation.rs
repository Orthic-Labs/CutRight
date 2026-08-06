use serde::Deserialize;
use serde_json::json;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

// The env lock lives in crate::test_support (always-compiled) so Windows test
// code can share the SAME lock — this module is macOS-gated.
#[cfg(test)]
pub(crate) use crate::test_support::test_env_guard;

#[derive(Debug, PartialEq, Eq)]
pub enum ApplePolishOutcome {
    Cleaned(String),
    Unavailable(&'static str),
    Failed(&'static str),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ApplePolishContext {
    pub app_name: Option<String>,
    pub window_title: Option<String>,
}

#[derive(Deserialize)]
struct HelperResponse {
    ok: bool,
    text: Option<String>,
    #[serde(rename = "unavailableReason")]
    unavailable_reason: Option<String>,
    error: Option<String>,
}

pub fn polish(input: &str, context: &ApplePolishContext) -> ApplePolishOutcome {
    let input = input.trim();
    if input.is_empty() {
        return ApplePolishOutcome::Unavailable("empty_input");
    }
    let Some(helper) = resolve_helper_bin() else {
        return ApplePolishOutcome::Unavailable("helper_missing");
    };
    let payload = json!({
        "input": input,
        "appName": context.app_name,
        "windowTitle": context.window_title,
        "mode": "app"
    })
    .to_string();
    let timeout = helper_timeout(input.chars().count());

    let mut child = match Command::new(&helper)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return ApplePolishOutcome::Unavailable("spawn_failed"),
    };

    if child
        .stdin
        .take()
        .and_then(|mut stdin| stdin.write_all(payload.as_bytes()).ok())
        .is_none()
    {
        let _ = child.kill();
        let _ = child.wait();
        return ApplePolishOutcome::Failed("stdin");
    }

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return ApplePolishOutcome::Failed("timeout");
            }
            Err(_) => return ApplePolishOutcome::Failed("wait"),
        }
    }

    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(_) => return ApplePolishOutcome::Failed("output"),
    };
    let raw = match String::from_utf8(output.stdout) {
        Ok(raw) => raw,
        Err(_) => return ApplePolishOutcome::Failed("bad_utf8"),
    };
    let response: HelperResponse = match serde_json::from_str(raw.trim()) {
        Ok(response) => response,
        Err(_) => return ApplePolishOutcome::Failed("bad_json"),
    };

    if response.ok {
        return response
            .text
            .as_deref()
            .and_then(|text| normalize_output(input, text))
            .map(ApplePolishOutcome::Cleaned)
            .unwrap_or(ApplePolishOutcome::Failed("rejected_output"));
    }
    if response.error.is_some() {
        return ApplePolishOutcome::Failed("helper_error");
    }
    ApplePolishOutcome::Unavailable(unavailable_reason(response.unavailable_reason.as_deref()))
}

fn resolve_helper_bin() -> Option<PathBuf> {
    #[cfg(any(debug_assertions, test))]
    if let Some(path) = std::env::var_os("HEARDRIGHT_APPLE_FOUNDATION_BIN") {
        return Some(PathBuf::from(path));
    }
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let sibling = dir.join("heardright-apple-foundation-polish");
    if sibling.exists() {
        return Some(sibling);
    }
    #[cfg(target_os = "macos")]
    if let Some(contents_dir) = dir.parent() {
        let resource = contents_dir
            .join("Resources")
            .join("heardright-apple-foundation-polish");
        if resource.exists() {
            return Some(resource);
        }
    }
    None
}

fn helper_timeout(input_chars: usize) -> Duration {
    // Env override wins (tests, tuning). Otherwise scale with input length:
    // on-device generation time grows with tokens, so a fixed cap that fits a
    // one-liner strangles a 60s dictation. Measured M1 floor ~3.7s for a short
    // input; ~15ms/char covers generation growth with slack.
    let ms = std::env::var("HEARDRIGHT_APPLE_FOUNDATION_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or_else(|| 6_000 + (input_chars as u64) * 15)
        .clamp(250, 30_000);
    Duration::from_millis(ms)
}

fn normalize_output(input: &str, output: &str) -> Option<String> {
    let trimmed = output.trim();
    if trimmed.is_empty() || trimmed.starts_with("```") {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("transcript:") || lower.contains("the user wants") {
        return None;
    }
    let trimmed = if lower.starts_with("polished output:") {
        trimmed["polished output:".len()..].trim()
    } else {
        trimmed
    };
    if trimmed.is_empty() || trimmed.chars().count() > input.chars().count().saturating_add(1_200) {
        return None;
    }
    Some(trimmed.to_string())
}

fn unavailable_reason(reason: Option<&str>) -> &'static str {
    match reason {
        Some("empty_input") => "empty_input",
        Some("model_unavailable") => "model_unavailable",
        Some("empty_output") => "empty_output",
        Some("unsupported_os_or_sdk") => "unsupported_os_or_sdk",
        _ => "unavailable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn with_fake_helper(script: &str, test: impl FnOnce()) {
        let _guard = test_env_guard();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "heardright-apple-foundation-test-{}-{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&dir).unwrap();
        let helper = dir.join("helper.sh");
        fs::write(&helper, script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&helper).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&helper, perms).unwrap();
        }
        let old_bin = std::env::var("HEARDRIGHT_APPLE_FOUNDATION_BIN").ok();
        let old_timeout = std::env::var("HEARDRIGHT_APPLE_FOUNDATION_TIMEOUT_MS").ok();
        std::env::set_var("HEARDRIGHT_APPLE_FOUNDATION_BIN", &helper);
        // Process startup can exceed 300 ms on a busy release machine even for
        // this tiny shell helper. Keep the fake-helper tests comfortably above
        // startup jitter while remaining far below the production timeout.
        std::env::set_var("HEARDRIGHT_APPLE_FOUNDATION_TIMEOUT_MS", "1000");
        test();
        if let Some(value) = old_bin {
            std::env::set_var("HEARDRIGHT_APPLE_FOUNDATION_BIN", value);
        } else {
            std::env::remove_var("HEARDRIGHT_APPLE_FOUNDATION_BIN");
        }
        if let Some(value) = old_timeout {
            std::env::set_var("HEARDRIGHT_APPLE_FOUNDATION_TIMEOUT_MS", value);
        } else {
            std::env::remove_var("HEARDRIGHT_APPLE_FOUNDATION_TIMEOUT_MS");
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn accepts_clean_helper_output() {
        with_fake_helper(
            r#"#!/bin/sh
cat >/dev/null
printf '{"ok":true,"text":"Send Adrian the invoice tomorrow."}\n'
"#,
            || {
                assert_eq!(
                    polish(
                        "um send adrian the invoice tomorrow",
                        &ApplePolishContext::default()
                    ),
                    ApplePolishOutcome::Cleaned("Send Adrian the invoice tomorrow.".to_string())
                );
            },
        );
    }

    #[test]
    fn unavailable_helper_output_is_not_failure() {
        with_fake_helper(
            r#"#!/bin/sh
cat >/dev/null
printf '{"ok":false,"unavailableReason":"model_unavailable"}\n'
"#,
            || {
                assert_eq!(
                    polish("hello world", &ApplePolishContext::default()),
                    ApplePolishOutcome::Unavailable("model_unavailable")
                );
            },
        );
    }

    #[test]
    fn rejects_wrapped_or_oversized_helper_output() {
        with_fake_helper(
            r#"#!/bin/sh
cat >/dev/null
printf '{"ok":true,"text":"Transcript: hello"}\n'
"#,
            || {
                assert_eq!(
                    polish("hello", &ApplePolishContext::default()),
                    ApplePolishOutcome::Failed("rejected_output")
                );
            },
        );
    }

    #[test]
    fn bad_helper_json_fails_closed() {
        with_fake_helper(
            r#"#!/bin/sh
cat >/dev/null
printf 'not-json\n'
"#,
            || {
                assert_eq!(
                    polish("hello world", &ApplePolishContext::default()),
                    ApplePolishOutcome::Failed("bad_json")
                );
            },
        );
    }

    #[test]
    fn helper_timeout_fails_closed() {
        with_fake_helper(
            r#"#!/bin/sh
cat >/dev/null
sleep 2
"#,
            || {
                assert_eq!(
                    polish("hello world", &ApplePolishContext::default()),
                    ApplePolishOutcome::Failed("timeout")
                );
            },
        );
    }
}
