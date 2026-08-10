//! v1 → v2 migration descriptors and helpers.
//!
//! This module groups the four frozen v1 → v2 migration steps:
//!
//! 1. `identity-map`     — translate v1 string ids into v2 stable ids.
//! 2. `ms-to-ns`         — keep millisecond fields, add rational-time fields
//!    (v2 keeps both representations and bounds them).
//! 3. `effect-table`     — translate legacy `Remotion:Effect` ids to the
//!    native effect registry; legacy effects become
//!    provenance records, not active capabilities.
//! 4. `provider-ledger`  — translate external provider records (WhisperX,
//!    HeardRight cloud, CodeRight) into a local-only
//!    provenance ledger; the active v2 configuration
//!    has zero external runtime dependencies.
//!
//! The runner is owned by `video-state::migrate`; this module only
//! provides the frozen descriptor set and a small helper that
//! `video-project::legacy` calls when emitting the migration receipt.

use serde::{Deserialize, Serialize};

use crate::migrate::{MigrationPlan, MigrationStep};

/// The from-version string used in v1 → v2 descriptors.
pub const FROM_VERSION: &str = "v1";
/// The to-version string used in v1 → v2 descriptors.
pub const TO_VERSION: &str = "v2";

/// A frozen v1 → v2 migration descriptor, identical in shape to the
/// descriptor files written under `fixtures/migrations/v1-to-v2/`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrozenStep {
    /// Source schema version.
    pub from: String,
    /// Target schema version.
    pub to: String,
    /// 1-based ordering within the plan.
    pub step: u32,
    /// Stable name.
    pub name: String,
    /// Whether the step is destructive.
    pub requires_backup: bool,
    /// JSON paths the step mutates.
    pub touched_fields: Vec<String>,
    /// Human-readable description.
    pub description: String,
}

impl From<FrozenStep> for MigrationStep {
    fn from(f: FrozenStep) -> Self {
        Self {
            from: f.from,
            to: f.to,
            step: f.step,
            name: f.name,
            requires_backup: f.requires_backup,
            touched_fields: f.touched_fields,
            description: f.description,
        }
    }
}

/// The frozen descriptor set. The order is part of the contract.
pub const FROZEN_STEPS: &[(&str, &str, bool, &[&str])] = &[
    (
        "identity-map",
        "Translate v1 string ids (e.g. \"clip_42\") into v2 stable BLAKE3 ids; bind them to a hash-chained log.",
        true,
        &[
            "candidates[*].id",
            "candidates[*].source_id",
            "finish.slots[*].id",
            "effects[*].id",
        ],
    ),
    (
        "ms-to-ns",
        "Co-exist millisecond and rational time; v2 keeps ms for compatibility and adds rational time fields with a frozen origin.",
        true,
        &[
            "candidates[*].start_ms",
            "candidates[*].end_ms",
            "edit.clips[*].t0_ms",
            "edit.clips[*].t1_ms",
        ],
    ),
    (
        "effect-table",
        "Map every legacy effect id (Remotion:StatCounter, HyperFrames:*) to a native v2 effect id; legacy ids become provenance rows under native effects.",
        true,
        &[
            "effects[*].legacy_id",
            "effects[*].native_id",
            "effects[*].provenance",
        ],
    ),
    (
        "provider-ledger",
        "Translate external provider records (WhisperX, HeardRight cloud, CodeRight) to a local-only provenance ledger; the active v2 configuration has zero external runtime dependencies.",
        false,
        &[
            "providers[*].kind",
            "providers[*].external_endpoint",
            "providers[*].local_provenance",
        ],
    ),
];

/// Build a [`MigrationPlan`] from the frozen descriptor set.
pub fn v1_to_v2_plan() -> MigrationPlan {
    let steps: Vec<MigrationStep> = FROZEN_STEPS
        .iter()
        .enumerate()
        .map(|(i, (name, desc, backup, fields))| MigrationStep {
            from: FROM_VERSION.to_string(),
            to: TO_VERSION.to_string(),
            step: (i + 1) as u32,
            name: (*name).to_string(),
            requires_backup: *backup,
            touched_fields: fields.iter().map(|s| (*s).to_string()).collect(),
            description: (*desc).to_string(),
        })
        .collect();
    let touched_fields: Vec<String> = steps
        .iter()
        .flat_map(|s| s.touched_fields.iter().cloned())
        .collect();
    let backup_count = steps.iter().filter(|s| s.requires_backup).count();
    MigrationPlan {
        from: FROM_VERSION.to_string(),
        to: TO_VERSION.to_string(),
        steps,
        touched_fields,
        backup_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_steps_are_contiguous_from_v1_to_v2() {
        let plan = v1_to_v2_plan();
        assert_eq!(plan.from, FROM_VERSION);
        assert_eq!(plan.to, TO_VERSION);
        assert_eq!(plan.steps.len(), FROZEN_STEPS.len());
        for (i, step) in plan.steps.iter().enumerate() {
            assert_eq!(step.step as usize, i + 1);
            assert!(!step.touched_fields.is_empty());
        }
        assert_eq!(plan.backup_count, 3);
    }

    #[test]
    fn frozen_steps_cover_required_axes() {
        let plan = v1_to_v2_plan();
        let mut all = String::new();
        for s in &plan.steps {
            all.push_str(&s.name);
            all.push('|');
        }
        assert!(all.contains("identity-map"));
        assert!(all.contains("ms-to-ns"));
        assert!(all.contains("effect-table"));
        assert!(all.contains("provider-ledger"));
    }
}
