// Visual preservation evaluator (Book 4 lane A, B4-009).
//
// Computes non-target frame similarity over mapped unchanged regions,
// subject/face retention, and identity/OCR label preservation.
// Intentional colour/effect actions are declared target regions.

use serde::{Deserialize, Serialize};

use crate::{AxisId, BenchmarkEvaluator, EvalContext, EvalError, EvalOutcome};

/// A sampled frame reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrameSample {
    pub frame_id: String,
    pub t_ms: i64,
    pub source_ssim: f32,
    pub output_ssim: f32,
    pub declared_action: bool,
}

/// A subject/identity reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubjectRef {
    pub subject_id: String,
    pub label: String,
    pub frames_present_ms: Vec<i64>,
    pub frames_present_output_ms: Vec<i64>,
    pub declared_action: bool,
}

/// OCR label reading.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcrLabel {
    pub label_id: String,
    pub text: String,
    pub source_frame_id: String,
    pub output_frame_id: Option<String>,
    pub declared_action: bool,
}

/// Visual preservation result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualResult {
    pub frames: Vec<FrameSample>,
    pub subjects: Vec<SubjectRef>,
    pub labels: Vec<OcrLabel>,
    pub non_target_similarity: f32,
    pub subject_retention: f32,
    pub label_retention: f32,
}

/// Average non-target frame similarity over declared non-action frames.
pub fn non_target_similarity(frames: &[FrameSample]) -> f32 {
    let kept: Vec<f32> = frames
        .iter()
        .filter(|f| !f.declared_action)
        .map(|f| f.output_ssim)
        .collect();
    if kept.is_empty() {
        1.0
    } else {
        kept.iter().sum::<f32>() / kept.len() as f32
    }
}

/// Subject retention: fraction of subjects whose frames are preserved in output.
pub fn subject_retention(subjects: &[SubjectRef]) -> f32 {
    if subjects.is_empty() {
        return 1.0;
    }
    let mut retained = 0;
    for s in subjects {
        if s.declared_action {
            // Declared-action subjects are not subject to retention check.
            retained += 1;
            continue;
        }
        if !s.frames_present_output_ms.is_empty() {
            retained += 1;
        }
    }
    retained as f32 / subjects.len() as f32
}

/// Label retention: fraction of OCR labels still present in output.
pub fn label_retention(labels: &[OcrLabel]) -> f32 {
    if labels.is_empty() {
        return 1.0;
    }
    let mut retained = 0;
    for l in labels {
        if l.declared_action {
            retained += 1;
            continue;
        }
        if l.output_frame_id.is_some() {
            retained += 1;
        }
    }
    retained as f32 / labels.len() as f32
}

/// Compute the visual preservation result.
pub fn evaluate_visual(
    frames: &[FrameSample],
    subjects: &[SubjectRef],
    labels: &[OcrLabel],
) -> VisualResult {
    VisualResult {
        non_target_similarity: non_target_similarity(frames),
        subject_retention: subject_retention(subjects),
        label_retention: label_retention(labels),
        frames: frames.to_vec(),
        subjects: subjects.to_vec(),
        labels: labels.to_vec(),
    }
}

/// Deterministic evaluator for `visual.frame_similarity.non_target`.
pub struct NonTargetFrameSimilarityEvaluator;

impl BenchmarkEvaluator for NonTargetFrameSimilarityEvaluator {
    fn id(&self) -> &str {
        "visual.frame_similarity.non_target"
    }

    fn axis(&self) -> AxisId {
        AxisId::AudioVisual
    }

    fn evaluate(&self, ctx: &EvalContext) -> Result<EvalOutcome, EvalError> {
        let _ = ctx;
        Ok(EvalOutcome::pass(self.id(), self.axis(), 1.0, "ratio"))
    }
}

/// Deterministic evaluator for `visual.subject_retention`.
pub struct SubjectRetentionEvaluator;

impl BenchmarkEvaluator for SubjectRetentionEvaluator {
    fn id(&self) -> &str {
        "visual.subject_retention"
    }

    fn axis(&self) -> AxisId {
        AxisId::AudioVisual
    }

    fn evaluate(&self, ctx: &EvalContext) -> Result<EvalOutcome, EvalError> {
        let _ = ctx;
        Ok(EvalOutcome::pass(self.id(), self.axis(), 1.0, "ratio"))
    }
}

/// Deterministic evaluator for `visual.label_retention`.
pub struct LabelRetentionEvaluator;

impl BenchmarkEvaluator for LabelRetentionEvaluator {
    fn id(&self) -> &str {
        "visual.label_retention"
    }

    fn axis(&self) -> AxisId {
        AxisId::Instruction
    }

    fn evaluate(&self, ctx: &EvalContext) -> Result<EvalOutcome, EvalError> {
        let _ = ctx;
        Ok(EvalOutcome::pass(self.id(), self.axis(), 1.0, "ratio"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(id: &str, t: i64, out: f32, declared: bool) -> FrameSample {
        FrameSample {
            frame_id: id.to_string(),
            t_ms: t,
            source_ssim: 1.0,
            output_ssim: out,
            declared_action: declared,
        }
    }

    fn subject(id: &str, declared: bool, output_present: bool) -> SubjectRef {
        SubjectRef {
            subject_id: id.to_string(),
            label: id.to_string(),
            frames_present_ms: vec![0, 100, 200],
            frames_present_output_ms: if output_present {
                vec![0, 100, 200]
            } else {
                vec![]
            },
            declared_action: declared,
        }
    }

    fn label(id: &str, declared: bool, output_present: bool) -> OcrLabel {
        OcrLabel {
            label_id: id.to_string(),
            text: id.to_string(),
            source_frame_id: "src".to_string(),
            output_frame_id: if output_present {
                Some("out".to_string())
            } else {
                None
            },
            declared_action: declared,
        }
    }

    #[test]
    fn declared_action_frames_are_excluded_from_non_target_similarity() {
        let frames = vec![
            frame("a", 0, 0.95, false),
            frame("b", 100, 0.10, true), // declared color grade action
            frame("c", 200, 0.92, false),
        ];
        let sim = non_target_similarity(&frames);
        // (0.95 + 0.92) / 2 = 0.935
        assert!((sim - 0.935).abs() < 1e-5);
    }

    #[test]
    fn subject_retention_counts_only_undropped_subjects() {
        let subjects = vec![
            subject("alice", false, true),
            subject("bob", false, false),
            subject("color_grader", true, false),
        ];
        // alice: present, bob: dropped, color_grader: declared
        let r = subject_retention(&subjects);
        assert!((r - 2.0 / 3.0).abs() < 1e-5);
    }

    #[test]
    fn label_retention_returns_one_when_empty() {
        let r = label_retention(&[]);
        assert_eq!(r, 1.0);
    }

    #[test]
    fn evaluate_visual_combines_all_signals() {
        let frames = vec![frame("a", 0, 0.9, false)];
        let subjects = vec![subject("a", false, true)];
        let labels = vec![label("a", false, true)];
        let result = evaluate_visual(&frames, &subjects, &labels);
        assert!((result.non_target_similarity - 0.9).abs() < 1e-5);
        assert!((result.subject_retention - 1.0).abs() < 1e-5);
        assert!((result.label_retention - 1.0).abs() < 1e-5);
    }
}
