//! import-closure CLI: transitive source-closure scanner for CutRight v2.
//!
//! Usage:
//!   import-closure scan --root SNAPSHOT_DIR [options]
//!
//! Options:
//!   --root PATH        Pinned snapshot root to scan (required).
//!   --ledger PATH      Disposition ledger (imports/v2/dispositions.json).
//!   --corpus PATH      Source corpus (imports/v2/source-corpus.json); when
//!                      given together with --ledger, every corpus row must
//!                      have a matching ledger entry.
//!   --source-id ID     Source id used for the node disposition lookup
//!                      (default: cutright).
//!   --help             Print this help.
//!
//! Exits nonzero on dangling references, path escapes, symlink escapes,
//! submodules, device files, mutable URLs, missing ledger rows, or any
//! unclassified reachable node.

mod scan;

use std::path::PathBuf;
use std::process::ExitCode;

const HELP: &str = "import-closure: transitive source-closure scanner for the CutRight v2 corpus

USAGE:
    import-closure scan --root <SNAPSHOT_DIR> [OPTIONS]

OPTIONS:
    --root <PATH>       Pinned snapshot root to scan (required)
    --ledger <PATH>     Disposition ledger (imports/v2/dispositions.json)
    --corpus <PATH>     Source corpus (imports/v2/source-corpus.json)
    --source-id <ID>    Source id for disposition lookup [default: cutright]
    -h, --help          Print help

OUTPUT:
    Deterministic sorted JSON node graph on stdout: source_id, path,
    sha256, references, and disposition lookup result per node.

EXIT STATUS:
    0   closure is clean and fully classified
    1   dangling reference, path/symlink escape, submodule, device file,
        mutable URL, missing ledger row, or unclassified reachable node
    2   invalid arguments
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{HELP}");
        return ExitCode::SUCCESS;
    }

    let mut rest = args.as_slice();
    match rest.first() {
        Some(c) if c == "scan" => {
            rest = &rest[1..];
        }
        Some(other) => {
            eprintln!("unknown command: {other}\n\n{HELP}");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("{HELP}");
            return ExitCode::from(2);
        }
    }

    let config = match parse_scan_options(rest) {
        Ok(config) => config,
        Err(code) => return code,
    };

    match scan::scan(&config) {
        Ok(report) => {
            print!("{}", scan::report_to_json(&report));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("import-closure: closure violation:\n{error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_scan_options(rest: &[String]) -> Result<scan::ScanConfig, ExitCode> {
    let mut config = scan::ScanConfig {
        root: PathBuf::new(),
        source_id: "cutright".to_string(),
        ledger: None,
        corpus: None,
    };
    let mut i = 0;
    while i < rest.len() {
        let flag = rest[i].as_str();
        let value = || -> Result<String, ExitCode> {
            rest.get(i + 1).cloned().ok_or_else(|| {
                eprintln!("missing value for {flag}");
                ExitCode::from(2)
            })
        };
        match flag {
            "--root" => config.root = PathBuf::from(value()?),
            "--ledger" => config.ledger = Some(PathBuf::from(value()?)),
            "--corpus" => config.corpus = Some(PathBuf::from(value()?)),
            "--source-id" => config.source_id = value()?,
            other => {
                eprintln!("unknown option: {other}\n\n{HELP}");
                return Err(ExitCode::from(2));
            }
        }
        i += 2;
    }
    if config.root.as_os_str().is_empty() {
        eprintln!("--root is required\n\n{HELP}");
        return Err(ExitCode::from(2));
    }
    Ok(config)
}
