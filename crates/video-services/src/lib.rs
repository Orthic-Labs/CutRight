//! CutRight v2 service façade (CR-V2-B3-022).
//!
//! The [`VideoServices`] struct is the single entry point for project
//! orchestration. It exposes a [`PackService`], an [`EvidenceService`],
//! a [`JobService`] and an [`InferenceService`] under stable IDs. The
//! façade never hands out raw mutable handles; every service returns
//! opaque tokens that the runtime/evidence/job layer can resolve.

#[allow(unpredictable_function_pointer_comparisons)]
pub mod dispatch;
pub mod evidence;
pub mod jobs;
pub mod runtime;

// Keep daemon-owned lease rules in one source until transport wiring is added.
#[path = "../../video-daemon/src/leases.rs"]
pub mod leases;

pub use evidence::{EvidenceHandle, EvidenceQuery, EvidenceService};
pub use jobs::{JobHandle, JobService, JobSubmission};
pub use leases::{LeaseCompletion, LeaseError, LeaseRegistry, MutationCapability};
pub use runtime::{PackHandle, PackService, RuntimeService};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use video_capabilities::CapabilityRegistry;
use video_project::{ActionBatch, ActionExecutor, ActionExecutorError, ExecutorReport};
use video_sessions::SessionGuard;

/// Aggregate façade. Built once per process and reused for the lifetime
/// of the Studio / CLI run.
#[derive(Debug, Clone)]
pub struct VideoServices {
    pub packs: PackService,
    pub evidence: EvidenceService,
    pub jobs: JobService,
    pub inference: InferenceService,
    leases: LeaseRegistry,
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
            leases: LeaseRegistry::default(),
        }
    }

    /// Shared mutation authority for every transport principal.
    pub fn leases(&self) -> &LeaseRegistry {
        &self.leases
    }

    /// Route one generated write operation through the sole ActionExecutor.
    pub fn route_mutation(
        &self,
        request: MutationRequest<'_>,
    ) -> Result<ExecutorReport, ServiceError> {
        let binding = binding_for(request.operation)?;
        if !binding.execution_mode.starts_with("write") {
            return Err(ServiceError::NotMutation(request.operation.into()));
        }
        let capability = self.leases.acquire(
            request.project_id,
            request.batch.expected_revision.clone(),
            request.principal,
            request.lease_ttl,
        )?;
        let executor = ActionExecutor::new(&self.packs.config().project_root);
        let report = executor
            .execute(request.batch, request.registry, request.sessions, None)
            .map_err(ServiceError::Executor)?;
        let completion = if report.is_applied() {
            LeaseCompletion::Applied {
                new_revision: report.receipt.new_revision.clone(),
                receipt_id: report.receipt.receipt_id.clone(),
            }
        } else {
            LeaseCompletion::Failed {
                receipt_id: report.receipt.receipt_id.clone(),
            }
        };
        self.leases.complete(&capability, completion)?;
        Ok(report)
    }

    /// Route read/compute handlers through the generated operation surface.
    /// Mutations must use [`Self::route_mutation`] so they receive a lease.
    pub fn route_operation(
        &self,
        operation: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, ServiceError> {
        let binding = binding_for(operation)?;
        if binding.execution_mode.starts_with("write") {
            return Err(ServiceError::MutationRequiresLease(operation.into()));
        }
        dispatch::dispatch(operation, input).map_err(ServiceError::Dispatch)
    }
}

/// Inputs shared by every surface when asking the façade to mutate a project.
pub struct MutationRequest<'a> {
    pub operation: &'a str,
    pub project_id: &'a str,
    pub principal: &'a str,
    pub batch: &'a ActionBatch,
    pub registry: &'a CapabilityRegistry,
    pub sessions: &'a SessionGuard,
    pub lease_ttl: std::time::Duration,
}

impl<'a> MutationRequest<'a> {
    pub fn new(
        operation: &'a str,
        project_id: &'a str,
        principal: &'a str,
        batch: &'a ActionBatch,
        registry: &'a CapabilityRegistry,
        sessions: &'a SessionGuard,
    ) -> Self {
        Self {
            operation,
            project_id,
            principal,
            batch,
            registry,
            sessions,
            lease_ttl: leases::DEFAULT_LEASE_TTL,
        }
    }
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("unknown operation: {0}")]
    UnknownOperation(String),
    #[error("operation is not a mutation: {0}")]
    NotMutation(String),
    #[error("mutation requires a capability lease: {0}")]
    MutationRequiresLease(String),
    #[error("generated dispatch failed: {0}")]
    Dispatch(#[from] dispatch::DispatchError),
    #[error("mutation lease failed: {0}")]
    Lease(#[from] LeaseError),
    #[error("action executor failed: {0}")]
    Executor(#[from] ActionExecutorError),
}

fn binding_for(operation: &str) -> Result<&'static dispatch::OperationBinding, ServiceError> {
    dispatch::OPERATIONS
        .iter()
        .find(|binding| binding.operation == operation)
        .ok_or_else(|| ServiceError::UnknownOperation(operation.into()))
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

    /// Roots this service will resolve against once the bounded model
    /// router is integrated.
    pub fn config(&self) -> &ServiceConfig {
        &self.config
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
