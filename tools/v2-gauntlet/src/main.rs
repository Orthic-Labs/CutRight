//! CutRight v2 evidence gauntlet — CLI entry point.
//!
//! Modes:
//! - `--self-test`: verifies the gauntlet's own contract against the committed
//!   fixtures (the weak fixture MUST fail with a surviving mutant; seeds must
//!   reproduce; an unavailable coverage backend must be `unproven`, never pass).
//! - normal: `--workspace DIR --changed FILE.json [--seed N] [--order-runs N]
//!   [--receipt-out PATH]` runs all three layers against the given workspace.
//!
//! Output is a deterministic local JSON receipt (sorted, no timestamps). The
//! gauntlet is optional for normal book gates and is never integrated with CI
//! or a hosted service.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use v2_gauntlet::{
    discover_tests, fisher_yates, json_string, language_for_path,
    mutant_result_json, run_coverage_layer, run_mutation_layer, run_order_layer, rules_for,
    ChangedFile, Language, LayerStatus,
};

const DEFAULT_SEED: u64 = 0xC0FFEE;
const DEFAULT_ORDER_RUNS: usize = 3;
const DEADLINE: Duration = Duration::from_secs(240);

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

fn read_changed_file(path: &Path) -> Result<Vec<ChangedFile>, String> {
    let text = fs::read_to_string(path).map_err(|err| format!("cannot read changed manifest: {err}"))?;
    parse_changed_manifest(&text)
}

/// Minimal parser for {"files":[{"path":"...","lines":[1,2]}]}.
fn parse_changed_manifest(text: &str) -> Result<Vec<ChangedFile>, String> {
    let mut files: Vec<ChangedFile> = Vec::new();
    let files_start = text
        .find("\"files\"")
        .ok_or_else(|| "changed manifest needs a files array".to_string())?;
    let array = &text[files_start..];
    let open = array.find('[').ok_or("files array not opened")?;
    let mut depth = 0usize;
    let mut end = None;
    for (index, ch) in array[open..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(open + index + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end.ok_or("files array not closed")?;
    let array_text = &array[open + 1..end - 1];
    for object in split_top_level(array_text, '{', '}') {
        let path_value = extract_string_field(&object, "path")
            .ok_or_else(|| format!("changed file entry missing path: {object}"))?;
        let mut lines: Vec<u64> = Vec::new();
        if let Some(lines_start) = object.find("\"lines\"") {
            let rest = &object[lines_start..];
            if let Some(lo) = rest.find('[') {
                if let Some(hi) = rest.find(']') {
                    for token in rest[lo + 1..hi].split(',') {
                        let token = token.trim();
                        if token.is_empty() {
                            continue;
                        }
                        let value: u64 = token
                            .parse()
                            .map_err(|_| format!("invalid line number {token:?}"))?;
                        lines.push(value);
                    }
                }
            }
        }
        files.push(ChangedFile { path: path_value, lines });
    }
    Ok(files)
}

fn split_top_level(text: &str, open: char, close: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = None;
    for (index, ch) in text.char_indices() {
        if ch == open {
            if depth == 0 {
                start = Some(index);
            }
            depth += 1;
        } else if ch == close {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                if let Some(start) = start.take() {
                    out.push(text[start..index + 1].to_string());
                }
            }
        }
    }
    out
}

fn extract_string_field(object: &str, field: &str) -> Option<String> {
    let marker = format!("\"{field}\"");
    let index = object.find(&marker)?;
    let rest = &object[index + marker.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn render_receipt(
    workspace: &str,
    seed: u64,
    order_runs: usize,
    mutation: (&LayerStatus, &[v2_gauntlet::MutantResult]),
    coverage: &LayerStatus,
    order: (&LayerStatus, &[Vec<String>], &[u64]),
) -> String {
    let (mutation_status, mutants) = mutation;
    let (order_status, orders, run_seeds) = order;
    let surviving: Vec<&v2_gauntlet::MutantResult> = mutants
        .iter()
        .filter(|result| matches!(result.outcome, v2_gauntlet::MutantOutcome::Survived))
        .collect();
    let mutant_rows: Vec<String> = mutants.iter().map(mutant_result_json).collect();
    let surviving_rows: Vec<String> = surviving.iter().map(|result| mutant_result_json(result)).collect();
    let order_rows: Vec<String> = orders
        .iter()
        .zip(run_seeds)
        .map(|(order, run_seed)| {
            let items: Vec<String> = order.iter().map(|item| json_string(item)).collect();
            format!(
                "{{\"run_seed\":{},\"order\":[{}]}}",
                run_seed,
                items.join(",")
            )
        })
        .collect();
    format!(
        "{{\"schema_version\":1,\"kind\":\"gauntlet\",\"workspace\":{},\"seed\":{},\"order_runs\":{},\"layers\":{{\"mutation\":{},\"coverage\":{},\"order\":{}}},\"mutants\":[{}],\"surviving_mutants\":[{}],\"order_runs_detail\":[{}]}}",
        json_string(workspace),
        seed,
        order_runs,
        mutation_status.to_json(),
        coverage.to_json(),
        order_status.to_json(),
        mutant_rows.join(","),
        surviving_rows.join(","),
        order_rows.join(","),
    )
}

fn run_workspace(
    workspace: &Path,
    changed: &[ChangedFile],
    seed: u64,
    order_runs: usize,
) -> (String, i32) {
    let (mutation_status, mutants) = run_mutation_layer(workspace, changed, DEADLINE);
    let coverage_status = run_coverage_layer(workspace, changed);
    let order_scratch = copy_workspace_for_order(workspace);
    let (order_status, orders, run_seeds) =
        run_order_layer(&order_scratch, seed, order_runs, DEADLINE);
    let _ = fs::remove_dir_all(&order_scratch);
    let receipt = render_receipt(
        &workspace.to_string_lossy(),
        seed,
        order_runs,
        (&mutation_status, &mutants),
        &coverage_status,
        (&order_status, &orders, &run_seeds),
    );
    let failed = mutation_status.is_gate_failure() || order_status.is_gate_failure();
    (receipt, if failed { 1 } else { 0 })
}

fn copy_workspace_for_order(workspace: &Path) -> PathBuf {
    let scratch = std::env::temp_dir().join(format!(
        "v2-gauntlet-order-ws-{}-{}",
        std::process::id(),
        rand_suffix()
    ));
    if v2_gauntlet::copy_dir(workspace, &scratch).is_err() {
        return workspace.to_path_buf();
    }
    scratch
}

fn rand_suffix() -> u64 {
    let mut rng = v2_gauntlet::Xorshift64::new(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1),
    );
    rng.next_u64()
}

/// Self-test: the gauntlet proves its own contract on committed fixtures.
fn run_self_test() -> i32 {
    let fixtures = fixtures_root();
    let mut checks: Vec<(String, bool, String)> = Vec::new();

    // 1. Mutant generation is deterministic and language-aware.
    let rust_line = "    if value > max {";
    let first: Vec<String> = rules_for(Language::Rust)
        .iter()
        .filter_map(|rule| rule.apply(rust_line))
        .collect();
    let second: Vec<String> = rules_for(Language::Rust)
        .iter()
        .filter_map(|rule| rule.apply(rust_line))
        .collect();
    let ts_line = "  if (a === b) {";
    let ts_first: Vec<String> = rules_for(Language::TypeScript)
        .iter()
        .filter_map(|rule| rule.apply(ts_line))
        .collect();
    checks.push((
        "mutant_generation_deterministic".to_string(),
        first == second && !first.is_empty() && !ts_first.is_empty(),
        format!("rust_mutants={first:?} ts_mutants={ts_first:?}"),
    ));
    checks.push((
        "language_detection".to_string(),
        language_for_path(Path::new("src/lib.rs")) == Some(Language::Rust)
            && language_for_path(Path::new("app/main.ts")) == Some(Language::TypeScript)
            && language_for_path(Path::new("script.py")).is_none(),
        "rs/ts supported; other extensions skipped".to_string(),
    ));

    // 2. Seeded order is reproducible and seed-dependent.
    let items: Vec<String> = (0..12).map(|i| format!("test_{i:02}")).collect();
    let a = fisher_yates(&items, 42);
    let b = fisher_yates(&items, 42);
    let c = fisher_yates(&items, 43);
    checks.push((
        "order_seed_reproducible".to_string(),
        a == b && a != c && a != items,
        format!("seed42={a:?}"),
    ));

    // 3. Weak fixture must fail with a surviving mutant.
    let weak = fixtures.join("weak");
    let weak_changed = read_changed_file(&weak.join("changed.json")).unwrap_or_default();
    let (weak_status, weak_mutants) = run_mutation_layer(&weak, &weak_changed, DEADLINE);
    let weak_survivors = weak_mutants
        .iter()
        .filter(|result| matches!(result.outcome, v2_gauntlet::MutantOutcome::Survived))
        .count();
    checks.push((
        "weak_fixture_surviving_mutant_fails".to_string(),
        weak_status == LayerStatus::Failed && weak_survivors >= 1,
        format!("status={} survivors={weak_survivors}", weak_status.name()),
    ));

    // 4. Strong fixture: all mutants killed → passed.
    let strong = fixtures.join("strong");
    let strong_changed = read_changed_file(&strong.join("changed.json")).unwrap_or_default();
    let (strong_status, strong_mutants) = run_mutation_layer(&strong, &strong_changed, DEADLINE);
    checks.push((
        "strong_fixture_all_mutants_killed".to_string(),
        strong_status == LayerStatus::Passed
            && strong_mutants.iter().all(|result| {
                matches!(result.outcome, v2_gauntlet::MutantOutcome::Killed)
            }),
        format!("status={}", strong_status.name()),
    ));

    // 5. Coverage backend: unavailable must be unproven, never pass.
    let coverage_status = run_coverage_layer(&strong, &strong_changed);
    let llvm_cov_present = std::process::Command::new("cargo")
        .args(["llvm-cov", "--version"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);
    let coverage_expectation = if llvm_cov_present {
        // Backend installed: any honest executed outcome is acceptable.
        !matches!(coverage_status, LayerStatus::Skipped { .. })
    } else {
        matches!(coverage_status, LayerStatus::Unproven { .. })
    };
    checks.push((
        "unavailable_coverage_backend_is_unproven".to_string(),
        coverage_expectation,
        format!("status={}", coverage_status.to_json()),
    ));

    // 6. Unsupported mutation shapes are skipped with reasons.
    let unsupported = vec![
        ChangedFile { path: "src/notes.txt".to_string(), lines: vec![1] },
        ChangedFile { path: "src/lib.rs".to_string(), lines: vec![1] },
    ];
    let scratch = std::env::temp_dir().join(format!("v2-gauntlet-selftest-{}", std::process::id()));
    let _ = fs::create_dir_all(scratch.join("src"));
    let _ = fs::write(scratch.join("src/notes.txt"), "hello\n");
    let _ = fs::write(scratch.join("src/lib.rs"), "// comment only line\npub fn x() {}\n");
    let (skip_status, skip_results) = run_mutation_layer(&scratch, &unsupported, DEADLINE);
    let _ = fs::remove_dir_all(&scratch);
    let reasons: Vec<String> = skip_results
        .iter()
        .filter_map(|result| match &result.outcome {
            v2_gauntlet::MutantOutcome::Skipped { reason } => Some(reason.clone()),
            _ => None,
        })
        .collect();
    checks.push((
        "unsupported_shapes_skipped_with_reasons".to_string(),
        reasons.iter().any(|reason| reason == "unsupported_file_type")
            && reasons.iter().any(|reason| reason == "unsupported_mutation_shape")
            && matches!(skip_status, LayerStatus::Skipped { .. }),
        format!("reasons={reasons:?}"),
    ));

    // 7. Test-order layer records seed and order; reproducible on the strong
    //    fixture copy.
    let strong_copy = copy_workspace_for_order(&strong);
    let (order_status, orders, run_seeds) =
        run_order_layer(&strong_copy, DEFAULT_SEED, DEFAULT_ORDER_RUNS, DEADLINE);
    let _ = fs::remove_dir_all(&strong_copy);
    let tests_found = !discover_tests(&strong).is_empty();
    let reproducible = orders.len() == DEFAULT_ORDER_RUNS
        && run_seeds.len() == DEFAULT_ORDER_RUNS
        && fisher_yates(&discover_tests(&strong), run_seeds[0]) == orders[0];
    checks.push((
        "order_layer_recorded_and_reproducible".to_string(),
        tests_found
            && reproducible
            && (order_status == LayerStatus::Passed
                || matches!(order_status, LayerStatus::Unproven { .. })),
        format!(
            "status={} seed={} runs={}",
            order_status.name(),
            DEFAULT_SEED,
            DEFAULT_ORDER_RUNS
        ),
    ));

    let all_ok = checks.iter().all(|(_, ok, _)| *ok);
    let check_rows: Vec<String> = checks
        .iter()
        .map(|(name, ok, evidence)| {
            format!(
                "{{\"check\":{},\"ok\":{},\"evidence\":{}}}",
                json_string(name),
                ok,
                json_string(evidence)
            )
        })
        .collect();
    let report = format!(
        "{{\"schema_version\":1,\"kind\":\"gauntlet-self-test\",\"seed\":{},\"order_runs\":{},\"verdict\":{},\"checks\":[{}]}}",
        DEFAULT_SEED,
        DEFAULT_ORDER_RUNS,
        json_string(if all_ok { "pass" } else { "fail" }),
        check_rows.join(",")
    );
    println!("{report}");
    if all_ok {
        0
    } else {
        1
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut workspace: Option<PathBuf> = None;
    let mut changed: Option<PathBuf> = None;
    let mut seed = DEFAULT_SEED;
    let mut order_runs = DEFAULT_ORDER_RUNS;
    let mut receipt_out: Option<PathBuf> = None;
    let mut self_test = false;

    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--self-test" => self_test = true,
            "--workspace" => {
                index += 1;
                workspace = args.get(index).map(PathBuf::from);
            }
            "--changed" => {
                index += 1;
                changed = args.get(index).map(PathBuf::from);
            }
            "--seed" => {
                index += 1;
                seed = args
                    .get(index)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(DEFAULT_SEED);
            }
            "--order-runs" => {
                index += 1;
                order_runs = args
                    .get(index)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(DEFAULT_ORDER_RUNS);
            }
            "--receipt-out" => {
                index += 1;
                receipt_out = args.get(index).map(PathBuf::from);
            }
            "--help" | "-h" => {
                println!("usage: v2-gauntlet --self-test | --workspace DIR --changed FILE [--seed N] [--order-runs N] [--receipt-out PATH]");
                return;
            }
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
        }
        index += 1;
    }

    if self_test {
        std::process::exit(run_self_test());
    }

    let (workspace, changed_path) = match (workspace, changed) {
        (Some(workspace), Some(changed)) => (workspace, changed),
        _ => {
            eprintln!("usage: v2-gauntlet --self-test | --workspace DIR --changed FILE [--seed N] [--order-runs N] [--receipt-out PATH]");
            std::process::exit(2);
        }
    };
    if !workspace.is_dir() {
        eprintln!("workspace missing: {}", workspace.display());
        std::process::exit(2);
    }
    let changed_files = match read_changed_file(&changed_path) {
        Ok(files) => files,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let (receipt, code) = run_workspace(&workspace, &changed_files, seed, order_runs);
    match receipt_out {
        Some(path) => {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&path, receipt.clone() + "\n");
            println!("wrote {}", path.display());
        }
        None => println!("{receipt}"),
    }
    std::process::exit(code);
}
