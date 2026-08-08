use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeRationalTime {
    pub numerator: i64,
    pub denominator: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NodeReceipt {
    pub node_id: String,
    pub status: String,
    #[serde(default)]
    pub frame_hash: Option<String>,
    #[serde(default)]
    pub audio_peak_offset_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeRenderReceipt {
    pub schema_version: u32,
    pub locked_cut_sha256: String,
    pub artifact_sha256: String,
    pub duration: NativeRationalTime,
    pub rendered_frames: u64,
    pub audio_frames: u64,
    pub nodes: Vec<NodeReceipt>,
}

impl NativeRenderReceipt {
    pub fn validate(&self) -> bool {
        self.schema_version == 1
            && !self.locked_cut_sha256.is_empty()
            && !self.artifact_sha256.is_empty()
            && self.duration.denominator > 0
    }
}
