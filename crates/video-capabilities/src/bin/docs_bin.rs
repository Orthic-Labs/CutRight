//! Capability docs generator binary (CR-V2-B2-016).
//!
//! Renders `docs/dispatch/v2/book-2/capabilities.md` from the canonical
//! registry. Exits 0 on success.

use std::path::PathBuf;
use std::process::ExitCode;

use video_capabilities::{docs, RegistryDocument};

fn main() -> ExitCode {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    let registry_path = repo_root.join("docs/dispatch/v2/source/capability-registry.json");
    let out_path = repo_root.join("docs/dispatch/v2/book-2/capabilities.md");

    let doc = match RegistryDocument::load(&registry_path) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("failed to load registry: {err}");
            return ExitCode::FAILURE;
        }
    };
    match docs::write_markdown(&doc, &out_path) {
        Ok(bytes) => {
            eprintln!("wrote {} bytes to {}", bytes, out_path.display());
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("failed to write docs: {err}");
            ExitCode::FAILURE
        }
    }
}
