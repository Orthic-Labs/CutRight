//! Still/motion preview rendering for the typed effect registry (REV2 plan
//! §15.3), across every renderer `video_project::effects::EffectRenderer`
//! names:
//!
//! - `Ffmpeg` — the generic drawbox lavfi card path below
//!   ([`render_effect_still`]/[`render_effect_motion`]/[`EffectCard`]): the
//!   fast path, kept working for any future effect that just needs a
//!   labeled box, not real typography.
//! - `Ass` — [`render_effect_ass_preview`]: real libass-rendered karaoke
//!   text (a real `.ass` subtitle document burned in via ffmpeg's
//!   `subtitles` filter), the fast deterministic renderer for fixed
//!   karaoke/phrase captions per
//!   `skills/content-video-editor/workflows/finish.md` ("ASS for fast fixed
//!   karaoke/phrase captions; Remotion for branded kinetic; HyperFrames for
//!   bespoke type"). Requires a libass-enabled ffmpeg build
//!   (`--enable-libass`); [`ass_subtitles_toolchain_status`] probes this the
//!   same way `crates/video-cli/src/doctor.rs`'s existing
//!   `render.caption_renderer.listed` check does, and
//!   [`render_effect_ass_preview`] fails loudly with that same evidence
//!   rather than silently falling back to `Ffmpeg` when libass is absent.
//! - `Remotion` — [`render_effect_remotion_preview`]: a real Node/React
//!   render through the `apps/effects` Remotion package (branded kinetic
//!   motion), invoked through `video_core::process_runner` — never a bare
//!   spawn — with a duration-scaled timeout and an explicit environment
//!   allow-list. [`remotion_toolchain_status`] performs the same
//!   honest-missing check.
//! - `HyperFrames` has no implementation or dependency anywhere in this
//!   workspace; `video_project::effects::EffectRenderer::HyperFrames`
//!   fails loudly by design (see that module) rather than being folded into
//!   `Remotion` silently.
//!
//! Every renderer is a data value on a registry entry, not a branch a new
//! effect has to add — adding a sixth effect means adding one JSON entry,
//! never a new match arm in `video_project::effects::render_effect_preview`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;
use video_core::process_runner::TempFileGuard;

use crate::process::{
    media_env_allow, run_media_command, scaled_timeout, string_args, SHORT_OP_TIMEOUT,
};
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

// ---------------------------------------------------------------------
// Shared outcome shape for the two external-renderer paths (Ass, Remotion)
// ---------------------------------------------------------------------

/// What [`render_effect_ass_preview`]/[`render_effect_remotion_preview`]
/// produced: the still preview path, the (motion, motion-reduced) pair when
/// motion was requested, and a receipt-ready toolchain identity string.
/// Deliberately the same shape for both renderers so
/// `video_project::effects::render_effect_preview` can write one receipt
/// regardless of which renderer produced the outputs.
pub struct ExternalEffectPreviewOutcome {
    pub still_path: PathBuf,
    pub motion_paths: Option<(PathBuf, PathBuf)>,
    pub tool_identity: String,
}

// ---------------------------------------------------------------------
// `Ass` renderer: real libass-rendered karaoke text via ffmpeg's
// `subtitles` filter. Fast, deterministic, no browser — the renderer
// finish.md names for fixed karaoke/phrase captions.
// ---------------------------------------------------------------------

/// Fixed demo phrase the `caption.bold-karaoke.v1` preview renders: the
/// registry entry's `props_schema` only carries `highlight_color` and
/// `emphasis_scale` (no caption text field — a real per-project caption
/// document is a `CaptionDocument`, not registry props), so the preview
/// proves the karaoke-sweep mechanism and typography on fixed sample text,
/// same convention the ffmpeg drawbox path used (falling back to the
/// effect_id as a label when no text prop exists).
const KARAOKE_DEMO_WORDS: &[&str] = &["CutRight", "bold", "karaoke", "caption"];

/// ASS override colors are `&HAABBGGRR&` (alpha, then BGR — the reverse of
/// CSS RGB order); alpha `00` is fully opaque.
fn ass_bgr_color((r, g, b): (u8, u8, u8)) -> String {
    format!("&H00{b:02X}{g:02X}{r:02X}&")
}

fn parse_hex_rgb(value: &str) -> Option<(u8, u8, u8)> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

/// Build one `.ass` document for the karaoke demo phrase.
///
/// `animated`: when `true`, each word carries a `\k<centiseconds>` karaoke
/// timing tag so libass sweeps the highlight color across the line over
/// `duration_secs` (the real per-word karaoke behavior); when `false`
/// (the reduced-motion/still variant), the whole line is statically
/// colored `highlight_color` from frame zero — the
/// `ReducedMotionBehavior::StaticFallback` this registry entry declares
/// ("All cue words render at full emphasis simultaneously").
fn build_ass_document(highlight_rgb: (u8, u8, u8), emphasis_scale: f64, animated: bool) -> String {
    let font_size = (44.0 * emphasis_scale.clamp(1.0, 2.0)).round() as i64;
    let base_color = ass_bgr_color((245, 240, 232)); // off-white base text
    let highlight_color = ass_bgr_color(highlight_rgb);

    let text = if animated {
        let per_word_cs = 45; // 0.45s per word x 4 words = 1.8s sweep, comfortably inside the 1.5s+ preview.
        KARAOKE_DEMO_WORDS
            .iter()
            .map(|word| format!("{{\\k{per_word_cs}}}{word}"))
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        format!("{{\\c{highlight_color}}}{}", KARAOKE_DEMO_WORDS.join(" "))
    };

    format!(
        "[Script Info]\n\
         ScriptType: v4.00+\n\
         PlayResX: 1280\n\
         PlayResY: 720\n\
         ScaledBorderAndShadow: yes\n\
         \n\
         [V4+ Styles]\n\
         Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n\
         Style: Base,Arial,{font_size},{base_color},{highlight_color},&H00000000&,&H80000000&,-1,0,1,2,0,2,60,60,80,1\n\
         \n\
         [Events]\n\
         Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n\
         Dialogue: 0,0:00:00.00,0:00:05.00,Base,,0,0,0,,{text}\n"
    )
}

/// Escape a filesystem path for use as an ffmpeg filter option value
/// (`subtitles=filename='<escaped>'`): backslashes, single quotes, and
/// colons all need escaping inside ffmpeg's filtergraph mini-language, and
/// a colon otherwise terminates the filter's key=value list.
fn escape_ffmpeg_filter_path(path: &Path) -> String {
    let raw = path.display().to_string();
    let mut escaped = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if matches!(ch, '\\' | '\'' | ':') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

/// Probe whether the resolved ffmpeg build has libass support (the
/// `subtitles` filter), the same evidence
/// `crates/video-cli/src/doctor.rs::check_caption_renderer` already reports
/// under `videoctl doctor --profile render`. Returns the resolved toolchain
/// on success so callers don't re-resolve it; on failure names exactly what
/// was checked and how to fix it — never a bare "unavailable".
pub fn ass_subtitles_toolchain_status() -> Result<MediaToolchain, RenderError> {
    let toolchain = toolchain::resolve()?;
    let outcome = run_media_command(
        &toolchain.ffmpeg,
        string_args(["-hide_banner", "-filters"]),
        SHORT_OP_TIMEOUT,
        RenderError::Failed,
    )?;
    let listing = String::from_utf8_lossy(&outcome.stdout);
    let has_subtitles = listing
        .lines()
        .any(|line| line.split_whitespace().any(|token| token == "subtitles"));
    if has_subtitles {
        Ok(toolchain)
    } else {
        Err(RenderError::RendererUnavailable(format!(
            "ass renderer requires a libass-enabled ffmpeg build (checked `{} -hide_banner \
             -filters` for the `subtitles` filter; not present in the resolved toolchain at \
             {} version {}). Install an ffmpeg build with libass (--enable-libass) for burned-in \
             captions.",
            toolchain.ffmpeg.display(),
            toolchain.ffmpeg.display(),
            toolchain.version,
        )))
    }
}

/// Render `caption.bold-karaoke.v1`'s still + (when `needs_motion`) motion
/// preview pair through libass, failing loudly (never falling back to the
/// `ffmpeg` drawbox renderer) when libass is unavailable.
pub fn render_effect_ass_preview(
    props: &Value,
    needs_motion: bool,
    duration_secs: f64,
    output_dir: &Path,
) -> Result<ExternalEffectPreviewOutcome, RenderError> {
    if duration_secs <= 0.0 {
        return Err(RenderError::Failed(
            "ass effect preview duration must be positive".into(),
        ));
    }
    let toolchain = ass_subtitles_toolchain_status()?;

    let highlight_rgb = props
        .get("highlight_color")
        .and_then(Value::as_str)
        .and_then(parse_hex_rgb)
        .unwrap_or((255, 194, 75));
    let emphasis_scale = props
        .get("emphasis_scale")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);

    std::fs::create_dir_all(output_dir)?;
    let reduced_ass_path = output_dir.join("caption-reduced.ass");
    std::fs::write(
        &reduced_ass_path,
        build_ass_document(highlight_rgb, emphasis_scale, false),
    )?;

    let timeout = scaled_timeout(
        (duration_secs * 1000.0) as i64,
        Duration::from_secs(60),
        Duration::from_secs(5),
    );

    // Still: the fully revealed (reduced/static) state, one frame — matches
    // the ffmpeg drawbox path's convention of always showing the finished
    // composite in the still preview.
    let still_path = output_dir.join("still.png");
    render_ass_still(&toolchain, &reduced_ass_path, &still_path, timeout)?;

    let motion_paths = if needs_motion {
        let animated_ass_path = output_dir.join("caption.ass");
        std::fs::write(
            &animated_ass_path,
            build_ass_document(highlight_rgb, emphasis_scale, true),
        )?;
        let motion_path = output_dir.join("motion.mp4");
        render_ass_motion(
            &toolchain,
            &animated_ass_path,
            duration_secs,
            &motion_path,
            timeout,
        )?;
        let reduced_path = output_dir.join("motion-reduced.mp4");
        render_ass_motion(
            &toolchain,
            &reduced_ass_path,
            duration_secs,
            &reduced_path,
            timeout,
        )?;
        Some((motion_path, reduced_path))
    } else {
        None
    };

    Ok(ExternalEffectPreviewOutcome {
        still_path,
        motion_paths,
        tool_identity: format!("ffmpeg+libass:{}", toolchain.identity()),
    })
}

fn render_ass_still(
    toolchain: &MediaToolchain,
    ass_path: &Path,
    output: &Path,
    timeout: Duration,
) -> Result<(), RenderError> {
    let filter = format!(
        "subtitles=filename='{}'",
        escape_ffmpeg_filter_path(ass_path)
    );
    let mut args = string_args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "lavfi",
        "-i",
    ]);
    args.push("color=c=black:s=1280x720".to_string());
    args.extend(string_args(["-frames:v", "1", "-vf"]));
    args.push(filter);
    args.push(output.display().to_string());
    run_media_command(&toolchain.ffmpeg, args, timeout, RenderError::Failed)?;
    Ok(())
}

fn render_ass_motion(
    toolchain: &MediaToolchain,
    ass_path: &Path,
    duration_secs: f64,
    output: &Path,
    timeout: Duration,
) -> Result<(), RenderError> {
    let filter = format!(
        "subtitles=filename='{}'",
        escape_ffmpeg_filter_path(ass_path)
    );
    let mut args = string_args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "lavfi",
        "-i",
    ]);
    args.push(format!("color=c=black:s=1280x720:d={duration_secs:.3}"));
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
    run_media_command(&toolchain.ffmpeg, args, timeout, RenderError::Failed)?;
    Ok(())
}

// ---------------------------------------------------------------------
// `Remotion` renderer: real Node/React render through apps/effects,
// invoked via video_core::process_runner — never a bare spawn.
// ---------------------------------------------------------------------

/// Exact Remotion version pinned in `apps/effects/package.json`'s
/// `dependencies.remotion`. Kept in sync by hand (a test in this module
/// reads the package.json and asserts they match) rather than parsed at
/// build time, since this crate has no JSON-in-build-script dependency
/// today and adding one purely for a version string is heavier than a
/// checked assertion.
const REMOTION_PINNED_VERSION: &str = "4.0.503";

/// Registry `effect_id` -> Remotion composition id (mirrors
/// `apps/effects/src/schemas.ts::EFFECT_ID_TO_COMPOSITION_ID` and
/// `apps/effects/scripts/render.mjs::effectIdToCompositionId`): Remotion
/// composition ids reject `.`.
fn effect_id_to_composition_id(effect_id: &str) -> String {
    effect_id.replace('.', "-")
}

/// Resolve the `apps/effects` package root relative to this crate's own
/// manifest directory (never an absolute developer path; §9.3), overridable
/// via `CUTRIGHT_EFFECTS_PACKAGE` for out-of-tree installs.
fn effects_package_root() -> PathBuf {
    if let Some(path) = std::env::var_os("CUTRIGHT_EFFECTS_PACKAGE") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("apps/effects")
}

/// Resolve the Node executable: `CUTRIGHT_NODE` override (must exist), else
/// `node` on `PATH`. Mirrors `video_providers::whisperx::discover_python`'s
/// discovery shape.
fn discover_node() -> Result<PathBuf, RenderError> {
    if let Some(path) = std::env::var_os("CUTRIGHT_NODE") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(RenderError::RendererUnavailable(format!(
            "CUTRIGHT_NODE={} does not exist",
            path.display()
        )));
    }
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join("node");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(RenderError::RendererUnavailable(
        "no Node executable found: set CUTRIGHT_NODE or put `node` on PATH".into(),
    ))
}

/// Honest missing/remediation probe for the Remotion toolchain: Node
/// executable, the `apps/effects` package root, its installed
/// `node_modules`, and the render CLI script — in that order, naming the
/// first thing missing rather than a generic failure. Mirrors
/// `ass_subtitles_toolchain_status`'s shape; `crates/video-cli/src/doctor.rs`
/// reuses this same discovery for `videoctl doctor --profile render`.
pub fn remotion_toolchain_status() -> Result<(PathBuf, PathBuf), RenderError> {
    let node = discover_node()?;
    let package_root = effects_package_root();
    if !package_root.is_dir() {
        return Err(RenderError::RendererUnavailable(format!(
            "apps/effects package root not found at {} (set CUTRIGHT_EFFECTS_PACKAGE)",
            package_root.display()
        )));
    }
    let node_modules = package_root.join("node_modules");
    if !node_modules.is_dir() {
        return Err(RenderError::RendererUnavailable(format!(
            "apps/effects dependencies are not installed ({} does not exist); run \
             `pnpm --dir apps/effects install`",
            node_modules.display()
        )));
    }
    let render_script = package_root.join("scripts/render.mjs");
    if !render_script.is_file() {
        return Err(RenderError::RendererUnavailable(format!(
            "apps/effects render script not found at {}",
            render_script.display()
        )));
    }
    Ok((node, render_script))
}

/// Render one Remotion-backed registry effect's still + (when
/// `needs_motion`) motion preview pair, by shelling out to
/// `apps/effects/scripts/render.mjs preview` through
/// `video_core::process_runner`: a bounded, duration-scaled timeout, an
/// explicit environment allow-list (`media_env_allow`: `PATH`, `HOME`,
/// `TMPDIR` — `HOME` matters here because it's where Remotion's Chrome
/// Headless Shell cache lives), and byte-capped stdout/stderr. Props are
/// assumed already validated against the registry entry's `props_schema` by
/// the caller (`video_project::effects::render_effect_preview` validates
/// before matching on `renderer`) — this function never receives
/// caller-controlled props Node hasn't been vetted to see.
pub fn render_effect_remotion_preview(
    effect_id: &str,
    props: &Value,
    needs_motion: bool,
    duration_secs: f64,
    output_dir: &Path,
) -> Result<ExternalEffectPreviewOutcome, RenderError> {
    if duration_secs <= 0.0 {
        return Err(RenderError::Failed(
            "remotion effect preview duration must be positive".into(),
        ));
    }
    let (node, render_script) = remotion_toolchain_status()?;
    let package_root = effects_package_root();

    std::fs::create_dir_all(output_dir)?;
    let props_guard = TempFileGuard::new(
        &format!(
            "cutright-remotion-props-{}",
            blake3::hash(effect_id.as_bytes()).to_hex()
        ),
        ".json",
    );
    std::fs::write(&props_guard.path, serde_json::to_vec(props)?)?;

    let args = vec![
        render_script.display().to_string(),
        "preview".to_string(),
        "--composition".to_string(),
        effect_id_to_composition_id(effect_id),
        "--props-file".to_string(),
        props_guard.path.display().to_string(),
        "--output-dir".to_string(),
        output_dir.display().to_string(),
        "--motion".to_string(),
        (if needs_motion { "true" } else { "false" }).to_string(),
        "--duration".to_string(),
        format!("{duration_secs:.3}"),
    ];

    // Cold Node/webpack bundle + a first-ever Chrome Headless Shell
    // download can legitimately take minutes; generous floor + per-output-
    // second budget, matching this crate's other render-class timeouts
    // (`process.rs`'s `FINAL_RENDER_FLOOR` is 10 minutes for full delivery
    // renders — this can involve a one-time ~100MB browser download on top
    // of that, so the floor here is larger).
    let timeout = scaled_timeout(
        (duration_secs * 1000.0) as i64,
        Duration::from_secs(10 * 60),
        Duration::from_secs(30),
    );

    let mut spec_env = media_env_allow();
    // Remotion/Puppeteer also check these; harmless to pass through when
    // set, and several CI/sandboxed environments rely on one of them to
    // avoid a sandboxed-Chrome launch failure.
    for key in ["CI", "REMOTION_CHROME_MODE", "NODE_OPTIONS"] {
        if let Ok(value) = std::env::var(key) {
            spec_env.push((key.to_string(), value));
        }
    }

    let outcome = video_core::process_runner::run_process(
        &video_core::process_runner::ProcessSpec {
            executable: node,
            args,
            env_allow: spec_env,
            working_dir: Some(package_root),
            timeout,
            stdout_cap_bytes: 1024 * 1024,
            stderr_cap_bytes: 4 * 1024 * 1024,
        },
        &video_core::process_runner::CancellationToken::new(),
    )?;
    if !outcome.success() {
        let mut message = String::from_utf8_lossy(&outcome.stderr).trim().to_string();
        if outcome.stderr_truncated {
            message.push_str(" ...[stderr truncated]");
        }
        if let Some(signal) = outcome.signal {
            message.push_str(&format!(" (terminated by signal {signal})"));
        }
        message.push_str(&format!(
            " [exit_code={:?}, duration={:?}]",
            outcome.exit_code, outcome.duration
        ));
        return Err(RenderError::Failed(format!(
            "remotion render.mjs preview failed: {message}"
        )));
    }

    let still_path = output_dir.join("still.png");
    let motion_paths = if needs_motion {
        Some((
            output_dir.join("motion.mp4"),
            output_dir.join("motion-reduced.mp4"),
        ))
    } else {
        None
    };

    Ok(ExternalEffectPreviewOutcome {
        still_path,
        motion_paths,
        tool_identity: format!("remotion:{REMOTION_PINNED_VERSION}"),
    })
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

    // -------------------------------------------------------------
    // Ass renderer
    // -------------------------------------------------------------

    #[test]
    fn ass_bgr_color_reverses_rgb_into_ass_bbggrr_order() {
        // ASS override colors are &HAABBGGRR&, the reverse byte order of
        // CSS/RGB — a swapped-order bug here would silently render the
        // wrong highlight color, so pin the conversion directly.
        assert_eq!(ass_bgr_color((0xFF, 0xC2, 0x4B)), "&H004BC2FF&");
        assert_eq!(ass_bgr_color((0, 0, 0)), "&H00000000&");
        assert_eq!(ass_bgr_color((255, 255, 255)), "&H00FFFFFF&");
    }

    #[test]
    fn build_ass_document_differs_between_animated_and_reduced_motion() {
        let animated = build_ass_document((255, 194, 75), 1.4, true);
        let reduced = build_ass_document((255, 194, 75), 1.4, false);
        assert!(
            animated.contains("\\k"),
            "animated variant must carry karaoke timing tags"
        );
        assert!(
            !reduced.contains("\\k"),
            "reduced-motion variant must not carry karaoke timing tags"
        );
        for word in KARAOKE_DEMO_WORDS {
            assert!(animated.contains(word));
            assert!(reduced.contains(word));
        }
        assert_ne!(animated, reduced);
    }

    #[test]
    fn build_ass_document_is_deterministic() {
        let a = build_ass_document((255, 194, 75), 1.4, true);
        let b = build_ass_document((255, 194, 75), 1.4, true);
        assert_eq!(a, b);
    }

    #[test]
    fn escape_ffmpeg_filter_path_escapes_colons_and_quotes() {
        let escaped = escape_ffmpeg_filter_path(Path::new("/tmp/a:b'c.ass"));
        assert_eq!(escaped, "/tmp/a\\:b\\'c.ass");
    }

    /// Direct proof (not inferred from `video_project`'s registry-level
    /// test) that the `ass` toolchain probe fails loudly and names libass
    /// specifically when it's absent — the exact "missing-toolchain path
    /// errors clearly" contract. This workspace's local ffmpeg build has no
    /// libass (`--enable-libass` is absent from its configure line and
    /// `-filters` lists no `subtitles` entry), so this exercises the real
    /// failure path, not a mock; if a future machine's ffmpeg does have
    /// libass, the equally valid alternative branch below requires success.
    #[test]
    fn ass_subtitles_toolchain_status_reports_missing_libass_clearly_or_succeeds() {
        match ass_subtitles_toolchain_status() {
            Ok(_) => {}
            Err(RenderError::RendererUnavailable(message)) => {
                assert!(message.to_lowercase().contains("libass"));
                assert!(message.contains("--enable-libass"));
            }
            Err(other) => panic!("expected RendererUnavailable, got: {other}"),
        }
    }

    #[test]
    fn render_effect_ass_preview_fails_loudly_when_libass_is_absent_or_renders_when_present() {
        let dir = unique_dir("ass-preview");
        let props = serde_json::json!({"highlight_color": "#FFC24B", "emphasis_scale": 1.4});
        match render_effect_ass_preview(&props, true, 1.5, &dir) {
            Ok(outcome) => {
                assert!(outcome.still_path.is_file());
                let (motion, reduced) = outcome.motion_paths.expect("motion paths expected");
                assert!(motion.is_file());
                assert!(reduced.is_file());
            }
            Err(RenderError::RendererUnavailable(message)) => {
                assert!(message.to_lowercase().contains("libass"));
            }
            Err(other) => panic!("expected success or RendererUnavailable, got: {other}"),
        }
        fs::remove_dir_all(&dir).ok();
    }

    // -------------------------------------------------------------
    // Remotion renderer
    // -------------------------------------------------------------

    #[test]
    fn effect_id_to_composition_id_replaces_dots_with_dashes() {
        assert_eq!(
            effect_id_to_composition_id("lower-third.identity-card.v1"),
            "lower-third-identity-card-v1"
        );
    }

    /// `REMOTION_PINNED_VERSION` must stay byte-identical to
    /// `apps/effects/package.json`'s `dependencies.remotion` — this is the
    /// exact-pin the plan requires (no `^`/`~` ranges), asserted here so a
    /// bump to one and not the other fails a fast, no-Node-required test
    /// instead of silently drifting the render receipt's tool identity.
    #[test]
    fn remotion_pinned_version_matches_package_json() {
        let package_json_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("apps/effects/package.json");
        let text = fs::read_to_string(&package_json_path)
            .unwrap_or_else(|error| panic!("read {}: {error}", package_json_path.display()));
        let parsed: serde_json::Value =
            serde_json::from_str(&text).expect("parse apps/effects/package.json");
        let declared = parsed["dependencies"]["remotion"]
            .as_str()
            .expect("dependencies.remotion is a string");
        assert!(
            !declared.starts_with('^') && !declared.starts_with('~'),
            "apps/effects/package.json must pin an exact remotion version, got {declared:?}"
        );
        assert_eq!(
            declared, REMOTION_PINNED_VERSION,
            "REMOTION_PINNED_VERSION in effects.rs is out of sync with package.json"
        );
    }

    /// `discover_node`/`effects_package_root` read process-global
    /// `CUTRIGHT_NODE`/`CUTRIGHT_EFFECTS_PACKAGE`; guard every test that
    /// mutates either so it can't interleave with
    /// `render_effect_remotion_preview_renders_all_four_effects_and_is_deterministic`,
    /// which relies on default (unset) discovery and would otherwise race
    /// against a concurrently-running mutator — the same hazard
    /// `video_providers::whisperx`'s `discovery_tests` module guards for
    /// `CUTRIGHT_WHISPERX_PYTHON`.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn remotion_toolchain_status_reports_missing_node_clearly() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let path = PathBuf::from("/definitely/not/a/real/node/binary/on/this/machine");
        assert!(!path.exists());
        std::env::set_var("CUTRIGHT_NODE", &path);
        let result = discover_node();
        std::env::remove_var("CUTRIGHT_NODE");
        let error = result.expect_err("nonexistent CUTRIGHT_NODE override must fail");
        assert!(matches!(error, RenderError::RendererUnavailable(_)));
        assert!(error.to_string().contains(&path.display().to_string()));
    }

    /// Real end-to-end proof: each of the four Remotion-rendered effects
    /// renders its still preview, one of them (`stat-counter.v1`, the
    /// `expressive` profile) also renders both the motion and
    /// motion-reduced variants, and two independent renders of the same
    /// props+version produce byte-identical still frames. Requires
    /// `apps/effects`'s `node_modules` to be installed
    /// (`pnpm --dir apps/effects install`) — this is a checked prerequisite
    /// of this pass, not an optional environment, so this test does not
    /// honestly-degrade the way the `ass` tests above do; a missing
    /// toolchain here is a real failure.
    #[test]
    fn render_effect_remotion_preview_renders_all_four_effects_and_is_deterministic() {
        // Holds ENV_LOCK for the whole test (a real render, so this is slow)
        // purely to exclude the CUTRIGHT_NODE/CUTRIGHT_EFFECTS_PACKAGE
        // env-mutating tests above — this test itself never mutates either.
        let _guard = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let fixtures_raw = include_str!("../../../fixtures/effects/props-fixtures.json");
        let fixtures: serde_json::Value =
            serde_json::from_str(fixtures_raw).expect("parse props fixtures");
        let effects = fixtures["effects"].as_object().expect("effects object");

        let remotion_effect_ids = [
            "lower-third.identity-card.v1",
            "stat-counter.v1",
            "quote-card.v1",
            "cta-end-card.v1",
        ];

        for effect_id in remotion_effect_ids {
            let props = &effects[effect_id]["valid"];
            let needs_motion = effect_id == "stat-counter.v1";
            let dir = unique_dir(&format!("remotion-{}", effect_id.replace('.', "-")));
            let outcome = render_effect_remotion_preview(effect_id, props, needs_motion, 1.5, &dir)
                .unwrap_or_else(|error| panic!("expected {effect_id} to render: {error}"));
            assert!(outcome.still_path.is_file());
            assert!(fs::metadata(&outcome.still_path).unwrap().len() > 0);
            assert_eq!(
                outcome.tool_identity,
                format!("remotion:{REMOTION_PINNED_VERSION}")
            );
            if needs_motion {
                let (motion, reduced) = outcome.motion_paths.expect("motion paths expected");
                assert!(motion.is_file());
                assert!(fs::metadata(&motion).unwrap().len() > 0);
                assert!(reduced.is_file());
                assert!(fs::metadata(&reduced).unwrap().len() > 0);
            } else {
                assert!(outcome.motion_paths.is_none());
            }
            fs::remove_dir_all(&dir).ok();
        }

        // Determinism: same props + same pinned version render
        // byte-identical still frames.
        let props = &effects["cta-end-card.v1"]["valid"];
        let dir_a = unique_dir("remotion-determinism-a");
        let dir_b = unique_dir("remotion-determinism-b");
        let outcome_a =
            render_effect_remotion_preview("cta-end-card.v1", props, false, 1.5, &dir_a)
                .expect("first determinism render");
        let outcome_b =
            render_effect_remotion_preview("cta-end-card.v1", props, false, 1.5, &dir_b)
                .expect("second determinism render");
        let bytes_a = fs::read(&outcome_a.still_path).expect("read first still");
        let bytes_b = fs::read(&outcome_b.still_path).expect("read second still");
        assert_eq!(
            blake3::hash(&bytes_a),
            blake3::hash(&bytes_b),
            "same props + same pinned Remotion version must render byte-identical frames"
        );
        fs::remove_dir_all(&dir_a).ok();
        fs::remove_dir_all(&dir_b).ok();
    }
}
