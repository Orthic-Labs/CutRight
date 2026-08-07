//! CutRight v2 service façade (CR-V2-B3-022).
//!
//! The [`VideoServices`] struct is the single entry point for project
//! orchestration. It exposes a [`PackService`], an [`EvidenceService`],
//! a [`JobService`] and an [`InferenceService`] under stable IDs. The
//! façade never hands out raw mutable handles; every service returns
//! opaque tokens that the runtime/evidence/job layer can resolve.

pub mod evidence;
pub mod jobs;
pub mod runtime;

pub use evidence::{EvidenceHandle, EvidenceQuery, EvidenceService};
pub use jobs::{JobHandle, JobService, JobSubmission};
pub use runtime::{PackHandle, PackService, RuntimeService};

use serde::{Deserialize, Serialize};

/// Aggregate façade. Built once per process and reused for the lifetime
/// of the Studio / CLI run.
#[derive(Debug, Clone)]
pub struct VideoServices {
    pub packs: PackService,
    pub evidence: EvidenceService,
    pub jobs: JobService,
    pub inference: InferenceService,
}

impl VideoServices {
    /// Build a fresh façade. The four services are independent but
    /// share the supplied [`ServiceConfig`] so the same project root
    /// is used everywhere.
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            packs: PackService::new(config.clone()),
            evidence: EvidenceService::new(config.clone()),
            jobs: JobService::new(config.clone()),
            inference: InferenceService::new(config.clone()),
        }
    }
}

/// Shared configuration for every service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub project_root: String,
    pub pack_root: String,
    pub evidence_root: String,
    pub lease_root: String,
}

/// Inference is a thin alias today; the real implementation will plug in
/// the bounded model router once the inference lane is integrated.
#[derive(Debug, Clone)]
pub struct InferenceService {
    config: ServiceConfig,
}

impl InferenceService {
    pub fn new(config: ServiceConfig) -> Self {
        Self { config }
    }

    /// Stable capability id advertised by the inference service.
    pub fn capability_id(&self) -> &'static str {
        "cap.inference.route"
    }
}

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
    fn facade_builds_with_all_services() {
        let svc = VideoServices::new(cfg());
        assert_eq!(svc.packs.capability_id(), "cap.pack.manage");
        assert_eq!(svc.evidence.capability_id(), "cap.evidence.read");
        assert_eq!(svc.jobs.capability_id(), "cap.render.dispatch");
        assert_eq!(svc.inference.capability_id(), "cap.inference.route");
    }

    #[test]
    fn services_hand_out_opaque_handles() {
        let svc = VideoServices::new(cfg());
        let pack = svc.packs.activate("speech").unwrap();
        assert_eq!(pack.as_str(), "pack:handle:speech");
    }
}
