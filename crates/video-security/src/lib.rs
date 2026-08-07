//! CutRight v2 local security primitives.
//!
//! The crate hosts the worker sandbox/grant model and the untrusted-media
//! limits. It does not implement any platform-specific sandboxing primitive
//! itself; the worker's harness on each target owns the syscall/shell
//! boundary. This crate supplies the policy decisions and the validation
//! routines that the harness asks before granting a worker any access.

pub mod media_limits;
pub mod sandbox;
pub mod trust;
