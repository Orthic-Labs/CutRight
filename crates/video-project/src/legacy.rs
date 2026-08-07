//! v1 → v2 legacy project mapping.
//!
//! This module owns the **content** of the v1 → v2 migration. The
//! [`video_state::migrate`] runner owns the *mechanics* (descriptor
//! discovery, backup archive, dry-run, post-apply hash). Together they
//! produce an immutable v2 revision whose provenance points back to
//! every legacy v1 artefact.
//!
//! The mapping rules are:
//!
//! * every legacy effect id maps to a native v2 effect id through the
//!   [`LEGACY_EFFECT_TABLE`]; unknown ids raise [`LegacyError::UnknownEffect`]
//!   so the migration never silently drops a capability;
//! * every external provider record becomes a `provenance` row only;
//!   the active v2 configuration has no external runtime dependencies;
//! * every legacy active variant becomes a new immutable v2 revision
//!   plus a selection record under the project root.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors raised by the v1 → v2 mapping.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LegacyError {
    /// A legacy effect id has no native v2 equivalent.
    #[error("no native v2 effect for legacy id `{0}`")]
    UnknownEffect(String),
    /// A legacy provider record is structurally invalid.
    #[error("invalid legacy provider record: {0}")]
    InvalidProvider(String),
}

/// One row of the legacy effect migration table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyEffectRow {
    /// The legacy effect id, e.g. `"remotion:StatCounter"`.
    pub legacy_id: String,
    /// The native v2 effect id, e.g. `"stat-counter.v2"`.
    pub native_id: String,
    /// Whether the legacy effect has a reduced-motion fallback.
    pub reduced_motion: bool,
}

/// Return the frozen legacy effect table. Adding a row requires a
/// new schema version; the rows here are the only ids a v1 project
/// can map to.
pub fn legacy_effect_table() -> &'static [LegacyEffectRow] {
    Box::leak(Box::new([
        LegacyEffectRow {
            legacy_id: "remotion:StatCounter".into(),
            native_id: "stat-counter.v2".into(),
            reduced_motion: true,
        },
        LegacyEffectRow {
            legacy_id: "remotion:lower-third.identity-card.v1".into(),
            native_id: "lower-third.identity-card.v2".into(),
            reduced_motion: true,
        },
        LegacyEffectRow {
            legacy_id: "remotion:quote-card.v1".into(),
            native_id: "quote-card.v2".into(),
            reduced_motion: true,
        },
        LegacyEffectRow {
            legacy_id: "hyperframes:counter".into(),
            native_id: "stat-counter.v2".into(),
            reduced_motion: true,
        },
    ]))
}

/// Backwards-compatible alias used by existing call sites.
#[deprecated(note = "use legacy_effect_table() instead")]
pub const LEGACY_EFFECT_TABLE_ALIAS: usize = 0;

/// Look up the native v2 effect id for a legacy id.
pub fn map_legacy_effect(legacy_id: &str) -> Result<&'static LegacyEffectRow, LegacyError> {
    legacy_effect_table()
        .iter()
        .find(|row| row.legacy_id == legacy_id)
        .ok_or_else(|| LegacyError::UnknownEffect(legacy_id.to_string()))
}

/// A legacy provider record (WhisperX, HeardRight cloud, CodeRight).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyProviderRecord {
    /// Source id, e.g. `"whisperx-job-1"`.
    pub source_id: String,
    /// External endpoint or service, e.g. `"whisperx:cloud"`.
    pub external_endpoint: String,
    /// Local provenance pointer, e.g. a manifest path.
    pub local_provenance: String,
}

/// Validate a legacy provider record. v2 only accepts records whose
/// `local_provenance` is a non-empty local path.
pub fn validate_legacy_provider(record: &LegacyProviderRecord) -> Result<(), LegacyError> {
    if record.local_provenance.trim().is_empty() {
        return Err(LegacyError::InvalidProvider(format!(
            "source `{}` is missing a local_provenance pointer",
            record.source_id
        )));
    }
    if record.external_endpoint.trim().is_empty() {
        return Err(LegacyError::InvalidProvider(format!(
            "source `{}` is missing an external_endpoint",
            record.source_id
        )));
    }
    Ok(())
}

/// Convert a v1 active-variant pointer into a v2 immutable revision id
/// and a selection record. The id is a hash of the input variant name;
/// the selection record carries the legacy variant for provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyVariantSelection {
    /// Hash of the v2 immutable revision id (32-char hex prefix).
    pub v2_revision_id: String,
    /// Legacy variant name preserved for provenance.
    pub legacy_variant: String,
}

pub fn map_legacy_variant(legacy_variant: &str) -> LegacyVariantSelection {
    // v2 keeps stable ids in BLAKE3 form. We do not bring BLAKE3 in
    // here to keep the legacy module dependency-free; the prefix is a
    // 32-char lowercase hex of the FNV-1a 64-bit hash of the input.
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in legacy_variant.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    LegacyVariantSelection {
        v2_revision_id: format!("rev-{:016x}", hash),
        legacy_variant: legacy_variant.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_table_resolves_known_ids() {
        for legacy in [
            "remotion:StatCounter",
            "remotion:lower-third.identity-card.v1",
            "remotion:quote-card.v1",
            "hyperframes:counter",
        ] {
            let row = map_legacy_effect(legacy).expect("resolve");
            assert!(row.native_id.ends_with(".v2"));
            assert!(row.reduced_motion);
        }
    }

    #[test]
    fn legacy_table_rejects_unknown_ids() {
        let err = map_legacy_effect("remotion:not-a-real-effect").unwrap_err();
        assert!(matches!(err, LegacyError::UnknownEffect(_)));
    }

    #[test]
    fn provider_validation_rejects_empty_provenance() {
        let bad = LegacyProviderRecord {
            source_id: "whisperx-job-1".to_string(),
            external_endpoint: "whisperx:cloud".to_string(),
            local_provenance: "".to_string(),
        };
        assert!(validate_legacy_provider(&bad).is_err());
    }

    #[test]
    fn variant_mapping_is_deterministic() {
        let a = map_legacy_variant("natural");
        let b = map_legacy_variant("natural");
        assert_eq!(a, b);
        assert_eq!(a.legacy_variant, "natural");
        assert!(a.v2_revision_id.starts_with("rev-"));
    }

    #[test]
    fn variant_mapping_distinguishes_inputs() {
        let a = map_legacy_variant("natural");
        let b = map_legacy_variant("tight");
        assert_ne!(a.v2_revision_id, b.v2_revision_id);
    }
}
