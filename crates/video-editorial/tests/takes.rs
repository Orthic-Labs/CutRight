// Integration tests for deterministic take clustering
// (Book 4 lane B, B4-013).

use std::collections::BTreeSet;
use video_editorial::deterministic::takes::{
    cluster_takes, is_duplicate, normalize_tokens, token_overlap, ClusterPolicy, Take,
};

fn take(id: &str, toks: &[&str], neg: bool) -> Take {
    Take {
        take_id: id.into(),
        start_ms: 0,
        end_ms: 1000,
        tokens: normalize_tokens(&toks.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
        named_entities: BTreeSet::new(),
        has_negation: neg,
        numbers: vec![],
        retake_marker: false,
        embedding_ref: Some("e".into()),
    }
}

#[test]
fn duplicates_cluster() {
    let policy = ClusterPolicy::default();
    let a = take("t1", &["hello", "world", "today"], false);
    let b = take("t2", &["hello", "world", "yesterday"], false);
    assert!(is_duplicate(&a, &b, &policy));
    let clusters = cluster_takes(&[a, b], &policy);
    assert_eq!(clusters.len(), 1);
}

#[test]
fn contradictories_do_not_cluster() {
    let policy = ClusterPolicy::default();
    let a = take("t1", &["yes", "go"], false);
    let b = take("t2", &["yes", "go"], true);
    assert!(!is_duplicate(&a, &b, &policy));
    let clusters = cluster_takes(&[a, b], &policy);
    assert_eq!(clusters.len(), 2);
}

#[test]
fn empty_input_yields_no_clusters() {
    let clusters = cluster_takes(&[], &ClusterPolicy::default());
    assert!(clusters.is_empty());
}

#[test]
fn jaccard_basic() {
    let a = normalize_tokens(&vec!["alpha".into(), "beta".into()]);
    let b = normalize_tokens(&vec!["alpha".into(), "gamma".into()]);
    assert!((token_overlap(&a, &b) - 1.0 / 3.0).abs() < 1e-5);
}
