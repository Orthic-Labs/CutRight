//! Project-scoped mutation capability leases.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Default lifetime for a mutation capability.
pub const DEFAULT_LEASE_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
struct ActiveLease {
    token: u64,
    principal: String,
    revision: String,
    expires_at: Instant,
}

#[derive(Debug, Clone)]
struct ProjectState {
    current_revision: String,
    active: Option<ActiveLease>,
}

#[derive(Debug, Default)]
struct LeaseState {
    next_token: u64,
    projects: BTreeMap<String, ProjectState>,
}

/// Shared in-process lease authority. The daemon owns one instance and shares
/// clones with every transport surface.
#[derive(Debug, Clone, Default)]
pub struct LeaseRegistry {
    state: Arc<Mutex<LeaseState>>,
}

/// Short-lived project/revision/principal capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationCapability {
    project_id: String,
    revision: String,
    principal: String,
    token: u64,
    expires_at: Instant,
}

impl MutationCapability {
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    pub fn revision(&self) -> &str {
        &self.revision
    }
    pub fn principal(&self) -> &str {
        &self.principal
    }
    pub fn token(&self) -> u64 {
        self.token
    }
    pub fn expires_at(&self) -> Instant {
        self.expires_at
    }
}

/// Receipt-backed completion of a mutation lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaseCompletion {
    /// Advance CAS state only after the executor has emitted its receipt.
    Applied {
        new_revision: String,
        receipt_id: String,
    },
    /// Release after a typed failed receipt; current revision is unchanged.
    Failed { receipt_id: String },
    /// Preserve authority until expiry; no success or release is inferred.
    Disconnected,
}

/// Lease errors are stable enough for transport callers to branch on them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaseError {
    EmptyField(&'static str),
    ProjectUnknown(String),
    AlreadyHeld { project_id: String },
    StaleRevision { current_revision: String },
    InvalidCapability,
    Expired,
    InvalidCompletion,
}

impl std::fmt::Display for LeaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField(field) => write!(f, "lease field {field} is empty"),
            Self::ProjectUnknown(project) => write!(f, "unknown project {project}"),
            Self::AlreadyHeld { project_id } => {
                write!(f, "mutation lease already held for project {project_id}")
            }
            Self::StaleRevision { current_revision } => {
                write!(f, "stale revision; current revision is {current_revision}")
            }
            Self::InvalidCapability => write!(f, "invalid mutation capability"),
            Self::Expired => write!(f, "mutation capability expired"),
            Self::InvalidCompletion => write!(f, "invalid mutation completion"),
        }
    }
}

impl std::error::Error for LeaseError {}

impl LeaseRegistry {
    /// Register a project and its current revision before accepting mutations.
    pub fn register_project(
        &self,
        project_id: impl Into<String>,
        current_revision: impl Into<String>,
    ) -> Result<(), LeaseError> {
        let project_id = non_empty(project_id.into(), "project_id")?;
        let current_revision = non_empty(current_revision.into(), "current_revision")?;
        let mut state = self.state.lock().expect("lease registry mutex poisoned");
        state.projects.insert(
            project_id,
            ProjectState {
                current_revision,
                active: None,
            },
        );
        Ok(())
    }

    /// Acquire the one mutation capability for a project at exact revision.
    pub fn acquire(
        &self,
        project_id: impl Into<String>,
        revision: impl Into<String>,
        principal: impl Into<String>,
        ttl: Duration,
    ) -> Result<MutationCapability, LeaseError> {
        self.acquire_at(project_id, revision, principal, ttl, Instant::now())
    }

    /// Deterministic clock-injectable acquisition used by focused checks.
    pub fn acquire_at(
        &self,
        project_id: impl Into<String>,
        revision: impl Into<String>,
        principal: impl Into<String>,
        ttl: Duration,
        now: Instant,
    ) -> Result<MutationCapability, LeaseError> {
        let project_id = non_empty(project_id.into(), "project_id")?;
        let revision = non_empty(revision.into(), "revision")?;
        let principal = non_empty(principal.into(), "principal")?;
        let mut state = self.state.lock().expect("lease registry mutex poisoned");
        {
            let project = state
                .projects
                .get(&project_id)
                .ok_or_else(|| LeaseError::ProjectUnknown(project_id.clone()))?;
            if project
                .active
                .as_ref()
                .is_some_and(|lease| lease.expires_at > now)
            {
                return Err(LeaseError::AlreadyHeld { project_id });
            }
            if project.current_revision != revision {
                return Err(LeaseError::StaleRevision {
                    current_revision: project.current_revision.clone(),
                });
            }
        }
        state.next_token = state.next_token.wrapping_add(1).max(1);
        let token = state.next_token;
        let expires_at = now + ttl;
        let project = state
            .projects
            .get_mut(&project_id)
            .ok_or_else(|| LeaseError::ProjectUnknown(project_id.clone()))?;
        project.active = None;
        project.active = Some(ActiveLease {
            token,
            principal: principal.clone(),
            revision: revision.clone(),
            expires_at,
        });
        Ok(MutationCapability {
            project_id,
            revision,
            principal,
            token,
            expires_at,
        })
    }

    /// Complete only after the caller has a durable receipt outcome.
    pub fn complete(
        &self,
        capability: &MutationCapability,
        completion: LeaseCompletion,
    ) -> Result<(), LeaseError> {
        self.complete_at(capability, completion, Instant::now())
    }

    pub fn complete_at(
        &self,
        capability: &MutationCapability,
        completion: LeaseCompletion,
        now: Instant,
    ) -> Result<(), LeaseError> {
        let mut state = self.state.lock().expect("lease registry mutex poisoned");
        let project = state
            .projects
            .get_mut(&capability.project_id)
            .ok_or_else(|| LeaseError::ProjectUnknown(capability.project_id.clone()))?;
        let active = project
            .active
            .as_ref()
            .ok_or(LeaseError::InvalidCapability)?;
        if active.token != capability.token
            || active.principal != capability.principal
            || active.revision != capability.revision
        {
            return Err(LeaseError::InvalidCapability);
        }
        if active.expires_at <= now {
            project.active = None;
            return Err(LeaseError::Expired);
        }
        match completion {
            LeaseCompletion::Applied {
                new_revision,
                receipt_id,
            } => {
                if receipt_id.is_empty() || new_revision.is_empty() {
                    return Err(LeaseError::InvalidCompletion);
                }
                if new_revision == capability.revision {
                    return Err(LeaseError::InvalidCompletion);
                }
                project.current_revision = new_revision;
                project.active = None;
            }
            LeaseCompletion::Failed { receipt_id } => {
                if receipt_id.is_empty() {
                    return Err(LeaseError::InvalidCompletion);
                }
                project.active = None;
            }
            LeaseCompletion::Disconnected => {
                // Keep active authority until expiry. A disconnect is not a
                // receipt and therefore cannot imply success or release.
            }
        }
        Ok(())
    }

    /// Drop expired capabilities without changing current revision state.
    pub fn reap_expired(&self) {
        let now = Instant::now();
        let mut state = self.state.lock().expect("lease registry mutex poisoned");
        for project in state.projects.values_mut() {
            if project
                .active
                .as_ref()
                .is_some_and(|lease| lease.expires_at <= now)
            {
                project.active = None;
            }
        }
    }

    pub fn current_revision(&self, project_id: &str) -> Option<String> {
        self.state
            .lock()
            .expect("lease registry mutex poisoned")
            .projects
            .get(project_id)
            .map(|project| project.current_revision.clone())
    }
}

fn non_empty(value: String, field: &'static str) -> Result<String, LeaseError> {
    if value.is_empty() {
        Err(LeaseError::EmptyField(field))
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::thread;

    #[test]
    fn concurrent_mutations_have_one_winner_then_stale_revision() {
        let registry = LeaseRegistry::default();
        registry.register_project("project", "rev-1").unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let contenders: Vec<_> = (0..2)
            .map(|index| {
                let registry = registry.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    registry.acquire(
                        "project",
                        "rev-1",
                        format!("principal-{index}"),
                        DEFAULT_LEASE_TTL,
                    )
                })
            })
            .collect();
        let mut winner = None;
        let mut busy = 0;
        for contender in contenders {
            match contender.join().unwrap() {
                Ok(capability) => winner = Some(capability),
                Err(LeaseError::AlreadyHeld { .. }) => busy += 1,
                other => panic!("unexpected contender result: {other:?}"),
            }
        }
        assert_eq!(busy, 1);
        registry
            .complete(
                &winner.unwrap(),
                LeaseCompletion::Applied {
                    new_revision: "rev-2".into(),
                    receipt_id: "rcpt-1".into(),
                },
            )
            .unwrap();
        assert_eq!(
            registry.acquire("project", "rev-1", "retry", DEFAULT_LEASE_TTL),
            Err(LeaseError::StaleRevision {
                current_revision: "rev-2".into()
            })
        );
    }

    #[test]
    fn disconnect_keeps_authority_and_never_advances_revision() {
        let registry = LeaseRegistry::default();
        registry.register_project("project", "rev-1").unwrap();
        let now = Instant::now();
        let capability = registry
            .acquire_at(
                "project",
                "rev-1",
                "principal",
                Duration::from_secs(10),
                now,
            )
            .unwrap();
        registry
            .complete_at(&capability, LeaseCompletion::Disconnected, now)
            .unwrap();
        assert_eq!(registry.current_revision("project"), Some("rev-1".into()));
        assert!(matches!(
            registry.acquire_at("project", "rev-1", "other", DEFAULT_LEASE_TTL, now),
            Err(LeaseError::AlreadyHeld { .. })
        ));
    }
}
