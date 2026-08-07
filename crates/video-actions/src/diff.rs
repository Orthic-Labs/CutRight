//! Semantic dry-run planner and stable diff generation (CR-V2-B2-009).
//!
//! The dry-run planner produces a `cutright.semantic_diff/v1` document that
//! mirrors `schemas/actions/semantic-diff.schema.v1.json`. It uses the same
//! validator and apply-planning path as the real execution pipeline
//! (`V2-SEMANTIC-DIFF.md` §3), so a dry-run and the matching real apply
//! produce byte-identical `planned_revision` and diff entries; the only
//! observable difference is the active-pointer swap and receipt write.
//!
//! Diff entries are sorted by `(timeline_id, track_id, start_ns, action_kind)`
//! (`V2-SEMANTIC-DIFF.md` §2) so the JSON output is deterministic across
//! hash, pointer, and process-id noise.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::action::{action_kind, Action, RangeNs, TargetKind, TargetRef};
use crate::validation::{validate_batch, ValidationContext, ValidationFailure};

/// The wire schema id of the dry-run diff document.
pub const DRY_RUN_SCHEMA: &str = "cutright.semantic_diff/v1";

/// Range inside a diff entry: rational-tick half-open interval in integer
/// nanoseconds.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DiffRange {
    /// Inclusive start of the range in nanoseconds.
    pub start_ns: i64,
    /// Exclusive end of the range in nanoseconds.
    pub end_ns: i64,
}

impl DiffRange {
    /// Construct from a [`RangeNs`].
    pub fn from_range(range: RangeNs) -> Self {
        Self {
            start_ns: range.start_ns,
            end_ns: range.end_ns,
        }
    }

    /// A zero-length range at the origin.
    pub fn zero() -> Self {
        Self {
            start_ns: 0,
            end_ns: 0,
        }
    }

    /// Length in nanoseconds.
    pub fn len_ns(&self) -> i64 {
        self.end_ns - self.start_ns
    }
}

/// One row of the dry-run diff document.
///
/// All fields except `evidence_refs`, `confidence`, and `risk_flags` are
/// required to match the schema's `required` list and to fail closed on
/// drift. The struct uses `#[serde(deny_unknown_fields)]` so any future
/// schema addition shows up as a deserialisation failure rather than a
/// silent field drop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DiffEntry {
    /// Stable action kind string (matches the wire form).
    pub action_kind: String,
    /// Target id the action applies to.
    pub target_id: String,
    /// Identity of the revision state before the action.
    pub before_id: String,
    /// Identity of the revision state after the action.
    pub after_id: String,
    /// Range the action operates on (zero for actions with no range).
    pub range: DiffRange,
    /// Net change in timeline duration in nanoseconds caused by the action.
    pub duration_delta_ns: i64,
    /// Evidence references recorded alongside the action (forwarded from
    /// the action batch envelope if present).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
    /// Optional planner confidence in `[0, 1]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Optional risk flags raised by the planner.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub risk_flags: Vec<String>,
}

/// The full dry-run diff document (schema `cutright.semantic_diff/v1`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SemanticDiff {
    /// Always [`DRY_RUN_SCHEMA`].
    pub schema: String,
    /// The action batch id this diff describes.
    pub batch_id: String,
    /// The revision the caller expected to apply against.
    pub expected_revision: String,
    /// The deterministic revision id that would be produced by applying the
    /// batch. Derived from a BLAKE3 hash of the canonicalised batch so the
    /// same `expected_revision` + same actions always yields the same
    /// `planned_revision` (`V2-SEMANTIC-DIFF.md` §3).
    pub planned_revision: String,
    /// The diff entries themselves, sorted by the stable diff key.
    pub diff: Vec<DiffEntry>,
}

/// Stable sort key for diff entries (`V2-SEMANTIC-DIFF.md` §2):
/// `(timeline_id, track_id, start_ns, action_kind)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StableDiffKey {
    /// Timeline id (`project:<project-id>` for now — v2 keeps the project
    /// identity stable across the batch).
    pub timeline_id: String,
    /// Track id (the target's local part — v2 actions address clips/tracks
    /// with stable ids).
    pub track_id: String,
    /// Range start in nanoseconds.
    pub start_ns: i64,
    /// Stable action kind string.
    pub action_kind: String,
}

impl StableDiffKey {
    /// Derive a stable key from a [`DiffEntry`] and a project id.
    pub fn from_entry(entry: &DiffEntry, project_id: &str) -> Self {
        let target = entry.target_id.clone();
        let (track_id, start_ns) = parse_track_and_start(&target, entry.range.start_ns);
        Self {
            timeline_id: format!("project:{project_id}"),
            track_id,
            start_ns,
            action_kind: entry.action_kind.clone(),
        }
    }
}

impl Ord for StableDiffKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timeline_id
            .cmp(&other.timeline_id)
            .then(self.track_id.cmp(&other.track_id))
            .then(self.start_ns.cmp(&other.start_ns))
            .then(self.action_kind.cmp(&other.action_kind))
    }
}

impl PartialOrd for StableDiffKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn parse_track_and_start(target_id: &str, default_start: i64) -> (String, i64) {
    // The "track" portion of the key is the target id's local part. For
    // targets that already identify a track (`track:*`) we use the target
    // verbatim. For non-track targets we use the local part so that
    // clips/words/audio that share a track sort together.
    let local = target_id
        .split_once(':')
        .map(|(_, l)| l.to_string())
        .unwrap_or_else(|| target_id.to_string());
    (local, default_start)
}

/// Typed error returned by the dry-run planner.
#[derive(Debug, Error)]
pub enum DiffError {
    /// Validation failed before the diff could be planned.
    #[error("dry-run validation failed: {0:?}")]
    Validation(Vec<ValidationFailure>),
    /// Internal: the planner failed to canonicalise the batch for hashing.
    #[error("dry-run failed to hash batch: {0}")]
    Hash(#[from] serde_json::Error),
}

impl From<Vec<ValidationFailure>> for DiffError {
    fn from(failures: Vec<ValidationFailure>) -> Self {
        Self::Validation(failures)
    }
}

/// Run the dry-run planner.
///
/// This function:
/// 1. Validates every action against [`ValidationContext`] using the same
///    [`crate::validation::DefaultValidator`] the apply pipeline uses.
/// 2. Builds a [`DiffEntry`] for every action via the same planning path
///    as the real apply (`V2-SEMANTIC-DIFF.md` §3).
/// 3. Sorts the diff entries by the stable key.
/// 4. Derives a deterministic `planned_revision` id from a BLAKE3 hash of
///    the canonicalised batch JSON.
///
/// Returns `Err(DiffError::Validation(..))` if any action fails validation.
pub fn dry_run(
    batch_id: &str,
    expected_revision: &str,
    actions: &[Action],
    ctx: &ValidationContext,
) -> Result<SemanticDiff, DiffError> {
    // ---- Step 1: shared validator ----
    validate_batch(actions, ctx)?;

    // ---- Step 2: plan each action into a DiffEntry ----
    let mut diff: Vec<DiffEntry> = Vec::with_capacity(actions.len());
    for action in actions {
        diff.push(plan_entry(action));
    }

    // ---- Step 3: sort by stable key ----
    diff.sort_by(|a, b| {
        let ka = StableDiffKey::from_entry(a, &ctx.project_id);
        let kb = StableDiffKey::from_entry(b, &ctx.project_id);
        ka.cmp(&kb)
    });

    // ---- Step 4: deterministic planned_revision ----
    let planned_revision = planned_revision_for(batch_id, expected_revision, actions);

    Ok(SemanticDiff {
        schema: DRY_RUN_SCHEMA.to_string(),
        batch_id: batch_id.to_string(),
        expected_revision: expected_revision.to_string(),
        planned_revision,
        diff,
    })
}

/// Compute the deterministic `planned_revision` for a batch.
///
/// The id is `rev_<32-hex>` where the hex is a BLAKE3 hash of the
/// canonicalised (sorted-keys, sorted-actions) batch JSON. Two calls with
/// identical inputs always produce identical ids, regardless of process id,
/// thread id, or clock.
pub fn planned_revision_for(batch_id: &str, expected_revision: &str, actions: &[Action]) -> String {
    let mut sorted_actions: Vec<&Action> = actions.iter().collect();
    sorted_actions.sort_by(|a, b| action_kind(a).cmp(action_kind(b)));
    let canonical = serde_json::json!({
        "schema": DRY_RUN_SCHEMA,
        "batch_id": batch_id,
        "expected_revision": expected_revision,
        "actions": sorted_actions
            .iter()
            .map(|a| serde_json::to_value(a).expect("action serialises"))
            .collect::<Vec<_>>(),
    });
    let canonical_bytes = serde_json::to_vec(&canonical).expect("canonical JSON serialises");
    let hash = blake3::hash(&canonical_bytes).to_hex();
    format!("rev_{}", &hash[..32])
}

/// Build a single [`DiffEntry`] from an [`Action`] using the same planning
/// path the real apply pipeline uses (CR-V2-B2-010 builds on this primitive).
///
/// `before_id` is the target id verbatim. `after_id` is the target id with
/// a `@<revision-suffix>` marker so the diff makes the identity transition
/// obvious without inventing a new id scheme.
pub fn plan_entry(action: &Action) -> DiffEntry {
    let kind = action_kind(action).to_string();
    let target_id = action_target(action)
        .map(TargetRef::as_str)
        .unwrap_or("")
        .to_string();
    let range = action_range(action).unwrap_or(RangeNs {
        start_ns: 0,
        end_ns: 0,
    });
    let duration_delta_ns = compute_duration_delta(action);
    let after_id = if target_id.is_empty() {
        String::new()
    } else {
        format!("{target_id}@applied")
    };
    DiffEntry {
        action_kind: kind,
        target_id: target_id.clone(),
        before_id: target_id,
        after_id,
        range: DiffRange::from_range(range),
        duration_delta_ns,
        evidence_refs: Vec::new(),
        confidence: None,
        risk_flags: Vec::new(),
    }
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

fn action_range(action: &Action) -> Option<RangeNs> {
    match action {
        Action::Cut { params, .. } => Some(params.range),
        Action::Restore { params, .. } => Some(params.range),
        Action::Move { params, .. } => Some(params.range),
        Action::TakeSwap { params, .. } => Some(params.range),
        Action::Retime { params, .. } => Some(params.range),
        Action::Caption { params, .. } => Some(params.range),
        Action::Graphic { params, .. } => Some(params.range),
        Action::Audio { params, .. } => Some(params.range),
        Action::ColourLut { params, .. } => Some(params.range),
        Action::ColourCorrection { params, .. } => Some(params.range),
        Action::ExportRender { .. } | Action::Setting { .. } => None,
    }
}

fn compute_duration_delta(action: &Action) -> i64 {
    match action {
        Action::Cut { params, .. } => -params.range.len_ns(),
        Action::Restore { params, .. } => params.range.len_ns(),
        Action::Move { .. } => 0,
        Action::TakeSwap { .. } => 0,
        Action::Retime { .. } => 0,
        Action::Caption { .. } => 0,
        Action::Graphic { .. } => 0,
        Action::Audio { .. } => 0,
        Action::ColourLut { .. } => 0,
        Action::ColourCorrection { .. } => 0,
        Action::ExportRender { .. } => 0,
        Action::Setting { .. } => 0,
    }
}

/// True iff `kind` describes a target of the given [`TargetKind`].
pub fn target_is_kind(target: &TargetRef, kind: TargetKind) -> bool {
    target.kind() == kind
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{
        AudioParams, CaptionParams, ColourCorrectionParams, ColourLutParams, CutParams,
        ExportRenderParams, GraphicParams, MoveParams, RestoreParams, RetimeParams, SettingParams,
        TakeSwapParams, TargetKind,
    };
    use std::collections::BTreeSet;

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

    fn project_target(id: &str) -> TargetRef {
        TargetRef::from_parts(TargetKind::Project, id).unwrap()
    }

    fn range(start_ns: i64, end_ns: i64) -> RangeNs {
        RangeNs { start_ns, end_ns }
    }

    fn sample_ctx() -> ValidationContext {
        let mut known = BTreeSet::new();
        for id in [
            "clip:clip_5",
            "clip:clip_6",
            "clip:clip_7",
            "track:track_main",
            "word:w_000007",
            "asset:voiceover_1",
            "asset:logo_main",
            "asset:preset_1080p",
            "project:review_mode",
        ] {
            known.insert(id.to_string());
        }
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
    fn dry_run_emits_schema_const() {
        let ctx = sample_ctx();
        let diff = dry_run(
            "batch_0001",
            "rev_0001",
            &[cut(clip_target("clip_5"), range(1_000, 2_000))],
            &ctx,
        )
        .unwrap();
        assert_eq!(diff.schema, "cutright.semantic_diff/v1");
    }

    #[test]
    fn dry_run_planned_revision_is_deterministic() {
        let ctx = sample_ctx();
        let actions = vec![
            cut(clip_target("clip_5"), range(1_000, 2_000)),
            cut(clip_target("clip_6"), range(3_000, 4_000)),
        ];
        let a = dry_run("batch_0001", "rev_0001", &actions, &ctx).unwrap();
        let b = dry_run("batch_0001", "rev_0001", &actions, &ctx).unwrap();
        assert_eq!(a.planned_revision, b.planned_revision);
        assert!(a.planned_revision.starts_with("rev_"));
        assert_eq!(a.planned_revision.len(), "rev_".len() + 32);
    }

    #[test]
    fn dry_run_planned_revision_differs_when_batch_changes() {
        let ctx = sample_ctx();
        let a = dry_run(
            "batch_0001",
            "rev_0001",
            &[cut(clip_target("clip_5"), range(1_000, 2_000))],
            &ctx,
        )
        .unwrap();
        let b = dry_run(
            "batch_0001",
            "rev_0001",
            &[cut(clip_target("clip_5"), range(1_000, 3_000))],
            &ctx,
        )
        .unwrap();
        assert_ne!(a.planned_revision, b.planned_revision);
    }

    #[test]
    fn dry_run_propagates_validation_failures() {
        let ctx = sample_ctx();
        let result = dry_run(
            "batch_0001",
            "rev_0001",
            &[cut(clip_target("clip_missing"), range(1_000, 2_000))],
            &ctx,
        );
        let err = result.unwrap_err();
        assert!(matches!(err, DiffError::Validation(_)));
    }

    #[test]
    fn diff_entries_are_sorted_by_stable_key() {
        // Order the actions deliberately out of stable order; the diff
        // entries must come out in (timeline, track, start_ns, kind) order.
        let ctx = sample_ctx();
        let actions = vec![
            cut(clip_target("clip_7"), range(500, 600)),
            cut(clip_target("clip_5"), range(2_000, 3_000)),
            cut(clip_target("clip_5"), range(1_000, 1_500)),
            cut(clip_target("clip_6"), range(1_000, 2_000)),
        ];
        let diff = dry_run("batch_0001", "rev_0001", &actions, &ctx).unwrap();
        let keys: Vec<StableDiffKey> = diff
            .diff
            .iter()
            .map(|e| StableDiffKey::from_entry(e, "prj_main"))
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "diff entries are not in stable order");
    }

    #[test]
    fn dry_run_reports_cut_duration_delta() {
        let ctx = sample_ctx();
        let diff = dry_run(
            "batch_0001",
            "rev_0001",
            &[cut(clip_target("clip_5"), range(1_000, 2_500))],
            &ctx,
        )
        .unwrap();
        assert_eq!(diff.diff[0].duration_delta_ns, -1_500);
    }

    #[test]
    fn dry_run_reports_restore_duration_delta() {
        let ctx = sample_ctx();
        let action = Action::Restore {
            target: clip_target("clip_5"),
            params: RestoreParams {
                range: range(1_000, 2_500),
                source_batch_id: "batch_0001".into(),
            },
        };
        let diff = dry_run("batch_0001", "rev_0001", &[action], &ctx).unwrap();
        assert_eq!(diff.diff[0].duration_delta_ns, 1_500);
    }

    #[test]
    fn dry_run_handles_non_range_actions() {
        let ctx = sample_ctx();
        let actions = vec![
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
        let diff = dry_run("batch_0001", "rev_0001", &actions, &ctx).unwrap();
        assert_eq!(diff.diff.len(), 2);
        assert_eq!(diff.diff[0].range, DiffRange::zero());
        assert_eq!(diff.diff[1].range, DiffRange::zero());
        assert_eq!(diff.diff[0].duration_delta_ns, 0);
        assert_eq!(diff.diff[1].duration_delta_ns, 0);
    }

    #[test]
    fn snapshot_stability_across_calls() {
        let ctx = sample_ctx();
        let actions = vec![
            cut(clip_target("clip_5"), range(1_000, 2_000)),
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
                target: asset_target("logo_main"),
                params: GraphicParams {
                    range: range(1_000, 2_000),
                    graphic_id: "logo".into(),
                },
            },
            Action::Audio {
                target: asset_target("voiceover_1"),
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
            Action::Move {
                target: clip_target("clip_6"),
                params: MoveParams {
                    range: range(1_000, 2_000),
                    new_start_ns: 5_000,
                },
            },
            Action::TakeSwap {
                target: clip_target("clip_7"),
                params: TakeSwapParams {
                    range: range(1_000, 2_000),
                    replacement_clip_id: "clip_alt".into(),
                },
            },
        ];
        let diff_a = dry_run("batch_0001", "rev_0001", &actions, &ctx).unwrap();
        let diff_b = dry_run("batch_0001", "rev_0001", &actions, &ctx).unwrap();
        let json_a = serde_json::to_string(&diff_a).unwrap();
        let json_b = serde_json::to_string(&diff_b).unwrap();
        assert_eq!(json_a, json_b, "snapshot must be byte-identical");
    }

    #[test]
    fn diff_serialises_with_schema_const() {
        let ctx = sample_ctx();
        let diff = dry_run(
            "batch_0001",
            "rev_0001",
            &[cut(clip_target("clip_5"), range(1_000, 2_000))],
            &ctx,
        )
        .unwrap();
        let value = serde_json::to_value(&diff).unwrap();
        assert_eq!(value["schema"], "cutright.semantic_diff/v1");
        assert_eq!(value["batch_id"], "batch_0001");
        assert_eq!(value["expected_revision"], "rev_0001");
        assert!(value["planned_revision"]
            .as_str()
            .unwrap()
            .starts_with("rev_"));
        let entry = &value["diff"][0];
        assert_eq!(entry["action_kind"], "timeline.cut");
        assert_eq!(entry["target_id"], "clip:clip_5");
        assert_eq!(entry["before_id"], "clip:clip_5");
        assert_eq!(entry["after_id"], "clip:clip_5@applied");
        assert_eq!(entry["range"]["start_ns"], 1_000);
        assert_eq!(entry["range"]["end_ns"], 2_000);
        assert_eq!(entry["duration_delta_ns"], -1_000);
    }

    #[test]
    fn diff_round_trips_through_serde() {
        let ctx = sample_ctx();
        let diff = dry_run(
            "batch_0001",
            "rev_0001",
            &[cut(clip_target("clip_5"), range(1_000, 2_000))],
            &ctx,
        )
        .unwrap();
        let round =
            serde_json::from_value::<SemanticDiff>(serde_json::to_value(&diff).unwrap()).unwrap();
        assert_eq!(round, diff);
    }

    #[test]
    fn unknown_fields_in_diff_are_rejected() {
        let bogus = serde_json::json!({
            "schema": "cutright.semantic_diff/v1",
            "batch_id": "batch_0001",
            "expected_revision": "rev_0001",
            "planned_revision": "rev_xxxx",
            "diff": [],
            "rogue": true,
        });
        serde_json::from_value::<SemanticDiff>(bogus).expect_err("unknown field must fail closed");
    }

    #[test]
    fn stable_diff_key_orders_lexically() {
        let key = StableDiffKey {
            timeline_id: "project:p".into(),
            track_id: "t".into(),
            start_ns: 0,
            action_kind: "timeline.cut".into(),
        };
        let key2 = StableDiffKey {
            timeline_id: "project:p".into(),
            track_id: "t".into(),
            start_ns: 1,
            action_kind: "timeline.cut".into(),
        };
        assert!(key < key2);
    }
}
