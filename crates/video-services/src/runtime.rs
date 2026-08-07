//! Runtime service (CR-V2-B3-022).
//!
//! The runtime service exposes a stable capability id (`cap.pack.manage`)
//! and resolves pack activations to opaque handles. The handle is the
//! only thing the project layer is allowed to carry; resolving the
//! handle to a concrete executable path is the runtime layer's job.

use serde::{Deserialize, Serialize};

use crate::ServiceConfig;

/// Stable capability id advertised by the pack service.
pub const PACK_SERVICE_CAPABILITY: &str = "cap.pack.manage";

/// Stable capability id advertised by the runtime service (alias).
pub const RUNTIME_SERVICE_CAPABILITY: &str = "cap.runtime.resolve";

/// Pack service. Hands out pack handles; never returns raw paths.
#[derive(Debug, Clone)]
pub struct PackService {
    config: ServiceConfig,
}

impl PackService {
    pub fn new(config: ServiceConfig) -> Self {
        Self { config }
    }

    pub fn capability_id(&self) -> &'static str {
        PACK_SERVICE_CAPABILITY
    }

    /// Activate a pack by id. The returned handle is opaque to callers.
    pub fn activate(&self, pack_id: &str) -> Result<PackHandle, RuntimeError> {
        if pack_id.is_empty() {
            return Err(RuntimeError::EmptyPackId);
        }
        Ok(PackHandle(format!("pack:handle:{pack_id}")))
    }

    /// Configuration accessor.
    pub fn config(&self) -> &ServiceConfig {
        &self.config
    }
}

/// Runtime service alias. `PackService` is the real implementation; the
/// alias is kept for documentation purposes.
#[derive(Debug, Clone)]
pub struct RuntimeService {
    pub packs: PackService,
}

impl RuntimeService {
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            packs: PackService::new(config),
        }
    }

    pub fn capability_id(&self) -> &'static str {
        RUNTIME_SERVICE_CAPABILITY
    }
}

/// Opaque pack handle. The inner string is never decoded by callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackHandle(pub String);

impl PackHandle {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Runtime errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeError {
    EmptyPackId,
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::EmptyPackId => write!(f, "empty pack id"),
        }
    }
}

impl std::error::Error for RuntimeError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> ServiceConfig {
        ServiceConfig {
            project_root: "/tmp/proj".into(),
            pack_root: "/tmp/packs".into(),
            evidence_root: "/tmp/evidence".into(),
            lease_root: "/tmp/lease".into(),
        }
    }

    #[test]
    fn activate_returns_handle() {
        let svc = PackService::new(cfg());
        let h = svc.activate("speech").unwrap();
        assert_eq!(h.as_str(), "pack:handle:speech");
    }

    #[test]
    fn empty_id_is_rejected() {
        let svc = PackService::new(cfg());
        assert!(svc.activate("").is_err());
    }
}
