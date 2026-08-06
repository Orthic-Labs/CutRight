//! Cross-platform Sherpa-ONNX streaming keyword probe.
//!
//! macOS & Windows use identical model, graph, thresholds, CPU thread count,
//! active paths, VAD pre-roll & timestamp cut policy. Primary dictation ASR
//! remains independent.

use std::ffi::{c_char, c_float, c_int, CStr, CString};
use std::path::{Path, PathBuf};

use libloading::Library;
use parakeet_rs::{TimedToken, TranscriptionResult};
use serde::Deserialize;

const SAMPLE_RATE: c_int = 16_000;
const FEATURE_DIM: c_int = 80;
pub const NUM_THREADS: c_int = 1;
pub const MAX_ACTIVE_PATHS: c_int = 8;
pub const NUM_TRAILING_BLANKS: c_int = 0;
const KEYWORDS_SCORE: c_float = 1.0;
const KEYWORDS_THRESHOLD: c_float = 0.25;
const KWS_FEED_CHUNK_SAMPLES: usize = SAMPLE_RATE as usize / 50;
const KWS_DECODE_STEP_SAMPLES: usize = SAMPLE_RATE as usize / 20;
const WAKE_WINDOW_SAMPLES: usize = SAMPLE_RATE as usize * 3 / 2;
const WAKE_DECODE_TAIL_SAMPLES: usize = SAMPLE_RATE as usize / 5;
const WAKE_REFRACTORY_SAMPLES: usize = SAMPLE_RATE as usize;
// Confirm needs the full allowed two-second keyword span before the wake
// timestamp. A 300 ms anchor clipped the shipped blended-Zephyr Send incident
// even though the higher-threshold confirm model recognized the intact window.
const CONFIRM_CONTEXT_SAMPLES: usize = SAMPLE_RATE as usize * 2;
pub const CONFIRM_STEP_SAMPLES: usize = SAMPLE_RATE as usize / 20;
pub const CONFIRM_TIMEOUT_SAMPLES: usize = SAMPLE_RATE as usize * 3 / 2;
/// A real two-word command completed in 0.72-1.12s across the 14-clip
/// production corpus. Reject paths spanning more than 2s so Sherpa cannot
/// combine an old wake-like prefix with a much later action word.
const MAX_KEYWORD_SPAN_SECONDS: f32 = 2.0;

const ENCODER: &str = "encoder-epoch-13-avg-2-chunk-16-left-64.int8.onnx";
const DECODER: &str = "decoder-epoch-13-avg-2-chunk-16-left-64.onnx";
const JOINER: &str = "joiner-epoch-13-avg-2-chunk-16-left-64.int8.onnx";

#[cfg(target_os = "macos")]
const SHERPA_LIBRARY: &str = "libsherpa-onnx-c-api.dylib";
#[cfg(target_os = "windows")]
const SHERPA_LIBRARY: &str = "sherpa-onnx-c-api.dll";

#[repr(C)]
struct OnlineTransducerModelConfig {
    encoder: *const c_char,
    decoder: *const c_char,
    joiner: *const c_char,
}

#[repr(C)]
struct OnlineParaformerModelConfig {
    encoder: *const c_char,
    decoder: *const c_char,
}

#[repr(C)]
struct OnlineZipformer2CtcModelConfig {
    model: *const c_char,
}

#[repr(C)]
struct OnlineNemoCtcModelConfig {
    model: *const c_char,
}

#[repr(C)]
struct OnlineToneCtcModelConfig {
    model: *const c_char,
}

#[repr(C)]
struct OnlineModelConfig {
    transducer: OnlineTransducerModelConfig,
    paraformer: OnlineParaformerModelConfig,
    zipformer2_ctc: OnlineZipformer2CtcModelConfig,
    tokens: *const c_char,
    num_threads: c_int,
    provider: *const c_char,
    debug: c_int,
    model_type: *const c_char,
    modeling_unit: *const c_char,
    bpe_vocab: *const c_char,
    tokens_buf: *const c_char,
    tokens_buf_size: c_int,
    nemo_ctc: OnlineNemoCtcModelConfig,
    t_one_ctc: OnlineToneCtcModelConfig,
}

#[repr(C)]
struct FeatureConfig {
    sample_rate: c_int,
    feature_dim: c_int,
}

#[repr(C)]
struct KeywordSpotterConfig {
    feat_config: FeatureConfig,
    model_config: OnlineModelConfig,
    max_active_paths: c_int,
    num_trailing_blanks: c_int,
    keywords_score: c_float,
    keywords_threshold: c_float,
    keywords_file: *const c_char,
    keywords_buf: *const c_char,
    keywords_buf_size: c_int,
}

enum KeywordSpotter {}
enum OnlineStream {}

type CreateSpotter = unsafe extern "C" fn(*const KeywordSpotterConfig) -> *const KeywordSpotter;
type DestroySpotter = unsafe extern "C" fn(*const KeywordSpotter);
type CreateStream = unsafe extern "C" fn(*const KeywordSpotter) -> *const OnlineStream;
type DestroyStream = unsafe extern "C" fn(*const OnlineStream);
type AcceptWaveform = unsafe extern "C" fn(*const OnlineStream, c_int, *const c_float, c_int);
type IsReady = unsafe extern "C" fn(*const KeywordSpotter, *const OnlineStream) -> c_int;
type Decode = unsafe extern "C" fn(*const KeywordSpotter, *const OnlineStream);
type GetResultJson =
    unsafe extern "C" fn(*const KeywordSpotter, *const OnlineStream) -> *const c_char;
type FreeResultJson = unsafe extern "C" fn(*const c_char);

struct SherpaApi {
    create_spotter: CreateSpotter,
    destroy_spotter: DestroySpotter,
    create_stream: CreateStream,
    destroy_stream: DestroyStream,
    accept_waveform: AcceptWaveform,
    is_ready: IsReady,
    decode: Decode,
    get_result_json: GetResultJson,
    free_result_json: FreeResultJson,
    _library: Library,
}

impl SherpaApi {
    unsafe fn load(path: &Path) -> Result<Self, String> {
        let library = unsafe { Library::new(path) }
            .map_err(|error| format!("load Sherpa runtime {}: {error}", path.display()))?;
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {
                *unsafe { library.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .map_err(|error| format!("Sherpa runtime symbol {}: {error}", $name))?
            };
        }
        Ok(Self {
            create_spotter: symbol!("SherpaOnnxCreateKeywordSpotter", CreateSpotter),
            destroy_spotter: symbol!("SherpaOnnxDestroyKeywordSpotter", DestroySpotter),
            create_stream: symbol!("SherpaOnnxCreateKeywordStream", CreateStream),
            destroy_stream: symbol!("SherpaOnnxDestroyOnlineStream", DestroyStream),
            accept_waveform: symbol!("SherpaOnnxOnlineStreamAcceptWaveform", AcceptWaveform),
            is_ready: symbol!("SherpaOnnxIsKeywordStreamReady", IsReady),
            decode: symbol!("SherpaOnnxDecodeKeywordStream", Decode),
            get_result_json: symbol!("SherpaOnnxGetKeywordResultAsJson", GetResultJson),
            free_result_json: symbol!("SherpaOnnxFreeKeywordResultJson", FreeResultJson),
            _library: library,
        })
    }
}

#[derive(Deserialize)]
struct KeywordResult {
    #[serde(default)]
    keyword: String,
    #[serde(default)]
    timestamps: Vec<f32>,
}

/// Optional third spotter: the hey-prefixed always-on wake word. Kept OUT of
/// [`validate_model_dir`] on purpose — a payload without this file must still
/// load the shipped tail cascade, so the pointer is simply null and
/// [`SherpaKws::hey_wake_available`] reports false.
const HEY_WAKE_KEYWORDS_FILE: &str = "keywords-hey-wake.txt";
const HEY_WAKE_KEYWORD: &str = "HEY_ZEPHYR";

pub struct SherpaKws {
    api: SherpaApi,
    model_dir: PathBuf,
    wake_spotter: *const KeywordSpotter,
    confirm_spotter: *const KeywordSpotter,
    /// Null when the payload predates the hey-wake keyword file.
    hey_wake_spotter: *const KeywordSpotter,
    hey_wake_available: bool,
    hey_wake_threshold: Option<f32>,
    idle_wake_stream: Option<WakeStream>,
    clock: CascadeClock,
    wake_stream: Option<WakeStream>,
    confirmation_stream: Option<ConfirmationStream>,
    latched: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WakeCandidate {
    start_sample: usize,
    end_sample: usize,
}

struct ConfirmationStream {
    stream: *const OnlineStream,
    candidate: WakeCandidate,
    seed_origin_sample: usize,
    fed_until_sample: usize,
    accepted_until_sample: usize,
    pending_audio: Vec<f32>,
    next_decode_sample: usize,
}

struct WakeStream {
    stream: *const OnlineStream,
    seed_origin_sample: usize,
    fed_until_sample: usize,
    accepted_until_sample: usize,
    pending_audio: Vec<f32>,
    next_decode_sample: usize,
}

#[derive(Debug, Eq, PartialEq)]
enum ConfirmationFeed {
    Duplicate,
    Suffix { offset: usize },
    Stale,
    Gap,
}

fn next_decode_boundary(sample: usize) -> usize {
    sample.saturating_add(KWS_DECODE_STEP_SAMPLES)
}

fn classify_confirmation_feed(
    fed_until_sample: usize,
    snapshot_start_sample: usize,
    snapshot_end_sample: usize,
) -> ConfirmationFeed {
    if snapshot_end_sample < fed_until_sample {
        ConfirmationFeed::Stale
    } else if fed_until_sample < snapshot_start_sample {
        ConfirmationFeed::Gap
    } else if snapshot_end_sample == fed_until_sample {
        ConfirmationFeed::Duplicate
    } else {
        ConfirmationFeed::Suffix {
            offset: fed_until_sample - snapshot_start_sample,
        }
    }
}

#[derive(Default)]
struct CascadeClock {
    candidate: Option<WakeCandidate>,
    last_wake_start_sample: Option<usize>,
}

impl CascadeClock {
    fn observe_wake(&mut self, start_sample: usize, end_sample: usize) -> bool {
        if self
            .last_wake_start_sample
            .is_some_and(|last| start_sample.abs_diff(last) < WAKE_REFRACTORY_SAMPLES)
        {
            return false;
        }
        self.candidate = Some(WakeCandidate {
            start_sample,
            end_sample: end_sample.max(start_sample),
        });
        self.last_wake_start_sample = Some(start_sample);
        true
    }

    fn candidate(&self) -> Option<WakeCandidate> {
        self.candidate
    }

    fn expire_if_needed(&mut self, live_sample: usize) -> bool {
        let expired = self.candidate.is_some_and(|candidate| {
            live_sample > candidate.end_sample.saturating_add(CONFIRM_TIMEOUT_SAMPLES)
        });
        if expired {
            self.candidate = None;
        }
        expired
    }

    fn clear(&mut self) {
        *self = Self::default();
    }
}

// Probe is created, mutated & destroyed by one ASR worker thread.
unsafe impl Send for SherpaKws {}

impl SherpaKws {
    pub fn load(model_dir: &Path) -> Result<Self, String> {
        validate_model_dir(model_dir)?;
        let library_path = resolve_library_path().ok_or_else(|| {
            format!("bundled Sherpa runtime {SHERPA_LIBRARY} missing beside ONNX Runtime")
        })?;
        let api = unsafe { SherpaApi::load(&library_path) }?;

        let wake_spotter = create_spotter(&api, model_dir, "keywords-wake.txt")?;
        let confirm_spotter = match create_spotter(&api, model_dir, "keywords-confirm.txt") {
            Ok(spotter) => spotter,
            Err(error) => {
                unsafe { (api.destroy_spotter)(wake_spotter) };
                return Err(error);
            }
        };
        let hey_wake_available = model_dir.join(HEY_WAKE_KEYWORDS_FILE).is_file();
        tracing::info!(
            "Sherpa KWS cascade loaded: model={} cpu_threads={} max_active_paths={} trailing_blanks={}",
            model_dir.display(),
            NUM_THREADS,
            MAX_ACTIVE_PATHS,
            NUM_TRAILING_BLANKS
        );
        Ok(Self {
            api,
            model_dir: model_dir.to_path_buf(),
            wake_spotter,
            confirm_spotter,
            hey_wake_spotter: std::ptr::null(),
            hey_wake_available,
            hey_wake_threshold: None,
            idle_wake_stream: None,
            clock: CascadeClock::default(),
            wake_stream: None,
            confirmation_stream: None,
            latched: false,
        })
    }

    /// True when the payload shipped `keywords-hey-wake.txt`. The spotter stays
    /// lazy until the marker-gated owner starts wake listening.
    pub fn hey_wake_available(&self) -> bool {
        self.hey_wake_available
    }

    pub fn start_idle_wake(&mut self, threshold: f32) -> Result<(), String> {
        if !self.hey_wake_available {
            return Err(format!("{HEY_WAKE_KEYWORDS_FILE} is not packaged"));
        }
        let threshold = threshold.clamp(0.01, 0.99);
        self.stop_idle_wake();
        if self
            .hey_wake_threshold
            .is_none_or(|current| (current - threshold).abs() > f32::EPSILON)
        {
            if !self.hey_wake_spotter.is_null() {
                unsafe { (self.api.destroy_spotter)(self.hey_wake_spotter) };
                self.hey_wake_spotter = std::ptr::null();
            }
            self.hey_wake_spotter = create_spotter_with_threshold(
                &self.api,
                &self.model_dir,
                HEY_WAKE_KEYWORDS_FILE,
                threshold,
            )?;
            self.hey_wake_threshold = Some(threshold);
        }
        let stream = unsafe { (self.api.create_stream)(self.hey_wake_spotter) };
        if stream.is_null() {
            return Err("Sherpa idle wake stream creation failed".into());
        }
        self.idle_wake_stream = Some(WakeStream {
            stream,
            seed_origin_sample: 0,
            fed_until_sample: 0,
            accepted_until_sample: 0,
            pending_audio: Vec::new(),
            next_decode_sample: KWS_DECODE_STEP_SAMPLES,
        });
        Ok(())
    }

    pub fn stop_idle_wake(&mut self) {
        if let Some(state) = self.idle_wake_stream.take() {
            unsafe { (self.api.destroy_stream)(state.stream) };
        }
    }

    pub fn feed_idle_wake(
        &mut self,
        audio: &[f32],
        start_sample: usize,
        live_total_sample: usize,
    ) -> Result<Option<(usize, usize)>, String> {
        let Some(state) = self.idle_wake_stream.as_ref() else {
            return Err("idle wake stream is not active".into());
        };
        if state.fed_until_sample != start_sample {
            return Err(format!(
                "idle wake PCM gap: expected={} actual={start_sample}",
                state.fed_until_sample
            ));
        }
        if start_sample.saturating_add(audio.len()) != live_total_sample {
            return Err(format!(
                "idle wake PCM cursor mismatch: start={start_sample} len={} total={live_total_sample}",
                audio.len()
            ));
        }
        let stream = state.stream;
        let seed_origin_sample = state.seed_origin_sample;
        self.accept_audio(stream, audio)?;
        self.idle_wake_stream
            .as_mut()
            .expect("idle wake stream exists")
            .fed_until_sample = live_total_sample;
        let Some(result) = self.decode_ready_result(self.hey_wake_spotter, stream)? else {
            return Ok(None);
        };
        if result.keyword.trim().trim_start_matches('@') != HEY_WAKE_KEYWORD {
            return Ok(None);
        }
        let start = seed_origin_sample.saturating_add(seconds_to_samples(
            result.timestamps.first().copied().unwrap_or(0.0),
        ));
        let end = seed_origin_sample
            .saturating_add(seconds_to_samples(
                result.timestamps.last().copied().unwrap_or(0.0),
            ))
            .saturating_add(SAMPLE_RATE as usize / 20);
        self.stop_idle_wake();
        Ok(Some((start, end.max(start))))
    }

    /// Score the trailing wake window for the hey-prefixed wake word.
    ///
    /// Independent of the tail cascade: no `CascadeClock` state, no latch, no
    /// action authority. Refractory/debounce is the caller's job because the
    /// idle listener owns its own sample clock. Threshold and boosting score
    /// come from `keywords-hey-wake.txt`, not from code.
    pub fn detect_hey_wake(&self, audio: &[f32]) -> Result<bool, String> {
        if self.hey_wake_spotter.is_null() {
            return Ok(false);
        }
        let window_start = audio.len().saturating_sub(WAKE_WINDOW_SAMPLES);
        let Some(hit) = self.decode_fresh(
            self.hey_wake_spotter,
            &audio[window_start..],
            WAKE_DECODE_TAIL_SAMPLES,
        )?
        else {
            return Ok(false);
        };
        Ok(hit.keyword.trim().trim_start_matches('@') == HEY_WAKE_KEYWORD)
    }

    pub fn transcribe(&mut self, audio: &[f32]) -> Result<String, String> {
        self.transcribe_result(audio).map(|result| result.text)
    }

    pub fn transcribe_result(&mut self, audio: &[f32]) -> Result<TranscriptionResult, String> {
        self.transcribe_cascade_result(audio, 0, audio.len(), false)
    }

    pub fn transcribe_fresh_confirmation_result(
        &mut self,
        audio: &[f32],
    ) -> Result<TranscriptionResult, String> {
        if self.latched {
            return Ok(empty_result());
        }
        let Some(result) = self.decode_fresh(self.confirm_spotter, audio, 0)? else {
            return Ok(empty_result());
        };
        if command_text(&result.keyword).is_none_or(|text| text == "zephyr") {
            return Ok(empty_result());
        }
        self.latched = true;
        self.destroy_confirmation_stream();
        Ok(transcription_result(result, 0))
    }

    pub fn transcribe_cascade_result(
        &mut self,
        audio: &[f32],
        snapshot_start_sample: usize,
        live_total_sample: usize,
        _confirmation_armed: bool,
    ) -> Result<TranscriptionResult, String> {
        if self.latched {
            return Ok(empty_result());
        }
        let snapshot_end_sample = match snapshot_start_sample.checked_add(audio.len()) {
            Some(end) => end,
            None => {
                self.invalidate_cascade();
                return Err("Sherpa KWS snapshot cursor overflow".into());
            }
        };
        if snapshot_end_sample != live_total_sample {
            self.invalidate_cascade();
            return Err("Sherpa KWS cascade snapshot cursor mismatch".into());
        }

        if self.clock.expire_if_needed(live_total_sample) {
            self.destroy_confirmation_stream();
        }
        let previous_candidate = self.clock.candidate();
        {
            let wake = match self.feed_wake(audio, snapshot_start_sample, live_total_sample) {
                Ok(wake) => wake,
                Err(error) => {
                    self.invalidate_cascade();
                    return Err(error);
                }
            };
            if let Some(wake) = wake {
                if wake.keyword.trim().trim_start_matches('@') == "ZEPHYR" {
                    let base = self
                        .wake_stream
                        .as_ref()
                        .map(|stream| stream.seed_origin_sample)
                        .unwrap_or(snapshot_start_sample);
                    let start = base.saturating_add(seconds_to_samples(
                        wake.timestamps.first().copied().unwrap_or(0.0),
                    ));
                    let end = base
                        .saturating_add(seconds_to_samples(
                            wake.timestamps.last().copied().unwrap_or(0.0),
                        ))
                        .saturating_add(SAMPLE_RATE as usize / 20);
                    self.clock.observe_wake(start, end);
                    if self.clock.candidate() != previous_candidate {
                        self.destroy_confirmation_stream();
                    }
                }
            }
        }

        let acoustic_candidate = self.clock.candidate();
        if let Some(candidate) = acoustic_candidate {
            let result = match self.feed_confirmation(
                audio,
                snapshot_start_sample,
                live_total_sample,
                candidate,
            ) {
                Ok(result) => result,
                Err(error) => {
                    self.invalidate_cascade();
                    return Err(error);
                }
            };
            if let Some(result) = result {
                if command_text(&result.keyword).is_some_and(|text| text != "zephyr") {
                    let seed_origin_sample = self
                        .confirmation_stream
                        .as_ref()
                        .map(|stream| stream.seed_origin_sample)
                        .unwrap_or(snapshot_start_sample);
                    self.latched = true;
                    self.destroy_confirmation_stream();
                    return Ok(transcription_result(
                        result,
                        seed_origin_sample.saturating_sub(snapshot_start_sample),
                    ));
                }
            }
        } else {
            self.destroy_confirmation_stream();
        }

        if let Some(candidate) = acoustic_candidate {
            let relative_start = candidate.start_sample.saturating_sub(snapshot_start_sample);
            return Ok(TranscriptionResult {
                text: "zephyr".into(),
                tokens: vec![TimedToken {
                    text: " zephyr".into(),
                    start: relative_start as f32 / SAMPLE_RATE as f32,
                    end: candidate.end_sample.saturating_sub(snapshot_start_sample) as f32
                        / SAMPLE_RATE as f32,
                }],
            });
        }
        Ok(empty_result())
    }

    fn feed_wake(
        &mut self,
        audio: &[f32],
        snapshot_start_sample: usize,
        live_total_sample: usize,
    ) -> Result<Option<KeywordResult>, String> {
        if self.wake_stream.is_none() {
            let stream = unsafe { (self.api.create_stream)(self.wake_spotter) };
            if stream.is_null() {
                return Err("Sherpa wake stream creation failed".into());
            }
            let next_decode_sample = next_decode_boundary(snapshot_start_sample);
            let (result, accepted_until_sample, pending_audio, next_decode_sample) = self
                .accept_and_decode_at_sample_boundaries(
                self.wake_spotter,
                stream,
                audio,
                snapshot_start_sample,
                snapshot_start_sample,
                Vec::new(),
                next_decode_sample,
            )?;
            self.wake_stream = Some(WakeStream {
                stream,
                seed_origin_sample: snapshot_start_sample,
                fed_until_sample: live_total_sample,
                accepted_until_sample,
                pending_audio,
                next_decode_sample,
            });
            return Ok(result);
        }

        let feed = {
            let state = self.wake_stream.as_ref().expect("wake stream exists");
            classify_confirmation_feed(
                state.fed_until_sample,
                snapshot_start_sample,
                live_total_sample,
            )
        };
        match feed {
            ConfirmationFeed::Duplicate => Ok(None),
            ConfirmationFeed::Stale => Err("Sherpa wake snapshot is stale".into()),
            ConfirmationFeed::Gap => {
                self.destroy_wake_stream();
                self.feed_wake(audio, snapshot_start_sample, live_total_sample)
            }
            ConfirmationFeed::Suffix { offset } => {
                let stream = self
                    .wake_stream
                    .as_ref()
                    .expect("wake stream exists")
                    .stream;
                let next_decode_sample = self
                    .wake_stream
                    .as_ref()
                    .expect("wake stream exists")
                    .next_decode_sample;
                let accepted_until_sample = self
                    .wake_stream
                    .as_ref()
                    .expect("wake stream exists")
                    .accepted_until_sample;
                let pending_audio = std::mem::take(
                    &mut self
                        .wake_stream
                        .as_mut()
                        .expect("wake stream exists")
                        .pending_audio,
                );
                let suffix_start = snapshot_start_sample.saturating_add(offset);
                let (result, accepted_until_sample, pending_audio, next_decode_sample) = self
                    .accept_and_decode_at_sample_boundaries(
                        self.wake_spotter,
                        stream,
                        &audio[offset..],
                        suffix_start,
                        accepted_until_sample,
                        pending_audio,
                        next_decode_sample,
                    )?;
                let state = self.wake_stream.as_mut().expect("wake stream exists");
                state.fed_until_sample = live_total_sample;
                state.accepted_until_sample = accepted_until_sample;
                state.pending_audio = pending_audio;
                state.next_decode_sample = next_decode_sample;
                Ok(result)
            }
        }
    }

    fn feed_confirmation(
        &mut self,
        audio: &[f32],
        snapshot_start_sample: usize,
        live_total_sample: usize,
        candidate: WakeCandidate,
    ) -> Result<Option<KeywordResult>, String> {
        if self
            .confirmation_stream
            .as_ref()
            .is_some_and(|stream| stream.candidate != candidate)
        {
            self.destroy_confirmation_stream();
        }

        if self.confirmation_stream.is_none() {
            let anchor_sample = candidate
                .start_sample
                .saturating_sub(CONFIRM_CONTEXT_SAMPLES)
                .max(snapshot_start_sample)
                .min(live_total_sample);
            let anchor_offset = anchor_sample - snapshot_start_sample;
            let stream = unsafe { (self.api.create_stream)(self.confirm_spotter) };
            if stream.is_null() {
                return Err("Sherpa confirmation stream creation failed".into());
            }
            let next_decode_sample = next_decode_boundary(anchor_sample);
            let (result, accepted_until_sample, pending_audio, next_decode_sample) = match self
                .accept_and_decode_at_sample_boundaries(
                self.confirm_spotter,
                stream,
                &audio[anchor_offset..],
                anchor_sample,
                anchor_sample,
                Vec::new(),
                next_decode_sample,
            ) {
                Ok(result) => result,
                Err(error) => {
                    unsafe { (self.api.destroy_stream)(stream) };
                    return Err(error);
                }
            };
            self.confirmation_stream = Some(ConfirmationStream {
                stream,
                candidate,
                seed_origin_sample: anchor_sample,
                fed_until_sample: live_total_sample,
                accepted_until_sample,
                pending_audio,
                next_decode_sample,
            });
            return Ok(result);
        }

        let feed = {
            let state = self
                .confirmation_stream
                .as_ref()
                .expect("confirmation stream checked above");
            classify_confirmation_feed(
                state.fed_until_sample,
                snapshot_start_sample,
                live_total_sample,
            )
        };
        match feed {
            ConfirmationFeed::Duplicate => Ok(None),
            ConfirmationFeed::Stale => Err("Sherpa confirmation snapshot is stale".into()),
            ConfirmationFeed::Gap => {
                self.destroy_confirmation_stream();
                self.feed_confirmation(audio, snapshot_start_sample, live_total_sample, candidate)
            }
            ConfirmationFeed::Suffix { offset } => {
                let suffix = &audio[offset..];
                let stream = self
                    .confirmation_stream
                    .as_ref()
                    .expect("confirmation stream checked above")
                    .stream;
                let next_decode_sample = self
                    .confirmation_stream
                    .as_ref()
                    .expect("confirmation stream checked above")
                    .next_decode_sample;
                let accepted_until_sample = self
                    .confirmation_stream
                    .as_ref()
                    .expect("confirmation stream checked above")
                    .accepted_until_sample;
                let pending_audio = std::mem::take(
                    &mut self
                        .confirmation_stream
                        .as_mut()
                        .expect("confirmation stream checked above")
                        .pending_audio,
                );
                let suffix_start = snapshot_start_sample.saturating_add(offset);
                let (result, accepted_until_sample, pending_audio, next_decode_sample) = self
                    .accept_and_decode_at_sample_boundaries(
                        self.confirm_spotter,
                        stream,
                        suffix,
                        suffix_start,
                        accepted_until_sample,
                        pending_audio,
                        next_decode_sample,
                    )?;
                let state = self
                    .confirmation_stream
                    .as_mut()
                    .expect("confirmation stream checked above");
                state.fed_until_sample = live_total_sample;
                state.accepted_until_sample = accepted_until_sample;
                state.pending_audio = pending_audio;
                state.next_decode_sample = next_decode_sample;
                Ok(result)
            }
        }
    }

    fn destroy_confirmation_stream(&mut self) {
        if let Some(state) = self.confirmation_stream.take() {
            unsafe { (self.api.destroy_stream)(state.stream) };
        }
    }

    fn destroy_wake_stream(&mut self) {
        if let Some(state) = self.wake_stream.take() {
            unsafe { (self.api.destroy_stream)(state.stream) };
        }
    }

    fn invalidate_cascade(&mut self) {
        self.destroy_wake_stream();
        self.destroy_confirmation_stream();
        self.clock.clear();
    }

    fn decode_fresh(
        &self,
        spotter: *const KeywordSpotter,
        audio: &[f32],
        tail_samples: usize,
    ) -> Result<Option<KeywordResult>, String> {
        let stream = unsafe { (self.api.create_stream)(spotter) };
        if stream.is_null() {
            return Err("Sherpa keyword stream creation failed".into());
        }
        let result = (|| {
            for chunk in audio.chunks(KWS_FEED_CHUNK_SAMPLES) {
                self.accept_audio(stream, chunk)?;
                if let Some(result) = self.decode_ready_result(spotter, stream)? {
                    return Ok(Some(result));
                }
            }
            let silence = vec![0.0; tail_samples];
            for chunk in silence.chunks(KWS_FEED_CHUNK_SAMPLES) {
                self.accept_audio(stream, chunk)?;
                if let Some(result) = self.decode_ready_result(spotter, stream)? {
                    return Ok(Some(result));
                }
            }
            Ok(None)
        })();
        unsafe { (self.api.destroy_stream)(stream) };
        result
    }

    fn accept_audio(&self, stream: *const OnlineStream, audio: &[f32]) -> Result<(), String> {
        if audio.is_empty() {
            return Ok(());
        }
        let count = c_int::try_from(audio.len())
            .map_err(|_| "Sherpa KWS audio chunk exceeds i32".to_string())?;
        unsafe {
            (self.api.accept_waveform)(stream, SAMPLE_RATE, audio.as_ptr(), count);
        }
        Ok(())
    }

    /// Keep native decoder checkpoints independent of executor backlog. Audio
    /// arrival stays event-driven; this only preserves model input geometry
    /// when latest-only mailbox replacement coalesces several capture blocks.
    fn accept_and_decode_at_sample_boundaries(
        &self,
        spotter: *const KeywordSpotter,
        stream: *const OnlineStream,
        audio: &[f32],
        audio_start_sample: usize,
        mut accepted_until_sample: usize,
        mut pending_audio: Vec<f32>,
        mut next_decode_sample: usize,
    ) -> Result<(Option<KeywordResult>, usize, Vec<f32>, usize), String> {
        let expected_start = accepted_until_sample.saturating_add(pending_audio.len());
        if audio_start_sample != expected_start {
            return Err(format!(
                "Sherpa KWS buffered cursor mismatch: expected={expected_start} got={audio_start_sample}"
            ));
        }
        pending_audio.extend_from_slice(audio);
        let mut first_result = None;
        let mut consumed = 0usize;
        loop {
            while next_decode_sample <= accepted_until_sample {
                next_decode_sample = next_decode_sample.saturating_add(KWS_DECODE_STEP_SAMPLES);
            }
            let needed = next_decode_sample.saturating_sub(accepted_until_sample);
            if pending_audio.len().saturating_sub(consumed) < needed {
                break;
            }
            self.accept_audio(stream, &pending_audio[consumed..consumed + needed])?;
            consumed += needed;
            accepted_until_sample = next_decode_sample;
            if let Some(result) = self.decode_ready_result(spotter, stream)? {
                first_result.get_or_insert(result);
            }
            next_decode_sample = next_decode_sample.saturating_add(KWS_DECODE_STEP_SAMPLES);
        }
        if consumed > 0 {
            pending_audio.drain(..consumed);
        }
        Ok((
            first_result,
            accepted_until_sample,
            pending_audio,
            next_decode_sample,
        ))
    }

    fn decode_ready_result(
        &self,
        spotter: *const KeywordSpotter,
        stream: *const OnlineStream,
    ) -> Result<Option<KeywordResult>, String> {
        let mut decode_steps = 0usize;
        while unsafe { (self.api.is_ready)(spotter, stream) } != 0 {
            unsafe { (self.api.decode)(spotter, stream) };
            decode_steps += 1;
            if decode_steps > 4_096 {
                return Err("Sherpa KWS decode readiness did not converge".into());
            }
            if let Some(result) = self.current_result(spotter, stream)? {
                if !keyword_timestamps_are_plausible(&result.timestamps) {
                    let start = result.timestamps.first().copied();
                    let end = result.timestamps.last().copied();
                    tracing::warn!(
                        keyword = result.keyword,
                        ?start,
                        ?end,
                        "Sherpa KWS rejected non-contiguous keyword path"
                    );
                    return Err(format!(
                        "Sherpa KWS keyword timing is non-contiguous: start={start:?} end={end:?}"
                    ));
                }
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    fn current_result(
        &self,
        spotter: *const KeywordSpotter,
        stream: *const OnlineStream,
    ) -> Result<Option<KeywordResult>, String> {
        let json_ptr = unsafe { (self.api.get_result_json)(spotter, stream) };
        if json_ptr.is_null() {
            return Err("Sherpa KWS returned null result JSON".into());
        }
        let json = unsafe { CStr::from_ptr(json_ptr) }
            .to_string_lossy()
            .into_owned();
        unsafe { (self.api.free_result_json)(json_ptr) };
        let result: KeywordResult = serde_json::from_str(&json)
            .map_err(|error| format!("parse Sherpa KWS result: {error}"))?;
        Ok(command_text(&result.keyword).map(|_| result))
    }

    pub fn reset_stream(&mut self) -> Result<(), String> {
        self.destroy_wake_stream();
        self.destroy_confirmation_stream();
        self.clock.clear();
        self.latched = false;
        Ok(())
    }
}

impl Drop for SherpaKws {
    fn drop(&mut self) {
        self.stop_idle_wake();
        self.destroy_wake_stream();
        self.destroy_confirmation_stream();
        unsafe {
            if !self.wake_spotter.is_null() {
                (self.api.destroy_spotter)(self.wake_spotter);
            }
            if !self.confirm_spotter.is_null() {
                (self.api.destroy_spotter)(self.confirm_spotter);
            }
            if !self.hey_wake_spotter.is_null() {
                (self.api.destroy_spotter)(self.hey_wake_spotter);
            }
        }
    }
}

fn create_spotter(
    api: &SherpaApi,
    model_dir: &Path,
    keywords_file: &str,
) -> Result<*const KeywordSpotter, String> {
    create_spotter_with_threshold(api, model_dir, keywords_file, KEYWORDS_THRESHOLD)
}

fn create_spotter_with_threshold(
    api: &SherpaApi,
    model_dir: &Path,
    keywords_file: &str,
    threshold: f32,
) -> Result<*const KeywordSpotter, String> {
    let encoder = path_cstring(&model_dir.join(ENCODER))?;
    let decoder = path_cstring(&model_dir.join(DECODER))?;
    let joiner = path_cstring(&model_dir.join(JOINER))?;
    let tokens = path_cstring(&model_dir.join("tokens.txt"))?;
    let keywords = path_cstring(&model_dir.join(keywords_file))?;
    let cpu = CString::new("cpu").expect("static CPU provider");
    let mut config: KeywordSpotterConfig = unsafe { std::mem::zeroed() };
    config.feat_config.sample_rate = SAMPLE_RATE;
    config.feat_config.feature_dim = FEATURE_DIM;
    config.model_config.transducer.encoder = encoder.as_ptr();
    config.model_config.transducer.decoder = decoder.as_ptr();
    config.model_config.transducer.joiner = joiner.as_ptr();
    config.model_config.tokens = tokens.as_ptr();
    config.model_config.num_threads = NUM_THREADS;
    config.model_config.provider = cpu.as_ptr();
    config.max_active_paths = MAX_ACTIVE_PATHS;
    config.num_trailing_blanks = NUM_TRAILING_BLANKS;
    config.keywords_score = KEYWORDS_SCORE;
    config.keywords_threshold = threshold;
    config.keywords_file = keywords.as_ptr();
    let spotter = unsafe { (api.create_spotter)(&config) };
    if spotter.is_null() {
        Err(format!(
            "Sherpa keyword spotter creation failed for {keywords_file}"
        ))
    } else {
        Ok(spotter)
    }
}

fn seconds_to_samples(seconds: f32) -> usize {
    (seconds.max(0.0) * SAMPLE_RATE as f32).round() as usize
}

fn transcription_result(result: KeywordResult, offset_samples: usize) -> TranscriptionResult {
    let text = command_text(&result.keyword).expect("validated Sherpa keyword label");
    let offset = offset_samples as f32 / SAMPLE_RATE as f32;
    let start = offset + result.timestamps.first().copied().unwrap_or(0.0);
    let end = offset + result.timestamps.last().copied().unwrap_or(start - offset) + 0.08;
    TranscriptionResult {
        text: text.to_string(),
        tokens: vec![TimedToken {
            text: " zephyr".to_string(),
            start,
            end: end.max(start),
        }],
    }
}

fn empty_result() -> TranscriptionResult {
    TranscriptionResult {
        text: String::new(),
        tokens: Vec::new(),
    }
}

fn command_text(keyword: &str) -> Option<&'static str> {
    match keyword.trim().trim_start_matches('@') {
        "ZEPHYR_STOP" => Some("zephyr stop"),
        "ZEPHYR_SEND" => Some("zephyr send"),
        "ZEPHYR_CANCEL" => Some("zephyr cancel"),
        "ZEPHYR" => Some("zephyr"),
        _ => None,
    }
}

/// Validate one emitted acoustic path. Request wall age, live-audio lag &
/// stream identity are enforced by the outer worker because Sherpa's timestamp
/// clock may omit leading silence & cannot be compared to accepted PCM count.
fn keyword_timestamps_are_plausible(timestamps: &[f32]) -> bool {
    let (Some(start), Some(end)) = (timestamps.first(), timestamps.last()) else {
        return false;
    };
    start.is_finite()
        && end.is_finite()
        && *start >= 0.0
        && *end >= *start
        && *end - *start <= MAX_KEYWORD_SPAN_SECONDS
        && timestamps
            .windows(2)
            .all(|pair| pair[0].is_finite() && pair[1].is_finite() && pair[1] >= pair[0])
}

fn validate_model_dir(dir: &Path) -> Result<(), String> {
    for file in [
        ENCODER,
        DECODER,
        JOINER,
        "tokens.txt",
        "keywords-wake.txt",
        "keywords-confirm.txt",
    ] {
        let path = dir.join(file);
        if !path.is_file() {
            return Err(format!(
                "bundled Sherpa KWS resource missing {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn path_cstring(path: &Path) -> Result<CString, String> {
    CString::new(path.to_string_lossy().as_bytes())
        .map_err(|_| format!("Sherpa resource path contains NUL: {}", path.display()))
}

fn resolve_library_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("HR_SHERPA_LIBRARY_PATH") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("ORT_DYLIB_PATH") {
        if let Some(parent) = Path::new(&path).parent() {
            candidates.push(parent.join(SHERPA_LIBRARY));
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(SHERPA_LIBRARY));
            candidates.push(dir.join("runtime").join(SHERPA_LIBRARY));
            candidates.push(dir.join("resources/runtime").join(SHERPA_LIBRARY));
            #[cfg(target_os = "macos")]
            if let Some(contents) = dir.parent() {
                candidates.push(contents.join("Resources/runtime").join(SHERPA_LIBRARY));
                candidates.push(
                    contents
                        .join("Resources/resources/runtime")
                        .join(SHERPA_LIBRARY),
                );
            }
        }
    }
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../src-tauri/resources/runtime")
            .join(SHERPA_LIBRARY),
    );
    candidates.into_iter().find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_decoder_contract_is_cross_platform() {
        assert_eq!(NUM_THREADS, 1);
        assert_eq!(MAX_ACTIVE_PATHS, 8);
        assert_eq!(NUM_TRAILING_BLANKS, 0);
        assert_eq!(CONFIRM_CONTEXT_SAMPLES, SAMPLE_RATE as usize * 2);
        assert_eq!(command_text("@ZEPHYR_STOP"), Some("zephyr stop"));
        assert_eq!(command_text("ZEPHYR_SEND"), Some("zephyr send"));
        assert_eq!(command_text("@ZEPHYR_CANCEL"), Some("zephyr cancel"));
    }

    #[test]
    fn first_decode_boundary_is_relative_to_native_stream_origin() {
        assert_eq!(next_decode_boundary(123), 123 + KWS_DECODE_STEP_SAMPLES);
        assert_eq!(
            next_decode_boundary(KWS_DECODE_STEP_SAMPLES),
            KWS_DECODE_STEP_SAMPLES * 2
        );
    }

    #[test]
    fn cascade_clock_deduplicates_wake_and_accrues_confirmation() {
        let mut clock = CascadeClock::default();
        assert!(clock.observe_wake(10_000, 18_000));
        assert!(!clock.observe_wake(10_240, 18_320));
        assert!(!clock.expire_if_needed(41_999));
        assert!(clock.expire_if_needed(42_001));
        assert!(clock.candidate().is_none());
    }

    #[test]
    fn wake_decode_accepts_each_unseen_arrival_without_timer_gating() {
        assert_eq!(
            classify_confirmation_feed(320, 0, 640),
            ConfirmationFeed::Suffix { offset: 320 }
        );
        assert_eq!(
            classify_confirmation_feed(640, 0, 800),
            ConfirmationFeed::Suffix { offset: 640 }
        );
    }

    #[test]
    fn persistent_confirmation_cursor_feeds_only_unseen_audio() {
        assert_eq!(
            classify_confirmation_feed(10_000, 9_000, 10_000),
            ConfirmationFeed::Duplicate
        );
        assert_eq!(
            classify_confirmation_feed(10_000, 9_000, 11_000),
            ConfirmationFeed::Suffix { offset: 1_000 }
        );
        assert_eq!(
            classify_confirmation_feed(10_000, 9_500, 11_000),
            ConfirmationFeed::Suffix { offset: 500 }
        );
    }

    #[test]
    fn persistent_confirmation_cursor_rejects_stale_and_detects_gaps() {
        assert_eq!(
            classify_confirmation_feed(10_000, 9_000, 9_999),
            ConfirmationFeed::Stale
        );
        assert_eq!(
            classify_confirmation_feed(10_000, 10_001, 11_000),
            ConfirmationFeed::Gap
        );
    }

    #[test]
    fn persistent_confirmation_origin_maps_timestamps_once() {
        let seed_origin = 24_000usize;
        let snapshot_start = 16_000usize;
        let snapshot_relative = seed_origin - snapshot_start;
        let result = KeywordResult {
            keyword: "@ZEPHYR_SEND".into(),
            timestamps: vec![0.5, 1.0],
        };
        let mapped = transcription_result(result, snapshot_relative);
        assert_eq!(mapped.text, "zephyr send");
        assert_eq!(mapped.tokens[0].start, 1.0);
        assert_eq!(mapped.tokens[0].end, 1.58);
    }

    #[test]
    fn production_cascade_resources_split_wake_from_action() {
        let resource_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/resources/kws");
        let wake = std::fs::read_to_string(resource_dir.join("keywords-wake.txt")).unwrap();
        let confirm = std::fs::read_to_string(resource_dir.join("keywords-confirm.txt")).unwrap();

        assert!(wake.contains("#0.05 @ZEPHYR"));
        assert!(!wake.contains("ZEPHYR_SEND"));
        assert!(confirm.contains("#0.15 @ZEPHYR_STOP"));
        assert!(confirm.contains("#0.18 @ZEPHYR_SEND"));
        assert!(confirm.contains("#0.18 @ZEPHYR_CANCEL"));
        assert_eq!(command_text("@ZEPHYR"), Some("zephyr"));
    }

    /// The always-on wake keyword must stay a SEPARATE file from the tail
    /// cascade: adding "hey zephyr" must never widen what can fire a
    /// STOP/SEND/CANCEL action, and the tail wake stays hey-free so ordinary
    /// mid-dictation "zephyr stop" is unaffected.
    #[test]
    fn hey_wake_resource_is_isolated_from_tail_cascade() {
        let resource_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/resources/kws");
        let hey = std::fs::read_to_string(resource_dir.join(HEY_WAKE_KEYWORDS_FILE)).unwrap();
        let wake = std::fs::read_to_string(resource_dir.join("keywords-wake.txt")).unwrap();
        let confirm = std::fs::read_to_string(resource_dir.join("keywords-confirm.txt")).unwrap();

        assert!(hey.contains("HH EY1 Z EH1 F ER0"));
        assert!(hey.contains(&format!("@{HEY_WAKE_KEYWORD}")));
        assert!(!hey.contains("ZEPHYR_STOP"));
        assert!(!hey.contains("ZEPHYR_SEND"));
        assert!(!hey.contains("ZEPHYR_CANCEL"));
        assert!(!wake.contains(HEY_WAKE_KEYWORD));
        assert!(!confirm.contains(HEY_WAKE_KEYWORD));
    }

    #[test]
    fn result_timestamps_remain_relative_to_app_owned_stream() {
        let timestamps = [0.24_f32, 0.92_f32];
        assert!(keyword_timestamps_are_plausible(&timestamps));
        assert_eq!(timestamps.first().copied(), Some(0.24));
        assert_eq!(
            timestamps.last().copied().map(|value| value + 0.08),
            Some(1.0)
        );
    }

    #[test]
    fn rejects_keyword_paths_spanning_unrelated_speech() {
        assert!(!keyword_timestamps_are_plausible(&[]));
        assert!(!keyword_timestamps_are_plausible(&[7.6, 21.64]));
        assert!(!keyword_timestamps_are_plausible(&[1.0, 0.8]));
        assert!(keyword_timestamps_are_plausible(&[0.36, 1.48]));
    }

    #[test]
    fn accepts_compact_path_without_inventing_a_second_timestamp_clock() {
        assert!(keyword_timestamps_are_plausible(&[7.6, 8.4]));
        assert!(keyword_timestamps_are_plausible(&[0.36, 1.12]));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn bundled_runtime_loads_and_accepts_silence() {
        let model_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/resources/kws");
        let mut kws = SherpaKws::load(&model_dir).expect("load bundled Sherpa KWS");
        assert!(
            kws.hey_wake_spotter.is_null(),
            "idle spotter must load lazily"
        );
        let result = kws
            .transcribe_result(&vec![0.0; SAMPLE_RATE as usize / 2])
            .expect("decode silence");
        assert!(result.text.is_empty());
        assert!(result.tokens.is_empty());
        kws.reset_stream().expect("reset bundled Sherpa stream");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn bundled_idle_wake_stream_is_direct_and_marker_owner_lazy() {
        let model_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/resources/kws");
        let mut kws = SherpaKws::load(&model_dir).expect("load bundled Sherpa KWS");
        assert!(kws.hey_wake_available());
        kws.start_idle_wake(0.25).expect("start idle wake");
        assert!(!kws.hey_wake_spotter.is_null());
        assert_eq!(
            kws.feed_idle_wake(&vec![0.0; 320], 0, 320)
                .expect("feed one arrival"),
            None
        );
        kws.stop_idle_wake();
        assert!(kws.idle_wake_stream.is_none());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn bundled_windows_runtime_matches_app_ort() {
        let resource_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/resources");
        let _ort = unsafe { Library::new(resource_dir.join("runtime/onnxruntime.dll")) }
            .expect("preload bundled ONNX Runtime");
        let mut kws = SherpaKws::load(&resource_dir.join("kws")).expect("load bundled Sherpa KWS");
        let result = kws
            .transcribe_result(&vec![0.0; SAMPLE_RATE as usize / 2])
            .expect("decode silence");
        assert!(result.text.is_empty());
        assert!(result.tokens.is_empty());
        kws.reset_stream().expect("reset bundled Sherpa stream");
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "explicit centralized-corpus production replay"]
    fn production_cascade_replays_reports() {
        #[derive(Deserialize)]
        struct Report {
            rows: Vec<ReportRow>,
        }
        #[derive(Deserialize)]
        struct ReportRow {
            row_id: String,
            audio_path: PathBuf,
            expected_detection: Option<String>,
        }

        let report_paths = std::env::var("HR_KWS_CASCADE_REPORTS")
            .expect("HR_KWS_CASCADE_REPORTS must contain semicolon-separated report paths");
        let resources = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/resources");
        let _ort = unsafe { Library::new(resources.join("runtime/onnxruntime.dll")) }
            .expect("preload bundled ONNX Runtime");
        let mut kws =
            SherpaKws::load(&resources.join("kws")).expect("load bundled Sherpa KWS cascade");
        let mut checked = 0usize;
        let mut positives = 0usize;
        let mut false_actions = Vec::new();
        let mut misses = Vec::new();

        for report_path in report_paths
            .split(';')
            .filter(|value| !value.trim().is_empty())
        {
            let report: Report =
                serde_json::from_slice(&std::fs::read(report_path).expect("read cascade report"))
                    .expect("parse cascade report");
            for row in report.rows {
                kws.reset_stream().unwrap();
                let audio = read_test_wav(&row.audio_path);
                let mut padded = audio.clone();
                padded.resize(audio.len() + CONFIRM_TIMEOUT_SAMPLES, 0.0);
                let mut live = TAIL_WAKE_NEW_AUDIO_SAMPLES_FOR_TEST;
                let mut confirmation_armed = false;
                let mut detected = None;
                while live <= padded.len() {
                    let at_end = live == padded.len();
                    let snapshot_start = live.saturating_sub(SAMPLE_RATE as usize * 3);
                    let result = kws
                        .transcribe_cascade_result(
                            &padded[snapshot_start..live],
                            snapshot_start,
                            live,
                            confirmation_armed,
                        )
                        .unwrap_or_else(|error| panic!("{}: {error}", row.row_id));
                    match result.text.as_str() {
                        "zephyr" => confirmation_armed = true,
                        "zephyr stop" | "zephyr send" | "zephyr cancel" => {
                            detected = Some(result.text.to_ascii_uppercase().replace(' ', "_"));
                            break;
                        }
                        "" => {}
                        other => panic!("{}: unexpected cascade output {other}", row.row_id),
                    }
                    if at_end {
                        break;
                    }
                    live = (live
                        + if confirmation_armed {
                            CONFIRM_STEP_SAMPLES
                        } else {
                            SAMPLE_RATE as usize / 4
                        })
                    .min(padded.len());
                }

                checked += 1;
                if let Some(expected) = row.expected_detection {
                    positives += 1;
                    if detected.as_deref() != Some(expected.as_str()) {
                        misses.push(format!(
                            "{} expected={expected} got={detected:?}",
                            row.row_id
                        ));
                    }
                } else if let Some(action) = detected {
                    false_actions.push(format!("{} got={action}", row.row_id));
                }
                if checked % 10 == 0 {
                    eprintln!(
                        "production cascade replay progress: checked={checked}/362 report={report_path}"
                    );
                }
            }
        }

        assert_eq!(checked, 362, "centralized corpus row count changed");
        assert_eq!(positives, 46, "centralized positive count changed");
        // One assertion for both halves: failing recall first would hide the
        // precision result, and a corpus run this expensive must never report
        // half its outcome.
        assert!(
            misses.is_empty() && false_actions.is_empty(),
            "production cascade: {} misses of {positives} positives {misses:#?}\n{} false actions of {} negatives {false_actions:#?}",
            misses.len(),
            false_actions.len(),
            checked - positives
        );
    }

    #[cfg(target_os = "windows")]
    const TAIL_WAKE_NEW_AUDIO_SAMPLES_FOR_TEST: usize = SAMPLE_RATE as usize / 4;

    #[cfg(target_os = "windows")]
    fn read_test_wav(path: &Path) -> Vec<f32> {
        let mut reader = hound::WavReader::open(path)
            .unwrap_or_else(|error| panic!("open {}: {error}", path.display()));
        let spec = reader.spec();
        assert_eq!(spec.sample_rate, SAMPLE_RATE as u32, "{}", path.display());
        assert_eq!(spec.channels, 1, "{}", path.display());
        match spec.sample_format {
            hound::SampleFormat::Int => {
                let scale = (1_i64 << (spec.bits_per_sample - 1)) as f32;
                reader
                    .samples::<i32>()
                    .map(|sample| sample.unwrap() as f32 / scale)
                    .collect()
            }
            hound::SampleFormat::Float => reader
                .samples::<f32>()
                .map(|sample| sample.unwrap())
                .collect(),
        }
    }
}
