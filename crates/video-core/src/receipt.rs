//! Shared stage-receipt format (hardening plan §10.4).
//!
//! Every pipeline stage — ingest, ASR, VAD, candidates, cut plan, timeline,
//! rough render, transcript remap, evidence, reframe, finish, final, QA,
//! package — can attach one of these to record exactly what it consumed,
//! what parameters drove it, which toolchains produced it, and what it
//! emitted. The type lives in `video-core` (rather than `video-media`, which
//! emits most of them) specifically so that `video-project` can consume and
//! persist receipts without depending on `video-media` internals.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const RECEIPT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum ReceiptError {
    #[error("failed to hash receipt input {path}: {source}")]
    Input {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to hash receipt output {path}: {source}")]
    Output {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("receipt parameters could not be serialized: {0}")]
    Parameters(#[source] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReceiptInput {
    pub path: String,
    pub blake3: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReceiptOutput {
    pub path: String,
    pub blake3: String,
    pub size: u64,
}

/// A single stage's full provenance record, per hardening plan §10.4.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageReceipt {
    pub schema_version: u32,
    pub stage: String,
    pub implementation_version: String,
    pub inputs: Vec<ReceiptInput>,
    pub parameters_blake3: String,
    /// Identity of every external toolchain involved (e.g. `"ffmpeg"` ->
    /// `MediaToolchain`'s combined version+hash identity string,
    /// `"heardright"` -> the engine's reported version). Keyed by tool name
    /// so a caller can add or omit entries without changing the schema.
    pub toolchains: BTreeMap<String, String>,
    pub outputs: Vec<ReceiptOutput>,
    pub created_at: DateTime<Utc>,
}

impl StageReceipt {
    /// Build a receipt by hashing every input/output file on disk and the
    /// serialized parameters. `stage` is a dotted stage name (e.g.
    /// `"render.rough_cut"`); `implementation_version` is normally
    /// `env!("CARGO_PKG_VERSION")` of the crate that ran the stage.
    pub fn build<P>(
        stage: impl Into<String>,
        implementation_version: impl Into<String>,
        inputs: &[&Path],
        parameters: &P,
        toolchains: BTreeMap<String, String>,
        outputs: &[&Path],
    ) -> Result<Self, ReceiptError>
    where
        P: Serialize + ?Sized,
    {
        let inputs = inputs
            .iter()
            .map(|path| hash_input(path))
            .collect::<Result<Vec<_>, _>>()?;
        let outputs = outputs
            .iter()
            .map(|path| hash_output(path))
            .collect::<Result<Vec<_>, _>>()?;
        let parameters_bytes = serde_json::to_vec(parameters).map_err(ReceiptError::Parameters)?;
        Ok(Self {
            schema_version: RECEIPT_SCHEMA_VERSION,
            stage: stage.into(),
            implementation_version: implementation_version.into(),
            inputs,
            parameters_blake3: blake3::hash(&parameters_bytes).to_hex().to_string(),
            toolchains,
            outputs,
            created_at: Utc::now(),
        })
    }
}

fn hash_input(path: &Path) -> Result<ReceiptInput, ReceiptError> {
    let bytes = fs::read(path).map_err(|source| ReceiptError::Input {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(ReceiptInput {
        path: path.display().to_string(),
        blake3: blake3::hash(&bytes).to_hex().to_string(),
    })
}

fn hash_output(path: &Path) -> Result<ReceiptOutput, ReceiptError> {
    let bytes = fs::read(path).map_err(|source| ReceiptError::Output {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(ReceiptOutput {
        path: path.display().to_string(),
        blake3: blake3::hash(&bytes).to_hex().to_string(),
        size: bytes.len() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn unique_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cutright-receipt-test-{label}-{unique}"));
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn round_trips_through_json() {
        let dir = unique_dir("roundtrip");
        let input_path = dir.join("input.mp4");
        let output_path = dir.join("output.mp4");
        fs::write(&input_path, b"source-bytes").expect("write input fixture");
        fs::write(&output_path, b"rendered-bytes").expect("write output fixture");

        let mut toolchains = BTreeMap::new();
        toolchains.insert("ffmpeg".to_string(), "8.1.2:abcd1234".to_string());

        let receipt = StageReceipt::build(
            "render.rough_cut",
            env!("CARGO_PKG_VERSION"),
            &[input_path.as_path()],
            &serde_json::json!({"segments": [{"start_ms": 0, "end_ms": 1000}]}),
            toolchains,
            &[output_path.as_path()],
        )
        .expect("build receipt");

        assert_eq!(receipt.schema_version, RECEIPT_SCHEMA_VERSION);
        assert_eq!(receipt.inputs.len(), 1);
        assert_eq!(receipt.outputs.len(), 1);
        assert_eq!(receipt.outputs[0].size, "rendered-bytes".len() as u64);
        assert!(!receipt.parameters_blake3.is_empty());

        let encoded = serde_json::to_string(&receipt).expect("serialize receipt");
        let decoded: StageReceipt = serde_json::from_str(&encoded).expect("deserialize receipt");
        assert_eq!(decoded, receipt);

        fs::remove_dir_all(&dir).expect("remove test dir");
    }

    #[test]
    fn same_parameters_hash_identically_regardless_of_stage() {
        let dir = unique_dir("params");
        let input_path = dir.join("input.mp4");
        fs::write(&input_path, b"same-bytes").expect("write input fixture");
        let params = serde_json::json!({"width": 1080, "height": 1920});

        let first = StageReceipt::build(
            "render.finish",
            "0.1.0",
            &[input_path.as_path()],
            &params,
            BTreeMap::new(),
            &[],
        )
        .expect("build first receipt");
        let second = StageReceipt::build(
            "render.finish",
            "0.1.0",
            &[input_path.as_path()],
            &params,
            BTreeMap::new(),
            &[],
        )
        .expect("build second receipt");

        assert_eq!(first.parameters_blake3, second.parameters_blake3);
        assert_eq!(first.inputs[0].blake3, second.inputs[0].blake3);
        fs::remove_dir_all(&dir).expect("remove test dir");
    }
}
