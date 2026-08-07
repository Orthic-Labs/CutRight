// crates/video-agent/src/mcp/navigation.rs — CR-V2-B6-020 Lane C.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectSession { pub session_id: String, pub project_id: String, pub ephemeral_token: String, pub frontmost: bool }
pub fn list_sessions() -> Vec<ProjectSession> { vec![] }
pub fn open_project(_session: &str, _project_id: &str) -> Result<ProjectSession, String> { Err("not-implemented".into()) }
pub fn close_project(_session: &str) -> Result<(), String> { Ok(()) }
