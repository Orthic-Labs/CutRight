//! Lane P-B: capability registry model, validator, code generator, and drift
//! detector (CR-V2-B2-012..016).
//!
//! Implements the v2 contract frozen by `27d5682..50f76d5`:
//! - `docs/architecture/V2-CAPABILITY-ACTION-CONTRACT.md` — capability
//!   registry entry, read vs mutation distinction, bounded/windowed read
//!   requirement.
//! - `docs/security/V2-ACTION-PERMISSIONS.md` — eight least-privilege scopes.
//! - `docs/architecture/V2-CRATE-DAG.md` — Lane P-B ownership of
//!   `video-capabilities` + `bindings/`.
//!
//! Per `V2-CRATE-DAG.md` rule 2 ("Lower never depends on higher"), this crate
//! depends only on `video-core` and never on `video-state`, `video-actions`,
//! `video-project`, `video-cli`, or `apps/studio`. Lane P-B owns the canonical
//! source registry at `docs/dispatch/v2/source/capability-registry.json`.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod codegen;
pub mod docs;
pub mod drift;
pub mod error;
pub mod permission;
pub mod registry;

#[path = "generated.rs"]
mod generated_inner;

pub use codegen::{
    generate_all, render_mcp_tool_registry, render_rust_enum, render_typescript, CodegenError,
    GENERATOR_SCHEMA,
};
pub use error::{RegistryError, RegistryResult};
pub use generated_inner::generated;
pub use permission::{PermissionGrant, PermissionSet, PermissionSetId, Scope, SCOPES};
pub use registry::{
    build_registry, Capability, CapabilityId, CapabilityKind, CapabilityOutputs,
    CapabilityRegistry, Degradation, RegistryDocument, REGISTRY_SCHEMA, REGISTRY_SCHEMA_VERSION,
};
