//! Full-quality libx264 delivery renders, plus the software archival/master
//! path (plan §15.2 "Export": "a software master/delivery path in addition
//! to hardware preview" — `render_to_preset` below is already a software
//! x264 delivery encode tuned for speed; `render_master` is the distinct,
//! higher-fidelity, fully color-managed archival path, encoded with the
//! software `prores_ks` codec so its artifact is never confusable with a
//! delivery preset render).

use std::path::Path;

use crate::color::{
    color_filter_chain, verify_output_color_metadata, ColorCorrection, ColorSpaceKind, CreativeLut,
    ExpectedColorMetadata, ShotMatchTarget,
};
use crate::native::{
    MacMediaBackend, MacNativeMode, NativeRequestContext, NativeTimelineRenderRequest,
    NativeTimelineRenderResult,
};
use crate::probe::probe_with_toolchain;
use crate::process::{
    duration_scaled_timeout_with_toolchain, rec709_output_args, run_media_command, string_args,
    FINAL_RENDER_FLOOR, FINAL_RENDER_PER_SOURCE_SECOND, LOUDNESS_MEASURE_FLOOR,
    LOUDNESS_PER_SOURCE_SECOND,
};
use crate::reframe::{reframe_filter, ReframeAnchor};
use crate::toolchain::{self, MediaToolchain};
use crate::RenderError;

#[derive(Clone, Debug, PartialEq)]
pub enum FinishRenderRoute {
    Legacy,
    Shadow(NativeTimelineRenderResult),
    Native(NativeTimelineRenderResult),
}

/// Explicit final-render strangler. Native mode never falls back; shadow
/// preserves legacy authority while returning native comparison evidence.
pub fn render_locked_timeline(
    backend: Option<&dyn MacMediaBackend>,
    context: &NativeRequestContext,
    request: &NativeTimelineRenderRequest,
    legacy: impl FnOnce() -> Result<(), RenderError>,
) -> Result<FinishRenderRoute, RenderError> {
    match request.mode {
        MacNativeMode::Legacy => {
            legacy()?;
            Ok(FinishRenderRoute::Legacy)
        }
        MacNativeMode::Shadow => {
            legacy()?;
            let backend = backend.ok_or_else(|| {
                RenderError::Failed("native timeline backend unavailable for shadow receipt".into())
            })?;
            backend
                .render_timeline(context, request)
                .map(FinishRenderRoute::Shadow)
                .map_err(|error| RenderError::Failed(error.to_string()))
        }
        MacNativeMode::Native => {
            let backend = backend
                .ok_or_else(|| RenderError::Failed("native timeline backend unavailable".into()))?;
            backend
                .render_timeline(context, request)
                .map(FinishRenderRoute::Native)
                .map_err(|error| RenderError::Failed(error.to_string()))
        }
    }
}

pub fn render_to_preset(
    input: &Path,
    output: &Path,
    width: u32,
    height: u32,
) -> Result<(), RenderError> {
    if !input.is_file() {
        return Err(RenderError::Failed(format!(
            "input does not exist: {}",
            input.display()
        )));
    }
    if input == output {
        return Err(RenderError::OutputIsInput);
    }
    if width == 0 || height == 0 {
        return Err(RenderError::Failed(
            "output dimensions must be nonzero".into(),
        ));
    }
    let toolchain = toolchain::resolve()?;
    let (filter, rec709_output) = preset_video_filter(input, width, height, None, &toolchain)?;
    let audio_filter = measured_loudnorm_filter(input, &toolchain)?;
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args([
        "-vf",
        &filter,
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-preset",
        "veryfast",
        "-crf",
        "23",
        "-c:a",
        "aac",
        "-ar",
        "48000",
        "-af",
        &audio_filter,
        "-movflags",
        "+faststart",
    ]));
    if rec709_output {
        args.extend(rec709_output_args());
    }
    args.push(output.display().to_string());
    let timeout = duration_scaled_timeout_with_toolchain(
        input,
        &toolchain,
        FINAL_RENDER_FLOOR,
        FINAL_RENDER_PER_SOURCE_SECOND,
    );
    run_media_command(&toolchain.ffmpeg, args, timeout, RenderError::Failed)?;
    Ok(())
}

/// See [`preset_video_filter`]'s doc comment on `toolchain`.
pub(crate) fn measured_loudnorm_filter(
    input: &Path,
    toolchain: &MediaToolchain,
) -> Result<String, RenderError> {
    let mut args = string_args(["-hide_banner", "-nostats", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args([
        "-af",
        "highpass=f=80,loudnorm=I=-14:TP=-1:LRA=11:print_format=json",
        "-f",
        "null",
        "-",
    ]));
    let timeout = duration_scaled_timeout_with_toolchain(
        input,
        toolchain,
        LOUDNESS_MEASURE_FLOOR,
        LOUDNESS_PER_SOURCE_SECOND,
    );
    let measurement = run_media_command(&toolchain.ffmpeg, args, timeout, RenderError::Failed)?;
    let stderr = String::from_utf8_lossy(&measurement.stderr);
    let start = stderr.rfind('{').ok_or_else(|| {
        RenderError::Failed("FFmpeg loudnorm did not emit measurement JSON".into())
    })?;
    let end = stderr[start..].find('}').ok_or_else(|| {
        RenderError::Failed("FFmpeg loudnorm emitted incomplete measurement JSON".into())
    })? + start
        + 1;
    let values: serde_json::Value = serde_json::from_str(&stderr[start..end]).map_err(|error| {
        RenderError::Failed(format!("invalid FFmpeg loudnorm measurement: {error}"))
    })?;
    let value = |key: &str| {
        values
            .get(key)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| RenderError::Failed(format!("FFmpeg loudnorm is missing {key}")))
    };
    Ok(format!(
        "highpass=f=80,loudnorm=I=-14:TP=-1:LRA=11:measured_I={}:measured_LRA={}:measured_TP={}:measured_thresh={}:offset={}:linear=true:print_format=summary",
        value("input_i")?,
        value("input_lra")?,
        value("input_tp")?,
        value("input_thresh")?,
        value("target_offset")?,
    ))
}

/// `toolchain` is threaded in rather than re-resolved here so a caller that
/// already resolved one for this operation (§10.3: resolve once, reuse)
/// doesn't pay for another `ffmpeg -filters` capability probe.
pub(crate) fn preset_video_filter(
    input: &Path,
    width: u32,
    height: u32,
    reframe_anchors: Option<&[ReframeAnchor]>,
    toolchain: &MediaToolchain,
) -> Result<(String, bool), RenderError> {
    let metadata = probe_with_toolchain(input, toolchain)
        .map_err(|error| RenderError::Failed(error.to_string()))?;
    let resize = reframe_filter(&metadata, width, height, reframe_anchors)?;
    if metadata.is_hdr == Some(true) {
        if !toolchain.capabilities.has_zscale {
            return Err(RenderError::CapabilityMissing(
                "HDR delivery requires FFmpeg built with the zscale filter; install a zimg-enabled FFmpeg build".into(),
            ));
        }
        Ok((
            format!(
                "zscale=transfer=linear:npl=100,format=gbrpf32le,tonemap=tonemap=hable:desat=0,zscale=primaries=bt709:transfer=bt709:matrix=bt709,format=yuv420p,{resize}"
            ),
            true,
        ))
    } else {
        Ok((resize, false))
    }
}

/// Grouped arguments for [`render_master`] — the full color-managed
/// archival/master render, distinct from the delivery-preset
/// [`render_to_preset`] path.
pub struct MasterRenderRequest<'a> {
    pub input: &'a Path,
    pub output: &'a Path,
    /// `None` renders at the input's native resolution (no resize) — the
    /// expected shape for an archival master.
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub color_space: ColorSpaceKind,
    pub correction: ColorCorrection,
    pub shot_match: Option<ShotMatchTarget>,
    pub lut: Option<CreativeLut>,
}

/// Renders the software archival/master delivery path (plan §15.2 Export):
/// full color pipeline (input-space-to-SDR conversion, exposure/white-
/// balance correction, shot matching, an optional bounded creative LUT),
/// encoded with the software `prores_ks` codec rather than the hardware
/// `h264_videotoolbox` preview path or the fast `libx264` delivery-preset
/// path — so the master is always a distinct, separately verifiable
/// artifact. Returns the color metadata the render was expected to carry;
/// the caller is responsible for probing the output and confirming it
/// (`verify_output_color_metadata` runs internally and this function
/// returns an error on any mismatch, so a successful return already proves
/// verification passed).
pub fn render_master(
    request: MasterRenderRequest<'_>,
) -> Result<ExpectedColorMetadata, RenderError> {
    let MasterRenderRequest {
        input,
        output,
        width,
        height,
        color_space,
        correction,
        shot_match,
        lut,
    } = request;
    if !input.is_file() {
        return Err(RenderError::Failed(format!(
            "input does not exist: {}",
            input.display()
        )));
    }
    if input == output {
        return Err(RenderError::OutputIsInput);
    }
    let toolchain = toolchain::resolve()?;
    if !toolchain.capabilities.has_prores_ks {
        return Err(RenderError::CapabilityMissing(
            "software master rendering requires FFmpeg built with the prores_ks encoder".into(),
        ));
    }
    let metadata = probe_with_toolchain(input, &toolchain)
        .map_err(|error| RenderError::Failed(error.to_string()))?;
    let target_width = width
        .or(metadata.width)
        .ok_or_else(|| RenderError::Failed("master render input has no width".into()))?;
    let target_height = height
        .or(metadata.height)
        .ok_or_else(|| RenderError::Failed("master render input has no height".into()))?;
    // No reframe anchors for the master path: an archival master keeps the
    // full native frame rather than any platform-specific crop.
    let resize = reframe_filter(&metadata, target_width, target_height, None)?;
    let (color_filter, expected_metadata) = color_filter_chain(
        color_space,
        &correction,
        shot_match.as_ref(),
        lut.as_ref(),
        &toolchain,
    )?;
    let filter = format!("{color_filter},{resize}");
    let audio_filter = measured_loudnorm_filter(input, &toolchain)?;

    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args([
        "-vf",
        &filter,
        "-c:v",
        "prores_ks",
        "-profile:v",
        "3",
        "-pix_fmt",
        "yuv422p10le",
        "-c:a",
        "pcm_s24le",
        "-af",
        &audio_filter,
    ]));
    args.extend(rec709_output_args());
    args.push(output.display().to_string());
    let timeout = duration_scaled_timeout_with_toolchain(
        input,
        &toolchain,
        FINAL_RENDER_FLOOR,
        // Archival ProRes encoding is heavier than the veryfast delivery
        // preset path, so it gets a larger per-second budget.
        FINAL_RENDER_PER_SOURCE_SECOND.saturating_mul(2),
    );
    run_media_command(&toolchain.ffmpeg, args, timeout, RenderError::Failed)?;
    verify_output_color_metadata(output, &expected_metadata, &toolchain)?;
    Ok(expected_metadata)
}

/// Companion to [`render_master`]: writes a review contact sheet for the
/// rendered master (plan §15.2: "a review contact sheet so a human can see
/// the graded result across the timeline at a glance").
pub fn render_master_contact_sheet(
    master_output: &Path,
    contact_sheet_output: &Path,
    columns: u32,
    rows: u32,
) -> Result<(), RenderError> {
    let toolchain = toolchain::resolve()?;
    crate::color::render_contact_sheet(
        master_output,
        contact_sheet_output,
        columns,
        rows,
        &toolchain,
    )
}

#[cfg(test)]
mod route_tests {
    use super::*;
    use std::cell::Cell;
    use std::time::Duration;

    fn request(mode: MacNativeMode) -> NativeTimelineRenderRequest {
        serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "lockedCutSha256": "0".repeat(64),
            "graph": {"schemaVersion":1,"sourcePath":"/tmp/source.mov","duration":{"numerator":1,"denominator":1},"assets":{},"nodes":[]},
            "outputPath": "/tmp/output.mp4",
            "allowedRoots": ["/tmp"],
            "video": {"width":64,"height":64,"frameRateNum":30,"frameRateDen":1},
            "audio": {"sampleRate":48000,"channels":2},
            "mode": mode
        }))
        .unwrap()
    }

    fn context() -> NativeRequestContext {
        NativeRequestContext {
            request_id: "route".into(),
            timeout: Duration::from_secs(1),
        }
    }

    #[test]
    fn final_modes_never_hide_native_fallback() {
        let legacy_ran = Cell::new(false);
        assert_eq!(
            render_locked_timeline(None, &context(), &request(MacNativeMode::Legacy), || {
                legacy_ran.set(true);
                Ok(())
            })
            .unwrap(),
            FinishRenderRoute::Legacy
        );
        assert!(legacy_ran.get());

        legacy_ran.set(false);
        let shadow =
            render_locked_timeline(None, &context(), &request(MacNativeMode::Shadow), || {
                legacy_ran.set(true);
                Ok(())
            });
        assert!(shadow.is_err());
        assert!(legacy_ran.get());

        legacy_ran.set(false);
        let native =
            render_locked_timeline(None, &context(), &request(MacNativeMode::Native), || {
                legacy_ran.set(true);
                Ok(())
            });
        assert!(native.is_err());
        assert!(!legacy_ran.get());
    }
}
