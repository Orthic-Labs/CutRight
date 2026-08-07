// Schema-bound Director request contract (Book 4 lane C, B4-017).
//
// Defines a Director request containing bounded summaries, candidate
// beats/takes, user brief, format constraints, and evidence references.
// The Director cannot emit raw timestamps; Rust compiles them.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateRef {
    pub beat_id: String,
    pub take_id: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorialRequest {
    pub user_brief: String,
    pub format: String,
    pub duration_ms_target: i64,
    pub candidates: Vec<CandidateRef>,
    pub evidence_refs: Vec<String>,
    pub model_revision: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorialProposal {
    pub proposal_id: String,
    pub selected: Vec<CandidateRef>,
    pub order: Vec<String>,
    pub arc_id: String,
    pub rationale: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub trait EditorialDirector {
    fn propose(&self, req: &EditorialRequest) -> Result<EditorialProposal, DirectorError>;
}

#[derive(Debug, thiserror::Error)]
pub enum DirectorError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("unknown candidate reference: {0}")]
    UnknownCandidate(String),
    #[error("schema invalid: {0}")]
    SchemaInvalid(String),
}

/// Validate a proposal: every `selected` ref must be in the request's
/// candidates; `order` must be a permutation of `selected` ids; arc_id
/// must not be empty; rationale must be non-empty.
pub fn validate_proposal(
    req: &EditorialRequest,
    prop: &EditorialProposal,
) -> Result<(), DirectorError> {
    let known: std::collections::HashSet<&str> = req
        .candidates
        .iter()
        .map(|c| c.take_id.as_str())
        .collect();
    for s in &prop.selected {
        if !known.contains(s.take_id.as_str()) {
            return Err(DirectorError::UnknownCandidate(s.take_id.clone()));
        }
    }
    let sel_ids: std::collections::HashSet<&str> =
        prop.selected.iter().map(|s| s.take_id.as_str()).collect();
    let ord_ids: std::collections::HashSet<&str> =
        prop.order.iter().map(|s| s.as_str()).collect();
    if sel_ids != ord_ids {
        return Err(DirectorError::SchemaInvalid("order permutation".into()));
    }
    if prop.arc_id.is_empty() {
        return Err(DirectorError::SchemaInvalid("missing arc_id".into()));
    }
    if prop.rationale.is_empty() {
        return Err(DirectorError::SchemaInvalid("missing rationale".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req() -> EditorialRequest {
        EditorialRequest {
            user_brief: "demo".into(),
            format: "shorts".into(),
            duration_ms_target: 30_000,
            candidates: vec![
                CandidateRef { beat_id: "b1".into(), take_id: "t1".into(), role: "hook".into() },
                CandidateRef { beat_id: "b2".into(), take_id: "t2".into(), role: "payoff".into() },
            ],
            evidence_refs: vec!["e1".into()],
            model_revision: "v1".into(),
        }
    }

    fn prop_ok() -> EditorialProposal {
        EditorialProposal {
            proposal_id: "p1".into(),
            selected: vec![
                CandidateRef { beat_id: "b1".into(), take_id: "t1".into(), role: "hook".into() },
                CandidateRef { beat_id: "b2".into(), take_id: "t2".into(), role: "payoff".into() },
            ],
            order: vec!["t1".into(), "t2".into()],
            arc_id: "shorts.hook-payoff".into(),
            rationale: vec!["hook -> payoff".into()],
            evidence_refs: vec!["e1".into()],
        }
    }

    #[test]
    fn valid_proposal_passes() {
        assert!(validate_proposal(&req(), &prop_ok()).is_ok());
    }

    #[test]
    fn unknown_candidate_rejected() {
        let mut p = prop_ok();
        p.selected.push(CandidateRef { beat_id: "bx".into(), take_id: "tx".into(), role: "x".into() });
        p.order.push("tx".into());
        assert!(matches!(validate_proposal(&req(), &p), Err(DirectorError::UnknownCandidate(_))));
    }

    #[test]
    fn missing_rationale_rejected() {
        let mut p = prop_ok();
        p.rationale.clear();
        assert!(matches!(validate_proposal(&req(), &p), Err(DirectorError::SchemaInvalid(_))));
    }
}