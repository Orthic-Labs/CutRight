//! `videoctl agent` provider-native MCP integration.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};
#[path = "../../video-runtime/src/agent_integration.rs"]
mod agent_integration;

use agent_integration::{
    add_owned_entry, remove_owned_entry, ConfigSnapshot, IntegrationError, Provider,
    SemanticDiff,
};

const SERVER_NAME: &str = "cutright";

#[derive(Debug, Clone)]
pub struct AgentCommand {
    pub provider: Provider,
    pub binary: PathBuf,
    pub config: PathBuf,
    pub remove: bool,
}

pub fn run(command: AgentCommand) -> Result<Value, String> {
    let snapshot = ConfigSnapshot::capture(&command.config).map_err(|error| error.to_string())?;
    let before: Value = serde_json::from_slice(snapshot.exact_bytes()).map_err(|error| error.to_string())?;
    let after = if command.remove {
        remove_owned_entry(&before)
    } else {
        add_owned_entry(&before, &command.binary)
    }
    .map_err(|error| error.to_string())?;
    let diff = SemanticDiff::between(&before, &after).map_err(|error| error.to_string())?;
    if !diff.unrelated_entries_preserved {
        return Err("provider diff touched an unrelated MCP entry".into());
    }

    let cli = resolve_cli(command.provider)?;
    let operation = if command.remove { "remove" } else { "add" };
    let native = native_mcp_command(&cli, operation, &command.binary, command.remove)?;
    fs::write(&command.config, serde_json::to_vec_pretty(&after).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?;
    Ok(json!({
        "event": format!("agent.{operation}"),
        "provider": command.provider,
        "server": SERVER_NAME,
        "snapshot_bytes": snapshot.exact_bytes().len(),
        "diff": diff,
        "provider_native": native,
        "rollback": {"restore_bytes": true, "path": command.config},
        "ready": true
    }))
}

pub fn status(provider: Provider, config: &Path) -> Result<Value, String> {
    let snapshot = ConfigSnapshot::capture(config).map_err(|error| error.to_string())?;
    let value: Value = serde_json::from_slice(snapshot.exact_bytes()).map_err(|error| error.to_string())?;
    let servers = value.get("mcpServers").and_then(Value::as_object);
    Ok(json!({
        "event": "agent.status",
        "provider": provider,
        "server": SERVER_NAME,
        "registered": servers.and_then(|entries| entries.get(SERVER_NAME)).is_some(),
        "provider_cli": resolve_cli(provider).map_err(|error| error.to_string())?
    }))
}

fn resolve_cli(provider: Provider) -> Result<PathBuf, String> {
    let variable = match provider {
        Provider::ClaudeCode => "CUTRIGHT_CLAUDE_CODE_BIN",
        Provider::Codex => "CUTRIGHT_CODEX_BIN",
    };
    if let Some(path) = env::var_os(variable).map(PathBuf::from) {
        if path.is_absolute() && path.is_file() {
            return Ok(path);
        }
        return Err(format!("{variable} must name an absolute installed CLI"));
    }
    which(provider.command_name()).ok_or_else(|| format!("installed {} CLI not found", provider.command_name()))
}

fn which(name: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|candidate| candidate.is_file())
    })
}

fn native_mcp_command(
    cli: &Path,
    operation: &str,
    binary: &Path,
    remove: bool,
) -> Result<Value, String> {
    let mut command = Command::new(cli);
    command.args(["mcp", operation, SERVER_NAME]);
    if !remove {
        command.args(["--transport", "stdio", "--"]);
        command.arg(binary);
    }
    let output = command.output().map_err(|error| IntegrationError::ProviderCommand(error.to_string()).to_string())?;
    if !output.status.success() {
        return Err(IntegrationError::ProviderCommand(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        )
        .to_string());
    }
    Ok(json!({"command": command.get_program(), "stdout": String::from_utf8_lossy(&output.stdout)}))
}
