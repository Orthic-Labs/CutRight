//!
//! CutRight native GPU and vector compositor (CR-V2-B5-017).
//!
//! The compositor is a **declarative** layer that takes a `RenderGraph`
//! and produces a list of composite commands. The graph must not contain
//! any `JavaScript`, `Html`, `Css`, or `fetch` node kind — the
//! determinism gate (see `V2-CRITIC-SEMANTICS.md`) rejects those.
//!
//! This module is a minimal-but-compiling stub. The full GPU and vector
//! runtime is implemented behind the `CR-V2-B5-021` render-graph compiler
//! but the data shapes are frozen here.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompositorError {
    #[error("forbidden node kind: {0}")]
    ForbiddenNode(String),
    #[error("graph has a cycle: node={0}")]
    Cycle(String),
    #[error("unknown node id: {0}")]
    UnknownNode(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Source,
    Transform,
    Mask,
    Text,
    Vector,
    Image,
    Video,
    Transition,
    Caption,
    Color,
    Audio,
    Composite,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderNode {
    pub id: String,
    pub kind: NodeKind,
    pub inputs: Vec<String>,
    pub props: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderGraph {
    pub id: String,
    pub version: String,
    pub nodes: Vec<RenderNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeCommand {
    pub id: String,
    pub op: String,
    pub inputs: Vec<String>,
    pub params: BTreeMap<String, String>,
}

pub struct NativeCompositor;

impl NativeCompositor {
    pub fn forbidden_kinds() -> &'static [&'static str] {
        &["javascript", "html", "css", "fetch", "executable"]
    }

    pub fn validate(graph: &RenderGraph) -> Result<(), CompositorError> {
        for n in &graph.nodes {
            let kind_str = format!("{:?}", n.kind).to_ascii_lowercase();
            if Self::forbidden_kinds().iter().any(|k| kind_str == *k) {
                return Err(CompositorError::ForbiddenNode(n.id.clone()));
            }
        }
        // Cycle detection via DFS coloring.
        let mut state: BTreeMap<String, u8> = BTreeMap::new();
        for n in &graph.nodes {
            if !state.contains_key(n.id.as_str()) {
                if let Some(back) = Self::dfs(n.id.as_str(), graph, &mut state) {
                    return Err(CompositorError::Cycle(back));
                }
            }
        }
        Ok(())
    }

    fn dfs(start: &str, graph: &RenderGraph, state: &mut BTreeMap<String, u8>) -> Option<String> {
        // iterative style with explicit stack to avoid recursion limits
        let mut stack: Vec<(String, usize)> = vec![(start.to_string(), 0)];
        state.insert(start.to_string(), 1);
        while let Some((node, idx)) = stack.last().cloned() {
            let cur = graph.nodes.iter().find(|n| n.id == node)?;
            if idx >= cur.inputs.len() {
                state.insert(node.clone(), 2);
                stack.pop();
                continue;
            }
            let next = cur.inputs[idx].clone();
            match state.get(next.as_str()).copied() {
                Some(1) => return Some(next),
                Some(2) => {}
                Some(_) => {}
                None => {
                    state.insert(next.clone(), 1);
                    stack.last_mut().unwrap().1 = idx + 1;
                    stack.push((next, 0));
                    continue;
                }
            }
            stack.last_mut().unwrap().1 = idx + 1;
        }
        None
    }

    pub fn compile(graph: &RenderGraph) -> Result<Vec<CompositeCommand>, CompositorError> {
        Self::validate(graph)?;
        let mut out = Vec::new();
        let mut sorted: Vec<&RenderNode> = Vec::new();
        let mut emitted: BTreeMap<String, bool> = BTreeMap::new();
        for n in &graph.nodes {
            Self::visit(n, graph, &mut emitted, &mut sorted)?;
        }
        for n in sorted {
            out.push(CompositeCommand {
                id: n.id.clone(),
                op: format!("{:?}", n.kind).to_ascii_lowercase(),
                inputs: n.inputs.clone(),
                params: n.props.clone(),
            });
        }
        Ok(out)
    }

    fn visit<'a>(
        node: &'a RenderNode,
        graph: &'a RenderGraph,
        emitted: &mut BTreeMap<String, bool>,
        sorted: &mut Vec<&'a RenderNode>,
    ) -> Result<(), CompositorError> {
        if emitted.get(node.id.as_str()).copied().unwrap_or(false) {
            return Ok(());
        }
        for input in &node.inputs {
            let n = graph
                .nodes
                .iter()
                .find(|n| n.id == *input)
                .ok_or_else(|| CompositorError::UnknownNode(input.clone()))?;
            Self::visit(n, graph, emitted, sorted)?;
        }
        emitted.insert(node.id.clone(), true);
        sorted.push(node);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_graph() -> RenderGraph {
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
    fn validates_simple_graph() {
        NativeCompositor::validate(&simple_graph()).expect("ok");
    }

    #[test]
    fn compiles_to_topological_order() {
        let cmds = NativeCompositor::compile(&simple_graph()).expect("ok");
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].id, "src");
        assert_eq!(cmds[1].id, "out");
    }

    #[test]
    fn rejects_cycle() {
        let graph = RenderGraph {
            id: "rg_1".to_string(),
            version: "v2".to_string(),
            nodes: vec![
                RenderNode {
                    id: "a".to_string(),
                    kind: NodeKind::Transform,
                    inputs: vec!["b".to_string()],
                    props: BTreeMap::new(),
                },
                RenderNode {
                    id: "b".to_string(),
                    kind: NodeKind::Transform,
                    inputs: vec!["a".to_string()],
                    props: BTreeMap::new(),
                },
            ],
        };
        let err = NativeCompositor::validate(&graph).err().expect("err");
        assert!(matches!(err, CompositorError::Cycle(_)));
    }
}
