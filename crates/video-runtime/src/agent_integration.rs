//! Provider MCP registration with exact snapshots and bounded semantic diffs.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

const OWNED_SERVER: &str = "cutright";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    ClaudeCode,
    Codex,
}

impl Provider {
    pub fn parse(value: &str) -> Result<Self, IntegrationError> {
        match value {
            "claude" | "claude-code" => Ok(Self::ClaudeCode),
            "codex" => Ok(Self::Codex),
            _ => Err(IntegrationError::UnsupportedProvider(value.into())),
        }
    }

    pub fn command_name(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude",
            Self::Codex => "codex",
        }
    }
}

#[derive(Debug)]
pub enum IntegrationError {
    UnsupportedProvider(String),
    InvalidConfig,
    MissingServers,
    ProviderCommand(String),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for IntegrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedProvider(value) => write!(formatter, "unsupported provider: {value}"),
            Self::InvalidConfig => {
                write!(formatter, "provider configuration must be a JSON object")
            }
            Self::MissingServers => {
                write!(formatter, "provider configuration has no mcpServers object")
            }
            Self::ProviderCommand(value) => write!(formatter, "provider CLI failed: {value}"),
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
        }
    }
}

impl std::error::Error for IntegrationError {}

impl From<std::io::Error> for IntegrationError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for IntegrationError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    pub path: PathBuf,
    #[serde(skip)]
    pub bytes: Vec<u8>,
}

impl ConfigSnapshot {
    pub fn capture(path: impl AsRef<Path>) -> Result<Self, IntegrationError> {
        let path = path.as_ref().to_path_buf();
        Ok(Self {
            bytes: fs::read(&path)?,
            path,
        })
    }

    #[allow(dead_code)]
    pub fn restore(&self) -> Result<(), IntegrationError> {
        fs::write(&self.path, &self.bytes)?;
        Ok(())
    }

    pub fn exact_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
    pub unrelated_entries_preserved: bool,
}

impl SemanticDiff {
    pub fn between(before: &Value, after: &Value) -> Result<Self, IntegrationError> {
        let before = servers(before)?;
        let after = servers(after)?;
        let keys = before
            .keys()
            .chain(after.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut diff = Self {
            added: Vec::new(),
            removed: Vec::new(),
            changed: Vec::new(),
            unrelated_entries_preserved: true,
        };
        for key in keys {
            match (before.get(&key), after.get(&key)) {
                (None, Some(_)) => diff.added.push(key),
                (Some(_), None) => diff.removed.push(key),
                (Some(before), Some(after)) if before != after => diff.changed.push(key),
                _ => {}
            }
        }
        diff.unrelated_entries_preserved = diff
            .added
            .iter()
            .chain(diff.removed.iter())
            .chain(diff.changed.iter())
            .all(|key| key == OWNED_SERVER);
        Ok(diff)
    }
}

pub fn owned_entry(binary: &Path) -> Value {
    serde_json::json!({
        "command": binary,
        "args": [],
        "env": {"CUTRIGHT_AGENT_MODE": "read-only"}
    })
}

pub fn add_owned_entry(config: &Value, binary: &Path) -> Result<Value, IntegrationError> {
    let mut root = config
        .as_object()
        .cloned()
        .ok_or(IntegrationError::InvalidConfig)?;
    let mut servers = servers(config)?.clone();
    servers.insert(OWNED_SERVER.into(), owned_entry(binary));
    root.insert("mcpServers".into(), Value::Object(servers));
    Ok(Value::Object(root))
}

pub fn remove_owned_entry(config: &Value) -> Result<Value, IntegrationError> {
    let mut root = config
        .as_object()
        .cloned()
        .ok_or(IntegrationError::InvalidConfig)?;
    let mut servers = servers(config)?.clone();
    servers.remove(OWNED_SERVER);
    root.insert("mcpServers".into(), Value::Object(servers));
    Ok(Value::Object(root))
}

fn servers(config: &Value) -> Result<&Map<String, Value>, IntegrationError> {
    config
        .get("mcpServers")
        .and_then(Value::as_object)
        .ok_or(IntegrationError::MissingServers)
}
