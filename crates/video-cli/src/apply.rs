//! `videoctl apply` — load a `cutright.action_batch/v1` from stdin/file,
//! drive the [`ActionExecutor`](video_project::ActionExecutor), and emit the
//! matching [`ExecutorReport`](video_project::ExecutorReport) to stdout
//! (CR-V2-B2-023).
//!
//! The CLI never duplicates executor logic. It only:
//! 1. Parses the action_batch JSON envelope.
//! 2. Loads the canonical capability registry (Lane P-B).
//! 3. Acquires a `SessionGuard` for the target project (Lane P-C).
//! 4. Hands the batch to the executor and serialises the report.
//!
//! Exit codes:
//!   0 — success
//!   1 — domain error (failed receipt)
//!   4 — invalid input (parse error)
//!   5 — project lock held (Lane P-C contention)

use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Args;
use video_capabilities::RegistryDocument;
use video_project::{
    ActionBatch, ActionExecutor, ExecutorReport, ACTION_BATCH_SCHEMA,
};
use video_sessions::{ProjectId, SessionGuard};

use crate::cli::ApplyArgs;

/// Run the `apply` subcommand. Returns the exit code.
pub fn run(args: &ApplyArgs) -> ExitCode {
    let raw = match read_input(args) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("apply: failed to read input: {err}");
            return ExitCode::from(4);
        }
    };
    let batch: ActionBatch = match serde_json::from_str(&raw) {
        Ok(b) => b,
        Err(err) => {
            eprintln!("apply: failed to parse action_batch: {err}");
            return ExitCode::from(4);
        }
    };
    if batch.schema != ACTION_BATCH_SCHEMA {
        eprintln!(
            "apply: action_batch schema {:?} does not match {}",
            batch.schema, ACTION_BATCH_SCHEMA
        );
        return ExitCode::from(4);
    }

    let project_dir = match resolve_project_dir(args, &batch) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("apply: {err}");
            return ExitCode::from(4);
        }
    };

    let registry_path = match resolve_registry_path(args) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("apply: {err}");
            return ExitCode::from(4);
        }
    };
    let registry_doc = match RegistryDocument::load(&registry_path) {
        Ok(doc) => doc,
        Err(err) => {
            eprintln!(
                "apply: failed to load capability registry at {}: {err}",
                registry_path.display()
            );
            return ExitCode::from(4);
        }
    };
    let registry = registry_doc.into_registry();

    let project_id = project_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".into());
    let guard = match SessionGuard::acquire(&project_dir, ProjectId::new(project_id)) {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("apply: failed to acquire project lock: {err}");
            return ExitCode::from(5);
        }
    };

    let executor = ActionExecutor::new(&project_dir);
    let report = match executor.execute(&batch, &registry, &guard, None) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("apply: executor failed: {err}");
            return ExitCode::from(1);
        }
    };
    print_report(&report);
    if report.is_applied() || report.is_dry_run() {
        ExitCode::from(0)
    } else {
        ExitCode::from(1)
    }
}

fn read_input(args: &ApplyArgs) -> io::Result<String> {
    if let Some(path) = &args.file {
        std::fs::read_to_string(path)
    } else if args.from_stdin || !atty_stdout() {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no --file given and stdin is empty",
        ))
    }
}

fn atty_stdout() -> bool {
    // We don't actually need stdout tty-ness; we only need to know whether
    // the user passed `--file` or `--from-stdin`. If neither was given we
    // default to reading stdin (CI / piped callers are the common case).
    // The flag exists so interactive users can pipe `cat batch.json |
    // videoctl apply`.
    false
}

fn resolve_project_dir(args: &ApplyArgs, batch: &ActionBatch) -> Result<PathBuf, String> {
    if let Some(p) = &args.project {
        return Ok(p.clone());
    }
    // Fall back to the env var; the Studio also exposes this through
    // `videoctl doctor` so the variable name is shared.
    if let Ok(value) = std::env::var("CUTRIGHT_PROJECT_DIR") {
        return Ok(PathBuf::from(value));
    }
    Err(format!(
        "could not determine project directory; pass --project or set CUTRIGHT_PROJECT_DIR (batch_id={})",
        batch.batch_id
    ))
}

fn resolve_registry_path(args: &ApplyArgs) -> Result<PathBuf, String> {
    if let Ok(value) = std::env::var("CUTRIGHT_CAPABILITY_REGISTRY") {
        return Ok(PathBuf::from(value));
    }
    if let Some(p) = &args.registry {
        return Ok(p.clone());
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| "could not resolve repo root".to_string())?;
    Ok(repo_root.join("docs/dispatch/v2/source/capability-registry.json"))
}

fn print_report(report: &ExecutorReport) {
    match serde_json::to_string_pretty(report) {
        Ok(text) => println!("{text}"),
        Err(err) => eprintln!("apply: failed to serialize report: {err}"),
    }
}

/// Re-export of the CLI args struct so `cli.rs` can hand the parsed
/// arguments in.
#[derive(Debug, Clone, Args)]
pub struct ApplyCommand {
    /// Args struct (re-exported for `cli.rs`).
    #[command(flatten)]
    pub args: ApplyArgs,
}