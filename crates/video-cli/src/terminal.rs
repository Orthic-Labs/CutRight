//! `videoctl agent terminal attach` client contract.
//!
//! This client joins a daemon-owned terminal. It does not infer state from
//! ANSI bytes and never starts a second provider for an existing session.

use std::collections::VecDeque;

pub const ATTACH_COMMAND: &[&str] = &["agent", "terminal", "attach"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachRequest {
    pub session_id: String,
    pub attach_token: String,
    pub columns: u16,
    pub rows: u16,
}

impl AttachRequest {
    pub fn argv(&self) -> Vec<String> {
        vec!["agent".into(), "terminal".into(), "attach".into(), self.session_id.clone()]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalEvent {
    Bytes(Vec<u8>),
    PromptVisible,
    Exit(Option<i32>),
    Error(String),
}

#[derive(Debug, Default)]
pub struct TerminalView {
    bytes: VecDeque<u8>,
    pub prompt_visible: bool,
    pub exit_code: Option<Option<i32>>,
}

impl TerminalView {
    pub fn apply(&mut self, event: TerminalEvent) {
        match event {
            TerminalEvent::Bytes(bytes) => self.bytes.extend(bytes),
            TerminalEvent::PromptVisible => self.prompt_visible = true,
            TerminalEvent::Exit(code) => self.exit_code = Some(code),
            TerminalEvent::Error(message) => self.exit_code = Some(Some(if message.is_empty() { 1 } else { 2 })),
        }
    }

    pub fn presentation_bytes(&self) -> Vec<u8> { self.bytes.iter().copied().collect() }
}

pub fn parse_terminal_event(bytes: &[u8]) -> TerminalEvent { TerminalEvent::Bytes(bytes.to_vec()) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attach_command_joins_existing_session() {
        let request = AttachRequest { session_id: "native-1".into(), attach_token: "secret".into(), columns: 120, rows: 40 };
        assert_eq!(request.argv(), vec!["agent", "terminal", "attach", "native-1"]);
    }

    #[test]
    fn terminal_bytes_are_presentation_only() {
        let mut view = TerminalView::default();
        view.apply(parse_terminal_event(b"success"));
        assert_eq!(view.presentation_bytes(), b"success");
        assert!(view.exit_code.is_none());
    }
}
