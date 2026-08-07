//! Focused creative native renderer, audio, and critic tests
//! (CR-V2-B5-026).
//!
//! Black-box tests over the public surface.

use video_core::creative_critic::{AxisScore, CreativeCritic, DeterministicVisualQa, Verdict};
use video_core::native_audio::{AudioCue, AudioProfile, NativeAudioEngine};
use video_core::native_compositor::{NativeCompositor, NodeKind, RenderGraph, RenderNode};
use video_core::native_motion::{MotionBeat, MotionClip, NativeMotionEngine};
use video_core::native_typography::{CaptionDocument, CaptionToken, NativeTypographyEngine, TypographyProfile};
use video_core::render_graph_compiler::RenderGraphCompiler;
use std::collections::BTreeMap;

fn ax(label: &str, score: f64, weight: f64) -> AxisScore {
    AxisScore {
        score,
        weight,
        evidence_refs: vec![format!("evidence:{label}")],
    }
}

fn single_prop(k: &str, v: &str) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert(k.to_string(), v.to_string());
    m
}

#[test]
fn critic_run_with_all_axes_above_threshold_passes() {
    let qa = DeterministicVisualQa::run("fpl_1").unwrap();
    let mut axes = BTreeMap::new();
    axes.insert("brand_alignment".to_string(), ax("brand_alignment", 0.9, 0.5));
    axes.insert("narrative_clarity".to_string(), ax("narrative_clarity", 0.85, 0.5));
    let ev = CreativeCritic::run("fpl_1", &qa, axes).unwrap();
    assert_eq!(ev.verdict, Verdict::Pass);
}

#[test]
fn critic_run_with_low_score_warns_or_fails() {
    let qa = DeterministicVisualQa::run("fpl_1").unwrap();
    let mut axes = BTreeMap::new();
    axes.insert("brand_alignment".to_string(), ax("brand_alignment", 0.5, 1.0));
    let ev = CreativeCritic::run("fpl_1", &qa, axes).unwrap();
    assert!(matches!(ev.verdict, Verdict::Warn | Verdict::Fail));
}

#[test]
fn native_audio_finish_rejects_unbound_cue() {
    let profile = AudioProfile {
        id: "ap_1".to_string(),
        platform: "ig_reels".to_string(),
        integrated_lufs: -16.0,
        true_peak_db: -1.0,
        reverb_budget_ms: 200,
    };
    let cue = AudioCue {
        id: "c_0".to_string(),
        kind: "music".to_string(),
        start_ms: 0,
        end_ms: 1000,
        evidence_ref: "".to_string(),
        reverb_ms: 50,
    };
    let err = NativeAudioEngine::finish(&profile, vec![cue]).err().expect("err");
    assert!(matches!(err, video_core::native_audio::AudioError::UnboundCue(_)));
}

#[test]
fn native_motion_rejects_unknown_transition() {
    let clip = MotionClip {
        id: "mc_1".to_string(),
        version: "v2".to_string(),
        beats: vec![MotionBeat {
            id: "mb_0".to_string(),
            start_ms: 0,
            end_ms: 1000,
            transition: "wipe".to_string(),
        }],
        allowed_transitions: vec!["cut".to_string()],
        platform: "ig_reels".to_string(),
    };
    let err = NativeMotionEngine::validate(&clip).err().expect("err");
    assert!(matches!(err, video_core::native_motion::MotionError::ForbiddenTransition(_, _)));
}

#[test]
fn native_typography_rejects_token_outside_safe_zone() {
    let profile = TypographyProfile {
        id: "tp_1".to_string(),
        platform: "ig_reels".to_string(),
        min_font_size: 14.0,
        safe_zone_x: 0.05,
        safe_zone_y: 0.05,
        safe_zone_w: 0.9,
        safe_zone_h: 0.9,
        reduced_motion: true,
        font_family: "Inter".to_string(),
    };
    let doc = CaptionDocument {
        id: "cd_1".to_string(),
        version: "v2".to_string(),
        tokens: vec![CaptionToken {
            id: "t_0".to_string(),
            text: "hi".to_string(),
            x: 0.99,
            y: 0.1,
            w: 0.4,
            h: 0.1,
            start_ms: 0,
            end_ms: 1000,
            evidence_ref: "evidence:ev_1".to_string(),
        }],
    };
    let err = NativeTypographyEngine::layout(&doc, &profile).err().expect("err");
    assert!(matches!(err, video_core::native_typography::TypographyError::OutsideSafeZone(_)));
}

#[test]
fn render_graph_compiler_rejects_legacy_renderer() {
    let g = RenderGraph {
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
                props: single_prop("via", "remotion"),
            },
        ],
    };
    let err = RenderGraphCompiler::compile(&g).err().expect("err");
    assert!(matches!(
        err,
        video_core::render_graph_compiler::RenderGraphCompileError::LegacyRenderer(_)
    ));
}

#[test]
fn native_compositor_compiles_linear_graph() {
    let g = RenderGraph {
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
                id: "xform".to_string(),
                kind: NodeKind::Transform,
                inputs: vec!["src".to_string()],
                props: BTreeMap::new(),
            },
            RenderNode {
                id: "out".to_string(),
                kind: NodeKind::Output,
                inputs: vec!["xform".to_string()],
                props: BTreeMap::new(),
            },
        ],
    };
    NativeCompositor::validate(&g).expect("ok");
    let cmds = NativeCompositor::compile(&g).expect("ok");
    assert_eq!(cmds.len(), 3);
}
