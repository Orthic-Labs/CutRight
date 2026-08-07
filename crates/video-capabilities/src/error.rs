//! Error type for the capability registry loader / validator.

use std::path::PathBuf;

use thiserror::Error;

/// Result alias used across `video-capabilities`.
pub type RegistryResult<T> = Result<T, RegistryError>;

/// Failure modes for loading, validating, and querying the registry.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// The on-disk file could not be read.
    #[error("capability registry I/O error reading {path}: {source}")]
    Io {
        /// File path that failed.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The JSON document was malformed.
    #[error("capability registry JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    /// The top-level `schema_version` was not the supported value.
    #[error("capability registry schema_version {found} is not supported (expected {expected})")]
    UnsupportedSchemaVersion {
        /// Value found in the document.
        found: u32,
        /// The single version this build understands.
        expected: u32,
    },
    /// An entry did not match `cutright.capability/v1`.
    #[error("capability registry entry {index} ({capability_id}) failed: {reason}")]
    InvalidEntry {
        /// Zero-based index into the document's `capabilities` array.
        index: usize,
        /// The capability_id of the offending entry (when known).
        capability_id: String,
        /// Human-readable reason.
        reason: String,
    },
    /// A capability referenced a permission set that is not declared in any
    /// bundled permission-set document.
    #[error("capability {capability_id} references unknown permission set {permission_set}")]
    DanglingPermissionSet {
        /// The capability that pointed at the missing set.
        capability_id: String,
        /// The dangling set id.
        permission_set: String,
    },
    /// A capability's `owner_component` did not match any known component.
    #[error("capability {capability_id} declares unknown owner_component {owner_component}")]
    UnknownOwnerComponent {
        /// The capability that pointed at the missing owner.
        capability_id: String,
        /// The unknown owner string.
        owner_component: String,
    },
    /// A `read` capability did not declare both `bounded: true` and
    /// `windowed: true`. Per `V2-CAPABILITY-ACTION-CONTRACT.md` reads must
    /// always be bounded + windowed.
    #[error(
        "read capability {capability_id} must declare outputs.bounded=true and outputs.windowed=true"
    )]
    ReadNotBoundedWindowed {
        /// The read capability that failed the rule.
        capability_id: String,
    },
    /// Two entries declared the same `(capability_id, version)`.
    #[error("duplicate capability entry {capability_id} version {version}")]
    DuplicateEntry {
        /// The duplicated capability_id.
        capability_id: String,
        /// The duplicated version.
        version: u32,
    },
}
