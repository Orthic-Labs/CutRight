//! SRT-driven caption card overlay rendering (delivery + subtitled presets),
//! plus the canonical word/phrase caption model (REV2 plan §15.2).
//!
//! The canonical model — [`CaptionDocument`]/[`CaptionCue`], built by
//! [`build_caption_document`] from a project's word-level transcript and a
//! [`CaptionProfile`] — is the source of truth. SRT/VTT (see
//! `video-project`'s `io/srt.rs`) are exports derived from it, not the other
//! way around. This module still burns raw SRT into caption cards for
//! backward compatibility (`render_captioned` below), because that is the
//! contract `video-project::final_render` already depends on; the model
//! lives here (not in `video-project`) because `video-media` has no
//! dependency on `video-project` and the render path needs it directly.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use video_core::Word;

use crate::final_render::{measured_loudnorm_filter, preset_video_filter};
use crate::native::{
    MacMediaBackend, MacNativeMode, NativeCaptionRequest, NativeRenderArtifact,
    NativeRequestContext,
};
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

// ---------------------------------------------------------------------
// Canonical caption model (REV2 plan §15.2)
// ---------------------------------------------------------------------

/// Schema version for [`CaptionDocument`] and [`CaptionProfile`]. Bumped
/// independently of `video_core::models::SCHEMA_VERSION` — these artifacts
/// are owned by the caption pipeline, not the transcript/timeline family.
pub const CAPTION_MODEL_SCHEMA_VERSION: u32 = 1;

/// Which platform a [`CaptionProfile`] is tuned for. Drives the safe zone
/// and, currently, the reading-speed/line-length defaults.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CaptionPlatform {
    /// Landscape delivery with YouTube's lower-third chrome (title-safe
    /// margins, progress bar, CC toggle) in mind.
    YoutubeLowerThird,
    /// Vertical short-form (Reels/TikTok/Shorts) with the bottom
    /// description/engagement-button stack in mind.
    VerticalBottom,
}

/// Required clearance from each frame edge, as a percentage of that edge's
/// dimension, that caption content must stay inside of.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaptionSafeZone {
    pub top_pct: f64,
    pub bottom_pct: f64,
    pub left_pct: f64,
    pub right_pct: f64,
}

impl CaptionSafeZone {
    /// The pixel box `(x, y, width, height)` caption content must be placed
    /// inside of for a frame of the given dimensions.
    pub fn content_box_px(&self, width: u32, height: u32) -> (u32, u32, u32, u32) {
        let left = ((width as f64) * self.left_pct / 100.0).round() as u32;
        let right = ((width as f64) * self.right_pct / 100.0).round() as u32;
        let top = ((height as f64) * self.top_pct / 100.0).round() as u32;
        let bottom = ((height as f64) * self.bottom_pct / 100.0).round() as u32;
        let box_width = width.saturating_sub(left.saturating_add(right));
        let box_height = height.saturating_sub(top.saturating_add(bottom));
        (left, top, box_width, box_height)
    }
}

/// Versioned caption constraints: reading speed, line-length, phrase-gap
/// grouping, per-platform safe zone, and the font/fallback chain. Every
/// numeric limit that used to be a scattered constant lives here instead
/// (REV2 plan §15.2 "reading-speed and line-length constraints ... not
/// scattered constants").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaptionProfile {
    pub schema_version: u32,
    pub id: String,
    pub platform: CaptionPlatform,
    /// Hard reading-speed ceiling: a cue's character count must not exceed
    /// `max_chars_per_second * cue_duration_seconds`, or it splits.
    pub max_chars_per_second: f64,
    pub max_chars_per_line: usize,
    pub max_lines: usize,
    /// A gap between consecutive words larger than this starts a new phrase
    /// group, independent of punctuation.
    pub max_gap_merge_ms: i64,
    pub safe_zone: CaptionSafeZone,
    pub font_family: String,
    /// Ordered fallback chain consulted when `font_family` is missing a
    /// glyph the cue text needs. Order is the deterministic search order.
    pub font_fallback_chain: Vec<String>,
}

impl CaptionProfile {
    /// Landscape / YouTube lower-third default.
    pub fn youtube_lower_third() -> Self {
        Self {
            schema_version: CAPTION_MODEL_SCHEMA_VERSION,
            id: "youtube-lower-third.v1".into(),
            platform: CaptionPlatform::YoutubeLowerThird,
            max_chars_per_second: 17.0,
            max_chars_per_line: 42,
            max_lines: 2,
            max_gap_merge_ms: 700,
            safe_zone: CaptionSafeZone {
                top_pct: 10.0,
                bottom_pct: 10.0,
                left_pct: 5.0,
                right_pct: 5.0,
            },
            font_family: "IBM Plex Sans".into(),
            font_fallback_chain: vec!["Noto Sans".into(), "DejaVu Sans".into()],
        }
    }

    /// Vertical short-form default (bottom UI chrome clearance).
    pub fn vertical_bottom() -> Self {
        Self {
            schema_version: CAPTION_MODEL_SCHEMA_VERSION,
            id: "vertical-bottom.v1".into(),
            platform: CaptionPlatform::VerticalBottom,
            max_chars_per_second: 15.0,
            max_chars_per_line: 24,
            max_lines: 2,
            max_gap_merge_ms: 700,
            safe_zone: CaptionSafeZone {
                top_pct: 12.0,
                bottom_pct: 20.0,
                left_pct: 6.0,
                right_pct: 6.0,
            },
            font_family: "IBM Plex Sans".into(),
            font_fallback_chain: vec!["Noto Sans".into(), "DejaVu Sans".into()],
        }
    }

    /// The profile for a preset, keyed the same way render call sites
    /// already key vertical vs. landscape output.
    pub fn for_platform(vertical: bool) -> Self {
        if vertical {
            Self::vertical_bottom()
        } else {
            Self::youtube_lower_third()
        }
    }
}

/// A font's known glyph coverage, expressed as inclusive Unicode scalar
/// ranges. Deliberately data-driven rather than querying the host OS/font
/// system: font-fallback decisions must be reproducible on any machine,
/// independent of what is actually installed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaptionFontDescriptor {
    pub name: String,
    pub coverage: Vec<(u32, u32)>,
}

impl CaptionFontDescriptor {
    pub fn covers(&self, ch: char) -> bool {
        let code = ch as u32;
        self.coverage
            .iter()
            .any(|&(lo, hi)| code >= lo && code <= hi)
    }

    /// Basic ASCII coverage (space through tilde).
    pub fn ascii(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            coverage: vec![(0x0020, 0x007E)],
        }
    }

    /// ASCII plus Latin-1 Supplement — covers most Western-European accented
    /// characters.
    pub fn latin_extended(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            coverage: vec![(0x0020, 0x007E), (0x00A0, 0x00FF), (0x0100, 0x017F)],
        }
    }
}

/// One glyph the primary font could not render, and the deterministic
/// fallback decision made for it. A cue's `font_notices` (rolled up onto the
/// [`CaptionDocument`]) is the inventory referenced by REV2 plan §15.2
/// "deterministic font fallback and notice inventory".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaptionFontNotice {
    /// The missing character, rendered as `U+XXXX 'c'` for human review.
    pub codepoint: String,
    pub requested_font: String,
    pub resolved_font: String,
    pub reason: String,
}

/// Resolve which single font should render `text`: `primary` if it covers
/// every non-whitespace glyph, otherwise the first font in `fallback_chain`
/// that covers every glyph `primary` is missing, otherwise the last entry in
/// `fallback_chain` (or `primary` if the chain is empty) — always a
/// deterministic choice, never left to the renderer. Every glyph `primary`
/// could not cover is recorded as a [`CaptionFontNotice`], in a fixed
/// (codepoint-sorted) order so the notice inventory is stable across runs.
pub fn resolve_font_for_text(
    text: &str,
    primary: &CaptionFontDescriptor,
    fallback_chain: &[CaptionFontDescriptor],
) -> (String, Vec<CaptionFontNotice>) {
    let mut uncovered: BTreeSet<char> = BTreeSet::new();
    for ch in text.chars() {
        if ch.is_whitespace() {
            continue;
        }
        if !primary.covers(ch) {
            uncovered.insert(ch);
        }
    }
    if uncovered.is_empty() {
        return (primary.name.clone(), Vec::new());
    }
    let covering_fallback = fallback_chain
        .iter()
        .find(|font| uncovered.iter().all(|&ch| font.covers(ch)));
    let (resolved_name, reason) = match covering_fallback {
        Some(font) => (
            font.name.clone(),
            "primary font missing glyph coverage; deterministic fallback selected".to_string(),
        ),
        None => (
            fallback_chain
                .last()
                .map(|font| font.name.clone())
                .unwrap_or_else(|| primary.name.clone()),
            "no font in the fallback chain covers every required glyph; using the final \
             fallback deterministically"
                .to_string(),
        ),
    };
    let notices = uncovered
        .into_iter()
        .map(|ch| CaptionFontNotice {
            codepoint: format!("U+{:04X} '{}'", ch as u32, ch),
            requested_font: primary.name.clone(),
            resolved_font: resolved_name.clone(),
            reason: reason.clone(),
        })
        .collect();
    (resolved_name, notices)
}

/// One canonical caption cue: pre-wrapped lines, provenance back to the
/// source words, and the font actually resolved for its text.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaptionCueModel {
    pub id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub lines: Vec<String>,
    pub word_ids: Vec<String>,
    pub font_family: String,
}

/// The canonical word/phrase caption artifact (REV2 plan §15.2): source of
/// truth for every export (SRT, VTT, burned cards). Deterministic — the same
/// words + profile + fonts always produce byte-identical cues.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaptionDocument {
    pub schema_version: u32,
    pub profile_id: String,
    pub platform: CaptionPlatform,
    pub cues: Vec<CaptionCueModel>,
    pub font_notices: Vec<CaptionFontNotice>,
}

fn is_sentence_end(text: &str) -> bool {
    text.trim_end().ends_with(['.', '?', '!'])
}

fn is_clause_end(text: &str) -> bool {
    text.trim_end().ends_with([',', ';', ':', '\u{2014}'])
}

/// Break words into phrase groups at a gap larger than
/// `profile.max_gap_merge_ms`, or right after a word ending a sentence.
/// Punctuation is preferred over an arbitrary word count so a phrase group
/// starts on a clean sentence boundary whenever the transcript has one.
fn group_into_phrases(words: &[Word], max_gap_merge_ms: i64) -> Vec<Vec<Word>> {
    let mut groups: Vec<Vec<Word>> = Vec::new();
    for word in words.iter().filter(|word| word.end_ms > word.start_ms) {
        let start_new = match groups.last().and_then(|group| group.last()) {
            None => true,
            Some(last) => {
                word.start_ms - last.end_ms > max_gap_merge_ms || is_sentence_end(&last.text)
            }
        };
        if start_new {
            groups.push(vec![word.clone()]);
        } else {
            groups
                .last_mut()
                .expect("just checked non-empty")
                .push(word.clone());
        }
    }
    groups
}

fn phrase_text(group: &[Word]) -> String {
    group
        .iter()
        .map(|word| word.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Greedy word wrap: as many whole words per line as fit within
/// `max_chars_per_line`. A single word longer than the limit still gets its
/// own line rather than being cut mid-word.
fn wrap_lines(text: &str, max_chars_per_line: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.chars().count() + 1 + word.chars().count() <= max_chars_per_line {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

/// Whether a phrase group must split to satisfy the reading-speed or
/// line/lines-per-cue limits in `profile`.
fn group_needs_split(group: &[Word], profile: &CaptionProfile) -> bool {
    if group.len() <= 1 {
        return false;
    }
    let text = phrase_text(group);
    let duration_ms =
        group.last().expect("nonempty").end_ms - group.first().expect("nonempty").start_ms;
    let duration_seconds = (duration_ms.max(1)) as f64 / 1000.0;
    let max_chars_allowed = profile.max_chars_per_second * duration_seconds;
    let char_count = text.chars().count() as f64;
    if char_count > max_chars_allowed {
        return true;
    }
    wrap_lines(&text, profile.max_chars_per_line).len() > profile.max_lines
}

/// Pick the split index (into the right half) closest to the group's
/// character-count midpoint, preferring a boundary right after a
/// clause-ending word (REV2 plan §15.2 "punctuation-aware phrase grouping
/// ... break at sentence and clause boundaries in preference to mid-phrase")
/// and falling back to the plain midpoint when no clause boundary exists.
fn best_split_index(group: &[Word]) -> usize {
    let total_chars: usize = group.iter().map(|word| word.text.chars().count() + 1).sum();
    let half = total_chars / 2;

    let mut running = 0usize;
    let mut best_clause: Option<(usize, usize)> = None;
    for (index, word) in group.iter().enumerate() {
        running += word.text.chars().count() + 1;
        if index == group.len() - 1 {
            break;
        }
        if is_clause_end(&word.text) {
            let distance = running.abs_diff(half);
            if best_clause.is_none_or(|(best_distance, _)| distance < best_distance) {
                best_clause = Some((distance, index + 1));
            }
        }
    }
    if let Some((_, split_at)) = best_clause {
        return split_at.clamp(1, group.len() - 1);
    }

    running = 0;
    let mut best_index = 1usize;
    let mut best_distance = usize::MAX;
    for (index, word) in group.iter().enumerate() {
        running += word.text.chars().count() + 1;
        if index == group.len() - 1 {
            break;
        }
        let distance = running.abs_diff(half);
        if distance < best_distance {
            best_distance = distance;
            best_index = index + 1;
        }
    }
    best_index.clamp(1, group.len() - 1)
}

fn split_for_constraints(group: Vec<Word>, profile: &CaptionProfile, out: &mut Vec<Vec<Word>>) {
    if group.is_empty() {
        return;
    }
    if !group_needs_split(&group, profile) {
        out.push(group);
        return;
    }
    let split_at = best_split_index(&group);
    let mut left = group;
    let right = left.split_off(split_at);
    split_for_constraints(left, profile, out);
    split_for_constraints(right, profile, out);
}

/// Build the canonical [`CaptionDocument`] from a project's word-level
/// transcript. Deterministic: identical `words`/`profile`/fonts always
/// produce a byte-identical document (no wall-clock, no randomness, no
/// filesystem/host-font lookups).
pub fn build_caption_document(
    words: &[Word],
    profile: &CaptionProfile,
    primary_font: &CaptionFontDescriptor,
    fallback_chain: &[CaptionFontDescriptor],
) -> CaptionDocument {
    let phrases = group_into_phrases(words, profile.max_gap_merge_ms);
    let mut cue_groups = Vec::new();
    for phrase in phrases {
        split_for_constraints(phrase, profile, &mut cue_groups);
    }

    let mut cues = Vec::with_capacity(cue_groups.len());
    let mut font_notices = Vec::new();
    for (index, group) in cue_groups.into_iter().enumerate() {
        if group.is_empty() {
            continue;
        }
        let text = phrase_text(&group);
        let lines = wrap_lines(&text, profile.max_chars_per_line);
        let (resolved_font, notices) = resolve_font_for_text(&text, primary_font, fallback_chain);
        font_notices.extend(notices);
        let start_ms = group.first().expect("nonempty").start_ms;
        let end_ms = group.last().expect("nonempty").end_ms.max(start_ms + 80);
        cues.push(CaptionCueModel {
            id: format!("cue_{index:06}"),
            start_ms,
            end_ms,
            lines,
            word_ids: group.iter().map(|word| word.id.clone()).collect(),
            font_family: resolved_font,
        });
    }

    CaptionDocument {
        schema_version: CAPTION_MODEL_SCHEMA_VERSION,
        profile_id: profile.id.clone(),
        platform: profile.platform,
        cues,
        font_notices,
    }
}

/// Same as [`render_preset_with_captions_and_reframe_with_receipt`], but the
/// receipt is specifically a per-preset caption receipt (REV2 plan §15.2):
/// its parameters bind the caption profile identity/version and platform, in
/// addition to preset geometry, and its inputs/outputs are hashed exactly
/// like every other [`video_core::StageReceipt`]. Additive: existing callers
/// of the plain render functions are unaffected.
#[allow(clippy::too_many_arguments)]
pub fn render_preset_with_captions_and_reframe_with_caption_receipt(
    input: &Path,
    captions: &Path,
    output: &Path,
    width: u32,
    height: u32,
    vertical: bool,
    reframe_anchors: Option<&[ReframeAnchor]>,
    profile: &CaptionProfile,
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
        "captions.preset_burn",
        &[input, captions],
        &serde_json::json!({
            "width": width,
            "height": height,
            "vertical": vertical,
            "reframe_anchor_count": reframe_anchors.map(<[_]>::len).unwrap_or(0),
            "caption_profile_id": profile.id,
            "caption_profile_schema_version": profile.schema_version,
            "caption_platform": profile.platform,
            "caption_safe_zone": profile.safe_zone,
        }),
        output,
    )
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
    // REV2 plan §15.2 "safe zones per platform": derive the placement box
    // from the profile matching this preset's orientation, so a vertical
    // caption never lands under a platform's UI chrome. Passed through as
    // additive request fields; a worker build that does not yet consume
    // them still renders (it just ignores unknown JSON keys), and one that
    // does gets deterministic placement instead of guessing.
    let profile = CaptionProfile::for_platform(vertical);
    let (safe_x, safe_y, safe_width, safe_height) = profile.safe_zone.content_box_px(width, height);
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
                "safe_zone_x": safe_x,
                "safe_zone_y": safe_y,
                "safe_zone_width": safe_width,
                "safe_zone_height": safe_height,
                "font_family": profile.font_family,
                "font_fallback_chain": profile.font_fallback_chain,
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

/// Explicit opt-in route for one caption card. Existing delivery render APIs
/// do not call this: FFmpeg remains final-render authority. In `Shadow` mode
/// callers must supply an isolated native output path, then compare it with
/// their legacy artifact before promotion.
pub fn render_caption_card_with_native_mode(
    mode: MacNativeMode,
    backend: Option<&dyn MacMediaBackend>,
    context: &NativeRequestContext,
    request: &NativeCaptionRequest,
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
                RenderError::CaptionFailed(
                    "native caption backend unavailable for shadow comparison".into(),
                )
            })?;
            Ok(Some(
                backend
                    .render_caption(context, request)
                    .map_err(native_caption_error)?,
            ))
        }
        MacNativeMode::Native => {
            let backend = backend.ok_or_else(|| {
                RenderError::CaptionFailed("native caption backend unavailable".into())
            })?;
            Ok(Some(
                backend
                    .render_caption(context, request)
                    .map_err(native_caption_error)?,
            ))
        }
    }
}

fn native_caption_error(error: crate::native::NativeMediaError) -> RenderError {
    RenderError::CaptionFailed(error.to_string())
}

#[cfg(test)]
mod caption_model_tests {
    use super::*;

    #[test]
    fn native_caption_shadow_requires_explicit_backend() {
        let context = NativeRequestContext {
            request_id: "caption-shadow".into(),
            timeout: std::time::Duration::from_secs(1),
        };
        let result = render_caption_card_with_native_mode(
            MacNativeMode::Shadow,
            None,
            &context,
            &NativeCaptionRequest {
                output_path: std::env::temp_dir().join("caption-shadow.png"),
                width: 1,
                height: 1,
                text: "caption".into(),
                vertical: false,
                allowed_roots: vec![std::env::temp_dir()],
            },
            || Ok(()),
        );
        assert!(
            matches!(result, Err(RenderError::CaptionFailed(message)) if message.contains("shadow"))
        );
    }

    fn word(id: &str, text: &str, start_ms: i64, end_ms: i64) -> Word {
        Word {
            id: id.into(),
            source_word_id: None,
            text: text.into(),
            start_ms,
            end_ms,
            confidence: 1.0,
            speaker: None,
            kind: "word".into(),
        }
    }

    fn plain_font() -> CaptionFontDescriptor {
        CaptionFontDescriptor::ascii("Test Sans")
    }

    #[test]
    fn splits_a_cue_that_would_exceed_the_reading_rate() {
        // Two words, 4 chars each, 100ms apart: as one cue that is 9 chars in
        // 200ms — 45 chars/sec, far past the youtube profile's 17 chars/sec
        // cap — so it must split rather than flash past. A lone word can, in
        // principle, still exceed the cap after splitting (there is nothing
        // smaller left to split into); the invariant this asserts is that
        // every cue with more than one word — i.e. every cue the splitter
        // could still reduce — satisfies the cap.
        let profile = CaptionProfile::youtube_lower_third();
        let words = vec![word("w1", "aaaa", 0, 100), word("w2", "bbbb", 100, 200)];
        let doc = build_caption_document(&words, &profile, &plain_font(), &[]);
        assert!(
            doc.cues.len() > 1,
            "expected the fast cue to split, got {:?}",
            doc.cues
        );
        for cue in &doc.cues {
            if cue.word_ids.len() <= 1 {
                continue; // irreducible: nothing smaller to split into.
            }
            let chars: usize = cue.lines.iter().map(|line| line.chars().count()).sum();
            let duration_s = ((cue.end_ms - cue.start_ms).max(1)) as f64 / 1000.0;
            assert!(
                chars as f64 <= profile.max_chars_per_second * duration_s + 1e-9,
                "multi-word cue {cue:?} exceeds the reading-rate cap"
            );
        }
    }

    #[test]
    fn wraps_long_lines_within_the_profile_limit() {
        let mut profile = CaptionProfile::vertical_bottom();
        profile.max_chars_per_second = 1_000.0; // isolate line-length behavior
        profile.max_gap_merge_ms = 10_000;
        profile.max_lines = 5;
        // No single word exceeds max_chars_per_line (24) on its own, so the
        // wrapper always has room to break between words.
        let words = vec![
            word("w1", "another", 0, 1_000),
            word("w2", "reasonably", 1_000, 2_000),
            word("w3", "long", 2_000, 3_000),
            word("w4", "phrase", 3_000, 4_000),
            word("w5", "that", 4_000, 5_000),
            word("w6", "keeps", 5_000, 6_000),
            word("w7", "going", 6_000, 7_000),
            word("w8", "here", 7_000, 8_000),
        ];
        let doc = build_caption_document(&words, &profile, &plain_font(), &[]);
        assert!(!doc.cues.is_empty());
        for cue in &doc.cues {
            for line in &cue.lines {
                assert!(
                    line.chars().count() <= profile.max_chars_per_line,
                    "line {line:?} exceeds max_chars_per_line {}",
                    profile.max_chars_per_line
                );
            }
        }
    }

    #[test]
    fn prefers_a_clause_boundary_over_a_mid_phrase_split() {
        let mut profile = CaptionProfile::youtube_lower_third();
        profile.max_chars_per_second = 1_000.0; // force the split via line-length only
        profile.max_chars_per_line = 20;
        profile.max_lines = 1;
        profile.max_gap_merge_ms = 10_000;
        // Two clauses: "First, clause here," and "second clause follows.".
        // The plain char-count midpoint falls inside the first clause; the
        // nearer clause boundary (after "here,") is what the split must use.
        let words = vec![
            word("w1", "First,", 0, 200),
            word("w2", "clause", 200, 400),
            word("w3", "here,", 400, 600),
            word("w4", "second", 600, 800),
            word("w5", "clause", 800, 1_000),
            word("w6", "follows.", 1_000, 1_200),
        ];
        let doc = build_caption_document(&words, &profile, &plain_font(), &[]);
        assert!(doc.cues.len() >= 2, "expected a split: {:?}", doc.cues);
        let first_cue_word_ids = &doc.cues[0].word_ids;
        assert_eq!(
            first_cue_word_ids,
            &["w1".to_string(), "w2".to_string(), "w3".to_string()],
            "split should land right after the clause-ending word, not mid-phrase"
        );
    }

    #[test]
    fn safe_zone_never_reaches_the_vertical_bottom_ui_chrome() {
        let youtube = CaptionProfile::youtube_lower_third();
        let vertical = CaptionProfile::vertical_bottom();
        let (_, _, _, youtube_height) = youtube.safe_zone.content_box_px(1920, 1080);
        let (_, _, _, vertical_height) = vertical.safe_zone.content_box_px(1080, 1920);
        // Vertical reserves more bottom clearance for platform UI chrome
        // (description/engagement buttons) than landscape's lower third.
        assert!(vertical.safe_zone.bottom_pct > youtube.safe_zone.bottom_pct);
        assert!(youtube_height > 0 && vertical_height > 0);
    }

    #[test]
    fn font_fallback_is_deterministic_and_recorded() {
        let primary = CaptionFontDescriptor::ascii("Primary");
        let fallback = CaptionFontDescriptor::latin_extended("Fallback");
        let words = vec![word("w1", "caf\u{e9}", 0, 500)]; // needs 'é'
        let profile = CaptionProfile::youtube_lower_third();
        let doc =
            build_caption_document(&words, &profile, &primary, std::slice::from_ref(&fallback));
        assert_eq!(doc.cues.len(), 1);
        assert_eq!(doc.cues[0].font_family, "Fallback");
        assert_eq!(doc.font_notices.len(), 1);
        assert_eq!(doc.font_notices[0].resolved_font, "Fallback");
        assert!(doc.font_notices[0].codepoint.contains("U+00E9"));

        // Run again: identical input must produce an identical notice list.
        let doc2 = build_caption_document(&words, &profile, &primary, &[fallback]);
        assert_eq!(doc.font_notices, doc2.font_notices);
    }

    #[test]
    fn same_transcript_and_profile_produce_byte_identical_cues() {
        let profile = CaptionProfile::youtube_lower_third();
        let font = plain_font();
        let words = vec![
            word("w1", "Today", 0, 300),
            word("w2", "we", 300, 500),
            word("w3", "build", 500, 900),
            word("w4", "this.", 900, 1_300),
            word("w5", "Then", 2_200, 2_500),
            word("w6", "we", 2_500, 2_700),
            word("w7", "ship", 2_700, 3_100),
            word("w8", "it.", 3_100, 3_400),
        ];
        let first = build_caption_document(&words, &profile, &font, &[]);
        let second = build_caption_document(&words, &profile, &font, &[]);
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap(),
            "same transcript + profile must yield byte-identical cues"
        );
        // Sanity: the sentence-ending word ("this.") starts a new phrase.
        assert!(first.cues.len() >= 2);
    }
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
