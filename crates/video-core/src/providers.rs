use crate::models::{Transcript, VadSignal};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider {provider} is unavailable: {reason}")]
    Unavailable { provider: String, reason: String },
    #[error("provider {provider} rejected the request: {reason}")]
    Rejected { provider: String, reason: String },
}

pub struct TranscriptionRequest {
    pub source_id: String,
    pub source_path: std::path::PathBuf,
    pub language_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TranscriptionOutput {
    pub transcript: Transcript,
    pub raw_response: Value,
    pub provider_model: String,
    pub warnings: Vec<String>,
}

pub trait TranscriptionProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn model_id(&self) -> &'static str;
    fn transcribe(
        &self,
        request: &TranscriptionRequest,
    ) -> Result<TranscriptionOutput, ProviderError>;
}

pub struct VadRequest {
    pub source_id: String,
    pub audio_path: std::path::PathBuf,
    pub sample_rate: u32,
    pub threshold: f32,
}

pub trait VadProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn analyze(&self, request: &VadRequest) -> Result<VadSignal, ProviderError>;
}
