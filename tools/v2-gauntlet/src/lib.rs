//! CutRight v2 evidence gauntlet — core engine.
//!
//! Concept adaptation of the workspace evidence gauntlet at pin
//! 6ee21f03a787e7b57dc412760a8996ea7a235302 (source_id "workspace-capabilities",
//! tools/gauntlet): diff-scoped three-layer hardening — changed-line mutation
//! testing, changed-line coverage, and seeded test-order randomisation —
//! reimplemented in pure std for the CutRight local toolchain. Local-only:
//! receipts are written to disk; there is no CI or hosted-service integration.
//!
//! Anti-gaming constraints, enforced in code:
//! - an unrun check is never `Passed` (it is `Unproven` or `Skipped`);
//! - unsupported mutation shapes are `Skipped` with a reason, never ignored;
//! - a surviving mutant always fails the mutation layer;
//! - mutated sources live in a throwaway copy; the original tree is untouched.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// Outcome of one gauntlet layer. `Unproven` means the layer could not be
/// executed (e.g. backend missing) and must never be read as a pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerStatus {
    Passed,
    Failed,
    Skipped { reason: String },
    Unproven { reason: String },
}

impl LayerStatus {
    pub fn name(&self) -> &'static str {
        match self {
            LayerStatus::Passed => "passed",
            LayerStatus::Failed => "failed",
            LayerStatus::Skipped { .. } => "skipped",
            LayerStatus::Unproven { .. } => "unproven",
        }
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            LayerStatus::Skipped { reason } | LayerStatus::Unproven { reason } => {
                Some(reason.as_str())
            }
            _ => None,
        }
    }

    pub fn is_gate_failure(&self) -> bool {
        matches!(self, LayerStatus::Failed)
    }

    pub fn to_json(&self) -> String {
        match self {
            LayerStatus::Passed => "{\"status\":\"passed\"}".to_string(),
            LayerStatus::Failed => "{\"status\":\"failed\"}".to_string(),
            LayerStatus::Skipped { reason } => {
                format!("{{\"status\":\"skipped\",\"reason\":{}}}", json_string(reason))
            }
            LayerStatus::Unproven { reason } => {
                format!("{{\"status\":\"unproven\",\"reason\":{}}}", json_string(reason))
            }
        }
    }
}

/// Supported source languages for changed-line mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    TypeScript,
}

pub fn language_for_path(path: &Path) -> Option<Language> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => Some(Language::Rust),
        Some("ts") | Some("tsx") | Some("mts") => Some(Language::TypeScript),
        _ => None,
    }
}

/// xorshift64 — the seeded PRNG driving deterministic test-order shuffles.
#[derive(Debug, Clone)]
pub struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    pub fn new(seed: u64) -> Self {
        Self { state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed } }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Uniform-ish integer in [0, bound).
    pub fn next_below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            return 0;
        }
        self.next_u64() % bound
    }
}

/// Seeded Fisher–Yates; the same seed always reproduces the same order.
pub fn fisher_yates<T: Clone>(items: &[T], seed: u64) -> Vec<T> {
    let mut out = items.to_vec();
    let mut rng = Xorshift64::new(seed);
    if out.len() < 2 {
        return out;
    }
    for i in (1..out.len()).rev() {
        let j = rng.next_below((i + 1) as u64) as usize;
        out.swap(i, j);
    }
    out
}

/// One syntactic fault transformation, applied to a single changed line.
#[derive(Debug, Clone, Copy)]
pub struct MutantRule {
    pub id: &'static str,
    pub language: Language,
}

impl MutantRule {
    /// Returns the mutated line, or None when the rule does not change it.
    pub fn apply(&self, line: &str) -> Option<String> {
        let mutated = match (self.id, self.language) {
            ("literal-zero", Language::Rust) => rewrite_return_literal(line, "0"),
            ("literal-zero", Language::TypeScript) => rewrite_return_literal(line, "0"),
            ("neg-flip", Language::Rust) => rewrite_return_negate(line),
            ("neg-flip", Language::TypeScript) => rewrite_return_negate(line),
            ("eq-flip", Language::Rust) => replace_once(line, " == ", " != "),
            ("neq-flip", Language::TypeScript) => {
                replace_once(line, "===", "!==").or_else(|| replace_once(line, "!==", "==="))
            }
            ("and-or-swap", Language::Rust) => {
                replace_once(line, "&&", "||").or_else(|| replace_once(line, "||", "&&"))
            }
            ("and-or-swap", Language::TypeScript) => {
                replace_once(line, "&&", "||").or_else(|| replace_once(line, "||", "&&"))
            }
            ("ge-flip", Language::Rust) => replace_once(line, " > ", " >= "),
            ("lt-flip", Language::Rust) => replace_once(line, " < ", " <= "),
            _ => None,
        };
        match mutated {
            Some(m) if m != line => Some(m),
            _ => None,
        }
    }
}

fn replace_once(line: &str, from: &str, to: &str) -> Option<String> {
    let index = line.find(from)?;
    let mut out = String::with_capacity(line.len() + to.len());
    out.push_str(&line[..index]);
    out.push_str(to);
    out.push_str(&line[index + from.len()..]);
    Some(out)
}

/// `return <integer>;` → `return <replacement>;`
fn rewrite_return_literal(line: &str, replacement: &str) -> Option<String> {
    let trimmed = line.trim();
    let body = trimmed.strip_prefix("return")?.trim_start();
    let (value, tail) = match body.strip_suffix(';') {
        Some(b) => (b.trim(), ";"),
        None => (body.trim(), ""),
    };
    if value.is_empty() || !value.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if value == replacement {
        return None;
    }
    let indent = &line[..line.len() - line.trim_start().len()];
    Some(format!("{indent}return {replacement}{tail}"))
}

/// `return <expr>;` → `return !(<expr>);` (non-integer expressions only).
fn rewrite_return_negate(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let body = trimmed.strip_prefix("return")?.trim_start();
    let (value, tail) = match body.strip_suffix(';') {
        Some(b) => (b.trim(), ";"),
        None => (body.trim(), ""),
    };
    if value.is_empty() || value.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let indent = &line[..line.len() - line.trim_start().len()];
    Some(format!("{indent}return !({value}){tail}"))
}

/// Ordered rule table per language; the first rule that changes the line wins
/// (workspace gauntlet behaviour, re-expressed for Rust/TypeScript).
pub fn rules_for(language: Language) -> Vec<MutantRule> {
    let mut rules = vec![
        MutantRule { id: "literal-zero", language },
        MutantRule { id: "neg-flip", language },
    ];
    match language {
        Language::Rust => {
            rules.push(MutantRule { id: "eq-flip", language });
            rules.push(MutantRule { id: "ge-flip", language });
            rules.push(MutantRule { id: "lt-flip", language });
        }
        Language::TypeScript => {
            rules.push(MutantRule { id: "neq-flip", language });
        }
    }
    rules.push(MutantRule { id: "and-or-swap", language });
    rules
}

/// One changed file with its 1-based changed line numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    pub path: String,
    pub lines: Vec<u64>,
}

/// Outcome of a single mutant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutantOutcome {
    Killed,
    Survived,
    Skipped { reason: String },
    Unproven { reason: String },
}

#[derive(Debug, Clone)]
pub struct MutantResult {
    pub file: String,
    pub line: u64,
    pub mutator: String,
    pub mutated_line: String,
    pub outcome: MutantOutcome,
}

/// Backend abstraction: how tests for a workspace copy are executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Cargo,
    NodeTest,
}

pub fn backend_available(kind: BackendKind) -> bool {
    let output = match kind {
        BackendKind::Cargo => Command::new("cargo").arg("--version").output(),
        BackendKind::NodeTest => Command::new("node").arg("--version").output(),
    };
    matches!(output, Ok(out) if out.status.success())
}

fn backend_for(language: Language) -> BackendKind {
    match language {
        Language::Rust => BackendKind::Cargo,
        Language::TypeScript => BackendKind::NodeTest,
    }
}

/// Execute a command with a deadline (pure std polling; no external crates).
fn run_with_deadline(
    program: &str,
    args: &[String],
    cwd: &Path,
    env: &[(String, String)],
    deadline: Duration,
) -> Option<bool> {
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => return None,
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status.success()),
            Ok(None) => {
                if start.elapsed() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return None,
        }
    }
}

/// Recursively copy a directory tree (pure std).
pub fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    let mut entries: Vec<_> = fs::read_dir(from)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name();
        if name == "target" || name == "node_modules" || name == ".git" {
            continue;
        }
        let source = entry.path();
        let dest = to.join(&name);
        if source.is_dir() {
            copy_dir(&source, &dest)?;
        } else {
            fs::copy(&source, &dest)?;
        }
    }
    Ok(())
}

fn scratch_dir(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "v2-gauntlet-{}-{}-{}",
        tag,
        std::process::id(),
        next_counter()
    ));
    fs::create_dir_all(&base).expect("scratch dir");
    base
}

fn next_counter() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Mutation layer: generate one mutant per changed line (first matching rule),
/// execute tests against a throwaway copy, classify killed/survived.
pub fn run_mutation_layer(
    workspace: &Path,
    changed: &[ChangedFile],
    deadline: Duration,
) -> (LayerStatus, Vec<MutantResult>) {
    let session = scratch_dir("session");
    let shared_target = session.join("gauntlet-target");
    let mut results: Vec<MutantResult> = Vec::new();
    for file in changed {
        let rel = Path::new(&file.path);
        let language = match language_for_path(rel) {
            Some(language) => language,
            None => {
                for line in &file.lines {
                    results.push(MutantResult {
                        file: file.path.clone(),
                        line: *line,
                        mutator: "none".to_string(),
                        mutated_line: String::new(),
                        outcome: MutantOutcome::Skipped {
                            reason: "unsupported_file_type".to_string(),
                        },
                    });
                }
                continue;
            }
        };
        let source_path = workspace.join(rel);
        let original = match fs::read_to_string(&source_path) {
            Ok(text) => text,
            Err(_) => {
                for line in &file.lines {
                    results.push(MutantResult {
                        file: file.path.clone(),
                        line: *line,
                        mutator: "none".to_string(),
                        mutated_line: String::new(),
                        outcome: MutantOutcome::Skipped {
                            reason: "missing_locator".to_string(),
                        },
                    });
                }
                continue;
            }
        };
        let lines: Vec<&str> = original.lines().collect();
        let backend = backend_for(language);
        let backend_ok = backend_available(backend);
        for line_no in &file.lines {
            let index = *line_no as usize;
            let make_skip = |reason: String| MutantResult {
                file: file.path.clone(),
                line: *line_no,
                mutator: "none".to_string(),
                mutated_line: String::new(),
                outcome: MutantOutcome::Skipped { reason },
            };
            if index == 0 || index > lines.len() {
                results.push(make_skip("out_of_range".to_string()));
                continue;
            }
            let line = lines[index - 1];
            let rules = rules_for(language);
            let rule = rules
                .iter()
                .find_map(|rule| rule.apply(line).map(|mutated| (rule, mutated)));
            let (rule, mutated_line) = match rule {
                Some(pair) => pair,
                None => {
                    results.push(make_skip("unsupported_mutation_shape".to_string()));
                    continue;
                }
            };
            if !backend_ok {
                results.push(MutantResult {
                    file: file.path.clone(),
                    line: *line_no,
                    mutator: rule.id.to_string(),
                    mutated_line,
                    outcome: MutantOutcome::Unproven {
                        reason: format!("test backend unavailable: {:?}", backend),
                    },
                });
                continue;
            }
            let outcome = execute_mutant(
                &session,
                &shared_target,
                workspace,
                rel,
                &original,
                index - 1,
                &mutated_line,
                backend,
                deadline,
            );
            results.push(MutantResult {
                file: file.path.clone(),
                line: *line_no,
                mutator: rule.id.to_string(),
                mutated_line,
                outcome,
            });
        }
    }
    let _ = fs::remove_dir_all(&session);
    let status = mutation_status(&results);
    (status, results)
}

#[allow(clippy::too_many_arguments)]
fn execute_mutant(
    session: &Path,
    shared_target: &Path,
    workspace: &Path,
    rel: &Path,
    original: &str,
    line_index: usize,
    mutated_line: &str,
    backend: BackendKind,
    deadline: Duration,
) -> MutantOutcome {
    let scratch = session.join(format!("mutant-{}", next_counter()));
    if copy_dir(workspace, &scratch).is_err() {
        let _ = fs::remove_dir_all(&scratch);
        return MutantOutcome::Unproven { reason: "scratch_copy_failed".to_string() };
    }
    let mut mutated_lines: Vec<String> = original.lines().map(String::from).collect();
    mutated_lines[line_index] = mutated_line.to_string();
    let target = scratch.join(rel);
    if fs::write(&target, mutated_lines.join("\n") + "\n").is_err() {
        let _ = fs::remove_dir_all(&scratch);
        return MutantOutcome::Unproven { reason: "scratch_write_failed".to_string() };
    }
    let manifest = find_manifest(&scratch, rel);
    let ran = match backend {
        BackendKind::Cargo => {
            let manifest = match manifest {
                Some(manifest) => manifest,
                None => {
                    let _ = fs::remove_dir_all(&scratch);
                    return MutantOutcome::Unproven {
                        reason: "no_cargo_manifest_for_changed_file".to_string(),
                    };
                }
            };
            let target_dir = shared_target;
            run_with_deadline(
                "cargo",
                &["test".to_string(), "--manifest-path".to_string(), manifest.to_string_lossy().to_string(), "--locked".to_string(), "--quiet".to_string()],
                &scratch,
                &[("CARGO_TARGET_DIR".to_string(), target_dir.to_string_lossy().to_string())],
                deadline,
            )
        }
        BackendKind::NodeTest => run_with_deadline(
            "node",
            &["--test".to_string()],
            &scratch,
            &[],
            deadline,
        ),
    };
    let _ = fs::remove_dir_all(&scratch);
    match ran {
        Some(true) => MutantOutcome::Survived,
        Some(false) => MutantOutcome::Killed,
        None => MutantOutcome::Unproven { reason: "test_run_timed_out_or_failed".to_string() },
    }
}

/// Find the nearest Cargo.toml above (or beside) the changed file in the copy.
fn find_manifest(scratch: &Path, rel: &Path) -> Option<PathBuf> {
    let mut dir = rel.parent().map(PathBuf::from).unwrap_or_default();
    loop {
        let candidate = scratch.join(&dir).join("Cargo.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => {
                let fallback = scratch.join("Cargo.toml");
                return fallback.is_file().then_some(fallback);
            }
        }
    }
}

pub fn mutation_status(results: &[MutantResult]) -> LayerStatus {
    let survived = results
        .iter()
        .filter(|r| matches!(r.outcome, MutantOutcome::Survived))
        .count();
    let killed = results
        .iter()
        .filter(|r| matches!(r.outcome, MutantOutcome::Killed))
        .count();
    let unproven = results
        .iter()
        .filter(|r| matches!(r.outcome, MutantOutcome::Unproven { .. }))
        .count();
    let mutants = results
        .iter()
        .filter(|r| r.mutator != "none")
        .count();
    if survived > 0 {
        return LayerStatus::Failed;
    }
    if mutants == 0 {
        return LayerStatus::Skipped { reason: "no_applicable_mutators_on_changed_lines".to_string() };
    }
    if unproven > 0 {
        return LayerStatus::Unproven {
            reason: format!("{unproven} mutant(s) could not be executed"),
        };
    }
    if killed > 0 {
        LayerStatus::Passed
    } else {
        LayerStatus::Skipped { reason: "no_mutants_executed".to_string() }
    }
}

/// Coverage layer. The instrumenting backend is optional; when it is absent
/// the layer is `Unproven`, never `Passed`.
pub fn run_coverage_layer(workspace: &Path, changed: &[ChangedFile]) -> LayerStatus {
    if !backend_available(BackendKind::Cargo) {
        return LayerStatus::Unproven { reason: "cargo unavailable; cannot probe coverage backend".to_string() };
    }
    let probe = Command::new("cargo")
        .args(["llvm-cov", "--version"])
        .output();
    let present = matches!(probe, Ok(out) if out.status.success());
    if !present {
        return LayerStatus::Unproven {
            reason: "coverage backend unavailable: cargo-llvm-cov not installed".to_string(),
        };
    }
    // Best-effort instrumented run; any inconclusive step degrades to Unproven.
    let manifest = workspace.join("Cargo.toml");
    if !manifest.is_file() {
        return LayerStatus::Unproven { reason: "no Cargo.toml at workspace root".to_string() };
    }
    let ran = run_with_deadline(
        "cargo",
        &[
            "llvm-cov".to_string(),
            "run".to_string(),
            "--locked".to_string(),
            "--manifest-path".to_string(),
            manifest.to_string_lossy().to_string(),
        ],
        workspace,
        &[],
        Duration::from_secs(240),
    );
    if ran != Some(true) {
        return LayerStatus::Unproven { reason: "instrumented test run inconclusive".to_string() };
    }
    match changed_line_coverage(workspace, changed) {
        Some(percent) => {
            if percent >= 100.0 {
                LayerStatus::Passed
            } else {
                LayerStatus::Failed
            }
        }
        None => LayerStatus::Unproven { reason: "coverage report parse inconclusive".to_string() },
    }
}

/// Parse `cargo llvm-cov report --json` segments and compute the fraction of
/// changed executable lines hit at least once. Conservative: any uncertainty
/// returns None (→ Unproven upstream).
fn changed_line_coverage(workspace: &Path, changed: &[ChangedFile]) -> Option<f64> {
    let manifest = workspace.join("Cargo.toml");
    let output = Command::new("cargo")
        .args([
            "llvm-cov",
            "report",
            "--json",
            "--manifest-path",
            manifest.to_str()?,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let mut total = 0u64;
    let mut covered = 0u64;
    // Segments format: [[line, col, count, hasCount, isRegionEntry], ...]
    if !text.contains("\"segments\":") {
        return None;
    }
    // Minimal scan: for each changed line, look for a segment entry starting at
    // that line whose count is positive. If a changed line has no segment we
    // treat it as non-executable (comments/signatures) and skip it.
    for file in changed {
        for line in &file.lines {
            let needle = format!("[{line},");
            if !text.contains(&needle) {
                continue;
            }
            total += 1;
            let count_positive = text
                .split(&needle)
                .skip(1)
                .filter_map(|rest| rest.split_once(']').map(|(head, _)| head))
                .any(|head| {
                    // head looks like "12,3,1,true,true" → third field is count
                    let fields: Vec<&str> = head.split(',').collect();
                    fields
                        .get(2)
                        .map(|count| count.trim().parse::<u64>().map(|n| n > 0).unwrap_or(false))
                        .unwrap_or(false)
                });
            if count_positive {
                covered += 1;
            }
        }
    }
    if total == 0 {
        return None;
    }
    Some((covered as f64 / total as f64) * 100.0)
}

/// Test-order layer: seeded Fisher–Yates orders, recorded and reproducible.
pub fn run_order_layer(
    workspace: &Path,
    seed: u64,
    runs: usize,
    deadline: Duration,
) -> (LayerStatus, Vec<Vec<String>>, Vec<u64>) {
    let tests = discover_tests(workspace);
    if tests.is_empty() {
        return (
            LayerStatus::Skipped { reason: "no_test_files_discovered".to_string() },
            Vec::new(),
            Vec::new(),
        );
    }
    let target_dir = scratch_dir("order-target");
    let mut orders = Vec::new();
    let mut run_seeds = Vec::new();
    for run in 0..runs {
        let run_seed = seed.wrapping_add(run as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let order = fisher_yates(&tests, run_seed);
        orders.push(order);
        run_seeds.push(run_seed);
    }
    if !backend_available(BackendKind::Cargo) || !workspace.join("Cargo.toml").is_file() {
        return (
            LayerStatus::Unproven { reason: "test backend unavailable for order runs".to_string() },
            orders,
            run_seeds,
        );
    }
    let mut failures = 0usize;
    for order in &orders {
        let env_value = order.join(",");
        let ok = run_with_deadline(
            "cargo",
            &[
                "test".to_string(),
                "--locked".to_string(),
                "--quiet".to_string(),
                "--".to_string(),
                "--test-threads=1".to_string(),
            ],
            workspace,
            &[
                ("GAUNTLET_TEST_ORDER".to_string(), env_value),
                ("CARGO_TARGET_DIR".to_string(), target_dir.to_string_lossy().to_string()),
            ],
            deadline,
        );
        if ok != Some(true) {
            failures += 1;
        }
    }
    let _ = fs::remove_dir_all(&target_dir);
    let status = if failures == 0 {
        LayerStatus::Passed
    } else {
        LayerStatus::Failed
    };
    (status, orders, run_seeds)
}

/// Deterministic discovery: sorted tests/*.rs plus src files containing #[test].
pub fn discover_tests(workspace: &Path) -> Vec<String> {
    let mut tests: Vec<String> = Vec::new();
    let tests_dir = workspace.join("tests");
    if tests_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&tests_dir) {
            let mut names: Vec<String> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .filter(|name| name.ends_with(".rs"))
                .collect();
            names.sort();
            for name in names {
                tests.push(format!("tests/{name}"));
            }
        }
    }
    let src_dir = workspace.join("src");
    if src_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&src_dir) {
            let mut names: Vec<String> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .filter(|name| name.ends_with(".rs"))
                .collect();
            names.sort();
            for name in names {
                let path = src_dir.join(&name);
                if let Ok(text) = fs::read_to_string(path) {
                    if text.contains("#[test]") {
                        tests.push(format!("src/{name}"));
                    }
                }
            }
        }
    }
    tests.sort();
    tests
}

pub fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Render one mutant result as JSON (deterministic field order).
pub fn mutant_result_json(result: &MutantResult) -> String {
    let outcome = match &result.outcome {
        MutantOutcome::Killed => "\"killed\"".to_string(),
        MutantOutcome::Survived => "\"survived\"".to_string(),
        MutantOutcome::Skipped { reason } => {
            format!("{{\"skipped\":{}}}", json_string(reason))
        }
        MutantOutcome::Unproven { reason } => {
            format!("{{\"unproven\":{}}}", json_string(reason))
        }
    };
    format!(
        "{{\"file\":{},\"line\":{},\"mutator\":{},\"mutated_line\":{},\"outcome\":{}}}",
        json_string(&result.file),
        result.line,
        json_string(&result.mutator),
        json_string(&result.mutated_line),
        outcome
    )
}
