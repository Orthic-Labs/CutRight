//! Trust / tamper-detection tree.
//!
//! Given a [`TrustInputs`] view of a project (source hashes, canonical
//! objects, revision ancestry, action/job/render/QA receipts, skill/catalog
//! hashes, active pack signatures) [`compute_trust`] returns a
//! [`TrustStatus`] that names the overall disposition and the exact failures
//! — repairable versus non-repairable. Models cannot override the result.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustOverall {
    Pass,
    PassWithNotes,
    FailRepairable,
    FailNonRepairable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustComponent {
    Sources,
    Revisions,
    Actions,
    Jobs,
    Renders,
    Qa,
    Skills,
    Packs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustFailure {
    HashMismatch,
    SignatureInvalid,
    Missing,
    Incompatible,
    NonRepairable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustFailureRecord {
    pub component: TrustComponent,
    pub path: String,
    pub failure: TrustFailure,
    pub repairable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashPair {
    pub id: String,
    pub declared: String,
    pub computed: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedItem {
    pub id: String,
    pub signature_valid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TrustInputs {
    pub sources: Vec<HashPair>,
    pub revisions: Vec<HashPair>,
    pub actions: Vec<HashPair>,
    pub jobs: Vec<HashPair>,
    pub renders: Vec<HashPair>,
    pub qa: Vec<HashPair>,
    pub skills: Vec<HashPair>,
    pub packs: Vec<SignedItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustStatus {
    pub overall: TrustOverall,
    pub sources_ok: bool,
    pub revisions_ok: bool,
    pub actions_ok: bool,
    pub jobs_ok: bool,
    pub renders_ok: bool,
    pub qa_ok: bool,
    pub skills_ok: bool,
    pub packs_ok: bool,
    pub failures: Vec<TrustFailureRecord>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TrustError {
    #[error("model refused to acknowledge tamper event: {0}")]
    ModelRefused(String),
    #[error("trust computation received too many failures: {0}")]
    TooManyFailures(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustComputation {
    pub status: TrustStatus,
    pub can_finalize: bool,
}

pub fn compute_trust(inputs: &TrustInputs) -> TrustComputation {
    let mut failures = Vec::new();
    let mut ok_status = |component: TrustComponent, items: &Vec<HashPair>| -> bool {
        let mut ok = true;
        for p in items {
            if p.declared != p.computed {
                failures.push(TrustFailureRecord {
                    component: component.clone(),
                    path: p.id.clone(),
                    failure: TrustFailure::HashMismatch,
                    repairable: true,
                });
                ok = false;
            }
        }
        ok
    };
    let sources_ok = ok_status(TrustComponent::Sources, &inputs.sources);
    let revisions_ok = ok_status(TrustComponent::Revisions, &inputs.revisions);
    let actions_ok = ok_status(TrustComponent::Actions, &inputs.actions);
    let jobs_ok = ok_status(TrustComponent::Jobs, &inputs.jobs);
    let renders_ok = ok_status(TrustComponent::Renders, &inputs.renders);
    let qa_ok = ok_status(TrustComponent::Qa, &inputs.qa);
    let skills_ok = ok_status(TrustComponent::Skills, &inputs.skills);
    let mut packs_ok = true;
    for p in &inputs.packs {
        if !p.signature_valid {
            failures.push(TrustFailureRecord {
                component: TrustComponent::Packs,
                path: p.id.clone(),
                failure: TrustFailure::SignatureInvalid,
                repairable: false,
            });
            packs_ok = false;
        }
    }
    let overall = classify_overall(
        sources_ok,
        revisions_ok,
        actions_ok,
        jobs_ok,
        renders_ok,
        qa_ok,
        skills_ok,
        packs_ok,
        &failures,
    );
    let can_finalize = matches!(overall, TrustOverall::Pass | TrustOverall::PassWithNotes);
    TrustComputation {
        status: TrustStatus {
            overall,
            sources_ok,
            revisions_ok,
            actions_ok,
            jobs_ok,
            renders_ok,
            qa_ok,
            skills_ok,
            packs_ok,
            failures,
        },
        can_finalize,
    }
}

fn classify_overall(
    sources_ok: bool,
    revisions_ok: bool,
    actions_ok: bool,
    jobs_ok: bool,
    renders_ok: bool,
    qa_ok: bool,
    skills_ok: bool,
    packs_ok: bool,
    failures: &[TrustFailureRecord],
) -> TrustOverall {
    let all_ok = sources_ok
        && revisions_ok
        && actions_ok
        && jobs_ok
        && renders_ok
        && qa_ok
        && skills_ok
        && packs_ok;
    if all_ok {
        return TrustOverall::Pass;
    }
    let non_repairable = failures.iter().any(|f| !f.repairable);
    if non_repairable {
        return TrustOverall::FailNonRepairable;
    }
    // sources/canonical/revisions are non-repairable if tampered; treat
    // them as such even though the per-line repairable flag stayed true.
    let canonical_failed = !sources_ok || !revisions_ok;
    if canonical_failed {
        return TrustOverall::FailNonRepairable;
    }
    let any_repairable = failures.iter().any(|f| f.repairable);
    if any_repairable {
        TrustOverall::FailRepairable
    } else {
        TrustOverall::PassWithNotes
    }
}

/// Hard requirement: a model cannot suppress a tamper result. Any attempt
/// to override [`TrustOverall`] is an error.
pub fn forbid_model_override(
    proposed: TrustOverall,
    authoritative: TrustOverall,
) -> Result<TrustOverall, TrustError> {
    if proposed != authoritative {
        Err(TrustError::ModelRefused(format!(
            "proposed {:?} disagrees with authoritative {:?}",
            proposed, authoritative
        )))
    } else {
        Ok(authoritative)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean() -> TrustInputs {
        TrustInputs::default()
    }

    #[test]
    fn clean_inputs_pass() {
        let c = compute_trust(&clean());
        assert_eq!(c.status.overall, TrustOverall::Pass);
        assert!(c.can_finalize);
    }

    #[test]
    fn source_tamper_is_non_repairable() {
        let mut i = clean();
        i.sources.push(HashPair {
            id: "src.mp4".into(),
            declared: "a".into(),
            computed: "b".into(),
        });
        let c = compute_trust(&i);
        assert_eq!(c.status.overall, TrustOverall::FailNonRepairable);
        assert!(!c.can_finalize);
    }

    #[test]
    fn receipt_tamper_is_repairable() {
        let mut i = clean();
        i.jobs.push(HashPair {
            id: "job.1".into(),
            declared: "a".into(),
            computed: "b".into(),
        });
        let c = compute_trust(&i);
        assert_eq!(c.status.overall, TrustOverall::FailRepairable);
        assert!(!c.can_finalize);
    }

    #[test]
    fn pack_signature_invalid_is_non_repairable() {
        let mut i = clean();
        i.packs.push(SignedItem {
            id: "pack1".into(),
            signature_valid: false,
        });
        let c = compute_trust(&i);
        assert_eq!(c.status.overall, TrustOverall::FailNonRepairable);
    }

    #[test]
    fn model_cannot_override_tamper_result() {
        let r = forbid_model_override(TrustOverall::Pass, TrustOverall::FailNonRepairable);
        assert!(r.is_err());
    }
}
