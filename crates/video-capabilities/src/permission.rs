//! Permission set types — the eight least-privilege scopes frozen by
//! `V2-ACTION-PERMISSIONS.md` (CR-V2-B2-004).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::error::RegistryResult;

/// The eight scopes. Order is part of the public contract — DO NOT reorder
/// without updating `V2-ACTION-PERMISSIONS.md`, the codegen output, and every
/// downstream gate.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// Read evidence nodes / transcripts / candidates.
    EvidenceRead,
    /// Produce asset plans (candidates, rough cuts).
    AssetPlan,
    /// Read timeline state.
    TimelineRead,
    /// Write timeline state.
    TimelineWrite,
    /// Schedule / run render jobs.
    Render,
    /// Run export jobs.
    Export,
    /// Read / write settings / preferences.
    Settings,
    /// Manage runtime packs.
    PackManage,
}

/// The full list of scopes, in the frozen order.
pub const SCOPES: &[Scope] = &[
    Scope::EvidenceRead,
    Scope::AssetPlan,
    Scope::TimelineRead,
    Scope::TimelineWrite,
    Scope::Render,
    Scope::Export,
    Scope::Settings,
    Scope::PackManage,
];

impl Scope {
    /// Stable snake_case string used by serde + drift detection.
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::EvidenceRead => "evidence_read",
            Scope::AssetPlan => "asset_plan",
            Scope::TimelineRead => "timeline_read",
            Scope::TimelineWrite => "timeline_write",
            Scope::Render => "render",
            Scope::Export => "export",
            Scope::Settings => "settings",
            Scope::PackManage => "pack_manage",
        }
    }
}

/// Stable, opaque id of a permission set document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(transparent)]
pub struct PermissionSetId(pub String);

impl PermissionSetId {
    /// Construct from an owned string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PermissionSetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// One grant entry inside a permission set document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct PermissionGrant {
    /// The capability id this grant applies to (may be a literal id or a
    /// wildcard prefix).
    pub capability: String,
    /// The scope granted.
    pub scope: Scope,
}

/// Schema id for `cutright.permission_set/v1`.
pub const PERMISSION_SET_SCHEMA: &str = "cutright.permission_set/v1";

/// Parsed, in-memory representation of a permission-set document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PermissionSet {
    /// Always [`PERMISSION_SET_SCHEMA`].
    pub schema: String,
    /// Stable id (`permission_set_id`).
    pub permission_set_id: String,
    /// Grants (capability → scope). Sorted by capability then scope.
    pub grants: Vec<PermissionGrant>,
}

impl PermissionSet {
    /// Load + parse from a UTF-8 JSON file.
    pub fn load(path: impl AsRef<std::path::Path>) -> RegistryResult<Self> {
        let path_ref = path.as_ref();
        let bytes = std::fs::read(path_ref).map_err(|source| crate::error::RegistryError::Io {
            path: path_ref.to_path_buf(),
            source,
        })?;
        let value: Self = serde_json::from_slice(&bytes)?;
        if value.schema != PERMISSION_SET_SCHEMA {
            return Err(crate::error::RegistryError::InvalidEntry {
                index: 0,
                capability_id: value.permission_set_id.clone(),
                reason: format!(
                    "permission set schema {} does not match {}",
                    value.schema, PERMISSION_SET_SCHEMA
                ),
            });
        }
        Ok(value)
    }

    /// Return the set of scopes granted for a single capability. The lookup is
    /// exact-match by capability id; wildcard prefixes are not used by the v2
    /// registry.
    pub fn scopes_for(&self, capability_id: &str) -> BTreeSet<Scope> {
        let mut out = BTreeSet::new();
        for grant in &self.grants {
            if grant.capability == capability_id {
                out.insert(grant.scope);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_strings_round_trip() {
        for scope in SCOPES {
            let s = scope.as_str();
            let de: Scope = serde_json::from_str(&format!("\"{s}\"")).unwrap();
            assert_eq!(*scope, de);
        }
    }

    #[test]
    fn permission_set_loads_v1_document() {
        let json = serde_json::json!({
            "schema": PERMISSION_SET_SCHEMA,
            "permission_set_id": "pset.editorial_engine",
            "grants": [
                { "capability": "timeline.cut", "scope": "timeline_read" },
                { "capability": "timeline.cut", "scope": "timeline_write" }
            ]
        });
        let doc: PermissionSet = serde_json::from_value(json).unwrap();
        let scopes = doc.scopes_for("timeline.cut");
        assert_eq!(scopes.len(), 2);
        assert!(scopes.contains(&Scope::TimelineRead));
        assert!(scopes.contains(&Scope::TimelineWrite));
    }

    #[test]
    fn permission_set_rejects_unknown_schema() {
        let bad = serde_json::json!({
            "schema": "cutright.permission_set/v9",
            "permission_set_id": "pset.bad",
            "grants": []
        });
        let err = PermissionSet::load_json_value(bad).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("does not match"));
    }

    #[test]
    fn permission_set_rejects_unknown_field() {
        let bad = serde_json::json!({
            "schema": PERMISSION_SET_SCHEMA,
            "permission_set_id": "pset.bad",
            "grants": [],
            "extra": true
        });
        let res: Result<PermissionSet, _> = serde_json::from_value(bad);
        assert!(res.is_err());
    }

    impl PermissionSet {
        fn load_json_value(value: serde_json::Value) -> RegistryResult<Self> {
            let bytes = serde_json::to_vec(&value).expect("serialize");
            let parsed: Self = serde_json::from_slice(&bytes)?;
            if parsed.schema != PERMISSION_SET_SCHEMA {
                return Err(crate::error::RegistryError::InvalidEntry {
                    index: 0,
                    capability_id: parsed.permission_set_id.clone(),
                    reason: format!(
                        "permission set schema {} does not match {}",
                        parsed.schema, PERMISSION_SET_SCHEMA
                    ),
                });
            }
            Ok(parsed)
        }
    }
}
