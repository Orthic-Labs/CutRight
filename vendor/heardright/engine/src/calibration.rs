//! One-shot recognition setup used by onboarding.
//!
//! Every candidate decodes identical retained PCM. A selected route is written
//! only after it passes the fixed-script quality gate; ordinary dictation never
//! retries another route on its own.

use std::fs::{self, OpenOptions};
use std::io::Write;

use serde::{Deserialize, Serialize};
use unicode_normalization_alignments::UnicodeNormalization;

use crate::asr::{AsrEp, AsrRuntime};
use crate::settings::{self, PersistedRecognitionRoute};

pub const REFERENCE_VERSION: &str = "onboarding-simple-en-v1";
pub const REFERENCE_TEXT: &str = "Today I am testing this microphone with a short and simple passage. I will speak at a calm and steady pace. The room feels quiet, and my voice sounds clear. A small bird rests beside the garden path. Soft light falls across the green leaves. I can describe each detail without rushing. This test helps the app select a stable speech system for this computer. I will finish the passage now and wait while the app checks the result.";
const SAMPLE_RATE: usize = 16_000;
const MIN_CAPTURE_SECONDS: f32 = 30.0;
const MAX_WER: f32 = 0.07;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationPlatform {
    AppleSilicon,
    IntelMac,
    Windows,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DmlAdapter {
    pub index: i32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalibrationRoute {
    pub id: String,
    pub encoder_compute: String,
    pub decoder_compute: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_name: Option<String>,
}

impl CalibrationRoute {
    fn persisted(&self) -> PersistedRecognitionRoute {
        PersistedRecognitionRoute {
            id: self.id.clone(),
            encoder_compute: self.encoder_compute.clone(),
            decoder_compute: self.decoder_compute.clone(),
            provider: self.provider.clone(),
            adapter_index: self.adapter_index,
            adapter_name: self.adapter_name.clone(),
        }
    }

    fn ep(&self) -> AsrEp {
        #[cfg(target_os = "windows")]
        if self.provider == "dml" {
            return AsrEp::Dml;
        }
        AsrEp::Cpu
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationAttempt {
    pub route: CalibrationRoute,
    pub transcript: String,
    pub wer: Option<f32>,
    pub latency_ms: u64,
    pub deletion_run_max: usize,
    pub premature_ending: bool,
    pub concentrated_deletion: bool,
    pub status: String,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_dir_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compile_cache: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<serde_json::Value>,
    pub words_per_second: Option<f32>,
    pub max_silent_emission_gap_seconds: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationResult {
    pub status: String,
    pub reference_version: String,
    pub selected_route: Option<CalibrationRoute>,
    pub attempts: Vec<CalibrationAttempt>,
    /// Route one passing means no transmission. Any route-one failure makes the
    /// local report eligible for automatic upload; it never contains PCM.
    pub upload_eligible: bool,
    pub failure_code: Option<String>,
    pub vad_speech_ratio_quarters: Option<[f32; 4]>,
    pub vad_speech_frame_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QualityGate {
    pub wer: f32,
    pub premature_ending: bool,
    pub concentrated_deletion: bool,
    pub deletion_run_max: usize,
}

pub fn normalize_transcript(text: &str) -> String {
    text.nfkd()
        .map(|(character, _alignment)| character)
        .filter(|character| !is_combining_mark(*character))
        .flat_map(char::to_lowercase)
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// Keep Unicode normalization dependency-free: current onboarding text is Latin
// and this covers combining diacritics emitted by platform ASR.
fn is_combining_mark(character: char) -> bool {
    matches!(character as u32, 0x0300..=0x036f | 0x1ab0..=0x1aff | 0x1dc0..=0x1dff | 0x20d0..=0x20ff | 0xfe20..=0xfe2f)
}

pub fn quality_gate(reference: &str, transcript: &str) -> QualityGate {
    let reference = normalize_transcript(reference);
    let transcript = normalize_transcript(transcript);
    let expected: Vec<&str> = reference.split_whitespace().collect();
    let actual: Vec<&str> = transcript.split_whitespace().collect();
    let operations = alignment(&expected, &actual);
    let errors = operations
        .iter()
        .filter(|op| !matches!(op, Edit::Match))
        .count();
    let longest_deletion = longest_deletion_run(&operations);
    let terminal_deletion = terminal_deletion_run(&operations);
    let deletion_threshold = (expected.len() / 10).max(4);
    QualityGate {
        wer: errors as f32 / expected.len().max(1) as f32,
        premature_ending: terminal_deletion >= deletion_threshold,
        concentrated_deletion: longest_deletion >= deletion_threshold,
        deletion_run_max: longest_deletion,
    }
}

#[derive(Clone, Copy)]
enum Edit {
    Match,
    Substitute,
    Delete,
    Insert,
}

fn alignment(expected: &[&str], actual: &[&str]) -> Vec<Edit> {
    let mut distance = vec![vec![0usize; actual.len() + 1]; expected.len() + 1];
    for (row, value) in distance.iter_mut().enumerate() {
        value[0] = row;
    }
    for column in 0..=actual.len() {
        distance[0][column] = column;
    }
    for row in 1..=expected.len() {
        for column in 1..=actual.len() {
            distance[row][column] = if expected[row - 1] == actual[column - 1] {
                distance[row - 1][column - 1]
            } else {
                1 + distance[row - 1][column - 1]
                    .min(distance[row - 1][column])
                    .min(distance[row][column - 1])
            };
        }
    }
    let mut operations = Vec::new();
    let (mut row, mut column) = (expected.len(), actual.len());
    while row > 0 || column > 0 {
        if row > 0 && column > 0 && expected[row - 1] == actual[column - 1] {
            operations.push(Edit::Match);
            row -= 1;
            column -= 1;
        } else if row > 0
            && column > 0
            && distance[row][column] == distance[row - 1][column - 1] + 1
        {
            operations.push(Edit::Substitute);
            row -= 1;
            column -= 1;
        } else if row > 0 && distance[row][column] == distance[row - 1][column] + 1 {
            operations.push(Edit::Delete);
            row -= 1;
        } else {
            operations.push(Edit::Insert);
            column -= 1;
        }
    }
    operations.reverse();
    operations
}

fn longest_deletion_run(operations: &[Edit]) -> usize {
    let (mut longest, mut current) = (0usize, 0usize);
    for operation in operations {
        if matches!(operation, Edit::Delete) {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    longest
}

fn terminal_deletion_run(operations: &[Edit]) -> usize {
    operations
        .iter()
        .rev()
        .take_while(|operation| matches!(operation, Edit::Delete))
        .count()
}

pub fn candidate_routes(
    platform: CalibrationPlatform,
    adapters: &[DmlAdapter],
) -> Vec<CalibrationRoute> {
    let route = |id: &str, encoder: &str, decoder: &str| CalibrationRoute {
        id: id.to_string(),
        encoder_compute: encoder.to_string(),
        decoder_compute: decoder.to_string(),
        provider: if matches!(
            platform,
            CalibrationPlatform::AppleSilicon | CalibrationPlatform::IntelMac
        ) {
            "coreml"
        } else {
            "cpu"
        }
        .to_string(),
        adapter_index: None,
        adapter_name: None,
    };
    match platform {
        CalibrationPlatform::AppleSilicon => vec![
            route("ane-all", "neural_engine", "neural_engine"),
            route("ane-encoder-cpu-decoder", "neural_engine", "cpu_only"),
            route("gpu-encoder-ane-decoder", "cpu_gpu", "neural_engine"),
            route("gpu-encoder-cpu-decoder", "cpu_gpu", "cpu_only"),
            route("cpu-only", "cpu_only", "cpu_only"),
        ],
        CalibrationPlatform::IntelMac => vec![
            route("gpu-all", "cpu_gpu", "cpu_gpu"),
            route("gpu-encoder-cpu-decoder", "cpu_gpu", "cpu_only"),
            route("cpu-only", "cpu_only", "cpu_only"),
        ],
        CalibrationPlatform::Windows => {
            let mut dml = adapters.to_vec();
            dml.sort_by_key(|adapter| {
                (
                    !adapter.name.to_ascii_lowercase().contains("nvidia"),
                    adapter.index,
                )
            });
            let mut routes = dml
                .into_iter()
                .map(|adapter| CalibrationRoute {
                    id: format!("dml-{}", adapter.index),
                    encoder_compute: "dml".to_string(),
                    decoder_compute: "dml".to_string(),
                    provider: "dml".to_string(),
                    adapter_index: Some(adapter.index),
                    adapter_name: Some(adapter.name),
                })
                .collect::<Vec<_>>();
            routes.push(route("cpu-only", "cpu_only", "cpu_only"));
            routes
        }
        CalibrationPlatform::Other => vec![route("cpu-only", "cpu_only", "cpu_only")],
    }
}

pub fn local_platform() -> CalibrationPlatform {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return CalibrationPlatform::AppleSilicon;
    #[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
    return CalibrationPlatform::IntelMac;
    #[cfg(target_os = "windows")]
    return CalibrationPlatform::Windows;
    #[allow(unreachable_code)]
    CalibrationPlatform::Other
}

pub fn run(models_dir: &std::path::Path, samples: &[f32]) -> CalibrationResult {
    let collect_diagnostics = std::env::var_os("HR_CALIBRATION_COLLECT_DIAGNOSTICS").is_some();
    run_with_diagnostics(models_dir, samples, collect_diagnostics)
}

fn run_with_diagnostics(
    models_dir: &std::path::Path,
    samples: &[f32],
    collect_diagnostics: bool,
) -> CalibrationResult {
    if samples.len() < (SAMPLE_RATE as f32 * MIN_CAPTURE_SECONDS) as usize {
        let result = CalibrationResult {
            status: "capture_failed".to_string(),
            reference_version: REFERENCE_VERSION.to_string(),
            selected_route: None,
            attempts: Vec::new(),
            upload_eligible: false,
            failure_code: Some("capture_too_short".to_string()),
            vad_speech_ratio_quarters: None,
            vad_speech_frame_count: None,
        };
        log_result(&result);
        return result;
    }
    let adapters = enumerate_dml_adapters().unwrap_or_default();
    let routes = candidate_routes(local_platform(), &adapters);
    let audio_policy =
        std::env::var("HR_AUDIO_CONDITIONING_POLICY").unwrap_or_else(|_| "default".into());
    let conditioned = heardright_core::audio_conditioning::condition_for_asr(
        samples,
        SAMPLE_RATE as u32,
        &audio_policy,
    );
    let mut attempts = Vec::with_capacity(routes.len());
    let mut failure_vad = None;
    for route in routes {
        let persisted = route.persisted();
        settings::apply_calibration_route_environment(&persisted);
        #[cfg(target_os = "macos")]
        crate::coreml_asr::clear_calibration_window_stats();
        let started = std::time::Instant::now();
        let attempt = match AsrRuntime::load(models_dir, route.ep()).and_then(|mut model| {
            verify_loaded_route(&model, &route)?;
            let (fingerprint, model_dir_sha256, compile_cache) = loaded_model_metadata(&model);
            model
                .transcribe(&conditioned)
                .map(|transcript| (transcript, fingerprint, model_dir_sha256, compile_cache))
        }) {
            Ok((transcript, model_fingerprint, model_dir_sha256, compile_cache)) => {
                let gate = quality_gate(REFERENCE_TEXT, &transcript);
                let duration = samples.len() as f32 / SAMPLE_RATE as f32;
                let words_per_second = normalize_transcript(&transcript).split_whitespace().count()
                    as f32
                    / duration.max(0.001);
                CalibrationAttempt {
                    route: route.clone(),
                    transcript,
                    wer: Some(gate.wer),
                    latency_ms: started.elapsed().as_millis() as u64,
                    deletion_run_max: gate.deletion_run_max,
                    premature_ending: gate.premature_ending,
                    concentrated_deletion: gate.concentrated_deletion,
                    status: if gate.wer <= MAX_WER
                        && !gate.premature_ending
                        && !gate.concentrated_deletion
                    {
                        "passed"
                    } else {
                        "quality_failed"
                    }
                    .to_string(),
                    error: None,
                    model_fingerprint,
                    model_dir_sha256,
                    compile_cache,
                    diagnostics: latest_window_stats(),
                    words_per_second: Some(words_per_second),
                    max_silent_emission_gap_seconds: None,
                }
            }
            Err(error) => CalibrationAttempt {
                route: route.clone(),
                transcript: String::new(),
                wer: None,
                latency_ms: started.elapsed().as_millis() as u64,
                deletion_run_max: 0,
                premature_ending: false,
                concentrated_deletion: false,
                status: "recognition_provider_failed".to_string(),
                error: Some(error),
                model_fingerprint: None,
                model_dir_sha256: None,
                compile_cache: None,
                diagnostics: latest_window_stats(),
                words_per_second: None,
                max_silent_emission_gap_seconds: None,
            },
        };
        tracing::info!(route = %route.id, elapsed_ms = started.elapsed().as_millis(), status = %attempt.status, "onboarding calibration candidate");
        let passed = attempt.status == "passed";
        attempts.push(attempt);
        if passed {
            let upload_eligible = attempts
                .first()
                .is_some_and(|first| first.status != "passed");
            let (vad_speech_ratio_quarters, vad_speech_frame_count) =
                if should_measure_vad(collect_diagnostics, upload_eligible) {
                    measure_vad_quarters(
                        failure_vad.get_or_insert_with(crate::vad::SpeechVad::new),
                        samples,
                    )
                } else {
                    (None, None)
                };
            let result = match settings::persist_recognition_route(&persisted) {
                Ok(()) => CalibrationResult {
                    status: "passed".to_string(),
                    reference_version: REFERENCE_VERSION.to_string(),
                    selected_route: Some(route),
                    upload_eligible,
                    attempts,
                    failure_code: None,
                    vad_speech_ratio_quarters,
                    vad_speech_frame_count,
                },
                Err(error) => CalibrationResult {
                    status: "failed".to_string(),
                    reference_version: REFERENCE_VERSION.to_string(),
                    selected_route: None,
                    upload_eligible,
                    attempts,
                    failure_code: Some(format!("persist_route:{error}")),
                    vad_speech_ratio_quarters,
                    vad_speech_frame_count,
                },
            };
            log_result(&result);
            return result;
        }
        if attempts.len() == 1 {
            failure_vad = Some(crate::vad::SpeechVad::new());
        }
    }
    let (vad_speech_ratio_quarters, vad_speech_frame_count) = failure_vad
        .as_mut()
        .map(|vad| measure_vad_quarters(vad, samples))
        .unwrap_or((None, None));
    let result = CalibrationResult {
        status: "failed".to_string(),
        reference_version: REFERENCE_VERSION.to_string(),
        selected_route: None,
        upload_eligible: attempts
            .first()
            .is_some_and(|first| first.status != "passed"),
        attempts,
        failure_code: Some("no_route_passed".to_string()),
        vad_speech_ratio_quarters,
        vad_speech_frame_count,
    };
    log_result(&result);
    result
}

fn should_measure_vad(collect_diagnostics: bool, failure_upload_eligible: bool) -> bool {
    collect_diagnostics || failure_upload_eligible
}

fn measure_vad_quarters(
    vad: &mut crate::vad::SpeechVad,
    samples: &[f32],
) -> (Option<[f32; 4]>, Option<usize>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while vad.status() == crate::vad::VadStatus::Loading && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    if vad.status() != crate::vad::VadStatus::Ready {
        return (None, None);
    }
    vad.reset();
    let mut speech_frames = 0usize;
    let ratios = std::array::from_fn(|quarter| {
        let start = samples.len() * quarter / 4;
        let end = samples.len() * (quarter + 1) / 4;
        let mut total = 0usize;
        let mut speech = 0usize;
        for frame in samples[start..end].chunks_exact(512) {
            total += 1;
            if vad.observe(frame) {
                speech += 1;
            }
        }
        speech_frames += speech;
        speech as f32 / total.max(1) as f32
    });
    (Some(ratios), Some(speech_frames))
}

#[cfg(target_os = "macos")]
fn loaded_model_metadata(model: &AsrRuntime) -> (Option<String>, Option<String>, Option<String>) {
    match model {
        AsrRuntime::CoreMlParakeet(model) => (
            Some(model.model_fingerprint().to_string()),
            model.model_dir_sha256().map(str::to_string),
            Some(model.compile_cache().to_string()),
        ),
        _ => (None, None, None),
    }
}

#[cfg(not(target_os = "macos"))]
fn loaded_model_metadata(_model: &AsrRuntime) -> (Option<String>, Option<String>, Option<String>) {
    (None, None, None)
}

#[cfg(target_os = "macos")]
fn latest_window_stats() -> Option<serde_json::Value> {
    let latest = crate::coreml_asr::take_latest_asr_window_stats();
    let windows = crate::coreml_asr::take_calibration_window_stats();
    serde_json::to_value(serde_json::json!({
        "latest": latest,
        "windows": windows,
    }))
    .ok()
}

#[cfg(not(target_os = "macos"))]
fn latest_window_stats() -> Option<serde_json::Value> {
    None
}

fn verify_loaded_route(model: &AsrRuntime, route: &CalibrationRoute) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    if route.provider == "coreml" {
        let AsrRuntime::CoreMlParakeet(model) = model else {
            return Err("calibration route did not load native CoreML Parakeet".to_string());
        };
        let (encoder, decoder) = model.configured_compute_route();
        if encoder != route.encoder_compute || decoder != route.decoder_compute {
            return Err(format!(
                "calibration route mismatch: requested {}/{} loaded {encoder}/{decoder}",
                route.encoder_compute, route.decoder_compute
            ));
        }
    }
    #[cfg(target_os = "windows")]
    if route.provider == "dml" {
        let configured = std::env::var("HR_DML_DEVICE_ID").ok();
        if configured.as_deref()
            != route
                .adapter_index
                .map(|index| index.to_string())
                .as_deref()
        {
            return Err(
                "calibration DirectML adapter did not reach session configuration".to_string(),
            );
        }
        if !matches!(model, AsrRuntime::Parakeet(_)) {
            return Err("calibration DirectML route did not load Parakeet".to_string());
        }
    }
    let _ = (model, route);
    Ok(())
}

fn log_result(result: &CalibrationResult) {
    let path = settings::app_data_root().join("onboarding-calibration.jsonl");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let (Ok(mut file), Ok(line)) = (
        OpenOptions::new().create(true).append(true).open(path),
        serde_json::to_string(result),
    ) {
        let _ = writeln!(file, "{line}");
    }
}

#[cfg(target_os = "windows")]
fn enumerate_dml_adapters() -> Result<Vec<DmlAdapter>, String> {
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ERROR_NOT_FOUND};
    let factory: IDXGIFactory1 =
        unsafe { CreateDXGIFactory1().map_err(|error| format!("create DXGI factory: {error}"))? };
    let mut adapters = Vec::new();
    for index in 0.. {
        match unsafe { factory.EnumAdapters1(index) } {
            Ok(adapter) => {
                let description = unsafe {
                    adapter
                        .GetDesc1()
                        .map_err(|error| format!("read DXGI adapter: {error}"))?
                };
                let name = String::from_utf16_lossy(&description.Description)
                    .trim_end_matches('\0')
                    .trim()
                    .to_string();
                if !name.is_empty() && description.Flags & 2 == 0 {
                    adapters.push(DmlAdapter {
                        index: index as i32,
                        name,
                    });
                }
            }
            Err(error) if error.code() == DXGI_ERROR_NOT_FOUND => break,
            Err(error) => return Err(format!("enumerate DXGI adapter: {error}")),
        }
    }
    Ok(adapters)
}

#[cfg(not(target_os = "windows"))]
fn enumerate_dml_adapters() -> Result<Vec<DmlAdapter>, String> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    #[test]
    fn diagnostic_success_requests_vad_even_without_failure_upload() {
        assert!(super::should_measure_vad(true, false));
        assert!(super::should_measure_vad(false, true));
        assert!(!super::should_measure_vad(false, false));
    }
    use super::*;

    #[test]
    fn normalization_folds_case_punctuation_and_marks() {
        assert_eq!(normalize_transcript("Café—TEST,  now!"), "cafe test now");
    }

    #[test]
    fn wer_is_zero_for_normalized_reference() {
        let gate = quality_gate(REFERENCE_TEXT, &normalize_transcript(REFERENCE_TEXT));
        assert_eq!(gate.wer, 0.0);
        assert!(!gate.premature_ending);
    }

    #[test]
    fn deletion_guard_rejects_a_missing_tail() {
        let reference = "one two three four five six seven eight nine ten";
        let gate = quality_gate(reference, "one two three four five six");
        assert!(gate.premature_ending);
        assert!(gate.concentrated_deletion);
    }

    #[test]
    fn fixed_script_wer_boundary_is_seven_percent() {
        let words = normalize_transcript(REFERENCE_TEXT)
            .split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>();
        assert_eq!(words.len(), 79);
        let with_errors = |count: usize| {
            let mut actual = words.clone();
            for word in actual.iter_mut().take(count) {
                *word = "wrong".to_string();
            }
            actual.join(" ")
        };
        assert!(quality_gate(REFERENCE_TEXT, &with_errors(5)).wer <= MAX_WER);
        assert!(quality_gate(REFERENCE_TEXT, &with_errors(6)).wer > MAX_WER);
    }

    #[test]
    fn windows_prefers_nvidia_then_other_dml_then_cpu() {
        let routes = candidate_routes(
            CalibrationPlatform::Windows,
            &[
                DmlAdapter {
                    index: 4,
                    name: "Intel Arc".into(),
                },
                DmlAdapter {
                    index: 2,
                    name: "NVIDIA RTX".into(),
                },
            ],
        );
        assert_eq!(
            routes
                .iter()
                .map(|route| route.id.as_str())
                .collect::<Vec<_>>(),
            vec!["dml-2", "dml-4", "cpu-only"]
        );
    }

    #[test]
    fn apple_routes_keep_ane_first_and_cpu_last() {
        let routes = candidate_routes(CalibrationPlatform::AppleSilicon, &[]);
        assert_eq!(routes.first().unwrap().id, "ane-all");
        assert_eq!(routes.last().unwrap().id, "cpu-only");
        assert_eq!(routes.len(), 5);
        assert_eq!(
            routes
                .iter()
                .map(|route| (
                    route.encoder_compute.as_str(),
                    route.decoder_compute.as_str()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("neural_engine", "neural_engine"),
                ("neural_engine", "cpu_only"),
                ("cpu_gpu", "neural_engine"),
                ("cpu_gpu", "cpu_only"),
                ("cpu_only", "cpu_only"),
            ]
        );
    }
}
