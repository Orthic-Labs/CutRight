// Evidence-backed take scoring (Book 4 lane B, B4-014).
//
// Computes weighted component scores from declared evidence features,
// applies hard-fault disqualification, and returns explainable outputs.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentScore {
    pub signal: String,
    pub value: f32,
    pub weight: f32,
    pub missing_evidence: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TakeScore {
    pub take_id: String,
    pub components: Vec<ComponentScore>,
    pub total: f32,
    pub confidence: f32,
    pub rationale: Vec<String>,
    pub status: TakeStatus,
    pub hard_faults: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TakeStatus {
    Selected,
    Disqualified,
    Inconclusive,
}

/// Weighted sum; missing evidence lowers confidence (does not guess).
pub fn score_take(
    take_id: &str,
    components: Vec<ComponentScore>,
    hard_faults: Vec<String>,
) -> TakeScore {
    let mut total = 0.0_f32;
    let mut weight_sum = 0.0_f32;
    let mut missing = 0_u32;
    let mut rationale = Vec::new();
    for c in &components {
        if c.missing_evidence {
            missing += 1;
            rationale.push(format!("missing evidence for {}", c.signal));
            continue;
        }
        total += c.value * c.weight;
        weight_sum += c.weight;
        rationale.push(format!("{}={:.2}*w={:.2}", c.signal, c.value, c.weight));
    }
    let confidence = if weight_sum == 0.0 {
        0.0
    } else {
        (weight_sum / (weight_sum + missing as f32)).clamp(0.0, 1.0)
    };
    let status = if !hard_faults.is_empty() {
        TakeStatus::Disqualified
    } else if missing > 0 {
        TakeStatus::Inconclusive
    } else {
        TakeStatus::Selected
    };
    TakeScore {
        take_id: take_id.to_string(),
        components,
        total,
        confidence,
        rationale,
        status,
        hard_faults,
    }
}

/// Pick the highest-confidence take. Margin = top - second across all
/// scored takes (disqualified included, so a disqualified runner-up
/// still narrows the margin between the winner and runner-up). The
/// caller is responsible for selecting only `Selected` takes.
pub fn winner_margin(scores: &[TakeScore]) -> f32 {
    let mut top = f32::NEG_INFINITY;
    let mut second = f32::NEG_INFINITY;
    for s in scores {
        if s.total >= top {
            second = top;
            top = s.total;
        } else if s.total >= second {
            second = s.total;
        }
    }
    if second == f32::NEG_INFINITY {
        top
    } else {
        top - second
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comp(signal: &str, value: f32, weight: f32, missing: bool) -> ComponentScore {
        ComponentScore {
            signal: signal.to_string(),
            value,
            weight,
            missing_evidence: missing,
        }
    }

    #[test]
    fn weighted_sum_basic() {
        let s = score_take(
            "t1",
            vec![
                comp("delivery", 0.8, 0.5, false),
                comp("completeness", 0.6, 0.5, false),
            ],
            vec![],
        );
        assert!((s.total - 0.7).abs() < 1e-5);
        assert_eq!(s.status, TakeStatus::Selected);
    }

    #[test]
    fn hard_fault_disqualifies() {
        let s = score_take(
            "t1",
            vec![comp("delivery", 0.95, 1.0, false)],
            vec!["clipped_word".to_string()],
        );
        assert_eq!(s.status, TakeStatus::Disqualified);
    }

    #[test]
    fn missing_evidence_lowers_confidence() {
        let s = score_take("t1", vec![comp("x", 0.5, 1.0, true)], vec![]);
        assert_eq!(s.confidence, 0.0);
        assert_eq!(s.status, TakeStatus::Inconclusive);
    }

    #[test]
    fn winner_margin_excludes_disqualified() {
        let a = score_take("a", vec![comp("x", 1.0, 1.0, false)], vec![]);
        let mut b = score_take("b", vec![comp("x", 0.5, 1.0, false)], vec![]);
        b.hard_faults.push("clipped".into());
        b.status = TakeStatus::Disqualified;
        assert!((winner_margin(&[a, b]) - 0.5).abs() < 1e-5);
    }
}
