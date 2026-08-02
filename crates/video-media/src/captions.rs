//! SRT-driven caption card overlay rendering (delivery + subtitled presets).

use std::fs;
use std::path::{Path, PathBuf};

use crate::final_render::{measured_loudnorm_filter, preset_video_filter};
use crate::probe::probe_with_toolchain;
use crate::process::{
    duration_scaled_timeout_with_toolchain, rec709_output_args, run_caption_card_worker,
    run_media_command, string_args, FINAL_RENDER_FLOOR, FINAL_RENDER_PER_SOURCE_SECOND,
};
use crate::reframe::ReframeAnchor;
use crate::toolchain::{self, MediaToolchain};
use crate::{build_receipt_multi, RenderError};

#[derive(Debug, Clone)]
struct CaptionCue {
    start_seconds: f64,
    end_seconds: f64,
    text: String,
}

struct CaptionRenderOptions<'a> {
    width: u32,
    height: u32,
    vertical: bool,
    video_filter: &'a str,
    audio_filter: Option<&'a str>,
    rec709_output: bool,
}

pub fn render_subtitled(input: &Path, captions: &Path, output: &Path) -> Result<(), RenderError> {
    let toolchain = toolchain::resolve()?;
    let metadata = probe_with_toolchain(input, &toolchain)
        .map_err(|error| RenderError::Failed(error.to_string()))?;
    let width = metadata
        .width
        .ok_or_else(|| RenderError::Failed("caption input has no width".into()))?;
    let height = metadata
        .height
        .ok_or_else(|| RenderError::Failed("caption input has no height".into()))?;
    render_captioned(
        input,
        captions,
        output,
        &toolchain,
        CaptionRenderOptions {
            width,
            height,
            vertical: false,
            video_filter: "setsar=1",
            audio_filter: None,
            rec709_output: false,
        },
    )
}

pub fn render_preset_with_captions(
    input: &Path,
    captions: &Path,
    output: &Path,
    width: u32,
    height: u32,
    vertical: bool,
) -> Result<(), RenderError> {
    render_preset_with_captions_and_reframe(input, captions, output, width, height, vertical, None)
}

pub fn render_preset_with_captions_and_reframe(
    input: &Path,
    captions: &Path,
    output: &Path,
    width: u32,
    height: u32,
    vertical: bool,
    reframe_anchors: Option<&[ReframeAnchor]>,
) -> Result<(), RenderError> {
    if width == 0 || height == 0 {
        return Err(RenderError::Failed(
            "output dimensions must be nonzero".into(),
        ));
    }
    let toolchain = toolchain::resolve()?;
    let (filter, rec709_output) =
        preset_video_filter(input, width, height, reframe_anchors, &toolchain)?;
    let audio_filter = measured_loudnorm_filter(input, &toolchain)?;
    render_captioned(
        input,
        captions,
        output,
        &toolchain,
        CaptionRenderOptions {
            width,
            height,
            vertical,
            video_filter: &filter,
            audio_filter: Some(&audio_filter),
            rec709_output,
        },
    )
}

/// Same as [`render_preset_with_captions_and_reframe`], but also returns a
/// [`video_core::StageReceipt`] (hardening plan §10.4). Additive: existing
/// callers are unaffected.
#[allow(clippy::too_many_arguments)]
pub fn render_preset_with_captions_and_reframe_with_receipt(
    input: &Path,
    captions: &Path,
    output: &Path,
    width: u32,
    height: u32,
    vertical: bool,
    reframe_anchors: Option<&[ReframeAnchor]>,
) -> Result<video_core::StageReceipt, RenderError> {
    render_preset_with_captions_and_reframe(
        input,
        captions,
        output,
        width,
        height,
        vertical,
        reframe_anchors,
    )?;
    build_receipt_multi(
        "render.finish_captioned",
        &[input, captions],
        &serde_json::json!({
            "width": width,
            "height": height,
            "vertical": vertical,
            "reframe_anchor_count": reframe_anchors.map(<[_]>::len).unwrap_or(0),
        }),
        output,
    )
}

fn render_captioned(
    input: &Path,
    captions: &Path,
    output: &Path,
    toolchain: &MediaToolchain,
    options: CaptionRenderOptions<'_>,
) -> Result<(), RenderError> {
    if !input.is_file() {
        return Err(RenderError::Failed(format!(
            "input does not exist: {}",
            input.display()
        )));
    }
    if !captions.is_file() {
        return Err(RenderError::Failed(format!(
            "captions do not exist: {}",
            captions.display()
        )));
    }
    if input == output {
        return Err(RenderError::OutputIsInput);
    }
    let cues = read_srt(captions)?;
    let cards = render_caption_cards(
        &cues,
        output,
        options.width,
        options.height,
        options.vertical,
    )?;
    let mut filter = format!("[0:v]{}[v0]", options.video_filter);
    for (index, cue) in cues.iter().enumerate() {
        let previous = format!("[v{index}]");
        let next = format!("[v{}]", index + 1);
        filter.push_str(&format!(
            ";{previous}[{}:v]overlay=0:0:enable='between(t,{:.3},{:.3})'{next}",
            index + 1,
            cue.start_seconds,
            cue.end_seconds
        ));
    }
    let last = format!("[v{}]", cues.len());
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    for card in &cards {
        args.extend(string_args(["-loop", "1", "-framerate", "30", "-i"]));
        args.push(card.display().to_string());
    }
    args.extend(string_args([
        "-filter_complex",
        &filter,
        "-map",
        &last,
        "-map",
        "0:a?",
    ]));
    args.extend(string_args([
        "-c:v", "libx264", "-pix_fmt", "yuv420p", "-preset", "slow", "-crf", "18", "-c:a", "aac",
        "-ar", "48000",
    ]));
    if let Some(audio_filter) = options.audio_filter {
        args.extend(string_args(["-af", audio_filter]));
    }
    if options.rec709_output {
        args.extend(rec709_output_args());
    }
    args.extend(string_args(["-shortest", "-movflags", "+faststart"]));
    args.push(output.display().to_string());
    let timeout = duration_scaled_timeout_with_toolchain(
        input,
        toolchain,
        FINAL_RENDER_FLOOR,
        FINAL_RENDER_PER_SOURCE_SECOND,
    );
    run_media_command(&toolchain.ffmpeg, args, timeout, RenderError::Failed)?;
    if output.is_file() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            "ffmpeg exited successfully but did not produce the captioned output".into(),
        ))
    }
}

fn read_srt(path: &Path) -> Result<Vec<CaptionCue>, RenderError> {
    let source = fs::read_to_string(path).map_err(RenderError::CaptionStart)?;
    source
        .split("\n\n")
        .filter(|chunk| !chunk.trim().is_empty())
        .map(|chunk| {
            let lines = chunk.lines().collect::<Vec<_>>();
            let timing = lines.get(1).ok_or_else(|| {
                RenderError::CaptionFailed("caption cue is missing timing".into())
            })?;
            let (start, end) = timing.split_once(" --> ").ok_or_else(|| {
                RenderError::CaptionFailed("caption cue has invalid timing".into())
            })?;
            let start_seconds = parse_srt_timestamp(start)?;
            let end_seconds = parse_srt_timestamp(end)?;
            let text = lines.get(2..).unwrap_or_default().join("\n");
            if end_seconds <= start_seconds || text.trim().is_empty() {
                return Err(RenderError::CaptionFailed(
                    "caption cue has invalid range or text".into(),
                ));
            }
            Ok(CaptionCue {
                start_seconds,
                end_seconds,
                text,
            })
        })
        .collect()
}

fn parse_srt_timestamp(value: &str) -> Result<f64, RenderError> {
    let normalized = value.replace(',', ".");
    let parts = normalized.split(':').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(RenderError::CaptionFailed(
            "caption timestamp has invalid format".into(),
        ));
    }
    let hours = parts[0]
        .parse::<f64>()
        .map_err(|_| RenderError::CaptionFailed("caption timestamp has invalid hours".into()))?;
    let minutes = parts[1]
        .parse::<f64>()
        .map_err(|_| RenderError::CaptionFailed("caption timestamp has invalid minutes".into()))?;
    let seconds = parts[2]
        .parse::<f64>()
        .map_err(|_| RenderError::CaptionFailed("caption timestamp has invalid seconds".into()))?;
    Ok(hours * 3_600.0 + minutes * 60.0 + seconds)
}

fn render_caption_cards(
    cues: &[CaptionCue],
    output: &Path,
    width: u32,
    height: u32,
    vertical: bool,
) -> Result<Vec<PathBuf>, RenderError> {
    let parent = output.parent().ok_or_else(|| {
        RenderError::CaptionFailed("caption output has no parent directory".into())
    })?;
    let cards_dir = parent.join(".cutright-caption-cards");
    fs::create_dir_all(&cards_dir).map_err(RenderError::CaptionStart)?;
    let worker = caption_card_worker()?;
    cues.iter()
        .enumerate()
        .map(|(index, cue)| {
            let card = cards_dir.join(format!("{:04}.png", index + 1));
            let request = serde_json::json!({
                "output_path": card,
                "width": width,
                "height": height,
                "text": cue.text,
                "vertical": vertical,
            });
            run_caption_card_worker(&worker, &request, &card)?;
            Ok(card)
        })
        .collect()
}

/// Materialize the embedded caption-card sidecar (hardening plan §10.2) at a
/// path addressed by the content hash of its embedded bytes, not by crate
/// version — a worker source edit with no crate version bump lands at a new
/// path instead of silently reusing a stale binary, and the on-disk bytes
/// are re-verified against that hash before every reuse.
fn caption_card_worker() -> Result<PathBuf, RenderError> {
    Ok(video_core::materialize_worker(
        include_bytes!(env!("CUTRIGHT_CAPTION_CARD")),
        "caption-card",
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn rendered_reframe_follows_timeline_anchors() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cutright-reframe-test-{unique}"));
        fs::create_dir_all(&root).expect("create reframe test directory");
        let input = root.join("input.mp4");
        let captions = root.join("captions.srt");
        let output = root.join("output.mp4");
        let generated = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=red:s=320x360:r=30",
                "-f",
                "lavfi",
                "-i",
                "color=blue:s=320x360:r=30",
                "-filter_complex",
                "[0:v][1:v]hstack",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=1000:sample_rate=48000",
                "-t",
                "2",
                "-c:v",
                "libx264",
                "-c:a",
                "aac",
            ])
            .arg(&input)
            .output()
            .expect("start reframe fixture ffmpeg");
        assert!(generated.status.success());
        fs::write(&captions, "").expect("write empty captions");
        let anchors = [
            ReframeAnchor {
                output_start_ms: 0,
                center_x: 0.25,
                center_y: 0.5,
            },
            ReframeAnchor {
                output_start_ms: 1_000,
                center_x: 0.75,
                center_y: 0.5,
            },
        ];
        render_preset_with_captions_and_reframe(
            &input,
            &captions,
            &output,
            360,
            640,
            true,
            Some(&anchors),
        )
        .expect("render reframed preset");
        let luminance = |time: &str| {
            let frame = Command::new("ffmpeg")
                .args(["-hide_banner", "-loglevel", "error", "-ss", time, "-i"])
                .arg(&output)
                .args([
                    "-frames:v",
                    "1",
                    "-vf",
                    "crop=10:10:0:0,signalstats,metadata=print:file=-",
                    "-f",
                    "null",
                    "-",
                ])
                .output()
                .expect("start reframe luminance ffmpeg");
            assert!(frame.status.success());
            String::from_utf8(frame.stdout)
                .expect("frame luminance is UTF-8")
                .lines()
                .find_map(|line| {
                    line.strip_prefix("lavfi.signalstats.YAVG=")
                        .map(str::parse::<f64>)
                })
                .expect("frame luminance is present")
                .expect("frame luminance is numeric")
        };
        assert!(luminance("0.25") > luminance("1.25") + 20.0);
        fs::remove_dir_all(root).expect("remove reframe test directory");
    }

    #[test]
    fn captioned_preset_shows_each_cue_only_during_its_interval() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cutright-caption-test-{unique}"));
        fs::create_dir_all(&root).expect("create test directory");
        let input = root.join("input.mp4");
        let captions = root.join("captions.srt");
        let output = root.join("output.mp4");
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
            .expect("start ffmpeg fixture");
        assert!(
            generated.status.success(),
            "fixture ffmpeg failed: {}",
            String::from_utf8_lossy(&generated.stderr)
        );
        fs::write(
            &captions,
            "1\n00:00:00,500 --> 00:00:01,000\nFIRST CUE\n\n2\n00:00:02,000 --> 00:00:02,500\nSECOND CUE\n",
        )
        .expect("write captions");
        render_preset_with_captions(&input, &captions, &output, 640, 360, false)
            .expect("render captioned preset");
        let frame_luminance = |time: &str| {
            let frame = Command::new("ffmpeg")
                .args(["-hide_banner", "-loglevel", "error", "-ss", time, "-i"])
                .arg(&output)
                .args([
                    "-frames:v",
                    "1",
                    "-vf",
                    "signalstats,metadata=print:file=-",
                    "-f",
                    "null",
                    "-",
                ])
                .output()
                .expect("start frame luminance ffmpeg");
            assert!(frame.status.success());
            String::from_utf8(frame.stdout)
                .expect("frame luminance is UTF-8")
                .lines()
                .find_map(|line| {
                    line.strip_prefix("lavfi.signalstats.YAVG=")
                        .map(str::parse::<f64>)
                })
                .expect("frame luminance is present")
                .expect("frame luminance is numeric")
        };
        let before = frame_luminance("0.25");
        let first_cue = frame_luminance("0.75");
        let gap = frame_luminance("1.50");
        let second_cue = frame_luminance("2.25");
        let after = frame_luminance("2.75");
        assert!(before < 17.0 && gap < 17.0 && after < 17.0);
        assert!(
            first_cue > before + 0.1 && second_cue > before + 0.1,
            "luminance before={before}, first={first_cue}, gap={gap}, second={second_cue}, after={after}"
        );
        fs::remove_dir_all(root).expect("remove test directory");
    }
}
