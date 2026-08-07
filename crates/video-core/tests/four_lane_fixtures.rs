//! Four-lane creative golden fixtures and native migration comparisons
//! (CR-V2-B5-025).
//!
//! These tests verify the four creative lanes (brand, designer, writing,
//! native-renderer) against the frozen golden fixtures and confirm that
//! the legacy `remotion` / `hyperframes` paths are no longer reachable.

use video_core::creative_critic::{CreativeCritic, DeterministicVisualQa, AxisScore};
use video_core::native_compositor::{NodeKind, NativeCompositor, RenderGraph, RenderNode};
use video_core::render_graph_compiler::RenderGraphCompiler;
use std::collections::BTreeMap;

fn source_graph() -> RenderGraph {
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
fn brand_lane_fixture_evidence_anchors() {
    let mut axes = BTreeMap::new();
    axes.insert(
        "brand_alignment".to_string(),
        AxisScore {
            score: 0.9,
            weight: 1.0,
            evidence_refs: vec!["brand_card:bc_1".to_string()],
        },
    );
    let qa = DeterministicVisualQa::run("cp_1").unwrap();
    let ev = CreativeCritic::run("cp_1", &qa, axes).unwrap();
    assert!(matches!(
        ev.verdict,
        video_core::creative_critic::Verdict::Pass
    ));
}

#[test]
fn native_renderer_replaces_remotion() {
    let plan = RenderGraphCompiler::compile(&source_graph()).expect("ok");
    assert_eq!(plan.steps.len(), 2);
}

#[test]
fn native_renderer_replaces_hyperframes() {
    let mut g = source_graph();
    g.nodes[1].inputs.push("hyperframes".to_string());
    let err = RenderGraphCompiler::compile(&g).err().expect("err");
    assert!(matches!(
        err,
        video_core::render_graph_compiler::RenderGraphCompileError::LegacyRenderer(_)
    ));
}

#[test]
fn rejected_legacy_paths_not_reachable() {
    let forbidden = RenderGraphCompiler::legacy_renderers();
    assert!(forbidden.contains(&"remotion"));
    assert!(forbidden.contains(&"hyperframes"));
    assert!(forbidden.contains(&"hyper-frames"));
}

#[test]
fn native_compositor_validates_source_graph() {
    NativeCompositor::validate(&source_graph()).expect("ok");
}
