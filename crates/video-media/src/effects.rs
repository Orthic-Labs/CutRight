//! Generic still/motion preview rendering for the typed effect registry
//! (REV2 plan §15.3). Every registry effect — caption profile, lower third,
//! stat counter, quote card, CTA end card — renders through this ONE
//! ffmpeg lavfi card path; effects differ only in the parameters
//! ([`EffectCard`]) `video-project`'s registry passes in, never in a new
//! render code path. That is the "effects are data, not code paths"
//! requirement: adding a sixth effect never touches this file.
//!
//! A dedicated Remotion-backed path for animated profiles is deliberately
//! not implemented here yet (plan §15.3: "renderer" is named for a later
//! Remotion addition, pinned version + license re-verification first). This
//! module is the `renderer: "ffmpeg"` implementation the registry can use
//! today; `video_project::effects::EffectRenderer::Remotion` exists as a
//! schema-stable placeholder for that later work.

use std::path::Path;

use crate::process::{run_media_command, string_args, SHORT_OP_TIMEOUT};
use crate::toolchain::{self, MediaToolchain};
use crate::RenderError;

/// One card's geometry + label + accent, independent of which registry
/// effect produced it. `footprint_px` is `(x, y, width, height)` — the
/// highlighted band the effect occupies on a `width`x`height` canvas.
#[derive(Debug, Clone)]
pub struct EffectCard {
    pub width: u32,
    pub height: u32,
    pub footprint_px: (u32, u32, u32, u32),
    pub label: String,
    pub accent_rgb: (u8, u8, u8),
}

/// Render one still preview frame (a single PNG) for `card`.
pub fn render_effect_still(card: &EffectCard, output: &Path) -> Result<(), RenderError> {
    if card.width == 0 || card.height == 0 {
        return Err(RenderError::Failed(
            "effect canvas dimensions must be nonzero".into(),
        ));
    }
    let toolchain = toolchain::resolve()?;
    let filter = still_filter(card);
    let mut args = string_args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "lavfi",
        "-i",
    ]);
    args.push(format!("color=c=black:s={}x{}", card.width, card.height));
    args.extend(string_args(["-frames:v", "1", "-vf"]));
    args.push(filter);
    args.push(output.display().to_string());
    run_media_command(
        &toolchain.ffmpeg,
        args,
        SHORT_OP_TIMEOUT,
        RenderError::Failed,
    )?;
    Ok(())
}

/// Render one short motion preview clip (mp4) for `card`.
///
/// `reduced_motion`: when `true`, the card appears at full opacity from
/// frame zero (the graceful-degradation fallback for `prefers-reduced-
/// motion`-equivalent playback); when `false`, it fades in over 400ms — the
/// "restrained" motion vocabulary the plan's `motion_profile` field uses.
pub fn render_effect_motion(
    card: &EffectCard,
    duration_secs: f64,
    reduced_motion: bool,
    output: &Path,
) -> Result<(), RenderError> {
    if card.width == 0 || card.height == 0 {
        return Err(RenderError::Failed(
            "effect canvas dimensions must be nonzero".into(),
        ));
    }
    if duration_secs <= 0.0 {
        return Err(RenderError::Failed(
            "effect motion preview duration must be positive".into(),
        ));
    }
    let toolchain = toolchain::resolve()?;
    let mut filter = still_filter(card);
    if !reduced_motion {
        filter.push_str(",fade=t=in:st=0:d=0.4");
    }
    let mut args = string_args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "lavfi",
        "-i",
    ]);
    args.push(format!(
        "color=c=black:s={}x{}:d={:.3}",
        card.width, card.height, duration_secs
    ));
    args.extend(string_args(["-vf"]));
    args.push(filter);
    args.extend(string_args([
        "-r",
        "30",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-preset",
        "veryfast",
        "-crf",
        "28",
        "-movflags",
        "+faststart",
    ]));
    args.push(output.display().to_string());
    run_media_command(
        &toolchain.ffmpeg,
        args,
        SHORT_OP_TIMEOUT,
        RenderError::Failed,
    )?;
    Ok(())
}

/// The resolved toolchain's receipt identity string
/// (`"<version>:<content_hash>"`), so callers building a
/// [`video_core::StageReceipt`] don't need their own `toolchain::resolve`
/// call.
pub fn effect_render_toolchain_identity() -> Result<(String, String), RenderError> {
    let toolchain: MediaToolchain = toolchain::resolve()?;
    Ok(("ffmpeg".to_string(), toolchain.identity()))
}

/// Draws the effect's footprint as two nested `drawbox` fills: an outer
/// full-opacity accent border and an inner near-black content fill. This is
/// deliberately `drawbox`-only, never `drawtext` — this workspace's actual
/// caption text rendering already goes through the CoreText-backed
/// `caption-card` sidecar (`captions.rs::caption_card_worker`) specifically
/// because `drawtext` needs a `--enable-libfreetype` FFmpeg build that
/// isn't guaranteed (and isn't present in this build). The registry's
/// preview fixtures only need to prove geometry, motion timing, and
/// receipt binding, not typography, so they stay on the filter this
/// toolchain's FFmpeg is guaranteed to have. `card.label` is still carried
/// through to the render receipt's bound parameters (see
/// `video_project::effects::render_effect_preview`), so the effect's actual
/// text content remains provenance-tracked even though this raster doesn't
/// draw it.
fn still_filter(card: &EffectCard) -> String {
    let (x, y, w, h) = card.footprint_px;
    let (r, g, b) = card.accent_rgb;
    let border = 6u32.min(w / 2).min(h / 2);
    format!(
        "drawbox=x={x}:y={y}:w={w}:h={h}:color=0x{r:02x}{g:02x}{b:02x}@0.9:t=fill,\
         drawbox=x={inner_x}:y={inner_y}:w={inner_w}:h={inner_h}:color=black@0.55:t=fill",
        inner_x = x + border,
        inner_y = y + border,
        inner_w = w.saturating_sub(border * 2),
        inner_h = h.saturating_sub(border * 2),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn unique_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("cutright-effects-media-test-{label}-{unique}"));
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    fn sample_card() -> EffectCard {
        EffectCard {
            width: 640,
            height: 360,
            footprint_px: (40, 230, 400, 90),
            label: "CutRight: 'quoted' 50% off".into(),
            accent_rgb: (223, 100, 40),
        }
    }

    #[test]
    fn still_filter_never_references_drawtext() {
        // Regression guard for the reason documented on `still_filter`: this
        // must stay `drawbox`-only so it never depends on an
        // FFmpeg build's optional libfreetype/drawtext support.
        let filter = still_filter(&sample_card());
        assert!(!filter.contains("drawtext"));
        assert!(filter.contains("drawbox"));
    }

    #[test]
    fn renders_still_preview_png() {
        let dir = unique_dir("still");
        let output = dir.join("still.png");
        render_effect_still(&sample_card(), &output).expect("render still preview");
        let bytes = fs::read(&output).expect("read rendered still");
        assert!(!bytes.is_empty());
        assert_eq!(&bytes[0..8], b"\x89PNG\r\n\x1a\n");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn renders_motion_preview_with_and_without_reduced_motion() {
        let dir = unique_dir("motion");
        let animated = dir.join("motion.mp4");
        let reduced = dir.join("motion-reduced.mp4");
        render_effect_motion(&sample_card(), 1.0, false, &animated)
            .expect("render animated motion preview");
        render_effect_motion(&sample_card(), 1.0, true, &reduced)
            .expect("render reduced-motion preview");
        assert!(fs::metadata(&animated).expect("animated metadata").len() > 0);
        assert!(fs::metadata(&reduced).expect("reduced metadata").len() > 0);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_zero_dimensions_and_nonpositive_duration() {
        let dir = unique_dir("invalid");
        let mut card = sample_card();
        card.width = 0;
        let still_err = render_effect_still(&card, &dir.join("still.png"));
        assert!(still_err.is_err());

        let card = sample_card();
        let motion_err = render_effect_motion(&card, 0.0, false, &dir.join("motion.mp4"));
        assert!(motion_err.is_err());
        fs::remove_dir_all(&dir).ok();
    }
}
