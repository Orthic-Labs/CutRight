//! `cutrightd` process host.
//!
//! The daemon owns only the process boundary. Project state, jobs, leases, and
//! operation execution stay behind the service layer and never enter this
//! crate's filesystem path.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use thiserror::Error;
use video_protocol::{read_message, Handshake, ProtocolError, PROTOCOL_MAJOR};

const SOCKET_DIR_MODE: u32 = 0o700;
const SOCKET_MODE: u32 = 0o600;
const OWNER_MARKER: &[u8] = b"cutrightd.endpoint/v1\n";

/// Daemon setup and authentication failures.
#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("cutrightd is supported only on macOS")]
    UnsupportedPlatform,
    #[error("daemon I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("daemon protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("socket endpoint exists but is not a stale CutRight endpoint")]
    EndpointNotOwned,
    #[error("another cutrightd already owns the socket endpoint")]
    AlreadyRunning,
    #[error("same-user peer credential check failed")]
    PeerCredentials,
    #[error("peer uid {actual} does not match daemon uid {expected}")]
    PeerMismatch { actual: u32, expected: u32 },
    #[error("handshake daemon instance mismatch")]
    InstanceMismatch,
    #[error("handshake nonce mismatch")]
    NonceMismatch,
    #[error("handshake principal is empty")]
    EmptyPrincipal,
    #[error("handshake feature set mismatch")]
    FeatureMismatch,
}

/// Inputs bound into every authenticated client connection.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    socket_path: PathBuf,
    daemon_instance_id: String,
    instance_nonce: String,
    features: Vec<String>,
}

impl DaemonConfig {
    /// Build configuration with an OS-random per-instance nonce.
    pub fn new(
        socket_path: impl Into<PathBuf>,
        daemon_instance_id: impl Into<String>,
        features: Vec<String>,
    ) -> Result<Self, DaemonError> {
        Ok(Self {
            socket_path: socket_path.into(),
            daemon_instance_id: non_empty(daemon_instance_id.into(), "daemon instance id")?,
            instance_nonce: random_nonce()?,
            features,
        })
    }

    /// Deterministic constructor for protocol tests; production code should
    /// use [`DaemonConfig::new`].
    pub fn with_nonce(
        socket_path: impl Into<PathBuf>,
        daemon_instance_id: impl Into<String>,
        instance_nonce: impl Into<String>,
        features: Vec<String>,
    ) -> Result<Self, DaemonError> {
        Ok(Self {
            socket_path: socket_path.into(),
            daemon_instance_id: non_empty(daemon_instance_id.into(), "daemon instance id")?,
            instance_nonce: non_empty(instance_nonce.into(), "instance nonce")?,
            features,
        })
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub fn instance_nonce(&self) -> &str {
        &self.instance_nonce
    }

    pub fn features(&self) -> &[String] {
        &self.features
    }
}

/// A bound `cutrightd` endpoint.
#[derive(Debug)]
pub struct Daemon {
    listener: UnixListener,
    config: DaemonConfig,
    marker_path: PathBuf,
}

impl Daemon {
    /// Bind the owner-only per-user Unix socket, replacing only a stale
    /// endpoint carrying CutRight's ownership marker.
    pub fn bind(config: DaemonConfig) -> Result<Self, DaemonError> {
        let parent = config
            .socket_path
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "socket has no parent"))?;
        fs::create_dir_all(parent)?;
        fs::set_permissions(parent, fs::Permissions::from_mode(SOCKET_DIR_MODE))?;
        let marker_path = marker_path(&config.socket_path)?;
        prepare_endpoint(&config.socket_path, &marker_path)?;
        let listener = UnixListener::bind(&config.socket_path).map_err(|error| {
            if error.kind() == io::ErrorKind::AddrInUse {
                DaemonError::AlreadyRunning
            } else {
                DaemonError::Io(error)
            }
        })?;
        fs::set_permissions(&config.socket_path, fs::Permissions::from_mode(SOCKET_MODE))?;
        write_marker(&marker_path)?;
        Ok(Self {
            listener,
            config,
            marker_path,
        })
    }

    /// Accept one connection and reject unauthenticated peers before a task
    /// or session can be decoded or created.
    pub fn accept_authenticated(&self) -> Result<AuthenticatedConnection, DaemonError> {
        let (mut stream, _) = self.listener.accept()?;
        let uid = peer_uid(&stream)?;
        let expected_uid = unsafe { libc::geteuid() } as u32;
        if uid != expected_uid {
            return Err(DaemonError::PeerMismatch {
                actual: uid,
                expected: expected_uid,
            });
        }
        let handshake: Handshake = read_message(&mut stream)?;
        handshake.validate()?;
        if handshake.protocol_major != PROTOCOL_MAJOR {
            return Err(DaemonError::Protocol(ProtocolError::UnsupportedMajor {
                actual: handshake.protocol_major,
                expected: PROTOCOL_MAJOR,
            }));
        }
        if handshake.daemon_instance_id != self.config.daemon_instance_id {
            return Err(DaemonError::InstanceMismatch);
        }
        if handshake.instance_nonce != self.config.instance_nonce {
            return Err(DaemonError::NonceMismatch);
        }
        if handshake.client_principal.name.is_empty() {
            return Err(DaemonError::EmptyPrincipal);
        }
        if handshake.features != self.config.features {
            return Err(DaemonError::FeatureMismatch);
        }
        Ok(AuthenticatedConnection {
            stream,
            handshake,
            uid,
        })
    }

    pub fn config(&self) -> &DaemonConfig {
        &self.config
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.config.socket_path);
        let _ = fs::remove_file(&self.marker_path);
    }
}

/// A connection that passed peer credentials and the complete daemon
/// handshake. No project, lease, or job is created by authentication.
#[derive(Debug)]
pub struct AuthenticatedConnection {
    stream: UnixStream,
    handshake: Handshake,
    uid: u32,
}

impl AuthenticatedConnection {
    pub fn handshake(&self) -> &Handshake {
        &self.handshake
    }

    pub fn uid(&self) -> u32 {
        self.uid
    }

    pub fn stream(&mut self) -> &mut UnixStream {
        &mut self.stream
    }
}

fn non_empty(value: String, field: &'static str) -> Result<String, DaemonError> {
    if value.is_empty() {
        return Err(DaemonError::Protocol(ProtocolError::EmptyField { field }));
    }
    Ok(value)
}

fn marker_path(socket_path: &Path) -> Result<PathBuf, DaemonError> {
    let file_name = socket_path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "socket has no filename"))?
        .to_string_lossy();
    Ok(socket_path.with_file_name(format!(".{file_name}.cutright-owner")))
}

fn prepare_endpoint(socket_path: &Path, marker_path: &Path) -> Result<(), DaemonError> {
    if !socket_path.exists() {
        if marker_path.exists() {
            fs::remove_file(marker_path)?;
        }
        return Ok(());
    }
    if UnixStream::connect(socket_path).is_ok() {
        return Err(DaemonError::AlreadyRunning);
    }
    if fs::read(marker_path).ok().as_deref() != Some(OWNER_MARKER) {
        return Err(DaemonError::EndpointNotOwned);
    }
    fs::remove_file(socket_path)?;
    fs::remove_file(marker_path)?;
    Ok(())
}

fn write_marker(path: &Path) -> Result<(), DaemonError> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(OWNER_MARKER)?;
    file.sync_all()?;
    Ok(())
}

fn random_nonce() -> Result<String, DaemonError> {
    let mut bytes = [0_u8; 32];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn peer_uid(stream: &UnixStream) -> Result<u32, DaemonError> {
    let mut euid = 0;
    let mut egid = 0;
    let result = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut euid, &mut egid) };
    if result != 0 {
        return Err(DaemonError::PeerCredentials);
    }
    Ok(euid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixStream;
    use std::thread;
    use tempfile::TempDir;
    use video_protocol::{write_frame, ClientPrincipal, Handshake, PROTOCOL_MINOR};

    fn config(dir: &TempDir) -> DaemonConfig {
        DaemonConfig::with_nonce(
            dir.path().join("cutrightd.sock"),
            "daemon-test",
            "nonce-test",
            vec!["cancel".into()],
        )
        .unwrap()
    }

    fn handshake(config: &DaemonConfig, nonce: &str) -> Handshake {
        Handshake {
            protocol_major: PROTOCOL_MAJOR,
            protocol_minor: PROTOCOL_MINOR,
            daemon_instance_id: config.daemon_instance_id.clone(),
            instance_nonce: nonce.into(),
            client_principal: ClientPrincipal::new("test-client").unwrap(),
            project_scope: None,
            features: config.features.clone(),
        }
    }

    #[test]
    fn endpoint_is_owner_only_and_authenticates_same_user() {
        let dir = tempfile::tempdir().unwrap();
        let daemon = Daemon::bind(config(&dir)).unwrap();
        let mut client = UnixStream::connect(daemon.config().socket_path()).unwrap();
        let expected = daemon.config().clone();
        write_frame(
            &mut client,
            &handshake(&expected, expected.instance_nonce()),
        )
        .unwrap();
        let connection = daemon.accept_authenticated().unwrap();
        assert_eq!(connection.handshake().daemon_instance_id, "daemon-test");
        assert_eq!(
            fs::metadata(dir.path()).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(daemon.config().socket_path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    #[test]
    fn nonce_mismatch_is_rejected_before_authenticated_connection() {
        let dir = tempfile::tempdir().unwrap();
        let daemon = Daemon::bind(config(&dir)).unwrap();
        let config = daemon.config().clone();
        let socket = config.socket_path().to_path_buf();
        let sender = thread::spawn(move || {
            let mut client = UnixStream::connect(socket).unwrap();
            write_frame(&mut client, &handshake(&config, "wrong-nonce")).unwrap();
        });
        let error = daemon.accept_authenticated().unwrap_err();
        sender.join().unwrap();
        assert!(matches!(error, DaemonError::NonceMismatch));
    }

    #[test]
    fn unowned_stale_endpoint_is_never_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let config = config(&dir);
        let stale = UnixListener::bind(config.socket_path()).unwrap();
        drop(stale);
        let error = Daemon::bind(config).unwrap_err();
        assert!(matches!(error, DaemonError::EndpointNotOwned));
    }
}
