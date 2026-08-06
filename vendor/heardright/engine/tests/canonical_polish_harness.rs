use heardright_engine::canonical_polish_harness::{
    pending_long_rows, prepare_product_input, resolve_cleanup_outcome, select_rows, EvalInputRow,
};
use heardright_engine::l3_cleanup::CleanupOutcome;
use std::collections::HashSet;

#[test]
fn selects_only_strictly_over_15_seconds_and_resumes_completed_ids() {
    let rows = vec![
        EvalInputRow::new("short", 15.0, "short text"),
        EvalInputRow::new("long-a", 15.01, "long a"),
        EvalInputRow::new("long-b", 20.0, "long b"),
    ];
    let completed = HashSet::from(["long-a".to_string()]);
    let selected = pending_long_rows(&rows, 15.0, &completed);
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].id, "long-b");
}

#[test]
fn product_input_strips_ai_transform_tail_before_l0() {
    let prepared = prepare_product_input("the footer is broken summarize");
    assert_eq!(prepared.text, "the footer is broken");
    assert_eq!(prepared.ai_transform.as_deref(), Some("summarize"));
    assert!(!prepared.cancelled);
}

#[test]
fn cleanup_failure_keeps_local_text_and_error_provenance() {
    let resolved = resolve_cleanup_outcome(
        CleanupOutcome::Failed {
            error_class: "response_rejected",
            circuit_open: true,
        },
        "local",
    );
    assert_eq!(resolved.hypothesis, "local");
    assert_eq!(resolved.status, "failed_local_fallback");
    assert_eq!(resolved.reason, Some("response_rejected"));
    assert!(resolved.circuit_open);
}

#[test]
fn explicit_clip_filter_and_limit_are_deterministic() {
    let rows = vec![
        EvalInputRow::new("a", 16.0, "one two three four five six seven eight"),
        EvalInputRow::new("b", 17.0, "one two three four five six seven eight"),
        EvalInputRow::new("c", 18.0, "one two three four five six seven eight"),
    ];
    let wanted = HashSet::from(["b".to_string(), "c".to_string()]);
    let selected = select_rows(&rows, 15.0, &HashSet::new(), Some(&wanted), Some(1));
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].id, "b");
}
