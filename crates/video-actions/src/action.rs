//! Typed [`Action`] enum, opaque [`TargetRef`] stable IDs, and per-variant
//! typed parameter structs that reject unknown fields (CR-V2-B2-007).
//!
//! Every variant mirrors a family declared in
//! `schemas/actions/semantic-diff.schema.v1.json` (cut, restore, move,
//! take_swap, retime, caption, graphic, audio, colour, export, setting).
//! All IDs and kinds match the patterns frozen by `V2-IDENTITY-TIME-REVISION.md`
//! (`^[A-Za-z0-9_-]+$` for IDs, `^[a-z][a-z0-9_.]+$` for action kinds) and the
//! schema envelope in `V2-CAPABILITY-ACTION-CONTRACT.md` (snake_case JSON with
//! unknown fields failing closed).
//!
//! All variants serialize as `{ "kind": "<dotted>", "target": "<id>", "params":
//! { ... } }`. Unknown action kinds deserialize to [`ParamError::UnknownActionKind`];
//! unknown JSON fields on any variant or params struct fail deserialization.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The reserved, opaque stable-id kinds from `schemas/core/identity.schema.v1.json`.
///
/// `Action::target` must be drawn from one of these kinds. We use this enum
/// purely as a sanity check at the action boundary — the wire format itself
/// just carries the raw string.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TargetKind {
    /// Project root identity.
    Project,
    /// Timeline identity.
    Timeline,
    /// Track within a timeline.
    Track,
    /// Clip on a track.
    Clip,
    /// Word inside a transcript bound to a clip.
    Word,
    /// Evidence node reference.
    EvidenceNode,
    /// Action batch identity (rarely used as a target, but reserved).
    ActionBatch,
    /// Job identity.
    Job,
    /// Media asset identity.
    Asset,
}

impl TargetKind {
    /// Parse a target kind from its wire string. Returns `None` for unknown kinds.
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "project" => Some(Self::Project),
            "timeline" => Some(Self::Timeline),
            "track" => Some(Self::Track),
            "clip" => Some(Self::Clip),
            "word" => Some(Self::Word),
            "evidence_node" => Some(Self::EvidenceNode),
            "action_batch" => Some(Self::ActionBatch),
            "job" => Some(Self::Job),
            "asset" => Some(Self::Asset),
            _ => None,
        }
    }

    /// Stable wire string for this kind.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Timeline => "timeline",
            Self::Track => "track",
            Self::Clip => "clip",
            Self::Word => "word",
            Self::EvidenceNode => "evidence_node",
            Self::ActionBatch => "action_batch",
            Self::Job => "job",
            Self::Asset => "asset",
        }
    }
}

/// Error type for [`TargetRef`] construction and validation.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TargetRefError {
    /// The raw value does not match the frozen `^[A-Za-z0-9_-]+$` pattern.
    #[error("target id {0:?} does not match ^[A-Za-z0-9_-]+$")]
    InvalidFormat(String),
    /// The raw value was empty after stripping the kind prefix.
    #[error("target id {0:?} is empty after the kind prefix")]
    EmptyLocal(String),
}

/// Opaque wrapper around a stable, schema-validated target id.
///
/// The id is "opaque" in the sense that callers MUST NOT infer identity from
/// names, paths, display labels, or array indexes (`V2-IDENTITY-TIME-REVISION.md`
/// §1). The wire format carries `<kind>:<local>` so a `TargetRef` can be
/// validated against [`TargetKind`] without losing its local string.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct TargetRef(String);

impl TargetRef {
    /// Construct a `TargetRef` from a raw wire string. The string MUST be of
    /// the form `<kind>:<local>` where:
    /// - `<kind>` is one of the reserved [`TargetKind`] wire strings.
    /// - `<local>` matches `^[A-Za-z0-9_-]+$` and is non-empty.
    pub fn new(raw: impl Into<String>) -> Result<Self, TargetRefError> {
        let value = raw.into();
        let (kind, local) = value
            .split_once(':')
            .ok_or_else(|| TargetRefError::InvalidFormat(value.clone()))?;
        if TargetKind::parse(kind).is_none() {
            return Err(TargetRefError::InvalidFormat(value));
        }
        if local.is_empty() || !is_valid_local(local) {
            return Err(if local.is_empty() {
                TargetRefError::EmptyLocal(value)
            } else {
                TargetRefError::InvalidFormat(value)
            });
        }
        Ok(Self(value))
    }

    /// Construct a `TargetRef` from an explicit kind and local string.
    pub fn from_parts(kind: TargetKind, local: impl Into<String>) -> Result<Self, TargetRefError> {
        let local = local.into();
        if local.is_empty() || !is_valid_local(&local) {
            return Err(if local.is_empty() {
                TargetRefError::EmptyLocal(format!("{}:", kind.as_str()))
            } else {
                TargetRefError::InvalidFormat(format!("{}:{}", kind.as_str(), local))
            });
        }
        Ok(Self(format!("{}:{}", kind.as_str(), local)))
    }

    /// The kind component of the target id.
    pub fn kind(&self) -> TargetKind {
        // SAFETY: `TargetRef::new` validated the prefix kind.
        let kind_str = self
            .0
            .split_once(':')
            .map(|(k, _)| k)
            .unwrap_or("");
        TargetKind::parse(kind_str).expect("TargetRef invariant: validated kind prefix")
    }

    /// The local id portion (after the `:` separator).
    pub fn local(&self) -> &str {
        // SAFETY: `TargetRef::new` validated the suffix as non-empty.
        self.0
            .split_once(':')
            .map(|(_, l)| l)
            .unwrap_or("")
    }

    /// Borrow the full wire string (e.g. `"clip:clip_5"`).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TargetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn is_valid_local(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Typed error returned when an action payload cannot be deserialised or
/// validated against the frozen action vocabulary.
#[derive(Debug, Error)]
pub enum ParamError {
    /// The `kind` field was missing or empty.
    #[error("action kind is missing or empty")]
    MissingKind,
    /// The `kind` field did not match the frozen vocabulary.
    #[error("unknown action kind {0:?}")]
    UnknownActionKind(String),
    /// The `target` field was missing or empty.
    #[error("action target is missing or empty")]
    MissingTarget,
    /// The `target` field was not a valid [`TargetRef`].
    #[error("invalid action target: {0}")]
    InvalidTarget(#[source] TargetRefError),
    /// The `params` object contained an unknown field for the chosen variant.
    #[error("unknown field {field:?} in params for {kind}")]
    UnknownField {
        /// The action kind whose params object held the unknown field.
        kind: &'static str,
        /// The offending field name.
        field: String,
    },
    /// Generic JSON parse failure bubbled up from serde.
    #[error("action JSON parse failed: {0}")]
    Json(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// Per-variant parameter structs.
//
// Every struct is `#[serde(deny_unknown_fields)]` so unknown keys fail
// closed (V2-CAPABILITY-ACTION-CONTRACT.md §3 "Unknown fields fail closed").
// ---------------------------------------------------------------------------

/// Rational-tick half-open range `[start_ns, end_ns)` measured in integer
/// nanoseconds. `end_ns > start_ns` is required for cut-style actions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RangeNs {
    /// Inclusive start of the range in nanoseconds. Must be `>= 0`.
    pub start_ns: i64,
    /// Exclusive end of the range in nanoseconds. Must be `> start_ns`.
    pub end_ns: i64,
}

impl RangeNs {
    /// Length of the range in nanoseconds. Panics on out-of-order ranges.
    pub fn len_ns(&self) -> i64 {
        self.end_ns - self.start_ns
    }

    /// `true` iff `end_ns > start_ns` and both are non-negative.
    pub fn is_valid(&self) -> bool {
        self.start_ns >= 0 && self.end_ns > self.start_ns
    }
}

/// Params for `timeline.cut` — remove a clip range from the active timeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CutParams {
    /// Range to remove.
    pub range: RangeNs,
    /// Optional human reason recorded in the receipt and inverse batch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Params for `timeline.restore` — put a previously cut range back.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RestoreParams {
    /// Range to restore, identical to the cut range.
    pub range: RangeNs,
    /// Reference to the original action batch the restore undoes.
    pub source_batch_id: String,
}

/// Params for `timeline.move` — move a clip range to a new timeline position.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MoveParams {
    /// Source range to move.
    pub range: RangeNs,
    /// New start position (timeline-relative nanoseconds).
    pub new_start_ns: i64,
}

/// Params for `take.swap` — swap a clip range for an alternative take.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TakeSwapParams {
    /// Range to swap.
    pub range: RangeNs,
    /// Stable id of the alternative take clip.
    pub replacement_clip_id: String,
}

/// Params for `track.retime` — change playback speed of a track range.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RetimeParams {
    /// Range to retime.
    pub range: RangeNs,
    /// Rational speed (numerator, denominator, both `u64`, both non-zero).
    /// Rational speed numerator (must be non-zero).
    pub speed_num: u64,
    /// Rational speed denominator (must be non-zero).
    pub speed_den: u64,
}

/// Params for `caption.edit` — edit a caption segment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CaptionParams {
    /// Range the caption covers.
    pub range: RangeNs,
    /// Replacement text for the caption.
    pub text: String,
}

/// Params for `graphic.edit` — edit a graphic overlay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GraphicParams {
    /// Range the graphic covers.
    pub range: RangeNs,
    /// Stable id of the graphic asset.
    pub graphic_id: String,
}

/// Params for `audio.edit` — edit an audio segment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AudioParams {
    /// Range the edit covers.
    pub range: RangeNs,
    /// Linear gain (1.0 = unity).
    pub gain: f64,
}

/// Params for `color.lut` — apply a LUT to a clip range.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ColourLutParams {
    /// Range the LUT applies to.
    pub range: RangeNs,
    /// Stable id of the LUT asset.
    pub lut_id: String,
}

/// Params for `color.correction` — primary colour correction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ColourCorrectionParams {
    /// Range the correction applies to.
    pub range: RangeNs,
    /// Exposure stops, clamped at validation time to a sane range.
    pub exposure_stops: f64,
    /// White-balance shift in Kelvin (signed).
    pub white_balance_kelvin: i64,
}

/// Params for `export.render` — render a deliverable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportRenderParams {
    /// Output preset id (matches an existing preset).
    pub preset_id: String,
    /// Optional target revision id; defaults to the active revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_revision: Option<String>,
}

/// Params for `setting.update` — update a project setting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SettingParams {
    /// Setting key (snake_case).
    pub key: String,
    /// Stringified setting value. Typed by the receiver.
    pub value: String,
}

// ---------------------------------------------------------------------------
// Action enum.
//
// The enum is internally tagged by `kind`. Every variant is named after the
// dotted kind string via `#[serde(rename = ...)]` so the wire form matches
// the schema exactly. Each variant carries a `target: TargetRef` and a typed
// `params: <VariantParams>`.
// ---------------------------------------------------------------------------

/// Typed v2 action. Mirrors every family declared in
/// `schemas/actions/semantic-diff.schema.v1.json`.
///
/// Each variant carries a strongly-typed `params` struct that rejects
/// unknown fields via `#[serde(deny_unknown_fields)]` so the wire format
/// fails closed on schema drift (`V2-CAPABILITY-ACTION-CONTRACT.md` §3).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Action {
    /// `timeline.cut` — remove a clip range.
    #[serde(rename = "timeline.cut")]
    Cut {
        /// Target clip id.
        target: TargetRef,
        /// Cut params.
        params: CutParams,
    },
    /// `timeline.restore` — restore a previously cut range.
    #[serde(rename = "timeline.restore")]
    Restore {
        /// Target clip id.
        target: TargetRef,
        /// Restore params.
        params: RestoreParams,
    },
    /// `timeline.move` — move a clip range.
    #[serde(rename = "timeline.move")]
    Move {
        /// Target clip id.
        target: TargetRef,
        /// Move params.
        params: MoveParams,
    },
    /// `take.swap` — swap a clip range for an alternative take.
    #[serde(rename = "take.swap")]
    TakeSwap {
        /// Target clip id.
        target: TargetRef,
        /// Take-swap params.
        params: TakeSwapParams,
    },
    /// `track.retime` — change track speed over a range.
    #[serde(rename = "track.retime")]
    Retime {
        /// Target track id.
        target: TargetRef,
        /// Retime params.
        params: RetimeParams,
    },
    /// `caption.edit` — edit a caption segment.
    #[serde(rename = "caption.edit")]
    Caption {
        /// Target caption id.
        target: TargetRef,
        /// Caption params.
        params: CaptionParams,
    },
    /// `graphic.edit` — edit a graphic overlay.
    #[serde(rename = "graphic.edit")]
    Graphic {
        /// Target graphic id.
        target: TargetRef,
        /// Graphic params.
        params: GraphicParams,
    },
    /// `audio.edit` — edit an audio segment.
    #[serde(rename = "audio.edit")]
    Audio {
        /// Target audio id.
        target: TargetRef,
        /// Audio params.
        params: AudioParams,
    },
    /// `color.lut` — apply a LUT to a clip range.
    #[serde(rename = "color.lut")]
    ColourLut {
        /// Target clip id.
        target: TargetRef,
        /// LUT params.
        params: ColourLutParams,
    },
    /// `color.correction` — primary colour correction.
    #[serde(rename = "color.correction")]
    ColourCorrection {
        /// Target clip id.
        target: TargetRef,
        /// Colour-correction params.
        params: ColourCorrectionParams,
    },
    /// `export.render` — render a deliverable.
    #[serde(rename = "export.render")]
    ExportRender {
        /// Target output preset id.
        target: TargetRef,
        /// Export-render params.
        params: ExportRenderParams,
    },
    /// `setting.update` — update a project setting.
    #[serde(rename = "setting.update")]
    Setting {
        /// Target setting id.
        target: TargetRef,
        /// Setting params.
        params: SettingParams,
    },
}

/// Frozen set of every supported action kind string.
///
/// Adding a new kind to [`Action`] MUST also add it here. Used by the
/// property test in `tests/action_kind_property.rs` (loaded from this module
/// via re-export).
pub const ACTION_KINDS: &[&str] = &[
    "timeline.cut",
    "timeline.restore",
    "timeline.move",
    "take.swap",
    "track.retime",
    "caption.edit",
    "graphic.edit",
    "audio.edit",
    "color.lut",
    "color.correction",
    "export.render",
    "setting.update",
];

/// Returns the wire kind string for any [`Action`] variant.
pub fn action_kind(action: &Action) -> &'static str {
    match action {
        Action::Cut { .. } => "timeline.cut",
        Action::Restore { .. } => "timeline.restore",
        Action::Move { .. } => "timeline.move",
        Action::TakeSwap { .. } => "take.swap",
        Action::Retime { .. } => "track.retime",
        Action::Caption { .. } => "caption.edit",
        Action::Graphic { .. } => "graphic.edit",
        Action::Audio { .. } => "audio.edit",
        Action::ColourLut { .. } => "color.lut",
        Action::ColourCorrection { .. } => "color.correction",
        Action::ExportRender { .. } => "export.render",
        Action::Setting { .. } => "setting.update",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_target() -> TargetRef {
        TargetRef::from_parts(TargetKind::Clip, "clip_5").unwrap()
    }

    #[test]
    fn target_ref_round_trips_through_transparent_serde() {
        let target = sample_target();
        let encoded = serde_json::to_string(&target).unwrap();
        assert_eq!(encoded, "\"clip:clip_5\"");
        let decoded: TargetRef = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, target);
    }

    #[test]
    fn target_ref_new_rejects_unknown_kind() {
        assert!(matches!(
            TargetRef::new("bogus:clip_5"),
            Err(TargetRefError::InvalidFormat(_))
        ));
    }

    #[test]
    fn target_ref_new_rejects_empty_local() {
        assert!(matches!(
            TargetRef::new("clip:"),
            Err(TargetRefError::EmptyLocal(_))
        ));
    }

    #[test]
    fn target_ref_new_rejects_missing_kind_separator() {
        assert!(matches!(
            TargetRef::new("clip_5"),
            Err(TargetRefError::InvalidFormat(_))
        ));
    }

    #[test]
    fn target_ref_new_rejects_disallowed_characters() {
        assert!(matches!(
            TargetRef::new("clip:bad id"),
            Err(TargetRefError::InvalidFormat(_))
        ));
    }

    #[test]
    fn target_ref_kind_and_local_split() {
        let target = TargetRef::new("track:track_main").unwrap();
        assert_eq!(target.kind(), TargetKind::Track);
        assert_eq!(target.local(), "track_main");
    }

    #[test]
    fn cut_action_round_trips() {
        let action = Action::Cut {
            target: sample_target(),
            params: CutParams {
                range: RangeNs {
                    start_ns: 1_200_000_000,
                    end_ns: 1_400_000_000,
                },
                reason: Some("filler um".into()),
            },
        };
        let value = serde_json::to_value(&action).unwrap();
        assert_eq!(value["kind"], "timeline.cut");
        assert_eq!(value["target"], "clip:clip_5");
        let decoded: Action = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, action);
    }

    #[test]
    fn cut_action_rejects_unknown_param_field() {
        let value = serde_json::json!({
            "kind": "timeline.cut",
            "target": "clip:clip_5",
            "params": {
                "range": { "start_ns": 0, "end_ns": 1 },
                "rogue": true,
            }
        });
        let err = serde_json::from_value::<Action>(value).unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn restore_action_round_trips() {
        let action = Action::Restore {
            target: sample_target(),
            params: RestoreParams {
                range: RangeNs {
                    start_ns: 0,
                    end_ns: 1_000,
                },
                source_batch_id: "batch_0001".into(),
            },
        };
        let decoded: Action = serde_json::from_value(serde_json::to_value(&action).unwrap()).unwrap();
        assert_eq!(decoded, action);
    }

    #[test]
    fn move_action_round_trips() {
        let action = Action::Move {
            target: sample_target(),
            params: MoveParams {
                range: RangeNs {
                    start_ns: 1_000,
                    end_ns: 2_000,
                },
                new_start_ns: 5_000,
            },
        };
        let decoded: Action = serde_json::from_value(serde_json::to_value(&action).unwrap()).unwrap();
        assert_eq!(decoded, action);
    }

    #[test]
    fn take_swap_action_round_trips() {
        let action = Action::TakeSwap {
            target: sample_target(),
            params: TakeSwapParams {
                range: RangeNs {
                    start_ns: 1_000,
                    end_ns: 2_000,
                },
                replacement_clip_id: "clip_alt_9".into(),
            },
        };
        let decoded: Action = serde_json::from_value(serde_json::to_value(&action).unwrap()).unwrap();
        assert_eq!(decoded, action);
    }

    #[test]
    fn retime_action_round_trips_and_rejects_zero_speed() {
        let action = Action::Retime {
            target: TargetRef::from_parts(TargetKind::Track, "track_main").unwrap(),
            params: RetimeParams {
                range: RangeNs {
                    start_ns: 0,
                    end_ns: 1_000,
                },
                speed_num: 1,
                speed_den: 2,
            },
        };
        let decoded: Action = serde_json::from_value(serde_json::to_value(&action).unwrap()).unwrap();
        assert_eq!(decoded, action);
    }

    #[test]
    fn caption_action_round_trips() {
        let action = Action::Caption {
            target: TargetRef::from_parts(TargetKind::Word, "w_000007").unwrap(),
            params: CaptionParams {
                range: RangeNs {
                    start_ns: 1_000,
                    end_ns: 2_000,
                },
                text: "Today we ship.".into(),
            },
        };
        let decoded: Action = serde_json::from_value(serde_json::to_value(&action).unwrap()).unwrap();
        assert_eq!(decoded, action);
    }

    #[test]
    fn graphic_action_round_trips() {
        let action = Action::Graphic {
            target: TargetRef::from_parts(TargetKind::Asset, "logo_main").unwrap(),
            params: GraphicParams {
                range: RangeNs {
                    start_ns: 0,
                    end_ns: 5_000,
                },
                graphic_id: "graphic_logo".into(),
            },
        };
        let decoded: Action = serde_json::from_value(serde_json::to_value(&action).unwrap()).unwrap();
        assert_eq!(decoded, action);
    }

    #[test]
    fn audio_action_round_trips() {
        let action = Action::Audio {
            target: TargetRef::from_parts(TargetKind::Asset, "voiceover_1").unwrap(),
            params: AudioParams {
                range: RangeNs {
                    start_ns: 0,
                    end_ns: 1_000_000,
                },
                gain: 1.5,
            },
        };
        let decoded: Action = serde_json::from_value(serde_json::to_value(&action).unwrap()).unwrap();
        assert_eq!(decoded, action);
    }

    #[test]
    fn colour_lut_action_round_trips() {
        let action = Action::ColourLut {
            target: sample_target(),
            params: ColourLutParams {
                range: RangeNs {
                    start_ns: 0,
                    end_ns: 1_000,
                },
                lut_id: "lut_cinematic_v3".into(),
            },
        };
        let decoded: Action = serde_json::from_value(serde_json::to_value(&action).unwrap()).unwrap();
        assert_eq!(decoded, action);
    }

    #[test]
    fn colour_correction_action_round_trips() {
        let action = Action::ColourCorrection {
            target: sample_target(),
            params: ColourCorrectionParams {
                range: RangeNs {
                    start_ns: 0,
                    end_ns: 1_000,
                },
                exposure_stops: 0.25,
                white_balance_kelvin: 200,
            },
        };
        let decoded: Action = serde_json::from_value(serde_json::to_value(&action).unwrap()).unwrap();
        assert_eq!(decoded, action);
    }

    #[test]
    fn export_render_action_round_trips() {
        let action = Action::ExportRender {
            target: TargetRef::from_parts(TargetKind::Asset, "preset_1080p").unwrap(),
            params: ExportRenderParams {
                preset_id: "preset_1080p".into(),
                target_revision: Some("rev_0042".into()),
            },
        };
        let decoded: Action = serde_json::from_value(serde_json::to_value(&action).unwrap()).unwrap();
        assert_eq!(decoded, action);
    }

    #[test]
    fn setting_action_round_trips() {
        let action = Action::Setting {
            target: TargetRef::from_parts(TargetKind::Project, "review_mode").unwrap(),
            params: SettingParams {
                key: "review_mode".into(),
                value: "autonomous".into(),
            },
        };
        let decoded: Action = serde_json::from_value(serde_json::to_value(&action).unwrap()).unwrap();
        assert_eq!(decoded, action);
    }

    #[test]
    fn unknown_action_kind_is_rejected() {
        let bogus = serde_json::json!({
            "kind": "no.such.action",
            "target": "clip:clip_5",
            "params": {}
        });
        let err = serde_json::from_value::<Action>(bogus).unwrap_err();
        // Serde emits "unknown variant" for internally-tagged enums.
        assert!(
            err.to_string().contains("unknown variant") || err.to_string().contains("no.such.action"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn every_declared_kind_round_trips() {
        let target = sample_target();
        let range = RangeNs {
            start_ns: 0,
            end_ns: 1_000,
        };
        for kind in ACTION_KINDS {
            let (action, expected_kind) = match *kind {
                "timeline.cut" => (
                    Action::Cut {
                        target: target.clone(),
                        params: CutParams {
                            range,
                            reason: None,
                        },
                    },
                    "timeline.cut",
                ),
                "timeline.restore" => (
                    Action::Restore {
                        target: target.clone(),
                        params: RestoreParams {
                            range,
                            source_batch_id: "batch_0001".into(),
                        },
                    },
                    "timeline.restore",
                ),
                "timeline.move" => (
                    Action::Move {
                        target: target.clone(),
                        params: MoveParams {
                            range,
                            new_start_ns: 1_000,
                        },
                    },
                    "timeline.move",
                ),
                "take.swap" => (
                    Action::TakeSwap {
                        target: target.clone(),
                        params: TakeSwapParams {
                            range,
                            replacement_clip_id: "clip_alt".into(),
                        },
                    },
                    "take.swap",
                ),
                "track.retime" => (
                    Action::Retime {
                        target: TargetRef::from_parts(TargetKind::Track, "track_main").unwrap(),
                        params: RetimeParams {
                            range,
                            speed_num: 1,
                            speed_den: 1,
                        },
                    },
                    "track.retime",
                ),
                "caption.edit" => (
                    Action::Caption {
                        target: TargetRef::from_parts(TargetKind::Word, "w_1").unwrap(),
                        params: CaptionParams {
                            range,
                            text: "hi".into(),
                        },
                    },
                    "caption.edit",
                ),
                "graphic.edit" => (
                    Action::Graphic {
                        target: TargetRef::from_parts(TargetKind::Asset, "g1").unwrap(),
                        params: GraphicParams {
                            range,
                            graphic_id: "g1".into(),
                        },
                    },
                    "graphic.edit",
                ),
                "audio.edit" => (
                    Action::Audio {
                        target: TargetRef::from_parts(TargetKind::Asset, "a1").unwrap(),
                        params: AudioParams { range, gain: 1.0 },
                    },
                    "audio.edit",
                ),
                "color.lut" => (
                    Action::ColourLut {
                        target: target.clone(),
                        params: ColourLutParams {
                            range,
                            lut_id: "lut".into(),
                        },
                    },
                    "color.lut",
                ),
                "color.correction" => (
                    Action::ColourCorrection {
                        target: target.clone(),
                        params: ColourCorrectionParams {
                            range,
                            exposure_stops: 0.0,
                            white_balance_kelvin: 0,
                        },
                    },
                    "color.correction",
                ),
                "export.render" => (
                    Action::ExportRender {
                        target: TargetRef::from_parts(TargetKind::Asset, "p1").unwrap(),
                        params: ExportRenderParams {
                            preset_id: "p1".into(),
                            target_revision: None,
                        },
                    },
                    "export.render",
                ),
                "setting.update" => (
                    Action::Setting {
                        target: TargetRef::from_parts(TargetKind::Project, "k1").unwrap(),
                        params: SettingParams {
                            key: "k1".into(),
                            value: "v".into(),
                        },
                    },
                    "setting.update",
                ),
                other => panic!("ACTION_KINDS contains unhandled kind {other}"),
            };
            assert_eq!(action_kind(&action), expected_kind);
            let value = serde_json::to_value(&action).unwrap();
            assert_eq!(value["kind"].as_str().unwrap(), expected_kind);
            let decoded: Action = serde_json::from_value(value).unwrap();
            assert_eq!(decoded, action);
        }
    }

    #[test]
    fn range_ns_validation() {
        let ok = RangeNs {
            start_ns: 0,
            end_ns: 1,
        };
        assert!(ok.is_valid());
        assert_eq!(ok.len_ns(), 1);
        let bad = RangeNs {
            start_ns: 5,
            end_ns: 5,
        };
        assert!(!bad.is_valid());
        let negative = RangeNs {
            start_ns: -1,
            end_ns: 1,
        };
        assert!(!negative.is_valid());
    }
}
