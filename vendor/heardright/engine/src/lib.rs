//! `heardright-engine` library — exposes the in-process types that both the
//! sidecar binary (`main.rs`) and the integration tests (`tests/` dir) need.
//!
//! The binary entry point is intentionally separate from the library so the
//! tests can construct a `runtime::EngineRuntime` directly without spawning
//! a subprocess or driving stdin/stdout.

#![allow(
    clippy::result_large_err,
    clippy::large_enum_variant,
    clippy::too_many_arguments,
    clippy::doc_lazy_continuation,
    clippy::manual_clamp
)] // intentional: large variants live on cold error paths, a few arg-heavy platform fns, doc-comment style

pub mod app_launch;
#[cfg(target_os = "macos")]
pub mod apple_foundation;
pub mod asr;
/// Native AVFoundation decode; fronts the ffmpeg path in `file_transcribe`.
#[cfg(target_os = "macos")]
pub mod av_decode;
pub mod calibration;
pub mod canonical_polish_harness;
pub mod command_classify;
pub mod command_dispatch;
#[cfg(target_os = "macos")]
pub mod coreml;
#[cfg(target_os = "macos")]
pub mod coreml_asr;
pub mod crash_capture;
pub mod delivery;
pub mod file_transcribe;
pub mod focus;
pub mod guardrail_replay;
pub mod inference_gate;
pub mod ipc;
pub mod l1c1_serialize;
pub mod l3_cleanup;
#[cfg(target_os = "macos")]
pub mod macos_input;
pub mod owner_diagnostics;
pub mod runtime;
pub mod screen_vocab;
pub mod settings;
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub mod sherpa_kws;
#[cfg(test)]
mod test_support;
pub mod text_polish;
pub mod vad;
pub mod vocabulary;
#[cfg(target_os = "macos")]
pub mod whisper_coreml;
#[cfg(target_os = "windows")]
pub mod whisper_win;
pub mod worker;
