//! HeardRight client protocol behavior (hardening plan §9.2, §9.3).
//!
//! One supervised HeardRight engine session backs both the transcription and
//! file-VAD provider traits. This module owns:
//!
//! - engine sourcing (§9.3, v2 standalone boundary): the speech engine binary
//!   ships inside the signed CutRight speech runtime pack. Release code never
//!   resolves the engine through environment overrides, installed-location
//!   probing, or bare-name lookup; until the pack is materialized,
//!   [`discover_engine`] returns the typed
//!   [`ProviderError::RuntimePackNotInstalled`] degraded state, and a
//!   pack-provided engine path is constructed explicitly via
//!   [`HeardRightClient::with_engine`]. No hard-coded absolute path, no
//!   model-directory knowledge — HeardRight resolves its own models and
//!   runtime.
//! - a health/capability handshake performed once per session, before any
//!   transcription/VAD request;
//! - unique request and trace IDs per request, generated locally;
//! - exact response correlation — a result frame whose `request_id` does not
//!   match the outstanding request is a protocol error, never silently
//!   accepted;
//! - protocol-major rejection with minor-version negotiation;
//! - a per-request timeout (the engine itself may run indefinitely, but no
//!   single request blocks CutRight forever);
//! - bounded stderr capture (via [`video_core::process_runner::ManagedChild`]);
//! - exactly one controlled restart after an unexpected engine exit;
//! - no model download or network fallback of any kind.

use serde_json::Value;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use video_core::process_runner::{ManagedChild, ProcessSpec};

/// Protocol major version this client speaks. A session whose handshake
/// reports a different major version is rejected outright (§9.2).
pub const CLIENT_PROTOCOL_MAJOR: u64 = 1;
/// Highest minor version this client understands; negotiation picks
/// `min(client_minor, engine_minor)`.
pub const CLIENT_PROTOCOL_MINOR: u64 = 0;

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const STDERR_CAP_BYTES: usize = 64 * 1024;

fn env_duration_secs(var: &str, default: Duration) -> Duration {
    std::env::var(var)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(default)
}

/// Per-request timeout (§9.2, §10.1), overridable via
/// `CUTRIGHT_HEARDRIGHT_REQUEST_TIMEOUT_SECS` — mainly so tests can exercise
/// timeout/restart behavior without waiting on the real (120s) default.
pub(crate) fn request_timeout() -> Duration {
    env_duration_secs(
        "CUTRIGHT_HEARDRIGHT_REQUEST_TIMEOUT_SECS",
        DEFAULT_REQUEST_TIMEOUT,
    )
}

fn handshake_timeout() -> Duration {
    env_duration_secs(
        "CUTRIGHT_HEARDRIGHT_HANDSHAKE_TIMEOUT_SECS",
        DEFAULT_HANDSHAKE_TIMEOUT,
    )
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error(
        "HeardRight speech engine is unavailable: the signed CutRight speech runtime pack is not installed"
    )]
    RuntimePackNotInstalled,
    #[error("HeardRight engine could not start: {0}")]
    Start(#[source] std::io::Error),
    #[error("HeardRight engine request failed: {0}")]
    Engine(String),
    #[error("HeardRight returned invalid JSON: {0}")]
    Json(#[source] serde_json::Error),
    #[error("HeardRight returned transcript text without native timed words")]
    MissingTimedWords,
    #[error(
        "HeardRight protocol major version {engine_major} is incompatible with client major version {CLIENT_PROTOCOL_MAJOR}"
    )]
    ProtocolMajorMismatch { engine_major: u64 },
    #[error("HeardRight request {sent} timed out waiting for a response ({elapsed:?})")]
    RequestTimeout { sent: String, elapsed: Duration },
    #[error(
        "HeardRight response correlation failed: expected request_id {expected}, got {actual}"
    )]
    Correlation { expected: String, actual: String },
    #[error("HeardRight engine exited unexpectedly (stderr: {stderr})")]
    UnexpectedExit { stderr: String },
    #[error("WhisperX Python interpreter was not found: {0}")]
    WhisperXPythonMissing(String),
    #[error("WhisperX alignment script was not found; set CUTRIGHT_WHISPERX_SCRIPT")]
    WhisperXScriptMissing,
    #[error("WhisperX invocation failed: {0}")]
    WhisperXProcess(#[from] video_core::process_runner::ProcessRunError),
    #[error("WhisperX invocation exited with a nonzero status: {0}")]
    WhisperXExit(String),
}

/// Identity reported by the engine's handshake response, carried into every
/// provenance record so callers can see exactly which engine/model/protocol
/// produced a result (§9.2, §10.7).
#[derive(Debug, Clone, serde::Serialize)]
pub struct EngineIdentity {
    pub engine_version: String,
    pub protocol_major: u64,
    pub protocol_minor: u64,
    pub negotiated_minor: u64,
    pub capabilities: Vec<String>,
}

fn next_request_counter() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

fn unique_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!(
        "{prefix}-{}-{}-{}",
        std::process::id(),
        nanos,
        next_request_counter()
    )
}

/// Resolve the HeardRight engine location (§9.3, v2 standalone boundary).
///
/// The speech engine ships inside the signed CutRight speech runtime pack.
/// Release code never resolves it through environment overrides,
/// installed-location probing, or bare-name lookup: until the pack is
/// materialized this returns the typed
/// [`ProviderError::RuntimePackNotInstalled`] degraded state. The pack
/// installer constructs a session from the pack-provided engine path
/// explicitly via [`HeardRightClient::with_engine`].
pub fn discover_engine() -> Result<PathBuf, ProviderError> {
    Err(ProviderError::RuntimePackNotInstalled)
}

/// One supervised HeardRight engine session: spawn, handshake, and a
/// request/response protocol loop with per-request timeout, exact
/// correlation, bounded stderr, and a single controlled restart.
pub(crate) struct Session {
    process: ManagedChild,
    stdin: std::process::ChildStdin,
    line_rx: mpsc::Receiver<std::io::Result<String>>,
    pub identity: EngineIdentity,
}

impl Session {
    fn spawn(engine: &std::path::Path) -> Result<Self, ProviderError> {
        // HeardRight owns model discovery, runtime loading, and platform
        // backend choice. CutRight passes no model-directory paths; the
        // HR_ASR_BACKEND value is a policy hint, not an internal model
        // location. The environment is an explicit allow-list (§10.1): only
        // PATH (needed to resolve any dynamic libraries/tools HeardRight
        // itself shells out to) and the one policy hint are passed through.
        let mut env_allow = vec![("HR_ASR_BACKEND".to_string(), "parakeet-tdt".to_string())];
        if let Ok(path) = env::var("PATH") {
            env_allow.push(("PATH".to_string(), path));
        }
        let spec = ProcessSpec {
            executable: engine.to_path_buf(),
            args: Vec::new(),
            env_allow,
            working_dir: None,
            timeout: request_timeout(),
            stdout_cap_bytes: usize::MAX / 4,
            stderr_cap_bytes: STDERR_CAP_BYTES,
        };
        let (process, stdin, stdout) = ManagedChild::spawn(&spec).map_err(|error| match error {
            video_core::process_runner::ProcessRunError::Spawn(_, io_error) => {
                ProviderError::Start(io_error)
            }
            other => ProviderError::Engine(other.to_string()),
        })?;

        // Reader thread: continuously reads lines from the engine's stdout
        // and forwards them over a channel, so `request()` can wait with a
        // per-request timeout instead of blocking indefinitely on I/O.
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        let _ = tx.send(Ok(String::new()));
                        break;
                    }
                    Ok(_) => {
                        if tx.send(Ok(line)).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = tx.send(Err(error));
                        break;
                    }
                }
            }
        });

        let mut session = Self {
            process,
            stdin,
            line_rx: rx,
            identity: EngineIdentity {
                engine_version: String::new(),
                protocol_major: 0,
                protocol_minor: 0,
                negotiated_minor: 0,
                capabilities: Vec::new(),
            },
        };
        session.handshake()?;
        Ok(session)
    }

    /// Health/capability handshake, required before any transcription/VAD
    /// request is sent on a freshly spawned session (§9.2). Rejects a
    /// protocol-major mismatch outright and negotiates the minor version.
    fn handshake(&mut self) -> Result<(), ProviderError> {
        let request_id = unique_id("cutright-hs");
        let request = serde_json::json!({
            "protocol_major": CLIENT_PROTOCOL_MAJOR,
            "protocol_minor": CLIENT_PROTOCOL_MINOR,
            "schema_name": "session_handshake_request",
            "schema_version": 1,
            "engine_version": env!("CARGO_PKG_VERSION"),
            "request_id": request_id,
            "trace_id": unique_id("cutright-trace"),
            "payload": { "kind": "session_handshake_request" },
        });
        let frame = self.exchange(
            request,
            &request_id,
            "session_handshake_result",
            handshake_timeout(),
        )?;
        let engine_major = frame
            .get("protocol_major")
            .and_then(Value::as_u64)
            .unwrap_or(CLIENT_PROTOCOL_MAJOR);
        let engine_minor = frame
            .get("protocol_minor")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if engine_major != CLIENT_PROTOCOL_MAJOR {
            return Err(ProviderError::ProtocolMajorMismatch { engine_major });
        }
        let capabilities = frame
            .pointer("/payload/capabilities")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        // `min()` is the actual negotiation contract (§9.2 minor-version
        // negotiation); it degenerates to a constant only while
        // CLIENT_PROTOCOL_MINOR is still 0.
        #[allow(clippy::unnecessary_min_or_max)]
        let negotiated_minor = engine_minor.min(CLIENT_PROTOCOL_MINOR);
        self.identity = EngineIdentity {
            engine_version: frame
                .get("engine_version")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            protocol_major: engine_major,
            protocol_minor: engine_minor,
            negotiated_minor,
            capabilities,
        };
        Ok(())
    }

    /// Send one request frame and wait — bounded by `timeout` — for the
    /// frame whose `schema_name` matches `result_schema` AND whose
    /// `request_id` matches `request_id`. `engine_error` frames and a closed
    /// stdout surface as explicit errors. A response with a mismatched
    /// `request_id` is a correlation failure and is never silently accepted.
    fn exchange(
        &mut self,
        request: Value,
        request_id: &str,
        result_schema: &str,
        timeout: Duration,
    ) -> Result<Value, ProviderError> {
        self.stdin
            .write_all(format!("{request}\n").as_bytes())
            .map_err(ProviderError::Start)?;
        self.stdin.flush().map_err(ProviderError::Start)?;

        let deadline = std::time::Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(ProviderError::RequestTimeout {
                    sent: request_id.to_string(),
                    elapsed: timeout,
                });
            }
            match self.line_rx.recv_timeout(remaining) {
                Err(RecvTimeoutError::Timeout) => {
                    return Err(ProviderError::RequestTimeout {
                        sent: request_id.to_string(),
                        elapsed: timeout,
                    });
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(self.unexpected_exit_error());
                }
                Ok(Err(io_error)) => return Err(ProviderError::Start(io_error)),
                Ok(Ok(line)) if line.is_empty() => {
                    // Reader thread signalled EOF (engine closed stdout).
                    return Err(self.unexpected_exit_error());
                }
                Ok(Ok(line)) => {
                    let frame: Value =
                        serde_json::from_str(line.trim_end()).map_err(ProviderError::Json)?;
                    let schema = frame.get("schema_name").and_then(Value::as_str);
                    match schema {
                        Some(name) if name == result_schema => {
                            let actual_id = frame
                                .get("request_id")
                                .and_then(Value::as_str)
                                .unwrap_or("");
                            if actual_id != request_id {
                                return Err(ProviderError::Correlation {
                                    expected: request_id.to_string(),
                                    actual: actual_id.to_string(),
                                });
                            }
                            return Ok(frame);
                        }
                        Some("engine_error") => {
                            let actual_id = frame
                                .get("request_id")
                                .and_then(Value::as_str)
                                .unwrap_or("");
                            if !actual_id.is_empty() && actual_id != request_id {
                                return Err(ProviderError::Correlation {
                                    expected: request_id.to_string(),
                                    actual: actual_id.to_string(),
                                });
                            }
                            return Err(ProviderError::Engine(
                                frame
                                    .pointer("/error/message")
                                    .and_then(Value::as_str)
                                    .unwrap_or("unknown engine error")
                                    .to_string(),
                            ));
                        }
                        _ => {
                            // Unrecognized frame (e.g. an async log/progress
                            // frame); keep waiting for the correlated result.
                            continue;
                        }
                    }
                }
            }
        }
    }

    fn unexpected_exit_error(&self) -> ProviderError {
        let (stderr, truncated) = self.process.stderr_snapshot();
        let mut stderr_text = String::from_utf8_lossy(&stderr).trim().to_string();
        if truncated {
            stderr_text.push_str(" ...[stderr truncated]");
        }
        if stderr_text.is_empty() {
            stderr_text = "(no stderr captured)".to_string();
        }
        stderr_text.push_str(&format!(
            " [engine={}, session uptime={:?}]",
            self.process.label(),
            self.process.duration()
        ));
        ProviderError::UnexpectedExit {
            stderr: stderr_text,
        }
    }

    /// True if the engine process has already exited (detected proactively,
    /// before writing a new request, so a dead session never gets a doomed
    /// write attempt).
    fn is_dead(&mut self) -> bool {
        self.process.has_exited()
    }
}

/// One supervised HeardRight engine session, with lazy start, a handshake
/// gate, per-request timeout, exact response correlation, and exactly one
/// controlled restart after an unexpected engine exit.
pub struct HeardRightClient {
    engine: PathBuf,
    session: Mutex<Option<Session>>,
}

impl std::fmt::Debug for HeardRightClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeardRightClient")
            .field("engine", &self.engine)
            .finish()
    }
}

impl HeardRightClient {
    pub fn discover() -> Result<Self, ProviderError> {
        let engine = discover_engine()?;
        Ok(Self {
            engine,
            session: Mutex::new(None),
        })
    }

    /// Construct a client from an explicit engine binary path provided by
    /// the signed CutRight speech runtime pack (§9.3). This is the only
    /// engine-sourcing path in release code: there is no environment
    /// override and no bare-name resolution.
    pub fn with_engine(engine: PathBuf) -> Self {
        Self {
            engine,
            session: Mutex::new(None),
        }
    }

    /// Current negotiated engine identity, if a session has been started.
    pub fn identity(&self) -> Option<EngineIdentity> {
        self.session
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|s| s.identity.clone()))
    }

    /// Public, download-free health/capability probe (§11.3): ensures a
    /// session exists — spawning one performs only the protocol handshake
    /// (§9.2), never a transcription or VAD request — and returns the
    /// negotiated engine identity. Reuses an already-live session instead
    /// of spawning a redundant one.
    pub fn health(&self) -> Result<EngineIdentity, ProviderError> {
        let mut guard = self
            .session
            .lock()
            .map_err(|_| ProviderError::Engine("HeardRight session lock poisoned".into()))?;
        if guard.is_none() {
            *guard = Some(Session::spawn(&self.engine)?);
        } else if guard.as_mut().expect("checked above").is_dead() {
            if let Some(mut dead) = guard.take() {
                dead.process.kill_tree();
            }
            *guard = Some(Session::spawn(&self.engine)?);
        }
        Ok(guard
            .as_ref()
            .expect("session initialized above")
            .identity
            .clone())
    }

    /// Send one request and wait for the correlated result frame, applying
    /// the restart-once policy: if the current session is dead or the
    /// request fails because the engine exited unexpectedly, start exactly
    /// one fresh session (handshake included) and retry once. A second
    /// failure is returned to the caller without a further restart — no
    /// unbounded retry loop, ever.
    pub(crate) fn request(
        &self,
        request_id: &str,
        build_request: impl Fn(&str, &str) -> Value,
        result_schema: &str,
        timeout: Duration,
    ) -> Result<Value, ProviderError> {
        let mut guard = self
            .session
            .lock()
            .map_err(|_| ProviderError::Engine("HeardRight session lock poisoned".into()))?;

        if guard.is_none() {
            *guard = Some(Session::spawn(&self.engine)?);
        } else if guard.as_mut().expect("checked above").is_dead() {
            // Proactive detection: the engine already exited since the last
            // request (e.g. crashed while idle). Replace it before even
            // attempting a write, rather than waiting for that write or the
            // subsequent read to fail.
            if let Some(mut dead) = guard.take() {
                dead.process.kill_tree();
            }
            *guard = Some(Session::spawn(&self.engine)?);
        }

        let trace_id = unique_id("cutright-trace");
        let request = build_request(request_id, &trace_id);
        let first_attempt = guard.as_mut().expect("session initialized above").exchange(
            request.clone(),
            request_id,
            result_schema,
            timeout,
        );

        match first_attempt {
            Ok(frame) => Ok(frame),
            Err(ProviderError::UnexpectedExit { .. })
            | Err(ProviderError::RequestTimeout { .. }) => {
                // One controlled restart: tear down the dead/hung session and
                // try exactly once more on a fresh one.
                if let Some(mut dead) = guard.take() {
                    dead.process.kill_tree();
                }
                let mut fresh = Session::spawn(&self.engine)?;
                let retry_trace_id = unique_id("cutright-trace");
                let retry_request = build_request(request_id, &retry_trace_id);
                let result = fresh.exchange(retry_request, request_id, result_schema, timeout);
                *guard = Some(fresh);
                result
            }
            Err(other) => Err(other),
        }
    }
}
