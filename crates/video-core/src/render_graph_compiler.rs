//!
//! Render-graph compiler (CR-V2-B5-021).
//!
//! Compiles a `RenderGraph` into a deterministic `CompiledPlan`. The
//! compiler **rejects any graph that references the legacy renderer
//! names** `remotion`, `hyperframes`, or `hyper-frames` — Book 5 freezes
//! the native path as the only active render graph.
//!
//! The compiler is a minimal-but-compiling stub. It validates the graph
//! and produces a topologically ordered plan. The actual GPU-side
//! binding is wired in the native renderer crate.

use crate::native_compositor::{NativeCompositor, RenderGraph, RenderNode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderGraphCompileError {
    #[error("legacy renderer name referenced: {0}")]
    LegacyRenderer(String),
    #[error("compositor error: {0}")]
    Compositor(#[from] crate::native_compositor::CompositorError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledPlan {
    pub id: String,
    pub version: String,
    pub graph_id: String,
    pub steps: Vec<CompiledStep>,
    pub metrics: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledStep {
    pub node_id: String,
    pub op: String,
    pub inputs: Vec<String>,
    pub params: BTreeMap<String, String>,
}

pub struct RenderGraphCompiler;

impl RenderGraphCompiler {
    pub fn legacy_renderers() -> &'static [&'static str] {
        &["remotion", "hyperframes", "hyper-frames"]
    }

    pub fn reject_legacy(graph: &RenderGraph) -> Result<(), RenderGraphCompileError> {
        for n in &graph.nodes {
            for v in n.props.values() {
                let lower = v.to_ascii_lowercase();
                if Self::legacy_renderers().iter().any(|r| lower.contains(r)) {
                    return Err(RenderGraphCompileError::LegacyRenderer(n.id.clone()));
                }
            }
            for inp in &n.inputs {
                let lower = inp.to_ascii_lowercase();
                if Self::legacy_renderers().iter().any(|r| lower == *r) {
                    return Err(RenderGraphCompileError::LegacyRenderer(n.id.clone()));
                }
            }
        }
        Ok(())
    }

    pub fn compile(graph: &RenderGraph) -> Result<CompiledPlan, RenderGraphCompileError> {
        Self::reject_legacy(graph)?;
        let cmds = NativeCompositor::compile(graph)?;
        let steps: Vec<CompiledStep> = cmds
            .into_iter()
            .map(|c| CompiledStep {
                node_id: c.id,
                op: c.op,
                inputs: c.inputs,
                params: c.params,
            })
            .collect();
        Ok(CompiledPlan {
            id: format!("compiled_{}", graph.id),
            version: "v2".to_string(),
            graph_id: graph.id.clone(),
            steps,
            metrics: BTreeMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_compositor::{NodeKind, RenderGraph, RenderNode};

    fn clean_graph() -> RenderGraph {
        RenderGraph {
            id: "rg_1".to_string(),
            version: "v2".to_string(),
            nodes: vec![
                RenderNode {
                    id: "src".to_string(),
                    kind: NodeKind::Source,
                    inputs: vec![],
                    props: BTreeMap::new(),
                },
                RenderNode {
                    id: "out".to_string(),
                    kind: NodeKind::Output,
                    inputs: vec!["src".to_string()],
                    props: BTreeMap::new(),
                },
            ],
        }
    }

    #[test]
    fn compiles_clean_graph() {
        let plan = RenderGraphCompiler::compile(&clean_graph()).expect("ok");
        assert_eq!(plan.steps.len(), 2);
    }

    #[test]
    fn rejects_legacy_renderer_name() {
        let mut g = clean_graph();
        g.nodes[0]
            .props
            .insert("renderer".to_string(), "remotion".to_string());
        let err = RenderGraphCompiler::compile(&g).err().expect("err");
        assert!(matches!(err, RenderGraphCompileError::LegacyRenderer(_)));
    }

    #[test]
    fn rejects_hyperframes_input() {
        let mut g = clean_graph();
        g.nodes[1].inputs.push("hyperframes".to_string());
        let err = RenderGraphCompiler::compile(&g).err().expect("err");
        assert!(matches!(err, RenderGraphCompileError::LegacyRenderer(_)));
    }
}
