use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GraphTime {
    pub numerator: i64,
    pub denominator: u32,
}
impl GraphTime {
    pub fn seconds(self) -> f64 {
        self.numerator as f64 / self.denominator.max(1) as f64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum FinishNode {
    Pullback {
        start_scale: f32,
        end_scale: f32,
        center_x: f32,
        center_y: f32,
    },
    PunchWave {
        peak_scale: f32,
        peak_time: GraphTime,
        width: GraphTime,
    },
    BiasedPushIn {
        start_scale: f32,
        end_scale: f32,
        bias_x: f32,
        bias_y: f32,
    },
    TextBloom {
        text: String,
        start_blur: f32,
        rise_px: f32,
    },
    AuthorityStack {
        lines: Vec<String>,
        stagger: GraphTime,
    },
    AssetPlacement {
        asset_id: String,
        cell: String,
        parallax: f32,
    },
    EditorTakeover {
        asset_id: String,
        first_word_id: String,
        last_word_id: String,
    },
    AudioCue {
        cue_id: String,
        target_peak: GraphTime,
    },
    ReverbThrow {
        source_id: String,
        split: GraphTime,
        wet_tail_ms: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FinishGraphNode {
    pub id: String,
    pub start: GraphTime,
    pub end: GraphTime,
    pub inputs: Vec<String>,
    pub node: FinishNode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FinishRenderGraph {
    pub schema_version: u32,
    pub source_path: String,
    pub duration: GraphTime,
    #[serde(default)]
    pub assets: BTreeMap<String, String>,
    pub nodes: Vec<FinishGraphNode>,
}

#[derive(Debug, Error, PartialEq)]
pub enum FinishGraphError {
    #[error("invalid graph time")]
    InvalidTime,
    #[error("duplicate node: {0}")]
    Duplicate(String),
    #[error("unknown input: {0}")]
    Unknown(String),
    #[error("cycle: {0}")]
    Cycle(String),
    #[error("unknown asset: {0}")]
    Asset(String),
    #[error("non-finite node value: {0}")]
    NonFinite(String),
    #[error("node outside duration: {0}")]
    Outside(String),
    #[error("invalid node contract: {0}")]
    InvalidNode(String),
}

impl FinishRenderGraph {
    pub fn validate(&self) -> Result<Vec<String>, FinishGraphError> {
        if self.schema_version != 1
            || self.source_path.trim().is_empty()
            || self.duration.denominator == 0
            || self.duration.seconds() <= 0.0
            || self.assets.values().any(|path| path.trim().is_empty())
        {
            return Err(FinishGraphError::InvalidTime);
        }
        let mut ids = BTreeSet::new();
        for n in &self.nodes {
            if n.id.trim().is_empty() || !ids.insert(n.id.clone()) {
                return Err(FinishGraphError::Duplicate(n.id.clone()));
            }
            let unique_inputs = n.inputs.iter().collect::<BTreeSet<_>>();
            if unique_inputs.len() != n.inputs.len() {
                return Err(FinishGraphError::InvalidNode(n.id.clone()));
            }
            if n.start.denominator == 0
                || n.end.denominator == 0
                || n.start.seconds() < 0.0
                || n.end.seconds() <= n.start.seconds()
                || n.end.seconds() > self.duration.seconds()
            {
                return Err(FinishGraphError::Outside(n.id.clone()));
            }
            for input in &n.inputs {
                if !ids.contains(input) && !self.nodes.iter().any(|x| x.id == *input) {
                    return Err(FinishGraphError::Unknown(input.clone()));
                }
            }
            self.validate_node(n)?;
        }
        let mut state = BTreeMap::new();
        let mut out = Vec::new();
        for n in &self.nodes {
            self.visit(n, &mut state, &mut out)?;
        }
        Ok(out)
    }
    fn validate_node(&self, n: &FinishGraphNode) -> Result<(), FinishGraphError> {
        let vals: Vec<f32> = match &n.node {
            FinishNode::Pullback {
                start_scale,
                end_scale,
                center_x,
                center_y,
            } => vec![*start_scale, *end_scale, *center_x, *center_y],
            FinishNode::PunchWave { peak_scale, .. } => vec![*peak_scale],
            FinishNode::BiasedPushIn {
                start_scale,
                end_scale,
                bias_x,
                bias_y,
            } => vec![*start_scale, *end_scale, *bias_x, *bias_y],
            FinishNode::TextBloom {
                start_blur,
                rise_px,
                ..
            } => vec![*start_blur, *rise_px],
            FinishNode::AssetPlacement { parallax, .. } => vec![*parallax],
            _ => vec![],
        };
        if vals.iter().any(|v| !v.is_finite()) {
            return Err(FinishGraphError::NonFinite(n.id.clone()));
        }
        let valid_time = |time: GraphTime| {
            time.denominator > 0 && time.numerator >= 0 && time.seconds() <= self.duration.seconds()
        };
        let valid_point = |x: f32, y: f32| (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y);
        let valid = match &n.node {
            FinishNode::Pullback {
                start_scale,
                end_scale,
                center_x,
                center_y,
            } => {
                (*start_scale - 1.3).abs() <= f32::EPSILON
                    && (*end_scale - 1.0).abs() <= f32::EPSILON
                    && (*center_x - 0.5).abs() <= f32::EPSILON
                    && (*center_y - 0.5).abs() <= f32::EPSILON
            }
            FinishNode::PunchWave {
                peak_scale,
                peak_time,
                width,
            } => {
                *peak_scale >= 1.0
                    && valid_time(*peak_time)
                    && width.numerator > 0
                    && valid_time(*width)
            }
            FinishNode::BiasedPushIn {
                start_scale,
                end_scale,
                bias_x,
                bias_y,
            } => *start_scale >= 1.0 && *end_scale >= 1.0 && valid_point(*bias_x, *bias_y),
            FinishNode::TextBloom {
                text, start_blur, ..
            } => !text.is_empty() && *start_blur >= 0.0,
            FinishNode::AuthorityStack { lines, stagger } => {
                !lines.is_empty()
                    && lines.iter().all(|line| !line.is_empty())
                    && valid_time(*stagger)
            }
            FinishNode::AssetPlacement { cell, .. } => {
                matches!(
                    cell.as_str(),
                    "topLeft" | "topRight" | "bottomLeft" | "bottomRight"
                )
            }
            FinishNode::EditorTakeover {
                first_word_id,
                last_word_id,
                ..
            } => !first_word_id.is_empty() && !last_word_id.is_empty(),
            FinishNode::AudioCue { target_peak, .. } => valid_time(*target_peak),
            FinishNode::ReverbThrow {
                split, wet_tail_ms, ..
            } => valid_time(*split) && *wet_tail_ms > 0,
        };
        if !valid {
            return Err(FinishGraphError::InvalidNode(n.id.clone()));
        }
        if let FinishNode::AssetPlacement { asset_id, .. }
        | FinishNode::EditorTakeover { asset_id, .. } = &n.node
        {
            if self
                .assets
                .get(asset_id)
                .is_none_or(|path| path.trim().is_empty())
            {
                return Err(FinishGraphError::Asset(asset_id.clone()));
            }
        }
        if let FinishNode::AudioCue { cue_id, .. } = &n.node {
            if self
                .assets
                .get(cue_id)
                .is_none_or(|path| path.trim().is_empty())
            {
                return Err(FinishGraphError::Asset(cue_id.clone()));
            }
        }
        Ok(())
    }
    fn visit(
        &self,
        n: &FinishGraphNode,
        state: &mut BTreeMap<String, u8>,
        out: &mut Vec<String>,
    ) -> Result<(), FinishGraphError> {
        if state.get(&n.id) == Some(&1) {
            return Err(FinishGraphError::Cycle(n.id.clone()));
        }
        if state.get(&n.id) == Some(&2) {
            return Ok(());
        }
        state.insert(n.id.clone(), 1);
        for i in &n.inputs {
            let x = self
                .nodes
                .iter()
                .find(|x| x.id == *i)
                .ok_or_else(|| FinishGraphError::Unknown(i.clone()))?;
            self.visit(x, state, out)?;
        }
        state.insert(n.id.clone(), 2);
        out.push(n.id.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(id: &str, inputs: Vec<&str>, kind: FinishNode) -> FinishGraphNode {
        FinishGraphNode {
            id: id.into(),
            start: GraphTime {
                numerator: 0,
                denominator: 1,
            },
            end: GraphTime {
                numerator: 1,
                denominator: 1,
            },
            inputs: inputs.into_iter().map(String::from).collect(),
            node: kind,
        }
    }
    fn graph(nodes: Vec<FinishGraphNode>) -> FinishRenderGraph {
        FinishRenderGraph {
            schema_version: 1,
            source_path: "clip.mov".into(),
            duration: GraphTime {
                numerator: 2,
                denominator: 1,
            },
            assets: [("a".into(), "assets/a.mov".into())].into_iter().collect(),
            nodes,
        }
    }
    #[test]
    fn stable_order_and_cycle_rejection() {
        let g = graph(vec![
            node(
                "out",
                vec!["src"],
                FinishNode::Pullback {
                    start_scale: 1.3,
                    end_scale: 1.0,
                    center_x: 0.5,
                    center_y: 0.5,
                },
            ),
            node(
                "src",
                vec![],
                FinishNode::TextBloom {
                    text: "x".into(),
                    start_blur: 2.0,
                    rise_px: 1.0,
                },
            ),
        ]);
        assert_eq!(g.validate().unwrap(), vec!["src", "out"]);
        let c = graph(vec![
            node(
                "a",
                vec!["b"],
                FinishNode::TextBloom {
                    text: "x".into(),
                    start_blur: 1.,
                    rise_px: 0.,
                },
            ),
            node(
                "b",
                vec!["a"],
                FinishNode::TextBloom {
                    text: "x".into(),
                    start_blur: 1.,
                    rise_px: 0.,
                },
            ),
        ]);
        assert!(matches!(c.validate(), Err(FinishGraphError::Cycle(_))));
    }
    #[test]
    fn rejects_unknown_asset_and_nonfinite() {
        let g = graph(vec![node(
            "a",
            vec![],
            FinishNode::AssetPlacement {
                asset_id: "missing".into(),
                cell: "topLeft".into(),
                parallax: 0.,
            },
        )]);
        assert!(matches!(g.validate(), Err(FinishGraphError::Asset(_))));
        let g = graph(vec![node(
            "a",
            vec![],
            FinishNode::Pullback {
                start_scale: f32::NAN,
                end_scale: 1.,
                center_x: 0.5,
                center_y: 0.5,
            },
        )]);
        assert!(matches!(g.validate(), Err(FinishGraphError::NonFinite(_))));
    }

    #[test]
    fn serializes_wire_contract_as_camel_case() {
        let g = graph(vec![node(
            "motion",
            vec![],
            FinishNode::Pullback {
                start_scale: 1.3,
                end_scale: 1.0,
                center_x: 0.5,
                center_y: 0.5,
            },
        )]);
        let value = serde_json::to_value(g).unwrap();
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["sourcePath"], "clip.mov");
        assert!((value["nodes"][0]["node"]["startScale"].as_f64().unwrap() - 1.3).abs() < 1e-6);
        assert!(value.get("schema_version").is_none());
    }
}
