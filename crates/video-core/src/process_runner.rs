//! Shared external-process abstraction (hardening plan §10.1).
//!
//! Every external command this crate spawns — the HeardRight engine session
//! and WhisperX today — goes through this module instead of raw
//! `std::process::Command` calls, so executable identity, environment,
//! timeout/kill-tree, output caps, exit reporting, cancellation, and duration
//! telemetry are handled uniformly. No command driven through this module can
//! wait indefinitely: a timeout is a mandatory field, not an option.

use std::collections::VecDeque;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Executable identity, argument list, environment, and policy for one
/// external process invocation. `args` is intentionally excluded from the
/// crate's `Debug`/error text: argument lists can carry source paths or
/// content and are kept out of logs (§10.1).
#[derive(Clone)]
pub struct ProcessSpec {
    pub executable: PathBuf,
    pub args: Vec<String>,
    /// Explicit environment allow-list. The child's environment is cleared
    /// first; only these (name, value) pairs are set.
    pub env_allow: Vec<(String, String)>,
    pub working_dir: Option<PathBuf>,
    /// Mandatory bound on how long a one-shot invocation may run.
    pub timeout: Duration,
    pub stdout_cap_bytes: usize,
    pub stderr_cap_bytes: usize,
}

impl std::fmt::Debug for ProcessSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessSpec")
            .field("executable", &self.executable)
            .field(
                "args",
                &format!("<{} argument(s), redacted>", self.args.len()),
            )
            .field(
                "env_allow_keys",
                &self
                    .env_allow
                    .iter()
                    .map(|(k, _)| k.as_str())
                    .collect::<Vec<_>>(),
            )
            .field("working_dir", &self.working_dir)
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl ProcessSpec {
    /// A label safe to put in error text and logs: the executable name only,
    /// never the argument list.
    pub fn label(&self) -> String {
        self.executable
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.executable.display().to_string())
    }
}

/// Structured result of one bounded, one-shot external command.
///
/// `stdout`/`stdout_truncated` are part of the general-purpose contract for
/// any future caller whose result is on stdout; today's only `run_process`
/// caller (WhisperX) writes its result to a temp file instead and reads
/// `stderr`/`exit_code`/`signal`/`duration` for error reporting.
#[derive(Debug, Clone, Default)]
pub struct ProcessOutcome {
    #[allow(dead_code)]
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    #[allow(dead_code)]
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub duration: Duration,
}

impl ProcessOutcome {
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessRunError {
    #[error("failed to start {0}: {1}")]
    Spawn(String, #[source] std::io::Error),
    #[error("{0} timed out after {1:?}")]
    Timeout(String, Duration),
    #[error("{0} was cancelled")]
    Cancelled(String),
}

/// Cooperative cancellation flag shared between a caller and a running
/// process. Checked on the run-process poll loop.
#[derive(Debug, Default, Clone)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// Requests cancellation of any in-flight `run_process` call sharing this
    /// token. Not yet called by this crate's own call sites (each
    /// `run_process` caller today owns a private, never-cancelled token and
    /// relies on the mandatory timeout instead) — exposed so a future
    /// cancel-in-flight caller (CLI Ctrl-C, a UI stop action) can share a
    /// token with a long-running command per §10.1.
    #[allow(dead_code)]
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Byte-capped ring buffer used to bound captured stdout/stderr.
struct CappedBuffer {
    data: VecDeque<u8>,
    cap: usize,
    truncated: bool,
}

impl CappedBuffer {
    fn new(cap: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(cap.min(64 * 1024)),
            cap,
            truncated: false,
        }
    }

    fn push(&mut self, chunk: &[u8]) {
        let remaining = self.cap.saturating_sub(self.data.len());
        let take = remaining.min(chunk.len());
        self.data.extend(&chunk[..take]);
        if take < chunk.len() {
            self.truncated = true;
        }
    }

    fn into_parts(self) -> (Vec<u8>, bool) {
        (self.data.into_iter().collect(), self.truncated)
    }
}

fn spawn_reader(
    mut reader: impl Read + Send + 'static,
    cap: usize,
) -> (thread::JoinHandle<()>, Arc<Mutex<CappedBuffer>>) {
    let buffer = Arc::new(Mutex::new(CappedBuffer::new(cap)));
    let buffer_for_thread = Arc::clone(&buffer);
    let handle = thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        loop {
            match reader.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    if let Ok(mut guard) = buffer_for_thread.lock() {
                        guard.push(&chunk[..n]);
                    }
                }
                Err(_) => break,
            }
        }
    });
    (handle, buffer)
}

#[cfg(unix)]
fn prepare_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    // SAFETY: `setsid` is async-signal-safe and is the only call made in the
    // pre-exec hook; it detaches the child into its own session/process
    // group so a timeout/cancel can signal the whole tree, not just the
    // immediate child (kill-tree, §10.1).
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
}

#[cfg(not(unix))]
fn prepare_process_group(_command: &mut Command) {}

#[cfg(unix)]
fn kill_tree(child: &mut Child) {
    let pid = child.id() as i32;
    // SAFETY: killpg with a negative pid targets the process group created
    // by `setsid` in `prepare_process_group`. Errors (already-exited group)
    // are ignored deliberately.
    unsafe {
        libc::killpg(pid, libc::SIGTERM);
    }
    thread::sleep(Duration::from_millis(300));
    if child.try_wait().ok().flatten().is_none() {
        unsafe {
            libc::killpg(pid, libc::SIGKILL);
        }
    }
    let _ = child.wait();
}

#[cfg(not(unix))]
fn kill_tree(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn exit_status_parts(status: std::process::ExitStatus) -> (Option<i32>, Option<i32>) {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        (status.code(), status.signal())
    }
    #[cfg(not(unix))]
    {
        (status.code(), None)
    }
}

/// Run one external command to completion under `spec`'s timeout, output
/// caps, and environment allow-list. Polls for exit rather than blocking
/// indefinitely; on timeout or cancellation the whole process tree is
/// signalled (kill-tree) and torn down before returning.
pub fn run_process(
    spec: &ProcessSpec,
    cancel: &CancellationToken,
) -> Result<ProcessOutcome, ProcessRunError> {
    let label = spec.label();
    let start = Instant::now();
    let mut command = Command::new(&spec.executable);
    command.args(&spec.args);
    command.env_clear();
    for (key, value) in &spec.env_allow {
        command.env(key, value);
    }
    if let Some(dir) = &spec.working_dir {
        command.current_dir(dir);
    }
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    prepare_process_group(&mut command);

    let mut child = command
        .spawn()
        .map_err(|error| ProcessRunError::Spawn(label.clone(), error))?;

    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let (stdout_handle, stdout_buffer) = spawn_reader(stdout, spec.stdout_cap_bytes);
    let (stderr_handle, stderr_buffer) = spawn_reader(stderr, spec.stderr_cap_bytes);

    let poll_interval = Duration::from_millis(20);
    let outcome_status = loop {
        if cancel.is_cancelled() {
            kill_tree(&mut child);
            return Err(ProcessRunError::Cancelled(label));
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if start.elapsed() >= spec.timeout {
                    kill_tree(&mut child);
                    return Err(ProcessRunError::Timeout(label, spec.timeout));
                }
                thread::sleep(poll_interval);
            }
            Err(error) => {
                kill_tree(&mut child);
                return Err(ProcessRunError::Spawn(label, error));
            }
        }
    };

    let _ = stdout_handle.join();
    let _ = stderr_handle.join();
    let (stdout_bytes, stdout_truncated) = Arc::try_unwrap(stdout_buffer)
        .map(|m| m.into_inner().expect("stdout buffer mutex").into_parts())
        .unwrap_or_default_parts();
    let (stderr_bytes, stderr_truncated) = Arc::try_unwrap(stderr_buffer)
        .map(|m| m.into_inner().expect("stderr buffer mutex").into_parts())
        .unwrap_or_default_parts();
    let (exit_code, signal) = exit_status_parts(outcome_status);

    Ok(ProcessOutcome {
        stdout: stdout_bytes,
        stderr: stderr_bytes,
        stdout_truncated,
        stderr_truncated,
        exit_code,
        signal,
        duration: start.elapsed(),
    })
}

/// Small helper trait so `Arc::try_unwrap(...).map(...)` above reads cleanly
/// even though the reader threads have already joined (so the `Arc` should
/// always have exactly one strong reference left).
trait UnwrapOrDefaultParts {
    fn unwrap_or_default_parts(self) -> (Vec<u8>, bool);
}

impl UnwrapOrDefaultParts for Result<(Vec<u8>, bool), Arc<Mutex<CappedBuffer>>> {
    fn unwrap_or_default_parts(self) -> (Vec<u8>, bool) {
        self.unwrap_or_default()
    }
}

/// A spawned long-lived child process (the HeardRight engine session) with
/// kill-tree teardown, bounded stderr capture, and duration telemetry. Unlike
/// [`run_process`], the caller drives stdin/stdout directly (a streaming
/// protocol), but process identity, environment allow-listing, working
/// directory, and teardown are still uniform.
pub struct ManagedChild {
    child: Child,
    label: String,
    started_at: Instant,
    stderr_buffer: Arc<Mutex<CappedBuffer>>,
    _stderr_handle: thread::JoinHandle<()>,
}

impl ManagedChild {
    /// Spawn `spec` as a long-lived child with piped stdin/stdout and a
    /// bounded, threaded stderr capture. Returns the child plus the raw
    /// stdin/stdout handles for the caller's protocol loop.
    pub fn spawn(spec: &ProcessSpec) -> Result<(Self, ChildStdin, ChildStdout), ProcessRunError> {
        let label = spec.label();
        let mut command = Command::new(&spec.executable);
        command.args(&spec.args);
        command.env_clear();
        for (key, value) in &spec.env_allow {
            command.env(key, value);
        }
        if let Some(dir) = &spec.working_dir {
            command.current_dir(dir);
        }
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        prepare_process_group(&mut command);

        let mut child = command
            .spawn()
            .map_err(|error| ProcessRunError::Spawn(label.clone(), error))?;
        let stdin = child.stdin.take().expect("stdin is piped");
        let stdout = child.stdout.take().expect("stdout is piped");
        let stderr: ChildStderr = child.stderr.take().expect("stderr is piped");
        let (stderr_handle, stderr_buffer) = spawn_reader(stderr, spec.stderr_cap_bytes);

        Ok((
            Self {
                child,
                label,
                started_at: Instant::now(),
                stderr_buffer,
                _stderr_handle: stderr_handle,
            },
            stdin,
            stdout,
        ))
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn duration(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Bounded snapshot of everything captured on stderr so far. Does not
    /// consume the buffer — safe to call repeatedly for error reporting.
    pub fn stderr_snapshot(&self) -> (Vec<u8>, bool) {
        match self.stderr_buffer.lock() {
            Ok(guard) => {
                let bytes: Vec<u8> = guard.data.iter().copied().collect();
                (bytes, guard.truncated)
            }
            Err(_) => (Vec::new(), false),
        }
    }

    /// True if the child has already exited (unexpected engine exit
    /// detection for §9.2's restart-once policy).
    pub fn has_exited(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }

    /// Kill-tree teardown: SIGTERM the whole process group, grace period,
    /// then SIGKILL if still alive.
    pub fn kill_tree(&mut self) {
        kill_tree(&mut self.child);
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            self.kill_tree();
        }
    }
}

/// Create a fresh, uniquely named temporary file path under the system temp
/// directory and remove it (best-effort) when the returned guard drops —
/// used by [`crate::whisperx`] so a killed/failed run never leaves stray
/// output files behind (§10.1 temp-file cleanup).
pub struct TempFileGuard {
    pub path: PathBuf,
}

impl TempFileGuard {
    pub fn new(prefix: &str, suffix: &str) -> Self {
        let unique = format!(
            "{prefix}-{}-{}{suffix}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        );
        Self {
            path: std::env::temp_dir().join(unique),
        }
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(executable: &str, args: &[&str], timeout: Duration) -> ProcessSpec {
        ProcessSpec {
            executable: PathBuf::from(executable),
            args: args.iter().map(|s| s.to_string()).collect(),
            env_allow: Vec::new(),
            working_dir: None,
            timeout,
            stdout_cap_bytes: 1024 * 1024,
            stderr_cap_bytes: 1024 * 1024,
        }
    }

    #[test]
    fn run_process_captures_stdout_and_exit_code() {
        let outcome = run_process(
            &spec("/bin/sh", &["-c", "echo hello"], Duration::from_secs(5)),
            &CancellationToken::new(),
        )
        .expect("run echo");
        assert!(outcome.success());
        assert_eq!(String::from_utf8_lossy(&outcome.stdout).trim(), "hello");
    }

    #[test]
    fn run_process_times_out_a_hanging_command() {
        let result = run_process(
            &spec("/bin/sh", &["-c", "sleep 5"], Duration::from_millis(200)),
            &CancellationToken::new(),
        );
        assert!(matches!(result, Err(ProcessRunError::Timeout(_, _))));
    }

    #[test]
    fn run_process_respects_cancellation() {
        let cancel = CancellationToken::new();
        let cancel_for_thread = cancel.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            cancel_for_thread.cancel();
        });
        let result = run_process(
            &spec("/bin/sh", &["-c", "sleep 5"], Duration::from_secs(5)),
            &cancel,
        );
        assert!(matches!(result, Err(ProcessRunError::Cancelled(_))));
    }

    #[test]
    fn run_process_truncates_oversized_stdout() {
        let outcome = run_process(
            &ProcessSpec {
                stdout_cap_bytes: 8,
                ..spec(
                    "/bin/sh",
                    &["-c", "printf '0123456789ABCDEF'"],
                    Duration::from_secs(5),
                )
            },
            &CancellationToken::new(),
        )
        .expect("run printf");
        assert!(outcome.stdout_truncated);
        assert_eq!(outcome.stdout.len(), 8);
    }

    #[test]
    fn process_spec_debug_never_includes_argument_values() {
        let spec = spec(
            "/bin/sh",
            &["-c", "echo secret-token-value"],
            Duration::from_secs(1),
        );
        let rendered = format!("{spec:?}");
        assert!(!rendered.contains("secret-token-value"));
    }
}
