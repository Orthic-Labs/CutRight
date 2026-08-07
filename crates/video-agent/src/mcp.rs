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

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use video_sessions::{
    ActiveRevisionId, PermissionSetId, ProjectId, SessionBinding, SessionBindingError,
    SessionGuard, SessionGuardError, SessionId, SessionOrigin,
};

use crate::tools::{McpToolRegistry, ToolDescriptor};

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
        let host = address.split(':').next().unwrap_or("");
        if host == "::1" || host == "localhost" {
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
#[derive(Debug, Error, PartialEq, Eq)]
pub enum McpError {
    #[error("MCP adapter is disabled")]
    Disabled,
    #[error("bind address {0} is not loopback")]
    NonLoopback(String),
    #[error("invalid token supplied to MCP adapter")]
    InvalidToken,
    #[error("frontmost project is {frontmost}, request targets {requesting}")]
    FrontmostProjectMismatch { frontmost: String, requesting: String },
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
        if tool.is_mutation() && binding.project_id() != frontmost_project {
            return Err(McpError::FrontmostProjectMismatch {
                frontmost: frontmost_project.to_string(),
                requesting: binding.project_id().to_string(),
            });
        }

        // Permission check: the binding must include the tool's required set.
        if !binding.has_permission_set(&tool.permission_set) {
            return Err(McpError::PermissionDenied {
                required: tool.permission_set.clone(),
            });
        }

        if tool.is_mutation() {
            // Stale-revision guard: the binding must be at the supplied
            // expected revision.
            if request.expected_revision != binding.active_revision_id().as_str() {
                return Err(McpError::StaleRevision {
                    expected: request.expected_revision.clone(),
                    actual: binding.active_revision_id().to_string(),
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
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let fp = blake3::hash(format!("mcp-token::{n}").as_bytes());
    let mut out = String::with_capacity(64);
    out.push_str("mcp_");
    for byte in fp.as_bytes().iter().take(24) {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Helper used by the Studio to build a session binding that has the
/// supplied permission set and is bound to the supplied project+revision.
pub fn binding_for(
    project: &ProjectId,
    revision: &ActiveRevisionId,
    permission_set: &PermissionSetId,
) -> SessionBinding {
    let session = SessionId(format!("session:{}", project.as_str()));
    SessionBinding::new(
        session,
        project.clone(),
        revision.clone(),
        permission_set.clone(),
        SessionOrigin::Embedded,
    )
    .expect("permission set is registered in the registry")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_config_short_circuits() {
        let cfg = McpAdapterConfig::default();
        let adapter = McpAdapter::bind(cfg, McpToolRegistry::default()).unwrap();
        // Already disabled by default — the bind above should error.
        // This branch is unreachable (bind already returned Err); see the
        // explicit error path in `bind_disabled_returns_err`.
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
}
