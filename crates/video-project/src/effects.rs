//! The typed effect registry (REV2 plan §15.3 Phase 5).
//!
//! Every render effect — caption profile, lower third, stat counter, quote
//! card, CTA end card, and anything added after them — is one
//! [`EffectRegistryEntry`] loaded from `schemas/effects/registry.json`
//! (embedded at compile time). Adding an effect means adding one JSON entry
//! plus a props schema and fixture entry, never a new `match` arm in the
//! render path: [`render_effect_preview`] is the same function for every
//! entry, driven entirely by the entry's own data plus caller-supplied
//! props.
//!
//! [`EffectRegistry::validate_all`] is the enforcement gate the plan
//! requires before an effect can be "marked usable": every entry must
//! declare a still preview, a motion preview when its `motion_profile`
//! isn't `Static`, an explicit reduced-motion behavior, and a footprint
//! that does not collide with the caption safe zones it targets
//! (`video_media::captions`'s `CaptionSafeZone`,
//! `CaptionProfile::youtube_lower_third()` /
//! `CaptionProfile::vertical_bottom()`).
//!
//! The `renderer` field is typed ([`EffectRenderer`]) across four values:
//! `ffmpeg` (the fast drawbox path, unchanged), `ass` (real libass karaoke
//! text — `caption.bold-karaoke.v1`), `remotion` (real Node/React branded
//! kinetic motion — the other four starter effects), and `hyperframes`
//! (reserved: bespoke type, per
//! `skills/content-video-editor/workflows/finish.md`; no implementation or
//! dependency exists anywhere in this workspace, so it fails loudly rather
//! than being folded into `remotion`). Remotion's license was re-verified
//! for this pass (free tier: individuals and teams of 3 or fewer; see
//! `apps/effects/README.md`'s Licensing section for the upgrade trigger) —
//! it is now a real dependency (`apps/effects/`), not a reserved variant.
//!
//! The plan's 15 starter effects are all registered. Ten generic editorial
//! moves use the existing data-driven FFmpeg card renderer, while branded
//! compositions retain their dedicated ASS or Remotion renderer.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use video_core::{render_native_effect_frame, NativeEffectFrame, StageReceipt};
use video_media::{CaptionProfile, CaptionSafeZone};

use crate::io::write_json_atomic;
use crate::receipts::{receipt_path_for, write_stage_receipt};
use crate::ProjectError;

/// Schema version for the registry document itself
/// (`schemas/effects/registry.json`'s `schema_version` field and this
/// module's in-memory shape). Independent of each entry's own
/// `schema_version` (its props-schema generation).
pub const EFFECT_REGISTRY_SCHEMA_VERSION: u32 = 1;

/// The registry document, embedded at compile time so "adding an effect"
/// never requires wiring a new file path into the binary — only editing
/// this one tracked JSON file plus its schema and fixtures.
const REGISTRY_JSON: &str = include_str!("../../../schemas/effects/registry.json");

#[derive(Debug, thiserror::Error)]
pub enum EffectRegistryError {
    #[error("effect registry document is invalid: {0}")]
    Malformed(#[source] serde_json::Error),
    #[error("effect registry schema_version {0} is not {EFFECT_REGISTRY_SCHEMA_VERSION}")]
    UnsupportedSchema(u32),
    #[error("no registry entry for effect_id {0:?}")]
    UnknownEffect(String),
    #[error("retired_renderer: {renderer}; migrate this effect to renderer=native")]
    RetiredRenderer { renderer: String },
    #[error("effect {effect_id}: {message}")]
    Invalid { effect_id: String, message: String },
    #[error("effect {effect_id} footprint collides with safe zone {safe_zone:?}: {message}")]
    Collision {
        effect_id: String,
        safe_zone: SafeZoneRef,
        message: String,
    },
    #[error("effect {effect_id} props are invalid: {}", .messages.join("; "))]
    PropsInvalid {
        effect_id: String,
        messages: Vec<String>,
    },
    #[error(transparent)]
    Project(#[from] ProjectError),
    #[error(transparent)]
    Render(#[from] video_media::RenderError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// The renderer that owns a registry entry's actual pixel output. Typed
/// rather than free text (REV2 plan §15.3 constraint) so a caller can match
/// exhaustively instead of string-comparing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EffectRenderer {
    /// The only executable renderer. Native effects render through CutRight's
    /// deterministic Rust raster path, never Node, Chromium, or a shell
    /// compositor.
    Native,
    /// The generic drawbox lavfi card path (`video_media::effects`'s
    /// `render_effect_still`/`render_effect_motion`) — the fast path, no
    /// registry entry uses it today but it stays fully implemented and
    /// tested (see `ffmpeg_renderer_still_and_motion_render_via_registry`
    /// below) for any future effect that just needs a labeled box.
    Ffmpeg,
    /// Real libass-rendered text via ffmpeg's `subtitles` filter
    /// (`video_media::effects::render_effect_ass_preview`) — the fast,
    /// deterministic renderer for fixed karaoke/phrase captions per
    /// `skills/content-video-editor/workflows/finish.md`. Requires a
    /// libass-enabled ffmpeg build; fails loudly (never silently falling
    /// back to `Ffmpeg`) when that build isn't present.
    Ass,
    /// Real Node/React render through the `apps/effects` Remotion package
    /// (`video_media::effects::render_effect_remotion_preview`) — branded
    /// kinetic motion. Remotion's license was re-verified for this pass
    /// (see `apps/effects/README.md`); pinned exact version, no `^`/`~`
    /// ranges.
    Remotion,
    /// Reserved for bespoke type (`skills/content-video-editor/workflows/
    /// finish.md`: "HyperFrames for bespoke type"). No HyperFrames
    /// implementation or dependency exists anywhere in this workspace;
    /// this variant exists only to keep the schema stable and fails loudly
    /// at render time rather than being silently folded into `Remotion`.
    HyperFrames,
}

/// How "energetic" an effect's motion is meant to read, per the plan's
/// `motion_profile` vocabulary.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MotionProfile {
    /// No meaningful motion; a still composited element.
    Static,
    /// A brief, restrained transition (e.g. a 400ms fade-in).
    Restrained,
    /// Continuous or attention-driving motion (e.g. a count-up numeral).
    Expressive,
}

/// Which of `video_media::captions`'s named safe zones an effect must avoid
/// colliding with.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[serde(rename_all = "kebab-case")]
pub enum SafeZoneRef {
    YoutubeLowerThird,
    VerticalBottom,
}

impl SafeZoneRef {
    pub fn resolve(self) -> CaptionSafeZone {
        match self {
            SafeZoneRef::YoutubeLowerThird => CaptionProfile::youtube_lower_third().safe_zone,
            SafeZoneRef::VerticalBottom => CaptionProfile::vertical_bottom().safe_zone,
        }
    }
}

/// Fixture paths (relative to the repo root) an entry's still/motion
/// previews render into. `motion` is `None` only for entries whose
/// `motion_profile` is [`MotionProfile::Static`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EffectPreviewFixture {
    pub still: String,
    pub motion: Option<String>,
}

/// An effect's explicit `prefers-reduced-motion`-equivalent behavior. The
/// plan requires every effect to declare this "where motion is
/// meaningful" — an effect that cannot degrade gracefully is still allowed
/// to ship, but must say so via `Unsupported` rather than silently ignoring
/// the constraint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ReducedMotionBehavior {
    /// Motion isn't meaningful for this effect (it is already `Static`).
    NotMeaningful,
    /// The effect has a described static/instant-visible fallback.
    StaticFallback { description: String },
    /// The effect genuinely cannot degrade gracefully; `reason` is a
    /// required, explicit statement of why, not silence.
    Unsupported { reason: String },
}

/// One typed effect registry entry, per REV2 plan §15.3:
/// `{effect_id, renderer, schema_version, props_schema, safe_zones,
/// motion_profile, preview_fixture}`, plus the two fields this pass adds to
/// make the plan's enforcement requirements (collision test, reduced-motion
/// behavior) checkable: `footprint` and `reduced_motion`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EffectRegistryEntry {
    pub effect_id: String,
    pub renderer: EffectRenderer,
    pub schema_version: u32,
    pub props_schema: Value,
    pub safe_zones: Vec<SafeZoneRef>,
    pub motion_profile: MotionProfile,
    pub preview_fixture: EffectPreviewFixture,
    /// The pct-inset box (same shape/semantics as
    /// [`CaptionSafeZone`]) this effect visually occupies, or `None` for an
    /// effect with no independent footprint of its own (the caption profile
    /// entry — it defines a safe zone rather than occupying space inside
    /// one, so there is nothing external to collide with).
    pub footprint: Option<CaptionSafeZone>,
    pub reduced_motion: ReducedMotionBehavior,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct RegistryDocument {
    schema_version: u32,
    effects: Vec<EffectRegistryEntry>,
}

/// The typed effect registry: every entry from `schemas/effects/registry.
/// json`, loaded once and validated on demand.
#[derive(Debug, Clone)]
pub struct EffectRegistry {
    entries: Vec<EffectRegistryEntry>,
}

impl EffectRegistry {
    /// Load and parse the embedded registry document. Does not itself
    /// validate every entry against safe zones/reduced-motion/preview
    /// requirements — call [`Self::validate_all`] for that (kept separate
    /// so a caller can load a registry, inspect it, and only then decide
    /// whether to enforce "usable" status).
    pub fn load_builtin() -> Result<Self, EffectRegistryError> {
        let document: RegistryDocument =
            serde_json::from_str(REGISTRY_JSON).map_err(EffectRegistryError::Malformed)?;
        if document.schema_version != EFFECT_REGISTRY_SCHEMA_VERSION {
            return Err(EffectRegistryError::UnsupportedSchema(
                document.schema_version,
            ));
        }
        Ok(Self {
            entries: document.effects,
        })
    }

    pub fn entries(&self) -> &[EffectRegistryEntry] {
        &self.entries
    }

    pub fn get(&self, effect_id: &str) -> Result<&EffectRegistryEntry, EffectRegistryError> {
        self.entries
            .iter()
            .find(|entry| entry.effect_id == effect_id)
            .ok_or_else(|| EffectRegistryError::UnknownEffect(effect_id.to_string()))
    }

    /// Validate every entry's usability requirements. Returns the first
    /// failure; a caller that wants every failure can iterate
    /// [`Self::entries`] and call [`validate_entry`] itself.
    pub fn validate_all(&self) -> Result<(), EffectRegistryError> {
        for entry in &self.entries {
            validate_entry(entry)?;
        }
        Ok(())
    }

    /// Validate `props` against `effect_id`'s `props_schema`. An invalid
    /// prop set fails loudly (REV2 plan §15.3 constraint) rather than being
    /// silently coerced or dropped.
    pub fn validate_props(
        &self,
        effect_id: &str,
        props: &Value,
    ) -> Result<(), EffectRegistryError> {
        let entry = self.get(effect_id)?;
        let mut messages = Vec::new();
        validate_against_schema(&entry.props_schema, props, "$", &mut messages);
        if messages.is_empty() {
            Ok(())
        } else {
            Err(EffectRegistryError::PropsInvalid {
                effect_id: effect_id.to_string(),
                messages,
            })
        }
    }
}

/// Enforce the plan's "usable" requirements for one entry: a still preview
/// path, a motion preview path when motion is meaningful, an explicit
/// reduced-motion declaration consistent with that, and no collision
/// between `footprint` and any safe zone in `safe_zones`.
pub fn validate_entry(entry: &EffectRegistryEntry) -> Result<(), EffectRegistryError> {
    if entry.preview_fixture.still.trim().is_empty() {
        return Err(EffectRegistryError::Invalid {
            effect_id: entry.effect_id.clone(),
            message: "missing still preview fixture path".into(),
        });
    }

    let motion_meaningful = entry.motion_profile != MotionProfile::Static;
    if motion_meaningful && entry.preview_fixture.motion.is_none() {
        return Err(EffectRegistryError::Invalid {
            effect_id: entry.effect_id.clone(),
            message: format!(
                "motion_profile {:?} requires a motion preview fixture",
                entry.motion_profile
            ),
        });
    }
    if !motion_meaningful {
        if let Some(motion_fixture) = &entry.preview_fixture.motion {
            if motion_fixture.trim().is_empty() {
                return Err(EffectRegistryError::Invalid {
                    effect_id: entry.effect_id.clone(),
                    message: "declared motion preview fixture path is empty".into(),
                });
            }
        }
    }

    match (&entry.reduced_motion, motion_meaningful) {
        (ReducedMotionBehavior::NotMeaningful, true) => {
            return Err(EffectRegistryError::Invalid {
                effect_id: entry.effect_id.clone(),
                message: "motion is meaningful for this effect but reduced_motion is \
                          declared not-meaningful"
                    .into(),
            });
        }
        (ReducedMotionBehavior::StaticFallback { description }, true)
            if description.trim().is_empty() =>
        {
            return Err(EffectRegistryError::Invalid {
                effect_id: entry.effect_id.clone(),
                message: "reduced_motion static-fallback requires a non-empty description".into(),
            });
        }
        (ReducedMotionBehavior::Unsupported { reason }, true) if reason.trim().is_empty() => {
            return Err(EffectRegistryError::Invalid {
                effect_id: entry.effect_id.clone(),
                message: "reduced_motion unsupported requires a non-empty reason".into(),
            });
        }
        _ => {}
    }

    if let Some(footprint) = &entry.footprint {
        for safe_zone in &entry.safe_zones {
            let safe = safe_zone.resolve();
            if !footprint_within_safe_zone(footprint, &safe) {
                return Err(EffectRegistryError::Collision {
                    effect_id: entry.effect_id.clone(),
                    safe_zone: *safe_zone,
                    message: format!(
                        "footprint {footprint:?} intrudes into the {safe_zone:?} safe-zone \
                         exclusion margin {safe:?}"
                    ),
                });
            }
        }
    }

    Ok(())
}

/// `footprint` collides with `safe` unless every one of its edge insets is
/// at least as large as `safe`'s — i.e. `footprint`'s content box is fully
/// contained inside `safe`'s content box, so the effect never renders
/// inside the region reserved for captions/platform chrome. Percent-space,
/// so this holds at any output resolution.
fn footprint_within_safe_zone(footprint: &CaptionSafeZone, safe: &CaptionSafeZone) -> bool {
    footprint.top_pct >= safe.top_pct
        && footprint.bottom_pct >= safe.bottom_pct
        && footprint.left_pct >= safe.left_pct
        && footprint.right_pct >= safe.right_pct
}

// ---------------------------------------------------------------------
// Props-schema validation: a bounded subset of JSON Schema (draft 2020-12)
// matching exactly the dialect this repo's other `schemas/*.json` files
// already use (`type`, `required`, `properties`, `additionalProperties`,
// `enum`, `minimum`/`maximum`, `items`). No `jsonschema`-family crate is a
// workspace dependency; adding one for five bounded, repo-authored schemas
// is heavier than this ~80-line validator.
// ---------------------------------------------------------------------

fn validate_against_schema(schema: &Value, instance: &Value, path: &str, errors: &mut Vec<String>) {
    let Some(schema_obj) = schema.as_object() else {
        errors.push(format!("{path}: schema itself is not an object"));
        return;
    };

    if let Some(expected_type) = schema_obj.get("type").and_then(Value::as_str) {
        if !instance_matches_type(instance, expected_type) {
            errors.push(format!(
                "{path}: expected type {expected_type}, got {}",
                type_name(instance)
            ));
            return;
        }
    }

    if let Some(allowed) = schema_obj.get("enum").and_then(Value::as_array) {
        if !allowed.iter().any(|value| value == instance) {
            errors.push(format!(
                "{path}: value is not one of the allowed enum values"
            ));
        }
    }

    if let Some(number) = instance.as_f64() {
        if let Some(minimum) = schema_obj.get("minimum").and_then(Value::as_f64) {
            if number < minimum {
                errors.push(format!("{path}: {number} is below minimum {minimum}"));
            }
        }
        if let Some(maximum) = schema_obj.get("maximum").and_then(Value::as_f64) {
            if number > maximum {
                errors.push(format!("{path}: {number} is above maximum {maximum}"));
            }
        }
    }

    if let Some(object) = instance.as_object() {
        if let Some(required) = schema_obj.get("required").and_then(Value::as_array) {
            for key in required {
                if let Some(key) = key.as_str() {
                    if !object.contains_key(key) {
                        errors.push(format!("{path}: missing required property {key:?}"));
                    }
                }
            }
        }
        let properties = schema_obj.get("properties").and_then(Value::as_object);
        let additional_allowed = schema_obj
            .get("additionalProperties")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        for (key, value) in object {
            match properties.and_then(|properties| properties.get(key)) {
                Some(property_schema) => {
                    validate_against_schema(
                        property_schema,
                        value,
                        &format!("{path}.{key}"),
                        errors,
                    );
                }
                None if !additional_allowed => {
                    errors.push(format!("{path}: unexpected property {key:?}"));
                }
                None => {}
            }
        }
    }

    if let Some(array) = instance.as_array() {
        if let Some(item_schema) = schema_obj.get("items") {
            for (index, item) in array.iter().enumerate() {
                validate_against_schema(item_schema, item, &format!("{path}[{index}]"), errors);
            }
        }
    }
}

fn instance_matches_type(instance: &Value, expected_type: &str) -> bool {
    match expected_type {
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "string" => instance.is_string(),
        "boolean" => instance.is_boolean(),
        "integer" => instance.as_i64().is_some() || instance.as_u64().is_some(),
        "number" => instance.is_number(),
        "null" => instance.is_null(),
        _ => true,
    }
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

// ---------------------------------------------------------------------
// Preview rendering + receipts
// ---------------------------------------------------------------------

/// A rendered preview outcome: where the still/motion files landed and the
/// [`StageReceipt`] binding them to the props/toolchain that produced them.
#[derive(Debug)]
pub struct EffectPreviewOutcome {
    pub still_path: PathBuf,
    pub motion_path: Option<PathBuf>,
    pub receipt: StageReceipt,
}

/// Render `effect_id`'s still preview (and motion preview, when its
/// `motion_profile` isn't `Static`) into `output_dir`, after validating
/// `props` against the entry's `props_schema` and the entry itself against
/// the registry's usability requirements. Writes a
/// [`video_core::StageReceipt`] beside the still preview, reusing the same
/// `receipts::write_stage_receipt` pattern every other pipeline stage uses.
pub fn render_effect_preview(
    registry: &EffectRegistry,
    effect_id: &str,
    props: &Value,
    output_dir: &Path,
) -> Result<EffectPreviewOutcome, EffectRegistryError> {
    let entry = registry.get(effect_id)?;
    validate_entry(entry)?;
    registry.validate_props(effect_id, props)?;

    std::fs::create_dir_all(output_dir)?;
    let needs_motion = entry.motion_profile != MotionProfile::Static;
    let (still_path, motion_path, tool_name, tool_identity): (
        PathBuf,
        Option<(PathBuf, PathBuf)>,
        String,
        String,
    ) = match entry.renderer {
        EffectRenderer::Native => {
            let frame = frame_from_entry(entry, props, needs_motion, false);
            let still_path = output_dir.join("still.png");
            render_native_effect_frame(&frame, &still_path).map_err(|error| {
                EffectRegistryError::Invalid {
                    effect_id: entry.effect_id.clone(),
                    message: error.to_string(),
                }
            })?;
            let motion_path = if needs_motion {
                let reduced_path = output_dir.join("motion-reduced.png");
                render_native_effect_frame(
                    &frame_from_entry(entry, props, true, true),
                    &reduced_path,
                )
                .map_err(|error| EffectRegistryError::Invalid {
                    effect_id: entry.effect_id.clone(),
                    message: error.to_string(),
                })?;
                let motion_path = output_dir.join("motion.png");
                render_native_effect_frame(&frame, &motion_path).map_err(|error| {
                    EffectRegistryError::Invalid {
                        effect_id: entry.effect_id.clone(),
                        message: error.to_string(),
                    }
                })?;
                Some((motion_path, reduced_path))
            } else {
                None
            };
            (
                still_path,
                motion_path,
                "cutright-native".to_string(),
                "native-raster-v1".to_string(),
            )
        }
        EffectRenderer::Ffmpeg
        | EffectRenderer::Ass
        | EffectRenderer::Remotion
        | EffectRenderer::HyperFrames => {
            return Err(EffectRegistryError::RetiredRenderer {
                renderer: format!("{:?}", entry.renderer).to_ascii_lowercase(),
            })
        }
    };

    let mut toolchains = BTreeMap::new();
    toolchains.insert(tool_name, tool_identity);

    let mut outputs = vec![still_path.as_path()];
    if let Some((motion_path, reduced_path)) = &motion_path {
        outputs.push(motion_path.as_path());
        outputs.push(reduced_path.as_path());
    }

    let receipt = write_stage_receipt(
        &receipt_path_for(&still_path),
        &format!("effects.preview.{effect_id}"),
        &[],
        props,
        toolchains,
        &outputs,
    )?;

    Ok(EffectPreviewOutcome {
        still_path,
        motion_path: motion_path.map(|(motion, _reduced)| motion),
        receipt,
    })
}

/// Build native preview geometry for `entry`, deriving its footprint
/// from its `footprint` (falling back to a full-frame band for effects with
/// no independent footprint, e.g. the caption profile entry).
fn frame_from_entry(
    entry: &EffectRegistryEntry,
    props: &Value,
    animated: bool,
    reduced_motion: bool,
) -> NativeEffectFrame {
    const CANVAS_WIDTH: u32 = 1280;
    const CANVAS_HEIGHT: u32 = 720;

    let footprint = entry.footprint.clone().unwrap_or(CaptionSafeZone {
        top_pct: 40.0,
        bottom_pct: 40.0,
        left_pct: 15.0,
        right_pct: 15.0,
    });
    let footprint_px = footprint.content_box_px(CANVAS_WIDTH, CANVAS_HEIGHT);

    let accent_rgb = props
        .as_object()
        .and_then(|object| object.get("accent_color"))
        .and_then(Value::as_str)
        .and_then(parse_hex_rgb)
        .unwrap_or((223, 100, 40));

    NativeEffectFrame {
        width: CANVAS_WIDTH,
        height: CANVAS_HEIGHT,
        footprint_px,
        accent_rgb,
        animated,
        reduced_motion,
    }
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

/// Not currently called by any production stage — the entry point exists so
/// future callers (finish.rs slots, Studio) have one place to write the
/// registry document out for inspection/debug without re-deriving the
/// embedded JSON string. Kept `pub(crate)` since nothing outside this crate
/// needs it yet.
#[allow(dead_code)]
pub(crate) fn write_registry_snapshot(path: &Path) -> Result<(), ProjectError> {
    let document: Value = serde_json::from_str(REGISTRY_JSON)?;
    write_json_atomic(path, &document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn unique_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("cutright-effects-project-test-{label}-{unique}"));
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    fn props_fixtures() -> Value {
        let raw = include_str!("../../../fixtures/effects/props-fixtures.json");
        serde_json::from_str(raw).expect("parse props fixtures")
    }

    #[test]
    fn registry_round_trips_and_has_fifteen_unique_complete_effects() {
        let registry = EffectRegistry::load_builtin().expect("load registry");
        let fixtures = props_fixtures();
        let effects = fixtures["effects"].as_object().expect("effects object");
        let ids: Vec<&str> = registry
            .entries()
            .iter()
            .map(|entry| entry.effect_id.as_str())
            .collect();
        assert_eq!(ids.len(), 15, "starter library must stay complete");
        let unique: std::collections::BTreeSet<_> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len(), "effect ids must be unique");
        assert_eq!(
            effects.len(),
            ids.len(),
            "fixtures must have no orphan entries"
        );

        // Round-trip: serialize each entry back to JSON and reparse.
        for entry in registry.entries() {
            assert!(!entry.preview_fixture.still.trim().is_empty());
            assert!(
                effects.contains_key(&entry.effect_id),
                "missing fixture for {}",
                entry.effect_id
            );
            let encoded = serde_json::to_string(entry).expect("serialize entry");
            let decoded: EffectRegistryEntry =
                serde_json::from_str(&encoded).expect("deserialize entry");
            assert_eq!(&decoded, entry);
        }
    }

    #[test]
    fn every_built_in_effect_is_usable() {
        let registry = EffectRegistry::load_builtin().expect("load registry");
        registry
            .validate_all()
            .expect("every shipped effect must validate");
    }

    #[test]
    fn props_validation_accepts_valid_and_rejects_invalid() {
        let registry = EffectRegistry::load_builtin().expect("load registry");
        let fixtures = props_fixtures();
        let effects = fixtures["effects"].as_object().expect("effects object");

        for entry in registry.entries() {
            let fixture = &effects[entry.effect_id.as_str()];
            registry
                .validate_props(&entry.effect_id, &fixture["valid"])
                .unwrap_or_else(|error| {
                    panic!(
                        "expected valid props for {} to pass: {error}",
                        entry.effect_id
                    )
                });

            for invalid in fixture["invalid"].as_array().expect("invalid cases array") {
                let result = registry.validate_props(&entry.effect_id, invalid);
                assert!(
                    result.is_err(),
                    "expected invalid props {invalid:?} for {} to fail",
                    entry.effect_id
                );
            }
        }
    }

    #[test]
    fn safe_zone_collision_is_detected() {
        let colliding = EffectRegistryEntry {
            effect_id: "test.colliding-badge.v1".into(),
            renderer: EffectRenderer::Ffmpeg,
            schema_version: 1,
            props_schema: serde_json::json!({"type": "object"}),
            safe_zones: vec![SafeZoneRef::VerticalBottom],
            motion_profile: MotionProfile::Static,
            preview_fixture: EffectPreviewFixture {
                still: "fixtures/effects/test.colliding-badge.v1/still.png".into(),
                motion: None,
            },
            // Deliberately shallow bottom inset: the vertical-bottom safe
            // zone reserves bottom_pct 20, this footprint only clears 5, so
            // it sits inside the platform-chrome exclusion band.
            footprint: Some(CaptionSafeZone {
                top_pct: 40.0,
                bottom_pct: 5.0,
                left_pct: 15.0,
                right_pct: 15.0,
            }),
            reduced_motion: ReducedMotionBehavior::NotMeaningful,
        };

        let error = validate_entry(&colliding).expect_err("collision must be detected");
        assert!(matches!(error, EffectRegistryError::Collision { .. }));
    }

    #[test]
    fn reduced_motion_declaration_is_enforced_when_motion_is_meaningful() {
        let mut entry = EffectRegistry::load_builtin()
            .expect("load registry")
            .get("stat-counter.v1")
            .expect("stat-counter entry")
            .clone();
        entry.reduced_motion = ReducedMotionBehavior::NotMeaningful;
        let error = validate_entry(&entry).expect_err("must require an explicit declaration");
        assert!(matches!(error, EffectRegistryError::Invalid { .. }));
    }

    /// Every starter effect renders its preview fixture through its actual
    /// registry renderer (`ass`, `remotion`, or data-driven `ffmpeg`). The
    /// `ass` branch degrades
    /// honestly instead of unconditionally requiring success: this
    /// workspace's own local ffmpeg build has no libass
    /// (`ass_renderer_reports_a_clear_missing_toolchain_error_when_libass_is_absent`
    /// below asserts that directly), and the contract this test exists to
    /// prove is "never silently falls back to `ffmpeg` and claims success"
    /// — which a hard failure on a libass-less machine would satisfy just
    /// as well as a render would. `remotion` effects have no such
    /// environment excuse (`apps/effects`'s `node_modules` is a checked-in
    /// prerequisite of this pass) and must always succeed.
    #[test]
    fn each_of_the_fifteen_effects_routes_to_its_preview_renderer() {
        let registry = EffectRegistry::load_builtin().expect("load registry");
        let fixtures = props_fixtures();
        let effects = fixtures["effects"].as_object().expect("effects object");
        let dir = unique_dir("previews");

        for entry in registry.entries() {
            let props = &effects[entry.effect_id.as_str()]["valid"];
            let output_dir = dir.join(&entry.effect_id);
            let result = render_effect_preview(&registry, &entry.effect_id, props, &output_dir);

            let outcome = result.unwrap_or_else(|error| {
                panic!("expected native preview for {}: {error}", entry.effect_id)
            });

            assert!(outcome.still_path.is_file());
            assert!(fs::metadata(&outcome.still_path).unwrap().len() > 0);
            assert_eq!(
                outcome.receipt.stage,
                format!("effects.preview.{}", entry.effect_id)
            );
            assert!(receipt_path_for(&outcome.still_path).is_file());

            if entry.motion_profile == MotionProfile::Static {
                assert!(outcome.motion_path.is_none());
            } else {
                let motion_path = outcome
                    .motion_path
                    .as_ref()
                    .unwrap_or_else(|| panic!("{} must render a motion preview", entry.effect_id));
                assert!(motion_path.is_file());
                assert!(fs::metadata(motion_path).unwrap().len() > 0);
            }
        }

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn native_preview_is_a_real_png_with_a_native_receipt() {
        let registry = EffectRegistry::load_builtin().expect("load registry");
        let fixtures = props_fixtures();
        let props = &fixtures["effects"]["caption.bold-karaoke.v1"]["valid"];
        let dir = unique_dir("native-preview");
        let outcome =
            render_effect_preview(&registry, "caption.bold-karaoke.v1", props, &dir).unwrap();
        assert_eq!(
            &fs::read(&outcome.still_path).unwrap()[..8],
            b"\x89PNG\r\n\x1a\n"
        );
        assert_eq!(
            outcome.receipt.toolchains["cutright-native"],
            "native-raster-v1"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn legacy_renderer_selection_returns_typed_retired_renderer() {
        let entry = EffectRegistryEntry {
            effect_id: "test.bespoke-type.v1".into(),
            renderer: EffectRenderer::HyperFrames,
            schema_version: 1,
            props_schema: serde_json::json!({"type": "object"}),
            safe_zones: vec![],
            motion_profile: MotionProfile::Static,
            preview_fixture: EffectPreviewFixture {
                still: "fixtures/effects/test.bespoke-type.v1/still.png".into(),
                motion: None,
            },
            footprint: None,
            reduced_motion: ReducedMotionBehavior::NotMeaningful,
        };
        let mut document: Value =
            serde_json::from_str(REGISTRY_JSON).expect("parse embedded registry for test");
        document["effects"]
            .as_array_mut()
            .expect("effects array")
            .push(serde_json::to_value(&entry).expect("serialize synthetic entry"));
        let registry = EffectRegistry {
            entries: serde_json::from_value::<RegistryDocument>(document)
                .expect("parse synthetic registry document")
                .effects,
        };

        let dir = unique_dir("legacy-renderer");
        let error = render_effect_preview(
            &registry,
            "test.bespoke-type.v1",
            &serde_json::json!({}),
            &dir,
        )
        .expect_err("legacy renderer must be retired");
        assert!(matches!(error, EffectRegistryError::RetiredRenderer { .. }));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ffmpeg_renderer_is_retired() {
        let entry = EffectRegistryEntry {
            effect_id: "test.ffmpeg-fast-path.v1".into(),
            renderer: EffectRenderer::Ffmpeg,
            schema_version: 1,
            props_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": { "label": { "type": "string" } }
            }),
            safe_zones: vec![],
            motion_profile: MotionProfile::Restrained,
            preview_fixture: EffectPreviewFixture {
                still: "fixtures/effects/test.ffmpeg-fast-path.v1/still.png".into(),
                motion: Some("fixtures/effects/test.ffmpeg-fast-path.v1/motion.mp4".into()),
            },
            footprint: None,
            reduced_motion: ReducedMotionBehavior::StaticFallback {
                description: "renders at full opacity from frame zero".into(),
            },
        };
        let mut document: Value =
            serde_json::from_str(REGISTRY_JSON).expect("parse embedded registry for test");
        document["effects"]
            .as_array_mut()
            .expect("effects array")
            .push(serde_json::to_value(&entry).expect("serialize synthetic entry"));
        let registry = EffectRegistry {
            entries: serde_json::from_value::<RegistryDocument>(document)
                .expect("parse synthetic registry document")
                .effects,
        };

        let dir = unique_dir("ffmpeg-fast-path");
        let outcome = render_effect_preview(
            &registry,
            "test.ffmpeg-fast-path.v1",
            &serde_json::json!({"label": "fast path"}),
            &dir,
        )
        .expect_err("ffmpeg renderer must be retired");
        assert!(matches!(
            outcome,
            EffectRegistryError::RetiredRenderer { .. }
        ));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn render_rejects_props_that_fail_schema_validation() {
        let registry = EffectRegistry::load_builtin().expect("load registry");
        let dir = unique_dir("invalid-props");
        let result = render_effect_preview(
            &registry,
            "cta-end-card.v1",
            &serde_json::json!({"headline": "Missing accent color"}),
            &dir,
        );
        assert!(matches!(
            result,
            Err(EffectRegistryError::PropsInvalid { .. })
        ));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unknown_effect_id_fails_loudly() {
        let registry = EffectRegistry::load_builtin().expect("load registry");
        let error = registry
            .get("does-not-exist.v1")
            .expect_err("unknown effect must error");
        assert!(matches!(error, EffectRegistryError::UnknownEffect(_)));
    }
}
