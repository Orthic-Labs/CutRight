// crates/video-project/src/inspect.rs — CR-V2-B6-019 Lane C.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InspectRequest { pub timeline_id: String, pub revision: String, pub start_ms: i64, pub end_ms: Option<i64>, pub max_frames: u32 }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InspectSample { pub frame_index: u32, pub position_ms: i64, pub image_ref: String, pub visible_object_ids: Vec<String> }
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InspectResponse { pub samples: Vec<InspectSample>, pub revision: String }
pub fn inspect_timeline(req: &InspectRequest) -> InspectResponse { InspectResponse { samples: vec![], revision: req.revision.clone() } }
