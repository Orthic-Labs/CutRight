// crates/video-agent/src/turn.rs — CR-V2-B6-017.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentTurn { pub turn_id: String, pub role: String, pub tool_calls: Vec<String>, pub result_refs: Vec<String> }
