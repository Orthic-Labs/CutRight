//! CutRight's protocol-clean stdio MCP bridge.
//!
//! This binary owns no project state and never launches Studio, Claude Code,
//! Codex, or a bundled model. It presents the external agent with the MCP
//! transport and leaves daemon/service execution behind the typed boundary.

#[path = "../mcp.rs"]
#[allow(dead_code)]
mod mcp;
#[cfg(not(test))]
#[path = "../tools.rs"]
#[allow(dead_code)]
mod tools;
#[cfg(test)]
#[allow(dead_code)]
mod tools {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ToolDescriptor {
        pub schema: String,
        pub capability_id: String,
        pub kind: ToolKind,
        pub owner_component: String,
        pub permission_set: String,
        pub description: String,
        pub input_schema: serde_json::Value,
    }

    impl ToolDescriptor {
        pub fn is_mutation(&self) -> bool {
            matches!(self.kind, ToolKind::Mutation)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum ToolKind {
        Read,
        Mutation,
    }

    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct McpToolRegistry {
        tools: BTreeMap<String, ToolDescriptor>,
    }

    impl McpToolRegistry {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn insert(&mut self, descriptor: ToolDescriptor) {
            self.tools
                .insert(descriptor.capability_id.clone(), descriptor);
        }

        pub fn lookup(&self, id: &str) -> Option<&ToolDescriptor> {
            self.tools.get(id)
        }

        pub fn iter(&self) -> impl Iterator<Item = (&str, &ToolDescriptor)> {
            self.tools
                .iter()
                .map(|(id, descriptor)| (id.as_str(), descriptor))
        }
    }
}

use std::env;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use mcp::{StdioBinding, StdioMcpServer};
use tools::{McpToolRegistry, ToolDescriptor, ToolKind};

fn main() {
    if let Err(error) = run() {
        eprintln!("cutright-mcp: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let daemon = ensure_daemon().map_err(|error| error.to_string())?;
    eprintln!("cutright-mcp: daemon {}", daemon.describe());

    let mut server = StdioMcpServer::new(default_registry(), binding_from_environment());
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());
    let mut line = String::new();
    loop {
        line.clear();
        let count = reader
            .by_ref()
            .take((mcp::STDIO_MAX_FRAME_BYTES + 1) as u64)
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if count == 0 {
            return Ok(());
        }
        if count > mcp::STDIO_MAX_FRAME_BYTES {
            return Err("stdio JSON line exceeds transport limit".into());
        }
        let request = serde_json::from_str(line.trim_end()).map_err(|error| error.to_string())?;
        if let Some(response) = server.handle(request) {
            serde_json::to_writer(&mut writer, &response).map_err(|error| error.to_string())?;
            writer.write_all(b"\n").map_err(|error| error.to_string())?;
            writer.flush().map_err(|error| error.to_string())?;
        }
    }
}

fn default_registry() -> McpToolRegistry {
    let mut registry = McpToolRegistry::new();
    registry.insert(ToolDescriptor {
        schema: "cutright.mcp_tool/v1".into(),
        capability_id: "project.read".into(),
        kind: ToolKind::Read,
        owner_component: "cutright-mcp".into(),
        permission_set: "pset.evidence_read_only".into(),
        description: "Use this when an agent needs the bound CutRight project projection. Do not use it to mutate projects, render media, or access arbitrary paths.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {}
        }),
    });
    registry
}

fn binding_from_environment() -> StdioBinding {
    StdioBinding {
        project_id: env::var("CUTRIGHT_PROJECT_ID").unwrap_or_else(|_| "unbound".into()),
        revision: env::var("CUTRIGHT_REVISION").unwrap_or_else(|_| "rev_0".into()),
        principal: env::var("CUTRIGHT_PRINCIPAL").unwrap_or_else(|_| "external-agent".into()),
    }
}

#[derive(Debug)]
enum DaemonState {
    Attached(PathBuf),
    Started(PathBuf),
    Embedded,
}

impl DaemonState {
    fn describe(&self) -> String {
        match self {
            Self::Attached(path) => format!("attached:{path:?}"),
            Self::Started(path) => format!("started:{path:?}"),
            Self::Embedded => "embedded-ready".into(),
        }
    }
}

fn ensure_daemon() -> Result<DaemonState, io::Error> {
    if let Some(socket) = env::var_os("CUTRIGHTD_SOCKET") {
        let socket = PathBuf::from(socket);
        if socket.exists() {
            return Ok(DaemonState::Attached(socket));
        }
    }

    if let Some(binary) = env::var_os("CUTRIGHTD_BIN") {
        let binary = PathBuf::from(binary);
        if !binary.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "CUTRIGHTD_BIN must be absolute",
            ));
        }
        let _child = Command::new(&binary)
            .arg("--background")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        return Ok(DaemonState::Started(binary));
    }

    // The bridge can complete read-only protocol operations without a GUI or
    // a provider child. A packaged daemon is attached through CUTRIGHTD_SOCKET
    // or started through the absolute CUTRIGHTD_BIN path above.
    Ok(DaemonState::Embedded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tool_is_strict_and_returns_structured_content() {
        let mut server = StdioMcpServer::new(default_registry(), StdioBinding::default());
        let listed = server
            .handle(mcp::StdioRequest {
                jsonrpc: "2.0".into(),
                id: Some(serde_json::json!(1)),
                method: "tools/list".into(),
                params: None,
            })
            .unwrap()
            .result
            .unwrap();
        assert_eq!(
            listed["tools"][0]["inputSchema"]["additionalProperties"],
            false
        );
        assert!(listed["tools"][0]["outputSchema"].is_object());

        let called = server
            .handle(mcp::StdioRequest {
                jsonrpc: "2.0".into(),
                id: Some(serde_json::json!(2)),
                method: "tools/call".into(),
                params: Some(serde_json::json!({"name": "project.read", "arguments": {}})),
            })
            .unwrap()
            .result
            .unwrap();
        assert_eq!(called["structuredContent"]["status"], "read_only");
        assert_eq!(called["structuredContent"]["truncated"], false);
    }
}
