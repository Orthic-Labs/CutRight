use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

use ort::{
    ep,
    session::{builder::GraphOptimizationLevel, Session},
    value::Value,
};

const SAMPLE_RATE: i64 = 16_000;
const FRAME_SAMPLES: usize = 512;
const CONTEXT_SAMPLES: usize = 64;
const INPUT_SAMPLES: usize = FRAME_SAMPLES + CONTEXT_SAMPLES;
const LOAD_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_PENDING_SAMPLES: usize = 16_000 * 30;
// Runtime VAD threshold. Real machines vary wildly — Windows WASAPI shared
// mode produced peak Silero scores far below the original 0.5 threshold, so
// the default is the lower field-proven floor. Optional calibration can raise
// it for fixed-room experiments, but it is not enabled by default because PTT
// often captures the user's first syllable rather than ambient noise.
const MIN_SPEECH_THRESHOLD: f32 = 0.20;
const SPEECH_THRESHOLD_RMS_MULTIPLIER: f32 = 8.0;
/// Cap on noise-derived threshold so a totally silent room (RMS ≈ 0)
/// doesn't push the threshold to 0 and fire on anything.
const MIN_NOISE_FLOOR_FOR_CALIBRATION: f32 = 0.005;

const MODEL_FILE: &str = "silero_vad_16k_op15.onnx";
#[cfg(target_os = "windows")]
const ORT_DYLIB_FILE: &str = "onnxruntime.dll";
#[cfg(target_os = "macos")]
const ORT_DYLIB_FILE: &str = "libonnxruntime.dylib";
#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
const ORT_DYLIB_FILE: &str = "libonnxruntime.so";

pub struct SpeechVad {
    state: LoadState,
    model_path: Option<PathBuf>,
    pending: Vec<f32>,
    /// Runtime speech threshold. Initialized to MIN_SPEECH_THRESHOLD; optional
    /// calibration can raise it from observed ambient noise.
    speech_threshold: f32,
    /// Runtime-derived RMS noise floor. Consumed by worker_sections via
    /// `noise_floor_rms()` so the energy gate scales with the room instead
    /// of being a magic constant.
    noise_floor_rms: f32,
    telemetry_observed_frames: u64,
    telemetry_speech_frames: u64,
}

/// Controls may use VAD only once inference is genuinely available. Loading or
/// failed VAD must never be silently interpreted as silence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VadStatus {
    Loading,
    Ready,
    Failed,
}

enum LoadState {
    Loading {
        started: Instant,
        rx: Receiver<Result<SileroVad, String>>,
    },
    Ready(SileroVad),
    Disabled,
}

struct SileroVad {
    backend: VadBackend,
    state: Vec<f32>,
    context: [f32; CONTEXT_SAMPLES],
}

/// Inference backend. Production uses ORT CPU on every desktop platform. The
/// macOS Core ML variant remains explicit-only for the parity harness; keeping
/// live VAD outside Core ML prevents it from sharing a failure domain with the
/// concurrent ASR probe runtime.
enum VadBackend {
    Ort(Session),
    #[cfg(target_os = "macos")]
    CoreMl(crate::coreml::Stage),
}

impl SpeechVad {
    pub fn new() -> Self {
        Self::with_model_path(default_model_path())
    }

    pub fn with_model_path(model_path: Option<PathBuf>) -> Self {
        let mut vad = Self {
            state: LoadState::Disabled,
            model_path,
            pending: Vec::with_capacity(FRAME_SAMPLES * 4),
            speech_threshold: MIN_SPEECH_THRESHOLD,
            noise_floor_rms: 0.0,
            telemetry_observed_frames: 0,
            telemetry_speech_frames: 0,
        };
        vad.start_loading();
        vad
    }

    pub fn reset(&mut self) {
        self.pending.clear();
        self.telemetry_observed_frames = 0;
        self.telemetry_speech_frames = 0;
        if let LoadState::Ready(vad) = &mut self.state {
            vad.reset();
        }
    }

    /// Set the speech detection threshold from observed ambient audio.
    /// This is opt-in only; the normal PTT path often captures the user's
    /// first syllable, which is not a valid noise sample.
    pub fn calibrate(&mut self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }
        let rms = heardright_core::audio_conditioning::rms(samples);
        // Clamp to MIN_NOISE_FLOOR_FOR_CALIBRATION so a totally silent room
        // doesn't push the threshold to zero (false-fire on anything).
        self.noise_floor_rms = rms.max(MIN_NOISE_FLOOR_FOR_CALIBRATION);
        self.speech_threshold =
            MIN_SPEECH_THRESHOLD.max(self.noise_floor_rms * SPEECH_THRESHOLD_RMS_MULTIPLIER);
        tracing::info!(
            "silero VAD calibrated: noise_floor_rms={:.5} speech_threshold={:.4}",
            self.noise_floor_rms,
            self.speech_threshold,
        );
    }

    /// Current speech threshold (post-calibration). Worker consumes this to
    /// override the energy gate floor too.
    pub fn speech_threshold(&self) -> f32 {
        self.speech_threshold
    }

    /// Measured noise floor RMS. Worker uses this for the energy gate.
    pub fn noise_floor_rms(&self) -> f32 {
        self.noise_floor_rms
    }

    pub fn observe(&mut self, samples: &[f32]) -> bool {
        if samples.is_empty() {
            return false;
        }
        self.poll_loading();
        match self.state {
            LoadState::Loading { .. } => {
                // Capture starts before the asynchronous model load can
                // necessarily finish. Preserve those first syllables and feed
                // them once Silero is ready instead of dropping them.
                self.push_pending(samples);
                return false;
            }
            LoadState::Disabled => {
                self.pending.clear();
                return false;
            }
            LoadState::Ready(_) => {}
        }
        self.push_pending(samples);
        let complete_frames = take_complete_frames(&mut self.pending);
        if complete_frames.is_empty() {
            return false;
        }
        let threshold = self.speech_threshold;
        match &mut self.state {
            LoadState::Ready(vad) => {
                let observed_frames = (complete_frames.len() / FRAME_SAMPLES) as u64;
                let confirmed = vad.observe(&complete_frames, threshold);
                match confirmed {
                    Ok((confirmed, speech_frames)) => {
                        self.telemetry_observed_frames = self
                            .telemetry_observed_frames
                            .saturating_add(observed_frames);
                        self.telemetry_speech_frames = self
                            .telemetry_speech_frames
                            .saturating_add(speech_frames as u64);
                        confirmed
                    }
                    Err(err) => {
                        tracing::warn!("silero VAD inference failed: {err}");
                        self.state = LoadState::Disabled;
                        false
                    }
                }
            }
            LoadState::Loading { .. } | LoadState::Disabled => false,
        }
    }

    pub fn recording_aggregate(&self) -> Option<(u64, u64)> {
        (self.telemetry_observed_frames > 0)
            .then_some((self.telemetry_observed_frames, self.telemetry_speech_frames))
    }

    pub fn status(&mut self) -> VadStatus {
        self.poll_loading();
        match self.state {
            LoadState::Loading { .. } => VadStatus::Loading,
            LoadState::Ready(_) => VadStatus::Ready,
            LoadState::Disabled => VadStatus::Failed,
        }
    }

    pub fn is_ready(&mut self) -> bool {
        self.status() == VadStatus::Ready
    }

    fn start_loading(&mut self) {
        let Some(path) = self.model_path.clone() else {
            tracing::warn!("silero VAD disabled: {MODEL_FILE} not found");
            self.state = LoadState::Disabled;
            return;
        };
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("silero_vad_load".into())
            .spawn(move || {
                let started = Instant::now();
                let result = SileroVad::load(path);
                tracing::info!(
                    "silero VAD load finished in {} ms ok={}",
                    started.elapsed().as_millis(),
                    result.is_ok()
                );
                let _ = tx.send(result);
            })
            .expect("spawn silero VAD loader");
        self.state = LoadState::Loading {
            started: Instant::now(),
            rx,
        };
    }

    fn poll_loading(&mut self) {
        let LoadState::Loading { started, rx } = &self.state else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(vad)) => {
                tracing::info!("silero VAD ready");
                self.state = LoadState::Ready(vad);
            }
            Ok(Err(err)) => {
                tracing::warn!("silero VAD disabled: {err}");
                self.state = LoadState::Disabled;
            }
            Err(mpsc::TryRecvError::Empty) if started.elapsed() > LOAD_TIMEOUT => {
                tracing::warn!(
                    "silero VAD disabled: model load exceeded {} ms",
                    LOAD_TIMEOUT.as_millis()
                );
                self.state = LoadState::Disabled;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                tracing::warn!("silero VAD disabled: loader disconnected");
                self.state = LoadState::Disabled;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    fn push_pending(&mut self, samples: &[f32]) {
        self.pending.extend_from_slice(samples);
        if self.pending.len() > MAX_PENDING_SAMPLES {
            let extra = self.pending.len() - MAX_PENDING_SAMPLES;
            self.pending.drain(..extra);
        }
    }

    /// Backend name once loading finished (`None` while loading/disabled).
    /// Diagnostic only — used by `examples/vad_parity_check.rs`.
    pub fn backend_name(&self) -> Option<&'static str> {
        match &self.state {
            LoadState::Ready(vad) => Some(match vad.backend {
                VadBackend::Ort(_) => "ort",
                #[cfg(target_os = "macos")]
                VadBackend::CoreMl(_) => "coreml",
            }),
            _ => None,
        }
    }

    #[cfg(test)]
    fn is_ready_for_test(&self) -> bool {
        matches!(self.state, LoadState::Ready(_))
    }
}

/// Remove and return only complete Silero frames, retaining the trailing
/// partial frame for the next capture callback. Silero is recurrent: dropping
/// a callback remainder creates gaps in both the waveform and model state.
fn take_complete_frames(pending: &mut Vec<f32>) -> Vec<f32> {
    let complete_len = pending.len() / FRAME_SAMPLES * FRAME_SAMPLES;
    if complete_len == 0 {
        return Vec::new();
    }
    let tail = pending.split_off(complete_len);
    std::mem::replace(pending, tail)
}

impl Default for SpeechVad {
    fn default() -> Self {
        Self::new()
    }
}

impl SileroVad {
    fn load(path: PathBuf) -> Result<Self, String> {
        let backend = Self::load_backend(&path)?;
        let mut vad = Self {
            backend,
            state: vec![0.0; 2 * 128],
            context: [0.0; CONTEXT_SAMPLES],
        };
        let _ = vad.observe(&vec![0.0; FRAME_SAMPLES], MIN_SPEECH_THRESHOLD)?;
        Ok(vad)
    }

    fn load_backend(path: &Path) -> Result<VadBackend, String> {
        #[cfg(target_os = "macos")]
        if matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("mlmodelc") | Some("mlpackage")
        ) {
            // Tiny model, one 32 ms frame per call: CPU-only avoids per-call
            // ANE/GPU dispatch overhead that would dwarf the compute.
            let stage = crate::coreml::Stage::load_cpu_only(path)?;
            return Ok(VadBackend::CoreMl(stage));
        }
        init_ort()?;
        let session = Session::builder()
            .map_err(|err| format!("session builder: {err}"))?
            .with_optimization_level(GraphOptimizationLevel::Disable)
            .map_err(|err| format!("graph optimization: {err}"))?
            .with_intra_threads(1)
            .map_err(|err| format!("intra threads: {err}"))?
            .with_inter_threads(1)
            .map_err(|err| format!("inter threads: {err}"))?
            .with_execution_providers([ep::CPU::default().build()])
            .map_err(|err| format!("cpu provider: {err}"))?
            .commit_from_file(path)
            .map_err(|err| format!("load {}: {err}", path.display()))?;
        Ok(VadBackend::Ort(session))
    }

    fn reset(&mut self) {
        self.state.fill(0.0);
        self.context.fill(0.0);
    }

    fn observe(&mut self, samples: &[f32], threshold: f32) -> Result<(bool, usize), String> {
        let mut confirmed = false;
        let mut speech_frames = 0usize;
        for frame in samples.chunks_exact(FRAME_SAMPLES) {
            let speech = self.observe_frame(frame, threshold)?;
            confirmed |= speech;
            speech_frames += usize::from(speech);
        }
        Ok((confirmed, speech_frames))
    }

    fn observe_frame(&mut self, frame: &[f32], threshold: f32) -> Result<bool, String> {
        let mut input = Vec::with_capacity(INPUT_SAMPLES);
        input.extend_from_slice(&self.context);
        input.extend(frame.iter().map(|sample| sample.clamp(-1.0, 1.0)));

        let (score, state_n) = match &mut self.backend {
            VadBackend::Ort(session) => {
                let outputs = session
                    .run(ort::inputs!(
                        "input" => Value::from_array(([1, INPUT_SAMPLES], input)).map_err(|err| format!("input value: {err}"))?,
                        "state" => Value::from_array(([2, 1, 128], self.state.clone())).map_err(|err| format!("state value: {err}"))?,
                        "sr" => Value::from_array(([] as [usize; 0], vec![SAMPLE_RATE])).map_err(|err| format!("sr value: {err}"))?
                    ))
                    .map_err(|err| format!("run: {err}"))?;

                let (_, output) = outputs["output"]
                    .try_extract_tensor::<f32>()
                    .map_err(|err| format!("output tensor: {err}"))?;
                let score = *output
                    .first()
                    .ok_or_else(|| "empty output tensor".to_string())?;
                let (_, state_n) = outputs["stateN"]
                    .try_extract_tensor::<f32>()
                    .map_err(|err| format!("stateN tensor: {err}"))?;
                (score, state_n.to_vec())
            }
            #[cfg(target_os = "macos")]
            VadBackend::CoreMl(stage) => {
                let input = crate::coreml::ml_f32(&[1, INPUT_SAMPLES], &input)?;
                let state = crate::coreml::ml_f32(&[2, 1, 128], &self.state)?;
                let outs = stage.predict_multi(
                    &[("input", &input), ("state_in", &state)],
                    &["output", "state_out"],
                )?;
                let score = *crate::coreml::read_f32(&outs[0])
                    .first()
                    .ok_or_else(|| "empty output tensor".to_string())?;
                (score, crate::coreml::read_f32(&outs[1]))
            }
        };
        if state_n.len() != 2 * 128 {
            return Err(format!("unexpected state length {}", state_n.len()));
        }
        tracing::trace!("silero VAD score={:.4}", score);
        self.state = state_n;
        self.context
            .copy_from_slice(&frame[FRAME_SAMPLES - CONTEXT_SAMPLES..]);
        Ok(score >= threshold)
    }
}

fn init_ort() -> Result<(), String> {
    if let Some(path) = ort_dylib_path() {
        tracing::info!("silero VAD loading ORT dylib={}", path.display());
        std::env::set_var("ORT_DYLIB_PATH", &path);
        let committed = ort::init_from(&path)
            .map_err(|err| format!("load ORT {}: {err}", path.display()))?
            .with_execution_providers([ep::CPU::default().build()])
            .with_telemetry(false)
            .commit();
        tracing::info!(
            "silero VAD ORT dylib={} committed={committed}",
            path.display()
        );
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    return Err(format!(
        "{ORT_DYLIB_FILE} not found; packaged macOS VAD requires the bundled ORT runtime"
    ));
    #[cfg(not(target_os = "macos"))]
    {
        ort::init()
            .with_execution_providers([ep::CPU::default().build()])
            .with_telemetry(false)
            .commit();
        Ok(())
    }
}

fn default_model_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("HR_VAD_MODEL").map(PathBuf::from) {
        // A .onnx file or a .mlmodelc directory — either backend.
        return path.exists().then_some(path);
    }
    model_path_candidates()
        .into_iter()
        .find(|candidate| candidate.is_file())
}

fn model_path_candidates() -> Vec<PathBuf> {
    resource_candidates("vad", MODEL_FILE)
}

fn ort_dylib_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("HR_VAD_ORT_DYLIB")
        .or_else(|| std::env::var_os("ORT_DYLIB_PATH"))
        .map(PathBuf::from)
    {
        if path.is_file() {
            return Some(path);
        }
        // Developer shells often retain an old ORT_DYLIB_PATH. An invalid
        // override must not suppress the app-bundled runtime: ort's dynamic
        // loader can otherwise recurse while formatting its load error.
        tracing::warn!(
            "ignoring missing ORT dylib override and checking bundled runtime: {}",
            path.display()
        );
    }
    resource_candidates("runtime", ORT_DYLIB_FILE)
        .into_iter()
        .find(|candidate| candidate.is_file())
}

fn resource_candidates(dir: &str, file: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            push_resource_candidates(&mut paths, exe_dir, dir, file);
            #[cfg(target_os = "macos")]
            if let Some(contents) = exe_dir.parent() {
                push_resource_candidates(&mut paths, &contents.join("Resources"), dir, file);
            }
        }
    }
    #[cfg(debug_assertions)]
    push_resource_candidates(
        &mut paths,
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/resources"),
        dir,
        file,
    );
    paths
}

fn push_resource_candidates(paths: &mut Vec<PathBuf>, base: &Path, dir: &str, file: &str) {
    paths.push(base.join(dir).join(file));
    paths.push(base.join("resources").join(dir).join(file));
}

#[cfg(test)]
mod tests {
    use super::{
        model_path_candidates, take_complete_frames, SpeechVad, VadStatus, FRAME_SAMPLES,
        MODEL_FILE,
    };

    #[cfg(target_os = "macos")]
    #[test]
    fn bundled_macos_default_selects_the_onnx_model() {
        let candidates = model_path_candidates();
        assert!(!candidates.is_empty());
        assert!(candidates
            .iter()
            .all(|path| path.file_name().and_then(|name| name.to_str()) == Some(MODEL_FILE)));
        assert!(candidates
            .iter()
            .all(|path| path.extension().and_then(|ext| ext.to_str()) == Some("onnx")));
    }

    #[test]
    fn chunked_capture_preserves_every_sample_across_silero_frames() {
        let input: Vec<f32> = (0..(FRAME_SAMPLES * 3 + 137))
            .map(|sample| sample as f32)
            .collect();
        let mut pending = Vec::new();
        let mut observed = Vec::new();

        for chunk in input.chunks(700) {
            pending.extend_from_slice(chunk);
            observed.extend(take_complete_frames(&mut pending));
        }

        assert_eq!(observed, input[..FRAME_SAMPLES * 3]);
        assert_eq!(pending, input[FRAME_SAMPLES * 3..]);
    }

    #[test]
    fn missing_model_does_not_confirm_speech() {
        let mut vad = SpeechVad::with_model_path(None);
        assert!(!vad.observe(&vec![0.8; FRAME_SAMPLES]));
        assert_eq!(vad.status(), VadStatus::Failed);
        assert!(!vad.is_ready());
    }

    #[test]
    fn silence_does_not_confirm_speech() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("info,heardright_engine=info")
            .with_writer(std::io::stderr)
            .try_init();
        let mut vad = SpeechVad::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(6);
        while std::time::Instant::now() < deadline {
            assert!(!vad.observe(&vec![0.0; FRAME_SAMPLES]));
            if vad.is_ready_for_test() {
                return;
            }
        }
        assert!(vad.is_ready_for_test(), "Silero VAD did not become ready");
    }
}
