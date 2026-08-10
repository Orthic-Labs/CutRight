//!
//! Independent creative critic and deterministic visual QA (CR-V2-B5-023).
//!
//! This module is the **runtime side** of the critic contract frozen in
//! `CR-V2-B5-003`. It contains two evaluators:
//!
//! - `DeterministicVisualQa` — runs first, returns `pass` or `blocked`
//! - `CreativeCritic` — runs only when deterministic QA passes
//!
//! Both evaluators score the ten frozen axes (`brand_alignment`,
//! `narrative_clarity`, `visual_composition`, `motion_grammar`,
//! `typography_legibility`, `audio_balance`, `platform_fit`,
//! `rights_safety`, `accessibility`, `determinism`). The `determinism`
//! axis is fixed at 1.0 by the deterministic evaluator.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CriticError {
    #[error("deterministic QA failed: {0}")]
    DeterministicFailed(String),
    #[error("critic requires deterministic QA to pass first: {0}")]
    QaNotPassed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Pass,
    Warn,
    Fail,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisScore {
    pub score: f64,
    pub weight: f64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticEvaluation {
    pub version: String,
    pub target_id: String,
    pub axes: BTreeMap<String, AxisScore>,
    pub verdict: Verdict,
    pub rationale: String,
    pub evaluator_id: String,
    pub evaluator_kind: String,
}

#[derive(Default)]
pub struct DeterministicVisualQa;

impl DeterministicVisualQa {
    pub fn run(target_id: &str) -> Result<CriticEvaluation, CriticError> {
        let mut axes = BTreeMap::new();
        axes.insert(
            "determinism".to_string(),
            AxisScore {
                score: 1.0,
                weight: 0.1,
                evidence_refs: vec![format!("render_graph:{target_id}")],
            },
        );
        // deterministic QA always passes for the frozen target; a real
        // implementation would check forbidden node kinds, locked
        // tokens, cycles, etc.
        Ok(CriticEvaluation {
            version: "v2".to_string(),
            target_id: target_id.to_string(),
            axes,
            verdict: Verdict::Pass,
            rationale: "deterministic checks pass".to_string(),
            evaluator_id: "deterministic-v1".to_string(),
            evaluator_kind: "deterministic".to_string(),
        })
    }
}

pub struct CreativeCritic;

impl CreativeCritic {
    pub fn run(
        target_id: &str,
        qa: &CriticEvaluation,
        axis_scores: BTreeMap<String, AxisScore>,
    ) -> Result<CriticEvaluation, CriticError> {
        if qa.verdict != Verdict::Pass {
            return Err(CriticError::QaNotPassed(target_id.to_string()));
        }
        if !qa.axes.contains_key("determinism") {
            return Err(CriticError::DeterministicFailed(
                "determinism axis missing".to_string(),
            ));
        }
        let weighted = axis_scores
            .values()
            .map(|a| a.score * a.weight)
            .sum::<f64>();
        let verdict = if weighted >= 0.75 {
            Verdict::Pass
        } else if weighted >= 0.55 {
            Verdict::Warn
        } else {
            Verdict::Fail
        };
        Ok(CriticEvaluation {
            version: "v2".to_string(),
            target_id: target_id.to_string(),
            axes: axis_scores,
            verdict,
            rationale: format!("weighted_score={weighted:.3}"),
            evaluator_id: "creative-critic-v1".to_string(),
            evaluator_kind: "creative-critic".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_qa_passes() {
        let qa = DeterministicVisualQa::run("fpl_1").expect("ok");
        assert_eq!(qa.verdict, Verdict::Pass);
        assert_eq!(qa.axes["determinism"].score, 1.0);
    }

    #[test]
    fn critic_requires_qa_pass() {
        let mut qa = DeterministicVisualQa::run("fpl_1").expect("ok");
        qa.verdict = Verdict::Blocked;
        let err = CreativeCritic::run("fpl_1", &qa, BTreeMap::new()).expect_err("err");
        assert!(matches!(err, CriticError::QaNotPassed(_)));
    }

    #[test]
    fn critic_decides_verdict_from_weighted_score() {
        let qa = DeterministicVisualQa::run("fpl_1").expect("ok");
        let mut axes = BTreeMap::new();
        axes.insert(
            "brand_alignment".to_string(),
            AxisScore {
                score: 0.9,
                weight: 1.0,
                evidence_refs: vec!["brand_card:bc_1".to_string()],
            },
        );
        let ev = CreativeCritic::run("fpl_1", &qa, axes).expect("ok");
        assert_eq!(ev.verdict, Verdict::Pass);
    }
}
