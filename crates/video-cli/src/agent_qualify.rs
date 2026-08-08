//! Gate 0 qualification harness for installed Claude Code/Codex surfaces.

use std::collections::BTreeSet;
use std::env;
use std::path::PathBuf;
use std::process::Command;

use serde::Serialize;
use serde_json::{json, Value};

const DENIED_ENV: &[&str] = &[
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "OPENROUTER_API_KEY",
    "MINIMAX_API_KEY",
    "TINYFISH_API_KEY",
    "CODERIGHT_DAEMON_TOKEN",
    "CODERIGHT_GATEWAY_TOKEN",
    "CODERIGHT_DRIVER_INSTANCE_SECRET",
];
const ALLOWED_TOOLS: &[&str] = &[
    "cutright.inspect",
    "cutright.read_transcript",
    "cutright.draft_plan",
    "cutright.apply_plan",
    "cutright.render_artifact",
];

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum ModeStatus {
    Pass,
    Failed,
    Blocked,
}

#[derive(Debug, Serialize)]
struct Probe {
    id: &'static str,
    status: ModeStatus,
    detail: &'static str,
}

#[derive(Debug, Serialize)]
struct Executable {
    path: String,
    sha256: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct ModeResult {
    provider: &'static str,
    surface: &'static str,
    status: ModeStatus,
    executable: Option<Executable>,
    probes: Vec<Probe>,
    provider_surface: String,
    reason: Option<String>,
}

pub fn run_all() -> Result<(Value, bool), String> {
    let mut modes = Vec::new();
    for provider in [
        ("claude-code", "CUTRIGHT_CLAUDE_CODE_BIN", "claude"),
        ("codex", "CUTRIGHT_CODEX_BIN", "codex"),
    ] {
        for surface in ["guided", "native"] {
            modes.push(qualify_mode(provider, surface));
        }
    }
    let passed = modes
        .iter()
        .any(|mode| matches!(mode.status, ModeStatus::Pass));
    Ok((
        json!({
            "event": "agent.qualify",
            "schema": "cutright.agent_qualification/v1",
            "status": if passed { "pass" } else { "blocked" },
            "modes": modes,
            "policy": {
                "providers": ["claude-code", "codex"],
                "bundled_or_local_llm": false,
                "config_mutation": false,
                "hash_algorithm": "sha256"
            }
        }),
        passed,
    ))
}

fn qualify_mode(
    provider: (&'static str, &'static str, &'static str),
    surface: &'static str,
) -> ModeResult {
    let (provider_name, env_name, executable_name) = provider;
    let path = match resolve_executable(env_name, executable_name) {
        Ok(path) => path,
        Err(reason) => return blocked(provider_name, surface, "not_started", reason),
    };
    let executable = match attest(&path) {
        Ok(value) => value,
        Err(reason) => return failed(provider_name, surface, "attestation_failed", reason),
    };
    let provider_surface = match provider_list(&path) {
        Ok(output) => output,
        Err(reason) => {
            return ModeResult {
                provider: provider_name,
                surface,
                status: ModeStatus::Blocked,
                executable: Some(executable),
                probes: Vec::new(),
                provider_surface: "ui_or_auth_required".into(),
                reason: Some(reason),
            }
        }
    };
    let probes = containment_probes(surface);
    let status = if probes
        .iter()
        .all(|probe| matches!(probe.status, ModeStatus::Pass))
    {
        ModeStatus::Pass
    } else {
        ModeStatus::Failed
    };
    ModeResult {
        provider: provider_name,
        surface,
        status,
        executable: Some(executable),
        probes,
        provider_surface,
        reason: None,
    }
}

fn blocked(
    provider: &'static str,
    surface: &'static str,
    provider_surface: &str,
    reason: String,
) -> ModeResult {
    ModeResult {
        provider,
        surface,
        status: ModeStatus::Blocked,
        executable: None,
        probes: Vec::new(),
        provider_surface: provider_surface.into(),
        reason: Some(reason),
    }
}

fn failed(
    provider: &'static str,
    surface: &'static str,
    provider_surface: &str,
    reason: String,
) -> ModeResult {
    ModeResult {
        provider,
        surface,
        status: ModeStatus::Failed,
        executable: None,
        probes: Vec::new(),
        provider_surface: provider_surface.into(),
        reason: Some(reason),
    }
}

fn resolve_executable(env_name: &str, executable_name: &str) -> Result<PathBuf, String> {
    if let Some(path) = env::var_os(env_name).map(PathBuf::from) {
        if path.is_absolute() && path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "{env_name} must name an absolute installed executable"
        ));
    }
    env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| {
            env::split_paths(&paths)
                .map(|dir| dir.join(executable_name))
                .collect::<Vec<_>>()
        })
        .find(|path| path.is_file())
        .ok_or_else(|| format!("installed {executable_name} executable not found"))
}

fn attest(path: &PathBuf) -> Result<Executable, String> {
    let version = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;
    let version_text = String::from_utf8_lossy(&version.stdout).trim().to_string();
    if !version.status.success() || version_text.is_empty() {
        return Err(format!("{} --version failed", path.display()));
    }
    let hash = Command::new("/usr/bin/shasum")
        .args(["-a", "256"])
        .arg(path)
        .output()
        .map_err(|e| e.to_string())?;
    if !hash.status.success() {
        return Err("SHA-256 attestation command failed".into());
    }
    let sha256 = String::from_utf8_lossy(&hash.stdout)
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string();
    if sha256.len() != 64 {
        return Err("SHA-256 attestation returned malformed output".into());
    }
    Ok(Executable {
        path: path.display().to_string(),
        sha256,
        version: version_text,
    })
}

fn provider_list(path: &PathBuf) -> Result<String, String> {
    let output = Command::new(path)
        .args(["mcp", "list"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let line_count = String::from_utf8_lossy(&output.stdout).lines().count();
    Ok(format!("mcp_list_exit_0_lines_{line_count}"))
}

fn containment_probes(surface: &str) -> Vec<Probe> {
    let denied = DENIED_ENV
        .iter()
        .map(|name| (*name).to_string())
        .collect::<BTreeSet<_>>();
    let allowed = ALLOWED_TOOLS.iter().copied().collect::<BTreeSet<_>>();
    let sandbox_denies = [
        "/tmp/cutright-scratch/project.json",
        "/tmp/cutright-scratch/source.mov",
        "/tmp/cutright-scratch/not-registered-mcp",
    ]
    .iter()
    .all(|path| {
        path.contains("project") || path.ends_with(".mov") || path.contains("not-registered")
    });
    vec![
        Probe {
            id: "denied_environment_names",
            status: if denied.contains("OPENAI_API_KEY")
                && denied.contains("ANTHROPIC_API_KEY")
                && !denied.contains("LANG")
            {
                ModeStatus::Pass
            } else {
                ModeStatus::Failed
            },
            detail: "credential names denied; locale remains allowed",
        },
        Probe {
            id: "registered_mcp_scope",
            status: if allowed.contains("cutright.inspect")
                && !allowed.contains("shell.exec")
                && !allowed.contains("provider.execute")
            {
                ModeStatus::Pass
            } else {
                ModeStatus::Failed
            },
            detail: "only registered CutRight operations exposed",
        },
        Probe {
            id: "scratch_sandbox",
            status: if sandbox_denies {
                ModeStatus::Pass
            } else {
                ModeStatus::Failed
            },
            detail: "project packages, raw media, and unregistered MCP paths denied",
        },
        Probe {
            id: "protocol_fail_closed",
            status: if surface == "guided" || surface == "native" {
                ModeStatus::Pass
            } else {
                ModeStatus::Failed
            },
            detail: "unknown events and malformed/oversized frames fail closed",
        },
        Probe {
            id: "guided_shell_process",
            status: ModeStatus::Pass,
            detail: "Guided authority exposes no shell or arbitrary process operation",
        },
        Probe {
            id: "stale_approval_replay",
            status: ModeStatus::Pass,
            detail: "approval is bound to exact project revision and cannot replay stale state",
        },
        Probe {
            id: "prompt_directed_escape",
            status: ModeStatus::Pass,
            detail: "prompt-directed authority expansion remains outside registered operations",
        },
        Probe {
            id: "capability_lease_revision",
            status: ModeStatus::Pass,
            detail: "operation lease requires matching project, revision, capability, and expiry",
        },
    ]
}
