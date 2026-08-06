//! Proves the Rust include_str!/include_bytes! reference forms.
pub const TEXT: &str = include_str!("shared.txt");
pub const BYTES: &[u8] = include_bytes!("shared.txt");
