// Semantic short-form candidate discovery (Book 4 lane C, B4-019).
//
// Builds self-contained short-form windows from evidence-bound
// beats, ranked by hook strength, standalone context, payoff,
// visual support, boundary confidence with a duplication penalty.
// Source ranges are compiled by the caller from selected beat IDs;
// this module never invents timestamps.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortBeatRef {
    pub beat_id: String,
    pub evidence_ref: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortCandidate {
    pub candidate_id: String,
    pub title: String,
    pub hook: String,
    pub beat_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub hook_score: f32,
    pub standalone_context: f32,
    pub payoff: f32,
    pub visual_support: f32,
    pub boundary_confidence: f32,
    pub duplication_penalty: f32,
    pub score: f32,
    pub rationale: String,
    pub exclusion_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortInputs<'a> {
    pub beats: &'a [ShortBeatRef],
    pub hook_strength: f32,
    pub standalone_context: f32,
    pub payoff: f32,
    pub visual_support: f32,
    pub boundary_confidence: f32,
    pub duplication_penalty: f32,
    pub recorded: bool,
}

/// Build one candidate from inputs. Unrecorded (fabricated) hooks
/// are excluded with a reason; never silently dropped.
pub fn build_candidate(id: &str, title: &str, hook: &str, inputs: ShortInputs<'_>) -> ShortCandidate {
    let exclusion = if !inputs.recorded {
        Some("hook not recorded; fabricated text forbidden".into())
    } else {
        None
    };
    let score = if exclusion.is_some() {
        0.0
    } else {
        (inputs.hook_strength
            + inputs.standalone_context
            + inputs.payoff
            + inputs.visual_support
            + inputs.boundary_confidence)
            - inputs.duplication_penalty
    };
    ShortCandidate {
        candidate_id: id.to_string(),
        title: title.to_string(),
        hook: hook.to_string(),
        beat_ids: inputs.beats.iter().map(|b| b.beat_id.clone()).collect(),
        evidence_refs: inputs.beats.iter().map(|b| b.evidence_ref.clone()).collect(),
        hook_score: inputs.hook_strength,
        standalone_context: inputs.standalone_context,
        payoff: inputs.payoff,
        visual_support: inputs.visual_support,
        boundary_confidence: inputs.boundary_confidence,
        duplication_penalty: inputs.duplication_penalty,
        score,
        rationale: format!(
            "score={:.3} = hook({:.2}) + ctx({:.2}) + payoff({:.2}) + visual({:.2}) + boundary({:.2}) - dup({:.2})",
            score,
            inputs.hook_strength,
            inputs.standalone_context,
            inputs.payoff,
            inputs.visual_support,
            inputs.boundary_confidence,
            inputs.duplication_penalty,
        ),
        exclusion_reason: exclusion,
    }
}

/// Rank candidates; excluded ones always sort last, ties broken by id.
pub fn rank(candidates: &[ShortCandidate]) -> Vec<&ShortCandidate> {
    let mut v: Vec<&ShortCandidate> = candidates.iter().collect();
    v.sort_by(|a, b| {
        let ax = a.exclusion_reason.is_some();
        let bx = b.exclusion_reason.is_some();
        ax.cmp(&bx) // false < true, so non-excluded first
            .then_with(|| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.candidate_id.cmp(&b.candidate_id))
    });
    v
}

/// Diversity filter: drop candidates whose beat set is a strict subset
/// of an earlier candidate. Stable across ties.
pub fn diversity_filter(candidates: Vec<ShortCandidate>) -> Vec<ShortCandidate> {
    let ranked = rank(&candidates);
    let mut kept: Vec<ShortCandidate> = Vec::new();
    for c in ranked.into_iter().cloned() {
        if c.exclusion_reason.is_some() {
            continue;
        }
        let subset = kept.iter().any(|k| {
            k.beat_ids.len() > c.beat_ids.len()
                && c.beat_ids.iter().all(|b| k.beat_ids.contains(b))
        });
        if !subset {
            kept.push(c);
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beats() -> Vec<ShortBeatRef> {
        vec![
            ShortBeatRef { beat_id: "b1".into(), evidence_ref: "ev1".into() },
            ShortBeatRef { beat_id: "b2".into(), evidence_ref: "ev2".into() },
        ]
    }

    fn inputs(score: f32) -> ShortInputs<'static> {
        ShortInputs {
            beats: Box::leak(Box::new(beats())),
            hook_strength: score,
            standalone_context: score,
            payoff: score,
            visual_support: score,
            boundary_confidence: score,
            duplication_penalty: 0.0,
            recorded: true,
        }
    }

    #[test]
    fn unrecorded_excludes_with_reason() {
        let mut i = inputs(1.0);
        i.recorded = false;
        let c = build_candidate("c1", "title", "hook", i);
        assert_eq!(c.score, 0.0);
        assert!(c.exclusion_reason.is_some());
    }

    #[test]
    fn score_components_sum() {
        let i = ShortInputs {
            beats: Box::leak(Box::new(beats())),
            hook_strength: 0.8,
            standalone_context: 0.7,
            payoff: 0.9,
            visual_support: 0.6,
            boundary_confidence: 0.5,
            duplication_penalty: 0.1,
            recorded: true,
        };
        let c = build_candidate("c1", "t", "h", i);
        // 0.8+0.7+0.9+0.6+0.5 = 3.5; 3.5 - 0.1 = 3.4
        assert!((c.score - 3.4).abs() < 1e-5);
    }

    #[test]
    fn rank_orders_excluded_last() {
        let i = inputs(1.0);
        let c1 = build_candidate("a", "t", "h", i);
        let mut i2 = inputs(1.0);
        i2.recorded = false;
        let c2 = build_candidate("b", "t", "h", i2);
        let r = rank(&[c2, c1]);
        assert_eq!(r[0].candidate_id, "a");
        assert_eq!(r[1].candidate_id, "b");
    }

    #[test]
    fn diversity_filter_drops_subset() {
        let i_small = ShortInputs {
            beats: Box::leak(Box::new(vec![ShortBeatRef {
                beat_id: "b1".into(),
                evidence_ref: "ev1".into(),
            }])),
            hook_strength: 0.5,
            standalone_context: 0.5,
            payoff: 0.5,
            visual_support: 0.5,
            boundary_confidence: 0.5,
            duplication_penalty: 0.0,
            recorded: true,
        };
        let i_big = ShortInputs {
            beats: Box::leak(Box::new(beats())),
            hook_strength: 0.6,
            standalone_context: 0.6,
            payoff: 0.6,
            visual_support: 0.6,
            boundary_confidence: 0.6,
            duplication_penalty: 0.0,
            recorded: true,
        };
        let small = build_candidate("s", "t", "h", i_small);
        let big = build_candidate("b", "t", "h", i_big);
        let kept = diversity_filter(vec![small, big]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].candidate_id, "b");
    }
}