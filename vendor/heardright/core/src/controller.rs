//! Pure dictation orchestration for fake/testable engines and pluggable stores.
//!
//! The real Rust runtime lives in `src-tauri/src/runtime.rs`. This controller
//! intentionally depends only on core traits and shapes so checkpoint flow tests
//! can run without linking Tauri or platform GUI imports.

use crate::delivery::{
    CopyFallbackReason, DeliveryBackend, DeliveryError, DeliveryOutcome, DeliveryRecord,
};
use crate::engine::{validate_frame, EnginePayload, FakeEngine};
use crate::history::{HistoryError, HistoryStore, MemoryHistoryStore};
use crate::state::{delivery_outcome_event, transition, AppEvent, AppState, TransitionError};

#[derive(Debug)]
pub enum ControllerError {
    State(TransitionError),
    Engine(String),
    Delivery(DeliveryError),
    History(HistoryError),
    NoLastDelivery,
}

impl std::fmt::Display for ControllerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::State(err) => write!(f, "invalid transition: {:?} + {:?}", err.state, err.event),
            Self::Engine(err) => write!(f, "engine error: {err}"),
            Self::Delivery(err) => write!(f, "delivery error: {err}"),
            Self::History(err) => write!(f, "history error: {err}"),
            Self::NoLastDelivery => write!(f, "no last delivery to repaste"),
        }
    }
}

impl std::error::Error for ControllerError {}

impl From<TransitionError> for ControllerError {
    fn from(value: TransitionError) -> Self {
        Self::State(value)
    }
}

impl From<DeliveryError> for ControllerError {
    fn from(value: DeliveryError) -> Self {
        Self::Delivery(value)
    }
}

impl From<HistoryError> for ControllerError {
    fn from(value: HistoryError) -> Self {
        Self::History(value)
    }
}

#[derive(Debug)]
pub struct DictationController<B: DeliveryBackend, H: HistoryStore> {
    state: AppState,
    engine: FakeEngine,
    delivery: B,
    history: H,
    last_delivery: Option<DeliveryRecord>,
    sequence: u64,
}

impl<B: DeliveryBackend, H: HistoryStore> DictationController<B, H> {
    pub fn new(engine: FakeEngine, delivery: B, history: H) -> Self {
        Self {
            state: AppState::Armed,
            engine,
            delivery,
            history,
            last_delivery: None,
            sequence: 0,
        }
    }

    pub fn app_state(&self) -> AppState {
        self.state.clone()
    }

    pub fn recent_history(&self) -> &[DeliveryRecord] {
        self.history.recent()
    }

    pub fn last_delivery(&self) -> Option<&DeliveryRecord> {
        self.last_delivery.as_ref()
    }

    pub fn start_dictation(&mut self) -> Result<DeliveryRecord, ControllerError> {
        self.start_dictation_with_observer(|_| {})
    }

    pub fn start_dictation_with_observer(
        &mut self,
        mut observer: impl FnMut(&AppState),
    ) -> Result<DeliveryRecord, ControllerError> {
        self.sequence = self.sequence.saturating_add(1);
        let session_id = format!("session-{}", self.sequence);
        let request_id = format!("request-{}", self.sequence);
        let trace_id = format!("trace-{}", self.sequence);

        self.apply(
            AppEvent::StartRecording {
                session_id: session_id.clone(),
            },
            &mut observer,
        )?;
        self.apply(AppEvent::StopRecording, &mut observer)?;

        let frame = self
            .engine
            .transcript_final_frame(&session_id, &request_id, &trace_id);
        validate_frame(&frame).map_err(|err| ControllerError::Engine(err.to_string()))?;
        let transcript = match frame.payload {
            Some(EnginePayload::TranscriptFinal { text, .. }) => text,
            _ => return Err(ControllerError::Engine("missing transcript".to_string())),
        };

        self.apply(
            AppEvent::TranscriptFinal {
                transcript: transcript.clone(),
                send_enter: false,
            },
            &mut observer,
        )?;

        let record = match self.delivery.deliver(&transcript) {
            Ok(record) => record,
            Err(err) => {
                self.apply(
                    AppEvent::Fail {
                        message: err.to_string(),
                    },
                    &mut observer,
                )?;
                return Err(ControllerError::Delivery(err));
            }
        };

        self.apply(delivery_outcome_event(&record), &mut observer)?;

        self.last_delivery = Some(record.clone());
        self.history.push(record.clone())?;
        Ok(record)
    }

    pub fn repaste_last_delivery(&mut self) -> Result<DeliveryRecord, ControllerError> {
        let transcript = self
            .last_delivery
            .as_ref()
            .map(|record| record.transcript.clone())
            .ok_or(ControllerError::NoLastDelivery)?;
        let record = self.delivery.deliver(&transcript)?;
        self.last_delivery = Some(record.clone());
        self.history.push(record.clone())?;
        Ok(record)
    }

    pub fn delete_history_item(&mut self, delivery_id: &str) -> Result<bool, ControllerError> {
        Ok(self.history.delete(delivery_id)?)
    }

    pub fn query_history(
        &self,
        query: &crate::history::HistoryQuery,
    ) -> Result<crate::history::HistoryPage, ControllerError> {
        Ok(self.history.query(query)?)
    }

    pub fn delete_history_matching(
        &mut self,
        filter: &crate::history::HistoryFilter,
    ) -> Result<usize, ControllerError> {
        Ok(self.history.delete_matching(filter)?)
    }

    pub fn prune_history_before(&mut self, cutoff_ms: u64) -> Result<usize, ControllerError> {
        Ok(self.history.prune_before(cutoff_ms)?)
    }

    pub fn update_history_item(
        &mut self,
        delivery_id: &str,
        transcript: String,
    ) -> Result<bool, ControllerError> {
        Ok(self.history.update_transcript(delivery_id, transcript)?)
    }

    pub fn clear_history(&mut self) -> Result<(), ControllerError> {
        self.history.clear()?;
        Ok(())
    }

    pub fn cancel(&mut self) {
        self.state = AppState::Armed;
    }

    fn apply(
        &mut self,
        event: AppEvent,
        observer: &mut impl FnMut(&AppState),
    ) -> Result<(), ControllerError> {
        let result = transition(self.state.clone(), event)?;
        self.state = result.next_state;
        observer(&self.state);
        Ok(())
    }
}

impl<B: DeliveryBackend> DictationController<B, MemoryHistoryStore> {
    pub fn for_test(engine: FakeEngine, delivery: B) -> Self {
        Self::new(engine, delivery, MemoryHistoryStore::default())
    }
}

#[derive(Debug, Clone)]
pub struct FakeDeliveryBackend {
    outcome: DeliveryOutcome,
    counter: u64,
}

impl FakeDeliveryBackend {
    pub fn pasted() -> Self {
        Self {
            outcome: DeliveryOutcome::Pasted,
            counter: 0,
        }
    }

    pub fn copied_fallback(reason: CopyFallbackReason) -> Self {
        Self {
            outcome: DeliveryOutcome::CopiedFallback { reason },
            counter: 0,
        }
    }

    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            outcome: DeliveryOutcome::Error {
                code: code.into(),
                message: message.into(),
            },
            counter: 0,
        }
    }
}

impl DeliveryBackend for FakeDeliveryBackend {
    fn deliver(&mut self, transcript: &str) -> Result<DeliveryRecord, DeliveryError> {
        self.counter = self.counter.saturating_add(1);
        Ok(DeliveryRecord::for_test(
            crate::delivery::DeliveryId::new(format!("delivery-{}", self.counter)),
            transcript,
            self.outcome.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delivery::CopyFallbackReason;

    #[test]
    fn fake_transcript_reaches_paste_history_and_last_delivery() {
        let engine = FakeEngine::fixed_transcript("hello checkpoint");
        let delivery = FakeDeliveryBackend::pasted();
        let mut controller = DictationController::for_test(engine, delivery);

        let record = controller.start_dictation().unwrap();

        assert_eq!(record.transcript, "hello checkpoint");
        assert!(matches!(record.outcome, DeliveryOutcome::Pasted));
        assert!(matches!(controller.app_state(), AppState::Pasted { .. }));
        assert_eq!(controller.recent_history().len(), 1);
        assert_eq!(
            controller.last_delivery().map(|r| r.delivery_id.as_str()),
            Some(record.delivery_id.as_str())
        );
    }

    #[test]
    fn fallback_records_explicit_reason_and_keeps_repaste_text() {
        let engine = FakeEngine::fixed_transcript("copy me");
        let delivery = FakeDeliveryBackend::copied_fallback(CopyFallbackReason::PasteFailed);
        let mut controller = DictationController::for_test(engine, delivery);

        let first = controller.start_dictation().unwrap();
        assert!(matches!(
            first.outcome,
            DeliveryOutcome::CopiedFallback {
                reason: CopyFallbackReason::PasteFailed
            }
        ));
        assert!(matches!(
            controller.app_state(),
            AppState::CopiedFallback { .. }
        ));

        let second = controller.repaste_last_delivery().unwrap();
        assert_eq!(second.transcript, "copy me");
        assert_eq!(controller.recent_history().len(), 2);
    }

    #[test]
    fn delivery_error_moves_state_to_error_and_keeps_last_transcript() {
        let engine = FakeEngine::fixed_transcript("will fail");
        let delivery = FakeDeliveryBackend::error("E_TEST_DELIVERY", "delivery failed");
        let mut controller = DictationController::for_test(engine, delivery);

        let record = controller.start_dictation().unwrap();
        assert!(matches!(record.outcome, DeliveryOutcome::Error { .. }));
        match controller.app_state() {
            AppState::Error {
                message,
                last_transcript,
            } => {
                assert_eq!(message, "delivery failed");
                assert_eq!(last_transcript.as_deref(), Some("will fail"));
            }
            other => panic!("expected Error state, got {other:?}"),
        }
    }
}
