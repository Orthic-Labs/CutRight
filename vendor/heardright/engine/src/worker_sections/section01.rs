// Capture coordinator for the sidecar.
//
// Owns one `CaptureSession` and recording buffer. Independent main-ASR and CPU
// KWS workers own their respective models and queues.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

#[cfg(test)]
#[path = "replay_driver.rs"]
mod replay_driver;

#[cfg(test)]
use replay_driver::ReplayDriver;
#[cfg(test)]
use wav_replay_source::{CaptureEvent, ScriptedCaptureEvent};

use heardright_capture::{
    list_input_devices, CaptureFirstBuffer, CaptureSession, CaptureTransport, DeviceInfo,
    MetricsSnapshot, OutDType, SessionConfig,
};
use parking_lot::Mutex;

use crate::asr::{scheduled_static15_ready_segment, AsrEp, AsrRuntime, ScheduledStatic15};
use crate::command_classify::CommandClassification;
use crate::focus::FocusTracker;
use crate::owner_diagnostics;
use heardright_core::engine::{FileTranscript, RecordingStageMetrics};
use heardright_core::mic_selection::{resolve_mic_selection, MicDeviceSnapshot};
use parakeet_rs::TranscriptionResult;
use serde_json::json;

const SAMPLE_RATE: u32 = 16_000;
const CAPTURE_POLL: Duration = Duration::from_millis(10);
/// A final dictation must fail closed instead of holding the interaction
/// indefinitely when the main ASR worker wedges.
const FINAL_REPLY_TIMEOUT: Duration = Duration::from_secs(120);
/// File transcription has no delivery authority but still needs bounded caller
/// latency for a wedged worker or dropped mailbox response.
const FILE_REPLY_TIMEOUT: Duration = Duration::from_secs(600);
// M13 (perf audit 2026-07-15): `refresh_current()` was called every streaming
// iteration (~100x/sec, gated only by CAPTURE_POLL) — cheap syscalls
// individually (GetForegroundWindow + title/class reads, no UIA walk) but the
// cadence was wasteful and contends the same focus mutex `finalize` reads
// from. Throttling to this cadence still catches a focus change well within
// one perceived "tick" of dictation, at a fraction of the lock traffic.
const FOCUS_REFRESH_THROTTLE_MS: u64 = 200;

// --- Streaming command / zephyr-tail auto-fire (hands-free, no manual stop) ---
// Silero alone decides whether speech is active and therefore owns the trailing
// pause clock. Perceptual `chunk_level`/`voice_ema` is display-only.
// Silence after speech that gives us one final standalone-command probe. Direct
// catalog commands also probe while speech is still arriving; unfinished direct
// phrases, app-launch verbs, & modifier chords wait until their next word resolves.
const PAUSE_GATE_MS: u64 = 180;
const CMD_RETRY_MS: u64 = 50;
const CMD_PREFIX_GRACE_MS: u64 = 2_500;
const COMMAND_EVENTS_JSONL_MAX_BYTES: u64 = 10 * 1024 * 1024;
// Only the first short utterance can be a standalone command. This is a probe
// bound, not a wait; long dictation should stop paying command-probe cost.
const CMD_WINDOW_SAMPLES: usize = SAMPLE_RATE as usize * 2;
const CMD_MIN_PROBE_SAMPLES: usize = SAMPLE_RATE as usize / 4;
const CMD_NEW_AUDIO_PROBE_SAMPLES: usize = SAMPLE_RATE as usize / 20;
// Once an utterance is longer than the standalone-command window, one main-ASR
// tail check at each speech boundary backs up a confirmed KWS acoustic miss. It
// reuses the strict trailing-control parser without weakening KWS thresholds.
// Keep a small prefix before Silero-confirmed speech so first consonants are not
// trimmed, but measure command eligibility from speech, not from capture start.
const CMD_PREROLL_SAMPLES: usize = SAMPLE_RATE as usize / 4;
// Keep the 20 ms detection cadence, but write routine successful/no-result
// diagnostics only once per second. Candidates, controls, errors, stale results
// & mailbox anomalies remain unsampled.
const KWS_DIAGNOSTIC_SAMPLE_EVERY_PROBES: u64 = 50;
// Expire a bare-Zephyr hint so literal dictated "zephyr" cannot stay armed.
const TAIL_PREFIX_GRACE_MS: u64 = 1_500;
#[cfg(any(target_os = "windows", test))]
const TAIL_PREFIX_NEW_AUDIO_SAMPLES: usize = SAMPLE_RATE as usize / 20;
const TAIL_WINDOW_SAMPLES: usize = SAMPLE_RATE as usize * 3;
// Sherpa receives only new suffix audio during normal incremental decoding.
// This two-second snapshot is reset/recovery backfill; native Sherpa separately
// enforces same maximum decoder-state horizon with overlapping stream epochs.
const FAST_TAIL_WINDOW_SAMPLES: usize = SAMPLE_RATE as usize * 3;
// Initial silence = accidental fire. Once voice has been heard, a long pause is
// end-of-dictation, not a cancellation.
const INITIAL_SILENCE_DISCARD_MS: u64 = 30_000;
const POST_SPEECH_SILENCE_STOP_MS: u64 = 60_000;
// F7 (Sol audit 2026-07-16): continuous speech with no pause never trips
// POST_SPEECH_SILENCE_STOP_MS above, so the recording PCM buffer would grow
// unbounded (~230MB/hour at 16kHz f32) and the eventual final decode gets
// increasingly expensive. Hard ceiling regardless of silence state; a
// recording that hits this auto-stops through the SAME mechanism as
// post-speech silence (buffer is transcribed and delivered, not discarded —
// see the `duration_cap_hit` branch in worker_streaming.rs).
const MAX_RECORDING_SECONDS: u64 = 30 * 60;
const MAX_RECORDING_SAMPLES: usize = MAX_RECORDING_SECONDS as usize * SAMPLE_RATE as usize;
const WAKE_DIAGNOSTICS_PREFIX: &str = "HR_WAKE_DIAGNOSTICS_JSON=";
const WAKE_DIAGNOSTICS_INTERVAL: Duration = Duration::from_secs(1);

/// Numeric-only idle-wake evidence. It deliberately contains no waveform,
/// transcript, candidate text, device name, or path. Sherpa's KWS C API does
/// not expose a confidence score for non-fires, so `score_available` remains
/// false until its scorer API grows one.
#[derive(Debug)]
struct WakeDiagnostics {
    started_at: Instant,
    last_emit: Instant,
    threshold: f32,
    mic_opened: bool,
    audio_samples: usize,
    audio_square_sum: f64,
    audio_peak: f32,
    chunks: u64,
    decode_attempts: u64,
    decode_ms_total: u64,
    decode_ms_max: u64,
    fire_count: u64,
    error_count: u64,
}

impl WakeDiagnostics {
    fn new(threshold: f32, mic_opened: bool) -> Self {
        let now = Instant::now();
        Self {
            started_at: now,
            last_emit: now,
            threshold,
            mic_opened,
            audio_samples: 0,
            audio_square_sum: 0.0,
            audio_peak: 0.0,
            chunks: 0,
            decode_attempts: 0,
            decode_ms_total: 0,
            decode_ms_max: 0,
            fire_count: 0,
            error_count: 0,
        }
    }

    fn started(threshold: f32) -> Self {
        let mut diagnostics = Self::new(threshold, true);
        diagnostics.emit("started", "started", "info", "wake_started");
        diagnostics
    }

    fn startup_failure(threshold: f32, code: &'static str) {
        let mut diagnostics = Self::new(threshold, false);
        diagnostics.error_count = 1;
        diagnostics.emit("startup", "error", "error", code);
    }

    fn observe_audio(&mut self, samples: &[f32]) {
        self.audio_samples = self.audio_samples.saturating_add(samples.len());
        self.chunks = self.chunks.saturating_add(1);
        for &sample in samples {
            let sample = sample as f64;
            self.audio_square_sum += sample * sample;
            self.audio_peak = self.audio_peak.max(sample.abs() as f32);
        }
        self.emit_if_due("observing", "no_match");
    }

    fn observe_decode(&mut self, decode_ms: u64) {
        self.decode_attempts = self.decode_attempts.saturating_add(1);
        self.decode_ms_total = self.decode_ms_total.saturating_add(decode_ms);
        self.decode_ms_max = self.decode_ms_max.max(decode_ms);
        self.emit_if_due("observing", "no_match");
    }

    fn fired(&mut self, fire_count: u64) {
        self.fire_count = fire_count;
        self.emit(
            "threshold_crossed",
            "fired",
            "info",
            "wake_threshold_crossed",
        );
    }

    fn failed(&mut self, stage: &'static str) {
        self.error_count = self.error_count.saturating_add(1);
        self.emit(stage, "error", "error", stage);
    }

    fn stopped(&mut self, fire_count: u64) {
        self.fire_count = fire_count;
        let (severity, code) = self.observation_status();
        self.emit("stopped", "stopped", severity, code);
    }

    fn emit_if_due(&mut self, phase: &'static str, result: &'static str) {
        if self.last_emit.elapsed() >= WAKE_DIAGNOSTICS_INTERVAL {
            let (severity, code) = self.observation_status();
            self.emit(phase, result, severity, code);
        }
    }

    fn observation_status(&self) -> (&'static str, &'static str) {
        if self.error_count > 0 {
            ("error", "wake_stopped_after_error")
        } else if self.audio_samples == 0 && self.started_at.elapsed() >= WAKE_DIAGNOSTICS_INTERVAL
        {
            ("warn", "wake_no_audio")
        } else if self.audio_samples > 0
            && self.decode_attempts == 0
            && self.started_at.elapsed() >= WAKE_DIAGNOSTICS_INTERVAL * 2
        {
            ("warn", "wake_no_kws_decode")
        } else {
            ("info", "wake_observing")
        }
    }

    fn emit(
        &mut self,
        phase: &'static str,
        result: &'static str,
        severity: &'static str,
        code: &'static str,
    ) {
        let payload = self.payload(phase, result, severity, code);
        if let Ok(serialized) = serde_json::to_string(&payload) {
            eprintln!("{WAKE_DIAGNOSTICS_PREFIX}{serialized}");
        }
        self.last_emit = Instant::now();
    }

    fn payload(
        &self,
        phase: &'static str,
        result: &'static str,
        severity: &'static str,
        code: &'static str,
    ) -> serde_json::Value {
        let audio_rms = if self.audio_samples == 0 {
            0.0
        } else {
            (self.audio_square_sum / self.audio_samples as f64).sqrt()
        };
        json!({
            "schema_version": 1,
            "event": "wake_diagnostic",
            "severity": severity,
            "code": code,
            "phase": phase,
            "result": result,
            "elapsed_ms": self.started_at.elapsed().as_millis() as u64,
            "threshold": self.threshold,
            "score_available": false,
            "mic_opened": self.mic_opened,
            "audio_samples": self.audio_samples,
            "audio_rms": audio_rms,
            "audio_peak": self.audio_peak,
            "chunks": self.chunks,
            "kws_decode_attempts": self.decode_attempts,
            "kws_decode_ms_total": self.decode_ms_total,
            "kws_decode_ms_max": self.decode_ms_max,
            "threshold_crossings": self.fire_count,
            "error_count": self.error_count,
        })
    }
}

enum WorkerCapture {
    Live(CaptureSession),
    #[cfg(test)]
    Replay(ReplayCapture),
}

impl WorkerCapture {
    fn is_replay(&self) -> bool {
        #[cfg(test)]
        if matches!(self, Self::Replay(_)) {
            return true;
        }
        false
    }

    fn read_f32(&mut self, max_samples: usize) -> Result<Vec<f32>, String> {
        match self {
            Self::Live(capture) => capture
                .read_f32(max_samples)
                .map_err(|error| error.to_string()),
            #[cfg(test)]
            Self::Replay(capture) => capture
                .read_f32(max_samples)
                .map_err(|error| error.to_string()),
        }
    }

    fn read_f32_blocking(
        &mut self,
        min_samples: usize,
        timeout_ms: u64,
    ) -> Result<Vec<f32>, String> {
        match self {
            Self::Live(capture) => capture
                .read_f32_blocking(min_samples, timeout_ms)
                .map_err(|error| error.to_string()),
            #[cfg(test)]
            Self::Replay(capture) => capture
                .read_f32_blocking(min_samples)
                .map_err(|error| error.to_string()),
        }
    }

    fn wait_first_buffer(
        &mut self,
        min_samples: usize,
        timeout_ms: u64,
    ) -> Result<CaptureFirstBuffer, String> {
        match self {
            Self::Live(capture) => capture
                .wait_first_buffer(min_samples, timeout_ms)
                .map_err(|error| error.to_string()),
            #[cfg(test)]
            Self::Replay(capture) => capture
                .wait_first_buffer(min_samples)
                .map_err(|error| error.to_string()),
        }
    }

    fn wait_for_audio(&mut self, timeout: Duration) -> bool {
        match self {
            Self::Live(capture) => capture.wait_for_audio(timeout),
            #[cfg(test)]
            Self::Replay(capture) => capture.wait_for_audio(),
        }
    }

    fn flush(&mut self) -> usize {
        match self {
            Self::Live(capture) => capture.flush(),
            #[cfg(test)]
            Self::Replay(capture) => capture.flush(),
        }
    }

    fn pause(&self) {
        match self {
            Self::Live(capture) => capture.pause(),
            #[cfg(test)]
            Self::Replay(capture) => capture.pause(),
        }
    }

    fn resume(&self) -> Result<(), String> {
        match self {
            Self::Live(capture) => capture.resume().map_err(|error| error.to_string()),
            #[cfg(test)]
            Self::Replay(capture) => capture.resume().map_err(|error| error.to_string()),
        }
    }

    fn stop(&mut self) -> MetricsSnapshot {
        match self {
            Self::Live(capture) => capture.stop(),
            #[cfg(test)]
            Self::Replay(capture) => capture.stop(),
        }
    }

    fn metrics_snapshot(&self) -> MetricsSnapshot {
        match self {
            Self::Live(capture) => capture.metrics_snapshot(),
            #[cfg(test)]
            Self::Replay(capture) => capture.metrics_snapshot(),
        }
    }

    fn input_rate(&self) -> u32 {
        match self {
            Self::Live(capture) => capture.input_rate(),
            #[cfg(test)]
            Self::Replay(_) => SAMPLE_RATE,
        }
    }

    fn input_channels(&self) -> u16 {
        match self {
            Self::Live(capture) => capture.input_channels(),
            #[cfg(test)]
            Self::Replay(_) => 1,
        }
    }

    fn input_sample_format(&self) -> &str {
        match self {
            Self::Live(capture) => capture.input_sample_format(),
            #[cfg(test)]
            Self::Replay(_) => "F32 replay",
        }
    }
}

#[cfg(test)]
#[derive(Default)]
struct ReplayCaptureState {
    driver: Option<ReplayDriver>,
    total_samples: usize,
    cursor: usize,
    paused: bool,
    submitted_total: usize,
    processed_total: usize,
    max_probe_ms: u64,
    reads: u64,
}

#[cfg(test)]
#[derive(Clone)]
struct ReplayCaptureController {
    state: Arc<(std::sync::Mutex<ReplayCaptureState>, std::sync::Condvar)>,
    clock: Arc<worker_clock::VirtualWorkerClock>,
}

#[cfg(test)]
impl Default for ReplayCaptureController {
    fn default() -> Self {
        Self {
            state: Arc::new((
                std::sync::Mutex::new(ReplayCaptureState::default()),
                std::sync::Condvar::new(),
            )),
            clock: Arc::new(worker_clock::VirtualWorkerClock::default()),
        }
    }
}

#[cfg(test)]
impl ReplayCaptureController {
    fn load(&self, audio: Vec<f32>) {
        self.load_scripted(audio, Vec::new());
    }

    fn load_scripted(&self, mut audio: Vec<f32>, scripted: Vec<ScriptedCaptureEvent>) {
        audio.resize(
            audio.len() + crate::sherpa_kws::CONFIRM_TIMEOUT_SAMPLES,
            0.0,
        );
        let total_samples = audio.len();
        let driver =
            ReplayDriver::from_samples(SAMPLE_RATE, audio, scripted, Arc::clone(&self.clock))
                .expect("valid replay samples");
        let (state, ready) = &*self.state;
        let mut state = state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *state = ReplayCaptureState {
            driver: Some(driver),
            total_samples,
            paused: true,
            ..ReplayCaptureState::default()
        };
        ready.notify_all();
    }

    fn virtual_clock(&self) -> Arc<worker_clock::VirtualWorkerClock> {
        Arc::clone(&self.clock)
    }

    fn pop_scripted_control(&self) -> Option<CaptureEvent> {
        let mut state = self
            .state
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.driver.as_mut().and_then(ReplayDriver::pop_control)
    }

    fn mark_lane_in_flight(
        &self,
        lane: worker_clock::ExecutorLane,
        submitted_at: worker_clock::WorkerInstant,
    ) {
        let (state, ready) = &*self.state;
        state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .driver
            .as_mut()
            .expect("replay loaded before lane submission")
            .mark_lane_submitted(lane, submitted_at)
            .unwrap_or_else(|error| panic!("invalid replay lane submission: {error:?}"));
        ready.notify_all();
    }

    fn schedule_lane_completion(&self, completion: LaneCompletion) {
        let (state, ready) = &*self.state;
        state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .driver
            .as_mut()
            .expect("replay loaded before lane completion")
            .supply_lane_duration(
                completion.lane,
                completion.submitted_at,
                completion.measured_decode_duration,
            )
            .unwrap_or_else(|error| panic!("invalid replay lane completion: {error:?}"));
        ready.notify_all();
    }

    fn lane_completion_ready(
        &self,
        lane: worker_clock::ExecutorLane,
        submitted_at: worker_clock::WorkerInstant,
    ) -> bool {
        let (state_mutex, ready) = &*self.state;
        let mut state = state_mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        loop {
            if state
                .driver
                .as_mut()
                .expect("replay loaded before lane readiness")
                .take_lane_completion(lane, submitted_at)
            {
                ready.notify_all();
                return true;
            }
            state = ready
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }

    fn finished(&self) -> Option<u64> {
        let state = self
            .state
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        (state.cursor == state.total_samples && state.submitted_total <= state.processed_total)
            .then_some(state.max_probe_ms)
    }

    fn debug_state(&self) -> String {
        let state = self
            .state
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        format!(
            "cursor={}/{} submitted={} processed={} paused={} reads={}",
            state.cursor,
            state.total_samples,
            state.submitted_total,
            state.processed_total,
            state.paused,
            state.reads
        )
    }

    fn note_submitted(&self, total_samples: usize) {
        let (state, ready) = &*self.state;
        let mut state = state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.submitted_total = state.submitted_total.max(total_samples);
        ready.notify_all();
    }

    fn note_processed(&self, total_samples: usize, probe_ms: u64) {
        let (state, ready) = &*self.state;
        let mut state = state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.processed_total = state.processed_total.max(total_samples);
        state.max_probe_ms = state.max_probe_ms.max(probe_ms);
        ready.notify_all();
    }
}

#[cfg(test)]
#[derive(Clone)]
struct ReplayCapture {
    controller: ReplayCaptureController,
}

#[cfg(test)]
impl ReplayCapture {
    fn read_f32(&mut self, max_samples: usize) -> anyhow::Result<Vec<f32>> {
        let (state_mutex, ready) = &*self.controller.state;
        let mut state = state_mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.paused {
            return Ok(Vec::new());
        }
        let block = state
            .driver
            .as_mut()
            .expect("replay loaded before capture")
            .read_audio(max_samples)
            .map_err(|error| anyhow::anyhow!("virtual replay clock: {error:?}"))?
            .unwrap_or_default();
        state.cursor = state.cursor.saturating_add(block.len());
        state.reads = state.reads.saturating_add((!block.is_empty()) as u64);
        ready.notify_all();
        Ok(block)
    }

    fn read_f32_blocking(&mut self, min_samples: usize) -> anyhow::Result<Vec<f32>> {
        let mut output = Vec::new();
        while output.len() < min_samples {
            let chunk = self.read_f32(min_samples.saturating_sub(output.len()))?;
            if chunk.is_empty() {
                break;
            }
            output.extend(chunk);
        }
        Ok(output)
    }

    fn wait_first_buffer(&mut self, min_samples: usize) -> anyhow::Result<CaptureFirstBuffer> {
        let audio = self.read_f32_blocking(min_samples)?;
        Ok(if audio.is_empty() {
            CaptureFirstBuffer::NoSamples
        } else {
            CaptureFirstBuffer::Data(audio)
        })
    }

    fn wait_for_audio(&self) -> bool {
        let (state_mutex, ready) = &*self.controller.state;
        let mut state = state_mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        while !state.paused
            && state.cursor < state.total_samples
            && state
                .driver
                .as_ref()
                .is_some_and(ReplayDriver::is_waiting_for_lane_duration)
        {
            state = ready
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        !state.paused && state.cursor < state.total_samples
    }

    fn flush(&mut self) -> usize {
        0
    }

    fn pause(&self) {
        let (state_mutex, ready) = &*self.controller.state;
        state_mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .paused = true;
        ready.notify_all();
    }

    fn resume(&self) -> anyhow::Result<()> {
        let (state_mutex, ready) = &*self.controller.state;
        state_mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .paused = false;
        ready.notify_all();
        Ok(())
    }

    fn stop(&mut self) -> MetricsSnapshot {
        self.pause();
        self.metrics_snapshot()
    }

    fn metrics_snapshot(&self) -> MetricsSnapshot {
        let state = self
            .controller
            .state
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        MetricsSnapshot {
            input_overflow_count: 0,
            dropped_blocks: 0,
            total_samples_emitted: state.cursor as u64,
            callback_invocations: state.reads,
            input_samples_received: state.cursor as u64,
            first_callback_latency_us: Some(0),
            async_error_count: 0,
            last_async_error_kind: None,
            terminal_error_kind: None,
            mean_callback_duration_us: 0.0,
            max_callback_duration_us: 0,
            downmix_locked_channel: None,
        }
    }
}

#[cfg(test)]
mod replay_capture_adapter_tests {
    use super::*;
    use worker_clock::WorkerClock;

    #[test]
    fn worker_capture_reads_sample_timed_audio_and_queues_control() {
        let controller = ReplayCaptureController::default();
        controller.load_scripted(
            vec![0.0; 640],
            vec![ScriptedCaptureEvent {
                at_sample: 320,
                kind: wav_replay_source::ScriptedCaptureEventKind::Control("stop".into()),
            }],
        );
        let clock = controller.virtual_clock();
        let mut capture = ReplayCapture {
            controller: controller.clone(),
        };
        capture.resume().unwrap();

        assert_eq!(capture.read_f32(320).unwrap().len(), 320);
        assert_eq!(capture.read_f32(320).unwrap().len(), 320);
        assert!(matches!(
            controller.pop_scripted_control(),
            Some(CaptureEvent::Control { name, .. }) if name == "stop"
        ));
        assert_eq!(
            clock
                .now()
                .duration_since(worker_clock::WorkerInstant::ZERO),
            Duration::from_millis(20)
        );
    }

    #[test]
    fn worker_capture_pause_stops_replay_reads() {
        let controller = ReplayCaptureController::default();
        controller.load(vec![0.0; 320]);
        let mut capture = ReplayCapture {
            controller: controller.clone(),
        };
        capture.resume().unwrap();
        capture.pause();

        assert!(capture.read_f32(320).unwrap().is_empty());
        assert!(controller.debug_state().contains("paused=true"));
    }

    #[test]
    fn worker_capture_releases_lane_only_at_virtual_completion() {
        let controller = ReplayCaptureController::default();
        controller.load(vec![0.0; 640]);
        let clock = controller.virtual_clock();
        let mut capture = ReplayCapture {
            controller: controller.clone(),
        };
        capture.resume().unwrap();
        assert_eq!(capture.read_f32(320).unwrap().len(), 320);

        let submitted_at = clock.now();
        controller.mark_lane_in_flight(worker_clock::ExecutorLane::Kws, submitted_at);
        controller.schedule_lane_completion(LaneCompletion {
            lane: worker_clock::ExecutorLane::Kws,
            submitted_at,
            measured_decode_duration: Duration::from_millis(10),
        });
        assert!(capture.read_f32(320).unwrap().is_empty());
        assert!(controller.lane_completion_ready(worker_clock::ExecutorLane::Kws, submitted_at,));
        assert_eq!(capture.read_f32(320).unwrap().len(), 320);
        assert_eq!(
            clock.now().duration_since(submitted_at),
            Duration::from_millis(20)
        );
    }

    #[test]
    fn capture_processes_audio_that_arrives_before_decoder_completion() {
        let controller = ReplayCaptureController::default();
        controller.load(vec![0.0; 640]);
        let clock = controller.virtual_clock();
        let mut capture = ReplayCapture {
            controller: controller.clone(),
        };
        capture.resume().unwrap();
        assert_eq!(capture.read_f32(320).unwrap().len(), 320);

        let submitted_at = clock.now();
        controller.mark_lane_in_flight(worker_clock::ExecutorLane::Kws, submitted_at);
        controller.schedule_lane_completion(LaneCompletion {
            lane: worker_clock::ExecutorLane::Kws,
            submitted_at,
            measured_decode_duration: Duration::from_millis(30),
        });

        assert_eq!(capture.read_f32(320).unwrap().len(), 320);
        assert!(capture.read_f32(320).unwrap().is_empty());
        assert!(controller.lane_completion_ready(worker_clock::ExecutorLane::Kws, submitted_at));
        assert_eq!(
            clock.now().duration_since(submitted_at),
            Duration::from_millis(30)
        );
    }
}

#[cfg(test)]
static REPLAY_CAPTURE: OnceLock<std::sync::Mutex<Option<ReplayCaptureController>>> =
    OnceLock::new();

#[cfg(test)]
static TRIGGER_REPLAY_ACTIVE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
fn install_trigger_replay() -> ReplayCaptureController {
    let controller = ReplayCaptureController::default();
    *REPLAY_CAPTURE
        .get_or_init(|| std::sync::Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(controller.clone());
    TRIGGER_REPLAY_ACTIVE.store(true, std::sync::atomic::Ordering::Release);
    controller
}

#[cfg(test)]
fn replay_capture() -> Option<ReplayCapture> {
    REPLAY_CAPTURE
        .get_or_init(|| std::sync::Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
        .map(|controller| ReplayCapture { controller })
}

#[cfg(test)]
fn replay_note_probe_submitted(total_samples: usize) {
    if let Some(controller) = REPLAY_CAPTURE
        .get_or_init(|| std::sync::Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
    {
        controller.note_submitted(total_samples);
    }
}

#[cfg(test)]
fn replay_note_probe_processed(total_samples: usize, probe_ms: u64) {
    if let Some(controller) = REPLAY_CAPTURE
        .get_or_init(|| std::sync::Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
    {
        controller.note_processed(total_samples, probe_ms);
    }
}

#[cfg(test)]
fn replay_controller() -> Option<ReplayCaptureController> {
    REPLAY_CAPTURE
        .get_or_init(|| std::sync::Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

#[cfg(test)]
fn replay_mark_lane_in_flight(
    lane: worker_clock::ExecutorLane,
    submitted_at: worker_clock::WorkerInstant,
) {
    if let Some(controller) = replay_controller() {
        controller.mark_lane_in_flight(lane, submitted_at);
    }
}

#[cfg(test)]
fn replay_schedule_lane_completion(completion: LaneCompletion) {
    if let Some(controller) = replay_controller() {
        controller.schedule_lane_completion(completion);
    }
}

#[cfg(test)]
fn replay_lane_completion_ready(completion: LaneCompletion) -> bool {
    replay_controller().is_none_or(|controller| {
        controller.lane_completion_ready(completion.lane, completion.submitted_at)
    })
}

// Command probes are useful operational diagnostics, but the recognized words
// are user content. Keep the event shape/timing while never writing dictated
// text into the ordinary local diagnostic log.
fn command_log_text(_text: &str) -> &'static str {
    "[redacted:diagnostics]"
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SilenceDecision {
    KeepRecording,
    DiscardInitialSilence,
    StopAfterPostSpeechSilence,
}

fn silence_decision(heard_voice: bool, silence_for: Duration) -> SilenceDecision {
    if !heard_voice && silence_for >= Duration::from_millis(INITIAL_SILENCE_DISCARD_MS) {
        return SilenceDecision::DiscardInitialSilence;
    }
    if heard_voice && silence_for >= Duration::from_millis(POST_SPEECH_SILENCE_STOP_MS) {
        return SilenceDecision::StopAfterPostSpeechSilence;
    }
    SilenceDecision::KeepRecording
}

/// F7: hard duration cap, independent of the heard_voice/silence state
/// `silence_decision` tracks. Checked alongside `StopAfterPostSpeechSilence`
/// at the call site so a recording that hits the cap stops through the exact
/// same transcribe+deliver path as a normal post-speech-silence stop.
fn recording_duration_cap_exceeded(buffer_samples: usize) -> bool {
    buffer_samples >= MAX_RECORDING_SAMPLES
}

#[derive(Debug)]
pub enum WorkerCmd {
    StartWakeListen {
        threshold: f32,
        reply: Sender<Result<(), String>>,
    },
    StopWakeListen {
        reply: Sender<u64>,
    },
    StartRecording {
        session_id: String,
    },
    StopRecording {
        send_enter: bool,
    },
    Cancel,
    /// Reload the ASR model if the configured backend/language changed.
    /// Callers that care about readiness pass a reply channel and wait for it
    /// before allowing recording to start.
    ReloadModel {
        reply: Option<Sender<Result<bool, String>>>,
    },
    TranscribeFile {
        path: PathBuf,
        reply: Sender<Result<FileTranscript, String>>,
    },
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum WorkerEvent {
    /// The model failed to load; the worker exits after emitting this.
    StartFailed { message: String },
    WakeFired {
        ts: f64,
        fire_count: u64,
        threshold: f32,
    },
    /// Real-time recording amplitude for the pill glow.
    RecordingLevel { session_id: String, level: f32 },
    /// Capture could not become live after one bounded cold reopen.
    CaptureFailed { session_id: String, message: String },
    /// Final transcript (or error) for the active session.
    TranscriptFinal {
        session_id: String,
        result: Result<String, String>,
        audio_secs: Option<f32>,
        stage_metrics: RecordingStageMetrics,
    },
    /// The worker detected a standalone command or a `zephyr stop/send/cancel`
    /// tail mid-recording and auto-stopped. `AutoStopBegin` moves the runtime to
    /// Transcribing before final ASR; this result then finalizes it.
    /// For a `zephyr send/stop` control tail the worker has ALREADY stripped the
    /// trigger from `result` using the intent it recognized at fire time (so a
    /// wake-word mis-transcription at delivery can't leave the trigger in the
    /// pasted text); `send_enter` carries the Enter intent that the stripped text
    /// no longer encodes. For a standalone command / silence auto-stop, `result`
    /// is the raw transcript and finalize recognizes+dispatches as on a manual stop.
    AutoStop {
        session_id: String,
        result: Result<String, String>,
        audio_secs: Option<f32>,
        send_enter: bool,
        stage_metrics: RecordingStageMetrics,
    },
    /// The 30s-of-silence backstop tripped — discard the recording (no delivery).
    RunawayDiscard { session_id: String },
    /// A hands-free control tail (`zephyr stop/send`) or standalone command was
    /// recognized and recording has stopped, but the final full-buffer transcribe
    /// or command dispatch is still ahead. Emitted first so the shell can leave
    /// `Recording`, put the PTT hook into release-suppression, and show
    /// "processing" immediately. The `AutoStop` with the transcript follows.
    AutoStopBegin {
        session_id: String,
        send_enter: bool,
    },
}

#[derive(Clone)]
pub struct WorkerHandle {
    pub cmd_tx: Sender<WorkerCmd>,
    pub event_rx: Arc<Mutex<Receiver<WorkerEvent>>>,
}

impl WorkerHandle {
    pub fn send(&self, cmd: WorkerCmd) -> Result<(), String> {
        self.cmd_tx
            .send(cmd)
            .map_err(|_| "worker command channel disconnected".to_string())
    }

    pub fn reload_model(&self) -> Result<bool, String> {
        let (reply_tx, reply_rx) = channel();
        self.send(WorkerCmd::ReloadModel {
            reply: Some(reply_tx),
        })?;
        reply_rx
            .recv_timeout(Duration::from_secs(30))
            .map_err(|error| match error {
                RecvTimeoutError::Timeout => "worker reload timed out".to_string(),
                RecvTimeoutError::Disconnected => "worker reload reply dropped".to_string(),
            })?
    }

    pub fn start_wake_listen(&self, threshold: f32) -> Result<(), String> {
        let (reply, result) = channel();
        self.send(WorkerCmd::StartWakeListen { threshold, reply })?;
        result
            .recv_timeout(Duration::from_secs(30))
            .map_err(|_| "worker wake start timed out".to_string())?
    }

    pub fn stop_wake_listen(&self) -> Result<u64, String> {
        let (reply, result) = channel();
        self.send(WorkerCmd::StopWakeListen { reply })?;
        result
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| "worker wake stop timed out".to_string())
    }

    /// Run a file transcription on the worker thread and wait for the result.
    /// Callable on a cloned handle so the IPC layer never holds the
    /// `EngineRuntime` mutex across this (multi-second) round-trip.
    pub fn transcribe_file(&self, path: PathBuf) -> Result<FileTranscript, String> {
        let (reply_tx, reply_rx) = channel();
        self.send(WorkerCmd::TranscribeFile {
            path,
            reply: reply_tx,
        })?;
        reply_rx
            .recv_timeout(FILE_REPLY_TIMEOUT)
            .map_err(|error| match error {
                RecvTimeoutError::Timeout => "worker transcription timed out".to_string(),
                RecvTimeoutError::Disconnected => "worker transcription reply dropped".to_string(),
            })?
    }
}

/// Spawn the worker thread. The model is loaded synchronously in the new
/// thread; if it fails, a `StartFailed` event is emitted and the thread exits.
pub fn spawn_worker(
    models_dir: PathBuf,
    ep: AsrEp,
    focus: Arc<Mutex<FocusTracker>>,
) -> Result<WorkerHandle, String> {
    spawn_worker_with_clock(
        models_dir,
        ep,
        focus,
        Arc::new(worker_clock::MonotonicWorkerClock::new()),
    )
}

fn spawn_worker_with_clock(
    models_dir: PathBuf,
    ep: AsrEp,
    focus: Arc<Mutex<FocusTracker>>,
    decision_clock: Arc<dyn worker_clock::WorkerClock>,
) -> Result<WorkerHandle, String> {
    let (cmd_tx, cmd_rx) = channel::<WorkerCmd>();
    let (event_tx, event_rx) = channel::<WorkerEvent>();
    let (ready_tx, ready_rx) = channel::<Result<(), String>>();

    std::thread::Builder::new()
        .name("hr-engine-worker".to_string())
        .spawn(move || {
            worker_main(
                models_dir,
                ep,
                focus,
                cmd_rx,
                event_tx,
                ready_tx,
                decision_clock,
            )
        })
        .map_err(|e| format!("spawn worker: {e}"))?;

    match ready_rx.recv() {
        Ok(Ok(())) => {}
        Ok(Err(message)) => return Err(message),
        Err(_) => return Err("worker exited before reporting ready".to_string()),
    }

    Ok(WorkerHandle {
        cmd_tx,
        event_rx: Arc::new(Mutex::new(event_rx)),
    })
}

/// Spawn a worker without actually loading a model or touching capture.
/// Used by tests that exercise the runtime state machine without real ASR.
pub fn spawn_dummy_worker(focus: Arc<Mutex<FocusTracker>>) -> Result<WorkerHandle, String> {
    let (cmd_tx, cmd_rx) = channel::<WorkerCmd>();
    let (event_tx, event_rx) = channel::<WorkerEvent>();

    std::thread::Builder::new()
        .name("hr-engine-dummy-worker".to_string())
        .spawn(move || {
            // Loop on commands; ignore start/stop; exit only on Shutdown.
            // Holding the focus handle keeps the test reference alive.
            loop {
                match cmd_rx.recv() {
                    Ok(WorkerCmd::Shutdown) | Err(_) => break,
                    Ok(WorkerCmd::StartRecording { .. })
                    | Ok(WorkerCmd::StopRecording { .. })
                    | Ok(WorkerCmd::Cancel) => {}
                    Ok(WorkerCmd::StartWakeListen { reply, .. }) => {
                        let _ = reply.send(Ok(()));
                    }
                    Ok(WorkerCmd::StopWakeListen { reply }) => {
                        let _ = reply.send(0);
                    }
                    Ok(WorkerCmd::ReloadModel { reply }) => {
                        if let Some(reply) = reply {
                            let _ = reply.send(Ok(false));
                        }
                    }
                    Ok(WorkerCmd::TranscribeFile { reply, .. }) => {
                        let _ = reply.send(Ok(FileTranscript {
                            text: String::new(),
                            srt: String::new(),
                            vtt: String::new(),
                            words: Vec::new(),
                        }));
                    }
                }
            }
            let _ = event_tx;
            let _ = focus;
        })
        .map_err(|e| format!("spawn dummy worker: {e}"))?;

    Ok(WorkerHandle {
        cmd_tx,
        event_rx: Arc::new(Mutex::new(event_rx)),
    })
}

/// Reload the ASR model iff the configured backend/language changed since it
/// was last loaded, then WARM it (one throwaway inference) so the first real
/// recording doesn't eat the CoreML graph-compile cost. Returns Ok(true) if it
/// reloaded, Ok(false) if no change. On error the caller keeps the existing
/// model.
fn reload_asr_if_changed(
    model: &mut AsrRuntime,
    last_model_key: &mut String,
    models_dir: &Path,
    ep: AsrEp,
) -> Result<bool, String> {
    let cur_model_key = asr_reload_key();
    if cur_model_key == *last_model_key {
        return Ok(false);
    }
    let warm = Instant::now();
    let mut m = AsrRuntime::load(models_dir, ep)?;
    warm_main_asr(&mut m).map_err(|error| format!("ASR model warmup failed: {error}"))?;
    *model = m;
    *last_model_key = cur_model_key;
    tracing::info!(
        "worker ASR model reloaded+warmed for key={} in {} ms",
        last_model_key,
        warm.elapsed().as_millis()
    );
    Ok(true)
}

/// Probe model is one fixed bundled Sherpa payload on macOS & Windows,
/// deliberately outside backend/language settings reloads.
fn reload_probe_asr_if_changed(
    model: &mut AsrRuntime,
    last_model_key: &mut String,
    models_dir: &Path,
    ep: AsrEp,
) -> Result<bool, String> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        let _ = (model, last_model_key, models_dir, ep);
        Ok(false)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    reload_asr_if_changed(model, last_model_key, models_dir, ep)
}

fn asr_reload_key() -> String {
    format!(
        "{}|{}",
        crate::settings::asr_backend(),
        crate::settings::dictation_language()
    )
}
