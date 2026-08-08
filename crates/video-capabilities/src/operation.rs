//! Goal-level operation contracts owned by the capability registry.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::CapabilityId;

/// Schema identifier for a goal-level operation contract.
pub const OPERATION_CONTRACT_SCHEMA: &str = "cutright.operation_contract/v1";

/// Stable identifiers for the initial external operation catalog.
pub const GOAL_OPERATION_IDS: &[&str] = &[
    "cutright.project.open_or_create",
    "cutright.edit.prepare",
    "cutright.evidence.query",
    "cutright.edit.preview",
    "cutright.edit.commit",
    "cutright.render.finish",
    "cutright.job.status",
    "cutright.job.input",
    "cutright.job.cancel",
    "cutright.job.resume",
    "cutright.job.result",
];

/// Stable opaque identifier for an operation handler.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(transparent)]
pub struct HandlerId(pub String);

impl HandlerId {
    /// Construct a handler identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return true for a stable lowercase dotted identifier.
    pub fn is_well_formed(value: &str) -> bool {
        let mut chars = value.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        first.is_ascii_lowercase()
            && value.len() > 1
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
    }
}

impl std::fmt::Display for HandlerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Execution boundary exposed by an operation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    /// Returns a bounded read without mutation.
    Read,
    /// Validates or derives a result without mutation.
    Compute,
    /// Performs a synchronous mutation.
    Write,
    /// Starts or advances durable work.
    Process,
    /// Performs a mutation that may also start durable work.
    WriteProcess,
}

/// Retry behavior promised by an operation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Idempotency {
    /// Repeating the request has the same effect.
    Idempotent,
    /// A request key makes retries safe.
    RetrySafe,
}

/// Lifecycle state emitted by an operation or its durable job.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    /// Request was accepted.
    Accepted,
    /// Work is executing.
    Running,
    /// User or caller input is required.
    InputRequired,
    /// Work was cancelled while preserving state.
    Cancelled,
    /// Work completed successfully.
    Completed,
    /// Work failed with a typed error.
    Failed,
}

/// Typed failure exposed by an operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperationError {
    /// Stable machine-readable error code.
    pub code: String,
    /// Human-readable explanation.
    pub description: String,
    /// Whether retrying can succeed without changing input.
    pub retryable: bool,
}

/// Schema-valid example for an operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperationExample {
    /// Short example label.
    pub name: String,
    /// Example request payload.
    pub input: serde_json::Value,
    /// Example response payload.
    pub output: serde_json::Value,
}

/// Complete metadata contract for one advertised operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperationContract {
    /// Capability-registry key for this operation.
    pub capability: CapabilityId,
    /// Stable implementation binding.
    pub handler_id: HandlerId,
    /// What the operation does.
    pub purpose: String,
    /// Situations in which callers should select it.
    pub use_when: Vec<String>,
    /// Situations in which callers must not select it.
    pub do_not_use: Vec<String>,
    /// Conditions that must hold before execution.
    pub preconditions: Vec<String>,
    /// Strict JSON schema for the operation output.
    pub output_schema: serde_json::Value,
    /// Durable or externally visible effects.
    pub effects: Vec<String>,
    /// Execution boundary.
    pub execution_mode: ExecutionMode,
    /// Retry guarantee.
    pub idempotency: Idempotency,
    /// States this operation may produce.
    pub lifecycle: Vec<LifecycleState>,
    /// Typed failures callers can handle.
    pub errors: Vec<OperationError>,
    /// Canonical request/response examples.
    pub examples: Vec<OperationExample>,
}

impl OperationContract {
    fn validate(&self) -> Result<(), OperationRegistryError> {
        if !CapabilityId::is_well_formed(self.capability.as_str()) {
            return Err(OperationRegistryError::InvalidContract {
                operation: self.capability.to_string(),
                reason: "capability must be a lowercase dotted id".into(),
            });
        }
        if !HandlerId::is_well_formed(self.handler_id.as_str()) {
            return Err(OperationRegistryError::InvalidContract {
                operation: self.capability.to_string(),
                reason: "handler_id must be a lowercase dotted id".into(),
            });
        }
        if !GOAL_OPERATION_IDS.contains(&self.capability.as_str()) {
            return Err(OperationRegistryError::UnexpectedOperation(
                self.capability.to_string(),
            ));
        }
        for (field, values) in [
            ("use_when", &self.use_when),
            ("do_not_use", &self.do_not_use),
            ("preconditions", &self.preconditions),
            ("effects", &self.effects),
        ] {
            if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
                return Err(OperationRegistryError::InvalidContract {
                    operation: self.capability.to_string(),
                    reason: format!("{field} must contain non-empty entries"),
                });
            }
        }
        if self.lifecycle.is_empty() || self.errors.is_empty() || self.examples.is_empty() {
            return Err(OperationRegistryError::InvalidContract {
                operation: self.capability.to_string(),
                reason: "lifecycle, errors, and examples are required".into(),
            });
        }
        let is_strict_object = self.output_schema.get("type")
            == Some(&serde_json::Value::String("object".into()))
            && self.output_schema.get("additionalProperties")
                == Some(&serde_json::Value::Bool(false));
        if !is_strict_object {
            return Err(OperationRegistryError::InvalidContract {
                operation: self.capability.to_string(),
                reason: "output_schema must be a strict object schema".into(),
            });
        }
        Ok(())
    }
}

/// Operation registry validation failure.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum OperationRegistryError {
    /// A contract has invalid metadata.
    #[error("invalid operation contract {operation}: {reason}")]
    InvalidContract {
        /// Operation with invalid metadata.
        operation: String,
        /// Validation reason.
        reason: String,
    },
    /// Two contracts advertise the same operation.
    #[error("duplicate advertised operation: {0}")]
    DuplicateOperation(String),
    /// Two operations bind to one handler.
    #[error("handler {handler} is bound to multiple operations: {first} and {second}")]
    DuplicateHandler {
        /// Handler with conflicting bindings.
        handler: String,
        /// First operation.
        first: String,
        /// Second operation.
        second: String,
    },
    /// An advertised operation lacks a live handler.
    #[error("advertised operation {operation} has no live handler {handler}")]
    MissingHandler {
        /// Operation without a handler.
        operation: String,
        /// Required handler.
        handler: String,
    },
    /// A live handler has no advertised operation.
    #[error("live handler is not advertised: {0}")]
    UnadvertisedHandler(String),
    /// A non-goal operation was inserted in the external catalog.
    #[error("unexpected goal-level operation: {0}")]
    UnexpectedOperation(String),
    /// The catalog does not contain exactly the frozen goal-level set.
    #[error("expected exactly eleven goal-level operations, found {found}")]
    WrongOperationCount {
        /// Number of operations found.
        found: usize,
    },
}

/// Registry of operation contracts keyed by [`CapabilityId`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationRegistry {
    contracts: BTreeMap<CapabilityId, OperationContract>,
}

impl OperationRegistry {
    /// Build a registry and reject duplicate operation keys.
    pub fn from_contracts<I>(contracts: I) -> Result<Self, OperationRegistryError>
    where
        I: IntoIterator<Item = OperationContract>,
    {
        let mut registry = Self::default();
        for contract in contracts {
            let key = contract.capability.clone();
            if registry.contracts.insert(key.clone(), contract).is_some() {
                return Err(OperationRegistryError::DuplicateOperation(key.to_string()));
            }
        }
        Ok(registry)
    }

    /// Return a contract by operation id.
    pub fn get(&self, operation: &str) -> Option<&OperationContract> {
        self.contracts.get(&CapabilityId::new(operation))
    }

    /// Return all advertised contracts in stable order.
    pub fn contracts(&self) -> impl Iterator<Item = &OperationContract> {
        self.contracts.values()
    }

    /// Validate the frozen eleven-operation catalog and its metadata.
    pub fn validate(&self) -> Result<(), OperationRegistryError> {
        if self.contracts.len() != GOAL_OPERATION_IDS.len() {
            return Err(OperationRegistryError::WrongOperationCount {
                found: self.contracts.len(),
            });
        }
        for expected in GOAL_OPERATION_IDS {
            let Some(contract) = self.get(expected) else {
                return Err(OperationRegistryError::InvalidContract {
                    operation: (*expected).into(),
                    reason: "required goal-level operation is missing".into(),
                });
            };
            contract.validate()?;
        }
        let mut handlers = BTreeMap::new();
        for contract in self.contracts() {
            if let Some(first) =
                handlers.insert(contract.handler_id.clone(), contract.capability.clone())
            {
                return Err(OperationRegistryError::DuplicateHandler {
                    handler: contract.handler_id.to_string(),
                    first: first.to_string(),
                    second: contract.capability.to_string(),
                });
            }
        }
        Ok(())
    }

    /// Validate that every advertised operation has exactly one live handler.
    pub fn validate_with_live_handlers<I, S>(
        &self,
        live_handlers: I,
    ) -> Result<(), OperationRegistryError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.validate()?;
        let live: BTreeSet<String> = live_handlers
            .into_iter()
            .map(|id| id.as_ref().to_string())
            .collect();
        for contract in self.contracts() {
            if !live.contains(contract.handler_id.as_str()) {
                return Err(OperationRegistryError::MissingHandler {
                    operation: contract.capability.to_string(),
                    handler: contract.handler_id.to_string(),
                });
            }
        }
        for handler in live {
            if !self
                .contracts()
                .any(|contract| contract.handler_id.as_str() == handler)
            {
                return Err(OperationRegistryError::UnadvertisedHandler(handler));
            }
        }
        Ok(())
    }
}

/// Handler identifiers that are live in the registry's current runtime.
pub const LIVE_HANDLER_IDS: &[&str] = &[
    "handler.project.open_or_create",
    "handler.edit.prepare",
    "handler.evidence.query",
    "handler.edit.preview",
    "handler.edit.commit",
    "handler.render.finish",
    "handler.job.status",
    "handler.job.input",
    "handler.job.cancel",
    "handler.job.resume",
    "handler.job.result",
];

fn strict_output(properties: &[(&str, &str)]) -> serde_json::Value {
    let properties = properties
        .iter()
        .map(|(name, kind)| ((*name).to_string(), serde_json::json!({ "type": kind })))
        .collect::<serde_json::Map<_, _>>();
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": properties
    })
}

fn contract(
    capability: &str,
    handler_id: &str,
    purpose: &str,
    execution_mode: ExecutionMode,
    idempotency: Idempotency,
    effects: &[&str],
    output_properties: &[(&str, &str)],
) -> OperationContract {
    OperationContract {
        capability: CapabilityId::new(capability),
        handler_id: HandlerId::new(handler_id),
        purpose: purpose.into(),
        use_when: vec![format!("{purpose} is required")],
        do_not_use: vec!["The caller lacks a project or approved authority".into()],
        preconditions: vec!["All referenced handles are valid and current".into()],
        output_schema: strict_output(output_properties),
        effects: effects.iter().map(|effect| (*effect).into()).collect(),
        execution_mode,
        idempotency,
        lifecycle: vec![
            LifecycleState::Accepted,
            LifecycleState::Running,
            LifecycleState::Completed,
            LifecycleState::Failed,
        ],
        errors: vec![OperationError {
            code: "invalid_request".into(),
            description: "Request failed typed validation".into(),
            retryable: false,
        }],
        examples: vec![OperationExample {
            name: "minimal".into(),
            input: serde_json::json!({}),
            output: serde_json::json!({}),
        }],
    }
}

/// Build the canonical eleven-operation registry.
pub fn default_operation_registry() -> OperationRegistry {
    OperationRegistry::from_contracts([
        contract(
            "cutright.project.open_or_create",
            "handler.project.open_or_create",
            "Bind immutable sources to a project",
            ExecutionMode::Write,
            Idempotency::Idempotent,
            &["Creates or returns project and revision handles"],
            &[("project_id", "string"), ("revision_id", "string")],
        ),
        contract(
            "cutright.edit.prepare",
            "handler.edit.prepare",
            "Prepare media and durable analysis",
            ExecutionMode::WriteProcess,
            Idempotency::RetrySafe,
            &["Creates a durable preparation task"],
            &[("task_id", "string")],
        ),
        contract(
            "cutright.evidence.query",
            "handler.evidence.query",
            "Return bounded editorial evidence",
            ExecutionMode::Read,
            Idempotency::Idempotent,
            &["Reads bounded evidence only"],
            &[("items", "array")],
        ),
        contract(
            "cutright.edit.preview",
            "handler.edit.preview",
            "Validate a plan without mutation",
            ExecutionMode::Compute,
            Idempotency::Idempotent,
            &["Returns a semantic diff without changing project state"],
            &[("valid", "boolean"), ("diff", "object")],
        ),
        contract(
            "cutright.edit.commit",
            "handler.edit.commit",
            "Apply an approved plan",
            ExecutionMode::Write,
            Idempotency::RetrySafe,
            &["Creates a new project revision and receipt"],
            &[("revision_id", "string"), ("receipt_id", "string")],
        ),
        contract(
            "cutright.render.finish",
            "handler.render.finish",
            "Render a selected variant and run QA",
            ExecutionMode::WriteProcess,
            Idempotency::RetrySafe,
            &["Creates render artifacts and evidence"],
            &[("task_id", "string")],
        ),
        contract(
            "cutright.job.status",
            "handler.job.status",
            "Return grounded task progress",
            ExecutionMode::Read,
            Idempotency::Idempotent,
            &["Reads durable task state"],
            &[("status", "string")],
        ),
        contract(
            "cutright.job.input",
            "handler.job.input",
            "Resolve one typed input requirement",
            ExecutionMode::Write,
            Idempotency::RetrySafe,
            &["Persists the supplied input and advances the task"],
            &[("status", "string")],
        ),
        contract(
            "cutright.job.cancel",
            "handler.job.cancel",
            "Cancel work while preserving state",
            ExecutionMode::WriteProcess,
            Idempotency::Idempotent,
            &["Persists cancellation and preserves prior outputs"],
            &[("status", "string")],
        ),
        contract(
            "cutright.job.resume",
            "handler.job.resume",
            "Resume from a verified checkpoint",
            ExecutionMode::WriteProcess,
            Idempotency::RetrySafe,
            &["Starts a new durable attempt"],
            &[("task_id", "string")],
        ),
        contract(
            "cutright.job.result",
            "handler.job.result",
            "Return outcome artifacts and receipts",
            ExecutionMode::Read,
            Idempotency::Idempotent,
            &["Reads completed task result and claim boundary"],
            &[
                ("status", "string"),
                ("artifacts", "array"),
                ("receipts", "array"),
            ],
        ),
    ])
    .expect("canonical operation catalog is valid")
}

/// Validate the canonical operation catalog against its live handler set.
pub fn validate_default_operation_registry() -> Result<(), OperationRegistryError> {
    default_operation_registry().validate_with_live_handlers(LIVE_HANDLER_IDS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_catalog_has_exactly_eleven_live_handlers() {
        let registry = default_operation_registry();
        registry
            .validate_with_live_handlers(LIVE_HANDLER_IDS)
            .expect("all operations have one live handler");
        assert_eq!(registry.contracts().count(), 11);
    }

    #[test]
    fn missing_handler_fails_validation() {
        let registry = default_operation_registry();
        let live = LIVE_HANDLER_IDS
            .iter()
            .copied()
            .filter(|id| *id != "handler.job.result");
        let error = registry
            .validate_with_live_handlers(live)
            .expect_err("missing handlers must fail closed");
        assert!(matches!(
            error,
            OperationRegistryError::MissingHandler { .. }
        ));
    }
}
