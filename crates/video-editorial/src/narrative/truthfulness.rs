// Truthfulness-aware ordering (Book 4 lane C, B4-018).
//
// Allows cold-open reorder only when chronology/causality remains
// truthful. Logs from/to index, reason, claim dependencies and
// chronology status.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    pub claim_id: String,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderLog {
    pub from_index: usize,
    pub to_index: usize,
    pub reason: String,
    pub claim_dependencies: Vec<String>,
    pub chronology_status: ChronologyStatus,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChronologyStatus {
    Preserved,
    ColdOpen,
    TruthfulnessRisk,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reorder {
    pub from_index: usize,
    pub to_index: usize,
    pub claim: Claim,
    pub introduces_false_sequence: bool,
    pub breaks_claim_dependency: bool,
}

/// Evaluate a reorder against truthfulness constraints.
pub fn evaluate_reorder(r: &Reorder) -> OrderLog {
    let status = if r.introduces_false_sequence || r.breaks_claim_dependency {
        ChronologyStatus::TruthfulnessRisk
    } else if r.from_index != r.to_index {
        ChronologyStatus::ColdOpen
    } else {
        ChronologyStatus::Preserved
    };
    let reason = match status {
        ChronologyStatus::Preserved => "no change".into(),
        ChronologyStatus::ColdOpen => "cold-open allowed".into(),
        ChronologyStatus::TruthfulnessRisk => {
            if r.introduces_false_sequence {
                "reorder implies false sequence".into()
            } else {
                "reorder breaks claim dependency".into()
            }
        }
    };
    OrderLog {
        from_index: r.from_index,
        to_index: r.to_index,
        reason,
        claim_dependencies: r.claim.depends_on.clone(),
        chronology_status: status,
        evidence_refs: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reorder_no_change_preserves() {
        let r = Reorder {
            from_index: 1,
            to_index: 1,
            claim: Claim {
                claim_id: "c1".into(),
                depends_on: vec![],
            },
            introduces_false_sequence: false,
            breaks_claim_dependency: false,
        };
        let log = evaluate_reorder(&r);
        assert_eq!(log.chronology_status, ChronologyStatus::Preserved);
    }

    #[test]
    fn reorder_with_false_sequence_escalates() {
        let r = Reorder {
            from_index: 1,
            to_index: 2,
            claim: Claim {
                claim_id: "c1".into(),
                depends_on: vec!["c2".into()],
            },
            introduces_false_sequence: true,
            breaks_claim_dependency: true,
        };
        let log = evaluate_reorder(&r);
        assert_eq!(log.chronology_status, ChronologyStatus::TruthfulnessRisk);
    }

    #[test]
    fn cold_open_allowed() {
        let r = Reorder {
            from_index: 3,
            to_index: 0,
            claim: Claim {
                claim_id: "c1".into(),
                depends_on: vec![],
            },
            introduces_false_sequence: false,
            breaks_claim_dependency: false,
        };
        let log = evaluate_reorder(&r);
        assert_eq!(log.chronology_status, ChronologyStatus::ColdOpen);
    }
}
