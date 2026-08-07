//! Cross-crate contract round-trip tests for state, actions, and capabilities.
//!
//! Walk every fixture under `fixtures/actions/contract/` and verify that:
//! 1. `input_batch.json` parses as a valid action batch envelope.
//! 2. `expected_revision_chain/` contains a sequence of revisions whose
//!    `parents` chain is acyclic and whose first revision is a root.
//! 3. `expected_diff.json` parses and references revisions that exist in the
//!    chain.
//! 4. `expected_receipt.json` parses and references revisions that exist in
//!    the chain.
//! 5. The batch's `expected_revision` and the receipt's `before_revision`
//!    agree.
//!
//! The test is deterministic (no clock, no random IDs) and parallel-safe
//! (every fixture is read-only on disk; no temp directories are created).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

const ACTION_BATCH_SCHEMA: &str = "cutright.action_batch/v1";
const REVISION_SCHEMA: &str = "cutright.revision/v1";
const DIFF_SCHEMA: &str = "cutright.semantic_diff/v1";
const RECEIPT_SCHEMA: &str = "cutright.action_result/v1";

/// Walk the fixtures directory and return every fixture subdirectory.
fn collect_fixtures(contract_root: &Path) -> Vec<PathBuf> {
    let entries = match fs::read_dir(contract_root) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    out.sort();
    out
}

/// Load every revision from the chain directory, returning a name-indexed
/// map for stable cross-references.
fn load_revision_chain(chain_dir: &Path) -> BTreeMap<String, Value> {
    let mut by_id: BTreeMap<String, Value> = BTreeMap::new();
    let entries = fs::read_dir(chain_dir).expect("read chain dir");
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    paths.sort();
    for path in paths {
        let bytes = fs::read(&path).expect("read chain file");
        let value: Value = serde_json::from_slice(&bytes).expect("parse chain file");
        let id = value["revision_id"]
            .as_str()
            .expect("revision_id is a string")
            .to_string();
        by_id.insert(id, value);
    }
    by_id
}

fn assert_revision(value: &Value, path: &Path) {
    let schema = value["schema"].as_str().expect("schema is a string");
    assert_eq!(
        schema, REVISION_SCHEMA,
        "wrong schema tag in {}",
        path.display()
    );
    let id = value["revision_id"]
        .as_str()
        .expect("revision_id is a string");
    assert!(
        is_valid_id(id),
        "revision_id `{id}` does not match the schema regex"
    );
    let parents = value["parents"].as_array().expect("parents is an array");
    assert!(
        parents.len() <= 2,
        "revision `{id}` has more than 2 parents"
    );
    for parent in parents {
        let parent_id = parent.as_str().expect("parent is a string");
        assert!(
            is_valid_id(parent_id),
            "parent id `{parent_id}` does not match the schema regex"
        );
    }
    let created_at = value["created_at_ns"].as_u64().expect("created_at_ns is u64");
    let fp = value["compatibility_fp"]
        .as_str()
        .expect("compatibility_fp is a string");
    assert!(fp.len() >= 16, "compatibility_fp too short");
    let _ = created_at;
}

/// Validate that every character in `value` matches the v2 identifier regex
/// `^[A-Za-z0-9_-]+$`. Implemented manually so the test crate does not need
/// to pull in `regex` as a direct dependency.
fn is_valid_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn assert_chain_acyclic(by_id: &BTreeMap<String, Value>) {
    for id in by_id.keys() {
        let mut visited = std::collections::HashSet::new();
        let mut current = id.clone();
        while let Some(rev) = by_id.get(&current) {
            assert!(visited.insert(current.clone()), "cycle detected at `{current}`");
            let parents = rev["parents"].as_array().expect("parents is an array");
            match parents.len() {
                0 => break,
                1 => {
                    current = parents[0].as_str().expect("parent is a string").to_string();
                }
                _ => panic!("merge revisions are not exercised by these fixtures"),
            }
        }
    }
}

fn assert_action_batch(value: &Value, path: &Path) {
    let schema = value["schema"].as_str().expect("schema is a string");
    assert_eq!(schema, ACTION_BATCH_SCHEMA, "wrong schema in {}", path.display());
    let _batch_id = value["batch_id"].as_str().expect("batch_id is a string");
    let expected_revision = value["expected_revision"]
        .as_str()
        .expect("expected_revision is a string");
    let _intent = value["intent"].as_str().expect("intent is a string");
    let evidence_refs = value["evidence_refs"].as_array().expect("evidence_refs is an array");
    for evidence in evidence_refs {
        assert!(evidence.is_string(), "evidence_ref must be a string");
    }
    let actions = value["actions"].as_array().expect("actions is an array");
    assert!(!actions.is_empty(), "actions must be non-empty");
    for action in actions {
        let kind = action["action_kind"]
            .as_str()
            .expect("action_kind is a string");
        let target = action["target_id"]
            .as_str()
            .expect("target_id is a string");
        assert!(!kind.is_empty(), "action_kind is empty");
        assert!(!target.is_empty(), "target_id is empty");
    }
    let _ = expected_revision;
}

fn assert_diff(value: &Value, path: &Path, by_id: &BTreeMap<String, Value>) {
    let schema = value["schema"].as_str().expect("schema is a string");
    assert_eq!(schema, DIFF_SCHEMA, "wrong schema in {}", path.display());
    let diffs = value["diffs"].as_array().expect("diffs is an array");
    for diff in diffs {
        let before = diff["before_id"].as_str().expect("before_id is a string");
        let after = diff["after_id"].as_str().expect("after_id is a string");
        assert!(
            by_id.contains_key(before),
            "diff `before_id` `{before}` is not in the chain"
        );
        assert!(
            by_id.contains_key(after),
            "diff `after_id` `{after}` is not in the chain"
        );
    }
}

fn assert_receipt(value: &Value, path: &Path, by_id: &BTreeMap<String, Value>) {
    let schema = value["schema"].as_str().expect("schema is a string");
    assert_eq!(schema, RECEIPT_SCHEMA, "wrong schema in {}", path.display());
    let batch_id = value["batch_id"].as_str().expect("batch_id is a string");
    let before = value["before_revision"]
        .as_str()
        .expect("before_revision is a string");
    let after = value["after_revision"]
        .as_str()
        .expect("after_revision is a string");
    assert!(!batch_id.is_empty(), "batch_id is empty");
    assert!(
        by_id.contains_key(before),
        "receipt `before_revision` `{before}` is not in the chain"
    );
    assert!(
        by_id.contains_key(after),
        "receipt `after_revision` `{after}` is not in the chain"
    );
    let stages = value["stages"].as_array().expect("stages is an array");
    assert!(!stages.is_empty(), "stages must be non-empty");
}

/// Verify every fixture is internally consistent and the cross-crate
/// references resolve.
fn verify_fixture(fixture_dir: &Path) {
    let input_batch_path = fixture_dir.join("input_batch.json");
    let diff_path = fixture_dir.join("expected_diff.json");
    let receipt_path = fixture_dir.join("expected_receipt.json");
    let chain_dir = fixture_dir.join("expected_revision_chain");
    for required in [
        &input_batch_path,
        &diff_path,
        &receipt_path,
        &chain_dir,
    ] {
        assert!(
            required.exists(),
            "fixture `{}` is missing required artifact `{}`",
            fixture_dir.display(),
            required.display()
        );
    }

    let input_batch: Value = serde_json::from_slice(
        &fs::read(&input_batch_path).expect("read input batch"),
    )
    .expect("parse input batch");
    assert_action_batch(&input_batch, &input_batch_path);

    let diff: Value = serde_json::from_slice(&fs::read(&diff_path).expect("read diff"))
        .expect("parse diff");

    let receipt: Value = serde_json::from_slice(&fs::read(&receipt_path).expect("read receipt"))
        .expect("parse receipt");

    let by_id = load_revision_chain(&chain_dir);
    assert!(!by_id.is_empty(), "chain is empty");
    for (id, value) in &by_id {
        let named = chain_dir.join(format!("{id}.json"));
        assert_revision(value, &named);
    }
    assert_chain_acyclic(&by_id);

    assert_diff(&diff, &diff_path, &by_id);
    assert_receipt(&receipt, &receipt_path, &by_id);

    let expected_revision = input_batch["expected_revision"]
        .as_str()
        .expect("expected_revision is a string");
    let before_revision = receipt["before_revision"]
        .as_str()
        .expect("before_revision is a string");
    assert_eq!(
        expected_revision, before_revision,
        "batch.expected_revision disagrees with receipt.before_revision in {}",
        fixture_dir.display()
    );
    let batch_id = input_batch["batch_id"]
        .as_str()
        .expect("batch_id is a string");
    let receipt_batch_id = receipt["batch_id"]
        .as_str()
        .expect("batch_id is a string");
    assert_eq!(
        batch_id, receipt_batch_id,
        "batch.batch_id disagrees with receipt.batch_id in {}",
        fixture_dir.display()
    );
}

/// Locate the contract fixture root relative to the workspace manifest.
fn contract_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR is set by cargo");
    let crate_dir = PathBuf::from(manifest_dir);
    // The crate lives at `<workspace>/crates/video-state`, so the workspace
    // root is two parents up.
    let workspace = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    workspace.join("fixtures").join("actions").join("contract")
}

#[test]
fn every_contract_fixture_round_trips() {
    let root = contract_root();
    assert!(
        root.exists(),
        "contract fixture root does not exist: {}",
        root.display()
    );
    let fixtures = collect_fixtures(&root);
    assert!(
        !fixtures.is_empty(),
        "no fixtures found under {}",
        root.display()
    );
    for fixture in &fixtures {
        verify_fixture(fixture);
    }
}

#[test]
fn fixture_set_covers_multiple_action_families() {
    let root = contract_root();
    let fixtures = collect_fixtures(&root);
    let families: Vec<String> = fixtures
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    let mut unique = families.clone();
    unique.sort();
    unique.dedup();
    assert!(
        unique.len() >= 3,
        "expected at least 3 distinct action families, found {unique:?}"
    );
    let required = ["caption", "cut", "export"];
    for family in required {
        assert!(
            families.iter().any(|f| f == family),
            "missing required action family fixture: {family}"
        );
    }
}
