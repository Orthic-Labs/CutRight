//! Evidence service (CR-V2-B3-022).
//!
//! The evidence service exposes a single read capability. The query
//! returns an opaque handle; the graph layer is responsible for the
//! actual traversal.

use serde::{Deserialize, Serialize};

use crate::ServiceConfig;

pub const EVIDENCE_SERVICE_CAPABILITY: &str = "cap.evidence.read";

#[derive(Debug, Clone)]
pub struct EvidenceService {
    config: ServiceConfig,
}

impl EvidenceService {
    pub fn new(config: ServiceConfig) -> Self {
        Self { config }
    }

    pub fn capability_id(&self) -> &'static str {
        EVIDENCE_SERVICE_CAPABILITY
    }

    /// Submit a read query. The result is an opaque handle.
    pub fn query(&self, q: EvidenceQuery) -> Result<EvidenceHandle, EvidenceError> {
        if q.scope.is_empty() {
            return Err(EvidenceError::EmptyScope);
        }
        Ok(EvidenceHandle(format!("evidence:handle:{}", q.scope)))
    }

    pub fn config(&self) -> &ServiceConfig {
        &self.config
    }
}

/// A read query. The scope is whatever the calling capability recognises;
/// the service treats it as opaque.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceQuery {
    pub scope: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceHandle(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceError {
    EmptyScope,
}

impl std::fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvidenceError::EmptyScope => write!(f, "empty evidence scope"),
        }
    }
}

impl std::error::Error for EvidenceError {}

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
    fn query_returns_handle() {
        let svc = EvidenceService::new(cfg());
        let h = svc
            .query(EvidenceQuery {
                scope: "evidence_graph".into(),
                limit: Some(100),
            })
            .unwrap();
        assert_eq!(h.0, "evidence:handle:evidence_graph");
    }

    #[test]
    fn empty_scope_is_rejected() {
        let svc = EvidenceService::new(cfg());
        let q = EvidenceQuery {
            scope: "".into(),
            limit: None,
        };
        assert!(svc.query(q).is_err());
    }
}
