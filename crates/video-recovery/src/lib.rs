//! Crash recovery and project repair.
//!
//! The crate exposes a small scanner/repairer used by the dispatcher when
//! it opens a project, after an interrupted job, or when a receipt becomes
//! unverified. Repair is purely a function of state — it never mints
//! receipts, never overwrites canonical bytes, and never deletes user files.

pub mod repair;
pub mod scan;
