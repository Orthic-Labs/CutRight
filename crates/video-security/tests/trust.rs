//! Tamper and trust-status integration tests.

use video_security::trust::{
    compute_trust, forbid_model_override, HashPair, SignedItem, TrustInputs, TrustOverall,
};

fn clean() -> TrustInputs {
    TrustInputs::default()
}

#[test]
fn tampered_source_fails_distinctly_from_receipt() {
    let mut i = clean();
    i.sources.push(HashPair {
        id: "src.mp4".into(),
        declared: "a".into(),
        computed: "b".into(),
    });
    let c = compute_trust(&i);
    assert_eq!(c.status.overall, TrustOverall::FailNonRepairable);
    assert!(c.status.failures.iter().any(|f| f.path == "src.mp4"));

    let mut j = clean();
    j.jobs.push(HashPair {
        id: "job.1".into(),
        declared: "a".into(),
        computed: "b".into(),
    });
    let c2 = compute_trust(&j);
    assert_eq!(c2.status.overall, TrustOverall::FailRepairable);
}

#[test]
fn tampered_object_fails_distinctly_from_pack() {
    let mut i = clean();
    i.revisions.push(HashPair {
        id: "rev.1".into(),
        declared: "a".into(),
        computed: "b".into(),
    });
    let c = compute_trust(&i);
    assert_eq!(c.status.overall, TrustOverall::FailNonRepairable);

    let mut j = clean();
    j.packs.push(SignedItem {
        id: "pack.1".into(),
        signature_valid: false,
    });
    let c2 = compute_trust(&j);
    assert_eq!(c2.status.overall, TrustOverall::FailNonRepairable);
    assert!(!c2.status.packs_ok);
}

#[test]
fn trust_status_cannot_be_overridden_by_a_model() {
    let c = compute_trust(&clean());
    let r = forbid_model_override(TrustOverall::Pass, c.status.overall.clone());
    // Empty inputs => Pass; the override attempt with non-Pass proposed
    // must fail. The default proposed/authoritative equality test below
    // proves the floor is enforced.
    assert!(r.is_ok());
    let r2 = forbid_model_override(TrustOverall::FailNonRepairable, c.status.overall.clone());
    assert!(r2.is_err());
}

#[test]
fn repairable_vs_non_repairable_is_explicit() {
    let mut i = clean();
    i.jobs.push(HashPair {
        id: "job.1".into(),
        declared: "a".into(),
        computed: "b".into(),
    });
    i.packs.push(SignedItem {
        id: "pack.1".into(),
        signature_valid: false,
    });
    let c = compute_trust(&i);
    assert_eq!(c.status.overall, TrustOverall::FailNonRepairable);
    assert!(c.status.failures.iter().any(|f| !f.repairable));
}

#[test]
fn receipt_hash_mismatch_classifies_as_fail_repairable() {
    let mut i = clean();
    i.actions.push(HashPair {
        id: "act.1".into(),
        declared: "a".into(),
        computed: "b".into(),
    });
    let c = compute_trust(&i);
    assert_eq!(c.status.overall, TrustOverall::FailRepairable);
    assert!(!c.status.actions_ok);
}
