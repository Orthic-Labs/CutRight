use crate::models::{Transcript, VadSignal};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider {provider} is unavailable: {reason}")]
    Unavailable { provider: String, reason: String },
    #[error("provider {provider} rejected the request: {reason}")]
    Rejected { provider: String, reason: String },
}

pub struct TranscriptionRequest {
    pub source_path: std::path::PathBuf,
    pub language_hint: Option<String>,
}

pub trait TranscriptionProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn transcribe(&self, request: &TranscriptionRequest) -> Result<Transcript, ProviderError>;
}

pub struct VadRequest {
    pub audio_path: std::path::PathBuf,
    pub sample_rate: u32,
}

pub trait VadProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn analyze(&self, request: &VadRequest) -> Result<VadSignal, ProviderError>;
}
