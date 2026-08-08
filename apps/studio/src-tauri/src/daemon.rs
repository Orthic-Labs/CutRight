use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRoute {
    pub provider: String,
    pub executable: String,
    pub model: String,
    pub guided_qualified: bool,
    pub native_ready: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub id: String,
    pub kind: String,
    pub text: String,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub session_id: String,
    pub goal: String,
    pub provider: String,
    pub model: String,
    pub status: String,
    pub plan: Option<String>,
    pub events: Vec<AgentEvent>,
    pub approval: Option<serde_json::Value>,
    pub progress: Option<f32>,
    pub result: Option<String>,
}
#[derive(Default)]
pub struct AgentDaemonState {
    sessions: Mutex<BTreeMap<String, AgentSession>>,
}

fn routes() -> Vec<AgentRoute> {
    ["claude_code", "codex"]
        .into_iter()
        .map(|provider| AgentRoute {
            provider: provider.into(),
            executable: provider.replace('_', "-"),
            model: "user-selected".into(),
            guided_qualified: false,
            native_ready: false,
        })
        .collect()
}
#[tauri::command]
pub fn agent_routes() -> Vec<AgentRoute> {
    routes()
}
#[tauri::command]
pub fn agent_session_start(
    state: State<'_, AgentDaemonState>,
    goal: String,
    provider: String,
    path: String,
) -> Result<AgentSession, String> {
    if goal.trim().is_empty() || !["claude_code", "codex"].contains(&provider.as_str()) {
        return Err("guided session requires a goal and supported provider".into());
    }
    let id = format!("guided-{}", uuid::Uuid::new_v4());
    let session = AgentSession {
        session_id: id.clone(),
        goal,
        provider,
        model: "user-selected".into(),
        status: "input_required".into(),
        plan: None,
        events: vec![AgentEvent {
            id: "created".into(),
            kind: "receipt".into(),
            text: format!("session bound to {path}"),
            created_at: chrono::Utc::now().to_rfc3339(),
        }],
        approval: None,
        progress: Some(0.0),
        result: None,
    };
    state
        .sessions
        .lock()
        .map_err(|_| "agent state unavailable".to_string())?
        .insert(id, session.clone());
    Ok(session)
}
#[tauri::command]
pub fn agent_session_events(
    state: State<'_, AgentDaemonState>,
    session_id: String,
) -> Result<AgentSession, String> {
    state
        .sessions
        .lock()
        .map_err(|_| "agent state unavailable".to_string())?
        .get(&session_id)
        .cloned()
        .ok_or_else(|| "agent session not found".into())
}
fn set_status(
    state: State<'_, AgentDaemonState>,
    session_id: String,
    status: &str,
) -> Result<(), String> {
    let mut sessions = state
        .sessions
        .lock()
        .map_err(|_| "agent state unavailable".to_string())?;
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| "agent session not found".to_string())?;
    session.status = status.into();
    Ok(())
}
#[tauri::command]
pub fn agent_session_pause(
    state: State<'_, AgentDaemonState>,
    session_id: String,
) -> Result<(), String> {
    set_status(state, session_id, "paused")
}
#[tauri::command]
pub fn agent_session_resume(
    state: State<'_, AgentDaemonState>,
    session_id: String,
) -> Result<(), String> {
    set_status(state, session_id, "running")
}
#[tauri::command]
pub fn agent_session_cancel(
    state: State<'_, AgentDaemonState>,
    session_id: String,
) -> Result<(), String> {
    set_status(state, session_id, "completed")
}
