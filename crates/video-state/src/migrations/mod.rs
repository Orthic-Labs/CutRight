//! v1 → v2 migration descriptors and helpers.
//!
//! `migrations::v2` defines the frozen v1 → v2 plan; the runner in
//! [`crate::migrate`] consumes these descriptors. New versions add
//! sibling modules; the `migrations::v2` module is never mutated.

pub mod v2;
