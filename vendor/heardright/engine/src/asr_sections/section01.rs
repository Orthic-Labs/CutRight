// ASR runtime for the sidecar.
//
// The sidecar loads the ASR model once and runs authoritative decode on every
// dictation stop. Long static Parakeet clips use padded-window decode so 15 s
// model limits do not become hard text boundaries.

use std::path::{Path, PathBuf};

use heardright_core::engine::FileTranscript;
#[cfg(target_os = "macos")]
use heardright_core::engine::TimedWord;
use parakeet_rs::{
    ExecutionConfig, ExecutionProvider, HrTransducer, TimedToken, TranscriptionResult,
};

const SAMPLE_RATE: usize = 16_000;
const PADDED_WINDOW_MS: u64 = 15_000;
const PADDED_WINDOW_PADDING_MS: u64 = 2_240;
const PADDED_WINDOW_SILENCE_MS: u64 = 200;
const PADDED_WINDOW_SILENCE_HOP_MS: u64 = 100;
const PADDED_WINDOW_OVERLAP_CHARS: usize = 16;
pub(crate) const AUDIBLE_BLANK_TRANSCRIPTION_ERROR: &str =
    "transcription produced no text although the audio wasn't silent — recording kept, please retry";
const UNIFIED_MODEL_SUBDIR: &str = "unified_static15_b128_sym_bits4_timestamp_hybrid";
const TDT_MODEL_SUBDIR: &str = "parakeet_tdt_v3_static1500_qint8_20260722";
#[cfg(target_os = "macos")]
const COREML_UNIFIED_MODEL_SUBDIR: &str = "parakeet-unified-en-0.6b";
#[cfg(target_os = "macos")]
const COREML_TDT_MODEL_SUBDIR: &str = "parakeet-tdt-v3";

/// Supported execution providers for the ONNX fallback path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsrEp {
    /// CUDA (NVIDIA GPU) — opt-in benchmark provider on Windows.
    #[cfg(all(target_os = "windows", feature = "cuda-bench"))]
    Cuda,
    /// DirectML (Windows GPU) — encoder EP on Windows.
    #[cfg(target_os = "windows")]
    Dml,
    /// CPU baseline. Default on Linux and the opt-in fallback on macOS.
    Cpu,
}

impl AsrEp {
    /// Probe-based default EP: DirectML if the device is available on
    /// Windows, otherwise CPU.
    pub fn resolve_default() -> Self {
        #[cfg(target_os = "windows")]
        {
            if let Some(route) = crate::settings::persisted_recognition_route() {
                return if route.provider == "dml" {
                    AsrEp::Dml
                } else {
                    AsrEp::Cpu
                };
            }
            if probe_directml_embedded() {
                AsrEp::Dml
            } else {
                AsrEp::Cpu
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            AsrEp::Cpu
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            #[cfg(all(target_os = "windows", feature = "cuda-bench"))]
            AsrEp::Cuda => "cuda",
            #[cfg(target_os = "windows")]
            AsrEp::Dml => "dml",
            AsrEp::Cpu => "cpu",
        }
    }

    fn provider(self) -> ExecutionProvider {
        match self {
            #[cfg(all(target_os = "windows", feature = "cuda-bench"))]
            AsrEp::Cuda => ExecutionProvider::Cuda,
            #[cfg(target_os = "windows")]
            AsrEp::Dml => ExecutionProvider::DirectML,
            AsrEp::Cpu => ExecutionProvider::Cpu,
        }
    }
}

pub enum AsrRuntime {
    Parakeet(HrTransducer),
    #[cfg(target_os = "macos")]
    CoreMlParakeet(crate::coreml_asr::CoreMlParakeet),
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    SherpaKws(crate::sherpa_kws::SherpaKws),
    /// Safety fallback selected when no KWS mechanism passes native acceptance.
    /// It keeps probe scheduling alive but can never emit text, timing, or control.
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    ProbeDisabled,
    #[cfg(target_os = "macos")]
    WhisperCoreMl {
        engine: crate::whisper_coreml::WhisperCoreMl,
        lang_tok: i32,
    },
    /// Windows Whisper backend — shells out to the staged whisper.cpp Vulkan
    /// CLI per utterance. See `whisper_win.rs`. `lang` is the Whisper language
    /// code to pass (`"auto"` for detect, else a specific code).
    #[cfg(target_os = "windows")]
    WhisperWin {
        engine: crate::whisper_win::WhisperWin,
        lang: String,
    },
}

impl AsrRuntime {
    /// Cross-platform isolated command/trigger runtime. Final dictation
    /// continues through `load`, using each platform's primary ASR unchanged.
    pub fn load_probe(models_dir: &Path, ep: AsrEp) -> Result<Self, String> {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            if std::env::var("HR_DISABLE_KWS").ok().as_deref() == Some("1") {
                return Ok(AsrRuntime::ProbeDisabled);
            }
            let _ = (models_dir, ep);
            let dir = std::env::var_os("HR_SHERPA_KWS_DIR")
                .map(PathBuf::from)
                .ok_or_else(|| "HR_SHERPA_KWS_DIR not supplied by shell".to_string())?;
            crate::sherpa_kws::SherpaKws::load(&dir).map(AsrRuntime::SherpaKws)
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        Self::load(models_dir, ep)
    }

    /// The background scheduler is a validated TDT-only production policy.
    /// Whisper and retired Unified/RNNT aliases retain their existing deferred
    /// windowing behavior.
    pub fn uses_scheduled_static15(&self) -> bool {
        if configured_backend() != "parakeet-tdt" {
            return false;
        }
        #[cfg(target_os = "macos")]
        {
            matches!(
                self,
                AsrRuntime::Parakeet(_) | AsrRuntime::CoreMlParakeet(_)
            )
        }
        #[cfg(not(target_os = "macos"))]
        {
            matches!(self, AsrRuntime::Parakeet(_))
        }
    }

    /// Load the configured ASR model from `models_dir`.
    ///
    /// macOS prefers native CoreML bundles so the always-hot sidecar owns the
    /// ANE-resident production path. macOS uses the validated Unified CoreML
    /// bundle for the default alias and keeps TDT as an explicit rollback lane.
    pub fn load(models_dir: &Path, ep: AsrEp) -> Result<Self, String> {
        // DirectML adapter selection is persisted by onboarding. It must be
        // applied before `HrTransducer::load`, where ORT builds its sessions.
        let persisted_route = crate::settings::persisted_recognition_route();
        crate::settings::apply_persisted_recognition_route_environment();
        #[cfg(target_os = "macos")]
        if std::env::var("HR_ASR_COREML").ok().as_deref() != Some("0") {
            let backend = configured_backend();
            let lang = crate::settings::dictation_language();
            // "auto"  = Parakeet auto-detect, 25 EU languages (free, bundled).
            // "multi" = Whisper auto-detect across 100+ languages (Pro).
            // <code>  = Whisper locked to that language (Pro) — Parakeet has no
            //           language input, so forcing a language (incl. an EU one,
            //           for short-utterance reliability) requires Whisper.
            let force_whisper =
                (lang != "auto" && !lang.is_empty()) || backend.starts_with("whisper");
            if force_whisper {
                // "multi" maps to Whisper's own "auto" detect token.
                let whisper_lang = if lang == "multi" { "auto" } else { &lang };
                return try_load_coreml_whisper(models_dir, whisper_lang);
            }
            if backend.starts_with("parakeet") || backend == "rnnt" {
                if let Some(runtime) = try_load_coreml_parakeet(models_dir)? {
                    return Ok(runtime);
                }
                if persisted_route
                    .as_ref()
                    .is_some_and(|route| route.provider == "coreml")
                {
                    return Err("selected CoreML recognition route is unavailable; complete onboarding setup again".to_string());
                }
            }
        }

        // Windows Whisper lane (Lane D, 2026-07-02): Whisper multilingual is a
        // cross-platform Pro feature. The macOS engine is CoreML/ANE; Windows
        // shells out to the staged whisper.cpp Vulkan CLI per utterance
        // (`whisper_win.rs`). Same routing rule as macOS: a locked language, or
        // an explicit whisper backend selection, forces Whisper. "auto" stays on
        // Parakeet (free, bundled, on-device) — Whisper is Pro-gated at the
        // shell's config push, not here.
        #[cfg(target_os = "windows")]
        {
            let backend = configured_backend();
            let lang = crate::settings::dictation_language();
            let force_whisper =
                (lang != "auto" && !lang.is_empty()) || backend.starts_with("whisper");
            if force_whisper {
                let whisper_lang = if lang == "multi" {
                    "auto".to_string()
                } else {
                    lang.clone()
                };
                return try_load_whisper_win(models_dir, whisper_lang);
            }
        }

        let model_subdir = model_subdir_for(&configured_backend());
        let candidate = models_dir.join(&model_subdir);
        let model_dir = if candidate.is_dir() {
            candidate.as_path()
        } else {
            models_dir
        };
        let mut cfg = ExecutionConfig::new().with_execution_provider(ep.provider());
        #[cfg(target_os = "windows")]
        if matches!(ep, AsrEp::Dml) {
            if let Some(adapter_index) = std::env::var("HR_DML_DEVICE_ID")
                .ok()
                .and_then(|value| value.parse::<i32>().ok())
                .filter(|index| *index >= 0)
            {
                cfg = cfg.with_directml_device_id(adapter_index);
            }
        }
        let mut model = HrTransducer::load(model_dir, Some(cfg)).map_err(|e| {
            format!(
                "model load ({}) on {}: {}",
                model_dir.display(),
                ep.as_str(),
                e
            )
        })?;
        apply_context_bias(&mut model);
        tracing::info!(
            "ASR model loaded from {} on {}",
            model_dir.display(),
            ep.as_str()
        );
        Ok(AsrRuntime::Parakeet(model))
    }

    /// Run a mutation plus inference as one transaction. A shared runtime uses
    /// mutable decoder bias, so callers that change bias must hold this lease
    /// through the matching decode.
    pub(crate) fn with_inference_lease<T>(
        &mut self,
        owner: &'static str,
        work: impl FnOnce(&mut Self) -> Result<T, String>,
    ) -> Result<T, String> {
        #[cfg(target_os = "macos")]
        if matches!(
            self,
            AsrRuntime::CoreMlParakeet(_) | AsrRuntime::WhisperCoreMl { .. }
        ) {
            let _lease = crate::coreml::inference_lease(owner);
            return work(self);
        }
        let _lease = crate::inference_gate::lease(owner);
        work(self)
    }

    pub fn transcribe(&mut self, samples: &[f32]) -> Result<String, String> {
        let owner = match self {
            AsrRuntime::Parakeet(_) => "parakeet_asr",
            #[cfg(target_os = "macos")]
            AsrRuntime::CoreMlParakeet(_) => "parakeet_asr",
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            AsrRuntime::SherpaKws(_) => "sherpa_kws",
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            AsrRuntime::ProbeDisabled => "disabled_probe",
            #[cfg(target_os = "macos")]
            AsrRuntime::WhisperCoreMl { .. } => "whisper_asr",
            #[cfg(target_os = "windows")]
            AsrRuntime::WhisperWin { .. } => "whisper_win_asr",
        };
        self.with_inference_lease(owner, |model| model.transcribe_under_lease(samples))
    }

    pub(crate) fn transcribe_under_lease(&mut self, samples: &[f32]) -> Result<String, String> {
        match self {
            AsrRuntime::Parakeet(m) => transcribe_padded_window(samples, |window| {
                m.transcribe_result(window).map_err(|e| e.to_string())
            })
            .map(|result| result.text),
            #[cfg(target_os = "macos")]
            AsrRuntime::CoreMlParakeet(m) => {
                if samples.len() <= padded_window_samples() {
                    m.transcribe(samples)
                } else {
                    transcribe_padded_window(samples, |window| coreml_parakeet_result(m, window))
                        .map(|result| result.text)
                }
            }
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            AsrRuntime::SherpaKws(m) => m.transcribe(samples),
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            AsrRuntime::ProbeDisabled => Ok(String::new()),
            #[cfg(target_os = "macos")]
            AsrRuntime::WhisperCoreMl {
                engine, lang_tok, ..
            } => engine.transcribe_lang_windowed(samples, *lang_tok),
            #[cfg(target_os = "windows")]
            AsrRuntime::WhisperWin { engine, lang } => engine.transcribe_lang(samples, lang),
        }
    }

    /// Final dictation rejects fully blank audible output, but never changes
    /// compute routes or retries on alternate stages.
    pub(crate) fn transcribe_final_under_lease(
        &mut self,
        samples: &[f32],
    ) -> Result<String, String> {
        match self {
            #[cfg(target_os = "macos")]
            AsrRuntime::CoreMlParakeet(m) => {
                let result = if samples.len() <= padded_window_samples() {
                    m.transcribe(samples)
                } else {
                    transcribe_padded_window(samples, |window| coreml_parakeet_result(m, window))
                        .map(|result| result.text)
                };
                match result {
                    Ok(text) if text.trim().is_empty() => {
                        Err(AUDIBLE_BLANK_TRANSCRIPTION_ERROR.to_string())
                    }
                    other => other,
                }
            }
            other => other.transcribe_under_lease(samples),
        }
    }

    pub fn transcribe_result(&mut self, samples: &[f32]) -> Result<TranscriptionResult, String> {
        self.with_inference_lease("parakeet_asr_result", |model| {
            model.transcribe_result_under_lease(samples)
        })
    }

    /// Timed CPU KWS decode. Probe runtime is worker-owned and independent
    /// from main ASR, so this path must never take the accelerator gate.
    pub(crate) fn transcribe_probe_result(
        &mut self,
        samples: &[f32],
    ) -> Result<TranscriptionResult, String> {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let AsrRuntime::SherpaKws(model) = self {
            return model.transcribe_result(samples);
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if matches!(self, AsrRuntime::ProbeDisabled) {
            return Ok(TranscriptionResult {
                text: String::new(),
                tokens: Vec::new(),
            });
        }
        Err("timed KWS decode requires Sherpa runtime".into())
    }

    /// Fresh-window Sherpa cascade decode on an app-owned absolute PCM clock.
    /// Full ASR may arm confirmation, but only Sherpa can return an action.
    pub(crate) fn transcribe_probe_cascade_result(
        &mut self,
        samples: &[f32],
        snapshot_start_sample: usize,
        live_total_sample: usize,
        confirmation_armed: bool,
    ) -> Result<TranscriptionResult, String> {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let AsrRuntime::SherpaKws(model) = self {
            return model.transcribe_cascade_result(
                samples,
                snapshot_start_sample,
                live_total_sample,
                confirmation_armed,
            );
        }
        self.transcribe_probe_result(samples)
    }

    pub(crate) fn transcribe_probe_fresh_confirmation_result(
        &mut self,
        samples: &[f32],
    ) -> Result<TranscriptionResult, String> {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let AsrRuntime::SherpaKws(model) = self {
            return model.transcribe_fresh_confirmation_result(samples);
        }
        self.transcribe_probe_result(samples)
    }

    pub(crate) fn transcribe_result_under_lease(
        &mut self,
        samples: &[f32],
    ) -> Result<TranscriptionResult, String> {
        match self {
            AsrRuntime::Parakeet(m) => transcribe_padded_window(samples, |window| {
                m.transcribe_result(window).map_err(|e| e.to_string())
            }),
            #[cfg(target_os = "macos")]
            AsrRuntime::CoreMlParakeet(m) => {
                transcribe_padded_window(samples, |window| coreml_parakeet_result(m, window))
            }
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            AsrRuntime::SherpaKws(m) => m.transcribe_result(samples),
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            AsrRuntime::ProbeDisabled => Ok(TranscriptionResult {
                text: String::new(),
                tokens: Vec::new(),
            }),
            #[cfg(target_os = "macos")]
            AsrRuntime::WhisperCoreMl { .. } => {
                Err("streaming partial timestamps are unavailable for this ASR backend".into())
            }
            #[cfg(target_os = "windows")]
            AsrRuntime::WhisperWin { .. } => {
                Err("streaming partial timestamps are unavailable for this ASR backend".into())
            }
        }
    }

    /// Sherpa control firing requires token-frame timing. If a timed decode
    /// fails, do not retry as text-only: that would reintroduce full-buffer
    /// final transcription of fired control audio.
    pub(crate) fn requires_timed_control_probe(&self) -> bool {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            matches!(self, AsrRuntime::SherpaKws(_))
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            false
        }
    }

    pub(crate) fn reset_probe_stream(&mut self) -> Result<(), String> {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let AsrRuntime::SherpaKws(model) = self {
            return model.reset_stream();
        }
        Ok(())
    }

    pub(crate) fn start_idle_wake(&mut self, threshold: f32) -> Result<(), String> {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let AsrRuntime::SherpaKws(model) = self {
            return model.start_idle_wake(threshold);
        }
        Err("idle wake requires Sherpa KWS".into())
    }

    pub(crate) fn stop_idle_wake(&mut self) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let AsrRuntime::SherpaKws(model) = self {
            model.stop_idle_wake();
        }
    }

    pub(crate) fn feed_idle_wake(
        &mut self,
        samples: &[f32],
        start_sample: usize,
        live_total_sample: usize,
    ) -> Result<Option<(usize, usize)>, String> {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let AsrRuntime::SherpaKws(model) = self {
            return model.feed_idle_wake(samples, start_sample, live_total_sample);
        }
        Err("idle wake requires Sherpa KWS".into())
    }

    pub fn transcribe_file(&mut self, samples: &[f32]) -> Result<FileTranscript, String> {
        self.with_inference_lease("parakeet_file_asr", |model| {
            model.transcribe_file_under_lease(samples)
        })
    }

    pub(crate) fn transcribe_file_under_lease(
        &mut self,
        samples: &[f32],
    ) -> Result<FileTranscript, String> {
        match self {
            #[cfg(target_os = "macos")]
            AsrRuntime::CoreMlParakeet(m) => {
                if samples.len() > padded_window_samples() {
                    let result = transcribe_padded_window(samples, |window| {
                        coreml_parakeet_result(m, window)
                    })?;
                    let words = timed_tokens_to_words(&result.tokens);
                    let (srt, vtt) = crate::coreml_asr::render_subtitles(&words);
                    Ok(FileTranscript {
                        text: result.text,
                        srt,
                        vtt,
                        words: words
                            .into_iter()
                            .map(|word| TimedWord {
                                text: word.text,
                                start_ms: (word.start.max(0.0) * 1_000.0).round() as i64,
                                end_ms: (word.end.max(word.start + 0.08) * 1_000.0).round() as i64,
                            })
                            .collect(),
                    })
                } else {
                    let (text, words) = m.transcribe_timed(samples)?;
                    let (srt, vtt) = crate::coreml_asr::render_subtitles(&words);
                    Ok(FileTranscript {
                        text,
                        srt,
                        vtt,
                        words: words
                            .into_iter()
                            .map(|word| TimedWord {
                                text: word.text,
                                start_ms: (word.start.max(0.0) * 1_000.0).round() as i64,
                                end_ms: (word.end.max(word.start + 0.08) * 1_000.0).round() as i64,
                            })
                            .collect(),
                    })
                }
            }
            other => {
                let text = other.transcribe_under_lease(samples)?;
                Ok(FileTranscript {
                    text,
                    srt: String::new(),
                    vtt: String::new(),
                    words: Vec::new(),
                })
            }
        }
    }
}

#[derive(Debug)]
struct Static15Checkpoint {
    start_sample: usize,
    end_sample: usize,
    text: String,
    overlap_bytes: usize,
    inserted_separator: bool,
}

/// Per-recording static-15 scheduler shared by the Windows ONNX/DML and macOS
/// Core ML TDT runtimes. Audio remains owned by the worker's single, hard-capped
/// recording buffer; checkpoints retain enough provenance to roll back only
/// segments overlapping a later KWS cut.
#[derive(Debug, Default)]
pub(crate) struct ScheduledStatic15 {
    checkpoints: Vec<Static15Checkpoint>,
    committed_text: String,
    committed_samples: usize,
}

impl ScheduledStatic15 {
    pub(crate) fn reset(&mut self) {
        self.checkpoints.clear();
        self.committed_text.clear();
        self.committed_samples = 0;
    }

    pub(crate) fn background_windows(&self) -> usize {
        self.checkpoints.len()
    }

    pub(crate) fn committed_samples(&self) -> usize {
        self.committed_samples
    }

    pub(crate) fn committed_text_counts(&self) -> (usize, usize) {
        (
            self.committed_text.chars().count(),
            self.committed_text.split_whitespace().count(),
        )
    }

    #[cfg(test)]
    pub(crate) fn tail_samples(&self, total_samples: usize) -> usize {
        total_samples.saturating_sub(self.committed_samples)
    }

    pub(crate) fn commit_window(
        &mut self,
        start_sample: usize,
        end_sample: usize,
        text: &str,
    ) -> Result<(), String> {
        if start_sample != self.committed_samples {
            return Err(format!(
                "scheduled static-15 out of order: expected {}, got {}",
                self.committed_samples, start_sample
            ));
        }
        if end_sample <= start_sample {
            return Err("scheduled static-15 window must advance".to_string());
        }
        if text.trim().is_empty() {
            return Err(
                "scheduled static-15 window produced no text; defer full audio to stop".to_string(),
            );
        }
        let overlap_bytes = if self.committed_text.is_empty() {
            0
        } else {
            suffix_prefix_overlap(&self.committed_text, text, PADDED_WINDOW_OVERLAP_CHARS)
        };
        let remainder = &text[overlap_bytes..];
        let inserted_separator = !self.committed_text.is_empty()
            && !remainder.is_empty()
            && overlap_bytes == 0
            && !self.committed_text.ends_with(char::is_whitespace)
            && !remainder.starts_with(char::is_whitespace);
        append_with_overlap(&mut self.committed_text, text);
        self.checkpoints.push(Static15Checkpoint {
            start_sample,
            end_sample,
            text: text.to_string(),
            overlap_bytes,
            inserted_separator,
        });
        self.committed_samples = end_sample;
        Ok(())
    }

    /// Decode every complete 15-second window that capture has made ready.
    /// The cut and seam policy are exactly the locked deferred static-15 policy.
    #[cfg(test)]
    pub(crate) fn process_ready<F>(
        &mut self,
        samples: &[f32],
        mut decode: F,
    ) -> Result<usize, String>
    where
        F: FnMut(&[f32]) -> Result<String, String>,
    {
        let mut committed_now = 0;
        while let Some(segment) = scheduled_static15_ready_segment(samples, self.committed_samples)
        {
            let text = decode(&samples[segment.clone()])?;
            self.commit_window(segment.start, segment.end, &text)?;
            committed_now += 1;
        }
        Ok(committed_now)
    }

    pub(crate) fn finish<F>(&mut self, samples: &[f32], mut decode: F) -> Result<String, String>
    where
        F: FnMut(&[f32]) -> Result<String, String>,
    {
        // A KWS command cut can land inside the latest padded segment. Retain
        // only checkpoints fully before the cut, then decode exactly from that
        // retained cursor to the cut.
        if samples.len() < self.committed_samples {
            self.checkpoints
                .retain(|checkpoint| checkpoint.end_sample <= samples.len());
            self.committed_samples = self
                .checkpoints
                .last()
                .map_or(0, |checkpoint| checkpoint.end_sample);
            self.committed_text.clear();
            for checkpoint in &self.checkpoints {
                if checkpoint.inserted_separator {
                    self.committed_text.push(' ');
                }
                self.committed_text
                    .push_str(&checkpoint.text[checkpoint.overlap_bytes..]);
            }
        }
        let mut combined = self.committed_text.clone();
        let mut expected_start = 0;
        for checkpoint in &self.checkpoints {
            debug_assert_eq!(checkpoint.start_sample, expected_start);
            expected_start = checkpoint.end_sample;
        }
        if self.committed_samples < samples.len() {
            let tail_samples = samples.len() - self.committed_samples;
            match decode(&samples[self.committed_samples..]) {
                Ok(text) => {
                    let tail_chars = text.chars().count();
                    let tail_words = text.split_whitespace().count();
                    let tail_empty = text.trim().is_empty();
                    let overlap_bytes = if combined.is_empty() {
                        0
                    } else {
                        suffix_prefix_overlap(&combined, &text, PADDED_WINDOW_OVERLAP_CHARS)
                    };
                    append_with_overlap(&mut combined, &text);
                    tracing::info!(
                        committed_samples = self.committed_samples,
                        tail_samples,
                        tail_chars,
                        tail_words,
                        tail_empty,
                        overlap_bytes,
                        assembled_chars = combined.chars().count(),
                        assembled_words = combined.split_whitespace().count(),
                        "scheduled static-15 final tail assembled"
                    );
                }
                Err(error) => {
                    // The audible-blank guard exists to catch silent model
                    // collapse. Non-empty committed text proves the model is
                    // decoding this very recording, so a blank tail (trailing
                    // breath/noise after a trigger cut) must keep the committed
                    // transcript instead of failing the whole assembly —
                    // field failure 2026-08-01: 46s of committed speech was
                    // discarded because the 2s post-"zephyr send" tail decoded
                    // blank twice.
                    if error == AUDIBLE_BLANK_TRANSCRIPTION_ERROR && !combined.trim().is_empty() {
                        tracing::info!(
                            committed_samples = self.committed_samples,
                            tail_samples,
                            "scheduled static-15 final tail blank on audible audio; keeping committed transcript"
                        );
                    } else {
                        return Err(format!(
                            "scheduled static-15 partial transcript: committed through sample {}; \
                             final tail of {tail_samples} samples failed: {error}",
                            self.committed_samples
                        ));
                    }
                }
            }
        }
        tracing::info!(
            background_windows = self.background_windows(),
            committed_samples = self.committed_samples,
            total_samples = samples.len(),
            assembled_chars = combined.chars().count(),
            assembled_words = combined.split_whitespace().count(),
            assembled_empty = combined.trim().is_empty(),
            "scheduled static-15 assembly complete"
        );
        Ok(combined.trim().to_string())
    }
}

#[doc(hidden)]
pub fn scheduled_static15_ready_segment(
    samples: &[f32],
    start_sample: usize,
) -> Option<std::ops::Range<usize>> {
    let window = padded_window_samples();
    if start_sample + window >= samples.len() {
        return None;
    }
    let window_end = start_sample + window;
    let target_end = start_sample + window.saturating_sub(ms_to_samples(PADDED_WINDOW_PADDING_MS));
    let cut = quiet_cut_in(samples, target_end, window_end)
        .unwrap_or(window_end)
        .max(start_sample + 1);
    Some(start_sample..cut)
}

#[doc(hidden)]
pub fn append_scheduled_static15_text(buffer: &mut String, next: &str) {
    append_with_overlap(buffer, next);
}

fn transcribe_padded_window<F>(
    samples: &[f32],
    mut decode: F,
) -> Result<TranscriptionResult, String>
where
    F: FnMut(&[f32]) -> Result<TranscriptionResult, String>,
{
    let window = padded_window_samples();
    if samples.len() <= window {
        return decode(samples);
    }

    let padding = ms_to_samples(PADDED_WINDOW_PADDING_MS);
    let target = window.saturating_sub(padding);
    let mut text = String::new();
    let mut tokens = Vec::new();
    let mut start = 0usize;

    // Decode exactly to the quiet cut, matching the iOS runtime
    // (`ParakeetCoreMLRuntime.decodePaddedWindow`). The quiet-cut boundary is
    // unchanged — only the COMMIT rule differs from the previous form, which
    // decoded the full window and kept tokens whose 80 ms-quantised frame time
    // fell before the cut. That quantisation adjudicates every seam and gets
    // some wrong.
    //
    // REQUIRES the separator branch in `append_with_overlap`: these windows are
    // DISJOINT, so there is no character overlap to consume, and a raw
    // concatenation welds words across the seam. Porting this rule without that
    // fix measured 8.55% WER; with it, see the eval doc.
    while start < samples.len() {
        let window_end = (start + window).min(samples.len());
        if window_end >= samples.len() {
            break;
        }
        let target_end = (start + target).min(samples.len());
        let stable_end = quiet_cut_in(samples, target_end, window_end).unwrap_or(window_end);
        let segment: &[f32] = if start < stable_end {
            &samples[start..stable_end]
        } else {
            &[]
        };
        let result = decode(segment)?;
        append_with_overlap(&mut text, &result.text);
        let offset = start as f32 / SAMPLE_RATE as f32;
        tokens.extend(result.tokens.iter().map(|token| TimedToken {
            text: token.text.clone(),
            start: offset + token.start,
            end: offset + token.end,
        }));
        start = stable_end.max(start + 1);
    }
    // Tail after the final quiet cut.
    if start < samples.len() {
        let result = decode(&samples[start..])?;
        append_with_overlap(&mut text, &result.text);
        let offset = start as f32 / SAMPLE_RATE as f32;
        tokens.extend(result.tokens.iter().map(|token| TimedToken {
            text: token.text.clone(),
            start: offset + token.start,
            end: offset + token.end,
        }));
    }

    Ok(TranscriptionResult {
        text: text.trim().to_string(),
        tokens,
    })
}

#[cfg(target_os = "macos")]
fn coreml_parakeet_result(
    model: &crate::coreml_asr::CoreMlParakeet,
    samples: &[f32],
) -> Result<TranscriptionResult, String> {
    let pieces = model.transcribe_pieces_timed(samples)?;
    let text = pieces.iter().map(|p| p.text.as_str()).collect::<String>();
    let tokens = pieces
        .into_iter()
        .map(|piece| TimedToken {
            text: piece.text,
            start: piece.start,
            // Real TDT-predicted duration. This used to be a fixed 80 ms, which
            // made every token's end synthetic; `timed_tokens_to_words` then
            // overwrote it with the NEXT word's start, so a word's cue ran
            // through any pause that followed it.
            end: piece.start + piece.duration,
        })
        .collect();
    Ok(TranscriptionResult {
        text: text.trim().to_string(),
        tokens,
    })
}

#[cfg(target_os = "macos")]
fn timed_tokens_to_words(tokens: &[TimedToken]) -> Vec<crate::coreml_asr::TimedTok> {
    let mut words: Vec<crate::coreml_asr::TimedTok> = Vec::new();
    for token in tokens {
        let starts_word = token
            .text
            .chars()
            .next()
            .map(|ch| ch.is_whitespace())
            .unwrap_or(false);
        let clean = token.text.trim().to_string();
        if clean.is_empty() {
            continue;
        }
        if starts_word || words.is_empty() {
            words.push(crate::coreml_asr::TimedTok {
                text: clean,
                start: token.start,
                end: token.end,
            });
        } else if let Some(last) = words.last_mut() {
            last.text.push_str(&clean);
            last.end = token.end;
        }
    }
    // Word ends come from the model's own durations. Two clamps only:
    // never shorter than one 80 ms frame (zero-width cues are invalid in SRT,
    // and a piece that shares a frame with the next reports 0), and never past
    // the next word's start (durations are per-token, so a chain can otherwise
    // overhang). Deliberately does NOT stretch each cue to the next word's
    // start: that erased real pauses and made word-level highlighting run on
    // through silence.
    for i in 0..words.len() {
        let floor = words[i].start + 0.08;
        let mut end = words[i].end.max(floor);
        if i + 1 < words.len() {
            end = end.min(words[i + 1].start.max(floor));
        }
        words[i].end = end;
    }
    words
}
