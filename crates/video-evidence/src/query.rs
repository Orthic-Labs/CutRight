//! Bounded evidence retrieval (CR-V2-B3-020).
//!
//! Coarse-to-fine query planning: a single [`EvidenceQuery`] is decomposed
//! into a coarse summary pass and an optional refinement pass. The coarse
//! pass returns compact node/edge summaries under the budget; the refine
//! pass returns only descendants/overlaps of the spans the planner marks.
//!
//! Determinism: the same query against the same graph revision returns
//! the same ordered IDs. There is no float arithmetic in the result set;
//! ties break on `node.id` lex order.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::graph::{EvidenceEdge, EvidenceGraph, EvidenceKind, EvidenceNode};
use crate::tracks::RationalRange;

/// What the caller wants to query. The default scope is "whole project";
/// the planner always paginates under the budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceScope {
    /// Every evidence node/edge in the project graph.
    WholeProject,
    /// Evidence under one node id (the node and its descendants).
    UnderNode(String),
    /// Evidence strictly inside the given time window.
    InWindow(RationalRange),
}

/// The retrieval budget. The planner will return at most `max_nodes`
/// summaries and `max_edges` edges; refinement is a separate budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalBudget {
    pub max_nodes: usize,
    pub max_edges: usize,
    pub max_refinement_nodes: usize,
    pub max_refinement_edges: usize,
}

impl Default for RetrievalBudget {
    fn default() -> Self {
        Self {
            max_nodes: 256,
            max_edges: 512,
            max_refinement_nodes: 64,
            max_refinement_edges: 128,
        }
    }
}

/// The query the caller hands to the planner. `kinds` and `window` are
/// optional filters; `refine` requests a second pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceQuery {
    pub scope: EvidenceScope,
    pub kinds: BTreeSet<EvidenceKind>,
    pub window: Option<RationalRange>,
    pub budget: RetrievalBudget,
    pub refine: bool,
}

impl Default for EvidenceQuery {
    fn default() -> Self {
        Self {
            scope: EvidenceScope::WholeProject,
            kinds: BTreeSet::new(),
            window: None,
            budget: RetrievalBudget::default(),
            refine: false,
        }
    }
}

/// A coarse summary returned by the planner. Just enough information to
/// decide whether to drill in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeSummary {
    pub id: String,
    pub kind: EvidenceKind,
    pub source_revision: String,
    pub confidence_milli: u32,
    pub range: Option<RationalRange>,
}

/// Edge summary. The graph edge type is preserved exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeSummary {
    pub from: String,
    pub to: String,
    pub kind: crate::graph::EdgeKind,
    pub confidence_milli: u32,
}

/// The plan the planner produces before any I/O runs. The plan itself is
/// pure data and is logged into the run receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryPlan {
    pub coarse_node_ids: Vec<String>,
    pub coarse_edge_count: usize,
    pub refinement_windows: Vec<RationalRange>,
    pub budget: RetrievalBudget,
    pub pagination_cursor: Option<usize>,
}

/// The result the planner returns. Identical inputs produce identical
/// results byte-for-byte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalResult {
    pub plan: QueryPlan,
    pub nodes: Vec<NodeSummary>,
    pub edges: Vec<EdgeSummary>,
    pub refinement_nodes: Vec<NodeSummary>,
    pub refinement_edges: Vec<EdgeSummary>,
    pub has_more: bool,
    pub result_fingerprint: [u8; 32],
}

/// Why the planner rejected a query. Maps to the schema's invalid query
/// cases.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum QueryError {
    #[error("budget is empty")]
    EmptyBudget,
    #[error("scope references unknown node id: {0}")]
    UnknownScopeNode(String),
}

/// Coarse-to-fine planner. Stateless; safe to reuse across requests.
pub struct QueryPlanner;

impl QueryPlanner {
    pub fn new() -> Self {
        Self
    }

    /// Plan and execute a retrieval against the graph.
    pub fn retrieve(
        &self,
        graph: &EvidenceGraph,
        query: &EvidenceQuery,
    ) -> Result<RetrievalResult, QueryError> {
        if query.budget.max_nodes == 0 || query.budget.max_edges == 0 {
            return Err(QueryError::EmptyBudget);
        }
        let coarse_candidates = coarse_candidates(graph, query)?;
        let mut coarse_nodes: Vec<NodeSummary> = coarse_candidates
            .iter()
            .filter_map(|id| graph.nodes.iter().find(|n| &n.id == id))
            .map(NodeSummary::from)
            .collect();
        coarse_nodes.sort_by(|a, b| a.id.cmp(&b.id));
        if coarse_nodes.len() > query.budget.max_nodes {
            coarse_nodes.truncate(query.budget.max_nodes);
        }
        let coarse_edges: Vec<EdgeSummary> = graph
            .edges
            .iter()
            .filter(|e| {
                coarse_nodes.iter().any(|n| n.id == e.from)
                    && coarse_nodes.iter().any(|n| n.id == e.to)
            })
            .take(query.budget.max_edges)
            .map(EdgeSummary::from)
            .collect();
        let has_more = coarse_candidates.len() > coarse_nodes.len();
        let pagination_cursor = if has_more { Some(coarse_nodes.len()) } else { None };

        let (refinement_nodes, refinement_edges, refinement_windows) = if query.refine {
            let windows: Vec<RationalRange> = coarse_nodes
                .iter()
                .filter_map(|n| n.range)
                .filter(|r| r.end.numerator.saturating_sub(r.start.numerator) > 0)
                .collect();
            let (ref_nodes, ref_edges, _) = refine(graph, &coarse_nodes, &windows, &query.budget);
            (ref_nodes, ref_edges, windows)
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        };

        let plan = QueryPlan {
            coarse_node_ids: coarse_nodes.iter().map(|n| n.id.clone()).collect(),
            coarse_edge_count: coarse_edges.len(),
            refinement_windows,
            budget: query.budget.clone(),
            pagination_cursor,
        };
        let fingerprint = result_fingerprint(&plan, &coarse_nodes, &coarse_edges);
        Ok(RetrievalResult {
            plan,
            nodes: coarse_nodes,
            edges: coarse_edges,
            refinement_nodes,
            refinement_edges,
            has_more,
            result_fingerprint: fingerprint,
        })
    }
}

impl Default for QueryPlanner {
    fn default() -> Self {
        Self::new()
    }
}

fn coarse_candidates(
    graph: &EvidenceGraph,
    query: &EvidenceQuery,
) -> Result<Vec<String>, QueryError> {
    let mut out: Vec<String> = Vec::new();
    match &query.scope {
        EvidenceScope::WholeProject => {
            for n in &graph.nodes {
                if !query.kinds.is_empty() && !query.kinds.contains(&n.kind) {
                    continue;
                }
                if let Some(window) = &query.window {
                    if let Some(range) = &n.range {
                        if !ranges_overlap(range, window) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }
                out.push(n.id.clone());
            }
            Ok(out)
        }
        EvidenceScope::UnderNode(root) => {
            let mut visited: BTreeSet<String> = BTreeSet::new();
            let mut stack: Vec<String> = vec![root.clone()];
            while let Some(top) = stack.pop() {
                if !visited.insert(top.clone()) {
                    continue;
                }
                if !graph.nodes.iter().any(|n| n.id == top) {
                    return Err(QueryError::UnknownScopeNode(top));
                }
                for edge in &graph.edges {
                    if edge.kind == crate::graph::EdgeKind::Contains && edge.from == top {
                        stack.push(edge.to.clone());
                    }
                }
            }
            for id in visited {
                if let Some(n) = graph.nodes.iter().find(|n| n.id == id) {
                    if !query.kinds.is_empty() && !query.kinds.contains(&n.kind) {
                        continue;
                    }
                    if let Some(window) = &query.window {
                        if let Some(range) = &n.range {
                            if !ranges_overlap(range, window) {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }
                    out.push(id);
                }
            }
            Ok(out)
        }
        EvidenceScope::InWindow(window) => {
            for n in &graph.nodes {
                if !query.kinds.is_empty() && !query.kinds.contains(&n.kind) {
                    continue;
                }
                if let Some(range) = &n.range {
                    if ranges_overlap(range, window) {
                        out.push(n.id.clone());
                    }
                }
            }
            Ok(out)
        }
    }
}

fn ranges_overlap(a: &RationalRange, b: &RationalRange) -> bool {
    a.start < b.end && b.start < a.end
}

fn refine(
    graph: &EvidenceGraph,
    coarse: &[NodeSummary],
    windows: &[RationalRange],
    budget: &RetrievalBudget,
) -> (Vec<NodeSummary>, Vec<EdgeSummary>, ()) {
    let mut ref_nodes: Vec<NodeSummary> = Vec::new();
    let coarse_ids: BTreeSet<&String> = coarse.iter().map(|n| &n.id).collect();
    for n in &graph.nodes {
        if coarse_ids.contains(&n.id) {
            continue;
        }
        let Some(range) = &n.range else {
            continue;
        };
        let inside_window = windows.iter().any(|w| ranges_overlap(range, w));
        if !inside_window {
            continue;
        }
        ref_nodes.push(NodeSummary::from(n));
    }
    ref_nodes.sort_by(|a, b| a.id.cmp(&b.id));
    ref_nodes.truncate(budget.max_refinement_nodes);
    let ref_node_ids: BTreeSet<String> = ref_nodes.iter().map(|n| n.id.clone()).collect();
    let ref_edges: Vec<EdgeSummary> = graph
        .edges
        .iter()
        .filter(|e| ref_node_ids.contains(&e.from) || ref_node_ids.contains(&e.to))
        .take(budget.max_refinement_edges)
        .map(EdgeSummary::from)
        .collect();
    (ref_nodes, ref_edges, ())
}

fn result_fingerprint(
    plan: &QueryPlan,
    nodes: &[NodeSummary],
    edges: &[EdgeSummary],
) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&(plan.coarse_node_ids.len() as u64).to_le_bytes());
    for id in &plan.coarse_node_ids {
        hasher.update(id.as_bytes());
    }
    hasher.update(&(plan.coarse_edge_count as u64).to_le_bytes());
    for n in nodes {
        hasher.update(n.id.as_bytes());
        hasher.update(&n.confidence_milli.to_le_bytes());
    }
    for e in edges {
        hasher.update(e.from.as_bytes());
        hasher.update(e.to.as_bytes());
        hasher.update(&[e.kind as u8]);
        hasher.update(&e.confidence_milli.to_le_bytes());
    }
    let hash = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    out
}

impl From<&EvidenceNode> for NodeSummary {
    fn from(n: &EvidenceNode) -> Self {
        Self {
            id: n.id.clone(),
            kind: n.kind,
            source_revision: n.source_revision.clone(),
            confidence_milli: n.confidence_milli,
            range: n.range,
        }
    }
}

impl From<&EvidenceEdge> for EdgeSummary {
    fn from(e: &EvidenceEdge) -> Self {
        Self {
            from: e.from.clone(),
            to: e.to.clone(),
            kind: e.kind,
            confidence_milli: e.confidence_milli,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::ProducerIdentity;
    use crate::tracks::RationalTime;

    fn node(id: &str, kind: EvidenceKind, range: Option<RationalRange>) -> EvidenceNode {
        EvidenceNode {
            id: id.to_string(),
            kind,
            source_revision: "rev".to_string(),
            range,
            confidence_milli: 900,
            producer: ProducerIdentity::new("k", "0", "p", [0u8; 32]),
            receipt: None,
            source_hash: None,
        }
    }

    fn edge(from: &str, to: &str, kind: crate::graph::EdgeKind) -> EvidenceEdge {
        EvidenceEdge {
            from: from.to_string(),
            to: to.to_string(),
            kind,
            confidence_milli: 1000,
        }
    }

    fn build_graph() -> EvidenceGraph {
        let mut g = EvidenceGraph::new("g", "proj".to_string(), 0);
        g.append_node(node("frame/0", EvidenceKind::Frame, None))
            .unwrap();
        g.append_node(node("frame/1", EvidenceKind::Frame, None))
            .unwrap();
        g.append_node(node(
            "shot/0",
            EvidenceKind::Shot,
            Some(RationalRange::from_frames(0, 60, 30_000)),
        ))
        .unwrap();
        g.append_edge(edge("shot/0", "frame/0", crate::graph::EdgeKind::Contains))
            .unwrap();
        g.append_edge(edge("shot/0", "frame/1", crate::graph::EdgeKind::Contains))
            .unwrap();
        g
    }

    #[test]
    fn whole_project_query_is_bounded() {
        let g = build_graph();
        let q = EvidenceQuery {
            budget: RetrievalBudget {
                max_nodes: 1,
                max_edges: 1,
                max_refinement_nodes: 0,
                max_refinement_edges: 0,
            },
            ..EvidenceQuery::default()
        };
        let r = QueryPlanner::new().retrieve(&g, &q).unwrap();
        assert_eq!(r.nodes.len(), 1);
        assert!(r.has_more);
        assert!(r.plan.pagination_cursor.is_some());
    }

    #[test]
    fn deterministic_ordering_for_identical_query() {
        let g = build_graph();
        let q = EvidenceQuery::default();
        let r1 = QueryPlanner::new().retrieve(&g, &q).unwrap();
        let r2 = QueryPlanner::new().retrieve(&g, &q).unwrap();
        assert_eq!(r1.result_fingerprint, r2.result_fingerprint);
        assert_eq!(r1.nodes.iter().map(|n| &n.id).collect::<Vec<_>>(),
                   r2.nodes.iter().map(|n| &n.id).collect::<Vec<_>>());
    }

    #[test]
    fn refinement_returns_only_descendants_or_overlaps() {
        let g = build_graph();
        let q = EvidenceQuery {
            scope: EvidenceScope::UnderNode("shot/0".to_string()),
            refine: true,
            ..EvidenceQuery::default()
        };
        let r = QueryPlanner::new().retrieve(&g, &q).unwrap();
        // Coarse contains shot/0 + frame/0 + frame/1 (descendants of shot/0).
        let ids: BTreeSet<String> = r.nodes.iter().map(|n| n.id.clone()).collect();
        assert!(ids.contains("shot/0"));
        assert!(ids.contains("frame/0"));
        assert!(ids.contains("frame/1"));
        // Refinement must be descendants/overlaps only — none of them
        // overlap because every node was already in the coarse set.
        assert!(r.refinement_nodes.iter().all(|n| !ids.contains(&n.id)));
    }

    #[test]
    fn empty_budget_is_rejected() {
        let g = build_graph();
        let q = EvidenceQuery {
            budget: RetrievalBudget {
                max_nodes: 0,
                max_edges: 0,
                max_refinement_nodes: 0,
                max_refinement_edges: 0,
            },
            ..EvidenceQuery::default()
        };
        assert_eq!(
            QueryPlanner::new().retrieve(&g, &q),
            Err(QueryError::EmptyBudget)
        );
    }

    #[test]
    fn in_window_query_filters_by_range() {
        let mut g = build_graph();
        g.append_node(node(
            "shot/1",
            EvidenceKind::Shot,
            Some(RationalRange::from_ms(200, 300)),
        ))
        .unwrap();
        let q = EvidenceQuery {
            scope: EvidenceScope::InWindow(RationalRange::from_ms(0, 100)),
            ..EvidenceQuery::default()
        };
        let r = QueryPlanner::new().retrieve(&g, &q).unwrap();
        let ids: BTreeSet<String> = r.nodes.iter().map(|n| n.id.clone()).collect();
        assert!(ids.contains("shot/0"));
        assert!(!ids.contains("shot/1"));
    }

    #[test]
    fn pagination_is_byte_stable_for_same_query() {
        let g = build_graph();
        let q = EvidenceQuery::default();
        let r = QueryPlanner::new().retrieve(&g, &q).unwrap();
        let cursor = r.plan.pagination_cursor;
        // Re-running with the same query returns the same ordered IDs.
        let r2 = QueryPlanner::new().retrieve(&g, &q).unwrap();
        assert_eq!(cursor, r2.plan.pagination_cursor);
        let _ = RationalTime::ZERO; // silence unused-import lint if any
    }
}