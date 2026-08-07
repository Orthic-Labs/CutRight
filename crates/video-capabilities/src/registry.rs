//! Capability registry model + validator (CR-V2-B2-012).
//!
//! Frozen contracts:
//! - `schemas/capabilities/registry.schema.v1.json`
//! - `docs/architecture/V2-CAPABILITY-ACTION-CONTRACT.md`
//!
//! The registry validates:
//! 1. Top-level `schema_version == 1`.
//! 2. Every entry matches `cutright.capability/v1`.
//! 3. `capability_id` matches `^[a-z][a-z0-9_.]+$`.
//! 4. `version >= 1` (enforced by serde).
//! 5. `kind` is `read` or `mutation`.
//! 6. `owner_component` is a known component (see [`KNOWN_OWNER_COMPONENTS`]).
//! 7. `permission_set` is declared in a bundled permission-set document.
//! 8. Reads must declare `outputs.bounded == true` AND `outputs.windowed == true`.
//! 9. `(capability_id, version)` is unique across the document.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{RegistryError, RegistryResult};
use crate::permission::{PermissionSet, PermissionSetId};

/// Schema id every registry entry must declare.
pub const REGISTRY_SCHEMA: &str = "cutright.capability/v1";

/// Schema version the loader accepts (and increments when shapes change).
pub const REGISTRY_SCHEMA_VERSION: u32 = 1;

/// `owner_component` strings the registry treats as known. Adding a new
/// component is a deliberate cross-cutting change: it must be added here, in
/// the DAG, and in the v2-eval scripts at the same time.
pub const KNOWN_OWNER_COMPONENTS: &[&str] = &[
    "video-core",
    "video-actions",
    "video-state",
    "video-sessions",
    "video-capabilities",
    "video-project",
    "video-cli",
    "video-evidence",
    "video-jobs",
    "video-runtime",
    "video-media",
    "video-inference",
    "video-providers",
    "studio-tauri",
    "loopback-mcp",
];

/// `read` vs `mutation`. Mirrors `schemas/capabilities/registry.schema.v1.json`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    /// Read-only snapshot. Must be `bounded + windowed`.
    Read,
    /// Mutates a project revision; produces a receipt.
    Mutation,
}

/// Degradation status of a capability at runtime.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Degradation {
    /// Capability is fully available.
    Ok,
    /// Capability is available but with reduced fidelity.
    Degraded,
    /// Capability is unavailable in this runtime.
    Missing,
}

/// Optional output-shape hint. Required to be `bounded + windowed` on every
/// `read`; `mutation` entries usually leave these unset.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityOutputs {
    /// The output is bounded in cardinality / size.
    #[serde(default)]
    pub bounded: bool,
    /// The output is windowed in time (or any other axis).
    #[serde(default)]
    pub windowed: bool,
    /// Optional max item count (e.g. window size).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u32>,
}

/// Stable, opaque id of a capability (snake_case).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(transparent)]
pub struct CapabilityId(pub String);

impl CapabilityId {
    /// Construct from an owned string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// Return true iff the id matches `^[a-z][a-z0-9_.]+$`.
    pub fn is_well_formed(value: &str) -> bool {
        let mut chars = value.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        if !first.is_ascii_lowercase() {
            return false;
        }
        if value.len() < 2 {
            return false;
        }
        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
    }
}

impl std::fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A single capability entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    /// Always [`REGISTRY_SCHEMA`].
    pub schema: String,
    /// Stable, snake_case capability id.
    pub capability_id: CapabilityId,
    /// Monotone version. Two capabilities may share an id only if their
    /// versions differ.
    pub version: u32,
    /// `read` or `mutation`.
    pub kind: CapabilityKind,
    /// Owning crate / component (e.g. `video-actions`).
    pub owner_component: String,
    /// Permission-set id this capability inherits.
    pub permission_set: String,
    /// Typed input schema (JSON-schema-like fragment).
    pub inputs: serde_json::Value,
    /// Output shape (bounded/windowed for reads).
    pub outputs: CapabilityOutputs,
    /// Optional eval suite references.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub eval_suites: Vec<String>,
    /// Optional degradation state. Defaults to [`Degradation::Ok`] when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degradation: Option<Degradation>,
}

impl Capability {
    /// Return true iff this is a `read` capability whose outputs are
    /// `bounded + windowed`.
    pub fn is_well_formed_read(&self) -> bool {
        if self.kind != CapabilityKind::Read {
            return true;
        }
        self.outputs.bounded && self.outputs.windowed
    }
}

/// Wire-level document — the on-disk shape of the canonical registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RegistryDocument {
    /// Always [`REGISTRY_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Stable id of this registry document.
    pub registry_id: String,
    /// Human-readable note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Capability entries (order preserved for codegen determinism).
    pub capabilities: Vec<Capability>,
    /// Permission sets referenced by these capabilities (inlined so the
    /// registry document is self-contained).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permission_sets: Vec<PermissionSet>,
}

impl RegistryDocument {
    /// Parse + validate a registry document from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> RegistryResult<Self> {
        let doc: RegistryDocument = serde_json::from_slice(bytes)?;
        doc.validate()?;
        Ok(doc)
    }

    /// Load + validate a registry document from a UTF-8 JSON file.
    pub fn load(path: impl AsRef<Path>) -> RegistryResult<Self> {
        let path_ref = path.as_ref();
        let bytes = std::fs::read(path_ref).map_err(|source| RegistryError::Io {
            path: path_ref.to_path_buf(),
            source,
        })?;
        Self::from_bytes(&bytes)
    }

    /// Validate the document against every rule in
    /// `schemas/capabilities/registry.schema.v1.json` and the cross-checks in
    /// `V2-CAPABILITY-ACTION-CONTRACT.md` / `V2-ACTION-PERMISSIONS.md`.
    pub fn validate(&self) -> RegistryResult<()> {
        if self.schema_version != REGISTRY_SCHEMA_VERSION {
            return Err(RegistryError::UnsupportedSchemaVersion {
                found: self.schema_version,
                expected: REGISTRY_SCHEMA_VERSION,
            });
        }

        // Index permission sets by id, regardless of document order.
        let mut psets: BTreeMap<String, &PermissionSet> = BTreeMap::new();
        for pset in &self.permission_sets {
            if pset.schema != crate::permission::PERMISSION_SET_SCHEMA {
                return Err(RegistryError::InvalidEntry {
                    index: 0,
                    capability_id: pset.permission_set_id.clone(),
                    reason: format!(
                        "permission set schema {} does not match {}",
                        pset.schema,
                        crate::permission::PERMISSION_SET_SCHEMA
                    ),
                });
            }
            psets.insert(pset.permission_set_id.clone(), pset);
        }

        let mut seen: BTreeSet<(String, u32)> = BTreeSet::new();
        let known_owners: BTreeSet<&'static str> = KNOWN_OWNER_COMPONENTS.iter().copied().collect();

        for (index, cap) in self.capabilities.iter().enumerate() {
            if cap.schema != REGISTRY_SCHEMA {
                return Err(RegistryError::InvalidEntry {
                    index,
                    capability_id: cap.capability_id.0.clone(),
                    reason: format!(
                        "entry schema {} does not match {}",
                        cap.schema, REGISTRY_SCHEMA
                    ),
                });
            }
            if !CapabilityId::is_well_formed(&cap.capability_id.0) {
                return Err(RegistryError::InvalidEntry {
                    index,
                    capability_id: cap.capability_id.0.clone(),
                    reason: "capability_id must match ^[a-z][a-z0-9_.]+$".to_string(),
                });
            }
            if !known_owners.contains(cap.owner_component.as_str()) {
                return Err(RegistryError::UnknownOwnerComponent {
                    capability_id: cap.capability_id.0.clone(),
                    owner_component: cap.owner_component.clone(),
                });
            }
            if !psets.contains_key(&cap.permission_set) {
                return Err(RegistryError::DanglingPermissionSet {
                    capability_id: cap.capability_id.0.clone(),
                    permission_set: cap.permission_set.clone(),
                });
            }
            if !cap.is_well_formed_read() {
                return Err(RegistryError::ReadNotBoundedWindowed {
                    capability_id: cap.capability_id.0.clone(),
                });
            }
            let key = (cap.capability_id.0.clone(), cap.version);
            if !seen.insert(key.clone()) {
                return Err(RegistryError::DuplicateEntry {
                    capability_id: cap.capability_id.0.clone(),
                    version: cap.version,
                });
            }
        }
        Ok(())
    }

    /// Convert into a query-friendly [`CapabilityRegistry`].
    pub fn into_registry(self) -> CapabilityRegistry {
        let mut by_id: BTreeMap<CapabilityId, Capability> = BTreeMap::new();
        for cap in self.capabilities {
            by_id.insert(cap.capability_id.clone(), cap);
        }
        let mut permission_sets: BTreeMap<String, PermissionSet> = BTreeMap::new();
        for pset in self.permission_sets {
            permission_sets.insert(pset.permission_set_id.clone(), pset);
        }
        CapabilityRegistry {
            schema_version: self.schema_version,
            registry_id: self.registry_id,
            capabilities: by_id,
            permission_sets,
        }
    }
}

/// Query-friendly in-memory view of a validated registry.
#[derive(Debug, Clone)]
pub struct CapabilityRegistry {
    /// Schema version (always [`REGISTRY_SCHEMA_VERSION`] post-validation).
    pub schema_version: u32,
    /// Stable registry id.
    pub registry_id: String,
    /// All declared capabilities keyed by capability_id.
    pub capabilities: BTreeMap<CapabilityId, Capability>,
    /// All declared permission sets keyed by id.
    pub permission_sets: BTreeMap<String, PermissionSet>,
}

impl CapabilityRegistry {
    /// Look up a capability by id.
    pub fn get(&self, id: &str) -> Option<&Capability> {
        self.capabilities.get(&CapabilityId(id.to_string()))
    }

    /// All capability ids, sorted.
    pub fn capability_ids(&self) -> Vec<&CapabilityId> {
        self.capabilities.keys().collect()
    }

    /// Resolve the permission set for a capability by id.
    pub fn permission_set_for(&self, capability_id: &str) -> Option<&PermissionSet> {
        let cap = self.get(capability_id)?;
        self.permission_sets.get(&cap.permission_set)
    }

    /// Resolve a permission set directly by id.
    pub fn permission_set(&self, id: &PermissionSetId) -> Option<&PermissionSet> {
        self.permission_sets.get(id.as_str())
    }

    /// Number of capabilities in this registry.
    pub fn len(&self) -> usize {
        self.capabilities.len()
    }

    /// True iff there are no capabilities.
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }
}

/// Helper for callers that build a [`CapabilityRegistry`] from in-memory
/// capability + permission-set slices (e.g. tests).
pub fn build_registry(
    registry_id: impl Into<String>,
    capabilities: Vec<Capability>,
    permission_sets: Vec<PermissionSet>,
) -> RegistryResult<CapabilityRegistry> {
    let mut doc = RegistryDocument {
        schema_version: REGISTRY_SCHEMA_VERSION,
        registry_id: registry_id.into(),
        note: None,
        capabilities,
        permission_sets,
    };
    // Ensure deterministic order for tests.
    doc.capabilities
        .sort_by(|a, b| a.capability_id.cmp(&b.capability_id));
    doc.permission_sets
        .sort_by(|a, b| a.permission_set_id.cmp(&b.permission_set_id));
    doc.validate()?;
    Ok(doc.into_registry())
}

#[allow(dead_code)]
fn _unused_path_warning(path: &Path) -> PathBuf {
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permission::{PermissionGrant, Scope, PERMISSION_SET_SCHEMA};

    fn well_formed_inputs() -> serde_json::Value {
        serde_json::json!({ "type": "object", "additionalProperties": false })
    }

    fn minimal_read() -> Capability {
        Capability {
            schema: REGISTRY_SCHEMA.into(),
            capability_id: CapabilityId::new("timeline.read"),
            version: 1,
            kind: CapabilityKind::Read,
            owner_component: "video-project".into(),
            permission_set: "pset.basic".into(),
            inputs: well_formed_inputs(),
            outputs: CapabilityOutputs {
                bounded: true,
                windowed: true,
                max_items: Some(100),
            },
            eval_suites: vec!["eval.timeline_read".into()],
            degradation: Some(Degradation::Ok),
        }
    }

    fn minimal_mutation() -> Capability {
        Capability {
            schema: REGISTRY_SCHEMA.into(),
            capability_id: CapabilityId::new("timeline.cut"),
            version: 1,
            kind: CapabilityKind::Mutation,
            owner_component: "video-actions".into(),
            permission_set: "pset.editor".into(),
            inputs: well_formed_inputs(),
            outputs: CapabilityOutputs::default(),
            eval_suites: vec![],
            degradation: Some(Degradation::Ok),
        }
    }

    fn basic_pset() -> PermissionSet {
        PermissionSet {
            schema: PERMISSION_SET_SCHEMA.into(),
            permission_set_id: "pset.basic".into(),
            grants: vec![PermissionGrant {
                capability: "timeline.read".into(),
                scope: Scope::TimelineRead,
            }],
        }
    }

    fn editor_pset() -> PermissionSet {
        PermissionSet {
            schema: PERMISSION_SET_SCHEMA.into(),
            permission_set_id: "pset.editor".into(),
            grants: vec![
                PermissionGrant {
                    capability: "timeline.cut".into(),
                    scope: Scope::TimelineWrite,
                },
                PermissionGrant {
                    capability: "timeline.cut".into(),
                    scope: Scope::TimelineRead,
                },
            ],
        }
    }

    #[test]
    fn capability_id_validation_rejects_bad_shapes() {
        assert!(CapabilityId::is_well_formed("timeline.read"));
        assert!(CapabilityId::is_well_formed("a.b.c_2"));
        assert!(!CapabilityId::is_well_formed("Timeline.read"));
        assert!(!CapabilityId::is_well_formed("1timeline.read"));
        assert!(!CapabilityId::is_well_formed("a"));
        assert!(!CapabilityId::is_well_formed("a-b"));
        assert!(!CapabilityId::is_well_formed(""));
    }

    #[test]
    fn registry_round_trips_a_minimal_document() {
        let doc = RegistryDocument {
            schema_version: REGISTRY_SCHEMA_VERSION,
            registry_id: "cutright-v2-test".into(),
            note: None,
            capabilities: vec![minimal_read(), minimal_mutation()],
            permission_sets: vec![basic_pset(), editor_pset()],
        };
        let bytes = serde_json::to_vec(&doc).unwrap();
        let parsed = RegistryDocument::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.capabilities.len(), 2);
        let registry = parsed.into_registry();
        assert_eq!(registry.len(), 2);
        let pset = registry
            .permission_set_for("timeline.cut")
            .expect("editor pset");
        assert_eq!(pset.permission_set_id, "pset.editor");
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let mut doc = RegistryDocument {
            schema_version: 2,
            registry_id: "cutright-v2-test".into(),
            note: None,
            capabilities: vec![minimal_read()],
            permission_sets: vec![basic_pset()],
        };
        doc.capabilities[0].schema = REGISTRY_SCHEMA.into();
        let bytes = serde_json::to_vec(&doc).unwrap();
        let err = RegistryDocument::from_bytes(&bytes).unwrap_err();
        assert!(matches!(
            err,
            RegistryError::UnsupportedSchemaVersion {
                found: 2,
                expected: 1
            }
        ));
    }

    #[test]
    fn rejects_unknown_owner_component() {
        let mut bad = minimal_read();
        bad.owner_component = "video-ghost".into();
        let doc = RegistryDocument {
            schema_version: REGISTRY_SCHEMA_VERSION,
            registry_id: "cutright-v2-test".into(),
            note: None,
            capabilities: vec![bad],
            permission_sets: vec![basic_pset()],
        };
        let err = doc.validate().unwrap_err();
        assert!(matches!(err, RegistryError::UnknownOwnerComponent { .. }));
    }

    #[test]
    fn rejects_dangling_permission_set() {
        let mut bad = minimal_read();
        bad.permission_set = "pset.does_not_exist".into();
        let doc = RegistryDocument {
            schema_version: REGISTRY_SCHEMA_VERSION,
            registry_id: "cutright-v2-test".into(),
            note: None,
            capabilities: vec![bad],
            permission_sets: vec![basic_pset()],
        };
        let err = doc.validate().unwrap_err();
        assert!(matches!(err, RegistryError::DanglingPermissionSet { .. }));
    }

    #[test]
    fn rejects_read_missing_bounded_or_windowed() {
        let mut bad = minimal_read();
        bad.outputs.bounded = false;
        let doc = RegistryDocument {
            schema_version: REGISTRY_SCHEMA_VERSION,
            registry_id: "cutright-v2-test".into(),
            note: None,
            capabilities: vec![bad],
            permission_sets: vec![basic_pset()],
        };
        let err = doc.validate().unwrap_err();
        assert!(matches!(err, RegistryError::ReadNotBoundedWindowed { .. }));

        let mut bad = minimal_read();
        bad.outputs.windowed = false;
        let doc = RegistryDocument {
            schema_version: REGISTRY_SCHEMA_VERSION,
            registry_id: "cutright-v2-test".into(),
            note: None,
            capabilities: vec![bad],
            permission_sets: vec![basic_pset()],
        };
        let err = doc.validate().unwrap_err();
        assert!(matches!(err, RegistryError::ReadNotBoundedWindowed { .. }));
    }

    #[test]
    fn rejects_duplicate_capability_version() {
        // Same capability_id AND same version (1,1) collide; the validator
        // must refuse the document.
        let dup = minimal_read();
        let doc = RegistryDocument {
            schema_version: REGISTRY_SCHEMA_VERSION,
            registry_id: "cutright-v2-test".into(),
            note: None,
            capabilities: vec![minimal_read(), dup],
            permission_sets: vec![basic_pset()],
        };
        let err = doc.validate().unwrap_err();
        assert!(matches!(err, RegistryError::DuplicateEntry { .. }));
    }

    #[test]
    fn allows_same_capability_id_at_different_versions() {
        let mut v2 = minimal_read();
        v2.version = 2;
        let doc = RegistryDocument {
            schema_version: REGISTRY_SCHEMA_VERSION,
            registry_id: "cutright-v2-test".into(),
            note: None,
            capabilities: vec![minimal_read(), v2],
            permission_sets: vec![basic_pset()],
        };
        assert!(doc.validate().is_ok());
    }

    #[test]
    fn rejects_malformed_capability_id() {
        let mut bad = minimal_read();
        bad.capability_id = CapabilityId::new("Timeline.Read");
        let doc = RegistryDocument {
            schema_version: REGISTRY_SCHEMA_VERSION,
            registry_id: "cutright-v2-test".into(),
            note: None,
            capabilities: vec![bad],
            permission_sets: vec![basic_pset()],
        };
        let err = doc.validate().unwrap_err();
        assert!(matches!(err, RegistryError::InvalidEntry { .. }));
    }

    #[test]
    fn rejects_unknown_field_in_capability_entry() {
        let value = serde_json::json!({
            "schema": REGISTRY_SCHEMA,
            "capability_id": "timeline.read",
            "version": 1,
            "kind": "read",
            "owner_component": "video-project",
            "permission_set": "pset.basic",
            "inputs": {},
            "outputs": { "bounded": true, "windowed": true },
            "sneaky_field": 42
        });
        let res: Result<Capability, _> = serde_json::from_value(value);
        assert!(res.is_err(), "deny_unknown_fields must reject sneaky_field");
    }

    #[test]
    fn rejects_unknown_field_in_registry_document() {
        let value = serde_json::json!({
            "schema_version": 1,
            "registry_id": "test",
            "capabilities": [],
            "permission_sets": [],
            "sneaky_field": 42
        });
        let res: Result<RegistryDocument, _> = serde_json::from_value(value);
        assert!(res.is_err(), "deny_unknown_fields must reject sneaky_field");
    }

    #[test]
    fn build_registry_helper_produces_sorted_view() {
        let registry = build_registry(
            "test",
            vec![minimal_mutation(), minimal_read()],
            vec![editor_pset(), basic_pset()],
        )
        .unwrap();
        let ids = registry.capability_ids();
        assert_eq!(ids[0].as_str(), "timeline.cut");
        assert_eq!(ids[1].as_str(), "timeline.read");
    }
}
