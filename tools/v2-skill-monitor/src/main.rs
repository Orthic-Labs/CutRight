//! v2-skill-monitor CLI.
//!
//! Usage:
//!   v2-skill-monitor --root <cutright-root> [--out <file>]
//!
//! Read-only: never writes to the skills tree. Emits a deterministic JSON
//! health report (stdout or `--out`). Exit codes: 0 all healthy/degraded,
//! 1 at least one failed skill, 2 missing skills root or usage error.

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
    let root = match flag(&args, "--root") {
        Some(root) => root,
        None => {
            eprintln!("usage: v2-skill-monitor --root <root> [--out <file>]");
            return ExitCode::from(2);
        }
    };
    let report = v2_skill_monitor::monitor(&root);
    let rendered = v2_skill_monitor::render_report(&report);
    match flag(&args, "--out") {
        Some(path) => {
            if let Err(err) = std::fs::write(&path, rendered.as_bytes()) {
                eprintln!("error: cannot write {}: {err}", path.display());
                return ExitCode::from(2);
            }
        }
        None => println!("{rendered}"),
    }
    match report.root_status {
        v2_skill_monitor::RootStatus::Missing => {
            eprintln!("v2-skill-monitor: skills root missing under {}", root.display());
            ExitCode::from(2)
        }
        v2_skill_monitor::RootStatus::Present if report.has_failures() => ExitCode::from(1),
        v2_skill_monitor::RootStatus::Present => ExitCode::SUCCESS,
    }
}
