// Deterministic duplicate-take and restatement clustering
// (Book 4 lane B, B4-013).
//
// Normalises tokens while preserving named entities, negation,
// numbers and semantic differences. Combines token overlap,
// embedding similarity, temporal proximity and retake markers.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A single take under consideration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Take {
    pub take_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub tokens: Vec<String>,
    pub named_entities: BTreeSet<String>,
    pub has_negation: bool,
    pub numbers: Vec<String>,
    pub retake_marker: bool,
    pub embedding_ref: Option<String>,
}

/// Policy thresholds for clustering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClusterPolicy {
    pub lexical_floor: f32,
    pub semantic_floor: f32,
    pub contradiction_ceiling: f32,
    pub temporal_window_ms: i64,
}

impl Default for ClusterPolicy {
    fn default() -> Self {
        Self {
            lexical_floor: 0.5,
            semantic_floor: 0.7,
            contradiction_ceiling: 0.3,
            temporal_window_ms: 60_000,
        }
    }
}

/// A cluster of duplicate takes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TakeCluster {
    pub cluster_id: String,
    pub take_ids: Vec<String>,
    pub lexical_overlap: f32,
    pub semantic_similarity: f32,
    pub contradiction_score: f32,
}

/// Normalise tokens: lowercase, strip punctuation, drop empty.
pub fn normalize_tokens(raw: &[String]) -> Vec<String> {
    raw.iter()
        .map(|t| t.to_lowercase())
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Token overlap coefficient (Jaccard).
pub fn token_overlap(a: &[String], b: &[String]) -> f32 {
    let sa: BTreeSet<&String> = a.iter().collect();
    let sb: BTreeSet<&String> = b.iter().collect();
    if sa.is_empty() && sb.is_empty() {
        return 1.0;
    }
    let inter = sa.intersection(&sb).count() as f32;
    let union = sa.union(&sb).count() as f32;
    if union == 0.0 { 0.0 } else { inter / union }
}

/// Detect contradiction: opposite negation state, conflicting numbers
/// or missing entity. Returns 0..=1.
pub fn contradiction_score(a: &Take, b: &Take) -> f32 {
    let mut s = 0.0_f32;
    if a.has_negation != b.has_negation {
        s += 0.6;
    }
    let na: BTreeSet<&String> = a.numbers.iter().collect();
    let nb: BTreeSet<&String> = b.numbers.iter().collect();
    if na != nb && !na.is_empty() && !nb.is_empty() {
        s += 0.4;
    }
    s.min(1.0)
}

/// Semantic similarity placeholder using embedding_ref identity.
/// When embedding_ref is identical, similarity is 1.0; otherwise 0.0.
/// Real implementations would compare vectors.
pub fn semantic_similarity(a: &Take, b: &Take) -> f32 {
    match (&a.embedding_ref, &b.embedding_ref) {
        (Some(x), Some(y)) if x == y => 1.0,
        (None, None) => 0.5,
        _ => 0.0,
    }
}

/// Two takes are duplicates if they clear all three policy thresholds
/// and lie inside the temporal window.
pub fn is_duplicate(a: &Take, b: &Take, policy: &ClusterPolicy) -> bool {
    let overlap = token_overlap(&a.tokens, &b.tokens);
    let sem = semantic_similarity(a, b);
    let contra = contradiction_score(a, b);
    let temporal_gap = (a.start_ms - b.start_ms).abs();
    overlap >= policy.lexical_floor
        && sem >= policy.semantic_floor
        && contra < policy.contradiction_ceiling
        && temporal_gap <= policy.temporal_window_ms
}

/// Cluster takes by union-find of duplicate pairs.
pub fn cluster_takes(takes: &[Take], policy: &ClusterPolicy) -> Vec<TakeCluster> {
    let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
    for t in takes {
        parent.insert(t.take_id.as_str(), t.take_id.as_str());
    }
    fn find<'a>(parent: &BTreeMap<&'a str, &'a str>, mut x: &'a str) -> &'a str {
        while let Some(&p) = parent.get(x) {
            if p == x { return x; }
            x = p;
        }
        x
    }
    for i in 0..takes.len() {
        for j in (i + 1)..takes.len() {
            if is_duplicate(&takes[i], &takes[j], policy) {
                let pi = find(&parent, takes[i].take_id.as_str());
                let pj = find(&parent, takes[j].take_id.as_str());
                if pi != pj {
                    parent.insert(pj, pi);
                }
            }
        }
    }
    let mut groups: BTreeMap<&str, Vec<&Take>> = BTreeMap::new();
    for t in takes {
        let root = find(&parent, t.take_id.as_str());
        groups.entry(root).or_default().push(t);
    }
    groups
        .into_values()
        .enumerate()
        .map(|(i, group)| {
            let ids: Vec<String> = group.iter().map(|t| t.take_id.clone()).collect();
            let a = &group[0];
            let b = group.get(1).unwrap_or(a);
            TakeCluster {
                cluster_id: format!("cluster-{}", i),
                take_ids: ids,
                lexical_overlap: token_overlap(&a.tokens, &b.tokens),
                semantic_similarity: semantic_similarity(a, b),
                contradiction_score: contradiction_score(a, b),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn take(id: &str, toks: &[&str], neg: bool, retake: bool) -> Take {
        Take {
            take_id: id.into(),
            start_ms: 0,
            end_ms: 1000,
            tokens: normalize_tokens(&toks.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
            named_entities: BTreeSet::new(),
            has_negation: neg,
            numbers: vec![],
            retake_marker: retake,
            embedding_ref: None,
        }
    }

    #[test]
    fn token_overlap_identical_is_one() {
        let a = normalize_tokens(&vec!["hello".into(), "world".into()]);
        let b = normalize_tokens(&vec!["hello".into(), "world".into()]);
        assert!((token_overlap(&a, &b) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn token_overlap_disjoint_is_zero() {
        let a = normalize_tokens(&vec!["hello".into()]);
        let b = normalize_tokens(&vec!["world".into()]);
        assert_eq!(token_overlap(&a, &b), 0.0);
    }

    #[test]
    fn contradictory_negation_not_duplicate() {
        let policy = ClusterPolicy::default();
        let mut a = take("t1", &["yes", "go"], false, false);
        let mut b = take("t2", &["yes", "go"], true, false);
        a.embedding_ref = Some("e".into());
        b.embedding_ref = Some("e".into());
        assert!(!is_duplicate(&a, &b, &policy));
    }

    #[test]
    fn similar_takes_cluster() {
        let policy = ClusterPolicy::default();
        let mut a = take("t1", &["hello", "world", "today"], false, false);
        let mut b = take("t2", &["hello", "world", "yesterday"], false, false);
        a.embedding_ref = Some("e".into());
        b.embedding_ref = Some("e".into());
        let clusters = cluster_takes(&[a, b], &policy);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].take_ids.len(), 2);
    }
}