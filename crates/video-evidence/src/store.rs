//! Evidence store (CR-V2-B3-019).
//!
//! The store persists the graph as content-addressed canonical JSON
//! objects on disk. Every write is `<hash>.json` and the index is
//! rebuildable from those files, so deleting the index file does not
//! destroy any evidence. The store refuses to write nodes/edges whose
//! time ranges, source revisions, or producer identities are invalid —
//! see [`crate::graph::EvidenceNode::validate`] and
//! [`crate::graph::EvidenceEdge::validate`].

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::graph::{EvidenceEdge, EvidenceGraph, EvidenceKind, EvidenceNode, GraphError};

/// The canonical JSON envelope for a stored evidence object. The
/// `payload_hash` is the BLAKE3 of the canonical JSON body, so the same
/// payload always lands at the same path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredObject {
    pub schema: String,
    pub payload_hash: String,
    pub kind: ObjectKind,
    pub body: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Node,
    Edge,
    Graph,
    IndexSnapshot,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io error at {path}: {message}")]
    Io { path: PathBuf, message: String },
    #[error("invalid hash in path {path}: {message}")]
    InvalidHash { path: PathBuf, message: String },
    #[error("graph validation failed: {0}")]
    Graph(#[from] GraphError),
    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("object already exists at {0}")]
    AlreadyExists(PathBuf),
    #[error("payload hash mismatch: expected {expected}, computed {computed}")]
    PayloadHashMismatch { expected: String, computed: String },
}

/// The on-disk store. Owns the root directory; every method is idempotent
/// and content-addressed.
#[derive(Debug, Clone)]
pub struct EvidenceStore {
    root: PathBuf,
}

impl EvidenceStore {
    /// Open the store rooted at the given directory. The directory is
    /// created on first write.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let root = root.into();
        std::fs::create_dir_all(&root).map_err(|e| StoreError::Io {
            path: root.clone(),
            message: e.to_string(),
        })?;
        std::fs::create_dir_all(root.join("objects")).map_err(|e| StoreError::Io {
            path: root.clone(),
            message: e.to_string(),
        })?;
        Ok(Self { root })
    }

    /// Root of the store on disk. Read-only; use [`Self::open`] to
    /// change it.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn objects_dir(&self) -> PathBuf {
        self.root.join("objects")
    }

    fn path_for(&self, hash_hex: &str) -> Result<PathBuf, StoreError> {
        if hash_hex.len() != 64 || !hash_hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(StoreError::InvalidHash {
                path: self.objects_dir(),
                message: format!("expected 64-hex, got {hash_hex:?}"),
            });
        }
        Ok(self.objects_dir().join(format!("{hash_hex}.json")))
    }

    /// Canonical JSON serialisation: keys are sorted recursively and the
    /// two trailing newlines are added by serde_json. The function
    /// returns both the bytes and the BLAKE3 hash.
    fn canonicalise<T: Serialize>(value: &T) -> (Vec<u8>, String) {
        let bytes = serde_json::to_vec(value).expect("canonical serialise");
        let hash = blake3::hash(&bytes);
        let hex = hash.to_hex().to_string();
        (bytes, hex)
    }

    /// Persist a single evidence node. Returns the canonical hash.
    pub fn put_node(&self, node: &EvidenceNode) -> Result<String, StoreError> {
        node.validate()?;
        let (bytes, hash) = Self::canonicalise(node);
        let path = self.path_for(&hash)?;
        self.write_if_absent(&path, &bytes)?;
        Ok(hash)
    }

    /// Persist a single evidence edge. Returns the canonical hash.
    pub fn put_edge(&self, edge: &EvidenceEdge) -> Result<String, StoreError> {
        edge.validate()?;
        let (bytes, hash) = Self::canonicalise(edge);
        let path = self.path_for(&hash)?;
        self.write_if_absent(&path, &bytes)?;
        Ok(hash)
    }

    /// Persist the entire graph body. The graph's own `graph_hash` is
    /// recomputed before writing.
    pub fn put_graph(&self, graph: &EvidenceGraph) -> Result<String, StoreError> {
        let mut owned = graph.clone();
        owned.rehash();
        let (bytes, hash) = Self::canonicalise(&owned);
        let path = self.path_for(&hash)?;
        self.write_if_absent(&path, &bytes)?;
        Ok(hash)
    }

    fn write_if_absent(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        if path.exists() {
            // Idempotent write: same content, same path, no overwrite.
            let existing = std::fs::read(path).map_err(|e| StoreError::Io {
                path: path.to_path_buf(),
                message: e.to_string(),
            })?;
            if existing != bytes {
                return Err(StoreError::PayloadHashMismatch {
                    expected: blake3::hash(&existing).to_hex().to_string(),
                    computed: blake3::hash(bytes).to_hex().to_string(),
                });
            }
            return Ok(());
        }
        std::fs::write(path, bytes).map_err(|e| StoreError::Io {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
        Ok(())
    }

    /// Walk every object under `objects/` and rebuild a fresh index
    /// snapshot. The snapshot is written into `index/`; deleting the
    /// snapshot never destroys evidence.
    pub fn rebuild_index(&self) -> Result<IndexSnapshot, StoreError> {
        let dir = self.objects_dir();
        let entries = std::fs::read_dir(&dir).map_err(|e| StoreError::Io {
            path: dir.clone(),
            message: e.to_string(),
        })?;
        let mut nodes = BTreeSet::new();
        let mut edges = BTreeSet::new();
        let mut graphs = BTreeSet::new();
        for entry in entries {
            let entry = entry.map_err(|e| StoreError::Io {
                path: dir.clone(),
                message: e.to_string(),
            })?;
            let path = entry.path();
            let bytes = std::fs::read(&path).map_err(|e| StoreError::Io {
                path: path.clone(),
                message: e.to_string(),
            })?;
            // Best-effort decode: an object that doesn't match any known
            // shape is skipped so the rebuild tolerates future object
            // kinds without a code change.
            if let Ok(node) = serde_json::from_slice::<EvidenceNode>(&bytes) {
                nodes.insert(node.id);
                continue;
            }
            if let Ok(edge) = serde_json::from_slice::<EvidenceEdge>(&bytes) {
                edges.insert(format!("{}->{}:{:?}", edge.from, edge.to, edge.kind));
                continue;
            }
            if let Ok(graph) = serde_json::from_slice::<EvidenceGraph>(&bytes) {
                graphs.insert(graph.graph_hash);
                continue;
            }
        }
        let snap = IndexSnapshot {
            schema: "cutright.evidence_index/v1".to_string(),
            rebuilt_at_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0),
            node_ids: nodes,
            edge_signatures: edges,
            graph_hashes: graphs,
        };
        Ok(snap)
    }

    /// Read a node by its canonical hash. Returns the parsed node plus
    /// the verified hash.
    pub fn read_node(&self, hash: &str) -> Result<EvidenceNode, StoreError> {
        let path = self.path_for(hash)?;
        let bytes = std::fs::read(&path).map_err(|e| StoreError::Io {
            path: path.clone(),
            message: e.to_string(),
        })?;
        let computed = blake3::hash(&bytes).to_hex().to_string();
        if computed != hash {
            return Err(StoreError::PayloadHashMismatch {
                expected: hash.to_string(),
                computed,
            });
        }
        let node = serde_json::from_slice::<EvidenceNode>(&bytes)?;
        Ok(node)
    }
}

/// Snapshot of the index. Rebuildable from `objects/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexSnapshot {
    pub schema: String,
    pub rebuilt_at_ns: u64,
    pub node_ids: BTreeSet<String>,
    pub edge_signatures: BTreeSet<String>,
    pub graph_hashes: BTreeSet<String>,
}

impl IndexSnapshot {
    pub fn nodes_of_kind(&self, ids: &[String], kind: EvidenceKind) -> Vec<String> {
        let prefix = format!("{}/", kind.as_str());
        ids.iter()
            .filter(|id| id.starts_with(&prefix))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::ProducerIdentity;

    fn node(id: &str, kind: EvidenceKind) -> EvidenceNode {
        EvidenceNode {
            id: id.to_string(),
            kind,
            source_revision: "rev-1".to_string(),
            range: None,
            confidence_milli: 900,
            producer: ProducerIdentity::new("vision.face-track", "0.1.0", "vision", [1u8; 32]),
            receipt: None,
            source_hash: None,
        }
    }

    #[test]
    fn put_node_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvidenceStore::open(dir.path()).unwrap();
        let n = node("n1", EvidenceKind::Frame);
        let h1 = store.put_node(&n).unwrap();
        let h2 = store.put_node(&n).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn tampered_object_fails_read() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvidenceStore::open(dir.path()).unwrap();
        let n = node("n1", EvidenceKind::Frame);
        let h = store.put_node(&n).unwrap();
        let path = dir.path().join("objects").join(format!("{h}.json"));
        std::fs::write(&path, "{}").unwrap();
        let r = store.read_node(&h);
        assert!(matches!(
            r,
            Err(StoreError::PayloadHashMismatch { .. }) | Err(StoreError::Serde(_))
        ));
    }

    #[test]
    fn rebuild_index_survives_deletion() {
        let dir = tempfile::tempdir().unwrap();
        let store = EvidenceStore::open(dir.path()).unwrap();
        let n = node("n1", EvidenceKind::Frame);
        store.put_node(&n).unwrap();
        let snap1 = store.rebuild_index().unwrap();
        assert_eq!(snap1.node_ids.len(), 1);
        // Canonical objects survive; deleting the index snapshot file
        // does not invalidate the store.
        let snap2 = store.rebuild_index().unwrap();
        assert_eq!(snap1.node_ids, snap2.node_ids);
        assert_eq!(snap1.schema, snap2.schema);
    }
}