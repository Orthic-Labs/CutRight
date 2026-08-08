//! Small, vendor-neutral boundary for user-installed Claude Code and Codex.
//!
//! This crate owns launch descriptions, executable attestation, denied ambient
//! environment names, bounded stdio supervision, parser normalization, and
//! replay cursors. It has no model, bundled runtime, or stateful storage
//! dependency.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const DENIED_ENV_VARS: &[&str] = &[
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "OPENROUTER_API_KEY",
    "MINIMAX_API_KEY",
    "TINYFISH_API_KEY",
    "CODERIGHT_DAEMON_TOKEN",
    "CODERIGHT_GATEWAY_TOKEN",
    "CODERIGHT_DRIVER_INSTANCE_SECRET",
];

pub const CUTRIGHT_MCP_SERVER_NAME: &str = "cutright";
pub const ENVIRONMENT_POLICY_VERSION: &str = "cutright-provider-env-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityLease {
    pub project_id: String,
    pub revision: String,
    pub capability: String,
    pub expires_at_unix_ms: u64,
}

impl CapabilityLease {
    pub fn new(
        project_id: impl Into<String>,
        revision: impl Into<String>,
        capability: impl Into<String>,
        expires_at_unix_ms: u64,
    ) -> Result<Self, DriverHostError> {
        let lease = Self {
            project_id: project_id.into(),
            revision: revision.into(),
            capability: capability.into(),
            expires_at_unix_ms,
        };
        if lease.project_id.is_empty() || lease.revision.is_empty() || lease.capability.is_empty() {
            return Err(DriverHostError::InvalidLaunch(
                "lease scope fields must not be empty".into(),
            ));
        }
        Ok(lease)
    }
    pub fn permits(
        &self,
        project_id: &str,
        revision: &str,
        capability: &str,
        now_unix_ms: u64,
    ) -> bool {
        self.project_id == project_id
            && self.revision == revision
            && self.capability == capability
            && now_unix_ms < self.expires_at_unix_ms
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CutrightOperation {
    InspectSource,
    ReadTranscript,
    DraftEditorialPlan,
    ApplyEditorialPlan,
    RenderArtifact,
}

impl CutrightOperation {
    pub const fn requires_approval(self) -> bool {
        matches!(self, Self::ApplyEditorialPlan | Self::RenderArtifact)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationRequest {
    pub operation: CutrightOperation,
    pub media_handle: String,
    pub project_id: String,
    pub revision: String,
    pub lease: CapabilityLease,
}

impl OperationRequest {
    pub fn validate(&self, now_unix_ms: u64) -> Result<(), DriverHostError> {
        if self.media_handle.is_empty()
            || self.media_handle.contains('/')
            || self.media_handle.contains('\\')
        {
            return Err(DriverHostError::InvalidLaunch(
                "operations accept media handles, not filesystem paths".into(),
            ));
        }
        if !self.lease.permits(
            &self.project_id,
            &self.revision,
            "cutright.editorial",
            now_unix_ms,
        ) {
            return Err(DriverHostError::Resume(
                "lease is not bound to this project and revision".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentPolicy {
    pub version: String,
    pub allowed_names: BTreeSet<String>,
    pub denied_names: BTreeSet<String>,
}

impl Default for EnvironmentPolicy {
    fn default() -> Self {
        Self {
            version: ENVIRONMENT_POLICY_VERSION.into(),
            allowed_names: ["LANG", "LC_ALL", "PATH", "HOME", "TMPDIR"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            denied_names: denied_env_vars(),
        }
    }
}

impl EnvironmentPolicy {
    pub fn for_driver(driver: DriverKind) -> Self {
        let mut policy = Self::default();
        policy.allowed_names.insert(
            match driver {
                DriverKind::Claude => "ANTHROPIC_AUTH_TOKEN",
                DriverKind::Codex => "OPENAI_ORG_ID",
            }
            .into(),
        );
        policy
    }
    pub fn sanitize(
        &self,
        parent: &BTreeMap<String, String>,
        explicit: &BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        let mut clean = BTreeMap::new();
        for name in &self.allowed_names {
            if let Some(value) = explicit.get(name).or_else(|| parent.get(name)) {
                clean.insert(name.clone(), value.clone());
            }
        }
        clean.retain(|name, _| !self.denied_names.contains(name));
        clean
    }
    pub fn receipt(&self) -> EnvironmentPolicyReceipt {
        EnvironmentPolicyReceipt {
            version: self.version.clone(),
            allowed_name_hashes: self
                .allowed_names
                .iter()
                .map(|name| {
                    (
                        name.clone(),
                        blake3::hash(name.as_bytes()).to_hex().to_string(),
                    )
                })
                .collect(),
            denied_name_hashes: self
                .denied_names
                .iter()
                .map(|name| blake3::hash(name.as_bytes()).to_hex().to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentPolicyReceipt {
    pub version: String,
    pub allowed_name_hashes: BTreeMap<String, String>,
    pub denied_name_hashes: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriverKind {
    Claude,
    Codex,
}

impl DriverKind {
    pub const fn executable_name(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
    pub const fn protocol(self) -> &'static str {
        match self {
            Self::Claude => "stream_json",
            Self::Codex => "app_server_json_rpc",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchSpec {
    pub driver: DriverKind,
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub env_remove: BTreeSet<String>,
}

impl LaunchSpec {
    pub fn for_driver(
        driver: DriverKind,
        program: impl Into<PathBuf>,
        cwd: impl Into<PathBuf>,
    ) -> Self {
        Self {
            driver,
            program: program.into(),
            args: Vec::new(),
            cwd: cwd.into(),
            env: BTreeMap::new(),
            env_remove: denied_env_vars(),
        }
    }
    pub fn validate(&self) -> Result<(), DriverHostError> {
        if !self.program.is_absolute() {
            return Err(DriverHostError::InvalidLaunch(
                "provider executable must be absolute".into(),
            ));
        }
        if !self.cwd.is_absolute() {
            return Err(DriverHostError::InvalidLaunch(
                "provider cwd must be absolute".into(),
            ));
        }
        if self.program.as_os_str().is_empty() {
            return Err(DriverHostError::InvalidLaunch(
                "provider executable is empty".into(),
            ));
        }
        Ok(())
    }
}

pub fn denied_env_vars() -> BTreeSet<String> {
    DENIED_ENV_VARS
        .iter()
        .map(|name| (*name).to_string())
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NormalizedEventKind {
    SessionStarted {
        vendor_session_id: String,
    },
    TurnStarted {
        vendor_turn_id: String,
    },
    AssistantDelta {
        text: String,
    },
    AssistantCompleted {
        text: String,
    },
    ToolCallStarted {
        call_id: String,
        tool_name: String,
        arguments: Value,
    },
    ToolResult {
        call_id: String,
        is_error: bool,
        content: Value,
    },
    ApprovalRequested {
        request_id: String,
        action: String,
        details: Value,
    },
    TurnCompleted,
    TurnInterrupted,
    TurnFailed {
        message: String,
    },
    GatewayReady,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NormalizedEvent {
    pub driver: DriverKind,
    pub source_offset: u64,
    pub event_key: String,
    pub vendor_method: String,
    pub kind: NormalizedEventKind,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DriverHostError {
    #[error("invalid {driver:?} message: {message}")]
    InvalidMessage { driver: DriverKind, message: String },
    #[error("unknown required {driver:?} event '{method}'")]
    UnknownRequiredEvent { driver: DriverKind, method: String },
    #[error("invalid launch: {0}")]
    InvalidLaunch(String),
    #[error("provider process: {0}")]
    Process(String),
    #[error("resume refused: {0}")]
    Resume(String),
}

fn invalid(driver: DriverKind, message: impl Into<String>) -> DriverHostError {
    DriverHostError::InvalidMessage {
        driver,
        message: message.into(),
    }
}

fn string(value: Option<&Value>, field: &str) -> Result<String, DriverHostError> {
    value
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| DriverHostError::InvalidMessage {
            driver: DriverKind::Claude,
            message: format!("missing string {field}"),
        })
}

fn event(
    driver: DriverKind,
    offset: u64,
    method: &str,
    kind: NormalizedEventKind,
) -> NormalizedEvent {
    NormalizedEvent {
        driver,
        source_offset: offset,
        event_key: format!("{}:{offset}:{method}", driver.executable_name()),
        vendor_method: method.into(),
        kind,
    }
}

fn content_text(value: &Value) -> String {
    value.as_str().map(str::to_owned).unwrap_or_else(|| {
        value
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default()
    })
}

/// Normalize one Claude Code stream-json line. Unknown required events fail closed;
/// known heartbeat/metadata events are intentionally ignored.
pub fn parse_claude_stream_line(
    line: &str,
    offset: u64,
) -> Result<Option<NormalizedEvent>, DriverHostError> {
    let value: Value = serde_json::from_str(line)
        .map_err(|error| invalid(DriverKind::Claude, error.to_string()))?;
    let kind = value
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(DriverKind::Claude, "missing event type"))?;
    let ignored = [
        "ping",
        "message_start",
        "message_delta",
        "message_stop",
        "rate_limit_event",
        "system",
    ];
    match kind {
        "system" => value
            .get("subtype")
            .and_then(Value::as_str)
            .filter(|subtype| *subtype == "init")
            .map(|_| {
                Ok(Some(event(
                    DriverKind::Claude,
                    offset,
                    kind,
                    NormalizedEventKind::GatewayReady,
                )))
            })
            .unwrap_or(Ok(None)),
        "assistant" => {
            let text = content_text(value.pointer("/message/content").unwrap_or(&Value::Null));
            Ok(Some(event(
                DriverKind::Claude,
                offset,
                kind,
                NormalizedEventKind::AssistantCompleted { text },
            )))
        }
        "content_block_delta" => Ok(Some(event(
            DriverKind::Claude,
            offset,
            kind,
            NormalizedEventKind::AssistantDelta {
                text: content_text(value.pointer("/delta/text").unwrap_or(&Value::Null)),
            },
        ))),
        "result" => {
            let subtype = value
                .get("subtype")
                .and_then(Value::as_str)
                .unwrap_or("success");
            let result = value
                .get("result")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let normalized = match subtype {
                "interrupted" => NormalizedEventKind::TurnInterrupted,
                "error" | "failure" => NormalizedEventKind::TurnFailed { message: result },
                _ => NormalizedEventKind::TurnCompleted,
            };
            Ok(Some(event(DriverKind::Claude, offset, kind, normalized)))
        }
        "tool_use" => Ok(Some(event(
            DriverKind::Claude,
            offset,
            kind,
            NormalizedEventKind::ToolCallStarted {
                call_id: string(value.get("id"), "id")?,
                tool_name: string(value.get("name"), "name")?,
                arguments: value.get("input").cloned().unwrap_or(Value::Null),
            },
        ))),
        "tool_result" => Ok(Some(event(
            DriverKind::Claude,
            offset,
            kind,
            NormalizedEventKind::ToolResult {
                call_id: string(value.get("tool_use_id"), "tool_use_id")?,
                is_error: value
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                content: value.get("content").cloned().unwrap_or(Value::Null),
            },
        ))),
        "control_request"
            if value.pointer("/request/subtype").and_then(Value::as_str)
                == Some("can_use_tool") =>
        {
            Ok(Some(event(
                DriverKind::Claude,
                offset,
                kind,
                NormalizedEventKind::ApprovalRequested {
                    request_id: string(value.get("request_id"), "request_id")?,
                    action: value
                        .pointer("/request/tool_name")
                        .and_then(Value::as_str)
                        .unwrap_or("tool")
                        .into(),
                    details: value.get("request").cloned().unwrap_or(Value::Null),
                },
            )))
        }
        other if ignored.contains(&other) => Ok(None),
        other => Err(DriverHostError::UnknownRequiredEvent {
            driver: DriverKind::Claude,
            method: other.into(),
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CodexMessage {
    Response {
        id: Value,
        result: Option<Value>,
        error: Option<Value>,
    },
    Event(NormalizedEvent),
}

/// Normalize one Codex app-server JSON-RPC line without treating arbitrary methods as safe.
pub fn parse_codex_jsonrpc_line(
    line: &str,
    offset: u64,
) -> Result<Option<CodexMessage>, DriverHostError> {
    let value: Value = serde_json::from_str(line)
        .map_err(|error| invalid(DriverKind::Codex, error.to_string()))?;
    if let Some(id) = value.get("id") {
        return Ok(Some(CodexMessage::Response {
            id: id.clone(),
            result: value.get("result").cloned(),
            error: value.get("error").cloned(),
        }));
    }
    let method = value
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(DriverKind::Codex, "JSON-RPC message lacks id and method"))?;
    let params = value.get("params").cloned().unwrap_or(Value::Null);
    let kind = match method {
        "thread/started" => NormalizedEventKind::SessionStarted {
            vendor_session_id: params
                .pointer("/thread/id")
                .or_else(|| params.get("threadId"))
                .and_then(Value::as_str)
                .ok_or_else(|| invalid(DriverKind::Codex, "thread/started lacks thread id"))?
                .into(),
        },
        "turn/started" => NormalizedEventKind::TurnStarted {
            vendor_turn_id: params
                .pointer("/turn/id")
                .or_else(|| params.get("turnId"))
                .and_then(Value::as_str)
                .ok_or_else(|| invalid(DriverKind::Codex, "turn/started lacks turn id"))?
                .into(),
        },
        "item/agentMessage/delta" => NormalizedEventKind::AssistantDelta {
            text: params
                .get("delta")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
        },
        "item/agentMessage/completed" => NormalizedEventKind::AssistantCompleted {
            text: params
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
        },
        "item/commandExecution/requestApproval" | "item/fileChange/requestApproval" => {
            NormalizedEventKind::ApprovalRequested {
                request_id: params
                    .get("requestId")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .into(),
                action: method.into(),
                details: params,
            }
        }
        "turn/completed" => NormalizedEventKind::TurnCompleted,
        "turn/interrupted" => NormalizedEventKind::TurnInterrupted,
        "gateway/ready" => NormalizedEventKind::GatewayReady,
        other => {
            return Err(DriverHostError::UnknownRequiredEvent {
                driver: DriverKind::Codex,
                method: other.into(),
            })
        }
    };
    Ok(Some(CodexMessage::Event(event(
        DriverKind::Codex,
        offset,
        method,
        kind,
    ))))
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverCursor {
    pub input_offset: u64,
    pub output_offset: u64,
    pub last_event_key: Option<String>,
}

pub fn validate_resume(
    cursor: &DriverCursor,
    requested_input: u64,
    requested_output: u64,
) -> Result<(), DriverHostError> {
    if requested_input < cursor.input_offset || requested_output < cursor.output_offset {
        return Err(DriverHostError::Resume("cursor moved backwards".into()));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutableAttestation {
    pub driver: DriverKind,
    pub executable: PathBuf,
    pub sha256: String,
    pub version: String,
}

pub fn attest_executable(
    driver: DriverKind,
    executable: impl AsRef<Path>,
    version: impl Into<String>,
) -> Result<ExecutableAttestation, DriverHostError> {
    let executable = executable.as_ref();
    if !executable.is_absolute() {
        return Err(DriverHostError::InvalidLaunch(
            "executable attestation requires absolute path".into(),
        ));
    }
    let bytes =
        std::fs::read(executable).map_err(|error| DriverHostError::Process(error.to_string()))?;
    Ok(ExecutableAttestation {
        driver,
        executable: executable.into(),
        sha256: blake3::hash(&bytes).to_hex().to_string(),
        version: version.into(),
    })
}

/// Spawn only after validation, clearing the inherited environment before applying
/// explicit non-secret values. The returned process exposes stdio only.
pub fn spawn_stdio(spec: &LaunchSpec) -> Result<DriverProcess, DriverHostError> {
    spec.validate()?;
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env_clear();
    for (name, value) in &spec.env {
        if !DENIED_ENV_VARS.contains(&name.as_str()) {
            command.env(name, value);
        }
    }
    let child = command
        .spawn()
        .map_err(|error| DriverHostError::Process(error.to_string()))?;
    Ok(DriverProcess {
        child,
        stdin: None,
        stdout: None,
    })
}

#[derive(Debug)]
pub struct DriverProcess {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
}

impl DriverProcess {
    pub fn take_stdin(&mut self) -> Result<&mut ChildStdin, DriverHostError> {
        if self.stdin.is_none() {
            self.stdin = self.child.stdin.take();
        }
        self.stdin
            .as_mut()
            .ok_or_else(|| DriverHostError::Process("provider stdin unavailable".into()))
    }
    pub fn take_stdout(&mut self) -> Result<&mut BufReader<ChildStdout>, DriverHostError> {
        if self.stdout.is_none() {
            self.stdout = self.child.stdout.take().map(BufReader::new);
        }
        self.stdout
            .as_mut()
            .ok_or_else(|| DriverHostError::Process("provider stdout unavailable".into()))
    }
    pub fn send_json(&mut self, value: &Value) -> Result<(), DriverHostError> {
        let stdin = self.take_stdin()?;
        writeln!(stdin, "{}", value).map_err(|error| DriverHostError::Process(error.to_string()))
    }
    pub fn read_line(&mut self, line: &mut String) -> Result<usize, DriverHostError> {
        self.take_stdout()?
            .read_line(line)
            .map_err(|error| DriverHostError::Process(error.to_string()))
    }
    pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, DriverHostError> {
        self.child
            .try_wait()
            .map_err(|error| DriverHostError::Process(error.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderDescriptor {
    pub driver: DriverKind,
    pub protocol: String,
    pub user_installed: bool,
    pub bundled_runtime: bool,
}

pub fn provider_registry() -> Vec<ProviderDescriptor> {
    DriverKind::all()
        .iter()
        .map(|driver| ProviderDescriptor {
            driver: *driver,
            protocol: driver.protocol().into(),
            user_installed: true,
            bundled_runtime: false,
        })
        .collect()
}

impl DriverKind {
    const fn all() -> [Self; 2] {
        [Self::Claude, Self::Codex]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_routes_strip_ambient_secrets() {
        assert!(denied_env_vars().contains("OPENAI_API_KEY"));
        assert!(denied_env_vars().contains("ANTHROPIC_API_KEY"));
    }
    #[test]
    fn claude_and_codex_normalize_known_events() {
        assert!(matches!(
            parse_claude_stream_line(r#"{"type":"content_block_delta","delta":{"text":"hi"}}"#, 1)
                .unwrap()
                .unwrap()
                .kind,
            NormalizedEventKind::AssistantDelta { .. }
        ));
        assert!(matches!(
            parse_codex_jsonrpc_line(r#"{"method":"turn/completed","params":{}}"#, 1)
                .unwrap()
                .unwrap(),
            CodexMessage::Event(_)
        ));
    }
    #[test]
    fn unknown_protocol_events_fail_closed() {
        assert!(parse_claude_stream_line(r#"{"type":"future_event"}"#, 0).is_err());
        assert!(parse_codex_jsonrpc_line(r#"{"method":"future/event","params":{}}"#, 0).is_err());
    }
    #[test]
    fn resume_cursor_never_moves_backwards() {
        let cursor = DriverCursor {
            input_offset: 4,
            output_offset: 8,
            last_event_key: None,
        };
        assert!(validate_resume(&cursor, 4, 8).is_ok());
        assert!(validate_resume(&cursor, 3, 8).is_err());
    }
    #[test]
    fn registry_has_only_user_installed_routes() {
        let routes = provider_registry();
        assert_eq!(routes.len(), 2);
        assert!(routes
            .iter()
            .all(|route| route.user_installed && !route.bundled_runtime));
    }

    #[test]
    fn lease_is_bound_to_project_revision_and_expiry() {
        let lease = CapabilityLease::new("p1", "r2", "cutright.editorial", 10).unwrap();
        assert!(lease.permits("p1", "r2", "cutright.editorial", 9));
        assert!(!lease.permits("p2", "r2", "cutright.editorial", 9));
        assert!(!lease.permits("p1", "r2", "cutright.editorial", 10));
    }

    #[test]
    fn environment_policy_drops_parent_secrets_and_keeps_provider_login_surface() {
        let policy = EnvironmentPolicy::for_driver(DriverKind::Claude);
        let parent = BTreeMap::from([
            (String::from("ANTHROPIC_API_KEY"), String::from("secret")),
            (String::from("LANG"), String::from("en_US.UTF-8")),
        ]);
        let clean = policy.sanitize(&parent, &BTreeMap::new());
        assert!(!clean.contains_key("ANTHROPIC_API_KEY"));
        assert_eq!(clean.get("LANG").map(String::as_str), Some("en_US.UTF-8"));
    }

    #[test]
    fn operation_requests_reject_paths_and_wrong_scope() {
        let lease = CapabilityLease::new("p1", "r1", "cutright.editorial", 99).unwrap();
        let mut request = OperationRequest {
            operation: CutrightOperation::ReadTranscript,
            media_handle: "/tmp/raw.mov".into(),
            project_id: "p1".into(),
            revision: "r1".into(),
            lease,
        };
        assert!(request.validate(1).is_err());
        request.media_handle = "media:source-1".into();
        assert!(request.validate(1).is_ok());
    }
}
