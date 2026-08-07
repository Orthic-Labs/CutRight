//! Retrieval executor (CR-V2-B3-020).
//!
//! Wraps the planner ([`crate::query::QueryPlanner`]) and turns its
//! [`crate::query::RetrievalResult`] into fully typed [`EvidenceNode`] and
//! [`EvidenceEdge`] values the rest of the workspace can consume. The
//! executor never returns more objects than the budget allows; a paginated
//! query is split across calls using the [`RetrievalCursor`] returned by
//! the previous result.
//!
//! Every retrieval query and the hashes of the evidence it returned are
//! recorded in a [`RetrievalRecord`] for the run receipt.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::graph::{EvidenceEdge, EvidenceGraph, EvidenceId};
use crate::query::{
    EdgeSummary, EvidenceQuery, NodeSummary, QueryPlanner, RetrievalBudget, RetrievalResult,
};
use crate::tracks::RationalRange;

/// One recorded retrieval call. The run receipts concatenate these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalRecord {
    pub query_fingerprint: [u8; 32],
    pub result_fingerprint: [u8; 32],
    pub node_hashes: Vec<[u8; 32]>,
    pub edge_signatures: Vec<String>,
}

/// Pagination cursor returned by [`Retriever::execute`]. A `None` cursor
/// means the result is the last page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalCursor {
    pub offset: usize,
    pub page_size: usize,
}

/// High-level retriever. Holds the planner; one retriever instance is
/// safe to reuse across calls because the planner is stateless.
pub struct Retriever {
    planner: QueryPlanner,
}

impl Default for Retriever {
    fn default() -> Self {
        Self::new()
    }
}

impl Retriever {
    pub fn new() -> Self {
        Self {
            planner: QueryPlanner::new(),
        }
    }

    /// Execute a bounded retrieval against the graph.
    pub fn execute(
        &self,
        graph: &EvidenceGraph,
        query: &EvidenceQuery,
    ) -> Result<RetrievalPage, crate::query::QueryError> {
        let result = self.planner.retrieve(graph, query)?;
        let record = build_record(&result);
        let cursor = result
            .plan
            .pagination_cursor
            .map(|offset| RetrievalCursor {
                offset,
                page_size: query.budget.max_nodes,
            });
        let node_ids: Vec<String> = result.nodes.iter().map(|s| s.id.clone()).collect();
        let edge_signatures: Vec<String> = result
            .edges
            .iter()
            .map(|e| format!("{}->{}:{:?}", e.from, e.to, e.kind))
            .collect();
        Ok(RetrievalPage {
            result,
            record,
            cursor,
            node_ids,
            edge_signatures,
        })
    }
}

/// One page of evidence. Holds both the planner result and the typed
/// node/edge identifiers the caller requested.
#[derive(Debug, Clone)]
pub struct RetrievalPage {
    pub result: RetrievalResult,
    pub record: RetrievalRecord,
    pub cursor: Option<RetrievalCursor>,
    pub node_ids: Vec<String>,
    pub edge_signatures: Vec<String>,
}

impl RetrievalPage {
    /// All evidence ids the page returned, in deterministic order.
    pub fn all_ids(&self) -> Vec<EvidenceId> {
        let mut ids: Vec<EvidenceId> = self
            .result
            .nodes
            .iter()
            .map(|n| n.id.clone())
            .collect();
        ids.extend(
            self.result
                .refinement_nodes
                .iter()
                .map(|n| n.id.clone()),
        );
        ids.sort();
        ids.dedup();
        ids
    }

    /// Unique edge signatures the page returned, sorted.
    pub fn edge_signatures(&self) -> Vec<String> {
        let mut sigs: BTreeSet<String> = BTreeSet::new();
        for e in &self.result.edges {
            sigs.insert(format!("{}->{}:{:?}", e.from, e.to, e.kind));
        }
        for e in &self.result.refinement_edges {
            sigs.insert(format!("{}->{}:{:?}", e.from, e.to, e.kind));
        }
        sigs.into_iter().collect()
    }
}

fn build_record(result: &RetrievalResult) -> RetrievalRecord {
    let mut hashes: Vec<[u8; 32]> = Vec::new();
    for n in &result.nodes {
        let mut hasher = blake3::Hasher::new();
        hasher.update(n.id.as_bytes());
        hasher.update(&n.confidence_milli.to_le_bytes());
        let h = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(h.as_bytes());
        hashes.push(out);
    }
    hashes.sort();
    let mut sigs: Vec<String> = Vec::new();
    for e in &result.edges {
        sigs.push(format!("{}->{}:{:?}", e.from, e.to, e.kind));
    }
    sigs.sort();
    let mut hasher = blake3::Hasher::new();
    for id in &result.plan.coarse_node_ids {
        hasher.update(id.as_bytes());
    }
    let query_fp = hasher.finalize();
    let mut query_fp_arr = [0u8; 32];
    query_fp_arr.copy_from_slice(query_fp.as_bytes());
    RetrievalRecord {
        query_fingerprint: query_fp_arr,
        result_fingerprint: result.result_fingerprint,
        node_hashes: hashes,
        edge_signatures: sigs,
    }
}

/// Helper for tests / receipts: convert a [`NodeSummary`] list into a
/// sorted ids vector.
pub fn sorted_ids(summaries: &[NodeSummary]) -> Vec<String> {
    let mut ids: Vec<String> = summaries.iter().map(|n| n.id.clone()).collect();
    ids.sort();
    ids
}

/// Helper for tests / receipts: convert a [`EdgeSummary`] list into
/// signature strings.
pub fn edge_signatures(summaries: &[EdgeSummary]) -> Vec<String> {
    let mut out: Vec<String> = summaries
        .iter()
        .map(|e| format!("{}->{}:{:?}", e.from, e.to, e.kind))
        .collect();
    out.sort();
    out
}

/// Convenience for callers that want a single-page bounded answer for
/// every call site.
pub fn bounded_pagination(budget: &RetrievalBudget) -> usize {
    budget.max_nodes.max(1)
}

#[allow(dead_code)]
fn _doc_range() -> Option<RationalRange> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::ProducerIdentity;
    use crate::query::EvidenceScope;

    fn node(id: &str, kind: crate::graph::EvidenceKind) -> crate::graph::EvidenceNode {
        crate::graph::EvidenceNode {
            id: id.to_string(),
            kind,
            source_revision: "rev".to_string(),
            range: None,
            confidence_milli: 900,
            producer: ProducerIdentity::new("k", "0", "p", [0u8; 32]),
            receipt: None,
            source_hash: None,
        }
    }

    fn edge(
        from: &str,
        to: &str,
        kind: crate::graph::EdgeKind,
    ) -> EvidenceEdge {
        EvidenceEdge {
            from: from.to_string(),
            to: to.to_string(),
            kind,
            confidence_milli: 1000,
        }
    }

    fn build_graph() -> EvidenceGraph {
        let mut g = EvidenceGraph::new("g", "proj".to_string(), 0);
        for i in 0..5 {
            g.append_node(node(&format!("frame/{i}"), crate::graph::EvidenceKind::Frame))
                .unwrap();
        }
        g
    }

    #[test]
    fn retriever_returns_record_for_query() {
        let g = build_graph();
        let q = EvidenceQuery::default();
        let page = Retriever::new().execute(&g, &q).unwrap();
        assert_eq!(page.record.node_hashes.len(), 5);
        assert!(!page.record.query_fingerprint.iter().all(|b| *b == 0));
    }

    #[test]
    fn retriever_paginates_when_overflowing() {
        let g = build_graph();
        let q = EvidenceQuery {
            budget: RetrievalBudget {
                max_nodes: 2,
                max_edges: 1,
                max_refinement_nodes: 0,
                max_refinement_edges: 0,
            },
            ..EvidenceQuery::default()
        };
        let page = Retriever::new().execute(&g, &q).unwrap();
        assert_eq!(page.result.nodes.len(), 2);
        assert!(page.cursor.is_some());
        assert!(page.result.has_more);
    }

    #[test]
    fn retriever_refines_inside_window() {
        let mut g = EvidenceGraph::new("g", "proj".to_string(), 0);
        g.append_node(node("shot/0", crate::graph::EvidenceKind::Shot))
            .unwrap();
        g.append_node(node("shot/1", crate::graph::EvidenceKind::Shot))
            .unwrap();
        g.append_edge(edge("shot/0", "shot/1", crate::graph::EdgeKind::Overlaps))
            .unwrap();
        let q = EvidenceQuery {
            scope: EvidenceScope::UnderNode("shot/0".to_string()),
            refine: true,
            ..EvidenceQuery::default()
        };
        let page = Retriever::new().execute(&g, &q).unwrap();
        let ids = page.all_ids();
        assert!(ids.contains(&"shot/0".to_string()));
        // No children → refinement empty.
        assert!(page.result.refinement_nodes.is_empty());
    }
}