//! video-editorial — CutRight v2 editorial reasoning.
//!
//! Lane B (deterministic beat, take, score, boundary, disfluency,
//! dead-air modules) and lane C (narrative arc, ordering, hook,
//! shorts, confidence, critic, reflection, repair, engine) live here.
//! The crate is the canonical implementation of Book 4 lanes B + C.

#![doc = "video-editorial is part of the CutRight v2 Book 4 editorial lanes B and C."]

pub mod benchmark_binding;
pub mod deterministic;
pub mod engine;
pub mod narrative;
pub mod plan;
pub mod read_models;
pub mod rough_cut;
pub mod variants;
