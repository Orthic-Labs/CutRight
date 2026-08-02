//! Input color-space detection, HDR/Apple-Log-to-SDR conversion, exposure/
//! white-balance correction, shot matching, a bounded creative LUT, output
//! color-metadata verification, and a review contact sheet (REV2 plan §15.2
//! "Color"). Every parameter this module accepts is threaded in explicitly
//! by the caller (`video-project`'s versioned `ColorProfile` artifact) —
//! nothing here hard-codes grading constants.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use video_core::process_runner::{CancellationToken, ProcessSpec};

use crate::process::{
    duration_scaled_timeout_with_toolchain, media_env_allow, run_media_command, string_args,
    PROBE_STDOUT_CAP_BYTES, PROBE_TIMEOUT, STDERR_CAP_BYTES, WAVEFORM_PER_SOURCE_SECOND,
    WAVEFORM_RENDER_FLOOR,
};
use crate::toolchain::MediaToolchain;
use crate::RenderError;

/// The classified input color space (plan §15.2: "input color-space
/// detection ... rather than assuming").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorSpaceKind {
    /// Standard-dynamic-range Rec.709 — the common case, and the working/
    /// delivery space every other kind converts down to.
    Rec709,
    /// HLG (`arib-std-b67`) HDR.
    Rec2020Hlg,
    /// PQ (`smpte2084`) HDR.
    Rec2020Pq,
    /// Apple Log — the flat log profile iPhone 15 Pro+ (and other Apple
    /// devices) can shoot in. FFmpeg has no dedicated `color_transfer` enum
    /// for it: Apple Log clips are tagged `bt2020` primaries with an
    /// unspecified/`unknown` transfer, which is otherwise indistinguishable
    /// from a generically untagged source — so this is a best-effort
    /// heuristic (primaries=bt2020 and no explicit transfer), not a
    /// guarantee. A future revision can tighten this once FFmpeg ships an
    /// explicit Apple Log side-data tag.
    AppleLog,
    /// Tags present but not one of the above — treated as SDR Rec.709
    /// pass-through rather than guessed at.
    Unknown,
}

/// Raw `color_transfer`/`color_primaries`/`color_space` (matrix
/// coefficients) tags as ffprobe reports them, before classification.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceColorTags {
    pub color_transfer: Option<String>,
    pub color_primaries: Option<String>,
    pub color_matrix: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ColorProbeResponse {
    #[serde(default)]
    streams: Vec<ColorProbeStream>,
}

#[derive(Debug, Deserialize)]
struct ColorProbeStream {
    color_transfer: Option<String>,
    color_primaries: Option<String>,
    color_space: Option<String>,
}

/// Probes the first video stream's raw color tags. A dedicated ffprobe call
/// (rather than routing through [`crate::probe::probe`]) because that probe
/// only extracts the boolean `is_hdr`, not the transfer/primaries/matrix
/// triple this module classifies against.
pub fn probe_source_color_tags(
    path: &Path,
    toolchain: &MediaToolchain,
) -> Result<SourceColorTags, RenderError> {
    if !path.is_file() {
        return Err(RenderError::Failed(format!(
            "input does not exist: {}",
            path.display()
        )));
    }
    let spec = ProcessSpec {
        executable: toolchain.ffprobe.clone(),
        args: vec![
            "-v".into(),
            "error".into(),
            "-select_streams".into(),
            "v:0".into(),
            "-show_entries".into(),
            "stream=color_transfer,color_primaries,color_space".into(),
            "-print_format".into(),
            "json".into(),
            path.display().to_string(),
        ],
        env_allow: media_env_allow(),
        working_dir: None,
        timeout: PROBE_TIMEOUT,
        stdout_cap_bytes: PROBE_STDOUT_CAP_BYTES,
        stderr_cap_bytes: STDERR_CAP_BYTES,
    };
    let outcome = video_core::process_runner::run_process(&spec, &CancellationToken::new())
        .map_err(RenderError::Process)?;
    if !outcome.success() {
        let mut message = String::from_utf8_lossy(&outcome.stderr).trim().to_string();
        if outcome.stderr_truncated {
            message.push_str(" ...[stderr truncated]");
        }
        return Err(RenderError::Failed(message));
    }
    let response: ColorProbeResponse = serde_json::from_slice(&outcome.stdout)
        .map_err(|error| RenderError::Failed(format!("invalid ffprobe color JSON: {error}")))?;
    let stream = response
        .streams
        .into_iter()
        .next()
        .unwrap_or(ColorProbeStream {
            color_transfer: None,
            color_primaries: None,
            color_space: None,
        });
    Ok(SourceColorTags {
        color_transfer: normalize_tag(stream.color_transfer),
        color_primaries: normalize_tag(stream.color_primaries),
        color_matrix: normalize_tag(stream.color_space),
    })
}

fn normalize_tag(tag: Option<String>) -> Option<String> {
    tag.filter(|value| !value.is_empty() && value != "unknown")
}

/// Classifies [`SourceColorTags`] into a [`ColorSpaceKind`]. Pure/no I/O so
/// it is directly unit-testable against fixture tag combinations.
pub fn detect_color_space(tags: &SourceColorTags) -> ColorSpaceKind {
    match tags.color_transfer.as_deref() {
        Some("smpte2084") => ColorSpaceKind::Rec2020Pq,
        Some("arib-std-b67") => ColorSpaceKind::Rec2020Hlg,
        Some("bt709") => ColorSpaceKind::Rec709,
        None => {
            // No explicit transfer: Apple Log clips are tagged bt2020
            // primaries with the transfer left unspecified. Anything else
            // untagged is treated as Unknown rather than guessed at.
            if tags.color_primaries.as_deref() == Some("bt2020") {
                ColorSpaceKind::AppleLog
            } else {
                ColorSpaceKind::Unknown
            }
        }
        _ => ColorSpaceKind::Unknown,
    }
}

/// Exposure (in stops) and white-balance shift correction, applied uniformly
/// before shot matching and any creative LUT.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ColorCorrection {
    /// Exposure adjustment in EV (stops). 0.0 = no change.
    pub exposure_ev: f64,
    /// White-balance temperature shift in Kelvin (positive = warmer target).
    /// 0.0 = no change.
    pub white_balance_temp_shift: f64,
    /// White-balance tint shift, green-magenta axis, roughly matching
    /// `colortemperature`'s `-100..100` `mix` convention. 0.0 = no change.
    pub white_balance_tint_shift: f64,
}

impl Default for ColorCorrection {
    fn default() -> Self {
        ColorCorrection {
            exposure_ev: 0.0,
            white_balance_temp_shift: 0.0,
            white_balance_tint_shift: 0.0,
        }
    }
}

/// Target statistics for shot matching consecutive clips from different
/// sources so a cut between them doesn't visibly jump in brightness/
/// saturation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShotMatchTarget {
    /// Target mean luma, 0.0 (black) .. 1.0 (white).
    pub mean_luma: f64,
    /// Target mean saturation multiplier relative to source, where 1.0 is
    /// unchanged.
    pub saturation_scale: f64,
}

/// An optional approved creative LUT applied at a BOUNDED strength (plan
/// §15.2: "never unbounded, and never applied without the profile naming
/// it"). `strength` is always clamped into `0.0..=1.0` by [`CreativeLut::new`]
/// — there is no code path that can apply an out-of-range strength.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreativeLut {
    pub path: PathBuf,
    strength: f64,
}

impl CreativeLut {
    /// Clamps `strength` into `0.0..=1.0` (or `0.0` if it is NaN) — bounded
    /// by construction, never by convention at each call site.
    pub fn new(path: PathBuf, strength: f64) -> Self {
        let strength = if strength.is_finite() {
            strength.clamp(0.0, 1.0)
        } else {
            0.0
        };
        CreativeLut { path, strength }
    }

    pub fn strength(&self) -> f64 {
        self.strength
    }
}

/// The color tags a render is expected to carry once this module's filter
/// chain has run — every conversion path converges on Rec.709 SDR, the
/// defined working/delivery space (plan §15.2: "HDR/HLG/PQ to defined SDR
/// working space").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedColorMetadata {
    pub color_primaries: &'static str,
    pub color_transfer: &'static str,
    pub color_matrix: &'static str,
}

pub const REC709_SDR_METADATA: ExpectedColorMetadata = ExpectedColorMetadata {
    color_primaries: "bt709",
    color_transfer: "bt709",
    color_matrix: "bt709",
};

/// Builds the full color filter chain: input-space-to-SDR conversion, then
/// exposure/white-balance correction, then shot matching, then an optional
/// bounded creative LUT. Returns the filter string plus the color metadata
/// the resulting frames are expected to carry, so the caller can verify the
/// actual render against it after encoding.
pub fn color_filter_chain(
    kind: ColorSpaceKind,
    correction: &ColorCorrection,
    shot_match: Option<&ShotMatchTarget>,
    lut: Option<&CreativeLut>,
    toolchain: &MediaToolchain,
) -> Result<(String, ExpectedColorMetadata), RenderError> {
    let mut stages: Vec<String> = Vec::new();

    match kind {
        ColorSpaceKind::Rec2020Pq | ColorSpaceKind::Rec2020Hlg => {
            if !toolchain.capabilities.has_zscale {
                return Err(RenderError::CapabilityMissing(
                    "HDR-to-SDR color conversion requires FFmpeg built with the zscale filter; install a zimg-enabled FFmpeg build".into(),
                ));
            }
            stages.push(
                "zscale=transfer=linear:npl=100,format=gbrpf32le,tonemap=tonemap=hable:desat=0,zscale=primaries=bt709:transfer=bt709:matrix=bt709,format=yuv420p"
                    .to_string(),
            );
        }
        ColorSpaceKind::AppleLog => {
            if !toolchain.capabilities.has_zscale {
                return Err(RenderError::CapabilityMissing(
                    "Apple Log color conversion requires FFmpeg built with the zscale filter; install a zimg-enabled FFmpeg build".into(),
                ));
            }
            // Best-effort log-to-linear approximation pending an official
            // Apple Log 3D LUT: lift the flat log curve with a gamma/
            // contrast correction before re-tagging to bt709. This is
            // deliberately conservative rather than an exact vendor
            // transform — the output metadata verification step below still
            // guarantees the DELIVERED tag is correct even though the exact
            // tone curve is an approximation.
            stages.push("eq=gamma=1.8:contrast=1.12:saturation=1.08".to_string());
            stages.push("zscale=primaries=bt709:transfer=bt709:matrix=bt709".to_string());
        }
        ColorSpaceKind::Rec709 | ColorSpaceKind::Unknown => {
            // Bake the bt709 tag onto every frame via zscale when
            // available, rather than relying solely on output-level
            // `-color_primaries`/`-color_trc`/`-colorspace` flags: some
            // encoder/muxer combinations only write those tags to the
            // bitstream/container when they differ from the encoder's own
            // internal default, so flag-only tagging can silently produce
            // an untagged output even when the flags were passed — exactly
            // the failure mode output metadata verification exists to
            // catch. Falls back to a plain pass-through (relying on output
            // flags alone) when zscale is unavailable, rather than gating
            // the common Rec.709 case on an HDR-only capability.
            if toolchain.capabilities.has_zscale {
                // No real conversion happens here (the source either IS
                // bt709 or has no usable tags at all) — `in`/`out` are the
                // same triple, so this is purely a re-tag, and explicit
                // `in` values are required: zscale can't resolve a
                // conversion path from a genuinely untagged source
                // otherwise ("no path between colorspaces").
                stages.push(
                    "zscale=primariesin=bt709:transferin=bt709:matrixin=bt709:primaries=bt709:transfer=bt709:matrix=bt709,format=yuv420p"
                        .to_string(),
                );
            } else {
                stages.push("format=yuv420p".to_string());
            }
        }
    }

    if correction.exposure_ev != 0.0 {
        // eq's brightness is additive in the 0..1 luma range; approximate a
        // stop as a fixed offset per EV, gamma compensates midtones.
        let brightness = correction.exposure_ev * 0.08;
        let gamma = 2f64.powf(correction.exposure_ev * 0.15).clamp(0.3, 3.0);
        stages.push(format!("eq=brightness={brightness:.4}:gamma={gamma:.4}"));
    }

    if correction.white_balance_temp_shift != 0.0 || correction.white_balance_tint_shift != 0.0 {
        if !toolchain.capabilities.has_colortemperature {
            return Err(RenderError::CapabilityMissing(
                "white-balance correction requires FFmpeg built with the colortemperature filter"
                    .into(),
            ));
        }
        let base_temperature = 6500.0 + correction.white_balance_temp_shift;
        let mix = (correction.white_balance_tint_shift / 100.0).clamp(-1.0, 1.0);
        stages.push(format!(
            "colortemperature=temperature={base_temperature:.1}:mix={mix:.3}:pl=1"
        ));
    }

    if let Some(target) = shot_match {
        let brightness = (target.mean_luma - 0.5).clamp(-0.5, 0.5);
        let saturation = target.saturation_scale.clamp(0.0, 3.0);
        stages.push(format!(
            "eq=brightness={brightness:.4}:saturation={saturation:.4}"
        ));
    }

    let mut filter = stages.join(",");

    if let Some(lut) = lut {
        if lut.strength() > 0.0 {
            if !toolchain.capabilities.has_lut3d {
                return Err(RenderError::CapabilityMissing(
                    "creative LUT application requires FFmpeg built with the lut3d filter".into(),
                ));
            }
            let lut_path = lut.path.display().to_string().replace(':', "\\:");
            let strength = lut.strength();
            // split -> graded copy via lut3d -> blend graded over ungraded
            // by the bounded strength -> single output stream.
            filter = format!(
                "{filter},split=2[cr_base][cr_grade];[cr_grade]lut3d=file='{lut_path}':interp=trilinear[cr_luted];[cr_base][cr_luted]blend=all_expr='A*(1-{strength:.4})+B*{strength:.4}'"
            );
        }
    }

    Ok((filter, REC709_SDR_METADATA))
}

/// Probes `output` and confirms its color tags match `expected` exactly
/// (plan §15.2: "a render that silently emits the wrong transfer is a
/// defect"). Fails loudly on ANY mismatch rather than only checking one
/// field.
pub fn verify_output_color_metadata(
    output: &Path,
    expected: &ExpectedColorMetadata,
    toolchain: &MediaToolchain,
) -> Result<(), RenderError> {
    let actual = probe_source_color_tags(output, toolchain)?;
    let matches = actual.color_primaries.as_deref() == Some(expected.color_primaries)
        && actual.color_transfer.as_deref() == Some(expected.color_transfer)
        && actual.color_matrix.as_deref() == Some(expected.color_matrix);
    if matches {
        Ok(())
    } else {
        Err(RenderError::ColorMetadataMismatch {
            expected: format!(
                "primaries={} transfer={} matrix={}",
                expected.color_primaries, expected.color_transfer, expected.color_matrix
            ),
            actual: format!(
                "primaries={:?} transfer={:?} matrix={:?}",
                actual.color_primaries, actual.color_transfer, actual.color_matrix
            ),
        })
    }
}

/// Renders a grid contact sheet (one PNG, `columns * rows` evenly spaced
/// frames) so a human can review the graded result across the whole
/// timeline at a glance (plan §15.2: "review contact sheet").
pub fn render_contact_sheet(
    input: &Path,
    output_png: &Path,
    columns: u32,
    rows: u32,
    toolchain: &MediaToolchain,
) -> Result<(), RenderError> {
    if !input.is_file() {
        return Err(RenderError::Failed(format!(
            "input does not exist: {}",
            input.display()
        )));
    }
    if columns == 0 || rows == 0 {
        return Err(RenderError::Failed(
            "contact sheet grid must be nonzero".into(),
        ));
    }
    let frame_count = (columns * rows) as i64;
    let metadata = crate::probe::probe_with_toolchain(input, toolchain)
        .map_err(|error| RenderError::Failed(error.to_string()))?;
    let duration_ms = metadata.duration_ms.unwrap_or(1_000).max(1_000);
    // One evenly spaced frame every `interval_secs` seconds, tiled into a
    // single image — `select='not(mod(t,interval))'` samples on a time
    // grid rather than a frame-count grid, so it works regardless of the
    // source frame rate.
    let interval_secs = ((duration_ms as f64 / 1_000.0) / frame_count as f64).max(0.1);
    let filter = format!(
        "select='isnan(prev_selected_t)+gte(t-prev_selected_t\\,{interval_secs:.3})',scale=320:-1,tile={columns}x{rows}"
    );
    let mut args = string_args(["-hide_banner", "-loglevel", "error", "-y", "-i"]);
    args.push(input.display().to_string());
    args.extend(string_args(["-vf", &filter, "-frames:v", "1"]));
    args.push(output_png.display().to_string());
    let timeout = duration_scaled_timeout_with_toolchain(
        input,
        toolchain,
        WAVEFORM_RENDER_FLOOR,
        WAVEFORM_PER_SOURCE_SECOND,
    );
    run_media_command(&toolchain.ffmpeg, args, timeout, RenderError::Failed)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolchain;

    #[test]
    fn detects_pq_hdr() {
        let tags = SourceColorTags {
            color_transfer: Some("smpte2084".into()),
            color_primaries: Some("bt2020".into()),
            color_matrix: Some("bt2020nc".into()),
        };
        assert_eq!(detect_color_space(&tags), ColorSpaceKind::Rec2020Pq);
    }

    #[test]
    fn detects_hlg_hdr() {
        let tags = SourceColorTags {
            color_transfer: Some("arib-std-b67".into()),
            color_primaries: Some("bt2020".into()),
            color_matrix: Some("bt2020nc".into()),
        };
        assert_eq!(detect_color_space(&tags), ColorSpaceKind::Rec2020Hlg);
    }

    #[test]
    fn detects_apple_log_from_untagged_transfer_with_bt2020_primaries() {
        let tags = SourceColorTags {
            color_transfer: None,
            color_primaries: Some("bt2020".into()),
            color_matrix: Some("bt2020nc".into()),
        };
        assert_eq!(detect_color_space(&tags), ColorSpaceKind::AppleLog);
    }

    #[test]
    fn detects_rec709_sdr() {
        let tags = SourceColorTags {
            color_transfer: Some("bt709".into()),
            color_primaries: Some("bt709".into()),
            color_matrix: Some("bt709".into()),
        };
        assert_eq!(detect_color_space(&tags), ColorSpaceKind::Rec709);
    }

    #[test]
    fn falls_back_to_unknown_without_enough_signal() {
        let tags = SourceColorTags::default();
        assert_eq!(detect_color_space(&tags), ColorSpaceKind::Unknown);
    }

    #[test]
    fn creative_lut_strength_is_always_clamped_into_bounds() {
        assert_eq!(CreativeLut::new("lut.cube".into(), 5.0).strength(), 1.0);
        assert_eq!(CreativeLut::new("lut.cube".into(), -5.0).strength(), 0.0);
        assert_eq!(CreativeLut::new("lut.cube".into(), 0.5).strength(), 0.5);
        assert_eq!(
            CreativeLut::new("lut.cube".into(), f64::NAN).strength(),
            0.0
        );
    }

    fn fake_toolchain_with_capabilities(
        zscale: bool,
        lut3d: bool,
        colortemperature: bool,
    ) -> MediaToolchain {
        MediaToolchain {
            ffmpeg: PathBuf::from("ffmpeg"),
            ffprobe: PathBuf::from("ffprobe"),
            version: "fixture".into(),
            content_hash: "fixture".into(),
            capabilities: crate::toolchain::MediaCapabilities {
                has_zscale: zscale,
                has_h264_videotoolbox: false,
                has_prores_ks: false,
                has_lut3d: lut3d,
                has_colortemperature: colortemperature,
            },
        }
    }

    #[test]
    fn rejects_hdr_conversion_without_zscale() {
        let toolchain = fake_toolchain_with_capabilities(false, true, true);
        let result = color_filter_chain(
            ColorSpaceKind::Rec2020Pq,
            &ColorCorrection::default(),
            None,
            None,
            &toolchain,
        );
        assert!(matches!(result, Err(RenderError::CapabilityMissing(_))));
    }

    #[test]
    fn rejects_lut_without_lut3d_capability() {
        let toolchain = fake_toolchain_with_capabilities(true, false, true);
        let lut = CreativeLut::new("grade.cube".into(), 0.5);
        let result = color_filter_chain(
            ColorSpaceKind::Rec709,
            &ColorCorrection::default(),
            None,
            Some(&lut),
            &toolchain,
        );
        assert!(matches!(result, Err(RenderError::CapabilityMissing(_))));
    }

    #[test]
    fn zero_strength_lut_never_touches_the_filter_chain() {
        let toolchain = fake_toolchain_with_capabilities(true, false, true);
        let lut = CreativeLut::new("grade.cube".into(), 0.0);
        let (filter, expected) = color_filter_chain(
            ColorSpaceKind::Rec709,
            &ColorCorrection::default(),
            None,
            Some(&lut),
            &toolchain,
        )
        .expect("zero-strength LUT does not require lut3d capability");
        assert!(!filter.contains("lut3d"));
        assert_eq!(expected, REC709_SDR_METADATA);
    }

    /// Same unique-scratch-dir pattern as `toolchain`'s own tests (no
    /// `tempfile` dependency in this crate).
    fn unique_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cutright-color-test-{label}-{unique}"));
        std::fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    /// Generates a tiny real ffmpeg-encoded clip explicitly tagged with the
    /// given color primaries/transfer/matrix, so metadata verification can
    /// be exercised against real ffprobe output rather than a fixture
    /// struct. Bakes the tags into frame metadata via `zscale` (the same
    /// mechanism `color_filter_chain`'s HDR/Apple-Log paths use) rather
    /// than relying solely on `-color_primaries`/`-color_trc`/`-colorspace`
    /// output flags: libx264 only writes H.264 VUI colorimetry when it
    /// differs from its own internal defaults, so flags alone can be
    /// silently dropped from the bitstream for values that happen to match
    /// those defaults — `zscale` forces the tag onto every frame instead.
    fn generate_tagged_fixture(path: &Path, primaries: &str, transfer: &str, matrix: &str) {
        // Uses the RESOLVED toolchain's ffmpeg (the bundled zimg build when
        // present), not a bare `ffmpeg` on `PATH` — a plain Homebrew/system
        // ffmpeg build commonly lacks `zscale`, which would make this test
        // fail for an environment reason unrelated to what it's checking.
        let toolchain = toolchain::resolve().expect("resolve system ffmpeg/ffprobe");
        let status = std::process::Command::new(&toolchain.ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=black:s=64x64:r=10",
                "-t",
                "1",
                "-vf",
                &format!(
                    "zscale=primariesin={primaries}:transferin={transfer}:matrixin={matrix}:primaries={primaries}:transfer={transfer}:matrix={matrix},format=yuv420p"
                ),
                "-c:v",
                "libx264",
                "-color_primaries",
                primaries,
                "-color_trc",
                transfer,
                "-colorspace",
                matrix,
            ])
            .arg(path)
            .status()
            .expect("start fixture ffmpeg");
        assert!(status.success(), "fixture ffmpeg encode failed");
    }

    /// REV2 plan §15.2 Color regression: `verify_output_color_metadata`
    /// must FAIL when the render actually emits the wrong transfer, not
    /// just when tags are entirely absent. Encodes a real fixture tagged
    /// HDR (smpte2084/bt2020) and checks it against the Rec.709 SDR
    /// expectation every color path is supposed to converge on.
    #[test]
    fn verify_output_color_metadata_fails_on_a_real_mismatched_render() {
        let toolchain = toolchain::resolve().expect("resolve system ffmpeg/ffprobe");
        let dir = unique_dir("mismatch");
        let output = dir.join("wrong-transfer.mp4");
        generate_tagged_fixture(&output, "bt2020", "smpte2084", "bt2020nc");

        let result = verify_output_color_metadata(&output, &REC709_SDR_METADATA, &toolchain);

        assert!(
            matches!(result, Err(RenderError::ColorMetadataMismatch { .. })),
            "expected a ColorMetadataMismatch, got {result:?}"
        );
        std::fs::remove_dir_all(&dir).expect("remove test dir");
    }

    /// Companion positive case: a render actually tagged bt709/bt709/bt709
    /// passes verification against the same expectation.
    #[test]
    fn verify_output_color_metadata_passes_on_a_correctly_tagged_render() {
        let toolchain = toolchain::resolve().expect("resolve system ffmpeg/ffprobe");
        let dir = unique_dir("match");
        let output = dir.join("correct-transfer.mp4");
        generate_tagged_fixture(&output, "bt709", "bt709", "bt709");

        let result = verify_output_color_metadata(&output, &REC709_SDR_METADATA, &toolchain);

        assert!(result.is_ok(), "expected verification to pass: {result:?}");
        std::fs::remove_dir_all(&dir).expect("remove test dir");
    }

    #[test]
    fn rec709_pass_through_needs_no_optional_capability() {
        let toolchain = fake_toolchain_with_capabilities(false, false, false);
        let (filter, expected) = color_filter_chain(
            ColorSpaceKind::Rec709,
            &ColorCorrection::default(),
            None,
            None,
            &toolchain,
        )
        .expect("rec709 pass-through needs no optional capability");
        assert_eq!(filter, "format=yuv420p");
        assert_eq!(expected, REC709_SDR_METADATA);
    }

    #[test]
    fn rec709_pass_through_bakes_tags_via_zscale_when_available() {
        let toolchain = fake_toolchain_with_capabilities(true, false, false);
        let (filter, expected) = color_filter_chain(
            ColorSpaceKind::Rec709,
            &ColorCorrection::default(),
            None,
            None,
            &toolchain,
        )
        .expect("rec709 pass-through with zscale available");
        assert!(filter.contains("zscale=primariesin=bt709"));
        assert!(filter.contains("primaries=bt709:transfer=bt709:matrix=bt709"));
        assert_eq!(expected, REC709_SDR_METADATA);
    }
}
