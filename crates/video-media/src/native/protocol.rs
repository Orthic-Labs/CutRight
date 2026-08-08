use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MAC_MEDIA_PROTOCOL_VERSION: u32 = 1;
pub const MAX_JSONL_LINE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestEnvelope {
    pub protocol_version: u32,
    pub request_id: String,
    pub operation: String,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResponseEnvelope {
    pub protocol_version: u32,
    pub request_id: String,
    pub operation: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<super::MacMediaCapabilities>,
    pub elapsed_nanoseconds: u64,
}

impl ResponseEnvelope {
    pub fn failure(
        request: &RequestEnvelope,
        code: impl Into<String>,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            protocol_version: MAC_MEDIA_PROTOCOL_VERSION,
            request_id: request.request_id.clone(),
            operation: request.operation.clone(),
            ok: false,
            result: None,
            error: Some(ErrorPayload {
                code: code.into(),
                message: message.into(),
                retryable,
            }),
            capabilities: None,
            elapsed_nanoseconds: 0,
        }
    }
}
