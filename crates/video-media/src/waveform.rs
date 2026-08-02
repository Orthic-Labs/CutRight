//! Waveform image rendering (whole-source and range).

use std::path::Path;

use crate::process::{
    duration_scaled_timeout, run_media_command, scaled_timeout, string_args, SHORT_OP_TIMEOUT,
    WAVEFORM_PER_SOURCE_SECOND, WAVEFORM_RENDER_FLOOR,
};
use crate::RenderError;

pub fn render_waveform(input: &Path, output: &Path) -> Result<(), RenderError> {
    if !input.is_file() {
        return Err(RenderError::Failed(format!(
            "input does not exist: {}",
            input.display()
        )));
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(RenderError::Start)?;
    }
    let ffmpeg = crate::ffmpeg_path()?;
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args([
        "-filter_complex",
        "showwavespic=s=1200x240:colors=0x44D7B6",
        "-frames:v",
        "1",
    ]));
    args.push(output.display().to_string());
    let timeout = duration_scaled_timeout(input, WAVEFORM_RENDER_FLOOR, WAVEFORM_PER_SOURCE_SECOND);
    run_media_command(&ffmpeg, args, timeout, RenderError::Failed)?;
    if output.is_file() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            "ffmpeg exited successfully but did not produce the waveform image".into(),
        ))
    }
}

pub fn render_waveform_range(
    input: &Path,
    start_ms: i64,
    end_ms: i64,
    output: &Path,
) -> Result<(), RenderError> {
    if !input.is_file() {
        return Err(RenderError::Failed(format!(
            "input does not exist: {}",
            input.display()
        )));
    }
    if end_ms <= start_ms {
        return Err(RenderError::InvalidSegment { start_ms, end_ms });
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(RenderError::Start)?;
    }
    let start = start_ms as f64 / 1_000.0;
    let end = end_ms as f64 / 1_000.0;
    let filter = format!(
        "atrim=start={start:.3}:end={end:.3},asetpts=PTS-STARTPTS,showwavespic=s=1200x180:colors=0x44D7B6"
    );
    let ffmpeg = crate::ffmpeg_path()?;
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args(["-filter_complex", &filter, "-frames:v", "1"]));
    args.push(output.display().to_string());
    let timeout = scaled_timeout(
        end_ms - start_ms,
        SHORT_OP_TIMEOUT,
        WAVEFORM_PER_SOURCE_SECOND,
    );
    run_media_command(&ffmpeg, args, timeout, RenderError::Failed)?;
    if output.is_file() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            "ffmpeg exited successfully but did not produce the waveform range image".into(),
        ))
    }
}
