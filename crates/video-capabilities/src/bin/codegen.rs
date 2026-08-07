//! One-shot codegen driver for `video-capabilities`.
//!
//! Reads the canonical capability registry and writes four artifacts:
//!   * Rust enum   → `crates/video-capabilities/src/generated.rs`
//!   * TypeScript  → `bindings/ts/capabilities.ts`
//!   * MCP tool    → `bindings/mcp/tool-registry.json`
//!
//! Run from the repo root:
//!
//! ```sh
//! cargo run -p video-capabilities --bin video-capabilities-codegen
//! ```
//!
//! Exit codes:
//!   0 — all artifacts written
//!   1 — load or write failure
//!
//! The CLI `videoctl capabilities list` does NOT call this binary; it loads
//! the registry at runtime and reuses `video_capabilities::RegistryDocument`.

use std::path::PathBuf;
use std::process::ExitCode;

use video_capabilities::{generate_all, RegistryDocument};

fn main() -> ExitCode {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    let registry_path = repo_root.join("docs/dispatch/v2/source/capability-registry.json");
    let rust_out = manifest_dir.join("src/generated.rs");
    let ts_out = repo_root.join("bindings/ts/capabilities.ts");
    let mcp_out = repo_root.join("bindings/mcp/tool-registry.json");

    eprintln!("loading {registry_path}", registry_path = registry_path.display());
    let doc = match RegistryDocument::load(&registry_path) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("failed to load registry: {err}");
            return ExitCode::FAILURE;
        }
    };

    match generate_all(&doc, &rust_out, &ts_out, &mcp_out) {
        Ok(report) => {
            for (kind, bytes) in report {
                eprintln!("wrote {kind} ({bytes} bytes)");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("codegen failed: {err}");
            ExitCode::FAILURE
        }
    }
}
