//! Daemon-owned Native CLI terminal sessions.
//!
//! Terminal bytes are presentation only. Project and job truth comes from
//! structured CutRight operations and receipts, never ANSI parsing.

use std::collections::VecDeque;

pub const MAX_TERMINAL_BYTES: usize = 256 * 1024;
pub const CUTRIGHT_MCP_CONFIG: &str = "cutright";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalDimensions {
    pub columns: u16,
    pub rows: u16,
}

impl Default for TerminalDimensions {
    fn default() -> Self {
        Self { columns: 120, rows: 40 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScratchSandbox {
    pub root: String,
    pub allows_project_package: bool,
    pub allows_raw_media: bool,
    pub allows_unregistered_mcp: bool,
}

impl ScratchSandbox {
    pub fn strict(root: impl Into<String>) -> Self {
        Self { root: root.into(), allows_project_package: false, allows_raw_media: false, allows_unregistered_mcp: false }
    }

    pub fn permits_path(&self, path: &str) -> bool {
        path.starts_with(&self.root) && !path.contains("project") && !path.ends_with(".mp4") && !path.ends_with(".mov")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalAttachToken(String);

impl TerminalAttachToken {
    pub fn new(value: impl Into<String>) -> Option<Self> {
        let value = value.into();
        (!value.is_empty() && value.len() <= 256).then_some(Self(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalState {
    Starting,
    Running,
    WaitingForPrompt,
    Exited { code: Option<i32> },
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSnapshot {
    pub session_id: String,
    pub dimensions: TerminalDimensions,
    pub state: TerminalState,
    pub attach_cursor: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct TerminalSession {
    pub session_id: String,
    pub process_group_id: i32,
    pub dimensions: TerminalDimensions,
    pub sandbox: ScratchSandbox,
    pub state: TerminalState,
    attach_token: TerminalAttachToken,
    bytes: VecDeque<u8>,
    cursor: u64,
    attached_displays: usize,
}

impl TerminalSession {
    pub fn start(session_id: impl Into<String>, process_group_id: i32, token: TerminalAttachToken, sandbox: ScratchSandbox) -> Self {
        Self { session_id: session_id.into(), process_group_id, dimensions: TerminalDimensions::default(), sandbox, state: TerminalState::Starting, attach_token: token, bytes: VecDeque::new(), cursor: 0, attached_displays: 0 }
    }

    pub fn authenticate(&self, token: &TerminalAttachToken) -> bool { self.attach_token == *token }

    pub fn attach(&mut self, token: &TerminalAttachToken) -> Option<TerminalSnapshot> {
        if !self.authenticate(token) { return None; }
        self.attached_displays += 1;
        Some(self.snapshot())
    }

    pub fn detach(&mut self) { self.attached_displays = self.attached_displays.saturating_sub(1); }

    pub fn attached_displays(&self) -> usize { self.attached_displays }

    pub fn resize(&mut self, token: &TerminalAttachToken, dimensions: TerminalDimensions) -> bool {
        if !self.authenticate(token) || dimensions.columns == 0 || dimensions.rows == 0 { return false; }
        self.dimensions = dimensions;
        true
    }

    pub fn append_output(&mut self, bytes: &[u8]) {
        self.bytes.extend(bytes.iter().copied());
        while self.bytes.len() > MAX_TERMINAL_BYTES { self.bytes.pop_front(); }
        self.cursor = self.cursor.saturating_add(bytes.len() as u64);
    }

    pub fn set_running(&mut self) { self.state = TerminalState::Running; }
    pub fn set_prompt_visible(&mut self) { self.state = TerminalState::WaitingForPrompt; }
    pub fn exit(&mut self, code: Option<i32>) { self.state = TerminalState::Exited { code }; }
    pub fn fail(&mut self, message: impl Into<String>) { self.state = TerminalState::Failed { message: message.into() }; }

    pub fn snapshot(&self) -> TerminalSnapshot {
        TerminalSnapshot { session_id: self.session_id.clone(), dimensions: self.dimensions, state: self.state.clone(), attach_cursor: self.cursor, bytes: self.bytes.iter().copied().collect() }
    }

    pub fn provider_success_is_state_neutral(&self, claimed_success: bool, receipt_id: Option<&str>) -> bool {
        !claimed_success || receipt_id.is_some_and(|id| !id.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detach_keeps_session_alive_and_bytes_bounded() {
        let token = TerminalAttachToken::new("token").unwrap();
        let mut session = TerminalSession::start("native-1", 44, token.clone(), ScratchSandbox::strict("/tmp/cutright-scratch"));
        session.set_running();
        assert!(session.attach(&token).is_some());
        session.detach();
        assert_eq!(session.attached_displays(), 0);
        session.append_output(&vec![b'x'; MAX_TERMINAL_BYTES + 3]);
        assert_eq!(session.snapshot().bytes.len(), MAX_TERMINAL_BYTES);
        assert_eq!(session.state, TerminalState::Running);
    }

    #[test]
    fn sandbox_denies_project_and_raw_media_paths() {
        let sandbox = ScratchSandbox::strict("/tmp/cutright-scratch");
        assert!(sandbox.permits_path("/tmp/cutright-scratch/log.txt"));
        assert!(!sandbox.permits_path("/tmp/cutright-scratch/project.json"));
        assert!(!sandbox.permits_path("/tmp/cutright-scratch/source.mov"));
    }

    #[test]
    fn fake_success_requires_structured_receipt() {
        let token = TerminalAttachToken::new("token").unwrap();
        let session = TerminalSession::start("native-1", 44, token, ScratchSandbox::strict("/tmp"));
        assert!(!session.provider_success_is_state_neutral(true, None));
        assert!(session.provider_success_is_state_neutral(true, Some("receipt-1")));
        assert!(session.provider_success_is_state_neutral(false, None));
    }
}
