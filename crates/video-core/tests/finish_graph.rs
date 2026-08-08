use std::collections::BTreeMap;
use video_core::{
    active_envelope, biased_push_scale, dry_wet_split, exponential_bloom, pullback_scale,
    punch_wave_scale, transient_offset_ms, upward_drift, FinishGraphNode, FinishNode,
    FinishRenderGraph, GraphTime,
};

#[test]
fn motion_samples_match_contract() {
    assert!((pullback_scale(0.0) - 1.3).abs() < 1e-6);
    assert!((pullback_scale(1.0) - 1.0).abs() < 1e-6);
    assert!(biased_push_scale(0.5, 0.8, 1.2) >= 1.0);
    assert!(punch_wave_scale(0.5, 1.2, 0.5, 0.2) > 1.0);
    assert_eq!(active_envelope(2.0, 0.0, 1.0), 0.0);
}

#[test]
fn typography_and_audio_samples_match_contract() {
    assert!(exponential_bloom(0.0, 18.0).abs() < 1e-6);
    assert!(upward_drift(1.0, 12.0) < 0.0);
    assert_eq!(transient_offset_ms(100, 140), 40);
    assert_eq!(transient_offset_ms(100, 200), 0);
    assert_eq!(dry_wet_split(10, 20), (1.0, 0.0));
    assert_eq!(dry_wet_split(20, 20), (0.0, 1.0));
}

#[test]
fn typed_graph_orders_dependencies_and_rejects_unknown_assets() {
    let time = |numerator| GraphTime {
        numerator,
        denominator: 1,
    };
    let node = |id: &str, inputs: Vec<&str>, node| FinishGraphNode {
        id: id.into(),
        start: time(0),
        end: time(1),
        inputs: inputs.into_iter().map(String::from).collect(),
        node,
    };
    let mut graph = FinishRenderGraph {
        schema_version: 1,
        source_path: "source.mov".into(),
        duration: time(2),
        assets: BTreeMap::new(),
        nodes: vec![
            node(
                "motion",
                vec!["text"],
                FinishNode::Pullback {
                    start_scale: 1.3,
                    end_scale: 1.0,
                    center_x: 0.5,
                    center_y: 0.5,
                },
            ),
            node(
                "text",
                vec![],
                FinishNode::TextBloom {
                    text: "Authority".into(),
                    start_blur: 12.0,
                    rise_px: 18.0,
                },
            ),
        ],
    };
    assert_eq!(graph.validate().unwrap(), vec!["text", "motion"]);
    let wire = serde_json::to_value(&graph).unwrap();
    assert_eq!(wire["sourcePath"], "source.mov");
    assert!((wire["nodes"][0]["node"]["startScale"].as_f64().unwrap() - 1.3).abs() < 1e-6);

    graph.nodes[0].node = FinishNode::AssetPlacement {
        asset_id: "missing".into(),
        cell: "topLeft".into(),
        parallax: 0.1,
    };
    assert!(graph.validate().is_err());
    graph.nodes[0].node = FinishNode::BiasedPushIn {
        start_scale: 0.9,
        end_scale: 1.2,
        bias_x: 0.5,
        bias_y: 0.5,
    };
    assert!(graph.validate().is_err());
}
