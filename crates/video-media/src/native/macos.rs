use std::collections::HashSet;
use std::env;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::{json, Value};
use video_core::process_runner::{ManagedChild, ProcessSpec};

use super::protocol::{
    RequestEnvelope, ResponseEnvelope, MAC_MEDIA_PROTOCOL_VERSION, MAX_JSONL_LINE_BYTES,
};
use super::{
    AnalyzeFramesRequest, MacMediaBackend, MacMediaCapabilities, NativeAssetInfo,
    NativeAudioFeatures, NativeAudioRequest, NativeCaptionRequest, NativeFrameAnalysis,
    NativeMediaError, NativePreviewRequest, NativeRenderArtifact, NativeRequestContext,
    NativeTimelineRenderRequest, NativeTimelineRenderResult,
};

const STDERR_CAP_BYTES: usize = 64 * 1024;

struct Session {
    process: Arc<Mutex<ManagedChild>>,
    stdin: std::process::ChildStdin,
    lines: Receiver<std::io::Result<String>>,
}

impl Session {
    fn spawn(worker: &Path, args: &[String]) -> Result<Self, NativeMediaError> {
        let spec = ProcessSpec {
            executable: worker.to_path_buf(),
            args: args.to_vec(),
            env_allow: media_worker_env(),
            working_dir: None,
            timeout: std::time::Duration::from_secs(30),
            stdout_cap_bytes: MAX_JSONL_LINE_BYTES,
            stderr_cap_bytes: STDERR_CAP_BYTES,
        };
        let (process, stdin, stdout) = ManagedChild::spawn(&spec)
            .map_err(|error| NativeMediaError::Start(error.to_string()))?;
        let (tx, lines) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut bytes = Vec::with_capacity(4096);
                match (&mut reader)
                    .take((MAX_JSONL_LINE_BYTES + 2) as u64)
                    .read_until(b'\n', &mut bytes)
                {
                    Ok(0) => {
                        let _ = tx.send(Ok(String::new()));
                        break;
                    }
                    Ok(_) => {
                        if bytes.last() == Some(&b'\n') {
                            bytes.pop();
                        }
                        if bytes.last() == Some(&b'\r') {
                            bytes.pop();
                        }
                        if bytes.len() > MAX_JSONL_LINE_BYTES {
                            let _ = tx.send(Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "worker stdout line exceeds cap",
                            )));
                            break;
                        }
                        let line = match String::from_utf8(bytes) {
                            Ok(line) => line,
                            Err(error) => {
                                let _ = tx.send(Err(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    error,
                                )));
                                break;
                            }
                        };
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
        Ok(Self {
            process: Arc::new(Mutex::new(process)),
            stdin,
            lines,
        })
    }

    fn exchange(
        &mut self,
        request: &RequestEnvelope,
        timeout: std::time::Duration,
    ) -> Result<ResponseEnvelope, NativeMediaError> {
        let mut frame = serde_json::to_vec(request)?;
        if frame.len() > MAX_JSONL_LINE_BYTES {
            return Err(NativeMediaError::Protocol(format!(
                "native media request exceeds {MAX_JSONL_LINE_BYTES}-byte JSONL cap"
            )));
        }
        frame.push(b'\n');
        self.stdin.write_all(&frame)?;
        self.stdin.flush()?;
        match self.lines.recv_timeout(timeout) {
            Ok(Ok(line)) if line.is_empty() => Err(self.unexpected_exit()),
            Ok(Ok(line)) => {
                let response: ResponseEnvelope = serde_json::from_str(&line)?;
                if response.protocol_version != MAC_MEDIA_PROTOCOL_VERSION {
                    return Err(NativeMediaError::Protocol(format!(
                        "unsupported response version {}",
                        response.protocol_version
                    )));
                }
                if response.request_id != request.request_id
                    || response.operation != request.operation
                {
                    return Err(NativeMediaError::Correlation {
                        expected: request.request_id.clone(),
                        actual: response.request_id,
                    });
                }
                Ok(response)
            }
            Ok(Err(error)) => Err(NativeMediaError::Io(error)),
            Err(RecvTimeoutError::Timeout) => Err(NativeMediaError::Timeout {
                request_id: request.request_id.clone(),
                timeout,
            }),
            Err(RecvTimeoutError::Disconnected) => Err(self.unexpected_exit()),
        }
    }

    fn unexpected_exit(&self) -> NativeMediaError {
        let (stderr, truncated) = self
            .process
            .lock()
            .map(|process| process.stderr_snapshot())
            .unwrap_or_default();
        let suffix = if truncated {
            " ...[stderr truncated]"
        } else {
            ""
        };
        NativeMediaError::UnexpectedExit(format!(
            "{}{}",
            String::from_utf8_lossy(&stderr).trim(),
            suffix
        ))
    }
}

/// One persistent, Rust-supervised worker. Calls serialize at this boundary so
/// input order is deterministic; the Swift worker may explicitly index any
/// operation that gains internal parallelism later.
pub struct MacMediaWorker {
    worker: PathBuf,
    worker_args: Vec<String>,
    session: Mutex<Option<Session>>,
    active_process: Mutex<Option<Arc<Mutex<ManagedChild>>>>,
    active_request: Mutex<Option<String>>,
    active_timeline_output: Mutex<Option<PathBuf>>,
    cancelled: Mutex<HashSet<String>>,
    sequence: AtomicU64,
}

impl MacMediaWorker {
    pub fn new() -> Result<Self, NativeMediaError> {
        #[cfg(target_os = "macos")]
        let worker = video_core::materialize_worker(
            include_bytes!(env!("CUTRIGHT_MACOS_MEDIA_WORKER")),
            "macos-media",
        )?;
        #[cfg(not(target_os = "macos"))]
        let worker = return Err(NativeMediaError::UnsupportedPlatform);
        Ok(Self {
            worker,
            worker_args: Vec::new(),
            session: Mutex::new(None),
            active_process: Mutex::new(None),
            active_request: Mutex::new(None),
            active_timeline_output: Mutex::new(None),
            cancelled: Mutex::new(HashSet::new()),
            sequence: AtomicU64::new(0),
        })
    }

    pub fn with_worker(worker: PathBuf) -> Self {
        Self {
            worker,
            worker_args: Vec::new(),
            session: Mutex::new(None),
            active_process: Mutex::new(None),
            active_request: Mutex::new(None),
            active_timeline_output: Mutex::new(None),
            cancelled: Mutex::new(HashSet::new()),
            sequence: AtomicU64::new(0),
        }
    }

    pub fn worker_blake3(&self) -> Result<String, NativeMediaError> {
        let bytes = std::fs::read(&self.worker)?;
        Ok(format!("blake3:{}", blake3::hash(&bytes).to_hex()))
    }

    #[cfg(test)]
    pub fn with_worker_args(worker: PathBuf, worker_args: Vec<String>) -> Self {
        Self {
            worker,
            worker_args,
            session: Mutex::new(None),
            active_process: Mutex::new(None),
            active_request: Mutex::new(None),
            active_timeline_output: Mutex::new(None),
            cancelled: Mutex::new(HashSet::new()),
            sequence: AtomicU64::new(0),
        }
    }

    fn request(
        &self,
        context: &NativeRequestContext,
        operation: &str,
        payload: Value,
    ) -> Result<ResponseEnvelope, NativeMediaError> {
        validate_paths(&payload)?;
        let request = RequestEnvelope {
            protocol_version: MAC_MEDIA_PROTOCOL_VERSION,
            request_id: context.request_id.clone(),
            operation: operation.to_string(),
            payload,
        };
        let mut guard = self
            .session
            .lock()
            .map_err(|_| NativeMediaError::Protocol("worker mutex poisoned".into()))?;
        for attempt in 0..2 {
            if guard.is_none() {
                *guard = Some(Session::spawn(&self.worker, &self.worker_args)?);
            }
            *self.active_request.lock().map_err(|_| {
                NativeMediaError::Protocol("active request mutex poisoned".into())
            })? = Some(context.request_id.clone());
            *self.active_process.lock().map_err(|_| {
                NativeMediaError::Protocol("active process mutex poisoned".into())
            })? = Some(guard.as_ref().expect("session initialized").process.clone());
            let response = guard
                .as_mut()
                .expect("session initialized")
                .exchange(&request, context.timeout);
            *self.active_process.lock().map_err(|_| {
                NativeMediaError::Protocol("active process mutex poisoned".into())
            })? = None;
            *self.active_request.lock().map_err(|_| {
                NativeMediaError::Protocol("active request mutex poisoned".into())
            })? = None;
            if self
                .cancelled
                .lock()
                .map_err(|_| NativeMediaError::Protocol("cancel mutex poisoned".into()))?
                .remove(&context.request_id)
            {
                *guard = None;
                return Err(NativeMediaError::Cancelled {
                    request_id: context.request_id.clone(),
                });
            }
            match response {
                Ok(response) => return response_to_result(response),
                Err(NativeMediaError::Timeout { .. }) => {
                    guard
                        .as_mut()
                        .expect("session initialized")
                        .process
                        .lock()
                        .map_err(|_| {
                            NativeMediaError::Protocol("worker process mutex poisoned".into())
                        })?
                        .kill_tree();
                    *guard = None;
                    return Err(NativeMediaError::Timeout {
                        request_id: context.request_id.clone(),
                        timeout: context.timeout,
                    });
                }
                Err(error @ NativeMediaError::UnexpectedExit(_)) if attempt == 0 => {
                    *guard = None;
                    let _ = error;
                }
                Err(error) => {
                    *guard = None;
                    return Err(error);
                }
            }
        }
        Err(NativeMediaError::UnexpectedExit(
            "worker did not restart".into(),
        ))
    }

    fn generated_context(&self, timeout: std::time::Duration) -> NativeRequestContext {
        NativeRequestContext {
            request_id: format!(
                "mac-media-{}-{}",
                std::process::id(),
                self.sequence.fetch_add(1, Ordering::Relaxed)
            ),
            timeout,
        }
    }

    #[cfg(test)]
    fn capabilities_for_test(
        &self,
        context: NativeRequestContext,
    ) -> Result<MacMediaCapabilities, NativeMediaError> {
        self.request(&context, "hello", json!({}))?
            .capabilities
            .ok_or_else(|| NativeMediaError::Protocol("hello response omitted capabilities".into()))
    }
}

fn response_to_result(response: ResponseEnvelope) -> Result<ResponseEnvelope, NativeMediaError> {
    if response.ok {
        return Ok(response);
    }
    let error = response.error.unwrap_or(super::protocol::ErrorPayload {
        code: "unknown".into(),
        message: "worker rejected request without an error".into(),
        retryable: false,
    });
    if error.code == "unsupported" {
        Err(NativeMediaError::Unsupported(error.message))
    } else {
        Err(NativeMediaError::Protocol(format!(
            "{}: {}",
            error.code, error.message
        )))
    }
}

fn media_worker_env() -> Vec<(String, String)> {
    ["PATH", "HOME", "TMPDIR"]
        .iter()
        .filter_map(|key| env::var(key).ok().map(|value| ((*key).to_string(), value)))
        .collect()
}

fn validate_paths(payload: &Value) -> Result<(), NativeMediaError> {
    let Some(object) = payload.as_object() else {
        return Ok(());
    };
    let has_direct_input = ["source", "sourcePath", "inputPath"]
        .iter()
        .any(|key| object.get(*key).and_then(Value::as_str).is_some());
    let frames = object.get("frames").and_then(Value::as_array);
    let has_output = object.get("outputPath").and_then(Value::as_str).is_some();
    if !has_direct_input && frames.is_none() && !has_output {
        return Ok(());
    }
    let roots = object
        .get("allowedRoots")
        .and_then(Value::as_array)
        .ok_or_else(|| NativeMediaError::InvalidPath(PathBuf::from("allowedRoots")))?;
    let roots: Vec<PathBuf> = roots
        .iter()
        .filter_map(Value::as_str)
        .filter_map(|root| Path::new(root).canonicalize().ok())
        .collect();
    if roots.is_empty() {
        return Err(NativeMediaError::InvalidPath(PathBuf::from("allowedRoots")));
    }
    let validate_input = |value: &str| -> Result<(), NativeMediaError> {
        let path = PathBuf::from(value);
        if !path.is_absolute() || !path.is_file() {
            return Err(NativeMediaError::InvalidPath(path));
        }
        let canonical = path
            .canonicalize()
            .map_err(|_| NativeMediaError::InvalidPath(path.clone()))?;
        if !roots.iter().any(|root| canonical.starts_with(root)) {
            return Err(NativeMediaError::InvalidPath(canonical));
        }
        Ok(())
    };
    for key in ["source", "sourcePath", "inputPath"] {
        if let Some(value) = object.get(key).and_then(Value::as_str) {
            validate_input(value)?;
        }
    }
    if let Some(frames) = frames {
        for frame in frames {
            let source = frame
                .as_object()
                .and_then(|value| value.get("sourcePath"))
                .and_then(Value::as_str)
                .ok_or_else(|| NativeMediaError::InvalidPath(PathBuf::from("frames.sourcePath")))?;
            validate_input(source)?;
        }
    }
    if let Some(value) = object.get("outputPath").and_then(Value::as_str) {
        let path = PathBuf::from(value);
        let parent = path
            .parent()
            .ok_or_else(|| NativeMediaError::InvalidPath(path.clone()))?;
        let canonical_parent = parent
            .canonicalize()
            .map_err(|_| NativeMediaError::InvalidPath(path.clone()))?;
        if !path.is_absolute() || !roots.iter().any(|root| canonical_parent.starts_with(root)) {
            return Err(NativeMediaError::InvalidPath(path));
        }
    }
    Ok(())
}

impl MacMediaBackend for MacMediaWorker {
    fn capabilities(&self) -> Result<MacMediaCapabilities, NativeMediaError> {
        let context = self.generated_context(std::time::Duration::from_secs(10));
        let response = self.request(&context, "hello", json!({}))?;
        let mut capabilities = response.capabilities.ok_or_else(|| {
            NativeMediaError::Protocol("hello response omitted capabilities".into())
        })?;
        capabilities.worker_blake3 = self.worker_blake3()?;
        Ok(capabilities)
    }
    fn inspect_asset(
        &self,
        context: &NativeRequestContext,
        source: &Path,
    ) -> Result<NativeAssetInfo, NativeMediaError> {
        serde_json::from_value(self.request(context, "inspectAsset", json!({ "sourcePath": source, "allowedRoots": [source.parent().unwrap_or(Path::new("/"))] }))?.result.unwrap_or(Value::Null)).map_err(NativeMediaError::Json)
    }
    fn analyze_frames(
        &self,
        context: &NativeRequestContext,
        request: &AnalyzeFramesRequest,
    ) -> Result<Vec<NativeFrameAnalysis>, NativeMediaError> {
        let value = self
            .request(context, "analyzeFrames", serde_json::to_value(request)?)?
            .result
            .unwrap_or(Value::Array(Vec::new()));
        serde_json::from_value(value).map_err(NativeMediaError::Json)
    }
    fn render_caption(
        &self,
        context: &NativeRequestContext,
        request: &NativeCaptionRequest,
    ) -> Result<NativeRenderArtifact, NativeMediaError> {
        serde_json::from_value(
            self.request(context, "renderCaption", serde_json::to_value(request)?)?
                .result
                .unwrap_or(Value::Null),
        )
        .map_err(NativeMediaError::Json)
    }
    fn render_preview(
        &self,
        context: &NativeRequestContext,
        request: &NativePreviewRequest,
    ) -> Result<NativeRenderArtifact, NativeMediaError> {
        serde_json::from_value(
            self.request(context, "renderPreview", serde_json::to_value(request)?)?
                .result
                .unwrap_or(Value::Null),
        )
        .map_err(NativeMediaError::Json)
    }
    fn audio_features(
        &self,
        context: &NativeRequestContext,
        request: &NativeAudioRequest,
    ) -> Result<NativeAudioFeatures, NativeMediaError> {
        serde_json::from_value(
            self.request(context, "audioFeatures", serde_json::to_value(request)?)?
                .result
                .unwrap_or(Value::Null),
        )
        .map_err(NativeMediaError::Json)
    }
    fn render_timeline(
        &self,
        context: &NativeRequestContext,
        request: &NativeTimelineRenderRequest,
    ) -> Result<NativeTimelineRenderResult, NativeMediaError> {
        if request.schema_version != 1
            || request.locked_cut_sha256.len() != 64
            || !request
                .locked_cut_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(NativeMediaError::Protocol(
                "invalid locked timeline request".into(),
            ));
        }
        if request.mode == super::MacNativeMode::Legacy {
            return Err(NativeMediaError::Unsupported(
                "legacy timeline mode uses FFmpeg".into(),
            ));
        }
        if request.video.width == 0
            || request.video.height == 0
            || request.video.width > 8_192
            || request.video.height > 8_192
            || request.video.frame_rate_num == 0
            || request.video.frame_rate_den == 0
            || u64::from(request.video.frame_rate_num)
                > 240 * u64::from(request.video.frame_rate_den)
            || request.audio.sample_rate == 0
            || request.audio.sample_rate > 192_000
            || request.audio.channels == 0
            || request.audio.channels > 8
        {
            return Err(NativeMediaError::Protocol(
                "timeline output exceeds bounds".into(),
            ));
        }
        request
            .graph
            .validate()
            .map_err(|error| NativeMediaError::Protocol(error.to_string()))?;
        validate_paths(&json!({
            "sourcePath": &request.graph.source_path,
            "outputPath": &request.output_path,
            "allowedRoots": &request.allowed_roots,
        }))?;
        for asset in request.graph.assets.values() {
            validate_paths(&json!({
                "sourcePath": asset,
                "allowedRoots": &request.allowed_roots,
            }))?;
        }
        *self
            .active_timeline_output
            .lock()
            .map_err(|_| NativeMediaError::Protocol("timeline output mutex poisoned".into()))? =
            Some(request.output_path.clone());
        let response = (|| {
            let value = self
                .request(context, "renderTimeline", serde_json::to_value(request)?)?
                .result
                .unwrap_or(Value::Null);
            serde_json::from_value(value).map_err(NativeMediaError::Json)
        })();
        *self
            .active_timeline_output
            .lock()
            .map_err(|_| NativeMediaError::Protocol("timeline output mutex poisoned".into()))? =
            None;
        if response.is_err() {
            if let Some(path) = timeline_temp_path(&request.output_path) {
                let _ = std::fs::remove_file(path);
            }
        }
        response
    }
    fn cancel(&self, request_id: &str) -> Result<(), NativeMediaError> {
        let active = self
            .active_request
            .lock()
            .map_err(|_| NativeMediaError::Protocol("active request mutex poisoned".into()))?
            .as_deref()
            == Some(request_id);
        if !active {
            return Ok(());
        }
        self.cancelled
            .lock()
            .map_err(|_| NativeMediaError::Protocol("cancel mutex poisoned".into()))?
            .insert(request_id.to_string());
        if let Some(process) = self
            .active_process
            .lock()
            .map_err(|_| NativeMediaError::Protocol("active process mutex poisoned".into()))?
            .clone()
        {
            process
                .lock()
                .map_err(|_| NativeMediaError::Protocol("worker process mutex poisoned".into()))?
                .kill_tree();
        }
        if let Some(output) = self
            .active_timeline_output
            .lock()
            .map_err(|_| NativeMediaError::Protocol("timeline output mutex poisoned".into()))?
            .as_ref()
        {
            if let Some(path) = timeline_temp_path(output) {
                let _ = std::fs::remove_file(path);
            }
        }
        Ok(())
    }
}

fn timeline_temp_path(output: &Path) -> Option<PathBuf> {
    let name = output.file_name()?.to_string_lossy();
    Some(output.parent()?.join(format!(".{name}.tmp.mp4")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::MacMediaBackend;
    use std::sync::Arc;

    #[test]
    fn active_cancel_does_not_retry_and_next_hello_restarts_worker() {
        let marker = tempfile::NamedTempFile::new().expect("marker path");
        let marker_path = marker.path().to_string_lossy().replace('\'', "'\\''");
        drop(marker);
        let script = format!(
            r#"read line; id=$(printf '%s' "$line" | sed -n 's/.*"requestId":"\([^"]*\)".*/\1/p'); if [ ! -f '{marker_path}' ]; then touch '{marker_path}'; sleep 5; fi; printf '{{"protocolVersion":1,"requestId":"%s","operation":"hello","ok":true,"capabilities":{{"avFoundation":true,"vision":true,"caption":true,"preview":false,"audio":true,"metal":false,"osVersion":"fixture","workerVersion":"fixture"}},"elapsedNanoseconds":0}}\n' "$id""#
        );
        let worker = Arc::new(MacMediaWorker::with_worker_args(
            PathBuf::from("/bin/sh"),
            vec!["-c".into(), script],
        ));
        let active = NativeRequestContext {
            request_id: "active".into(),
            timeout: std::time::Duration::from_secs(10),
        };
        let pending = {
            let worker = worker.clone();
            thread::spawn(move || worker.capabilities_for_test(active))
        };
        thread::sleep(std::time::Duration::from_millis(100));
        worker.cancel("active").expect("cancel active worker");
        assert!(matches!(
            pending.join().expect("join request"),
            Err(NativeMediaError::Cancelled { .. })
        ));
        let capabilities = worker.capabilities().expect("hello after restart");
        assert_eq!(capabilities.worker_version, "fixture");
    }

    #[test]
    fn path_validation_rejects_escape_and_output_outside_allowed_root() {
        let root = tempfile::tempdir().expect("root");
        let input = root.path().join("input.mov");
        std::fs::write(&input, b"fixture").expect("input");
        let escaped = tempfile::NamedTempFile::new().expect("escaped");
        assert!(validate_paths(
            &json!({ "sourcePath": escaped.path(), "allowedRoots": [root.path()] })
        )
        .is_err());
        assert!(validate_paths(&json!({ "inputPath": input, "outputPath": escaped.path(), "allowedRoots": [root.path()] })).is_err());
        assert!(validate_paths(&json!({ "sourcePath": input, "outputPath": root.path().join("out.png"), "allowedRoots": [root.path()] })).is_ok());
    }
}
