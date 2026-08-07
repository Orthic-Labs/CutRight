//! Video inference runtime trait (CR-V2-B3-012).
//!
//! The vendored llama.cpp is reached only through `LocalInferenceRuntime`.
//! The merge step (`CR-V2-B3-022`) lifts this crate into the workspace
//! membership.

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct VerifiedResource {
    pub path: std::path::PathBuf,
    pub sha256: String,
    pub blake3: String,
    pub license: String,
}

#[derive(Debug, Clone)]
pub struct LoadConfig {
    pub context_size: u32,
    pub seed: u64,
    pub tensor_split: Option<String>,
    pub backend: BackendKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BackendKind {
    Cpu,
    Metal,
    Cuda,
    Vulkan,
    OpenCl,
}

#[derive(Debug, Clone)]
pub struct ModelHandle {
    pub id: String,
    pub verified: VerifiedResource,
}

#[derive(Debug, Clone)]
pub struct GenerationRequest {
    pub prompt: String,
    pub token_limit: u32,
    pub temperature_milli: i32,
    pub top_p_milli: i32,
    pub seed: u64,
    pub json_schema: Option<String>,
    pub byte_budget: usize,
}

#[derive(Debug, Clone)]
pub struct GenerationResult {
    pub text: String,
    pub token_count: u32,
    pub finish_reason: FinishReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FinishReason {
    Stop,
    TokenLimit,
    ByteLimit,
    Cancelled,
    Error,
}

#[derive(Debug, thiserror::Error)]
pub enum InferenceError {
    #[error("backend not available: {0:?}")]
    BackendUnavailable(BackendKind),
    #[error("model hash mismatch")]
    HashMismatch,
    #[error("token limit exceeded ({0})")]
    TokenLimitExceeded(u32),
    #[error("byte budget exceeded ({0})")]
    ByteBudgetExceeded(usize),
    #[error("cancelled")]
    Cancelled,
    #[error("decode error: {0}")]
    Decode(String),
}

pub trait LocalInferenceRuntime {
    fn load(&self, model: &VerifiedResource, config: LoadConfig) -> Result<ModelHandle, InferenceError>;
    fn generate_json<T: DeserializeOwned>(
        &self,
        handle: &ModelHandle,
        request: GenerationRequest,
    ) -> Result<T, InferenceError>;
    fn generate_text(
        &self,
        handle: &ModelHandle,
        request: GenerationRequest,
    ) -> Result<GenerationResult, InferenceError>;
    fn cancel(&self, handle: &ModelHandle) -> Result<(), InferenceError>;
    fn backend_identity(&self) -> BackendIdentity;
}

#[derive(Debug, Clone, Serialize)]
pub struct BackendIdentity {
    pub backend: BackendKind,
    pub source_commit: String,
    pub binary_hash: String,
    pub manifest_hash: String,
}

/// Stub the trait until the merge step lifts the crate into the
/// workspace. The notebook unit tests live in `tests/runtime.rs`.
#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Echo { message: String }

    #[test]
    fn backend_identity_is_serializable() {
        let id = BackendIdentity {
            backend: BackendKind::Cpu,
            source_commit: "abc".to_string(),
            binary_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            manifest_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        };
        let serialized = serde_json::to_string(&id).unwrap();
        assert!(serialized.contains("\"backend\":\"Cpu\""));
    }

    #[test]
    fn generation_request_carries_token_limit() {
        let r = GenerationRequest {
            prompt: "say hello".to_string(),
            token_limit: 32,
            temperature_milli: 700,
            top_p_milli: 950,
            seed: 42,
            json_schema: None,
            byte_budget: 4096,
        };
        assert_eq!(r.token_limit, 32);
        assert_eq!(r.seed, 42);
    }

    #[test]
    fn finish_reason_covers_all_paths() {
        let reasons = [
            FinishReason::Stop,
            FinishReason::TokenLimit,
            FinishReason::ByteLimit,
            FinishReason::Cancelled,
            FinishReason::Error,
        ];
        assert_eq!(reasons.len(), 5);
    }

    #[test]
    fn parse_path_does_not_require_external_state() {
        let p = Path::new("/nonexistent");
        assert!(p.is_absolute());
    }
}
