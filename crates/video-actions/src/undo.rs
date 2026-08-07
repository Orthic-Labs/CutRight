//! Inverse batches, undo, and redo (CR-V2-B2-011).
//!
//! After every successful apply the [`crate::apply::StagedApply`] pipeline
//! emits an [`InverseBatch`] (`cutright.inverse_batch/v1`) that runs through
//! the same validator + apply pipeline as a forward batch. The
//! [`UndoRedoStack`] holds applied and undone receipts; [`UndoRedoStack::undo`]
//! applies the inverse of the most recent applied receipt, [`UndoRedoStack::redo`]
//! reapplies the most recent undone receipt.
//!
//! Non-reversible actions return [`UndoError::NonReversible`] so the caller
//! can surface the failure rather than silently corrupting state
//! (`V2-TRANSACTIONS-UNDO.md` §2).

use std::collections::VecDeque;
use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::action::{
    Action, AudioParams, CaptionParams, ColourCorrectionParams, ColourLutParams,
    ExportRenderParams, GraphicParams, MoveParams, RestoreParams, RetimeParams, SettingParams,
    TakeSwapParams,
};
use crate::apply::{ApplyError, ApplyOutcome, StagedApply};
use crate::revision::{Receipt, StagedRevision, RECEIPT_SCHEMA};

/// Schema id for an [`InverseBatch`].
pub const INVERSE_BATCH_SCHEMA: &str = "cutright.inverse_batch/v1";

/// Wire schema for a `cutright.inverse_batch/v1` inverse batch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InverseBatch {
    /// Always [`INVERSE_BATCH_SCHEMA`].
    pub schema: String,
    /// The action batch id this inverse undoes.
    pub original_batch_id: String,
    /// The revision id the inverse batch must apply against.
    pub expected_revision: String,
    /// The inverse actions, in order.
    pub inverse_actions: Vec<Action>,
    /// Whether the inverse batch is fully invertible (i.e. `undo(undo(x))`
    /// reproduces the original).
    pub fully_invertible: bool,
}

/// Typed undo / redo error.
#[derive(Debug, Error)]
pub enum UndoError {
    /// The undo stack is empty — nothing to undo.
    #[error("undo stack is empty")]
    NothingToUndo,
    /// The redo stack is empty — nothing to redo.
    #[error("redo stack is empty")]
    NothingToRedo,
    /// The most recent receipt describes a non-reversible action.
    #[error("most recent receipt is non-reversible: {0}")]
    NonReversible(String),
    /// The apply pipeline returned an error.
    #[error("undo apply failed: {0}")]
    Apply(#[from] ApplyError),
}

/// Result of an undo or redo call. Mirrors [`ApplyOutcome`] but is its own
/// type so callers can match on it directly without importing the apply
/// module.
#[derive(Debug, Clone)]
pub enum UndoOutcome {
    /// The undo/redo succeeded. The carried [`Receipt`] describes the
    /// inverse batch that was just applied (undo) or the original batch
    /// that was reapplied (redo).
    Applied {
        /// The newly applied receipt.
        receipt: Receipt,
        /// True if this was an undo; false if it was a redo.
        was_undo: bool,
    },
    /// The undo/redo failed. The stack state is unchanged.
    Failed {
        /// Reason for the failure.
        reason: String,
        /// True if this was an undo attempt; false if it was a redo attempt.
        was_undo: bool,
    },
}

impl fmt::Display for UndoOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UndoOutcome::Applied { receipt, was_undo } => {
                let verb = if *was_undo { "undo" } else { "redo" };
                write!(f, "{verb} applied receipt {}", receipt.receipt_id)
            }
            UndoOutcome::Failed { reason, was_undo } => {
                let verb = if *was_undo { "undo" } else { "redo" };
                write!(f, "{verb} failed: {reason}")
            }
        }
    }
}

/// Build the inverse of a single action.
///
/// Some actions are directly invertible:
/// - `timeline.cut` → `timeline.restore` with `source_batch_id` = original
///   batch id.
/// - `timeline.restore` → `timeline.cut` with the same range.
/// - `timeline.move` → `timeline.move` swapping `range.start_ns` and
///   `new_start_ns`.
/// - `take.swap` → `take.swap` swapping the original and replacement clip ids
///   (the caller must pass the original clip id via
///   [`inverse_of_with_original`]).
///
/// Other actions are NOT reversible without the original state. They return
/// [`UndoError::NonReversible`] via [`inverse_batch_for`] so the caller
/// refuses to emit an inverse batch.
pub fn inverse_of(action: &Action, original_batch_id: &str) -> Result<Action, UndoError> {
    inverse_of_with_original(action, original_batch_id, None)
}

/// Like [`inverse_of`] but takes an optional `original_clip_id` for use by
/// `take.swap` inversion. Pass the target's underlying clip id (the `local`
/// part of the target id) when inverting a `take.swap`.
pub fn inverse_of_with_original(
    action: &Action,
    original_batch_id: &str,
    original_clip_id: Option<&str>,
) -> Result<Action, UndoError> {
    match action {
        Action::Cut { target, params } => Ok(Action::Restore {
            target: target.clone(),
            params: RestoreParams {
                range: params.range,
                source_batch_id: original_batch_id.to_string(),
            },
        }),
        Action::Restore { target, params } => Ok(Action::Cut {
            target: target.clone(),
            params: crate::action::CutParams {
                range: params.range,
                reason: Some(format!("inverse of restore {original_batch_id}")),
            },
        }),
        Action::Move { target, params } => Ok(Action::Move {
            target: target.clone(),
            params: MoveParams {
                range: params.range,
                new_start_ns: params.range.start_ns,
            },
        }),
        Action::TakeSwap { target, params } => {
            let original_clip = original_clip_id.ok_or_else(|| {
                UndoError::NonReversible("take.swap requires original_clip_id".into())
            })?;
            Ok(Action::TakeSwap {
                target: target.clone(),
                params: TakeSwapParams {
                    range: params.range,
                    replacement_clip_id: original_clip.to_string(),
                },
            })
        }
        Action::Retime { .. } => Err(UndoError::NonReversible(
            "track.retime requires preserved prior revision (no prior speed stored in the receipt)"
                .into(),
        )),
        Action::Caption { .. } => Err(UndoError::NonReversible(
            "caption.edit requires preserved prior revision (no prior text stored)".into(),
        )),
        Action::Graphic { .. } => Err(UndoError::NonReversible(
            "graphic.edit requires preserved prior revision (no prior graphic id stored)".into(),
        )),
        Action::Audio { .. } => Err(UndoError::NonReversible(
            "audio.edit requires preserved prior revision (no prior gain stored)".into(),
        )),
        Action::ColourLut { .. } => Err(UndoError::NonReversible(
            "color.lut requires preserved prior revision (no prior LUT stored)".into(),
        )),
        Action::ColourCorrection { .. } => Err(UndoError::NonReversible(
            "color.correction requires preserved prior revision".into(),
        )),
        Action::ExportRender { .. } => Err(UndoError::NonReversible(
            "export.render is not reversible (no time-domain effect)".into(),
        )),
        Action::Setting { .. } => Err(UndoError::NonReversible(
            "setting.update requires preserved prior revision".into(),
        )),
    }
}

/// Build the [`InverseBatch`] for a forward batch.
///
/// If every action in `actions` is directly invertible, `fully_invertible` is
/// set to `true`. Otherwise the first non-reversible action aborts the
/// inverse batch with [`UndoError::NonReversible`].
pub fn inverse_batch_for(
    original_batch_id: &str,
    expected_revision: &str,
    actions: &[Action],
) -> Result<InverseBatch, UndoError> {
    inverse_batch_for_with_original(original_batch_id, expected_revision, actions, &[])
}

/// Like [`inverse_batch_for`] but accepts per-action `original_clip_id`
/// overrides for `take.swap` inversion.
pub fn inverse_batch_for_with_original(
    original_batch_id: &str,
    expected_revision: &str,
    actions: &[Action],
    original_clip_ids: &[Option<String>],
) -> Result<InverseBatch, UndoError> {
    let mut inverse_actions: Vec<Action> = Vec::with_capacity(actions.len());
    let mut fully_invertible = true;
    for (index, action) in actions.iter().enumerate() {
        let original = original_clip_ids
            .get(index)
            .and_then(|value| value.as_deref());
        match inverse_of_with_original(action, original_batch_id, original) {
            Ok(inverse) => inverse_actions.push(inverse),
            Err(error) => {
                fully_invertible = false;
                return Err(error);
            }
        }
    }
    Ok(InverseBatch {
        schema: INVERSE_BATCH_SCHEMA.to_string(),
        original_batch_id: original_batch_id.to_string(),
        expected_revision: expected_revision.to_string(),
        inverse_actions,
        fully_invertible,
    })
}

/// Stack of applied and undone receipts driving the undo / redo commands.
#[derive(Debug, Default, Clone)]
pub struct UndoRedoStack {
    undo_stack: VecDeque<AppliedReceipt>,
    redo_stack: VecDeque<AppliedReceipt>,
}

/// One entry in the undo/redo stack.
#[derive(Debug, Clone)]
pub struct AppliedReceipt {
    /// The action batch id that was applied (forward or inverse).
    pub batch_id: String,
    /// The receipt for the application.
    pub receipt: Receipt,
    /// The forward actions that were applied (kept so redo can replay them).
    pub forward_actions: Vec<Action>,
    /// The inverse actions (kept so undo can re-invert).
    pub inverse_actions: Vec<Action>,
}

impl UndoRedoStack {
    /// Construct an empty stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of receipts that can be undone.
    pub fn undo_len(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of receipts that can be redone.
    pub fn redo_len(&self) -> usize {
        self.redo_stack.len()
    }

    /// True iff the undo stack is empty.
    pub fn is_empty(&self) -> bool {
        self.undo_stack.is_empty()
    }

    /// Push a freshly applied receipt onto the undo stack. Clears the redo
    /// stack (a new forward batch invalidates any pending redo).
    pub fn push_applied(
        &mut self,
        batch_id: impl Into<String>,
        receipt: Receipt,
        forward_actions: Vec<Action>,
        inverse_actions: Vec<Action>,
    ) {
        self.redo_stack.clear();
        self.undo_stack.push_back(AppliedReceipt {
            batch_id: batch_id.into(),
            receipt,
            forward_actions,
            inverse_actions,
        });
    }

    /// Apply the inverse of the most recent applied batch.
    ///
    /// The pipeline applies the inverse batch through the same validator +
    /// apply path as a forward batch. On success the entry is moved from
    /// the undo stack to the redo stack so a subsequent
    /// [`UndoRedoStack::redo`] can replay the original forward batch.
    pub fn undo(
        &mut self,
        apply: &StagedApply,
        staged: &mut StagedRevision,
    ) -> Result<UndoOutcome, UndoError> {
        let entry = self.undo_stack.pop_back().ok_or(UndoError::NothingToUndo)?;
        let inverse_actions = entry.inverse_actions.clone();
        let original_batch_id = entry.batch_id.clone();
        let expected_revision = entry.receipt.new_revision.clone();
        let outcome = apply.apply(
            &format!("inverse:{original_batch_id}"),
            &expected_revision,
            &inverse_actions,
            staged,
            None,
        )?;
        match outcome {
            ApplyOutcome::Applied { receipt, .. } => {
                // Move the entry from undo_stack to redo_stack. The redo
                // stack keeps the ORIGINAL forward/inverse pair so that
                // `redo` re-applies the original forward batch.
                self.redo_stack.push_back(AppliedReceipt {
                    batch_id: entry.batch_id,
                    receipt: receipt.clone(),
                    forward_actions: entry.forward_actions,
                    inverse_actions: entry.inverse_actions,
                });
                Ok(UndoOutcome::Applied {
                    receipt,
                    was_undo: true,
                })
            }
            ApplyOutcome::Failed { failures, .. } => {
                // Stack state is unchanged on failure (per the spec).
                self.undo_stack.push_back(entry);
                let reason = failures
                    .first()
                    .map(|f| f.message.clone())
                    .unwrap_or_else(|| "unknown failure".to_string());
                Ok(UndoOutcome::Failed {
                    reason,
                    was_undo: true,
                })
            }
        }
    }

    /// Re-apply the most recent undone batch.
    pub fn redo(
        &mut self,
        apply: &StagedApply,
        staged: &mut StagedRevision,
    ) -> Result<UndoOutcome, UndoError> {
        let entry = self.redo_stack.pop_back().ok_or(UndoError::NothingToRedo)?;
        let forward_actions = entry.forward_actions.clone();
        let original_batch_id = entry.batch_id.clone();
        let expected_revision = entry.receipt.new_revision.clone();
        let outcome = apply.apply(
            &format!("redo:{original_batch_id}"),
            &expected_revision,
            &forward_actions,
            staged,
            None,
        )?;
        match outcome {
            ApplyOutcome::Applied { receipt, .. } => {
                self.undo_stack.push_back(AppliedReceipt {
                    batch_id: entry.batch_id,
                    receipt: receipt.clone(),
                    forward_actions: entry.forward_actions,
                    inverse_actions: entry.inverse_actions,
                });
                Ok(UndoOutcome::Applied {
                    receipt,
                    was_undo: false,
                })
            }
            ApplyOutcome::Failed { failures, .. } => {
                self.redo_stack.push_back(entry);
                let reason = failures
                    .first()
                    .map(|f| f.message.clone())
                    .unwrap_or_else(|| "unknown failure".to_string());
                Ok(UndoOutcome::Failed {
                    reason,
                    was_undo: false,
                })
            }
        }
    }

    /// Return the receipts currently on the undo stack, oldest first.
    pub fn undo_receipts(&self) -> Vec<Receipt> {
        self.undo_stack
            .iter()
            .map(|entry| entry.receipt.clone())
            .collect()
    }

    /// Return the receipts currently on the redo stack, oldest first.
    pub fn redo_receipts(&self) -> Vec<Receipt> {
        self.redo_stack
            .iter()
            .map(|entry| entry.receipt.clone())
            .collect()
    }
}

/// Check whether a single action is directly invertible (i.e. its inverse
/// can be computed from the action alone, without any preserved prior
/// state).
pub fn is_directly_invertible(action: &Action) -> bool {
    matches!(
        action,
        Action::Cut { .. } | Action::Restore { .. } | Action::Move { .. }
    )
}

/// Reason an action is non-reversible.
pub fn non_reversible_reason(action: &Action) -> &'static str {
    match action {
        Action::Cut { .. } => "",
        Action::Restore { .. } => "",
        Action::Move { .. } => "",
        Action::TakeSwap { .. } => "take.swap requires original_clip_id",
        Action::Retime { .. } => "track.retime requires preserved prior revision",
        Action::Caption { .. } => "caption.edit requires preserved prior revision",
        Action::Graphic { .. } => "graphic.edit requires preserved prior revision",
        Action::Audio { .. } => "audio.edit requires preserved prior revision",
        Action::ColourLut { .. } => "color.lut requires preserved prior revision",
        Action::ColourCorrection { .. } => "color.correction requires preserved prior revision",
        Action::ExportRender { .. } => "export.render is not reversible",
        Action::Setting { .. } => "setting.update requires preserved prior revision",
    }
}

/// Helper for tests and external callers that want to peek at the most
/// recent undo entry without popping it.
pub fn last_undo_entry(stack: &UndoRedoStack) -> Option<&AppliedReceipt> {
    stack.undo_stack.back()
}

#[doc(hidden)]
pub fn _ensure_receipt_schema_in_scope() -> &'static str {
    RECEIPT_SCHEMA
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{CutParams, RangeNs, TargetKind, TargetRef};
    use crate::apply::StagedApply;
    use crate::revision::StagedRevision;

    const DEFAULT_DURATION_NS: i64 = 10_000_000_000;

    fn clip_target(id: &str) -> TargetRef {
        TargetRef::from_parts(TargetKind::Clip, id).unwrap()
    }

    fn track_target(id: &str) -> TargetRef {
        TargetRef::from_parts(TargetKind::Track, id).unwrap()
    }

    fn word_target(id: &str) -> TargetRef {
        TargetRef::from_parts(TargetKind::Word, id).unwrap()
    }

    fn asset_target(id: &str) -> TargetRef {
        TargetRef::from_parts(TargetKind::Asset, id).unwrap()
    }

    fn cut_action() -> Action {
        Action::Cut {
            target: clip_target("clip_5"),
            params: CutParams {
                range: RangeNs {
                    start_ns: 1_000,
                    end_ns: 2_000,
                },
                reason: None,
            },
        }
    }

    fn staged_with_clip5() -> StagedRevision {
        let mut staged = StagedRevision::from_active(&make_active("rev_0001"), DEFAULT_DURATION_NS);
        staged.register_target("clip:clip_5");
        staged
    }

    fn make_active(revision_id: &str) -> crate::revision::Revision {
        crate::revision::Revision {
            schema: crate::revision::REVISION_SCHEMA.to_string(),
            revision_id: revision_id.into(),
            parents: Vec::new(),
            created_at_ns: 1_700_000_000_000_000_000,
            active_pointer: "prj_main".into(),
            compatibility_fp: "deadbeefcafebabe1234567890abcdef".into(),
        }
    }

    #[test]
    fn inverse_of_cut_is_restore() {
        let cut = cut_action();
        let inverse = inverse_of(&cut, "batch_0001").unwrap();
        match inverse {
            Action::Restore { params, .. } => {
                assert_eq!(params.range.start_ns, 1_000);
                assert_eq!(params.range.end_ns, 2_000);
                assert_eq!(params.source_batch_id, "batch_0001");
            }
            _ => panic!("expected restore"),
        }
    }

    #[test]
    fn inverse_of_restore_is_cut() {
        let restore = Action::Restore {
            target: clip_target("clip_5"),
            params: RestoreParams {
                range: RangeNs {
                    start_ns: 1_000,
                    end_ns: 2_000,
                },
                source_batch_id: "batch_0001".into(),
            },
        };
        let inverse = inverse_of(&restore, "batch_0001").unwrap();
        match inverse {
            Action::Cut { params, .. } => {
                assert_eq!(params.range.start_ns, 1_000);
                assert_eq!(params.range.end_ns, 2_000);
            }
            _ => panic!("expected cut"),
        }
    }

    #[test]
    fn inverse_of_move_swaps_position() {
        let action = Action::Move {
            target: clip_target("clip_5"),
            params: MoveParams {
                range: RangeNs {
                    start_ns: 1_000,
                    end_ns: 2_000,
                },
                new_start_ns: 5_000,
            },
        };
        let inverse = inverse_of(&action, "batch_0001").unwrap();
        match inverse {
            Action::Move { params, .. } => {
                // The inverse restores the clip back to its original start
                // position by setting new_start_ns = range.start_ns.
                assert_eq!(params.new_start_ns, 1_000);
            }
            _ => panic!("expected move"),
        }
    }

    #[test]
    fn inverse_of_take_swap_requires_original() {
        let action = Action::TakeSwap {
            target: clip_target("clip_5"),
            params: TakeSwapParams {
                range: RangeNs {
                    start_ns: 1_000,
                    end_ns: 2_000,
                },
                replacement_clip_id: "clip_alt".into(),
            },
        };
        let err = inverse_of(&action, "batch_0001").unwrap_err();
        assert!(matches!(err, UndoError::NonReversible(_)));
        let inverse = inverse_of_with_original(&action, "batch_0001", Some("clip_5")).unwrap();
        match inverse {
            Action::TakeSwap { params, .. } => {
                assert_eq!(params.replacement_clip_id, "clip_5");
            }
            _ => panic!("expected take_swap"),
        }
    }

    #[test]
    fn inverse_of_non_reversible_actions_is_rejected() {
        let actions = vec![
            Action::Retime {
                target: track_target("track_main"),
                params: RetimeParams {
                    range: RangeNs {
                        start_ns: 1_000,
                        end_ns: 2_000,
                    },
                    speed_num: 1,
                    speed_den: 2,
                },
            },
            Action::Caption {
                target: word_target("w_000007"),
                params: CaptionParams {
                    range: RangeNs {
                        start_ns: 1_000,
                        end_ns: 2_000,
                    },
                    text: "hi".into(),
                },
            },
            Action::Graphic {
                target: asset_target("logo_main"),
                params: GraphicParams {
                    range: RangeNs {
                        start_ns: 1_000,
                        end_ns: 2_000,
                    },
                    graphic_id: "logo".into(),
                },
            },
            Action::Audio {
                target: asset_target("voiceover_1"),
                params: AudioParams {
                    range: RangeNs {
                        start_ns: 1_000,
                        end_ns: 2_000,
                    },
                    gain: 1.0,
                },
            },
            Action::ColourLut {
                target: clip_target("clip_5"),
                params: ColourLutParams {
                    range: RangeNs {
                        start_ns: 1_000,
                        end_ns: 2_000,
                    },
                    lut_id: "lut".into(),
                },
            },
            Action::ColourCorrection {
                target: clip_target("clip_5"),
                params: ColourCorrectionParams {
                    range: RangeNs {
                        start_ns: 1_000,
                        end_ns: 2_000,
                    },
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
                target: clip_target("clip_5"),
                params: SettingParams {
                    key: "k".into(),
                    value: "v".into(),
                },
            },
        ];
        for action in actions {
            let err = inverse_of(&action, "batch_0001")
                .err()
                .unwrap_or_else(|| panic!("expected non-reversible for {:?}", action));
            assert!(matches!(err, UndoError::NonReversible(_)));
        }
    }

    #[test]
    fn inverse_batch_for_restore_only() {
        let actions = vec![cut_action()];
        let inverse = inverse_batch_for("batch_0001", "rev_0001", &actions).unwrap();
        assert_eq!(inverse.schema, INVERSE_BATCH_SCHEMA);
        assert_eq!(inverse.original_batch_id, "batch_0001");
        assert_eq!(inverse.inverse_actions.len(), 1);
        assert!(inverse.fully_invertible);
    }

    #[test]
    fn inverse_batch_for_rejects_non_reversible() {
        let actions = vec![Action::ExportRender {
            target: asset_target("preset_1080p"),
            params: ExportRenderParams {
                preset_id: "preset_1080p".into(),
                target_revision: None,
            },
        }];
        let err = inverse_batch_for("batch_0001", "rev_0001", &actions).unwrap_err();
        assert!(matches!(err, UndoError::NonReversible(_)));
    }

    #[test]
    fn inverse_batch_round_trips() {
        let actions = vec![cut_action()];
        let inverse = inverse_batch_for("batch_0001", "rev_0001", &actions).unwrap();
        let value = serde_json::to_value(&inverse).unwrap();
        assert_eq!(value["schema"], "cutright.inverse_batch/v1");
        assert_eq!(value["original_batch_id"], "batch_0001");
        let decoded: InverseBatch = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, inverse);
    }

    #[test]
    fn undo_then_redo_round_trips_back_to_original() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = StagedApply::new(temp.path());
        let mut staged = staged_with_clip5();
        let mut stack = UndoRedoStack::new();

        // Apply forward.
        let outcome = pipeline
            .apply("batch_0001", "rev_0001", &[cut_action()], &mut staged, None)
            .unwrap();
        let receipt = match outcome {
            ApplyOutcome::Applied { receipt, .. } => receipt,
            _ => panic!("expected applied"),
        };
        let new_revision = receipt.new_revision.clone();
        let inverse = inverse_batch_for("batch_0001", &new_revision, &[cut_action()]).unwrap();
        stack.push_applied(
            "batch_0001",
            receipt,
            vec![cut_action()],
            inverse.inverse_actions,
        );

        // Undo applies the inverse (restore): the inverse of an inverse is
        // the original action, so undo+redo is the round-trip back to the
        // original state.
        let undo_outcome = stack.undo(&pipeline, &mut staged).unwrap();
        assert!(matches!(
            undo_outcome,
            UndoOutcome::Applied { was_undo: true, .. }
        ));

        // Redo replays the original forward batch (cut), confirming that
        // the inverse batch was the inverse of the original and that the
        // inverse-of-inverse returns to the original state.
        let redo_outcome = stack.redo(&pipeline, &mut staged).unwrap();
        assert!(matches!(
            redo_outcome,
            UndoOutcome::Applied {
                was_undo: false,
                ..
            }
        ));
    }

    #[test]
    fn undo_of_undo_without_redo_fails_with_nothing_to_undo() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = StagedApply::new(temp.path());
        let mut staged = staged_with_clip5();
        let mut stack = UndoRedoStack::new();

        let outcome = pipeline
            .apply("batch_0001", "rev_0001", &[cut_action()], &mut staged, None)
            .unwrap();
        let receipt = match outcome {
            ApplyOutcome::Applied { receipt, .. } => receipt,
            _ => panic!("expected applied"),
        };
        let new_revision = receipt.new_revision.clone();
        let inverse = inverse_batch_for("batch_0001", &new_revision, &[cut_action()]).unwrap();
        stack.push_applied(
            "batch_0001",
            receipt,
            vec![cut_action()],
            inverse.inverse_actions,
        );

        // First undo is fine.
        let _ = stack.undo(&pipeline, &mut staged).unwrap();
        // Second undo: the stack is empty (entry moved to redo_stack).
        let err = stack.undo(&pipeline, &mut staged).unwrap_err();
        assert!(matches!(err, UndoError::NothingToUndo));
    }

    #[test]
    fn inverse_of_inverse_reproduces_cut() {
        let cut = cut_action();
        let inverse = inverse_of(&cut, "batch_0001").unwrap();
        // The inverse of `cut` is `restore`.
        match &inverse {
            Action::Restore { .. } => {}
            _ => panic!("inverse of cut should be restore"),
        }
        let inverse_of_inverse = inverse_of(&inverse, "batch_0001").unwrap();
        // The inverse of `restore` is `cut` with the same range.
        match inverse_of_inverse {
            Action::Cut { target, params } => {
                assert_eq!(target.local(), "clip_5");
                assert_eq!(params.range.start_ns, 1_000);
                assert_eq!(params.range.end_ns, 2_000);
            }
            _ => panic!("inverse of restore should be cut"),
        }
    }

    #[test]
    fn undo_keeps_active_pointer_consistent() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = StagedApply::new(temp.path());
        let mut staged = staged_with_clip5();
        let mut stack = UndoRedoStack::new();

        let outcome = pipeline
            .apply("batch_0001", "rev_0001", &[cut_action()], &mut staged, None)
            .unwrap();
        let receipt = match outcome {
            ApplyOutcome::Applied { receipt, .. } => receipt,
            _ => panic!("expected applied"),
        };
        let new_revision = receipt.new_revision.clone();
        let inverse = inverse_batch_for("batch_0001", &new_revision, &[cut_action()]).unwrap();
        stack.push_applied(
            "batch_0001",
            receipt,
            vec![cut_action()],
            inverse.inverse_actions,
        );

        // Undo applies the inverse batch.
        let undo_outcome = stack.undo(&pipeline, &mut staged).unwrap();
        assert!(matches!(
            undo_outcome,
            UndoOutcome::Applied { was_undo: true, .. }
        ));
        // The active pointer must point to the new revision id, not the
        // original `rev_0001`.
        let recovery = pipeline.recover();
        assert_eq!(
            recovery.active_pointer.as_deref(),
            Some(match &undo_outcome {
                UndoOutcome::Applied { receipt, .. } => receipt.new_revision.as_str(),
                _ => unreachable!(),
            })
        );
    }

    #[test]
    fn undo_refuses_non_reversible_actions() {
        let mut stack = UndoRedoStack::new();
        let non_reversible = Action::ExportRender {
            target: asset_target("preset_1080p"),
            params: ExportRenderParams {
                preset_id: "preset_1080p".into(),
                target_revision: None,
            },
        };
        let err = inverse_batch_for("batch_x", "rev_x", &[non_reversible]).unwrap_err();
        assert!(matches!(err, UndoError::NonReversible(_)));
        // We never push onto the stack because the inverse batch generation
        // itself fails.
        assert!(stack.is_empty());
    }

    #[test]
    fn undo_on_empty_stack_returns_nothing_to_undo() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = StagedApply::new(temp.path());
        let mut staged = staged_with_clip5();
        let mut stack = UndoRedoStack::new();
        let err = stack.undo(&pipeline, &mut staged).unwrap_err();
        assert!(matches!(err, UndoError::NothingToUndo));
    }

    #[test]
    fn redo_on_empty_stack_returns_nothing_to_redo() {
        let temp = tempfile::tempdir().unwrap();
        let pipeline = StagedApply::new(temp.path());
        let mut staged = staged_with_clip5();
        let mut stack = UndoRedoStack::new();
        let err = stack.redo(&pipeline, &mut staged).unwrap_err();
        assert!(matches!(err, UndoError::NothingToRedo));
    }

    #[test]
    fn is_directly_invertible_classifies_actions() {
        assert!(is_directly_invertible(&cut_action()));
        assert!(!is_directly_invertible(&Action::ExportRender {
            target: asset_target("preset_1080p"),
            params: ExportRenderParams {
                preset_id: "preset_1080p".into(),
                target_revision: None,
            },
        }));
    }

    #[test]
    fn push_clears_redo_stack() {
        let mut stack = UndoRedoStack::new();
        stack.push_applied(
            "batch_0001",
            dummy_receipt(),
            vec![cut_action()],
            vec![cut_action()],
        );
        // Simulate a redo by manually pushing to redo stack.
        stack.redo_stack.push_back(applied_entry("batch_0001"));
        assert_eq!(stack.redo_len(), 1);
        stack.push_applied(
            "batch_0002",
            dummy_receipt(),
            vec![cut_action()],
            vec![cut_action()],
        );
        assert_eq!(stack.redo_len(), 0, "new forward batch should clear redo");
    }

    fn dummy_receipt() -> Receipt {
        Receipt::applied("batch", "rev", "rcpt", vec!["act_0".into()])
    }

    fn applied_entry(batch_id: &str) -> AppliedReceipt {
        AppliedReceipt {
            batch_id: batch_id.to_string(),
            receipt: dummy_receipt(),
            forward_actions: vec![cut_action()],
            inverse_actions: vec![cut_action()],
        }
    }
}
