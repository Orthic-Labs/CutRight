//! Video evidence graph (CR-V2-B3-017).
//!
//! The crate is owned by Book 3 lane C. The merge step (`CR-V2-B3-022`)
//! lifts it into the workspace membership. Until then, the file is
//! dormant inside the source tree.
//!
//! The crate exposes:
//! - deterministic scene and shot segmentation (`scene`, `shot`)
//! - hierarchical evidence graph store (`graph`, `store`, `index`)
//! - bounded evidence retrieval (`query`, `retrieve`)
//! - vision tracks (`vision`, `tracks`)

pub mod scene;
pub mod shot;

pub use scene::{FrameSequence, FrameStat, SceneBoundary, SceneDetector, SceneRefinement};
pub use shot::{MotionFrame, ShotBoundary, ShotDetectionError, ShotDetector, ShotKind, ShotRefinement};
