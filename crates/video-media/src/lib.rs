mod audio;
mod audio_finish;
mod captions;
mod color;
mod effects;
mod evidence;
mod final_render;
mod probe;
mod process;
mod reframe;
mod rough_render;
mod toolchain;
mod waveform;

use std::path::{Path, PathBuf};
use thiserror::Error;
use video_core::process_runner::ProcessRunError;

pub use audio::{extract_audio_f32, extract_audio_f32_with_receipt, AudioError};
pub use audio_finish::{
    dialogue_chain_filter, duck_track_under_speech, ducking_filter, measure_loudness_and_clipping,
    measure_room_tone_step, process_dialogue_stem, process_dialogue_stem_with_receipt,
    AudioFinishError, CompressorParams, DeEsserParams, DialogueChainParams, LoudnessMeasurement,
    RoomToneStep,
};
pub use captions::{
    build_caption_document, render_preset_with_captions, render_preset_with_captions_and_reframe,
    render_preset_with_captions_and_reframe_with_caption_receipt,
    render_preset_with_captions_and_reframe_with_receipt, render_subtitled, resolve_font_for_text,
    CaptionCueModel, CaptionDocument, CaptionFontDescriptor, CaptionFontNotice, CaptionPlatform,
    CaptionProfile, CaptionSafeZone, CAPTION_MODEL_SCHEMA_VERSION,
};
pub use color::{
    color_filter_chain, detect_color_space, probe_source_color_tags, render_contact_sheet,
    verify_output_color_metadata, ColorCorrection, ColorSpaceKind, CreativeLut,
    ExpectedColorMetadata, ShotMatchTarget, SourceColorTags, REC709_SDR_METADATA,
};
pub use effects::{
    ass_subtitles_toolchain_status, effect_render_toolchain_identity, remotion_toolchain_status,
    render_effect_ass_preview, render_effect_motion, render_effect_remotion_preview,
    render_effect_still, EffectCard, ExternalEffectPreviewOutcome,
};
pub use evidence::{compose_decision_evidence, extract_frame};
pub use final_render::{
    render_master, render_master_contact_sheet, render_to_preset, MasterRenderRequest,
};
pub use probe::{probe, MediaMetadata, ProbeError};
pub use reframe::ReframeAnchor;
pub use rough_render::{
    render_boundary_probe, render_segments, render_segments_with_receipt, render_source_segments,
    RenderSegment, SourceRenderSegment,
};
pub use toolchain::{
    resolve as resolve_toolchain, MediaCapabilities, MediaToolchain, PackId, PackResourceId,
    PackResourceResolver, ToolchainError, VerifiedResource, ResolverError,
};
pub use waveform::{render_waveform, render_waveform_range};

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("render requires at least one segment")]
    NoSegments,
    #[error("render segment has an invalid range: {start_ms}..{end_ms}")]
    InvalidSegment { start_ms: i64, end_ms: i64 },
    #[error("render output must not overwrite the source")]
    OutputIsInput,
    #[error("ffmpeg could not start: {0}")]
    Start(#[source] std::io::Error),
    #[error("ffmpeg failed: {0}")]
    Failed(String),
    #[error("required FFmpeg capability is unavailable: {0}")]
    CapabilityMissing(String),
    #[error("output color metadata does not match the profile: expected {expected}, got {actual}")]
    ColorMetadataMismatch { expected: String, actual: String },
    #[error("caption card worker could not start: {0}")]
    CaptionStart(#[source] std::io::Error),
    #[error("caption card worker failed: {0}")]
    CaptionFailed(String),
    #[error("could not resolve a matched ffmpeg/ffprobe toolchain: {0}")]
    Toolchain(#[from] ToolchainError),
    #[error("could not materialize embedded worker: {0}")]
    WorkerMaterialize(#[from] video_core::ContentStoreError),
    #[error("ffmpeg process error: {0}")]
    Process(#[from] ProcessRunError),
    /// An external effect-render toolchain (libass-enabled ffmpeg for the
    /// `ass` renderer, or the Node/Remotion package for the `remotion`
    /// renderer) is not available on this machine. Distinct from `Failed`
    /// (the toolchain ran and produced an error) and `CapabilityMissing`
    /// (an ffmpeg *filter/encoder* is absent) — this is "the whole
    /// renderer's dependency chain could not even be resolved", named
    /// explicitly so a caller never mistakes it for the toolchain having
    /// run and rejected the input.
    #[error("external renderer toolchain unavailable: {0}")]
    RendererUnavailable(String),
    #[error("effect renderer I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("effect renderer prop serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Build a [`video_core::StageReceipt`] for a completed ffmpeg render/probe
/// stage: hashes `input`/`output`, hashes `parameters`, and records the
/// resolved [`MediaToolchain`] identity under the `"ffmpeg"` key.
pub(crate) fn build_receipt(
    stage: &str,
    input: &Path,
    parameters: &serde_json::Value,
    output: &Path,
) -> Result<video_core::StageReceipt, RenderError> {
    build_receipt_multi(stage, &[input], parameters, output)
}

pub(crate) fn build_receipt_multi(
    stage: &str,
    inputs: &[&Path],
    parameters: &serde_json::Value,
    output: &Path,
) -> Result<video_core::StageReceipt, RenderError> {
    let toolchain = toolchain::resolve()?;
    let mut toolchains = std::collections::BTreeMap::new();
    toolchains.insert("ffmpeg".to_string(), toolchain.identity());
    video_core::StageReceipt::build(
        stage,
        env!("CARGO_PKG_VERSION"),
        inputs,
        parameters,
        toolchains,
        &[output],
    )
    .map_err(|error| RenderError::Failed(error.to_string()))
}

/// The `ffmpeg` half of the resolved, verified toolchain pair (§10.3). Every
/// call site in this crate goes through this instead of a bare
/// `Command::new("ffmpeg")`, so `ffmpeg` and `ffprobe` (used by [`probe`])
/// always come from the same resolved build.
pub(crate) fn ffmpeg_path() -> Result<PathBuf, ToolchainError> {
    Ok(toolchain::resolve()?.ffmpeg)
}
