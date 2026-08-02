//! Single-frame extraction and decision-evidence composite rendering.

use std::fs;
use std::path::{Path, PathBuf};

use crate::process::{run_media_command, string_args, SHORT_OP_TIMEOUT};
use crate::RenderError;

pub fn extract_frame(input: &Path, timestamp_ms: i64, output: &Path) -> Result<(), RenderError> {
    if !input.is_file() {
        return Err(RenderError::Failed(format!(
            "input does not exist: {}",
            input.display()
        )));
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(RenderError::Start)?;
    }
    let timestamp = format!("{:.3}", timestamp_ms.max(0) as f64 / 1_000.0);
    let ffmpeg = crate::ffmpeg_path()?;
    let mut args = string_args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-ss",
        &timestamp,
        "-i",
    ]);
    args.push(input.display().to_string());
    args.extend(string_args(["-frames:v", "1", "-q:v", "2"]));
    args.push(output.display().to_string());
    run_media_command(&ffmpeg, args, SHORT_OP_TIMEOUT, RenderError::Failed)?;
    if output.is_file() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            "ffmpeg exited successfully but did not produce the extracted frame".into(),
        ))
    }
}

pub fn compose_decision_evidence(
    frames: &[PathBuf],
    waveform: &Path,
    output: &Path,
) -> Result<(), RenderError> {
    if frames.len() != 3 || frames.iter().any(|frame| !frame.is_file()) || !waveform.is_file() {
        return Err(RenderError::Failed(
            "decision evidence requires three frames and one waveform".into(),
        ));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(RenderError::Start)?;
    }
    let ffmpeg = crate::ffmpeg_path()?;
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y"]);
    for frame in frames {
        args.push("-i".to_string());
        args.push(frame.display().to_string());
    }
    args.push("-i".to_string());
    args.push(waveform.display().to_string());
    args.extend(string_args([
        "-filter_complex",
        "[0:v]scale=400:225[a];[1:v]scale=400:225[b];[2:v]scale=400:225[c];[a][b][c]hstack=inputs=3[filmstrip];[3:v]scale=1200:180[wave];[filmstrip][wave]vstack=inputs=2",
        "-frames:v",
        "1",
    ]));
    args.push(output.display().to_string());
    run_media_command(&ffmpeg, args, SHORT_OP_TIMEOUT, RenderError::Failed)?;
    if output.is_file() {
        Ok(())
    } else {
        Err(RenderError::Failed(
            "ffmpeg exited successfully but did not produce the decision evidence image".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// End-to-end proof that a real render call through the new plumbing
    /// (§10.1 process runner + §10.3 resolved toolchain) still produces
    /// correct output: extract a single frame from a short fixture clip.
    #[test]
    fn extract_frame_still_succeeds_through_the_shared_process_runner() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cutright-extract-frame-test-{unique}"));
        fs::create_dir_all(&root).expect("create test directory");
        let input = root.join("input.mp4");
        let output = root.join("frame.jpg");
        let generated = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=red:s=320x180:r=30",
                "-t",
                "1",
                "-c:v",
                "libx264",
            ])
            .arg(&input)
            .output()
            .expect("start extract-frame fixture ffmpeg");
        assert!(generated.status.success());

        extract_frame(&input, 500, &output).expect("extract a frame");
        assert!(output.is_file());
        fs::remove_dir_all(root).expect("remove test directory");
    }
}
