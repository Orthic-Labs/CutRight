//! Hardware-accelerated rough/preview renders — segment trims and
//! concatenation for in-app scrubbing, not final delivery quality.

use std::fs;
use std::path::{Path, PathBuf};

use crate::native::{
    MacMediaBackend, MacNativeMode, NativePreviewRequest, NativeRenderArtifact,
    NativeRequestContext,
};
use crate::probe::probe_with_toolchain;
use crate::process::{
    rec709_output_args, run_media_command, scaled_timeout, string_args, PREVIEW_RENDER_FLOOR,
    PREVIEW_RENDER_PER_SOURCE_SECOND, SHORT_OP_TIMEOUT,
};
use crate::toolchain::{self, MediaToolchain};
use crate::{build_receipt, RenderError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderSegment {
    pub start_ms: i64,
    pub end_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRenderSegment {
    pub input_index: usize,
    pub start_ms: i64,
    pub end_ms: i64,
}

pub fn render_segments(
    input: &Path,
    segments: &[RenderSegment],
    output: &Path,
) -> Result<(), RenderError> {
    if segments.is_empty() {
        return Err(RenderError::NoSegments);
    }
    if input == output {
        return Err(RenderError::OutputIsInput);
    }
    for segment in segments {
        if segment.start_ms < 0 || segment.end_ms <= segment.start_ms {
            return Err(RenderError::InvalidSegment {
                start_ms: segment.start_ms,
                end_ms: segment.end_ms,
            });
        }
    }

    let toolchain = toolchain::resolve()?;
    let source_filter = source_video_filter(input, &toolchain)?;
    let mut filter = String::new();
    for (index, segment) in segments.iter().enumerate() {
        let start = segment.start_ms as f64 / 1_000.0;
        let end = segment.end_ms as f64 / 1_000.0;
        filter.push_str(&format!(
            "[0:v]{},trim=start={start:.3}:end={end:.3},setpts=PTS-STARTPTS[v{index}];",
            source_filter.filter
        ));
        filter.push_str(&format!(
            "[0:a]atrim=start={start:.3}:end={end:.3},asetpts=PTS-STARTPTS[a{index}];"
        ));
    }
    for index in 0..segments.len() {
        filter.push_str(&format!("[v{index}][a{index}]"));
    }
    filter.push_str(&format!("concat=n={}:v=1:a=1[outv][outa]", segments.len()));

    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args([
        "-filter_complex",
        &filter,
        "-map",
        "[outv]",
        "-map",
        "[outa]",
    ]));
    args.extend(
        preview_video_args(&toolchain)?
            .into_iter()
            .map(str::to_string),
    );
    args.extend(string_args(["-c:a", "aac"]));
    if source_filter.rec709_output {
        args.extend(rec709_output_args());
    }
    args.extend(string_args(["-movflags", "+faststart"]));
    args.push(output.display().to_string());
    let total_ms: i64 = segments
        .iter()
        .map(|segment| segment.end_ms - segment.start_ms)
        .sum();
    let timeout = scaled_timeout(
        total_ms,
        PREVIEW_RENDER_FLOOR,
        PREVIEW_RENDER_PER_SOURCE_SECOND,
    );
    run_media_command(&toolchain.ffmpeg, args, timeout, RenderError::Failed)?;
    Ok(())
}

/// Same as [`render_segments`], but also returns a [`video_core::StageReceipt`]
/// recording the resolved ffmpeg/ffprobe toolchain identity plus the input
/// and output content hashes (hardening plan §10.4). Additive: existing
/// callers of `render_segments` are unaffected.
pub fn render_segments_with_receipt(
    input: &Path,
    segments: &[RenderSegment],
    output: &Path,
) -> Result<video_core::StageReceipt, RenderError> {
    render_segments(input, segments, output)?;
    build_receipt(
        "render.segments",
        input,
        &serde_json::json!({
            "segments": segments
                .iter()
                .map(|segment| serde_json::json!({
                    "start_ms": segment.start_ms,
                    "end_ms": segment.end_ms,
                }))
                .collect::<Vec<_>>(),
        }),
        output,
    )
}

pub fn render_source_segments(
    inputs: &[PathBuf],
    segments: &[SourceRenderSegment],
    output: &Path,
) -> Result<(), RenderError> {
    if inputs.is_empty() || segments.is_empty() {
        return Err(RenderError::NoSegments);
    }
    if inputs.iter().any(|input| input == output) {
        return Err(RenderError::OutputIsInput);
    }
    for segment in segments {
        if segment.input_index >= inputs.len()
            || segment.start_ms < 0
            || segment.end_ms <= segment.start_ms
        {
            return Err(RenderError::InvalidSegment {
                start_ms: segment.start_ms,
                end_ms: segment.end_ms,
            });
        }
    }
    let toolchain = toolchain::resolve()?;
    let source_filters = inputs
        .iter()
        .map(|input| source_video_filter(input, &toolchain))
        .collect::<Result<Vec<_>, _>>()?;
    let mut filter = String::new();
    for (index, segment) in segments.iter().enumerate() {
        let start = segment.start_ms as f64 / 1_000.0;
        let end = segment.end_ms as f64 / 1_000.0;
        filter.push_str(&format!(
            "[{}:v]{},trim=start={start:.3}:end={end:.3},setpts=PTS-STARTPTS[v{index}];",
            segment.input_index, source_filters[segment.input_index].filter
        ));
        filter.push_str(&format!(
            "[{}:a]atrim=start={start:.3}:end={end:.3},asetpts=PTS-STARTPTS[a{index}];",
            segment.input_index
        ));
    }
    for index in 0..segments.len() {
        filter.push_str(&format!("[v{index}][a{index}]"));
    }
    filter.push_str(&format!("concat=n={}:v=1:a=1[outv][outa]", segments.len()));
    let rec709_output = source_filters.iter().any(|filter| filter.rec709_output);
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y"]);
    for input in inputs {
        args.push("-i".to_string());
        args.push(input.display().to_string());
    }
    args.extend(string_args([
        "-filter_complex",
        &filter,
        "-map",
        "[outv]",
        "-map",
        "[outa]",
    ]));
    args.extend(
        preview_video_args(&toolchain)?
            .into_iter()
            .map(str::to_string),
    );
    args.extend(string_args(["-c:a", "aac"]));
    if rec709_output {
        args.extend(rec709_output_args());
    }
    args.extend(string_args(["-movflags", "+faststart"]));
    args.push(output.display().to_string());
    let total_ms: i64 = segments
        .iter()
        .map(|segment| segment.end_ms - segment.start_ms)
        .sum();
    let timeout = scaled_timeout(
        total_ms,
        PREVIEW_RENDER_FLOOR,
        PREVIEW_RENDER_PER_SOURCE_SECOND,
    );
    run_media_command(&toolchain.ffmpeg, args, timeout, RenderError::Failed)?;
    Ok(())
}

pub fn render_boundary_probe(
    input: &Path,
    boundary_ms: i64,
    output: &Path,
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
    let toolchain = toolchain::resolve()?;
    let metadata = probe_with_toolchain(input, &toolchain)
        .map_err(|error| RenderError::Failed(error.to_string()))?;
    let start_ms = boundary_ms.saturating_sub(800).max(0);
    let mut end_ms = boundary_ms.saturating_add(800);
    if let Some(duration) = metadata.duration_ms {
        end_ms = end_ms.min(duration);
    }
    if boundary_ms <= start_ms || end_ms <= boundary_ms {
        return Err(RenderError::InvalidSegment { start_ms, end_ms });
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(RenderError::Start)?;
    }
    let start = start_ms as f64 / 1_000.0;
    let boundary = boundary_ms as f64 / 1_000.0;
    let end = end_ms as f64 / 1_000.0;
    let filter = format!(
        "[0:v]trim=start={start:.3}:end={boundary:.3},setpts=PTS-STARTPTS[v0];[0:a]atrim=start={start:.3}:end={boundary:.3},asetpts=PTS-STARTPTS[a0];[0:v]trim=start={boundary:.3}:end={end:.3},setpts=PTS-STARTPTS[v1];[0:a]atrim=start={boundary:.3}:end={end:.3},asetpts=PTS-STARTPTS[a1];[v0][a0][v1][a1]concat=n=2:v=1:a=1[outv][outa]"
    );
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args([
        "-filter_complex",
        &filter,
        "-map",
        "[outv]",
        "-map",
        "[outa]",
    ]));
    args.extend(
        preview_video_args(&toolchain)?
            .into_iter()
            .map(str::to_string),
    );
    args.extend(string_args(["-c:a", "aac", "-movflags", "+faststart"]));
    args.push(output.display().to_string());
    run_media_command(
        &toolchain.ffmpeg,
        args,
        SHORT_OP_TIMEOUT,
        RenderError::Failed,
    )?;
    if output.is_file() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            "ffmpeg exited successfully but did not produce the boundary probe output".into(),
        ))
    }
}

/// Explicit route for a native *preview frame* artifact. This is deliberately
/// separate from `render_segments`/`render_boundary_probe`, which produce
/// FFmpeg MP4 previews. `Shadow` preserves legacy output & asks callers to
/// compare the separately-addressed native frame artifact.
pub fn render_preview_frame_with_native_mode(
    mode: MacNativeMode,
    backend: Option<&dyn MacMediaBackend>,
    context: &NativeRequestContext,
    request: &NativePreviewRequest,
    legacy: impl FnOnce() -> Result<(), RenderError>,
) -> Result<Option<NativeRenderArtifact>, RenderError> {
    match mode {
        MacNativeMode::Legacy => {
            legacy()?;
            Ok(None)
        }
        MacNativeMode::Shadow => {
            legacy()?;
            let backend = backend.ok_or_else(|| {
                RenderError::Failed(
                    "native preview backend unavailable for shadow comparison".into(),
                )
            })?;
            Ok(Some(
                backend
                    .render_preview(context, request)
                    .map_err(native_preview_error)?,
            ))
        }
        MacNativeMode::Native => {
            let backend = backend
                .ok_or_else(|| RenderError::Failed("native preview backend unavailable".into()))?;
            Ok(Some(
                backend
                    .render_preview(context, request)
                    .map_err(native_preview_error)?,
            ))
        }
    }
}

fn native_preview_error(error: crate::native::NativeMediaError) -> RenderError {
    RenderError::Failed(error.to_string())
}

pub(crate) struct SourceVideoFilter {
    pub(crate) filter: String,
    pub(crate) rec709_output: bool,
}

/// See [`crate::final_render::preset_video_filter`]'s doc comment on
/// `toolchain`.
pub(crate) fn source_video_filter(
    input: &Path,
    toolchain: &MediaToolchain,
) -> Result<SourceVideoFilter, RenderError> {
    let metadata = probe_with_toolchain(input, toolchain)
        .map_err(|error| RenderError::Failed(error.to_string()))?;
    if metadata.is_hdr == Some(true) {
        let (filter, _) = crate::final_render::preset_video_filter(input, 1, 1, None, toolchain)?;
        let tone_map = filter
            .strip_suffix(",scale=1:1:force_original_aspect_ratio=increase,crop=1:1,setsar=1")
            .ok_or_else(|| RenderError::Failed("invalid HDR normalization filter".into()))?;
        Ok(SourceVideoFilter {
            filter: tone_map.to_string(),
            rec709_output: true,
        })
    } else {
        Ok(SourceVideoFilter {
            filter: "null".into(),
            rec709_output: false,
        })
    }
}

/// See [`crate::final_render::preset_video_filter`]'s doc comment on
/// `toolchain`.
pub(crate) fn preview_video_args(
    toolchain: &MediaToolchain,
) -> Result<Vec<&'static str>, RenderError> {
    if !toolchain.capabilities.has_h264_videotoolbox {
        return Err(RenderError::CapabilityMissing(
            "rough preview rendering requires h264_videotoolbox on macOS".into(),
        ));
    }
    Ok(vec!["-c:v", "h264_videotoolbox", "-b:v", "10M"])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn native_preview_shadow_requires_explicit_backend() {
        let context = NativeRequestContext {
            request_id: "preview-shadow".into(),
            timeout: std::time::Duration::from_secs(1),
        };
        let result = render_preview_frame_with_native_mode(
            MacNativeMode::Shadow,
            None,
            &context,
            &NativePreviewRequest {
                input_path: std::env::temp_dir().join("preview-input.png"),
                output_path: std::env::temp_dir().join("preview-output.png"),
                crop_x: None,
                crop_y: None,
                crop_width: None,
                crop_height: None,
                rotation_degrees: None,
                allowed_roots: vec![std::env::temp_dir()],
            },
            || Ok(()),
        );
        assert!(matches!(result, Err(RenderError::Failed(message)) if message.contains("shadow")));
    }

    #[test]
    fn boundary_probe_renders_a_short_av_edit() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cutright-boundary-test-{unique}"));
        fs::create_dir_all(&root).expect("create boundary test directory");
        let input = root.join("input.mp4");
        let output = root.join("probe.mp4");
        let generated = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=black:s=640x360:r=30",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=1000:sample_rate=48000",
                "-t",
                "3",
                "-c:v",
                "libx264",
                "-c:a",
                "aac",
            ])
            .arg(&input)
            .output()
            .expect("start boundary fixture ffmpeg");
        assert!(generated.status.success());
        render_boundary_probe(&input, 1_500, &output).expect("render boundary probe");
        let metadata = crate::probe::probe(&output).expect("probe rendered boundary");
        assert!(metadata.has_video && metadata.has_audio);
        assert!(metadata
            .duration_ms
            .is_some_and(|duration| (1_500..=1_700).contains(&duration)));
        fs::remove_dir_all(root).expect("remove boundary test directory");
    }

    #[test]
    fn hdr_preview_is_tone_mapped_and_tagged_rec709() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cutright-hdr-test-{unique}"));
        fs::create_dir_all(&root).expect("create HDR test directory");
        let input = root.join("input-hdr.mp4");
        let output = root.join("preview.mp4");
        let generated = Command::new(crate::ffmpeg_path().expect("resolve ffmpeg toolchain"))
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=s=320x180:r=30",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=1000:sample_rate=48000",
                "-t",
                "1",
                "-c:v",
                "hevc_videotoolbox",
                "-pix_fmt",
                "p010le",
                "-color_primaries",
                "bt2020",
                "-color_trc",
                "smpte2084",
                "-colorspace",
                "bt2020nc",
                "-bsf:v",
                "hevc_metadata=colour_primaries=9:transfer_characteristics=16:matrix_coefficients=9",
                "-c:a",
                "aac",
            ])
            .arg(&input)
            .output()
            .expect("start HDR fixture ffmpeg");
        assert!(
            generated.status.success(),
            "HDR fixture ffmpeg failed: {}",
            String::from_utf8_lossy(&generated.stderr)
        );
        assert_eq!(
            crate::probe::probe(&input)
                .expect("probe HDR fixture")
                .is_hdr,
            Some(true)
        );

        render_segments(
            &input,
            &[RenderSegment {
                start_ms: 0,
                end_ms: 900,
            }],
            &output,
        )
        .expect("tone-map HDR preview");

        let tags = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=color_transfer,color_primaries,color_space",
                "-of",
                "default=noprint_wrappers=1",
            ])
            .arg(&output)
            .output()
            .expect("probe HDR preview tags");
        assert!(tags.status.success());
        let tags = String::from_utf8(tags.stdout).expect("tags are UTF-8");
        assert!(tags.contains("color_transfer=bt709"), "{tags}");
        assert!(tags.contains("color_primaries=bt709"), "{tags}");
        assert!(tags.contains("color_space=bt709"), "{tags}");
        assert_eq!(
            crate::probe::probe(&output)
                .expect("probe HDR preview")
                .is_hdr,
            Some(false)
        );
        fs::remove_dir_all(root).expect("remove HDR test directory");
    }
}
