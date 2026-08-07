// Independent critic with one bounded revision (Book 4 lane C, B4-020).
//
// Runs an independent critic over a proposal and its samples. A
// second disagreement escalates: per the dispatch, only one bounded
// revision request is permitted, and never silently suppressed.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CriticVerdict {
    Approve,
    RequestRevision,
    Block,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CriticFinding {
    pub finding_id: String,
    pub description: String,
    pub severity: FindingSeverity,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingSeverity {
    Info,
    Warn,
    Block,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProposalView {
    pub proposal_id: String,
    pub claim_count: usize,
    pub evidence_count: usize,
    pub has_unknown_candidates: bool,
    pub samples: Vec<SampleView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SampleView {
    pub sample_id: String,
    pub matches_evidence: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CriticOutcome {
    pub verdict: CriticVerdict,
    pub findings: Vec<CriticFinding>,
    pub revision_requested: bool,
}

/// Run the critic. Returns one finding per issue. If a revision is
/// requested but the caller already used the bounded revision, the
/// second disagreement escalates as a Block finding.
pub fn run_critic(view: &ProposalView, revision_used: bool) -> CriticOutcome {
    let mut findings: Vec<CriticFinding> = Vec::new();
    let mut revision_requested = false;

    if view.has_unknown_candidates {
        findings.push(CriticFinding {
            finding_id: "unknown-candidates".into(),
            description: "proposal references unknown candidates".into(),
            severity: FindingSeverity::Block,
            evidence_refs: vec![],
        });
    }

    if view.claim_count == 0 {
        findings.push(CriticFinding {
            finding_id: "no-claims".into(),
            description: "proposal contains no claims".into(),
            severity: FindingSeverity::Warn,
            evidence_refs: vec![],
        });
    }

    let mismatches: Vec<&SampleView> = view
        .samples
        .iter()
        .filter(|s| !s.matches_evidence)
        .collect();
    for s in &mismatches {
        findings.push(CriticFinding {
            finding_id: format!("sample-mismatch:{}", s.sample_id),
            description: "sample does not match evidence".into(),
            severity: FindingSeverity::Warn,
            evidence_refs: vec![],
        });
    }

    let has_block = findings
        .iter()
        .any(|f| matches!(f.severity, FindingSeverity::Block));
    let has_warn = findings
        .iter()
        .any(|f| matches!(f.severity, FindingSeverity::Warn));

    let verdict = if has_block {
        CriticVerdict::Block
    } else if has_warn {
        if revision_used {
            // Second disagreement escalates; cannot silently suppress.
            findings.push(CriticFinding {
                finding_id: "second-disagreement".into(),
                description: "critic requested a second revision".into(),
                severity: FindingSeverity::Block,
                evidence_refs: vec![],
            });
            revision_requested = false;
            CriticVerdict::Block
        } else {
            revision_requested = true;
            CriticVerdict::RequestRevision
        }
    } else {
        CriticVerdict::Approve
    };

    CriticOutcome {
        verdict,
        findings,
        revision_requested,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> ProposalView {
        ProposalView {
            proposal_id: "p".into(),
            claim_count: 1,
            evidence_count: 1,
            has_unknown_candidates: false,
            samples: vec![SampleView {
                sample_id: "s".into(),
                matches_evidence: true,
            }],
        }
    }

    #[test]
    fn approve_clean_proposal() {
        let r = run_critic(&view(), false);
        assert_eq!(r.verdict, CriticVerdict::Approve);
        assert!(!r.revision_requested);
    }

    #[test]
    fn first_disagreement_requests_revision() {
        let mut v = view();
        v.samples.push(SampleView {
            sample_id: "s2".into(),
            matches_evidence: false,
        });
        let r = run_critic(&v, false);
        assert_eq!(r.verdict, CriticVerdict::RequestRevision);
        assert!(r.revision_requested);
    }

    #[test]
    fn second_disagreement_escalates_block() {
        let mut v = view();
        v.samples.push(SampleView {
            sample_id: "s2".into(),
            matches_evidence: false,
        });
        let r = run_critic(&v, true);
        assert_eq!(r.verdict, CriticVerdict::Block);
        assert!(r.findings.iter().any(|f| f.finding_id == "second-disagreement"));
    }

    #[test]
    fn unknown_candidates_block() {
        let mut v = view();
        v.has_unknown_candidates = true;
        let r = run_critic(&v, false);
        assert_eq!(r.verdict, CriticVerdict::Block);
    }
}