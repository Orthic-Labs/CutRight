//! video-benchmarks — CutRight v2 benchmark evaluators, runner, and report.
//!
//! This crate is the canonical implementation of the Book 4 lane A work.
//! Production code depends only on the `BenchmarkEvaluator` trait and the
//! `MetricStatus` enum. The benchmark runner is read-only against
//! completed project revisions.

#![doc = "video-benchmarks is part of the CutRight v2 benchmark lane A."]

use serde::{Deserialize, Serialize};

/// The canonical result status for every benchmark metric.
///
/// This enum is the Rust counterpart of the JSON enum declared in
/// `docs/benchmarks/V2-TAXONOMY.md`. A metric that did not run is
/// `Unproven`, never `Pass`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricStatus {
    Pass,
    Fail,
    SkippedWithReason,
    Unsupported,
    Unproven,
}

/// Axis grouping for benchmark findings (Book 4 `V2-TAXONOMY.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Axis {
    KernelIntegrity,
    SpeechBoundary,
    AudioVisual,
    Editorial,
    Creative,
    Instruction,
    Reliability,
}

/// Identifier for an evaluation axis (matches the JSON enum strings).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AxisId {
    KernelIntegrity,
    SpeechBoundary,
    AudioVisual,
    Editorial,
    Creative,
    Instruction,
    Reliability,
}

impl AxisId {
    pub fn as_str(&self) -> &'static str {
        match self {
            AxisId::KernelIntegrity => "kernel_integrity",
            AxisId::SpeechBoundary => "speech_boundary",
            AxisId::AudioVisual => "audio_visual",
            AxisId::Editorial => "editorial",
            AxisId::Creative => "creative",
            AxisId::Instruction => "instruction",
            AxisId::Reliability => "reliability",
        }
    }
}

/// A single slide dimension for slicing benchmark results.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SliceKey(pub String);

impl SliceKey {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

/// A phrase referencing a single piece of evidence for a finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub source_range: [i64; 2],
    pub output_range: [i64; 2],
    pub frame_refs: Vec<String>,
    pub word_ids: Vec<String>,
}

/// A single deterministic evaluation outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalOutcome {
    pub metric_id: String,
    pub axis: AxisId,
    pub status: MetricStatus,
    pub value: Option<f64>,
    pub unit: String,
    pub slices: Vec<(SliceKey, String)>,
    pub reason: Option<String>,
    pub evidence: Vec<EvidenceRef>,
}

impl EvalOutcome {
    pub fn fail(metric_id: impl Into<String>, axis: AxisId, reason: impl Into<String>) -> Self {
        Self {
            metric_id: metric_id.into(),
            axis,
            status: MetricStatus::Fail,
            value: None,
            unit: "count".to_string(),
            slices: Vec::new(),
            reason: Some(reason.into()),
            evidence: Vec::new(),
        }
    }

    pub fn pass(
        metric_id: impl Into<String>,
        axis: AxisId,
        value: f64,
        unit: impl Into<String>,
    ) -> Self {
        Self {
            metric_id: metric_id.into(),
            axis,
            status: MetricStatus::Pass,
            value: Some(value),
            unit: unit.into(),
            slices: Vec::new(),
            reason: None,
            evidence: Vec::new(),
        }
    }
}

/// The shared trait every benchmark evaluator implements.
pub trait BenchmarkEvaluator {
    fn id(&self) -> &str;
    fn axis(&self) -> AxisId;
    fn evaluate(&self, ctx: &EvalContext) -> Result<EvalOutcome, EvalError>;
}

#[derive(Debug, Default)]
pub struct EvalContext {
    pub language: Option<String>,
    pub noise: Option<String>,
    pub format: Option<String>,
}

/// Evaluator error type.
#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("missing required input: {0}")]
    MissingInput(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

pub mod audio;
pub mod audio_visual;
pub mod collision;
pub mod crop;
pub mod editorial;
pub mod profile;
pub mod reliability;
pub mod report;
pub mod runner;
pub mod speech;
pub mod visual;
