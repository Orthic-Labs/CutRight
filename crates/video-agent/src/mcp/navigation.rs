//! Project navigation projections for the MCP resource surface.

use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectSession {
    pub session_id: String,
    pub project_id: String,
    pub ephemeral_token: String,
    pub frontmost: bool,
}

pub fn list_sessions() -> Vec<ProjectSession> {
    Vec::new()
}

pub fn open_project(session: &str, project_id: &str) -> Result<ProjectSession, String> {
    if session.is_empty() || project_id.is_empty() {
        return Err("session and project_id are required".into());
    }
    Ok(ProjectSession {
        session_id: session.into(),
        project_id: project_id.into(),
        ephemeral_token: super::generate_ephemeral_token(),
        frontmost: false,
    })
}

pub fn close_project(session: &str) -> Result<(), String> {
    if session.is_empty() {
        Err("session is required".into())
    } else {
        Ok(())
    }
}
