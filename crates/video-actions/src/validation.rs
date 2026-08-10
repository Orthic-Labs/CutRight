//! Pure semantic validators for every [`Action`] variant (CR-V2-B2-008).
//!
//! Every validator is **pure** (no I/O) and **typed**: it takes a single
//! [`Action`] plus an immutable [`ValidationContext`] view of the staged
//! revision and returns either `Ok(())` or a [`ValidationFailure`] with the
//! [`ValidationError`] taxonomy mandated by
//! `docs/architecture/V2-TRANSACTIONS-UNDO.md` §3.
//!
//! The validator rejects:
//! - Unknown targets (not present in the staged revision's known-target set).
//! - Out-of-range ranges (`start_ns < 0`, `end_ns <= start_ns`,
//!   `end_ns` past the revision's duration).
//! - Cross-project target references (target's project id does not match
//!   the staged revision's project id, see [`ValidationContext::target_projects`]).
//! - Rational-speed zero denominators / numerators.
//! - Empty caption text, empty replacement clip ids, empty preset ids.
//!
//! The validators **never** mutate the staged revision; mutation happens
//! during apply (CR-V2-B2-010).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use thiserror::Error;

use crate::action::{Action, RangeNs, TargetRef};

/// Typed validation failure codes.
///
/// Mirrors the `validation_error` value in
/// `schemas/actions/action-result.schema.v1.json` and the failure-code
/// taxonomy in `V2-TRANSACTIONS-UNDO.md` §3.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum ValidationError {
    /// Target id is not present in the staged revision.
    #[error("target {0:?} is not present in the staged revision")]
    UnknownTarget(String),
    /// Target id refers to a different project than the staged revision.
    #[error("target {target:?} belongs to project {target_project:?}, not staged project {staged_project:?}")]
    CrossProject {
        /// The rejected target.
        target: String,
        /// The project id embedded in the target.
        target_project: String,
        /// The staged revision's project id.
        staged_project: String,
    },
    /// Range start_ns is negative.
    #[error("range start_ns {0} must be non-negative")]
    NegativeStart(i64),
    /// Range end_ns is not strictly greater than start_ns.
    #[error("range end_ns {end_ns} must be strictly greater than start_ns {start_ns}")]
    InvalidRange {
        /// The invalid start_ns.
        start_ns: i64,
        /// The invalid end_ns.
        end_ns: i64,
    },
    /// Range extends past the staged revision's duration.
    #[error("range end_ns {end_ns} exceeds staged duration {duration_ns}")]
    RangeOverflow {
        /// The overflowing end_ns.
        end_ns: i64,
        /// The staged revision's duration.
        duration_ns: i64,
    },
    /// Rational speed numerator or denominator is zero.
    #[error("rational speed {num}/{den} must be strictly positive")]
    InvalidSpeed {
        /// The numerator.
        num: u64,
        /// The denominator.
        den: u64,
    },
    /// Required string field was empty.
    #[error("required string {field:?} was empty")]
    EmptyField {
        /// The empty field name.
        field: &'static str,
    },
    /// Audio gain was negative or non-finite.
    #[error("audio gain {0} must be finite and non-negative")]
    InvalidGain(f64),
    /// Exposure correction was non-finite or out of the [-10, +10] stops band.
    #[error("exposure_stops {0} must be finite and within [-10, 10]")]
    InvalidExposure(f64),
    /// White-balance shift was outside the [-5000, +5000] Kelvin band.
    #[error("white_balance_kelvin {0} must be within [-5000, 5000]")]
    InvalidWhiteBalance(i64),
}

/// A single validation failure attached to a specific action index.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationFailure {
    /// Index of the offending action within its batch.
    pub action_index: usize,
    /// Stable action kind string (matches the wire form).
    pub action_kind: String,
    /// Target of the offending action.
    pub target: String,
    /// The underlying validation error.
    pub error: ValidationError,
}

impl fmt::Display for ValidationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "validation failure on action {} ({} @ {}): {}",
            self.action_index, self.action_kind, self.target, self.error
        )
    }
}

impl std::error::Error for ValidationFailure {}

/// Read-only view of a staged revision used by every validator.
///
/// The view carries:
///
/// - `project_id` — every target's project prefix MUST equal this.
/// - `duration_ns` — every range MUST end at or before this.
/// - `known_targets` — every target id MUST be present here.
/// - `target_projects` — per-target project ids. A target present here whose
///   project does not equal `project_id` (and is not in `known_project_ids`)
///   is a cross-project reference and is rejected.
/// - `known_project_ids` — the set of project ids that ARE allowed as
///   cross-project references.
///
/// This struct is intentionally cheap to clone and pure; validators MUST NOT
/// take `&mut` on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationContext {
    /// Project the staged revision belongs to.
    pub project_id: String,
    /// Staged revision duration in nanoseconds.
    pub duration_ns: i64,
    /// All known target refs (`<kind>:<local>`).
    pub known_targets: BTreeSet<String>,
    /// Per-target project mapping used to detect cross-project references.
    pub target_projects: BTreeMap<String, String>,
    /// All known project ids (for cross-project reference checks).
    pub known_project_ids: BTreeSet<String>,
}

impl ValidationContext {
    /// Construct a context from raw parts. Panics if `project_id` is empty.
    pub fn new(
        project_id: impl Into<String>,
        duration_ns: i64,
        known_targets: BTreeSet<String>,
    ) -> Self {
        let project_id = project_id.into();
        assert!(!project_id.is_empty(), "project_id must be non-empty");
        let mut known_project_ids = BTreeSet::new();
        known_project_ids.insert(project_id.clone());
        Self {
            project_id,
            duration_ns,
            known_targets,
            target_projects: BTreeMap::new(),
            known_project_ids,
        }
    }

    /// Add a known project id (e.g. when validating a cross-project reference
    /// list). No-op if the id is already present.
    pub fn with_known_project(mut self, project_id: impl Into<String>) -> Self {
        self.known_project_ids.insert(project_id.into());
        self
    }

    /// Register a target's project id. The target must already be in
    /// `known_targets`.
    pub fn with_target_project(
        mut self,
        target: &TargetRef,
        project_id: impl Into<String>,
    ) -> Self {
        self.target_projects
            .insert(target.as_str().to_owned(), project_id.into());
        self
    }

    /// Returns true if `target` is known to the staged revision.
    pub fn knows_target(&self, target: &TargetRef) -> bool {
        self.known_targets.contains(target.as_str())
    }
}

/// Pure validator: validates a single [`Action`] against a [`ValidationContext`].
///
/// The trait lives here, not on `Action`, so the validator cannot accidentally
/// mutate the action.
pub trait Validator {
    /// Validate `action` against `ctx`. Returns `Ok(())` on success or a
    /// single-element [`ValidationFailure`] vector describing the failure.
    fn validate(
        &self,
        action_index: usize,
        action: &Action,
        ctx: &ValidationContext,
    ) -> Result<(), Vec<ValidationFailure>>;
}

/// Concrete validator used by both the dry-run planner (CR-V2-B2-009) and the
/// apply pipeline (CR-V2-B2-010).
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultValidator;

impl Validator for DefaultValidator {
    fn validate(
        &self,
        action_index: usize,
        action: &Action,
        ctx: &ValidationContext,
    ) -> Result<(), Vec<ValidationFailure>> {
        let mut failures: Vec<ValidationFailure> = Vec::new();
        let kind = action_kind(action);
        let target_str = target_str(action).to_owned();

        // ---- Target / cross-project checks (apply to every variant) ----
        if let Some(target) = action_target(action) {
            if !ctx.knows_target(target) {
                failures.push(mk_failure(
                    action_index,
                    kind,
                    &target_str,
                    ValidationError::UnknownTarget(target.as_str().to_owned()),
                ));
            }
            if let Some(target_project) = ctx.target_projects.get(target.as_str()) {
                if target_project != &ctx.project_id
                    && !ctx.known_project_ids.contains(target_project)
                {
                    failures.push(mk_failure(
                        action_index,
                        kind,
                        &target_str,
                        ValidationError::CrossProject {
                            target: target.as_str().to_owned(),
                            target_project: target_project.clone(),
                            staged_project: ctx.project_id.clone(),
                        },
                    ));
                }
            }
        }

        // ---- Variant-specific checks ----
        match action {
            Action::Cut { params, .. } => {
                validate_range(
                    action_index,
                    kind,
                    &target_str,
                    &params.range,
                    ctx,
                    &mut failures,
                );
            }
            Action::Restore { params, .. } => {
                validate_range(
                    action_index,
                    kind,
                    &target_str,
                    &params.range,
                    ctx,
                    &mut failures,
                );
            }
            Action::Move { params, .. } => {
                validate_range(
                    action_index,
                    kind,
                    &target_str,
                    &params.range,
                    ctx,
                    &mut failures,
                );
            }
            Action::TakeSwap { params, .. } => {
                validate_range(
                    action_index,
                    kind,
                    &target_str,
                    &params.range,
                    ctx,
                    &mut failures,
                );
            }
            Action::Retime { params, .. } => {
                validate_range(
                    action_index,
                    kind,
                    &target_str,
                    &params.range,
                    ctx,
                    &mut failures,
                );
            }
            Action::Caption { params, .. } => {
                validate_range(
                    action_index,
                    kind,
                    &target_str,
                    &params.range,
                    ctx,
                    &mut failures,
                );
            }
            Action::Graphic { params, .. } => {
                validate_range(
                    action_index,
                    kind,
                    &target_str,
                    &params.range,
                    ctx,
                    &mut failures,
                );
            }
            Action::Audio { params, .. } => {
                validate_range(
                    action_index,
                    kind,
                    &target_str,
                    &params.range,
                    ctx,
                    &mut failures,
                );
            }
            Action::ColourLut { params, .. } => {
                validate_range(
                    action_index,
                    kind,
                    &target_str,
                    &params.range,
                    ctx,
                    &mut failures,
                );
            }
            Action::ColourCorrection { params, .. } => {
                validate_range(
                    action_index,
                    kind,
                    &target_str,
                    &params.range,
                    ctx,
                    &mut failures,
                );
            }
            Action::ExportRender { params, .. } => {
                if params.preset_id.is_empty() {
                    failures.push(mk_failure(
                        action_index,
                        kind,
                        &target_str,
                        ValidationError::EmptyField { field: "preset_id" },
                    ));
                }
            }
            Action::Setting { params, .. } => {
                if params.key.is_empty() {
                    failures.push(mk_failure(
                        action_index,
                        kind,
                        &target_str,
                        ValidationError::EmptyField { field: "key" },
                    ));
                }
                if params.value.is_empty() {
                    failures.push(mk_failure(
                        action_index,
                        kind,
                        &target_str,
                        ValidationError::EmptyField { field: "value" },
                    ));
                }
            }
        }

        // ---- Variant-specific deep checks ----
        match action {
            Action::Retime { params, .. } => {
                if params.speed_num == 0 || params.speed_den == 0 {
                    failures.push(mk_failure(
                        action_index,
                        kind,
                        &target_str,
                        ValidationError::InvalidSpeed {
                            num: params.speed_num,
                            den: params.speed_den,
                        },
                    ));
                }
            }
            Action::TakeSwap { params, .. } => {
                if params.replacement_clip_id.is_empty() {
                    failures.push(mk_failure(
                        action_index,
                        kind,
                        &target_str,
                        ValidationError::EmptyField {
                            field: "replacement_clip_id",
                        },
                    ));
                }
            }
            Action::Caption { params, .. } => {
                if params.text.is_empty() {
                    failures.push(mk_failure(
                        action_index,
                        kind,
                        &target_str,
                        ValidationError::EmptyField { field: "text" },
                    ));
                }
            }
            Action::Graphic { params, .. } => {
                if params.graphic_id.is_empty() {
                    failures.push(mk_failure(
                        action_index,
                        kind,
                        &target_str,
                        ValidationError::EmptyField {
                            field: "graphic_id",
                        },
                    ));
                }
            }
            Action::Audio { params, .. } => {
                if !params.gain.is_finite() || params.gain < 0.0 {
                    failures.push(mk_failure(
                        action_index,
                        kind,
                        &target_str,
                        ValidationError::InvalidGain(params.gain),
                    ));
                }
            }
            Action::ColourLut { params, .. } => {
                if params.lut_id.is_empty() {
                    failures.push(mk_failure(
                        action_index,
                        kind,
                        &target_str,
                        ValidationError::EmptyField { field: "lut_id" },
                    ));
                }
            }
            Action::ColourCorrection { params, .. } => {
                if !params.exposure_stops.is_finite()
                    || params.exposure_stops < -10.0
                    || params.exposure_stops > 10.0
                {
                    failures.push(mk_failure(
                        action_index,
                        kind,
                        &target_str,
                        ValidationError::InvalidExposure(params.exposure_stops),
                    ));
                }
                if params.white_balance_kelvin < -5000 || params.white_balance_kelvin > 5000 {
                    failures.push(mk_failure(
                        action_index,
                        kind,
                        &target_str,
                        ValidationError::InvalidWhiteBalance(params.white_balance_kelvin),
                    ));
                }
            }
            _ => {}
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures)
        }
    }
}

/// Validate every action in a batch. Returns either `Ok(())` or a vector
/// with **every** failure (the validator never short-circuits).
pub fn validate_batch(
    actions: &[Action],
    ctx: &ValidationContext,
) -> Result<(), Vec<ValidationFailure>> {
    let validator = DefaultValidator;
    let mut all_failures: Vec<ValidationFailure> = Vec::new();
    for (index, action) in actions.iter().enumerate() {
        if let Err(mut failures) = validator.validate(index, action, ctx) {
            all_failures.append(&mut failures);
        }
    }
    if all_failures.is_empty() {
        Ok(())
    } else {
        Err(all_failures)
    }
}

fn validate_range(
    action_index: usize,
    kind: &str,
    target: &str,
    range: &RangeNs,
    ctx: &ValidationContext,
    failures: &mut Vec<ValidationFailure>,
) {
    if range.start_ns < 0 {
        failures.push(mk_failure(
            action_index,
            kind,
            target,
            ValidationError::NegativeStart(range.start_ns),
        ));
    }
    if range.end_ns <= range.start_ns {
        failures.push(mk_failure(
            action_index,
            kind,
            target,
            ValidationError::InvalidRange {
                start_ns: range.start_ns,
                end_ns: range.end_ns,
            },
        ));
    }
    if range.end_ns > ctx.duration_ns {
        failures.push(mk_failure(
            action_index,
            kind,
            target,
            ValidationError::RangeOverflow {
                end_ns: range.end_ns,
                duration_ns: ctx.duration_ns,
            },
        ));
    }
}

fn mk_failure(
    action_index: usize,
    kind: &str,
    target: &str,
    error: ValidationError,
) -> ValidationFailure {
    ValidationFailure {
        action_index,
        action_kind: kind.to_string(),
        target: target.to_string(),
        error,
    }
}

fn action_kind(action: &Action) -> &'static str {
    crate::action::action_kind(action)
}

fn action_target(action: &Action) -> Option<&TargetRef> {
    match action {
        Action::Cut { target, .. }
        | Action::Restore { target, .. }
        | Action::Move { target, .. }
        | Action::TakeSwap { target, .. }
        | Action::Retime { target, .. }
        | Action::Caption { target, .. }
        | Action::Graphic { target, .. }
        | Action::Audio { target, .. }
        | Action::ColourLut { target, .. }
        | Action::ColourCorrection { target, .. }
        | Action::ExportRender { target, .. }
        | Action::Setting { target, .. } => Some(target),
    }
}

fn target_str(action: &Action) -> &str {
    action_target(action).map(TargetRef::as_str).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{
        action_kind, AudioParams, CaptionParams, ColourCorrectionParams, ColourLutParams,
        CutParams, ExportRenderParams, GraphicParams, MoveParams, RestoreParams, RetimeParams,
        SettingParams, TakeSwapParams, TargetKind,
    };

    fn clip_target(id: &str) -> TargetRef {
        TargetRef::from_parts(TargetKind::Clip, id).unwrap()
    }

    fn word_target(id: &str) -> TargetRef {
        TargetRef::from_parts(TargetKind::Word, id).unwrap()
    }

    fn asset_target(id: &str) -> TargetRef {
        TargetRef::from_parts(TargetKind::Asset, id).unwrap()
    }

    fn track_target(id: &str) -> TargetRef {
        TargetRef::from_parts(TargetKind::Track, id).unwrap()
    }

    fn project_target(id: &str) -> TargetRef {
        TargetRef::from_parts(TargetKind::Project, id).unwrap()
    }

    fn range(start_ns: i64, end_ns: i64) -> RangeNs {
        RangeNs { start_ns, end_ns }
    }

    fn sample_ctx() -> ValidationContext {
        let mut known = BTreeSet::new();
        known.insert("clip:clip_5".to_string());
        known.insert("track:track_main".to_string());
        known.insert("word:w_000007".to_string());
        known.insert("asset:voiceover_1".to_string());
        known.insert("asset:logo_main".to_string());
        known.insert("asset:preset_1080p".to_string());
        known.insert("project:review_mode".to_string());
        ValidationContext::new("prj_main", 10_000_000_000, known)
    }

    fn cut(target: TargetRef, range: RangeNs) -> Action {
        Action::Cut {
            target,
            params: CutParams {
                range,
                reason: None,
            },
        }
    }

    #[test]
    fn happy_path_cut_validates() {
        let ctx = sample_ctx();
        let action = cut(clip_target("clip_5"), range(1_000, 2_000));
        assert!(DefaultValidator.validate(0, &action, &ctx).is_ok());
    }

    #[test]
    fn unknown_target_is_rejected() {
        let ctx = sample_ctx();
        let action = cut(clip_target("clip_missing"), range(1_000, 2_000));
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(
            err[0].error,
            ValidationError::UnknownTarget("clip:clip_missing".into())
        );
    }

    #[test]
    fn cross_project_target_is_rejected() {
        let mut ctx = sample_ctx();
        let target = clip_target("clip_x");
        ctx.known_targets.insert(target.as_str().to_string());
        ctx = ctx.with_target_project(&target, "prj_other");
        let action = cut(target, range(1_000, 2_000));
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(matches!(err[0].error, ValidationError::CrossProject { .. }));
    }

    #[test]
    fn cross_project_target_passes_when_project_is_known() {
        let mut ctx = sample_ctx();
        ctx = ctx.with_known_project("prj_other");
        let target = clip_target("clip_x");
        ctx.known_targets.insert(target.as_str().to_string());
        ctx = ctx.with_target_project(&target, "prj_other");
        let action = cut(target.clone(), range(1_000, 2_000));
        assert!(DefaultValidator.validate(0, &action, &ctx).is_ok());
    }

    #[test]
    fn same_project_target_passes_cross_project_check() {
        let mut ctx = sample_ctx();
        let target = clip_target("clip_5");
        ctx = ctx.with_target_project(&target, "prj_main");
        let action = cut(target, range(1_000, 2_000));
        assert!(DefaultValidator.validate(0, &action, &ctx).is_ok());
    }

    #[test]
    fn negative_start_is_rejected() {
        let ctx = sample_ctx();
        let action = cut(clip_target("clip_5"), range(-1, 100));
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err
            .iter()
            .any(|f| matches!(f.error, ValidationError::NegativeStart(_))));
    }

    #[test]
    fn zero_length_range_is_rejected() {
        let ctx = sample_ctx();
        let action = cut(clip_target("clip_5"), range(100, 100));
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err
            .iter()
            .any(|f| matches!(f.error, ValidationError::InvalidRange { .. })));
    }

    #[test]
    fn inverted_range_is_rejected() {
        let ctx = sample_ctx();
        let action = cut(clip_target("clip_5"), range(200, 100));
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err
            .iter()
            .any(|f| matches!(f.error, ValidationError::InvalidRange { .. })));
    }

    #[test]
    fn range_overflow_is_rejected() {
        let ctx = sample_ctx();
        let action = cut(clip_target("clip_5"), range(1_000, 11_000_000_000));
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err
            .iter()
            .any(|f| matches!(f.error, ValidationError::RangeOverflow { .. })));
    }

    #[test]
    fn retime_zero_speed_is_rejected() {
        let ctx = sample_ctx();
        let action = Action::Retime {
            target: track_target("track_main"),
            params: RetimeParams {
                range: range(1_000, 2_000),
                speed_num: 0,
                speed_den: 1,
            },
        };
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err
            .iter()
            .any(|f| matches!(f.error, ValidationError::InvalidSpeed { .. })));
    }

    #[test]
    fn retime_zero_denominator_is_rejected() {
        let ctx = sample_ctx();
        let action = Action::Retime {
            target: track_target("track_main"),
            params: RetimeParams {
                range: range(1_000, 2_000),
                speed_num: 1,
                speed_den: 0,
            },
        };
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err
            .iter()
            .any(|f| matches!(f.error, ValidationError::InvalidSpeed { .. })));
    }

    #[test]
    fn audio_negative_gain_is_rejected() {
        let ctx = sample_ctx();
        let action = Action::Audio {
            target: asset_target("voiceover_1"),
            params: AudioParams {
                range: range(1_000, 2_000),
                gain: -1.0,
            },
        };
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err
            .iter()
            .any(|f| matches!(f.error, ValidationError::InvalidGain(_))));
    }

    #[test]
    fn audio_non_finite_gain_is_rejected() {
        let ctx = sample_ctx();
        let action = Action::Audio {
            target: asset_target("voiceover_1"),
            params: AudioParams {
                range: range(1_000, 2_000),
                gain: f64::NAN,
            },
        };
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err
            .iter()
            .any(|f| matches!(f.error, ValidationError::InvalidGain(_))));
    }

    #[test]
    fn empty_caption_text_is_rejected() {
        let ctx = sample_ctx();
        let action = Action::Caption {
            target: word_target("w_000007"),
            params: CaptionParams {
                range: range(1_000, 2_000),
                text: String::new(),
            },
        };
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err
            .iter()
            .any(|f| matches!(f.error, ValidationError::EmptyField { field: "text" })));
    }

    #[test]
    fn empty_replacement_clip_is_rejected() {
        let ctx = sample_ctx();
        let action = Action::TakeSwap {
            target: clip_target("clip_5"),
            params: TakeSwapParams {
                range: range(1_000, 2_000),
                replacement_clip_id: String::new(),
            },
        };
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err.iter().any(|f| matches!(
            f.error,
            ValidationError::EmptyField {
                field: "replacement_clip_id"
            }
        )));
    }

    #[test]
    fn empty_graphic_id_is_rejected() {
        let ctx = sample_ctx();
        let action = Action::Graphic {
            target: asset_target("logo_main"),
            params: GraphicParams {
                range: range(1_000, 2_000),
                graphic_id: String::new(),
            },
        };
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err.iter().any(|f| matches!(
            f.error,
            ValidationError::EmptyField {
                field: "graphic_id"
            }
        )));
    }

    #[test]
    fn empty_lut_id_is_rejected() {
        let ctx = sample_ctx();
        let action = Action::ColourLut {
            target: clip_target("clip_5"),
            params: ColourLutParams {
                range: range(1_000, 2_000),
                lut_id: String::new(),
            },
        };
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err
            .iter()
            .any(|f| matches!(f.error, ValidationError::EmptyField { field: "lut_id" })));
    }

    #[test]
    fn exposure_out_of_band_is_rejected() {
        let ctx = sample_ctx();
        let action = Action::ColourCorrection {
            target: clip_target("clip_5"),
            params: ColourCorrectionParams {
                range: range(1_000, 2_000),
                exposure_stops: 50.0,
                white_balance_kelvin: 0,
            },
        };
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err
            .iter()
            .any(|f| matches!(f.error, ValidationError::InvalidExposure(_))));
    }

    #[test]
    fn white_balance_out_of_band_is_rejected() {
        let ctx = sample_ctx();
        let action = Action::ColourCorrection {
            target: clip_target("clip_5"),
            params: ColourCorrectionParams {
                range: range(1_000, 2_000),
                exposure_stops: 0.0,
                white_balance_kelvin: 50_000,
            },
        };
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err
            .iter()
            .any(|f| matches!(f.error, ValidationError::InvalidWhiteBalance(_))));
    }

    #[test]
    fn empty_preset_id_is_rejected() {
        let ctx = sample_ctx();
        let action = Action::ExportRender {
            target: asset_target("preset_1080p"),
            params: ExportRenderParams {
                preset_id: String::new(),
                target_revision: None,
            },
        };
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err
            .iter()
            .any(|f| matches!(f.error, ValidationError::EmptyField { field: "preset_id" })));
    }

    #[test]
    fn empty_setting_key_or_value_is_rejected() {
        let ctx = sample_ctx();
        let action = Action::Setting {
            target: project_target("review_mode"),
            params: SettingParams {
                key: String::new(),
                value: "autonomous".into(),
            },
        };
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err
            .iter()
            .any(|f| matches!(f.error, ValidationError::EmptyField { field: "key" })));

        let action = Action::Setting {
            target: project_target("review_mode"),
            params: SettingParams {
                key: "review_mode".into(),
                value: String::new(),
            },
        };
        let err = DefaultValidator.validate(0, &action, &ctx).unwrap_err();
        assert!(err
            .iter()
            .any(|f| matches!(f.error, ValidationError::EmptyField { field: "value" })));
    }

    #[test]
    fn validate_batch_reports_every_failure() {
        let ctx = sample_ctx();
        let actions = vec![
            cut(clip_target("clip_5"), range(-1, 100)), // negative_start
            cut(clip_target("clip_missing"), range(1_000, 2_000)), // unknown_target
            cut(clip_target("clip_5"), range(1_000, 2_000)), // ok
        ];
        let err = validate_batch(&actions, &ctx).unwrap_err();
        // At least 2 failures, one from each of the first two actions.
        assert!(err.len() >= 2);
        assert!(err
            .iter()
            .any(|f| f.action_index == 0 && matches!(f.error, ValidationError::NegativeStart(_))));
        assert!(err
            .iter()
            .any(|f| f.action_index == 1 && matches!(f.error, ValidationError::UnknownTarget(_))));
    }

    #[test]
    fn happy_path_every_variant_validates() {
        let mut ctx = sample_ctx();
        ctx.known_targets.insert("asset:g1".to_string());
        ctx.known_targets.insert("asset:a1".to_string());
        ctx.known_targets.insert("clip:clip_x".to_string());
        ctx.known_targets.insert("clip:clip_alt".to_string());

        let actions = vec![
            cut(clip_target("clip_5"), range(1_000, 2_000)),
            Action::Restore {
                target: clip_target("clip_5"),
                params: RestoreParams {
                    range: range(1_000, 2_000),
                    source_batch_id: "batch_0001".into(),
                },
            },
            Action::Move {
                target: clip_target("clip_5"),
                params: MoveParams {
                    range: range(1_000, 2_000),
                    new_start_ns: 5_000,
                },
            },
            Action::TakeSwap {
                target: clip_target("clip_5"),
                params: TakeSwapParams {
                    range: range(1_000, 2_000),
                    replacement_clip_id: "clip_alt".into(),
                },
            },
            Action::Retime {
                target: track_target("track_main"),
                params: RetimeParams {
                    range: range(1_000, 2_000),
                    speed_num: 1,
                    speed_den: 2,
                },
            },
            Action::Caption {
                target: word_target("w_000007"),
                params: CaptionParams {
                    range: range(1_000, 2_000),
                    text: "hi".into(),
                },
            },
            Action::Graphic {
                target: asset_target("g1"),
                params: GraphicParams {
                    range: range(1_000, 2_000),
                    graphic_id: "g1".into(),
                },
            },
            Action::Audio {
                target: asset_target("a1"),
                params: AudioParams {
                    range: range(1_000, 2_000),
                    gain: 1.0,
                },
            },
            Action::ColourLut {
                target: clip_target("clip_5"),
                params: ColourLutParams {
                    range: range(1_000, 2_000),
                    lut_id: "lut".into(),
                },
            },
            Action::ColourCorrection {
                target: clip_target("clip_5"),
                params: ColourCorrectionParams {
                    range: range(1_000, 2_000),
                    exposure_stops: 0.0,
                    white_balance_kelvin: 0,
                },
            },
            Action::ExportRender {
                target: asset_target("preset_1080p"),
                params: ExportRenderParams {
                    preset_id: "preset_1080p".into(),
                    target_revision: None,
                },
            },
            Action::Setting {
                target: project_target("review_mode"),
                params: SettingParams {
                    key: "review_mode".into(),
                    value: "autonomous".into(),
                },
            },
        ];
        for (index, action) in actions.iter().enumerate() {
            DefaultValidator
                .validate(index, action, &ctx)
                .unwrap_or_else(|errs| {
                    panic!(
                        "variant {} ({:?}) failed validation: {errs:?}",
                        index,
                        action_kind(action)
                    )
                });
        }
    }
}
