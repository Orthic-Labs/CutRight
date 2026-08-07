//! Rebuildable evidence index (CR-V2-B3-019).
//!
//! The index is a SQLite-shaped view of the store's canonical objects.
//! The actual storage is content-addressed JSON; this module is the
//! in-memory representation the rest of the crate queries. It is
//! disposable — deleting it does not destroy evidence; rebuilding it is
//! a single pass over `objects/`.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::graph::{EvidenceEdge, EvidenceGraph, EvidenceKind, EvidenceNode};
use crate::store::{EvidenceStore, IndexSnapshot, StoreError};

/// Lookup helpers built from an [`IndexSnapshot`]. The index is read-only
/// from this layer's perspective; mutations go through the store.
#[derive(Debug, Clone, Default)]
pub struct EvidenceIndex {
    by_id: BTreeMap<String, IndexedNode>,
    edges: Vec<IndexedEdge>,
    graphs: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedNode {
    pub id: String,
    pub kind: EvidenceKind,
    pub source_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedEdge {
    pub from: String,
    pub to: String,
    pub kind: crate::graph::EdgeKind,
}

impl EvidenceIndex {
    /// Build the index from a freshly-rebuilt snapshot. The snapshot is
    /// the only state the index holds; rebuilding it re-reads the
    /// canonical objects from disk.
    pub fn from_snapshot(snap: IndexSnapshot) -> Self {
        let mut by_id = BTreeMap::new();
        for id in &snap.node_ids {
            // The snapshot carries only IDs; the kind prefix is encoded
            // in the id. We split on `/` and look the prefix up in
            // `EvidenceKind::as_str`.
            let kind = id
                .split_once('/')
                .and_then(|(prefix, _)| EvidenceKind::from_str(prefix))
                .unwrap_or(EvidenceKind::Source);
            by_id.insert(
                id.clone(),
                IndexedNode {
                    id: id.clone(),
                    kind,
                    source_revision: String::new(),
                },
            );
        }
        let mut edges = Vec::new();
        for sig in &snap.edge_signatures {
            // signature format: from->to:kind
            if let Some((ft, kind_part)) = sig.split_once("->") {
                if let Some((to, kind_name)) = kind_part.split_once(':') {
                    if let Some(kind) = crate::graph::EdgeKind::from_str(kind_name) {
                        edges.push(IndexedEdge {
                            from: ft.to_string(),
                            to: to.to_string(),
                            kind,
                        });
                    }
                }
            }
        }
        Self {
            by_id,
            edges,
            graphs: snap.graph_hashes,
        }
    }

    /// Convenience: open the store, rebuild the snapshot, and build the
    /// index.
    pub fn rebuild(store: &EvidenceStore) -> Result<Self, StoreError> {
        let snap = store.rebuild_index()?;
        Ok(Self::from_snapshot(snap))
    }

    /// Whether a node with the given id is present in the index.
    pub fn contains(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    /// Return the kind of the node with the given id, if known.
    pub fn kind_of(&self, id: &str) -> Option<EvidenceKind> {
        self.by_id.get(id).map(|n| n.kind)
    }

    /// Return every node id of the requested kind.
    pub fn nodes_of_kind(&self, kind: EvidenceKind) -> Vec<String> {
        self.by_id
            .values()
            .filter(|n| n.kind == kind)
            .map(|n| n.id.clone())
            .collect()
    }

    /// Return every edge incident to the node id, in insertion order.
    pub fn edges_incident(&self, id: &str) -> Vec<IndexedEdge> {
        self.edges
            .iter()
            .filter(|e| e.from == id || e.to == id)
            .cloned()
            .collect()
    }

    /// Total node count.
    pub fn node_count(&self) -> usize {
        self.by_id.len()
    }

    /// Total edge count.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Number of stored graphs referenced by the index.
    pub fn graph_count(&self) -> usize {
        self.graphs.len()
    }

    /// A round-trippable snapshot. Lossy on `source_revision` because the
    /// snapshot keeps only ids.
    pub fn snapshot(&self) -> IndexSnapshot {
        IndexSnapshot {
            schema: "cutright.evidence_index/v1".to_string(),
            rebuilt_at_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0),
            node_ids: self.by_id.keys().cloned().collect(),
            edge_signatures: self
                .edges
                .iter()
                .map(|e| format!("{}->{}:{:?}", e.from, e.to, e.kind))
                .collect(),
            graph_hashes: self.graphs.clone(),
        }
    }

    /// Merge another [`EvidenceNode`] into the index. The node is not
    /// validated here — the store has already validated it.
    pub fn index_node(&mut self, node: &EvidenceNode) {
        self.by_id.insert(
            node.id.clone(),
            IndexedNode {
                id: node.id.clone(),
                kind: node.kind,
                source_revision: node.source_revision.clone(),
            },
        );
    }

    /// Merge another [`EvidenceEdge`] into the index.
    pub fn index_edge(&mut self, edge: &EvidenceEdge) {
        self.edges.push(IndexedEdge {
            from: edge.from.clone(),
            to: edge.to.clone(),
            kind: edge.kind,
        });
    }

    /// Merge the contents of an [`EvidenceGraph`] into the index.
    pub fn index_graph(&mut self, graph: &EvidenceGraph) {
        for n in &graph.nodes {
            self.index_node(n);
        }
        for e in &graph.edges {
            self.index_edge(e);
        }
        self.graphs.insert(graph.graph_hash.clone());
    }
}

impl EvidenceKind {
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "source" => Self::Source,
            "scene" => Self::Scene,
            "shot" => Self::Shot,
            "visual_event" => Self::VisualEvent,
            "frame" => Self::Frame,
            "face" => Self::Face,
            "subject" => Self::Subject,
            "pose" => Self::Pose,
            "gesture" => Self::Gesture,
            "text_region" => Self::TextRegion,
            "motion_region" => Self::MotionRegion,
            "audio_stream" => Self::AudioStream,
            "speaker_turn" => Self::SpeakerTurn,
            "utterance" => Self::Utterance,
            "word" => Self::Word,
            "speech_region" => Self::SpeechRegion,
            "music_section" => Self::MusicSection,
            "bar" => Self::Bar,
            "beat" => Self::Beat,
            "transient" => Self::Transient,
            "editorial_beat" => Self::EditorialBeat,
            "claim" => Self::Claim,
            "asset" => Self::Asset,
            _ => return None,
        })
    }
}

impl crate::graph::EdgeKind {
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "Contains" => Self::Contains,
            "Overlaps" => Self::Overlaps,
            "Supports" => Self::Supports,
            "Contradicts" => Self::Contradicts,
            "DerivedFrom" => Self::DerivedFrom,
            "SameSubject" => Self::SameSubject,
            "SameTake" => Self::SameTake,
            "SpokenBy" => Self::SpokenBy,
            "Visualises" => Self::Visualises,
            "SynchronisedWith" => Self::SynchronisedWith,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::ProducerIdentity;

    fn n(id: &str, kind: EvidenceKind) -> EvidenceNode {
        EvidenceNode {
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

    #[test]
    fn round_trip_index_preserves_node_count() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvidenceStore::open(dir.path()).unwrap();
        store.put_node(&n("frame/abc", EvidenceKind::Frame)).unwrap();
        store
            .put_node(&n("shot/abc", EvidenceKind::Shot))
            .unwrap();
        let idx = EvidenceIndex::rebuild(&store).unwrap();
        assert_eq!(idx.node_count(), 2);
        let snap = idx.snapshot();
        let idx2 = EvidenceIndex::from_snapshot(snap);
        assert_eq!(idx.node_count(), idx2.node_count());
    }

    #[test]
    fn nodes_of_kind_filters_correctly() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvidenceStore::open(dir.path()).unwrap();
        store.put_node(&n("frame/abc", EvidenceKind::Frame)).unwrap();
        store
            .put_node(&n("frame/def", EvidenceKind::Frame))
            .unwrap();
        store
            .put_node(&n("shot/abc", EvidenceKind::Shot))
            .unwrap();
        let idx = EvidenceIndex::rebuild(&store).unwrap();
        assert_eq!(idx.nodes_of_kind(EvidenceKind::Frame).len(), 2);
        assert_eq!(idx.nodes_of_kind(EvidenceKind::Shot).len(), 1);
    }
}