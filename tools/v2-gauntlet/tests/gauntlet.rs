//! Integration tests for the v2 gauntlet core (no nested cargo runs; the
//! end-to-end fixture behaviour is verified by `--self-test`).

use std::path::Path;

use v2_gauntlet::{
    fisher_yates, json_string, language_for_path, mutation_status, rules_for, ChangedFile,
    Language, LayerStatus, MutantOutcome, MutantResult, Xorshift64,
};

#[test]
fn layer_status_json_shapes() {
    assert_eq!(LayerStatus::Passed.to_json(), "{\"status\":\"passed\"}");
    assert_eq!(LayerStatus::Failed.to_json(), "{\"status\":\"failed\"}");
    let skipped = LayerStatus::Skipped { reason: "unsupported_mutation_shape".into() };
    assert_eq!(
        skipped.to_json(),
        "{\"status\":\"skipped\",\"reason\":\"unsupported_mutation_shape\"}"
    );
    let unproven = LayerStatus::Unproven { reason: "coverage backend unavailable".into() };
    assert!(unproven.to_json().starts_with("{\"status\":\"unproven\""));
    assert!(LayerStatus::Failed.is_gate_failure());
    assert!(!unproven.is_gate_failure());
    assert!(!skipped.is_gate_failure());
}

#[test]
fn seeded_order_is_reproducible_and_seed_dependent() {
    let items: Vec<String> = (0..16).map(|index| format!("test_{index}")).collect();
    let first = fisher_yates(&items, 7);
    let second = fisher_yates(&items, 7);
    let other = fisher_yates(&items, 8);
    assert_eq!(first, second);
    assert_ne!(first, other);
    let mut counts = std::collections::BTreeSet::new();
    for item in &first {
        assert!(counts.insert(item.clone()));
    }
    assert_eq!(counts.len(), items.len());
}

#[test]
fn xorshift_is_deterministic() {
    let mut a = Xorshift64::new(1234);
    let mut b = Xorshift64::new(1234);
    for _ in 0..8 {
        assert_eq!(a.next_u64(), b.next_u64());
    }
}

#[test]
fn languages_are_detected_from_extensions() {
    assert_eq!(language_for_path(Path::new("src/lib.rs")), Some(Language::Rust));
    assert_eq!(language_for_path(Path::new("a/b/c.ts")), Some(Language::TypeScript));
    assert_eq!(language_for_path(Path::new("a/b/c.tsx")), Some(Language::TypeScript));
    assert_eq!(language_for_path(Path::new("notes.md")), None);
    assert_eq!(language_for_path(Path::new("script.py")), None);
}

#[test]
fn rust_mutators_rewrite_changed_lines() {
    let rules = rules_for(Language::Rust);
    let apply_first = |line: &str| {
        rules
            .iter()
            .find_map(|rule| rule.apply(line).map(|mutated| (rule.id, mutated)))
    };
    assert_eq!(
        apply_first("        return 100;"),
        Some(("literal-zero", "        return 0;".to_string()))
    );
    assert_eq!(
        apply_first("    if value > max {"),
        Some(("ge-flip", "    if value >= max {".to_string()))
    );
    assert_eq!(
        apply_first("    if a == b {"),
        Some(("eq-flip", "    if a != b {".to_string()))
    );
    assert_eq!(
        apply_first("    if a && b {"),
        Some(("and-or-swap", "    if a || b {".to_string()))
    );
    assert_eq!(apply_first("    let unchanged = 1;"), None);
}

#[test]
fn typescript_mutators_rewrite_changed_lines() {
    let rules = rules_for(Language::TypeScript);
    let apply_first = |line: &str| {
        rules
            .iter()
            .find_map(|rule| rule.apply(line).map(|mutated| (rule.id, mutated)))
    };
    assert_eq!(
        apply_first("  if (a === b) {"),
        Some(("neq-flip", "  if (a !== b) {".to_string()))
    );
    assert_eq!(
        apply_first("  return flag && other;"),
        Some(("neg-flip", "  return !(flag && other);".to_string()))
    );
    assert_eq!(apply_first("  const x = 1;"), None);
}

fn result(outcome: MutantOutcome, mutator: &str) -> MutantResult {
    MutantResult {
        file: "src/lib.rs".to_string(),
        line: 1,
        mutator: mutator.to_string(),
        mutated_line: String::new(),
        outcome,
    }
}

#[test]
fn mutation_status_semantics() {
    // A survivor always fails the layer.
    let with_survivor = vec![
        result(MutantOutcome::Killed, "literal-zero"),
        result(MutantOutcome::Survived, "ge-flip"),
    ];
    assert_eq!(mutation_status(&with_survivor), LayerStatus::Failed);

    // All killed → passed.
    let all_killed = vec![
        result(MutantOutcome::Killed, "literal-zero"),
        result(MutantOutcome::Killed, "neg-flip"),
    ];
    assert_eq!(mutation_status(&all_killed), LayerStatus::Passed);

    // Nothing executed → unproven, never pass.
    let unproven = vec![result(
        MutantOutcome::Unproven { reason: "backend".into() },
        "literal-zero",
    )];
    assert!(matches!(mutation_status(&unproven), LayerStatus::Unproven { .. }));

    // Only skips (no mutators applied) → skipped with reason.
    let only_skips = vec![result(
        MutantOutcome::Skipped { reason: "unsupported_mutation_shape".into() },
        "none",
    )];
    assert!(matches!(mutation_status(&only_skips), LayerStatus::Skipped { .. }));
}

#[test]
fn changed_file_shape_is_stable() {
    let file = ChangedFile { path: "src/lib.rs".into(), lines: vec![5, 6] };
    assert_eq!(file.path, "src/lib.rs");
    assert_eq!(file.lines, vec![5, 6]);
}

#[test]
fn json_string_escapes_control_characters() {
    assert_eq!(json_string("a\"b\\c\nd"), "\"a\\\"b\\\\c\\nd\"");
}
