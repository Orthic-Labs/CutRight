//! v2-skill-compiler CLI.
//!
//! Usage:
//!   v2-skill-compiler --root <cutright-root> --pack-out <file> --topology-out <file>
//!
//! Consumes only `<root>/skills/`, `<root>/schemas/skills/`, and
//! `<root>/capabilities/registry.json`. Deterministic: identical inputs
//! produce byte-identical pack and topology files. Exit codes: 0 success,
//! 1 compilation errors, 2 usage error.

use std::path::PathBuf;
use std::process::ExitCode;

fn flag(args: &[String], name: &str) -> Option<PathBuf> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (root, pack_out, topology_out) = match (
        flag(&args, "--root"),
        flag(&args, "--pack-out"),
        flag(&args, "--topology-out"),
    ) {
        (Some(root), Some(pack), Some(topology)) => (root, pack, topology),
        _ => {
            eprintln!(
                "usage: v2-skill-compiler --root <root> --pack-out <file> --topology-out <file>"
            );
            return ExitCode::from(2);
        }
    };

    let report = v2_skill_compiler::compile(&root);
    if !report.ok() {
        let mut errors = report.errors.clone();
        errors.sort();
        errors.dedup();
        for error in &errors {
            eprintln!("error: {error}");
        }
        eprintln!("v2-skill-compiler: {} error(s)", errors.len());
        return ExitCode::from(1);
    }

    let pack = v2_skill_compiler::render_pack(&report);
    let topology = v2_skill_compiler::render_topology(&report);
    if let Err(err) = std::fs::write(&pack_out, pack.as_bytes()) {
        eprintln!("error: cannot write {}: {err}", pack_out.display());
        return ExitCode::from(1);
    }
    if let Err(err) = std::fs::write(&topology_out, topology.as_bytes()) {
        eprintln!("error: cannot write {}: {err}", topology_out.display());
        return ExitCode::from(1);
    }
    println!(
        "v2-skill-compiler: {} skill(s), {} schema(s), pack_hash from {} bytes",
        report.skills.len(),
        report.schemas.len(),
        pack.len()
    );
    ExitCode::SUCCESS
}
