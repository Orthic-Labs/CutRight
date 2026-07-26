use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ModelError {
    #[error("schema_version must be {SCHEMA_VERSION}")]
    UnsupportedSchema,
    #[error("source {0} has an invalid time range")]
    InvalidTimeRange(String),
    #[error("timeline segment {0} refers to a missing source")]
    MissingSource(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectManifest {
    pub schema_version: u32,
    pub project_id: String,
    pub kind: String,
    pub created_at: DateTime<Utc>,
    pub review_mode: ReviewMode,
    pub source_policy: SourcePolicy,
    pub outputs: Vec<OutputPreset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewMode {
    Reviewed,
    ReviewLight,
    Autonomous,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SourcePolicy {
    Immutable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutputPreset {
    pub id: String,
    pub aspect: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceManifest {
    pub schema_version: u32,
    pub sources: Vec<SourceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceEntry {
    pub source_id: String,
    pub path: String,
    pub blake3: String,
    pub duration_ms: Option<i64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub rotation_degrees: Option<i32>,
    pub is_hdr: Option<bool>,
    pub timebase: Option<Timebase>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Timebase {
    pub fps_num: u32,
    pub fps_den: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transcript {
    pub schema_version: u32,
    pub provider: String,
    pub source_id: String,
    pub language: String,
    pub words: Vec<Word>,
    #[serde(default)]
    pub events: Vec<TranscriptEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateManifest {
    pub schema_version: u32,
    pub candidates: Vec<Candidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Candidate {
    pub id: String,
    pub source_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub beat_label: String,
    pub take_rank: u32,
    pub drop_reason: Option<DropReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DropReason {
    FalseStart,
    Duplicate,
    Meta,
    Filler,
    Tangent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CutPlan {
    pub schema_version: u32,
    pub variant: String,
    pub gap_threshold_ms: i64,
    pub head_margin_ms: i64,
    pub tail_margin_ms: i64,
    pub segments: Vec<CutSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CutSegment {
    pub id: String,
    pub source_id: String,
    pub source_start_ms: i64,
    pub source_end_ms: i64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Word {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_word_id: Option<String>,
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub confidence: f32,
    pub speaker: Option<String>,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranscriptEvent {
    pub event_type: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VadSignal {
    pub schema_version: u32,
    pub source_id: String,
    pub sample_rate: u32,
    pub provider: String,
    pub regions: Vec<VadRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VadRegion {
    pub start_ms: i64,
    pub end_ms: i64,
    pub mean_probability: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Timeline {
    pub schema_version: u32,
    pub timebase: Timebase,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Track {
    pub id: String,
    #[serde(rename = "type")]
    pub track_type: String,
    pub segments: Vec<TimelineSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineSegment {
    pub id: String,
    pub source_id: String,
    pub source_start_ms: i64,
    pub source_end_ms: i64,
    pub output_start_ms: i64,
    pub output_end_ms: i64,
    pub speed: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FinishPlan {
    pub schema_version: u32,
    pub base_timeline: String,
    pub slots: Vec<FinishSlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FinishSlot {
    pub id: String,
    pub kind: String,
    pub renderer: String,
    pub effect_id: String,
    pub output_start_ms: i64,
    pub output_end_ms: i64,
    pub anchor: Option<String>,
    pub collision_policy: Option<String>,
    #[serde(default)]
    pub props: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderResponseEnvelope {
    pub provider: String,
    pub provider_model: String,
    pub request_hash: String,
    pub created_at: DateTime<Utc>,
    pub cost: ProviderCost,
    pub raw_response_path: String,
    pub normalised_output_path: String,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderCost {
    pub currency: String,
    pub estimated: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_transcript_round_trips() {
        let transcript = Transcript {
            schema_version: SCHEMA_VERSION,
            provider: "fixture".into(),
            source_id: "cam-a-001".into(),
            language: "en".into(),
            words: vec![Word {
                id: "w_000001".into(),
                source_word_id: Some("cam-a-001:w_000001".into()),
                text: "Today".into(),
                start_ms: 812,
                end_ms: 1_084,
                confidence: 0.98,
                speaker: Some("S0".into()),
                kind: "word".into(),
            }],
            events: Vec::new(),
        };
        let value = serde_json::to_value(&transcript).unwrap();
        let decoded: Transcript = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, transcript);
    }

    #[test]
    fn legacy_word_without_source_word_id_deserializes() {
        let word: Word = serde_json::from_value(serde_json::json!({
            "id": "w_000001", "text": "Today", "start_ms": 0, "end_ms": 100,
            "confidence": 1.0, "speaker": null, "kind": "word"
        }))
        .unwrap();
        assert_eq!(word.source_word_id, None);
    }

    #[test]
    fn all_phase_zero_artifacts_round_trip() {
        let vad = VadSignal {
            schema_version: SCHEMA_VERSION,
            source_id: "cam-a-001".into(),
            sample_rate: 16_000,
            provider: "silero-onnx".into(),
            regions: vec![VadRegion {
                start_ms: 780,
                end_ms: 4_290,
                mean_probability: 0.93,
            }],
        };
        let timeline = Timeline {
            schema_version: SCHEMA_VERSION,
            timebase: Timebase {
                fps_num: 30_000,
                fps_den: 1_001,
            },
            tracks: vec![Track {
                id: "video-main".into(),
                track_type: "video".into(),
                segments: vec![TimelineSegment {
                    id: "seg-001".into(),
                    source_id: "cam-a-001".into(),
                    source_start_ms: 812,
                    source_end_ms: 6_730,
                    output_start_ms: 0,
                    output_end_ms: 5_918,
                    speed: 1.0,
                    reason: "strongest hook take".into(),
                }],
            }],
        };
        let finish = FinishPlan {
            schema_version: SCHEMA_VERSION,
            base_timeline: "edit/timeline.json".into(),
            slots: vec![FinishSlot {
                id: "slot-001".into(),
                kind: "caption".into(),
                renderer: "remotion".into(),
                effect_id: "caption.bold-karaoke.v1".into(),
                output_start_ms: 0,
                output_end_ms: 5_918,
                anchor: Some("bottom-center".into()),
                collision_policy: Some("avoid-subject-and-platform-ui".into()),
                props: serde_json::Map::new(),
            }],
        };
        let envelope = ProviderResponseEnvelope {
            provider: "fixture".into(),
            provider_model: "fixture-v1".into(),
            request_hash: "blake3:fixture".into(),
            created_at: Utc::now(),
            cost: ProviderCost {
                currency: "USD".into(),
                estimated: Some(0.0),
            },
            raw_response_path: "cache/provider-responses/fixture.json".into(),
            normalised_output_path: "analysis/cloud-analysis/fixture.json".into(),
            warnings: vec![],
        };
        let vad_json = serde_json::to_value(&vad).unwrap();
        let timeline_json = serde_json::to_value(&timeline).unwrap();
        let finish_json = serde_json::to_value(&finish).unwrap();
        let envelope_json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(serde_json::from_value::<VadSignal>(vad_json).unwrap(), vad);
        assert_eq!(
            serde_json::from_value::<Timeline>(timeline_json).unwrap(),
            timeline
        );
        assert_eq!(
            serde_json::from_value::<FinishPlan>(finish_json).unwrap(),
            finish
        );
        assert_eq!(
            serde_json::from_value::<ProviderResponseEnvelope>(envelope_json).unwrap(),
            envelope
        );
    }

    #[test]
    fn edit_artifacts_round_trip() {
        let candidates = CandidateManifest {
            schema_version: SCHEMA_VERSION,
            candidates: vec![Candidate {
                id: "candidate-001".into(),
                source_id: "cam-a-001".into(),
                start_ms: 812,
                end_ms: 6_730,
                text: "Today we build this.".into(),
                beat_label: "hook".into(),
                take_rank: 1,
                drop_reason: None,
            }],
        };
        let cut_plan = CutPlan {
            schema_version: SCHEMA_VERSION,
            variant: "natural".into(),
            gap_threshold_ms: 400,
            head_margin_ms: 180,
            tail_margin_ms: 220,
            segments: vec![CutSegment {
                id: "segment-001".into(),
                source_id: "cam-a-001".into(),
                source_start_ms: 632,
                source_end_ms: 6_950,
                reason: "hook".into(),
            }],
        };
        let candidates_json = serde_json::to_value(&candidates).unwrap();
        let cut_plan_json = serde_json::to_value(&cut_plan).unwrap();
        assert_eq!(
            serde_json::from_value::<CandidateManifest>(candidates_json).unwrap(),
            candidates
        );
        assert_eq!(
            serde_json::from_value::<CutPlan>(cut_plan_json).unwrap(),
            cut_plan
        );
    }
}
