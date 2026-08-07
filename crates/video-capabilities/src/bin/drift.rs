//! Drift detector binary (CR-V2-B2-016).
//!
//! Walks the canonical capability registry, scans the in-tree source set
//! for `capability_id = "..."` literals, and emits a JSON report to stdout
//! describing any drift.
//!
//! Exit codes:
//!   0 — clean
//!   1 — drift detected (unknown references or stale generated artifacts)
//!   2 — loader / IO failure

use std::path::PathBuf;
use std::process::ExitCode;

use video_capabilities::drift::{detect_drift, DriftReportJson};

fn main() -> ExitCode {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf();
    let registry_path = repo_root.join("docs/dispatch/v2/source/capability-registry.json");
    let source_paths = vec![
        repo_root.join("crates/video-capabilities/src"),
        repo_root.join("crates/video-capabilities/tests"),
        repo_root.join("crates/video-cli/src"),
        repo_root.join("crates/video-cli/tests"),
        repo_root.join("bindings"),
        repo_root.join("apps/studio/src-tauri/src"),
        repo_root.join("crates/video-actions/src"),
        repo_root.join("crates/video-state/src"),
        repo_root.join("crates/video-sessions/src"),
        repo_root.join("crates/video-project/src"),
    ];

    // Generated artifacts that must be in sync with the registry.
    // The expected contents are produced by the codegen pipeline at build
    // time; for drift purposes we re-run the codegen in-memory so the
    // comparison is byte-exact.
    let generated_rust = repo_root.join("crates/video-capabilities/src/generated.rs");
    let generated_ts = repo_root.join("bindings/ts/capabilities.ts");
    let generated_mcp = repo_root.join("bindings/mcp/tool-registry.json");

    let doc = match video_capabilities::RegistryDocument::load(&registry_path) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!("failed to load registry: {err}");
            return ExitCode::from(2);
        }
    };
    let rust_expected = match video_capabilities::render_rust_enum(&doc) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("failed to render rust enum: {err}");
            return ExitCode::from(2);
        }
    };
    let ts_expected = match video_capabilities::render_typescript(&doc) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("failed to render typescript: {err}");
            return ExitCode::from(2);
        }
    };
    let mcp_expected = match video_capabilities::render_mcp_tool_registry(&doc) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("failed to render mcp tool registry: {err}");
            return ExitCode::from(2);
        }
    };
    let artifacts: Vec<(PathBuf, String)> = vec![
        (generated_rust, rust_expected),
        (generated_ts, ts_expected),
        (generated_mcp, mcp_expected),
    ];

    let report = match detect_drift(&repo_root, &registry_path, &source_paths, &artifacts) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("drift detection failed: {err}");
            return ExitCode::from(2);
        }
    };
    let json = DriftReportJson::from(&report);
    let serialized = match serde_json::to_string_pretty(&json) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("failed to serialize drift report: {err}");
            return ExitCode::from(2);
        }
    };
    println!("{serialized}");
    if report.is_clean() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}