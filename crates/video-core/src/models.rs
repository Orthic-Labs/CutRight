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
    #[error("word {0} has an invalid time range (end_ms must be greater than start_ms)")]
    InvalidWordTimeRange(String),
    #[error("word timeline is not sorted/non-overlapping at word {0}")]
    UnsortedWordTimeline(String),
    #[error("duplicate word id {0}")]
    DuplicateWordId(String),
    #[error("source_word_id {0} does not match the compound <source_id>:<word_id> pattern")]
    InvalidSourceWordId(String),
    #[error("duplicate source_word_id {0}")]
    DuplicateSourceWordId(String),
    #[error("timebase {0}/{1} is not rational (fps_num and fps_den must both be nonzero)")]
    InvalidTimebase(u32, u32),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectManifest {
    pub schema_version: u32,
    pub project_id: String,
    /// Immutable per-project identity (§12.7). `project_id` used to be derived
    /// from the folder name, so two projects called "reel" collided; decision
    /// and receipt identity therefore binds to this instead. Empty on a
    /// pre-migration manifest — `migrate_project` fills it in and nothing ever
    /// regenerates it, including on folder rename or source relink.
    #[serde(default)]
    pub project_instance_id: String,
    /// The human-facing name. Renaming the folder changes this and nothing else.
    #[serde(default)]
    pub title: String,
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

impl Timebase {
    /// A timebase is only meaningful as a rational number: both the
    /// numerator and denominator must be nonzero.
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.fps_num == 0 || self.fps_den == 0 {
            return Err(ModelError::InvalidTimebase(self.fps_num, self.fps_den));
        }
        Ok(())
    }
}

/// Canonical word-level transcript artifact. `deny_unknown_fields` enforces
/// the strict unknown-field handling required for canonical artifacts under
/// a fixed schema version (REV2 plan §8.4): any field this build does not
/// know about is a contract violation, not silently ignored data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Transcript {
    pub schema_version: u32,
    pub provider: String,
    pub source_id: String,
    pub language: String,
    pub words: Vec<Word>,
    #[serde(default)]
    pub events: Vec<TranscriptEvent>,
}

impl Transcript {
    /// Semantic validation beyond JSON Schema (REV2 plan §8.5): every word
    /// has a sane time range, the word timeline is sorted and
    /// non-overlapping, word ids are unique, and any `source_word_id` is
    /// both pattern-valid and unique within this transcript. Because
    /// `source_word_id` is namespaced by `source_id` (see
    /// [`Word::validate`]), per-transcript uniqueness combined with the
    /// pattern check is sufficient to keep the compound id globally unique
    /// across every transcript emitted for a project.
    pub fn validate(&self) -> Result<(), ModelError> {
        let mut seen_word_ids = std::collections::HashSet::new();
        let mut seen_source_word_ids = std::collections::HashSet::new();
        let mut previous_end_ms: Option<i64> = None;
        for word in &self.words {
            word.validate()?;
            if !seen_word_ids.insert(word.id.as_str()) {
                return Err(ModelError::DuplicateWordId(word.id.clone()));
            }
            if let Some(source_word_id) = &word.source_word_id {
                if !seen_source_word_ids.insert(source_word_id.as_str()) {
                    return Err(ModelError::DuplicateSourceWordId(source_word_id.clone()));
                }
            }
            if let Some(previous_end_ms) = previous_end_ms {
                if word.start_ms < previous_end_ms {
                    return Err(ModelError::UnsortedWordTimeline(word.id.clone()));
                }
            }
            previous_end_ms = Some(word.end_ms);
        }
        Ok(())
    }
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
    /// Number of filler words/disfluencies detected in this candidate. Recorded
    /// for the cut plan and human review; only acted on by
    /// [`FillerPolicy::Automatic`].
    #[serde(default)]
    pub filler_count: usize,
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

/// How aggressively candidate generation treats detected filler words and
/// false starts. The default records suggestions without dropping anything, so
/// a human still approves removals.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FillerPolicy {
    /// Keep all fillers; do not even record suggestions.
    Preserve,
    /// Record filler/false-start suggestions on candidates but drop nothing.
    #[default]
    SuggestOnly,
    /// Drop candidates that are pure filler or repeated false starts.
    Automatic,
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
#[serde(deny_unknown_fields)]
pub struct Word {
    pub id: String,
    /// Optional compound id binding this word back to its source transcript:
    /// `"<source_id>:<word_id>"`. Namespacing by `source_id` is what keeps
    /// the id globally unique across every transcript in a project, not
    /// just within one. See [`is_valid_source_word_id`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_word_id: Option<String>,
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub confidence: f32,
    pub speaker: Option<String>,
    pub kind: String,
}

/// Compound id pattern for [`Word::source_word_id`]: a nonempty source id, a
/// literal `:` separator, and a nonempty word id. Mirrors the `pattern`
/// constraint in `schemas/transcript.schema.json`.
pub fn is_valid_source_word_id(value: &str) -> bool {
    match value.split_once(':') {
        Some((source_id, word_id)) => !source_id.is_empty() && !word_id.is_empty(),
        None => false,
    }
}

impl Word {
    /// Semantic validation beyond JSON Schema (REV2 plan §8.5): `end_ms`
    /// must be strictly after `start_ms`, and any `source_word_id` must
    /// match the compound id pattern.
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.start_ms < 0 || self.end_ms <= self.start_ms {
            return Err(ModelError::InvalidWordTimeRange(self.id.clone()));
        }
        if let Some(source_word_id) = &self.source_word_id {
            if !is_valid_source_word_id(source_word_id) {
                return Err(ModelError::InvalidSourceWordId(source_word_id.clone()));
            }
        }
        Ok(())
    }
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
                filler_count: 0,
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

    // --- transcript.schema.json v1 fixture guard (REV2 plan §8.4/§8.5) ---
    // Fixtures documented in fixtures/schemas/transcript/v1/README.md.

    const VALID_BASIC: &str =
        include_str!("../../../fixtures/schemas/transcript/v1/valid/basic.json");
    const INVALID_MISSING_REQUIRED_FIELD: &str =
        include_str!("../../../fixtures/schemas/transcript/v1/invalid/missing_required_field.json");
    const INVALID_UNKNOWN_FIELD: &str =
        include_str!("../../../fixtures/schemas/transcript/v1/invalid/unknown_field.json");
    const INVALID_BAD_SOURCE_WORD_ID_PATTERN: &str = include_str!(
        "../../../fixtures/schemas/transcript/v1/invalid/bad_source_word_id_pattern.json"
    );
    const INVALID_UNSORTED_WORD_TIMELINE: &str =
        include_str!("../../../fixtures/schemas/transcript/v1/invalid/unsorted_word_timeline.json");

    #[test]
    fn transcript_schema_v1_valid_fixture_round_trips_and_validates() {
        let transcript: Transcript = serde_json::from_str(VALID_BASIC)
            .expect("valid/basic.json must deserialize as a Transcript");
        transcript
            .validate()
            .expect("valid/basic.json must pass semantic validation");
        let reserialized = serde_json::to_value(&transcript).unwrap();
        let round_tripped: Transcript = serde_json::from_value(reserialized).unwrap();
        assert_eq!(round_tripped, transcript);
        assert_eq!(
            transcript.words[0].source_word_id.as_deref(),
            Some("cam-a-001:w_000001")
        );
        assert_eq!(transcript.words[1].source_word_id, None);
    }

    #[test]
    fn transcript_schema_v1_invalid_fixtures_are_rejected() {
        serde_json::from_str::<Transcript>(INVALID_MISSING_REQUIRED_FIELD)
            .expect_err("missing_required_field.json must fail to deserialize");
        serde_json::from_str::<Transcript>(INVALID_UNKNOWN_FIELD)
            .expect_err("unknown_field.json must fail deny_unknown_fields deserialization");

        let bad_source_word_id: Transcript =
            serde_json::from_str(INVALID_BAD_SOURCE_WORD_ID_PATTERN)
                .expect("bad_source_word_id_pattern.json is shape-valid JSON");
        assert_eq!(
            bad_source_word_id.validate(),
            Err(ModelError::InvalidSourceWordId("cam-a-001_w_000001".into()))
        );

        let unsorted: Transcript = serde_json::from_str(INVALID_UNSORTED_WORD_TIMELINE)
            .expect("unsorted_word_timeline.json is shape-valid JSON");
        assert_eq!(
            unsorted.validate(),
            Err(ModelError::UnsortedWordTimeline("w_000002".into()))
        );
    }

    #[test]
    fn word_validate_rejects_inverted_and_negative_time_ranges() {
        let base = Word {
            id: "w_000001".into(),
            source_word_id: None,
            text: "hi".into(),
            start_ms: 0,
            end_ms: 100,
            confidence: 1.0,
            speaker: None,
            kind: "word".into(),
        };
        assert!(base.validate().is_ok());
        assert_eq!(
            Word {
                end_ms: 0,
                ..base.clone()
            }
            .validate(),
            Err(ModelError::InvalidWordTimeRange("w_000001".into()))
        );
        assert_eq!(
            Word {
                start_ms: -1,
                ..base
            }
            .validate(),
            Err(ModelError::InvalidWordTimeRange("w_000001".into()))
        );
    }

    #[test]
    fn transcript_validate_rejects_duplicate_word_and_source_word_ids() {
        let word = Word {
            id: "w_000001".into(),
            source_word_id: Some("cam-a-001:w_000001".into()),
            text: "hi".into(),
            start_ms: 0,
            end_ms: 100,
            confidence: 1.0,
            speaker: None,
            kind: "word".into(),
        };
        let duplicate_ids = Transcript {
            schema_version: SCHEMA_VERSION,
            provider: "fixture".into(),
            source_id: "cam-a-001".into(),
            language: "en".into(),
            words: vec![
                word.clone(),
                Word {
                    start_ms: 100,
                    end_ms: 200,
                    source_word_id: None,
                    ..word.clone()
                },
            ],
            events: Vec::new(),
        };
        assert_eq!(
            duplicate_ids.validate(),
            Err(ModelError::DuplicateWordId("w_000001".into()))
        );

        let duplicate_source_word_ids = Transcript {
            words: vec![
                word.clone(),
                Word {
                    id: "w_000002".into(),
                    start_ms: 100,
                    end_ms: 200,
                    ..word
                },
            ],
            ..duplicate_ids
        };
        assert_eq!(
            duplicate_source_word_ids.validate(),
            Err(ModelError::DuplicateSourceWordId(
                "cam-a-001:w_000001".into()
            ))
        );
    }

    #[test]
    fn timebase_validate_rejects_zero_numerator_or_denominator() {
        assert!(Timebase {
            fps_num: 30_000,
            fps_den: 1_001
        }
        .validate()
        .is_ok());
        assert_eq!(
            Timebase {
                fps_num: 0,
                fps_den: 1
            }
            .validate(),
            Err(ModelError::InvalidTimebase(0, 1))
        );
        assert_eq!(
            Timebase {
                fps_num: 30,
                fps_den: 0
            }
            .validate(),
            Err(ModelError::InvalidTimebase(30, 0))
        );
    }
}
