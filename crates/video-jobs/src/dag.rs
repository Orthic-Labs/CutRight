//! Content-addressed job DAG (CR-V2-B3-021).
//!
//! A [`JobDag`] is a directed acyclic graph of [`StageSpec`] nodes. Each
//! stage carries a deterministic fingerprint so identical inputs produce
//! identical cache keys across runs. The DAG is the unit of scheduling; the
//! runner ([`crate::runner`]) turns it into a [`crate::store::JobRecord`]
//! whose per-stage [`StageRecord`]s the persistent store mutates atomically.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::store::{StageRecord, StageState};

/// Stable identifier for a job.
pub type JobId = String;

/// Stable identifier for a stage within a job.
pub type StageId = String;

/// Why a stage failed. Permanent failures abort the run; retryable failures
/// are re-attempted up to [`StageSpec::max_attempts`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    Permanent,
    Retryable,
}

/// How the runner should classify an attempt failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorKind {
    /// Crash, missing dependency, contract violation. Cannot be retried.
    Permanent,
    /// Network timeout, contention, transient I/O. Eligible for retry.
    Retryable,
    /// Stage was cancelled mid-run. Not a failure for retry accounting.
    Cancelled,
}

/// Resource requirements for a stage. The runner will only mark a stage
/// Ready when resources are available. The model is intentionally tiny —
/// real resource accounting is layered on top.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBudget {
    pub cpu_milli: u32,
    pub memory_mb: u32,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            cpu_milli: 500,
            memory_mb: 256,
        }
    }
}

/// Declarative description of a single stage. Stages are immutable once
/// committed to a job record — the runner cannot mutate a spec to "fix" a
/// failed attempt; instead it persists the spec fingerprint so a retry
/// verifies the contract is still the same.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageSpec {
    pub id: StageId,
    pub kind: String,
    /// Stage ids whose [`StageRecord`] must reach [`StageState::Succeeded`]
    /// before this stage becomes [`StageState::Ready`].
    pub dependencies: Vec<StageId>,
    /// Parameters that feed the deterministic fingerprint. Identical
    /// parameters across runs yield identical fingerprints and therefore
    /// cache hits.
    pub parameters: serde_json::Value,
    /// Optional resource hint. Stages that exceed the system budget wait
    /// rather than start.
    pub resources: ResourceBudget,
    /// Maximum retry attempts for [`ErrorKind::Retryable`] failures. Zero
    /// means "do not retry; any retryable failure becomes permanent".
    pub max_attempts: u32,
}

impl StageSpec {
    /// Compute a BLAKE3 fingerprint of the stage spec. The fingerprint is
    /// part of the cache key, so two specs with identical JSON, dependencies
    /// and resources share the same fingerprint.
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut canonical = serde_json::Map::new();
        canonical.insert("id".to_string(), serde_json::Value::String(self.id.clone()));
        canonical.insert("kind".to_string(), serde_json::Value::String(self.kind.clone()));
        let mut deps: Vec<serde_json::Value> = self
            .dependencies
            .iter()
            .map(|d| serde_json::Value::String(d.clone()))
            .collect();
        deps.sort_by(|a, b| match (a, b) {
            (serde_json::Value::String(a), serde_json::Value::String(b)) => a.cmp(b),
            _ => std::cmp::Ordering::Equal,
        });
        canonical.insert("dependencies".to_string(), serde_json::Value::Array(deps));
        canonical.insert("parameters".to_string(), self.parameters.clone());
        canonical.insert("cpu_milli".to_string(), serde_json::json!(self.resources.cpu_milli));
        canonical.insert(
            "memory_mb".to_string(),
            serde_json::json!(self.resources.memory_mb),
        );
        canonical.insert(
            "max_attempts".to_string(),
            serde_json::json!(self.max_attempts),
        );
        let bytes = serde_json::to_vec(&canonical).expect("canonical stage spec is serializable");
        let hash = blake3::hash(&bytes);
        let mut out = [0u8; 32];
        out.copy_from_slice(hash.as_bytes());
        out
    }
}

/// The full job graph. Stages are keyed by [`StageId`] so the runner can
/// iterate in deterministic order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobDag {
    pub job_id: JobId,
    pub name: String,
    pub stages: BTreeMap<StageId, StageSpec>,
}

impl JobDag {
    /// Construct a new job DAG. Validates that every dependency references a
    /// known stage and that the graph is acyclic.
    pub fn new(job_id: JobId, name: String, stages: Vec<StageSpec>) -> Result<Self, DagError> {
        let mut map: BTreeMap<StageId, StageSpec> = BTreeMap::new();
        for stage in stages {
            if map.insert(stage.id.clone(), stage.clone()).is_some() {
                return Err(DagError::DuplicateStage(stage.id));
            }
        }
        let dag = Self {
            job_id,
            name,
            stages: map,
        };
        dag.validate()?;
        Ok(dag)
    }

    /// Walk every dependency edge and report the first failure.
    pub fn validate(&self) -> Result<(), DagError> {
        for stage in self.stages.values() {
            for dep in &stage.dependencies {
                if !self.stages.contains_key(dep) {
                    return Err(DagError::UnknownDependency {
                        stage: stage.id.clone(),
                        dependency: dep.clone(),
                    });
                }
                if dep == &stage.id {
                    return Err(DagError::SelfDependency(stage.id.clone()));
                }
            }
        }
        let order = self.topological_order()?;
        if order.len() != self.stages.len() {
            return Err(DagError::Cycle);
        }
        Ok(())
    }

    /// Return a deterministic topological order. The result is `Err` if a
    /// cycle is detected.
    pub fn topological_order(&self) -> Result<Vec<StageId>, DagError> {
        let mut in_degree: BTreeMap<StageId, usize> = BTreeMap::new();
        for stage in self.stages.values() {
            // In-degree here is the number of dependencies this stage still
            // needs to wait on; it reaches 0 once every `dep` has run.
            in_degree.insert(stage.id.clone(), stage.dependencies.len());
        }
        let mut queue: BTreeSet<StageId> = self
            .stages
            .keys()
            .filter(|id| in_degree.get(*id).copied().unwrap_or(0) == 0)
            .cloned()
            .collect();
        let mut order: Vec<StageId> = Vec::new();
        while let Some(next) = queue.iter().next().cloned() {
            queue.remove(&next);
            order.push(next.clone());
            for stage in self.stages.values() {
                if stage.dependencies.iter().any(|d| d == &next) {
                    let entry = in_degree.entry(stage.id.clone()).or_insert(0);
                    if *entry > 0 {
                        *entry -= 1;
                        if *entry == 0 {
                            queue.insert(stage.id.clone());
                        }
                    }
                }
            }
        }
        if order.len() != self.stages.len() {
            Err(DagError::Cycle)
        } else {
            Ok(order)
        }
    }

    /// Compute the fingerprint of the whole DAG. The DAG fingerprint is a
    /// deterministic concatenation of every stage fingerprint and is used
    /// as part of the cache key so re-arranging stages invalidates caches.
    pub fn fingerprint(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.job_id.as_bytes());
        hasher.update(self.name.as_bytes());
        for (id, spec) in &self.stages {
            hasher.update(id.as_bytes());
            hasher.update(&spec.fingerprint());
        }
        let hash = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(hash.as_bytes());
        out
    }

    /// Find every stage whose dependencies have all succeeded. The result
    /// is in deterministic id order.
    pub fn ready_stages(&self, records: &BTreeMap<StageId, StageRecord>) -> Vec<StageId> {
        let mut ready: Vec<StageId> = self
            .stages
            .keys()
            .filter(|id| {
                let Some(stage) = self.stages.get(*id) else {
                    return false;
                };
                let Some(record) = records.get(*id) else {
                    return false;
                };
                if record.state != StageState::Pending {
                    return false;
                }
                stage
                    .dependencies
                    .iter()
                    .all(|dep| records.get(dep).map(|r| r.state == StageState::Succeeded).unwrap_or(false))
            })
            .cloned()
            .collect();
        ready.sort();
        ready
    }
}

/// Errors produced while validating or walking a DAG.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DagError {
    #[error("duplicate stage id: {0}")]
    DuplicateStage(StageId),
    #[error("stage {stage} depends on unknown stage {dependency}")]
    UnknownDependency { stage: StageId, dependency: StageId },
    #[error("stage {0} depends on itself")]
    SelfDependency(StageId),
    #[error("DAG contains a cycle")]
    Cycle,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stage(id: &str, deps: Vec<&str>) -> StageSpec {
        StageSpec {
            id: id.to_string(),
            kind: "test".to_string(),
            dependencies: deps.into_iter().map(String::from).collect(),
            parameters: serde_json::json!({}),
            resources: ResourceBudget::default(),
            max_attempts: 3,
        }
    }

    #[test]
    fn empty_dag_is_valid() {
        let dag = JobDag::new("j".to_string(), "empty".to_string(), vec![]).unwrap();
        assert!(dag.validate().is_ok());
        assert_eq!(dag.topological_order().unwrap(), Vec::<StageId>::new());
    }

    #[test]
    fn linear_dag_orders_dependencies_first() {
        let dag = JobDag::new(
            "j".to_string(),
            "linear".to_string(),
            vec![stage("a", vec![]), stage("b", vec!["a"]), stage("c", vec!["b"])],
        )
        .unwrap();
        let order = dag.topological_order().unwrap();
        assert_eq!(order, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    #[test]
    fn cycle_is_rejected() {
        let dag = JobDag::new(
            "j".to_string(),
            "cycle".to_string(),
            vec![stage("a", vec!["b"]), stage("b", vec!["a"])],
        );
        assert_eq!(dag.unwrap_err(), DagError::Cycle);
    }

    #[test]
    fn unknown_dependency_is_rejected() {
        let dag = JobDag::new(
            "j".to_string(),
            "missing".to_string(),
            vec![stage("a", vec!["ghost"])],
        );
        assert!(matches!(dag.unwrap_err(), DagError::UnknownDependency { .. }));
    }

    #[test]
    fn fingerprint_changes_when_dependency_order_changes() {
        let a = JobDag::new(
            "j".to_string(),
            "x".to_string(),
            vec![stage("a", vec![]), stage("b", vec!["a"])],
        )
        .unwrap();
        let b = JobDag::new(
            "j".to_string(),
            "x".to_string(),
            vec![stage("a", vec![]), stage("c", vec!["a"])],
        )
        .unwrap();
        assert_ne!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn fingerprint_is_stable_for_identical_dag() {
        let a = JobDag::new(
            "j".to_string(),
            "x".to_string(),
            vec![stage("a", vec![]), stage("b", vec!["a"])],
        )
        .unwrap();
        let b = JobDag::new(
            "j".to_string(),
            "x".to_string(),
            vec![stage("a", vec![]), stage("b", vec!["a"])],
        )
        .unwrap();
        assert_eq!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn ready_stages_returns_only_pending_with_succeeded_deps() {
        let dag = JobDag::new(
            "j".to_string(),
            "ready".to_string(),
            vec![stage("a", vec![]), stage("b", vec!["a"])],
        )
        .unwrap();
        let mut records: BTreeMap<StageId, StageRecord> = BTreeMap::new();
        records.insert("a".to_string(), StageRecord::succeeded("a", [0u8; 32]));
        records.insert("b".to_string(), StageRecord::pending("b"));
        assert_eq!(dag.ready_stages(&records), vec!["b".to_string()]);
    }
}
