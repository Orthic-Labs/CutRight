//!
//! Writing and packaging copy as evidence-bound skills (CR-V2-B5-010).
//!
//! The writing lane emits **evidence-bound** copy artefacts:
//! - every `CopyAtom` references at least one `evidence_ref`
//! - every `Package` references at least one `CopyAtom` and one evidence
//! - claims that cannot be backed by an evidence_ref are rejected
//!
//! The handwritten owned skill id prefix is `writing.<verb>` or
//! `writing.package.<verb>`. The router resolves them through the
//! `SkillFamily::Writing` handler.

use crate::creative_skill_runtime::{
    SkillFamily, SkillRequest, SkillResult, SkillRuntime, SkillRuntimeError,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WritingError {
    #[error("copy atom requires at least one evidence_ref: id={0}")]
    UnboundAtom(String),
    #[error("package requires at least one copy_atom and one evidence_ref: id={0}")]
    UnboundPackage(String),
    #[error("runtime error: {0}")]
    Runtime(#[from] SkillRuntimeError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyAtom {
    pub id: String,
    pub version: String,
    pub text: String,
    pub kind: String,
    pub evidence_refs: Vec<String>,
    pub restricted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub id: String,
    pub version: String,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub copy_atom_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub struct WritingService {
    _private: (),
}

impl Default for WritingService {
    fn default() -> Self {
        Self::new()
    }
}

impl WritingService {
    pub fn new() -> Self {
        Self { _private: () }
    }

    pub fn register(runtime: &mut SkillRuntime) {
        runtime.register(SkillFamily::Writing, std::sync::Arc::new(Self::handle));
    }

    fn handle(req: &SkillRequest) -> Result<SkillResult, SkillRuntimeError> {
        Ok(SkillResult {
            version: crate::creative_skill_runtime::RUNTIME_VERSION.to_string(),
            skill_id: req.skill_id.clone(),
            output_kind: "writing_artefact".to_string(),
            output_id: format!("wrt_{}", req.input_id),
            content_hash: format!("sha256:writing:{}", req.input_id),
            metrics: BTreeMap::new(),
        })
    }

    pub fn assert_atom_bound(atom: &CopyAtom) -> Result<(), WritingError> {
        if atom.evidence_refs.is_empty() {
            Err(WritingError::UnboundAtom(atom.id.clone()))
        } else {
            Ok(())
        }
    }

    pub fn assert_package_bound(pkg: &Package) -> Result<(), WritingError> {
        if pkg.copy_atom_ids.is_empty() || pkg.evidence_refs.is_empty() {
            Err(WritingError::UnboundPackage(pkg.id.clone()))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_atom_without_evidence() {
        let atom = CopyAtom {
            id: "ca_1".to_string(),
            version: "v2".to_string(),
            text: "hello".to_string(),
            kind: "caption".to_string(),
            evidence_refs: vec![],
            restricted: false,
        };
        let err = WritingService::assert_atom_bound(&atom).err().expect("err");
        assert!(matches!(err, WritingError::UnboundAtom(_)));
    }

    #[test]
    fn accepts_atom_with_evidence() {
        let atom = CopyAtom {
            id: "ca_1".to_string(),
            version: "v2".to_string(),
            text: "hello".to_string(),
            kind: "caption".to_string(),
            evidence_refs: vec!["evidence:ev_1".to_string()],
            restricted: false,
        };
        WritingService::assert_atom_bound(&atom).expect("ok");
    }

    #[test]
    fn rejects_package_without_atoms() {
        let pkg = Package {
            id: "pkg_1".to_string(),
            version: "v2".to_string(),
            title: "T".to_string(),
            description: "D".to_string(),
            tags: vec![],
            copy_atom_ids: vec![],
            evidence_refs: vec!["evidence:ev_1".to_string()],
        };
        let err = WritingService::assert_package_bound(&pkg)
            .err()
            .expect("err");
        assert!(matches!(err, WritingError::UnboundPackage(_)));
    }
}
