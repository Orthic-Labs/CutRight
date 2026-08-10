//! Bounded, JSON-backed transport contracts for CutRight process communication.
//!
//! This crate deliberately contains no project, capability, operation, or
//! persistence model. It owns only the bytes and envelopes crossing a process
//! boundary. Frames use a four-byte big-endian length prefix and are rejected
//! before their payload is handed to a JSON decoder.

#![deny(unsafe_code)]

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvError, SendError, SyncSender, TrySendError};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// Protocol major version. A major mismatch is never negotiated silently.
pub const PROTOCOL_MAJOR: u16 = 1;
/// Protocol minor version supported by this crate.
pub const PROTOCOL_MINOR: u16 = 0;
/// Maximum encoded payload accepted in one frame.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
/// Maximum number of queued messages for a bounded transport channel.
pub const DEFAULT_QUEUE_CAPACITY: usize = 64;

/// Stable identifier used to correlate requests, results, and events.
pub type RequestId = String;

/// A transport-level failure. Payload decoding happens only after framing has
/// accepted the declared size.
#[derive(Debug, Error)]
pub enum ProtocolError {
    /// The four-byte length prefix is incomplete.
    #[error("truncated frame length prefix")]
    TruncatedLength,
    /// The frame length exceeds [`MAX_FRAME_BYTES`].
    #[error("frame is {actual} bytes, maximum is {maximum}")]
    OversizedFrame { actual: usize, maximum: usize },
    /// A frame declared zero bytes, which is not a protocol message.
    #[error("empty frame is not valid")]
    EmptyFrame,
    /// The declared payload was not fully available.
    #[error("truncated frame payload: expected {expected} bytes, received {actual}")]
    TruncatedPayload { expected: usize, actual: usize },
    /// A frame contained bytes after its declared payload.
    #[error("trailing bytes after frame payload")]
    TrailingBytes,
    /// The payload was not valid JSON for its target contract.
    #[error("invalid message payload: {0}")]
    InvalidPayload(#[from] serde_json::Error),
    /// The underlying transport returned an I/O error.
    #[error("transport I/O error: {0}")]
    Io(#[from] io::Error),
    /// A handshake used an incompatible major protocol version.
    #[error("unsupported protocol major version {actual}; expected {expected}")]
    UnsupportedMajor { actual: u16, expected: u16 },
    /// A required string field was empty.
    #[error("{field} must not be empty")]
    EmptyField { field: &'static str },
    /// A bounded queue has been closed.
    #[error("transport queue is closed")]
    QueueClosed,
    /// A bounded queue is full and a non-blocking send was requested.
    #[error("transport queue is full")]
    QueueFull,
    /// A send was cancelled before it could enter the queue.
    #[error("transport send was cancelled")]
    Cancelled,
}

/// Encode an arbitrary serializable payload as one length-prefixed frame.
pub fn encode_frame<T: Serialize>(value: &T) -> Result<Vec<u8>, ProtocolError> {
    let payload = serde_json::to_vec(value)?;
    if payload.is_empty() {
        return Err(ProtocolError::EmptyFrame);
    }
    if payload.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::OversizedFrame {
            actual: payload.len(),
            maximum: MAX_FRAME_BYTES,
        });
    }
    let length = u32::try_from(payload.len()).expect("MAX_FRAME_BYTES fits in u32");
    let mut frame = Vec::with_capacity(payload.len() + 4);
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

/// Read exactly one length-prefixed frame from `reader`.
pub fn read_frame<R: Read>(reader: &mut R) -> Result<Vec<u8>, ProtocolError> {
    let mut prefix = [0_u8; 4];
    read_exact_or_truncated(reader, &mut prefix, ProtocolError::TruncatedLength)?;
    let declared = u32::from_be_bytes(prefix) as usize;
    validate_frame_length(declared)?;

    let mut payload = vec![0_u8; declared];
    read_exact_or_truncated(
        reader,
        &mut payload,
        ProtocolError::TruncatedPayload {
            expected: declared,
            actual: 0,
        },
    )?;
    Ok(payload)
}

/// Write one length-prefixed frame and flush it.
pub fn write_frame<W: Write, T: Serialize>(writer: &mut W, value: &T) -> Result<(), ProtocolError> {
    writer.write_all(&encode_frame(value)?)?;
    writer.flush().map_err(ProtocolError::Io)
}

/// Decode one complete message from a length-prefixed reader.
pub fn read_message<R: Read, T: DeserializeOwned>(reader: &mut R) -> Result<T, ProtocolError> {
    let payload = read_frame(reader)?;
    serde_json::from_slice(&payload).map_err(ProtocolError::InvalidPayload)
}

/// Validate a declared frame size without allocating or decoding it.
pub fn validate_frame_length(length: usize) -> Result<(), ProtocolError> {
    if length == 0 {
        return Err(ProtocolError::EmptyFrame);
    }
    if length > MAX_FRAME_BYTES {
        return Err(ProtocolError::OversizedFrame {
            actual: length,
            maximum: MAX_FRAME_BYTES,
        });
    }
    Ok(())
}

fn read_exact_or_truncated<R: Read>(
    reader: &mut R,
    buffer: &mut [u8],
    error: ProtocolError,
) -> Result<(), ProtocolError> {
    match reader.read_exact(buffer) {
        Ok(()) => Ok(()),
        Err(io_error) if io_error.kind() == io::ErrorKind::UnexpectedEof => Err(error),
        Err(io_error) => Err(ProtocolError::Io(io_error)),
    }
}

/// Principal presented by a client during handshake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientPrincipal {
    /// Stable local principal name, not a credential.
    pub name: String,
}

impl ClientPrincipal {
    /// Construct a principal, rejecting an empty name.
    pub fn new(name: impl Into<String>) -> Result<Self, ProtocolError> {
        let name = name.into();
        if name.is_empty() {
            return Err(ProtocolError::EmptyField {
                field: "principal name",
            });
        }
        Ok(Self { name })
    }
}

/// Project and revision scope attached to an authenticated session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectScope {
    /// Opaque project identifier.
    pub project_id: String,
    /// Revision visible to this session.
    pub revision: u64,
}

impl ProjectScope {
    /// Construct a project scope, rejecting an empty identifier.
    pub fn new(project_id: impl Into<String>, revision: u64) -> Result<Self, ProtocolError> {
        let project_id = project_id.into();
        if project_id.is_empty() {
            return Err(ProtocolError::EmptyField {
                field: "project id",
            });
        }
        Ok(Self {
            project_id,
            revision,
        })
    }
}

/// Client-to-daemon handshake binding the session to a protocol and scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handshake {
    /// Supported protocol major.
    pub protocol_major: u16,
    /// Supported protocol minor.
    pub protocol_minor: u16,
    /// Daemon instance identifier.
    pub daemon_instance_id: String,
    /// Per-instance nonce, represented as an opaque string.
    pub instance_nonce: String,
    /// Authenticated local principal.
    pub client_principal: ClientPrincipal,
    /// Optional project/revision scope.
    pub project_scope: Option<ProjectScope>,
    /// Feature names negotiated by both peers.
    pub features: Vec<String>,
}

impl Handshake {
    /// Validate version and required identity fields.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.protocol_major != PROTOCOL_MAJOR {
            return Err(ProtocolError::UnsupportedMajor {
                actual: self.protocol_major,
                expected: PROTOCOL_MAJOR,
            });
        }
        for (field, value) in [
            ("daemon instance id", &self.daemon_instance_id),
            ("instance nonce", &self.instance_nonce),
        ] {
            if value.is_empty() {
                return Err(ProtocolError::EmptyField { field });
            }
        }
        if self.client_principal.name.is_empty() {
            return Err(ProtocolError::EmptyField {
                field: "principal name",
            });
        }
        Ok(())
    }
}

/// Generic work item carried over the transport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Task {
    /// Request a named service operation with opaque JSON input.
    Execute {
        /// Stable operation reference owned by the service layer.
        operation: String,
        /// Operation input, interpreted outside this crate.
        input: serde_json::Value,
    },
    /// Cancel an earlier request.
    Cancel {
        /// Request being cancelled.
        request_id: RequestId,
    },
}

/// A request envelope with explicit correlation and cancellation semantics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskRequest {
    /// Correlation identifier for this request.
    pub request_id: RequestId,
    /// Work to perform.
    pub task: Task,
}

/// Terminal state of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultStatus {
    /// Task completed successfully.
    Succeeded,
    /// Task failed with a typed transport-visible error.
    Failed,
    /// Task was cancelled before completion.
    Cancelled,
}

/// Result envelope correlated to one [`TaskRequest`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskResult {
    /// Correlation identifier from the request.
    pub request_id: RequestId,
    /// Terminal result state.
    pub status: ResultStatus,
    /// Opaque output for successful tasks.
    pub output: Option<serde_json::Value>,
    /// Bounded, human-readable failure text.
    pub error: Option<String>,
}

/// Kind of progress event emitted while a task is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    /// Task was accepted for execution.
    Accepted,
    /// Task made observable progress.
    Progress,
    /// Task cancellation was observed.
    CancellationRequested,
}

/// Ordered, correlated task event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskEvent {
    /// Correlation identifier from the request.
    pub request_id: RequestId,
    /// Monotonic sequence within this request.
    pub sequence: u64,
    /// Event classification.
    pub kind: EventKind,
    /// Opaque bounded event data.
    pub data: serde_json::Value,
}

/// Provider-session message carried by the daemon without exposing provider
/// credentials or provider-specific domain models to the protocol crate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderSessionMessage {
    /// Request correlation identifier.
    pub request_id: RequestId,
    /// Opaque provider/session name.
    pub provider: String,
    /// Provider action name.
    pub action: String,
    /// Opaque action payload.
    pub payload: serde_json::Value,
}

/// Cancellation token shared by producers and consumers of bounded queues.
#[derive(Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl fmt::Debug for CancellationToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("CancellationToken")
            .field(&self.is_cancelled())
            .finish()
    }
}

impl CancellationToken {
    /// Create a token in the non-cancelled state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark this token cancelled. Cancellation is idempotent.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    /// Return whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// Sending half of a bounded transport queue.
#[derive(Debug)]
pub struct BoundedSender<T> {
    sender: SyncSender<T>,
}

/// Receiving half of a bounded transport queue.
#[derive(Debug)]
pub struct BoundedReceiver<T> {
    receiver: Receiver<T>,
}

/// Create a bounded queue. A zero capacity is rejected because it cannot
/// provide useful buffering or deterministic backpressure for a transport.
pub fn bounded_queue<T>(
    capacity: usize,
) -> Result<(BoundedSender<T>, BoundedReceiver<T>), ProtocolError> {
    if capacity == 0 {
        return Err(ProtocolError::EmptyField {
            field: "queue capacity",
        });
    }
    let (sender, receiver) = mpsc::sync_channel(capacity);
    Ok((BoundedSender { sender }, BoundedReceiver { receiver }))
}

impl<T> Clone for BoundedSender<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<T> BoundedSender<T> {
    /// Send while applying backpressure until the queue accepts the item.
    pub fn send(&self, item: T) -> Result<(), ProtocolError> {
        self.sender
            .send(item)
            .map_err(|SendError(_)| ProtocolError::QueueClosed)
    }

    /// Send without waiting; a full queue is reported to the caller.
    pub fn try_send(&self, item: T) -> Result<(), ProtocolError> {
        match self.sender.try_send(item) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(ProtocolError::QueueFull),
            Err(TrySendError::Disconnected(_)) => Err(ProtocolError::QueueClosed),
        }
    }

    /// Send with bounded polling so cancellation cannot be stranded behind a
    /// full queue.
    pub fn send_cancellable(
        &self,
        mut item: T,
        cancellation: &CancellationToken,
    ) -> Result<(), ProtocolError> {
        loop {
            if cancellation.is_cancelled() {
                return Err(ProtocolError::Cancelled);
            }
            match self.sender.try_send(item) {
                Ok(()) => return Ok(()),
                Err(TrySendError::Full(returned)) => {
                    item = returned;
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(TrySendError::Disconnected(_)) => return Err(ProtocolError::QueueClosed),
            }
        }
    }
}

impl<T> BoundedReceiver<T> {
    /// Receive the next item, reporting a closed queue distinctly.
    pub fn recv(&self) -> Result<T, ProtocolError> {
        self.receiver
            .recv()
            .map_err(|RecvError| ProtocolError::QueueClosed)
    }

    /// Receive with a bounded wait, allowing a caller to observe cancellation.
    pub fn recv_cancellable(&self, cancellation: &CancellationToken) -> Result<T, ProtocolError> {
        loop {
            if cancellation.is_cancelled() {
                return Err(ProtocolError::Cancelled);
            }
            match self.receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(item) => return Ok(item),
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(ProtocolError::QueueClosed)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn malformed_payload_is_rejected_after_valid_framing() {
        let mut input = Cursor::new([0, 0, 0, 3, b'{', b'}', b'!']);
        let error = read_message::<_, Handshake>(&mut input).expect_err("invalid JSON must fail");
        assert!(matches!(error, ProtocolError::InvalidPayload(_)));
    }

    #[test]
    fn oversized_frame_is_rejected_before_payload_read_or_decode() {
        let declared = (MAX_FRAME_BYTES as u32 + 1).to_be_bytes();
        let mut input = Cursor::new(declared);
        let error =
            read_message::<_, Handshake>(&mut input).expect_err("oversized frame must fail");
        assert!(matches!(error, ProtocolError::OversizedFrame { .. }));
        assert_eq!(input.position(), 4, "payload was not read");
    }

    #[test]
    fn frame_round_trip_preserves_message() {
        let principal = ClientPrincipal::new("studio").unwrap();
        let message = Handshake {
            protocol_major: PROTOCOL_MAJOR,
            protocol_minor: PROTOCOL_MINOR,
            daemon_instance_id: "daemon-1".into(),
            instance_nonce: "nonce".into(),
            client_principal: principal,
            project_scope: Some(ProjectScope::new("project", 3).unwrap()),
            features: vec!["cancel".into()],
        };
        let frame = encode_frame(&message).unwrap();
        let decoded: Handshake = read_message(&mut Cursor::new(frame)).unwrap();
        assert_eq!(decoded, message);
        decoded.validate().unwrap();
    }

    #[test]
    fn full_queue_reports_backpressure_without_dropping_item() {
        let (sender, receiver) = bounded_queue(1).unwrap();
        sender.try_send(1).unwrap();
        assert!(matches!(sender.try_send(2), Err(ProtocolError::QueueFull)));
        assert_eq!(receiver.recv().unwrap(), 1);
        sender.try_send(2).unwrap();
        assert_eq!(receiver.recv().unwrap(), 2);
    }

    #[test]
    fn cancellation_stops_cancellable_send() {
        let (sender, _receiver) = bounded_queue(1).unwrap();
        sender.try_send(1).unwrap();
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        assert!(matches!(
            sender.send_cancellable(2, &cancellation),
            Err(ProtocolError::Cancelled)
        ));
    }
}
