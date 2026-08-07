// crates/video-agent/src/registry.rs — CR-V2-B6-017.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolRegistryEntry { pub tool_id: String, pub schema_hash: String, pub correction_common: bool }
pub fn registry_entries() -> Vec<ToolRegistryEntry> { vec![] }
