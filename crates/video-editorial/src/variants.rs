// Variant compilation (Book 4 lane C, B4-023).
//
// Compile a validated EditorialPlan + boundary consensus into
// versioned variant plans (natural, tight, long-form, short). Each
// variant binds to the same plan, evidence graph revision, and
// policy. Variants cannot contaminate one another.

use serde::{Deserialize, Serialize};

use crate::narrative::shorts::ShortCandidate;
use crate::plan::EditorialPlan;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariantKind {
    Natural,
    Tight,
    LongForm,
    Short,
}

impl VariantKind {
    pub fn id(self) -> &'static str {
        match self {
            VariantKind::Natural => "natural",
            VariantKind::Tight => "tight",
            VariantKind::LongForm => "long-form",
            VariantKind::Short => "short",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PadPolicy {
    None,
    Short,
    Generous,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariantPlan {
    pub variant_id: String,
    pub kind: VariantKind,
    pub plan_id: String,
    pub evidence_graph_revision: String,
    pub policy_locks: Vec<String>,
    pub pad_policy: PadPolicy,
    pub selected_segments: Vec<String>,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionedVariants {
    pub plan_id: String,
    pub evidence_graph_revision: String,
    pub policy_locks: Vec<String>,
    pub variants: Vec<VariantPlan>,
    pub version: u32,
}

/// Compile all four variants from a plan + boundary consensus.
/// Each variant reuses the same plan id, evidence revision and
/// policy locks. Segments are taken from the plan's order plus
/// shorts for the Short variant.
pub fn compile(
    plan: &EditorialPlan,
    evidence_graph_revision: &str,
    policy_locks: &[String],
) -> VersionedVariants {
    let mut variants: Vec<VariantPlan> = Vec::new();
    let base_segments: Vec<String> = plan.order.order.clone();

    for kind in [
        VariantKind::Natural,
        VariantKind::Tight,
        VariantKind::LongForm,
        VariantKind::Short,
    ] {
        let pad = match kind {
            VariantKind::Natural => PadPolicy::Short,
            VariantKind::Tight => PadPolicy::None,
            VariantKind::LongForm => PadPolicy::Generous,
            VariantKind::Short => PadPolicy::None,
        };
        let selected = match kind {
            VariantKind::Short => short_segments(&plan.shorts),
            _ => base_segments.clone(),
        };
        variants.push(VariantPlan {
            variant_id: format!("variant-{}-{}", plan.plan_id, kind.id()),
            kind,
            plan_id: plan.plan_id.clone(),
            evidence_graph_revision: evidence_graph_revision.to_string(),
            policy_locks: policy_locks.to_vec(),
            pad_policy: pad,
            selected_segments: selected,
            version: plan.version,
        });
    }

    VersionedVariants {
        plan_id: plan.plan_id.clone(),
        evidence_graph_revision: evidence_graph_revision.to_string(),
        policy_locks: policy_locks.to_vec(),
        variants,
        version: plan.version,
    }
}

fn short_segments(shorts: &[ShortCandidate]) -> Vec<String> {
    shorts
        .iter()
        .filter(|s| s.exclusion_reason.is_none())
        .flat_map(|s| s.beat_ids.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narrative::confidence::{ConfidenceEstimate, ReviewMode};
    use crate::narrative::order::OrderPlan;
    use crate::plan::EditorialPlan;

    fn plan() -> EditorialPlan {
        EditorialPlan {
            plan_id: "plan-1".into(),
            proposal_id: "p1".into(),
            review_mode: ReviewMode::Reviewed,
            order: OrderPlan {
                plan_id: "o".into(),
                order: vec!["seg1".into(), "seg2".into()],
                logs: vec![],
                has_truthfulness_risk: false,
            },
            shorts: vec![],
            confidence: ConfidenceEstimate {
                score: 0.9,
                escalations: vec![],
                requested_mode: ReviewMode::Reviewed,
                effective_mode: ReviewMode::Reviewed,
                rationale: vec![],
            },
            repair: None,
            evidence_refs: vec!["e1".into()],
            benchmark_refs: vec![],
            version: 1,
        }
    }

    #[test]
    fn compiles_four_variants() {
        let v = compile(&plan(), "rev-1", &["lock-1".into()]);
        assert_eq!(v.variants.len(), 4);
        let kinds: Vec<_> = v.variants.iter().map(|x| x.kind).collect();
        assert!(kinds.contains(&VariantKind::Natural));
        assert!(kinds.contains(&VariantKind::Tight));
        assert!(kinds.contains(&VariantKind::LongForm));
        assert!(kinds.contains(&VariantKind::Short));
    }

    #[test]
    fn variants_share_plan_revision_locks() {
        let v = compile(&plan(), "rev-1", &["lock-1".into(), "lock-2".into()]);
        for x in &v.variants {
            assert_eq!(x.plan_id, "plan-1");
            assert_eq!(x.evidence_graph_revision, "rev-1");
            assert_eq!(x.policy_locks, vec!["lock-1".to_string(), "lock-2".to_string()]);
            assert_eq!(x.version, 1);
        }
    }

    #[test]
    fn variants_do_not_contaminate_each_other() {
        let v = compile(&plan(), "rev-1", &[]);
        for (i, a) in v.variants.iter().enumerate() {
            for (j, b) in v.variants.iter().enumerate() {
                if i == j {
                    continue;
                }
                assert_ne!(a.variant_id, b.variant_id);
                assert_ne!(a.kind, b.kind);
            }
        }
    }
}