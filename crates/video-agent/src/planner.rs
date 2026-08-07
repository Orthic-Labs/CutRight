// crates/video-agent/src/planner.rs — CR-V2-B6-018 Lane C.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentPlan { pub goal: String, pub format: String, pub evidence_queries: Vec<String>, pub proposed_tools: Vec<String>, pub expected_actions: Vec<String>, pub review_points: Vec<String>, pub resource_budget: u64 }
impl AgentPlan { pub fn new(goal: &str) -> Self { Self { goal: goal.into(), format: "v2".into(), evidence_queries: vec![], proposed_tools: vec![], expected_actions: vec![], review_points: vec![], resource_budget: 0 } } }
