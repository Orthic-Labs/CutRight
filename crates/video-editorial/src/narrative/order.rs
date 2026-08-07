// Order log aggregation (Book 4 lane C, B4-018).

use serde::{Deserialize, Serialize};

use super::truthfulness::{ChronologyStatus, OrderLog};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderPlan {
    pub plan_id: String,
    pub order: Vec<String>,
    pub logs: Vec<OrderLog>,
    pub has_truthfulness_risk: bool,
}

/// Build an order plan from the final order and the reorder logs.
pub fn build_plan(plan_id: &str, order: Vec<String>, logs: Vec<OrderLog>) -> OrderPlan {
    let has_truthfulness_risk = logs
        .iter()
        .any(|l| matches!(l.chronology_status, ChronologyStatus::TruthfulnessRisk));
    OrderPlan {
        plan_id: plan_id.to_string(),
        order,
        logs,
        has_truthfulness_risk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::narrative::truthfulness::evaluate_reorder;
    use crate::narrative::truthfulness::{Claim, Reorder};

    #[test]
    fn plan_inherits_truthfulness_flag() {
        let r = Reorder {
            from_index: 0,
            to_index: 1,
            claim: Claim {
                claim_id: "c".into(),
                depends_on: vec![],
            },
            introduces_false_sequence: true,
            breaks_claim_dependency: false,
        };
        let log = evaluate_reorder(&r);
        let plan = build_plan("p1", vec!["a".into(), "b".into()], vec![log]);
        assert!(plan.has_truthfulness_risk);
    }

    #[test]
    fn plan_no_risk_when_no_reorders() {
        let plan = build_plan("p1", vec!["a".into()], vec![]);
        assert!(!plan.has_truthfulness_risk);
    }
}
