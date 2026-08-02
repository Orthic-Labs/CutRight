//! VAD provenance (hardening plan §10.7).
//!
//! `video_core::VadSignal` (the trait return type consumed by
//! `video-project`) is defined upstream and intentionally stays a thin,
//! stable artifact shape. [`VadProvenance`] is an additive, crate-local
//! record carrying everything §10.7 asks for; it is returned alongside the
//! signal from [`crate::HeardRightProvider::analyze_file_vad_with_provenance`]
//! for callers that want it, without changing the `VadProvider` trait
//! signature.

use serde_json::Value;
use video_core::VadSignal;

use crate::heardright::ProviderError;

/// Everything the plan asks for a VAD result's provenance to record.
#[derive(Debug, Clone, serde::Serialize)]
pub struct VadProvenance {
    /// BLAKE3 of the decoded audio file bytes CutRight sent to HeardRight.
    pub decoded_audio_blake3: String,
    pub model_revision: String,
    pub runtime_backend: String,
    pub threshold: f32,
    pub min_speech_ms: u32,
    pub min_silence_ms: u32,
    pub sample_rate: u32,
    /// How CutRight decoded the source before handing it to HeardRight (this
    /// crate always requests 16 kHz mono f32 PCM for file-VAD today).
    pub decode_policy: String,
    /// BLAKE3 of the outgoing request JSON, so a stored provenance record can
    /// be checked against a re-issued request.
    pub request_blake3: String,
    pub warnings: Vec<String>,
}

/// Parse a HeardRight `file_vad_result` payload into CutRight's
/// [`VadSignal`], validating that regions are non-inverted.
pub(crate) fn parse_vad_result(
    source_id: &str,
    payload: &Value,
) -> Result<VadSignal, ProviderError> {
    let sample_rate = payload
        .get("sample_rate")
        .and_then(Value::as_u64)
        .ok_or_else(|| ProviderError::Engine("file_vad_result has no sample_rate".into()))?
        as u32;
    let raw_regions = payload
        .get("regions")
        .and_then(Value::as_array)
        .ok_or_else(|| ProviderError::Engine("file_vad_result has no regions".into()))?;
    let mut regions = Vec::with_capacity(raw_regions.len());
    for (index, region) in raw_regions.iter().enumerate() {
        let start_ms = region
            .get("start_ms")
            .and_then(Value::as_i64)
            .ok_or_else(|| ProviderError::Engine(format!("vad region {index} has no start_ms")))?;
        let end_ms = region
            .get("end_ms")
            .and_then(Value::as_i64)
            .ok_or_else(|| ProviderError::Engine(format!("vad region {index} has no end_ms")))?;
        let mean_probability = region
            .get("mean_probability")
            .and_then(Value::as_f64)
            .ok_or_else(|| {
                ProviderError::Engine(format!("vad region {index} has no mean_probability"))
            })? as f32;
        if end_ms < start_ms {
            return Err(ProviderError::Engine(format!(
                "vad region {index} ends before it starts"
            )));
        }
        regions.push(video_core::VadRegion {
            start_ms,
            end_ms,
            mean_probability,
        });
    }
    Ok(VadSignal {
        schema_version: video_core::SCHEMA_VERSION,
        source_id: source_id.to_string(),
        sample_rate,
        provider: "heardright-silero".into(),
        regions,
    })
}

/// Build the provenance record for one `file_vad_result` (§10.7). `request`
/// is the outgoing request frame that was sent; `payload` is the engine's
/// result payload; `decoded_audio_bytes` is the audio CutRight sent.
pub(crate) fn build_provenance(
    request: &Value,
    payload: &Value,
    decoded_audio_bytes: &[u8],
    threshold: f32,
    sample_rate: u32,
) -> VadProvenance {
    let warnings = payload
        .get("warnings")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    VadProvenance {
        decoded_audio_blake3: blake3::hash(decoded_audio_bytes).to_hex().to_string(),
        model_revision: payload
            .get("model_revision")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        runtime_backend: payload
            .get("runtime")
            .or_else(|| payload.get("backend"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        threshold,
        min_speech_ms: payload
            .get("min_speech_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        min_silence_ms: payload
            .get("min_silence_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        sample_rate,
        decode_policy: "pcm_f32le_mono".to_string(),
        request_blake3: blake3::hash(request.to_string().as_bytes())
            .to_hex()
            .to_string(),
        warnings,
    }
}
