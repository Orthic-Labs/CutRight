//! Hierarchical evidence graph (CR-V2-B3-019).
//!
//! The graph is the typed spine of the evidence store: nodes carry the
//! evidence kind, identity, source revision, time range, confidence and
//! producer; edges carry the typed relation between nodes. The graph is
//! content-addressed — [`EvidenceGraph::graph_hash`] is the BLAKE3 digest
//! of the canonicalised body, so any mutation in the body is detectable
//! in a single comparison.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::tracks::{RationalRange, SourceHash};

/// Every evidence kind the store understands. Mirrors the schema frozen
/// by `schemas/evidence/node.schema.v1.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Source,
    Scene,
    Shot,
    VisualEvent,
    Frame,
    Face,
    Subject,
    Pose,
    Gesture,
    TextRegion,
    MotionRegion,
    AudioStream,
    SpeakerTurn,
    Utterance,
    Word,
    SpeechRegion,
    MusicSection,
    Bar,
    Beat,
    Transient,
    EditorialBeat,
    Claim,
    Asset,
}

impl EvidenceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EvidenceKind::Source => "source",
            EvidenceKind::Scene => "scene",
            EvidenceKind::Shot => "shot",
            EvidenceKind::VisualEvent => "visual_event",
            EvidenceKind::Frame => "frame",
            EvidenceKind::Face => "face",
            EvidenceKind::Subject => "subject",
            EvidenceKind::Pose => "pose",
            EvidenceKind::Gesture => "gesture",
            EvidenceKind::TextRegion => "text_region",
            EvidenceKind::MotionRegion => "motion_region",
            EvidenceKind::AudioStream => "audio_stream",
            EvidenceKind::SpeakerTurn => "speaker_turn",
            EvidenceKind::Utterance => "utterance",
            EvidenceKind::Word => "word",
            EvidenceKind::SpeechRegion => "speech_region",
            EvidenceKind::MusicSection => "music_section",
            EvidenceKind::Bar => "bar",
            EvidenceKind::Beat => "beat",
            EvidenceKind::Transient => "transient",
            EvidenceKind::EditorialBeat => "editorial_beat",
            EvidenceKind::Claim => "claim",
            EvidenceKind::Asset => "asset",
        }
    }
}

/// Stable evidence identity. The string is content-derived (BLAKE3 hex
/// prefix over the canonicalised node payload) so the same physical
/// evidence on a re-run resolves to the same id.
pub type EvidenceId = String;

/// A revision ID — every source, project, and pack carries one. Stored
/// verbatim, never parsed.
pub type RevisionId = String;

/// Receipt reference — opaque token the run receipts use to bind evidence
/// to the run that produced it.
pub type ReceiptRef = String;

/// The producer identity: the capability name + version + parameter hash
/// that produced the evidence.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProducerIdentity {
    pub capability: String,
    pub version: String,
    pub parameter_hash: [u8; 32],
    pub pack_id: String,
}

impl ProducerIdentity {
    pub fn new(capability: &str, version: &str, pack_id: &str, parameter_hash: [u8; 32]) -> Self {
        Self {
            capability: capability.to_string(),
            version: version.to_string(),
            parameter_hash,
            pack_id: pack_id.to_string(),
        }
    }
}

/// A single evidence node. The receipt reference is the only mutable
/// field on the node: it is set when the node is appended to a graph
/// and never updated afterwards.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvidenceNode {
    pub id: EvidenceId,
    pub kind: EvidenceKind,
    pub source_revision: RevisionId,
    pub range: Option<RationalRange>,
    pub confidence_milli: u32,
    pub producer: ProducerIdentity,
    pub receipt: Option<ReceiptRef>,
    pub source_hash: Option<SourceHash>,
}

impl EvidenceNode {
    /// Compute the BLAKE3 hash of the canonicalised node body. The hash
    /// does not depend on the receipt field, so an un-receipted node and
    /// a receipted node with the same payload collapse to the same
    /// digest — the receipt is a separate binding concern.
    pub fn payload_fingerprint(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.kind.as_str().as_bytes());
        hasher.update(self.source_revision.as_bytes());
        if let Some(range) = &self.range {
            hasher.update(&range.start.numerator.to_le_bytes());
            hasher.update(&range.start.denominator.to_le_bytes());
            hasher.update(&range.end.numerator.to_le_bytes());
            hasher.update(&range.end.denominator.to_le_bytes());
        }
        hasher.update(&self.confidence_milli.to_le_bytes());
        hasher.update(self.producer.capability.as_bytes());
        hasher.update(self.producer.version.as_bytes());
        hasher.update(&self.producer.parameter_hash);
        hasher.update(self.producer.pack_id.as_bytes());
        if let Some(h) = &self.source_hash {
            hasher.update(h);
        }
        let hash = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(hash.as_bytes());
        out
    }

    /// Compose the canonical identity string from the payload fingerprint.
    pub fn derive_id(&self) -> EvidenceId {
        let fp = self.payload_fingerprint();
        let hex = blake3::Hash::from_bytes(fp).to_hex();
        let short = &hex.as_str()[..16];
        format!("{}/{}", self.kind.as_str(), short)
    }

    /// Validate that the node carries a usable identity, source revision
    /// and confidence range. Returns the validation error if not.
    pub fn validate(&self) -> Result<(), GraphError> {
        if self.source_revision.is_empty() {
            return Err(GraphError::MissingSourceRevision);
        }
        if self.confidence_milli > 1000 {
            return Err(GraphError::InvalidConfidence);
        }
        if let Some(range) = &self.range {
            if !(range.start < range.end) {
                return Err(GraphError::InvalidRange);
            }
        }
        Ok(())
    }
}

/// Every typed edge relation the schema knows about. Symmetric relations
/// are allowed to form graph cycles; non-symmetric ones may not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Contains,
    Overlaps,
    Supports,
    Contradicts,
    DerivedFrom,
    SameSubject,
    SameTake,
    SpokenBy,
    Visualises,
    SynchronisedWith,
}

impl EdgeKind {
    pub fn is_symmetric(&self) -> bool {
        matches!(
            self,
            EdgeKind::Overlaps
                | EdgeKind::Supports
                | EdgeKind::Contradicts
                | EdgeKind::SameSubject
                | EdgeKind::SameTake
                | EdgeKind::SynchronisedWith
        )
    }
}

/// One edge in the graph. Endpoints are node IDs; the relation must be
/// one of [`EdgeKind`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvidenceEdge {
    pub from: EvidenceId,
    pub to: EvidenceId,
    pub kind: EdgeKind,
    pub confidence_milli: u32,
}

impl EvidenceEdge {
    pub fn validate(&self) -> Result<(), GraphError> {
        if self.from == self.to {
            return Err(GraphError::SelfLoop);
        }
        if self.confidence_milli > 1000 {
            return Err(GraphError::InvalidConfidence);
        }
        Ok(())
    }
}

/// The graph object. Every field is required and validated on append.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceGraph {
    pub graph_id: String,
    pub project_revision: RevisionId,
    pub source_revisions: BTreeSet<RevisionId>,
    pub nodes: Vec<EvidenceNode>,
    pub edges: Vec<EvidenceEdge>,
    pub created_at_ns: u64,
    /// BLAKE3 of the canonical body; computed when the graph is built.
    pub graph_hash: String,
}

impl EvidenceGraph {
    /// Construct an empty graph for the given project revision. Nodes
    /// and edges are appended through [`Self::append_node`] and
    /// [`Self::append_edge`], both of which keep the body canonical.
    pub fn new(graph_id: &str, project_revision: RevisionId, created_at_ns: u64) -> Self {
        Self {
            graph_id: graph_id.to_string(),
            project_revision,
            source_revisions: BTreeSet::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            created_at_ns,
            graph_hash: String::new(),
        }
    }

    /// Append a node. The graph is rebuilt incrementally so callers do
    /// not need to re-sort the body themselves.
    pub fn append_node(&mut self, node: EvidenceNode) -> Result<(), GraphError> {
        node.validate()?;
        if self.nodes.iter().any(|n| n.id == node.id) {
            return Err(GraphError::DuplicateNodeId(node.id));
        }
        self.source_revisions.insert(node.source_revision.clone());
        self.nodes.push(node);
        self.rehash();
        Ok(())
    }

    /// Append an edge.
    pub fn append_edge(&mut self, edge: EvidenceEdge) -> Result<(), GraphError> {
        edge.validate()?;
        if !self.nodes.iter().any(|n| n.id == edge.from) {
            return Err(GraphError::UnknownEndpoint(edge.from));
        }
        if !self.nodes.iter().any(|n| n.id == edge.to) {
            return Err(GraphError::UnknownEndpoint(edge.to));
        }
        if self.edges.iter().any(|e| {
            e.from == edge.from && e.to == edge.to && e.kind == edge.kind
        }) {
            return Err(GraphError::DuplicateEdge {
                from: edge.from,
                to: edge.to,
                kind: edge.kind,
            });
        }
        self.edges.push(edge);
        self.rehash();
        Ok(())
    }

    /// Return nodes of the requested kind, in insertion order.
    pub fn nodes_of_kind(&self, kind: EvidenceKind) -> Vec<&EvidenceNode> {
        self.nodes.iter().filter(|n| n.kind == kind).collect()
    }

    /// Return edges incident to a node.
    pub fn edges_incident(&self, node_id: &str) -> Vec<&EvidenceEdge> {
        self.edges
            .iter()
            .filter(|e| e.from == node_id || e.to == node_id)
            .collect()
    }

    /// Recompute the graph hash from the current body. Called
    /// automatically after appending; callers may invoke directly when
    /// they mutate the body through other paths.
    pub fn rehash(&mut self) {
        let mut sorted_nodes: Vec<&EvidenceNode> = self.nodes.iter().collect();
        sorted_nodes.sort_by(|a, b| a.id.cmp(&b.id));
        let mut sorted_edges: Vec<&EvidenceEdge> = self.edges.iter().collect();
        sorted_edges.sort_by(|a, b| {
            (&a.from, &a.to, a.kind as u8).cmp(&(&b.from, &b.to, b.kind as u8))
        });
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.graph_id.as_bytes());
        hasher.update(self.project_revision.as_bytes());
        for rev in &self.source_revisions {
            hasher.update(rev.as_bytes());
        }
        for n in &sorted_nodes {
            hasher.update(n.id.as_bytes());
            hasher.update(&n.payload_fingerprint());
        }
        for e in &sorted_edges {
            hasher.update(e.from.as_bytes());
            hasher.update(e.to.as_bytes());
            hasher.update(&[e.kind as u8]);
            hasher.update(&e.confidence_milli.to_le_bytes());
        }
        hasher.update(&self.created_at_ns.to_le_bytes());
        let hash = hasher.finalize();
        self.graph_hash = hash.to_hex().to_string();
    }
}

/// The errors the graph can raise on append. They map directly to the
/// schema's `additionalProperties: false` shape.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GraphError {
    #[error("node missing source revision")]
    MissingSourceRevision,
    #[error("node confidence out of range")]
    InvalidConfidence,
    #[error("node time range is empty or inverted")]
    InvalidRange,
    #[error("self-loop edges are forbidden")]
    SelfLoop,
    #[error("duplicate node id: {0}")]
    DuplicateNodeId(EvidenceId),
    #[error("unknown endpoint: {0}")]
    UnknownEndpoint(EvidenceId),
    #[error("duplicate edge from {from} to {to} of kind {kind:?}")]
    DuplicateEdge {
        from: EvidenceId,
        to: EvidenceId,
        kind: EdgeKind,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn producer() -> ProducerIdentity {
        ProducerIdentity::new("vision.face-track", "0.1.0", "vision", [1u8; 32])
    }

    fn node(id: &str, kind: EvidenceKind) -> EvidenceNode {
        EvidenceNode {
            id: id.to_string(),
            kind,
            source_revision: "rev-1".to_string(),
            range: None,
            confidence_milli: 900,
            producer: producer(),
            receipt: None,
            source_hash: None,
        }
    }

    #[test]
    fn empty_graph_has_empty_hash() {
        let g = EvidenceGraph::new("g", "proj".to_string(), 0);
        assert!(g.graph_hash.is_empty());
    }

    #[test]
    fn append_node_recomputes_hash() {
        let mut g = EvidenceGraph::new("g", "proj".to_string(), 0);
        g.append_node(node("n1", EvidenceKind::Frame)).unwrap();
        assert!(!g.graph_hash.is_empty());
        let g2 = g.clone();
        assert_eq!(g.graph_hash, g2.graph_hash);
    }

    #[test]
    fn identical_graphs_have_identical_hashes() {
        let mut g1 = EvidenceGraph::new("g", "proj".to_string(), 0);
        let mut g2 = EvidenceGraph::new("g", "proj".to_string(), 0);
        g1.append_node(node("a", EvidenceKind::Frame)).unwrap();
        g1.append_node(node("b", EvidenceKind::Shot)).unwrap();
        g2.append_node(node("b", EvidenceKind::Shot)).unwrap();
        g2.append_node(node("a", EvidenceKind::Frame)).unwrap();
        assert_eq!(g1.graph_hash, g2.graph_hash);
    }

    #[test]
    fn appending_unknown_endpoint_fails() {
        let mut g = EvidenceGraph::new("g", "proj".to_string(), 0);
        g.append_node(node("a", EvidenceKind::Frame)).unwrap();
        let edge = EvidenceEdge {
            from: "a".to_string(),
            to: "missing".to_string(),
            kind: EdgeKind::Contains,
            confidence_milli: 1000,
        };
        assert_eq!(
            g.append_edge(edge),
            Err(GraphError::UnknownEndpoint("missing".to_string()))
        );
    }

    #[test]
    fn self_loop_is_rejected() {
        let mut g = EvidenceGraph::new("g", "proj".to_string(), 0);
        g.append_node(node("a", EvidenceKind::Frame)).unwrap();
        let edge = EvidenceEdge {
            from: "a".to_string(),
            to: "a".to_string(),
            kind: EdgeKind::Contains,
            confidence_milli: 1000,
        };
        assert_eq!(g.append_edge(edge), Err(GraphError::SelfLoop));
    }

    #[test]
    fn duplicate_node_id_is_rejected() {
        let mut g = EvidenceGraph::new("g", "proj".to_string(), 0);
        g.append_node(node("a", EvidenceKind::Frame)).unwrap();
        assert!(matches!(
            g.append_node(node("a", EvidenceKind::Frame)),
            Err(GraphError::DuplicateNodeId(_))
        ));
    }
}