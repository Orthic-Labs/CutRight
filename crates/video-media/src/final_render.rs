//! Full-quality libx264 delivery renders.

use std::path::Path;

use crate::probe::probe_with_toolchain;
use crate::process::{
    duration_scaled_timeout_with_toolchain, rec709_output_args, run_media_command, string_args,
    FINAL_RENDER_FLOOR, FINAL_RENDER_PER_SOURCE_SECOND, LOUDNESS_MEASURE_FLOOR,
    LOUDNESS_PER_SOURCE_SECOND,
};
use crate::reframe::{reframe_filter, ReframeAnchor};
use crate::toolchain::{self, MediaToolchain};
use crate::RenderError;

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
