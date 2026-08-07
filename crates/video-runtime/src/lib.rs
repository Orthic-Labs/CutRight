//! CutRight v2 runtime support.
//!
//! The runtime crate owns offline pack repair helpers and the local
//! integrity checks invoked by the Studio Pack Manager.

pub mod offline_repair;

pub use offline_repair::{offline_repair, OfflineRepairAction, OfflineRepairOutcome};
