//! Guided provider sessions owned by CutRight.
//!
//! Vendor conversation identifiers are transport details. Durable state here
//! is the CutRight goal, plan, evidence, approvals, summaries, errors, and
//! receipts. Provider reasoning is never part of a session record.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    ClaudeCode,
    Codex,
}

impl Provider {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude_code",
            Self::Codex => "codex",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRoute {
    pub provider: Provider,
    pub executable: String,
    pub model: String,
    pub guided_qualified: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Starting,
    Running,
    InputRequired,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionReceipt {
    pub id: String,
    pub provider: Provider,
    pub status: SessionStatus,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    pub id: String,
    pub goal: String,
    pub provider: Provider,
    pub model: String,
    pub vendor_session_id: Option<String>,
    pub status: SessionStatus,
    pub plan: Vec<String>,
    pub evidence: Vec<String>,
    pub tool_calls: Vec<String>,
    pub approvals: Vec<String>,
    pub summaries: Vec<String>,
    pub errors: Vec<String>,
    pub receipts: Vec<SessionReceipt>,
}

impl SessionRecord {
    fn new(id: String, goal: String, route: &ProviderRoute) -> Self {
        Self {
            id,
            goal,
            provider: route.provider,
            model: route.model.clone(),
            vendor_session_id: None,
            status: SessionStatus::Starting,
            plan: Vec::new(),
            evidence: Vec::new(),
            tool_calls: Vec::new(),
            approvals: Vec::new(),
            summaries: Vec::new(),
            errors: Vec::new(),
            receipts: Vec::new(),
        }
    }

    fn receipt(&mut self, summary: impl Into<String>) {
        let summary = summary.into();
        self.summaries.push(summary.clone());
        self.receipts.push(SessionReceipt {
            id: format!("{}:receipt:{}", self.id, self.receipts.len() + 1),
            provider: self.provider,
            status: self.status,
            summary,
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEvent {
    ProviderReady,
    ProviderFailed,
    ApprovalRequested,
    ToolCall,
    ToolResult,
    TurnComplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    Unavailable,
    Preselected(ProviderRoute),
    RequiresExplicitChoice(Vec<ProviderRoute>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    NoGuidedRoute,
    ExplicitProviderRequired,
    ProviderNotQualified,
    SessionNotFound,
    InvalidTool,
    InvalidGoal,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderPolicy {
    pub saved_default: Option<Provider>,
    pub automatic_fallback: bool,
}

impl ProviderPolicy {
    pub fn select(&self, routes: &[ProviderRoute]) -> Selection {
        let ready: Vec<ProviderRoute> = routes
            .iter()
            .filter(|route| route.guided_qualified)
            .cloned()
            .collect();
        match ready.len() {
            0 => Selection::Unavailable,
            1 => Selection::Preselected(ready[0].clone()),
            _ => self
                .saved_default
                .and_then(|provider| ready.iter().find(|route| route.provider == provider).cloned())
                .map(Selection::Preselected)
                .unwrap_or(Selection::RequiresExplicitChoice(ready)),
        }
    }

    pub fn fallback_allowed(&self) -> bool {
        self.automatic_fallback
    }
}

#[derive(Debug, Default)]
pub struct SessionStore {
    sessions: BTreeMap<String, SessionRecord>,
    next_id: u64,
}

impl SessionStore {
    pub fn start(&mut self, goal: impl Into<String>, route: ProviderRoute) -> Result<String, SessionError> {
        let goal = goal.into();
        if goal.trim().is_empty() || !route.guided_qualified {
            return Err(if goal.trim().is_empty() { SessionError::InvalidGoal } else { SessionError::ProviderNotQualified });
        }
        self.next_id += 1;
        let id = format!("guided-{}", self.next_id);
        let record = SessionRecord::new(id.clone(), goal, &route);
        self.sessions.insert(id.clone(), record);
        Ok(id)
    }

    pub fn get(&self, id: &str) -> Result<&SessionRecord, SessionError> {
        self.sessions.get(id).ok_or(SessionError::SessionNotFound)
    }

    pub fn get_mut(&mut self, id: &str) -> Result<&mut SessionRecord, SessionError> {
        self.sessions.get_mut(id).ok_or(SessionError::SessionNotFound)
    }

    pub fn on_event(&mut self, id: &str, event: SessionEvent, detail: impl Into<String>) -> Result<(), SessionError> {
        let detail = detail.into();
        let session = self.get_mut(id)?;
        match event {
            SessionEvent::ProviderReady => session.status = SessionStatus::Running,
            SessionEvent::ProviderFailed => {
                session.status = SessionStatus::InputRequired;
                session.errors.push(detail);
            }
            SessionEvent::ApprovalRequested => session.approvals.push(detail),
            SessionEvent::ToolCall => session.tool_calls.push(detail),
            SessionEvent::ToolResult => session.evidence.push(detail),
            SessionEvent::TurnComplete => {
                session.status = SessionStatus::Completed;
                session.receipt(detail);
            }
        }
        Ok(())
    }

    pub fn pause(&mut self, id: &str) -> Result<(), SessionError> {
        let session = self.get_mut(id)?;
        if matches!(session.status, SessionStatus::Running | SessionStatus::Starting) {
            session.status = SessionStatus::Paused;
        }
        Ok(())
    }

    pub fn resume(&mut self, id: &str) -> Result<(), SessionError> {
        let session = self.get_mut(id)?;
        if matches!(session.status, SessionStatus::Paused | SessionStatus::InputRequired) {
            session.status = SessionStatus::Running;
        }
        Ok(())
    }

    /// Continue with a new provider from CutRight-owned state only.
    pub fn continue_with(&mut self, source_id: &str, route: ProviderRoute) -> Result<String, SessionError> {
        let source = self.get(source_id)?.clone();
        let id = self.start(source.goal, route)?;
        let target = self.get_mut(&id)?;
        target.plan = source.plan;
        target.evidence = source.evidence;
        target.summaries = source.summaries;
        target.receipts = source.receipts;
        Ok(id)
    }
}

pub fn registered_tool_allowed(tool_name: &str) -> bool {
    matches!(tool_name, "cutright.inspect" | "cutright.read_transcript" | "cutright.draft_plan" | "cutright.apply_plan" | "cutright.render_artifact")
}

pub fn provider_output_can_change_state(tool_name: &str) -> bool {
    registered_tool_allowed(tool_name) && matches!(tool_name, "cutright.apply_plan" | "cutright.render_artifact")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(provider: Provider, ready: bool) -> ProviderRoute {
        ProviderRoute { provider, executable: format!("/usr/local/bin/{}", provider.label()), model: "user-selected".into(), guided_qualified: ready }
    }

    #[test]
    fn selection_has_no_fake_ready_state() {
        assert_eq!(ProviderPolicy::default().select(&[]), Selection::Unavailable);
        assert!(matches!(ProviderPolicy::default().select(&[route(Provider::ClaudeCode, true), route(Provider::Codex, true)]), Selection::RequiresExplicitChoice(_)));
    }

    #[test]
    fn provider_failure_pauses_for_user_input_without_fallback() {
        let mut store = SessionStore::default();
        let id = store.start("make a short", route(Provider::ClaudeCode, true)).unwrap();
        store.on_event(&id, SessionEvent::ProviderFailed, "network error").unwrap();
        assert_eq!(store.get(&id).unwrap().status, SessionStatus::InputRequired);
        assert!(!ProviderPolicy::default().fallback_allowed());
    }

    #[test]
    fn continuation_copies_owned_state_and_not_vendor_id() {
        let mut store = SessionStore::default();
        let first = store.start("goal", route(Provider::ClaudeCode, true)).unwrap();
        store.get_mut(&first).unwrap().plan.push("draft".into());
        store.get_mut(&first).unwrap().vendor_session_id = Some("claude-internal".into());
        let second = store.continue_with(&first, route(Provider::Codex, true)).unwrap();
        assert_eq!(store.get(&second).unwrap().plan, vec!["draft"]);
        assert!(store.get(&second).unwrap().vendor_session_id.is_none());
    }

    #[test]
    fn only_registered_cutright_tools_have_authority() {
        assert!(registered_tool_allowed("cutright.inspect"));
        assert!(!registered_tool_allowed("shell.exec"));
        assert!(provider_output_can_change_state("cutright.apply_plan"));
        assert!(!provider_output_can_change_state("cutright.inspect"));
    }
}
