//! Integration tests for the hierarchical evidence graph
//! (CR-V2-B3-019).

use video_evidence::graph::{EvidenceEdge, EvidenceGraph, EvidenceKind, EdgeKind, ProducerIdentity};
use video_evidence::index::EvidenceIndex;
use video_evidence::store::EvidenceStore;

fn producer() -> ProducerIdentity {
    ProducerIdentity::new("vision.face-track", "0.1.0", "vision", [1u8; 32])
}

fn node(id: &str, kind: EvidenceKind) -> video_evidence::graph::EvidenceNode {
    video_evidence::graph::EvidenceNode {
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

fn edge(from: &str, to: &str, kind: EdgeKind) -> EvidenceEdge {
    EvidenceEdge {
        from: from.to_string(),
        to: to.to_string(),
        kind,
        confidence_milli: 1000,
    }
}

#[test]
fn canonical_objects_survive_index_deletion() {
    let dir = tempfile::tempdir().unwrap();
    let store = EvidenceStore::open(dir.path()).unwrap();
    store.put_node(&node("frame/a", EvidenceKind::Frame)).unwrap();
    store.put_node(&node("shot/a", EvidenceKind::Shot)).unwrap();
    store
        .put_edge(&edge("shot/a", "frame/a", EdgeKind::Contains))
        .unwrap();
    let mut g = EvidenceGraph::new("g", "proj".to_string(), 0);
    g.append_node(node("frame/a", EvidenceKind::Frame)).unwrap();
    g.append_node(node("shot/a", EvidenceKind::Shot)).unwrap();
    g.append_edge(edge("shot/a", "frame/a", EdgeKind::Contains))
        .unwrap();
    store.put_graph(&g).unwrap();
    let snap1 = store.rebuild_index().unwrap();
    assert_eq!(snap1.node_ids.len(), 2);
    // Delete and re-build. The timestamps will differ but the body
    // content is identical — the canonical objects survived.
    let _ = std::fs::remove_dir_all(dir.path().join("index"));
    let snap2 = store.rebuild_index().unwrap();
    assert_eq!(snap1.node_ids, snap2.node_ids);
    assert_eq!(snap1.edge_signatures, snap2.edge_signatures);
    assert_eq!(snap1.graph_hashes, snap2.graph_hashes);
}

#[test]
fn tampered_node_fails_read_verification() {
    let dir = tempfile::tempdir().unwrap();
    let store = EvidenceStore::open(dir.path()).unwrap();
    let n = node("frame/a", EvidenceKind::Frame);
    let hash = store.put_node(&n).unwrap();
    let path = dir.path().join("objects").join(format!("{hash}.json"));
    std::fs::write(&path, b"{}").unwrap();
    let result = store.read_node(&hash);
    assert!(result.is_err());
}

#[test]
fn graph_cycles_are_allowed_only_for_symmetric_relations() {
    assert!(EdgeKind::Overlaps.is_symmetric());
    assert!(EdgeKind::SynchronisedWith.is_symmetric());
    assert!(!EdgeKind::Contains.is_symmetric());
    assert!(!EdgeKind::DerivedFrom.is_symmetric());
}

#[test]
fn mutable_database_rows_are_not_canonical_truth() {
    let dir = tempfile::tempdir().unwrap();
    let store = EvidenceStore::open(dir.path()).unwrap();
    let n = node("frame/a", EvidenceKind::Frame);
    let h = store.put_node(&n).unwrap();
    // Removing every non-canonical file should not invalidate the store.
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() != "objects" {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
    let snap = store.rebuild_index().unwrap();
    assert_eq!(snap.node_ids.len(), 1);
    let _ = h; // touch to suppress unused
}

#[test]
fn index_survives_rebuild_across_passes() {
    let dir = tempfile::tempdir().unwrap();
    let store = EvidenceStore::open(dir.path()).unwrap();
    for i in 0..5 {
        store
            .put_node(&node(&format!("frame/{i}"), EvidenceKind::Frame))
            .unwrap();
    }
    let idx1 = EvidenceIndex::rebuild(&store).unwrap();
    assert_eq!(idx1.node_count(), 5);
    let snap = idx1.snapshot();
    let idx2 = EvidenceIndex::from_snapshot(snap);
    assert_eq!(idx1.node_count(), idx2.node_count());
}

#[test]
fn append_node_validates_inputs() {
    let mut g = EvidenceGraph::new("g", "proj".to_string(), 0);
    let mut n = node("frame/a", EvidenceKind::Frame);
    n.confidence_milli = 1500;
    assert!(g.append_node(n).is_err());
    let mut n = node("frame/b", EvidenceKind::Frame);
    n.source_revision = String::new();
    assert!(g.append_node(n).is_err());
}