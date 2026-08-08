//! MCP tool registry (CR-V2-B2-025).
//!
//! The adapter stores a generated mapping from tool id to
//! [`ToolDescriptor`]. The mapping is produced by the capabilities crate
//! (`render_mcp_tool_registry`) and handed to the adapter at bind time so
//! the adapter never invents its own routing table.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Schema marker for tool descriptors.
pub const TOOL_DESCRIPTOR_SCHEMA: &str = "cutright.mcp_tool/v1";

/// Per-tool descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    /// Schema marker.
    pub schema: String,
    /// Capability id (same as the registry entry).
    pub capability_id: String,
    /// Mutation or read.
    pub kind: ToolKind,
    /// Owner component for diagnostics.
    pub owner_component: String,
    /// Permission set required.
    pub permission_set: String,
    /// Description used in tool listings.
    pub description: String,
    /// JSON Schema for the input payload.
    pub input_schema: serde_json::Value,
}

impl ToolDescriptor {
    /// Public helper: is this a mutation tool?
    pub fn is_mutation(&self) -> bool {
        matches!(self.kind, ToolKind::Mutation)
    }
}

/// Read vs mutation. Mirrors `CapabilityKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Read,
    Mutation,
}

/// Registry of MCP tools. The default registry is empty; tests populate
/// it with synthetic entries that mirror the contract document.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolRegistry {
    tools: BTreeMap<String, ToolDescriptor>,
}

impl McpToolRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a descriptor; overwrites if the id is already present.
    pub fn insert(&mut self, descriptor: ToolDescriptor) {
        self.tools
            .insert(descriptor.capability_id.clone(), descriptor);
    }

    /// Look up a tool by id.
    pub fn lookup(&self, id: &str) -> Option<&ToolDescriptor> {
        self.tools.get(id)
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// True when the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Iterator over the registered tools in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &ToolDescriptor)> {
        self.tools.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Build a registry from a JSON envelope produced by
    /// `video_capabilities::codegen::render_mcp_tool_registry`.
    pub fn from_generated_envelope(
        envelope: &serde_json::Value,
    ) -> Result<Self, serde_json::Error> {
        let parsed: GeneratedEnvelope = serde_json::from_value(envelope.clone())?;
        let mut out = Self::new();
        for (_, entry) in parsed.tools {
            out.insert(ToolDescriptor {
                schema: entry.schema,
                capability_id: entry.capability_id,
                kind: entry.kind,
                owner_component: entry.owner_component,
                permission_set: entry.permission_set,
                description: entry.description,
                input_schema: entry.input_schema,
            });
        }
        Ok(out)
    }
}

/// Mirror of the codegen envelope; mirrors the JSON shape produced by
/// `render_mcp_tool_registry`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeneratedEnvelope {
    /// Schema marker.
    pub schema: String,
    /// Source registry id.
    pub registry_id: String,
    /// Generator schema marker.
    pub generator: String,
    /// Map of tool id -> descriptor.
    pub tools: BTreeMap<String, GeneratedToolEntry>,
}

/// Per-tool entry in the codegen envelope.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeneratedToolEntry {
    /// Schema marker.
    pub schema: String,
    /// Capability id.
    pub capability_id: String,
    /// Mutation or read.
    pub kind: ToolKind,
    /// Owner component.
    pub owner_component: String,
    /// Permission set.
    pub permission_set: String,
    /// Description.
    pub description: String,
    /// JSON Schema for input payload.
    pub input_schema: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(id: &str, kind: ToolKind, permission_set: &str) -> ToolDescriptor {
        ToolDescriptor {
            schema: TOOL_DESCRIPTOR_SCHEMA.to_string(),
            capability_id: id.to_string(),
            kind,
            owner_component: "video-actions".to_string(),
            permission_set: permission_set.to_string(),
            description: format!("Test tool {id}"),
            input_schema: serde_json::json!({}),
        }
    }

    #[test]
    fn round_trip_envelope() {
        let mut reg = McpToolRegistry::new();
        reg.insert(descriptor(
            "timeline.cut",
            ToolKind::Mutation,
            "pset.editorial_engine",
        ));
        reg.insert(descriptor(
            "evidence.read",
            ToolKind::Read,
            "pset.evidence_read_only",
        ));
        let value = serde_json::to_value(&reg).unwrap();
        let parsed = McpToolRegistry::from_generated_envelope(&value).unwrap();
        assert_eq!(parsed.len(), 2);
        assert!(parsed.lookup("timeline.cut").unwrap().is_mutation());
        assert!(!parsed.lookup("evidence.read").unwrap().is_mutation());
    }

    #[test]
    fn lookup_unknown_returns_none() {
        let reg = McpToolRegistry::new();
        assert!(reg.lookup("nonexistent").is_none());
    }
}
