// crates/video-agent/src/session.rs — CR-V2-B6-017 Lane C.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentSession {
    pub binding: String,
    pub observed_revision: String,
    pub plan: Option<String>,
    pub turn_log_refs: Vec<String>,
    pub tool_state: serde_json::Value,
    pub token_budget: u64,
    pub resource_budget: u64,
}
impl AgentSession {
    pub fn new(binding: &str, revision: &str) -> Self {
        Self { binding: binding.into(), observed_revision: revision.into(), plan: None, turn_log_refs: vec![], tool_state: serde_json::json!({}), token_budget: 0, resource_budget: 0 }
    }
}
