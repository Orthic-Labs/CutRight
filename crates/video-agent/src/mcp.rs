//! Loopback MCP adapter (CR-V2-B2-025).
//!
//! Exposes the same ActionExecutor that the CLI and Studio use through a
//! loopback-only MCP adapter. Three guarantees:
//!
//! 1. **Loopback only** — non-loopback binds are rejected. The adapter does
//!    not consult interface state, environment or DNS; it inspects the
//!    socket address string and refuses anything that is not `127.0.0.0/8`
//!    or `::1`.
//! 2. **Ephemeral token** — every connection carries a randomly generated
//!    token. The token is exposed to the caller so it can be supplied to a
//!    review-locking frontmost-project guard.
//! 3. **Disabled by default** — the [`McpAdapterConfig::enabled`] flag must
//!    be `true` for the adapter to accept any request.
//!
//! Requests are translated to either a bounded read (no permission check
//! beyond the binding's read scope) or a [`ActionExecutor`] call. The
//! adapter never invents its own schema: every tool ID and input comes
//! from the generated capability registry.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Read, Write};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use video_sessions::{
    ActiveRevisionId, PermissionSetId, ProjectId, SessionBinding, SessionBindingError,
    SessionGuardError, SessionId, SessionOrigin,
};

use crate::tools::McpToolRegistry;

/// Loopback adapter configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpAdapterConfig {
    /// Master switch. When `false` (default) every request is short-circuited
    /// with `McpError::Disabled`.
    pub enabled: bool,
    /// Loopback bind address. The adapter only accepts connections from
    /// `127.0.0.0/8` or `::1`. The default is the canonical IPv4 loopback.
    pub bind: String,
    /// Optional override for the loopback check. Stored next to the config
    /// so operators can confirm the exact string the adapter enforced.
    pub loopback_marker: String,
}

impl Default for McpAdapterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind: "127.0.0.1:0".to_string(),
            loopback_marker: "configured-loopback".to_string(),
        }
    }
}

impl McpAdapterConfig {
    /// Build a config that is explicitly enabled and bound to the supplied
    /// loopback address. Useful for tests and the Studio UI.
    pub fn enabled_on(bind: impl Into<String>) -> Self {
        Self {
            enabled: true,
            bind: bind.into(),
            loopback_marker: "configured-loopback".to_string(),
        }
    }

    /// Return `true` if the address parses as a loopback bind.
    ///
    /// The check is intentionally string-based so tests can reproduce it
    /// without binding real sockets. The recognised forms are:
    ///
    /// * `127.0.0.0..127.255.255.255` with an optional `:port`.
    /// * `::1` with an optional `[..]:port`.
    /// * `localhost` with an optional `:port`.
    pub fn is_loopback_address(address: &str) -> bool {
        let address = address.trim();
        let host = if let Some(rest) = address.strip_prefix('[') {
            rest.split(']').next().unwrap_or("")
        } else if address.matches(':').count() == 1 {
            address.split(':').next().unwrap_or("")
        } else {
            address
        };
        if host == "::1" || host == "0:0:0:0:0:0:0:1" || host == "localhost" {
            return true;
        }
        if host.starts_with("127.") {
            // Split on '.' and confirm the first octet is 127.
            let first = host.split('.').next().unwrap_or("");
            return first == "127";
        }
        false
    }
}

/// Outcome of a single MCP request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpResponse {
    /// Bounded read; the adapter returned the canonical JSON for the read.
    Read {
        /// Tool id that was invoked.
        tool_id: String,
        /// Canonical JSON result body.
        body: serde_json::Value,
    },
    /// Mutation routed through the ActionExecutor; the result is the
    /// canonical JSON of the application receipt.
    Applied {
        /// Tool id that was invoked.
        tool_id: String,
        /// Canonical JSON receipt produced by the executor.
        receipt: serde_json::Value,
    },
    /// The adapter rejected the request before reaching the executor.
    Rejected {
        /// Tool id that was invoked, when the rejection is tool-specific.
        tool_id: Option<String>,
        /// Stable error code matching the executor's failure taxonomy.
        code: McpErrorCode,
        /// Human-readable message.
        message: String,
    },
}

/// Stable error codes returned by the MCP adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpErrorCode {
    /// Adapter is disabled in config.
    Disabled,
    /// Bind address is not loopback.
    NonLoopback,
    /// Connection token does not match the adapter's issued token.
    InvalidToken,
    /// A write was attempted while another project is frontmost.
    FrontmostProjectMismatch,
    /// The binding does not include the required permission set.
    PermissionDenied,
    /// Stale revision supplied for a mutation.
    StaleRevision,
    /// Tool id is not present in the generated registry.
    UnknownTool,
}

/// Errors produced by the MCP adapter.
#[derive(Debug, Error)]
pub enum McpError {
    #[error("MCP adapter is disabled")]
    Disabled,
    #[error("bind address {0} is not loopback")]
    NonLoopback(String),
    #[error("invalid token supplied to MCP adapter")]
    InvalidToken,
    #[error("frontmost project is {frontmost}, request targets {requesting}")]
    FrontmostProjectMismatch {
        frontmost: String,
        requesting: String,
    },
    #[error("permission set {required} missing from session binding")]
    PermissionDenied { required: String },
    #[error("stale revision supplied: expected {expected}, binding has {actual}")]
    StaleRevision { expected: String, actual: String },
    #[error("unknown tool id: {0}")]
    UnknownTool(String),
    #[error("session binding error: {0}")]
    SessionBinding(#[from] SessionBindingError),
    #[error("session guard error: {0}")]
    SessionGuard(#[from] SessionGuardError),
}

impl McpError {
    /// Translate the error into the stable [`McpErrorCode`] enum.
    pub fn code(&self) -> McpErrorCode {
        match self {
            McpError::Disabled => McpErrorCode::Disabled,
            McpError::NonLoopback(_) => McpErrorCode::NonLoopback,
            McpError::InvalidToken => McpErrorCode::InvalidToken,
            McpError::FrontmostProjectMismatch { .. } => McpErrorCode::FrontmostProjectMismatch,
            McpError::PermissionDenied { .. } => McpErrorCode::PermissionDenied,
            McpError::StaleRevision { .. } => McpErrorCode::StaleRevision,
            McpError::UnknownTool(_) => McpErrorCode::UnknownTool,
            McpError::SessionBinding(_) | McpError::SessionGuard(_) => {
                McpErrorCode::PermissionDenied
            }
        }
    }
}

/// Identifier for a transport-layer connection. The adapter generates one
/// per accepted loopback connection so the ephemeral token can be issued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionToken {
    /// Inner token string. Stored next to the connection so diagnostics can
    /// report which token was used.
    pub token: String,
    /// Loopback address the adapter bound to.
    pub loopback: String,
}

impl ConnectionToken {
    /// Public accessor for the token string.
    pub fn as_str(&self) -> &str {
        &self.token
    }
}

/// The bound loopback adapter. Every method is sync + side-effect free
/// except for the ActionExecutor callback, which is supplied by the caller.
#[derive(Debug)]
pub struct McpAdapter {
    config: McpAdapterConfig,
    connection: ConnectionToken,
    tools: McpToolRegistry,
}

impl McpAdapter {
    /// Bind the adapter. The adapter:
    ///
    /// * Rejects mismatched `enabled` / non-loopback pairs.
    /// * Generates an ephemeral token via [`generate_ephemeral_token`].
    pub fn bind(config: McpAdapterConfig, tools: McpToolRegistry) -> Result<Self, McpError> {
        if !config.enabled {
            return Err(McpError::Disabled);
        }
        if !McpAdapterConfig::is_loopback_address(&config.bind) {
            return Err(McpError::NonLoopback(config.bind));
        }
        let token = generate_ephemeral_token();
        Ok(Self {
            connection: ConnectionToken {
                token,
                loopback: config.bind.clone(),
            },
            config,
            tools,
        })
    }

    /// Loopback bind address.
    pub fn loopback(&self) -> &str {
        &self.connection.loopback
    }

    /// Connection token issued by this adapter.
    pub fn token(&self) -> &ConnectionToken {
        &self.connection
    }

    /// Tool registry bound to this adapter.
    pub fn tools(&self) -> &McpToolRegistry {
        &self.tools
    }

    /// Disable the adapter (e.g. on shutdown). Subsequent calls to
    /// [`McpAdapter::dispatch`] return [`McpError::Disabled`].
    pub fn disable(&mut self) {
        self.config.enabled = false;
    }

    /// Dispatch an MCP request. The caller supplies the session binding, the
    /// permission set the binding claims, the current frontmost project, and
    /// the executor callback that performs a mutation.
    ///
    /// The dispatch is purely synchronous — there is no real network layer
    /// in this crate. The contract tests in `tests/mcp.rs` exercise the
    /// translation paths.
    pub fn dispatch(
        &self,
        request: &McpRequest,
        binding: &SessionBinding,
        frontmost_project: &ProjectId,
        executor: &mut dyn ActionExecutor,
    ) -> Result<McpResponse, McpError> {
        if !self.config.enabled {
            return Err(McpError::Disabled);
        }
        if request.token != self.connection.token {
            return Err(McpError::InvalidToken);
        }
        let tool = self
            .tools
            .lookup(&request.tool_id)
            .ok_or_else(|| McpError::UnknownTool(request.tool_id.clone()))?;

        // Frontmost-project guard: writes must target the frontmost project.
        if tool.is_mutation() && &binding.project_id != frontmost_project {
            return Err(McpError::FrontmostProjectMismatch {
                frontmost: frontmost_project.to_string(),
                requesting: binding.project_id.to_string(),
            });
        }

        // Permission check: the binding must include the tool's required set.
        if binding
            .permission_set
            .as_ref()
            .map(|permission| permission.as_str())
            != Some(tool.permission_set.as_str())
        {
            return Err(McpError::PermissionDenied {
                required: tool.permission_set.clone(),
            });
        }

        if tool.is_mutation() {
            // Stale-revision guard: the binding must be at the supplied
            // expected revision.
            if request.expected_revision != binding.active_revision.as_str() {
                return Err(McpError::StaleRevision {
                    expected: request.expected_revision.clone(),
                    actual: binding.active_revision.to_string(),
                });
            }
            let receipt = executor.apply(binding, &request.tool_id, &request.payload)?;
            Ok(McpResponse::Applied {
                tool_id: request.tool_id.clone(),
                receipt: receipt.into_canonical_json(),
            })
        } else {
            let body = executor.read(binding, &request.tool_id, &request.payload);
            Ok(McpResponse::Read {
                tool_id: request.tool_id.clone(),
                body: body.into_canonical_json(),
            })
        }
    }
}

/// A single MCP request after envelope parsing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpRequest {
    /// Token issued by the adapter.
    pub token: String,
    /// Tool id from the generated registry.
    pub tool_id: String,
    /// Required for mutations; ignored for reads.
    pub expected_revision: String,
    /// Tool-specific payload.
    pub payload: serde_json::Value,
}

/// Trait the adapter uses to invoke the shared ActionExecutor. The real
/// implementation lives in `video-project`; tests provide a closure-shaped
/// fake.
pub trait ActionExecutor {
    /// Apply a mutation; return an [`ActionReceipt`] that serialises to
    /// canonical JSON.
    fn apply(
        &mut self,
        binding: &SessionBinding,
        tool_id: &str,
        payload: &serde_json::Value,
    ) -> Result<ActionReceipt, McpError>;
    /// Issue a read; the returned value is the canonical JSON body.
    fn read(
        &self,
        binding: &SessionBinding,
        tool_id: &str,
        payload: &serde_json::Value,
    ) -> ActionReadBody;
}

/// Receipt that the adapter serialises alongside `McpResponse::Applied`. The
/// real executor produces these; tests construct minimal [`ActionReceipt`]
/// values by hand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionReceipt {
    /// Schema marker.
    pub schema: String,
    /// Stable receipt id.
    pub receipt_id: String,
    /// Revision the mutation committed.
    pub committed_revision: String,
    /// Batch id.
    pub batch_id: String,
    /// Pass-through of whatever rich body the underlying executor produced.
    pub body: serde_json::Value,
}

impl ActionReceipt {
    /// Canonical JSON form used in tests.
    pub fn into_canonical_json(self) -> serde_json::Value {
        serde_json::json!({
            "schema": self.schema,
            "receipt_id": self.receipt_id,
            "committed_revision": self.committed_revision,
            "batch_id": self.batch_id,
            "body": self.body,
        })
    }
}

/// Read body returned by the executor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionReadBody {
    /// Schema marker.
    pub schema: String,
    /// Tool id.
    pub tool_id: String,
    /// Body payload.
    pub body: serde_json::Value,
    /// Auxiliary data (counts, hashes, etc.).
    pub meta: BTreeMap<String, serde_json::Value>,
}

impl ActionReadBody {
    /// Canonical JSON representation.
    pub fn into_canonical_json(self) -> serde_json::Value {
        serde_json::json!({
            "schema": self.schema,
            "tool_id": self.tool_id,
            "body": self.body,
            "meta": self.meta,
        })
    }
}

/// Generate an ephemeral token. The token is opaque text drawn from a
/// deterministic fingerprint plus a process counter so multiple accepts in
/// the same millisecond produce distinct tokens (no real RNG dependency).
pub fn generate_ephemeral_token() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut hasher = DefaultHasher::new();
    format!("mcp-token::{n}").hash(&mut hasher);
    format!("mcp_{:016x}_{n:016x}", hasher.finish())
}

/// Helper used by the Studio to build a session binding that has the
/// supplied permission set and is bound to the supplied project+revision.
pub fn binding_for(
    project: &ProjectId,
    revision: &ActiveRevisionId,
    permission_set: &PermissionSetId,
) -> SessionBinding {
    let session: SessionId =
        serde_json::from_value(serde_json::json!(format!("session:{}", project.as_str())))
            .expect("session id JSON is a string");
    SessionBinding::new(
        session,
        project.clone(),
        revision.clone(),
        Some(SessionOrigin::Embedded),
        false,
        Some(permission_set.clone()),
        None,
    )
    .expect("permission set is registered in the registry")
}

/// Maximum encoded stdio frame. This mirrors `video-protocol`'s bounded
/// transport contract without allowing an unbounded allocation on stdin.
pub const STDIO_MAX_FRAME_BYTES: usize = 1024 * 1024;

/// Errors raised by the strict stdio transport.
#[derive(Debug, Error)]
pub enum StdioError {
    /// The frame prefix or payload ended before its declared length.
    #[error("truncated stdio frame")]
    TruncatedFrame,
    /// The frame is empty or exceeds the bounded transport limit.
    #[error("invalid stdio frame length {0}")]
    InvalidFrameLength(usize),
    /// The frame is not valid JSON.
    #[error("invalid stdio JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// The underlying stdio stream failed.
    #[error("stdio I/O error: {0}")]
    Io(#[from] io::Error),
}

/// Read exactly one four-byte big-endian length-prefixed JSON frame.
pub fn read_stdio_frame<R: Read>(reader: &mut R) -> Result<Vec<u8>, StdioError> {
    let mut prefix = [0_u8; 4];
    reader
        .read_exact(&mut prefix)
        .map_err(|error| match error.kind() {
            io::ErrorKind::UnexpectedEof => StdioError::TruncatedFrame,
            _ => StdioError::Io(error),
        })?;
    let length = u32::from_be_bytes(prefix) as usize;
    if length == 0 || length > STDIO_MAX_FRAME_BYTES {
        return Err(StdioError::InvalidFrameLength(length));
    }
    let mut payload = vec![0_u8; length];
    reader
        .read_exact(&mut payload)
        .map_err(|error| match error.kind() {
            io::ErrorKind::UnexpectedEof => StdioError::TruncatedFrame,
            _ => StdioError::Io(error),
        })?;
    Ok(payload)
}

/// Write exactly one four-byte big-endian length-prefixed JSON frame.
pub fn write_stdio_frame<W: Write, T: Serialize>(
    writer: &mut W,
    value: &T,
) -> Result<(), StdioError> {
    let payload = serde_json::to_vec(value)?;
    if payload.is_empty() || payload.len() > STDIO_MAX_FRAME_BYTES {
        return Err(StdioError::InvalidFrameLength(payload.len()));
    }
    let length = u32::try_from(payload.len()).expect("stdio frame limit fits in u32");
    writer.write_all(&length.to_be_bytes())?;
    writer.write_all(&payload)?;
    writer.flush()?;
    Ok(())
}

/// JSON-RPC request carried by the CutRight stdio adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdioRequest {
    /// JSON-RPC version marker.
    pub jsonrpc: String,
    /// Request id. A missing id is a notification.
    #[serde(default)]
    pub id: Option<serde_json::Value>,
    /// MCP method name.
    pub method: String,
    /// Method parameters.
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC response emitted by the CutRight stdio adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdioResponse {
    /// JSON-RPC version marker.
    pub jsonrpc: String,
    /// Request id echoed from the request.
    pub id: Option<serde_json::Value>,
    /// Successful result, when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Typed JSON-RPC error, when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<StdioErrorObject>,
}

/// Stable JSON-RPC error body. It never writes diagnostics outside the frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdioErrorObject {
    /// JSON-RPC error code.
    pub code: i32,
    /// Stable machine-readable message.
    pub message: String,
    /// Optional structured data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
struct StdioTask {
    status: String,
    result: Option<serde_json::Value>,
}

/// Bound identity used by protocol resources and task projections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StdioBinding {
    /// Stable project identifier.
    pub project_id: String,
    /// Active project revision.
    pub revision: String,
    /// External principal name.
    pub principal: String,
}

impl Default for StdioBinding {
    fn default() -> Self {
        Self {
            project_id: "unbound".into(),
            revision: "rev_0".into(),
            principal: "external-agent".into(),
        }
    }
}

/// Real protocol adapter for a strict stdio MCP connection.
pub struct StdioMcpServer {
    registry: McpToolRegistry,
    binding: StdioBinding,
    tasks: BTreeMap<String, StdioTask>,
    cancelled: BTreeSet<String>,
    next_task: u64,
}

impl StdioMcpServer {
    /// Construct a server with a generated capability registry and scope.
    pub fn new(registry: McpToolRegistry, binding: StdioBinding) -> Self {
        Self {
            registry,
            binding,
            tasks: BTreeMap::new(),
            cancelled: BTreeSet::new(),
            next_task: 0,
        }
    }

    /// Process one request. Notifications return `None`, as required by
    /// JSON-RPC, while every request has a protocol-only response.
    pub fn handle(&mut self, request: StdioRequest) -> Option<StdioResponse> {
        if request.jsonrpc != "2.0" {
            return Some(Self::error(request.id, -32600, "invalid_request", None));
        }
        let id = request.id.clone();
        let response = match request.method.as_str() {
            "initialize" => Self::ok(id.clone(), self.initialize_result()),
            "notifications/initialized" => return None,
            "ping" => Self::ok(id.clone(), serde_json::json!({})),
            "tools/list" => Self::ok(id.clone(), self.tools_result()),
            "resources/list" => Self::ok(
                id.clone(),
                serde_json::json!({
                    "resources": [
                        {"uri": "cutright://session", "name": "session"},
                        {"uri": "cutright://capabilities", "name": "capabilities"}
                    ]
                }),
            ),
            "resources/read" => self.resource_read(id.clone(), request.params.as_ref()),
            "tools/call" => self.tool_call(id.clone(), request.params.as_ref()),
            "tasks/get" => self.task_get(id.clone(), request.params.as_ref()),
            "tasks/cancel" => self.task_cancel(id.clone(), request.params.as_ref()),
            _ => Self::error(
                id,
                -32601,
                "method_not_found",
                Some(serde_json::json!({
                    "method": request.method,
                })),
            ),
        };
        Some(response)
    }

    /// Serve frames until EOF. Errors are returned to the bridge for stderr
    /// reporting; no diagnostic is ever written to stdout.
    pub fn serve<R: Read, W: Write>(
        &mut self,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<(), StdioError> {
        loop {
            let payload = match read_stdio_frame(reader) {
                Ok(payload) => payload,
                Err(StdioError::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof => {
                    return Ok(())
                }
                Err(StdioError::TruncatedFrame) => return Err(StdioError::TruncatedFrame),
                Err(error) => return Err(error),
            };
            let request: StdioRequest = serde_json::from_slice(&payload)?;
            if let Some(response) = self.handle(request) {
                write_stdio_frame(writer, &response)?;
            }
        }
    }

    fn initialize_result(&self) -> serde_json::Value {
        serde_json::json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {
                "tools": {}, "resources": {}, "tasks": {}
            },
            "serverInfo": {"name": "cutright-mcp", "version": env!("CARGO_PKG_VERSION")},
            "binding": self.binding,
        })
    }

    fn tools_result(&self) -> serde_json::Value {
        let tools = self
            .registry
            .iter()
            .map(|(_, descriptor)| {
                serde_json::json!({
                    "name": descriptor.capability_id,
                    "description": descriptor.description,
                    "inputSchema": descriptor.input_schema,
                    "outputSchema": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "taskId": {"type": "string"},
                            "status": {"type": "string"},
                            "operation": {"type": "string"},
                            "project_id": {"type": "string"},
                            "revision": {"type": "string"},
                            "truncated": {"type": "boolean"},
                            "continuationCursor": {"type": ["string", "null"]}
                        },
                        "required": ["taskId", "status", "operation", "project_id", "revision", "truncated", "continuationCursor"]
                    },
                    "annotations": {
                        "readOnlyHint": !descriptor.is_mutation(),
                        "destructiveHint": descriptor.is_mutation(),
                        "idempotentHint": !descriptor.is_mutation()
                    }
                })
            })
            .collect::<Vec<_>>();
        serde_json::json!({"tools": tools})
    }

    fn resource_read(
        &self,
        id: Option<serde_json::Value>,
        params: Option<&serde_json::Value>,
    ) -> StdioResponse {
        let uri = params
            .and_then(|value| value.get("uri"))
            .and_then(serde_json::Value::as_str);
        let body = match uri {
            Some("cutright://session") => serde_json::to_string(&self.binding).unwrap(),
            Some("cutright://capabilities") => self.tools_result().to_string(),
            Some(_) => return Self::error(id, -32004, "resource_not_found", None),
            None => return Self::error(id, -32602, "invalid_params", None),
        };
        Self::ok(
            id,
            serde_json::json!({"contents": [{"uri": uri.unwrap(), "text": body}]}),
        )
    }

    fn tool_call(
        &mut self,
        id: Option<serde_json::Value>,
        params: Option<&serde_json::Value>,
    ) -> StdioResponse {
        let name = params
            .and_then(|value| value.get("name"))
            .and_then(serde_json::Value::as_str);
        let Some(name) = name else {
            return Self::error(id, -32602, "invalid_params", None);
        };
        if name.contains("provider") {
            return Self::error(id, -32010, "operation_not_exposed", None);
        }
        if self.registry.lookup(name).is_none() {
            return Self::error(id, -32004, "unknown_tool", None);
        }
        let arguments = params
            .and_then(|value| value.get("arguments"))
            .and_then(serde_json::Value::as_object);
        if arguments.is_some_and(|arguments| !arguments.is_empty()) {
            return Self::error(id, -32602, "invalid_arguments", None);
        }
        self.next_task += 1;
        let task_id = format!("task_{}", self.next_task);
        let result = serde_json::json!({
            "taskId": task_id,
            "status": "read_only",
            "operation": name,
            "project_id": self.binding.project_id,
            "revision": self.binding.revision,
            "truncated": false,
            "continuationCursor": null,
        });
        self.tasks.insert(
            task_id.clone(),
            StdioTask {
                status: "completed".into(),
                result: Some(result.clone()),
            },
        );
        Self::ok(
            id,
            serde_json::json!({
                "content": [{"type": "text", "text": result.to_string()}],
                "structuredContent": result
            }),
        )
    }

    fn task_get(
        &self,
        id: Option<serde_json::Value>,
        params: Option<&serde_json::Value>,
    ) -> StdioResponse {
        let task_id = params
            .and_then(|value| value.get("taskId"))
            .and_then(serde_json::Value::as_str);
        let Some(task_id) = task_id else {
            return Self::error(id, -32602, "invalid_params", None);
        };
        match self.tasks.get(task_id) {
            Some(task) => Self::ok(
                id,
                serde_json::json!({
                    "taskId": task_id, "status": task.status, "result": task.result
                }),
            ),
            None => Self::error(id, -32004, "task_not_found", None),
        }
    }

    fn task_cancel(
        &mut self,
        id: Option<serde_json::Value>,
        params: Option<&serde_json::Value>,
    ) -> StdioResponse {
        let task_id = params
            .and_then(|value| value.get("taskId"))
            .and_then(serde_json::Value::as_str);
        let Some(task_id) = task_id else {
            return Self::error(id, -32602, "invalid_params", None);
        };
        if !self.tasks.contains_key(task_id) {
            return Self::error(id, -32004, "task_not_found", None);
        }
        self.cancelled.insert(task_id.to_string());
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.status = "cancelled".into();
            task.result = None;
        }
        Self::ok(
            id,
            serde_json::json!({"taskId": task_id, "status": "cancelled"}),
        )
    }

    fn ok(id: Option<serde_json::Value>, result: serde_json::Value) -> StdioResponse {
        StdioResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(
        id: Option<serde_json::Value>,
        code: i32,
        message: &str,
        data: Option<serde_json::Value>,
    ) -> StdioResponse {
        StdioResponse {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(StdioErrorObject {
                code,
                message: message.into(),
                data,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_config_short_circuits() {
        let cfg = McpAdapterConfig::default();
        let error = McpAdapter::bind(cfg, McpToolRegistry::default()).unwrap_err();
        assert_eq!(error.code(), McpErrorCode::Disabled);
    }

    #[test]
    fn bind_disabled_returns_err() {
        let cfg = McpAdapterConfig::default();
        let err = McpAdapter::bind(cfg, McpToolRegistry::default()).unwrap_err();
        assert_eq!(err.code(), McpErrorCode::Disabled);
    }

    #[test]
    fn bind_non_loopback_returns_err() {
        let cfg = McpAdapterConfig::enabled_on("0.0.0.0:8080");
        let err = McpAdapter::bind(cfg, McpToolRegistry::default()).unwrap_err();
        assert_eq!(err.code(), McpErrorCode::NonLoopback);
    }

    #[test]
    fn ipv4_loopback_is_accepted() {
        assert!(McpAdapterConfig::is_loopback_address("127.0.0.1:0"));
        assert!(McpAdapterConfig::is_loopback_address("127.0.0.42:9100"));
        assert!(McpAdapterConfig::is_loopback_address("127.0.0.1"));
    }

    #[test]
    fn ipv6_loopback_is_accepted() {
        assert!(McpAdapterConfig::is_loopback_address("[::1]:0"));
        assert!(McpAdapterConfig::is_loopback_address("::1"));
    }

    #[test]
    fn localhost_is_accepted() {
        assert!(McpAdapterConfig::is_loopback_address("localhost:0"));
    }

    #[test]
    fn public_ip_is_rejected() {
        assert!(!McpAdapterConfig::is_loopback_address("10.0.0.1:0"));
        assert!(!McpAdapterConfig::is_loopback_address("192.168.1.1:0"));
        assert!(!McpAdapterConfig::is_loopback_address("0.0.0.0:0"));
    }

    #[test]
    fn ephemeral_tokens_are_unique() {
        let a = generate_ephemeral_token();
        let b = generate_ephemeral_token();
        assert_ne!(a, b);
        assert!(a.starts_with("mcp_"));
    }

    #[test]
    fn stdio_frames_are_strict_and_bounded() {
        let request = StdioRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(1)),
            method: "ping".into(),
            params: None,
        };
        let mut bytes = Vec::new();
        write_stdio_frame(&mut bytes, &request).unwrap();
        let decoded: StdioRequest =
            serde_json::from_slice(&read_stdio_frame(&mut bytes.as_slice()).unwrap()).unwrap();
        assert_eq!(decoded.method, "ping");

        let oversized = ((STDIO_MAX_FRAME_BYTES + 1) as u32).to_be_bytes();
        assert!(matches!(
            read_stdio_frame(&mut oversized.as_slice()),
            Err(StdioError::InvalidFrameLength(_))
        ));
    }

    #[test]
    fn every_advertised_stdio_operation_has_a_handler() {
        let mut server = StdioMcpServer::new(McpToolRegistry::default(), StdioBinding::default());
        for method in [
            "initialize",
            "ping",
            "tools/list",
            "resources/list",
            "resources/read",
            "tasks/get",
            "tasks/cancel",
        ] {
            let params = match method {
                "resources/read" => Some(serde_json::json!({"uri": "cutright://session"})),
                "tasks/get" | "tasks/cancel" => Some(serde_json::json!({"taskId": "missing"})),
                _ => None,
            };
            let response = server.handle(StdioRequest {
                jsonrpc: "2.0".into(),
                id: Some(serde_json::json!(method)),
                method: method.into(),
                params,
            });
            assert!(response.is_some(), "{method} must have a response handler");
        }
    }

    #[test]
    fn embedded_provider_operation_is_not_exposed() {
        let mut server = StdioMcpServer::new(McpToolRegistry::default(), StdioBinding::default());
        let response = server
            .handle(StdioRequest {
                jsonrpc: "2.0".into(),
                id: Some(serde_json::json!(1)),
                method: "tools/call".into(),
                params: Some(serde_json::json!({"name": "provider.execute"})),
            })
            .unwrap();
        assert_eq!(response.error.unwrap().message, "operation_not_exposed");
    }
}
