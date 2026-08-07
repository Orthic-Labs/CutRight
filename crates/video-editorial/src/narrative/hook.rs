// Hook candidate ranking (Book 4 lane C, B4-018).
//
// Ranks hook/payoff candidates for specificity, promise,
// self-containment, and evidence strength.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HookCandidate {
    pub take_id: String,
    pub text: String,
    pub specificity: f32,
    pub promise: f32,
    pub self_containment: f32,
    pub evidence_strength: f32,
    pub recorded: bool,
}

/// Score a single hook. Recorded (user-provided/recorded wording) is
/// required to be considered for selection; fabricated text is forbidden.
pub fn score_hook(h: &HookCandidate) -> f32 {
    if !h.recorded {
        return 0.0;
    }
    (h.specificity + h.promise + h.self_containment + h.evidence_strength) / 4.0
}

/// Rank hooks; ties broken by take_id.
pub fn rank(hooks: &[HookCandidate]) -> Vec<&HookCandidate> {
    let mut v: Vec<&HookCandidate> = hooks.iter().collect();
    v.sort_by(|a, b| {
        score_hook(b)
            .partial_cmp(&score_hook(a))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.take_id.cmp(&b.take_id))
    });
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hook(id: &str, recorded: bool, s: f32, p: f32, sc: f32, e: f32) -> HookCandidate {
        HookCandidate {
            take_id: id.into(),
            text: format!("text-{}", id),
            specificity: s,
            promise: p,
            self_containment: sc,
            evidence_strength: e,
            recorded,
        }
    }

    #[test]
    fn unrecorded_hook_scores_zero() {
        let h = hook("h1", false, 1.0, 1.0, 1.0, 1.0);
        assert_eq!(score_hook(&h), 0.0);
    }

    #[test]
    fn recorded_hook_average() {
        let h = hook("h1", true, 0.8, 0.6, 0.4, 1.0);
        assert!((score_hook(&h) - 0.7).abs() < 1e-5);
    }

    #[test]
    fn rank_orders_by_score() {
        let hooks = vec![
            hook("a", true, 0.5, 0.5, 0.5, 0.5),
            hook("b", true, 0.9, 0.9, 0.9, 0.9),
        ];
        let r = rank(&hooks);
        assert_eq!(r[0].take_id, "b");
    }
}