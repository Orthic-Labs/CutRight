//! Protocol-facing MCP server helpers.

/// The stdio transport is explicit; there is no implicit network listener.
pub fn server_disabled_by_default() -> bool {
    true
}

/// The legacy socket surface remains loopback-only when enabled.
pub fn server_loopback_only() -> bool {
    true
}
