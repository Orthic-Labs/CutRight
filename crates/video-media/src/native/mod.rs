//! Stable, Rust-owned contract for optional macOS media acceleration.
//!
//! Final renders remain outside this boundary: callers select `Legacy` until
//! capability-specific parity evidence promotes a native operation.

mod macos;
pub mod protocol;

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use macos::MacMediaWorker;
pub use protocol::{RequestEnvelope, ResponseEnvelope, MAC_MEDIA_PROTOCOL_VERSION};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MacNativeMode {
    Legacy,
    Shadow,
    Native,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeRequestContext {
    pub request_id: String,
    pub timeout: Duration,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacMediaCapabilities {
    pub av_foundation: bool,
    pub vision: bool,
    pub caption: bool,
    pub preview: bool,
    pub audio: bool,
    pub metal: bool,
    pub os_version: String,
    pub worker_version: String,
    #[serde(default)]
    pub worker_blake3: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRationalTime {
    pub numerator: i64,
    pub denominator: i32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeVideoTrack {
    pub track_id: i32,
    pub natural_width: f64,
    pub natural_height: f64,
    pub preferred_transform: Vec<f64>,
    pub nominal_frame_rate: f64,
    pub minimum_frame_duration: Option<NativeRationalTime>,
    pub time_range_start: Option<NativeRationalTime>,
    pub time_range_duration: Option<NativeRationalTime>,
    pub color_properties: std::collections::BTreeMap<String, String>,
    pub hdr: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAudioTrack {
    pub track_id: i32,
    pub time_range_start: Option<NativeRationalTime>,
    pub time_range_duration: Option<NativeRationalTime>,
    pub language_code: Option<String>,
    pub format_descriptions: Vec<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAssetInfo {
    pub duration: Option<NativeRationalTime>,
    pub video_tracks: Vec<NativeVideoTrack>,
    pub audio_tracks: Vec<NativeAudioTrack>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeFrameAnalysis {
    pub source_frame_index: i32,
    pub timestamp: NativeRationalTime,
    pub orientation_transform: String,
    pub vision_revision: i32,
    pub faces: Vec<NativeVisionBox>,
    pub bodies: Vec<NativeVisionBox>,
    pub ocr_boxes: Vec<NativeOcrBox>,
    pub saliency: Option<NativeVisionBox>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeVisionBox {
    pub center_x: f64,
    pub center_y: f64,
    pub area: f64,
    pub confidence: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeOcrBox {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub confidence: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRenderArtifact {
    pub output_path: PathBuf,
    pub width: i32,
    pub height: i32,
    pub color_space: String,
    pub renderer: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAudioFeatures {
    pub sample_rate: f64,
    pub channel_count: i32,
    pub sample_count: i32,
    pub rms: f64,
    pub peak: f64,
    pub zero_crossing_rate: f64,
    pub spectral_flux: f64,
    pub envelope: Vec<f64>,
    pub classification: Option<String>,
    pub classification_confidence: Option<f64>,
    pub classifier_revision: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeFrameRequest {
    pub source_path: PathBuf,
    pub source_frame_index: i32,
    pub timestamp: NativeRationalTime,
    pub sequence_id: Option<String>,
    pub orientation: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeFramesRequest {
    pub frames: Vec<NativeFrameRequest>,
    pub allowed_roots: Vec<PathBuf>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCaptionRequest {
    pub output_path: PathBuf,
    pub width: i32,
    pub height: i32,
    pub text: String,
    pub vertical: bool,
    pub allowed_roots: Vec<PathBuf>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativePreviewRequest {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub crop_x: Option<f64>,
    pub crop_y: Option<f64>,
    pub crop_width: Option<f64>,
    pub crop_height: Option<f64>,
    pub rotation_degrees: Option<f64>,
    pub allowed_roots: Vec<PathBuf>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAudioRequest {
    pub source_path: PathBuf,
    pub start_seconds: Option<f64>,
    pub duration_seconds: Option<f64>,
    pub allowed_roots: Vec<PathBuf>,
}

/// Rust-owned locked timeline render contract. The typed graph is validated
/// again before it crosses the worker boundary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeTimelineRenderRequest {
    pub schema_version: u32,
    pub locked_cut_sha256: String,
    pub graph: video_core::FinishRenderGraph,
    pub output_path: PathBuf,
    pub allowed_roots: Vec<PathBuf>,
    pub video: NativeVideoOutputSpec,
    pub audio: NativeAudioOutputSpec,
    pub mode: MacNativeMode,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeVideoOutputSpec {
    pub width: u32,
    pub height: u32,
    pub frame_rate_num: u32,
    pub frame_rate_den: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeAudioOutputSpec {
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeTimelineRenderResult {
    pub schema_version: u32,
    pub artifact_sha256: String,
    pub pixel_sha256: String,
    pub audio_sha256: String,
    pub duration: NativeRationalTime,
    pub rendered_frames: u64,
    pub audio_frames: u64,
    pub node_receipts: Vec<serde_json::Value>,
}

#[derive(Debug, Error)]
pub enum NativeMediaError {
    #[error("macOS native media worker is unsupported on this platform")]
    UnsupportedPlatform,
    #[error("macOS native media capability is unsupported: {0}")]
    Unsupported(String),
    #[error("native media worker could not start: {0}")]
    Start(String),
    #[error("native media worker I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("native media worker could not be materialized: {0}")]
    Materialize(#[from] video_core::ContentStoreError),
    #[error("native media worker emitted invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("native media worker request {request_id} timed out after {timeout:?}")]
    Timeout {
        request_id: String,
        timeout: Duration,
    },
    #[error("native media worker request {request_id} was cancelled")]
    Cancelled { request_id: String },
    #[error("native media worker response correlation failed: expected {expected}, got {actual}")]
    Correlation { expected: String, actual: String },
    #[error("native media worker exited unexpectedly: {0}")]
    UnexpectedExit(String),
    #[error("native media worker protocol error: {0}")]
    Protocol(String),
    #[error("native media worker rejected path {0}")]
    InvalidPath(PathBuf),
}

pub trait MacMediaBackend: Send + Sync {
    fn capabilities(&self) -> Result<MacMediaCapabilities, NativeMediaError>;
    fn inspect_asset(
        &self,
        context: &NativeRequestContext,
        source: &Path,
    ) -> Result<NativeAssetInfo, NativeMediaError>;
    fn analyze_frames(
        &self,
        context: &NativeRequestContext,
        request: &AnalyzeFramesRequest,
    ) -> Result<Vec<NativeFrameAnalysis>, NativeMediaError>;
    fn render_caption(
        &self,
        context: &NativeRequestContext,
        request: &NativeCaptionRequest,
    ) -> Result<NativeRenderArtifact, NativeMediaError>;
    fn render_preview(
        &self,
        context: &NativeRequestContext,
        request: &NativePreviewRequest,
    ) -> Result<NativeRenderArtifact, NativeMediaError>;
    fn audio_features(
        &self,
        context: &NativeRequestContext,
        request: &NativeAudioRequest,
    ) -> Result<NativeAudioFeatures, NativeMediaError>;
    fn render_timeline(
        &self,
        context: &NativeRequestContext,
        request: &NativeTimelineRenderRequest,
    ) -> Result<NativeTimelineRenderResult, NativeMediaError>;
    fn cancel(&self, request_id: &str) -> Result<(), NativeMediaError>;
}
