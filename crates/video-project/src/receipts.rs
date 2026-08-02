//! Stage-receipt emission and verification (hardening plan §10.4 / §6.1).
//!
//! Every canonical pipeline stage writes one [`video_core::StageReceipt`]
//! beside the artifact it produced, named `<artifact-file-name>.receipt.json`
//! (or, for the §6.1 per-variant package receipt, `<variant>.artifact-
//! receipt.json` beside the rough cut it binds). This module owns the shared
//! write helper every stage in `lib.rs` calls, plus `videoctl receipts
//! verify`, which re-hashes every recorded input/output on disk and reports
//! any receipt whose bindings no longer hold.
//!
//! Receipts are purely additive: they never change the shape or path of the
//! artifact they describe, and a stage that has not run yet simply has no
//! receipt file — there is nothing to migrate.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use video_core::StageReceipt;

use crate::io::{hash_file, read_json, relative_artifact_path, write_json_atomic};
use crate::ProjectError;

/// The receipt path for one artifact: `<artifact>.receipt.json` beside it, so
/// a receipt can never collide with, or be mistaken for, the artifact it
/// describes.
pub(crate) fn receipt_path_for(artifact: &Path) -> PathBuf {
    let mut name = artifact
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    name.push(".receipt.json");
    artifact.with_file_name(name)
}

/// Build a [`StageReceipt`] from real, on-disk bytes and write it beside its
/// artifact. `inputs`/`outputs` are hashed at the moment this runs — never
/// placeholders — so the receipt is bound to exactly the bytes that produced
/// it and exactly the bytes it produced.
pub(crate) fn write_stage_receipt<P: Serialize + ?Sized>(
    receipt_path: &Path,
    stage: &str,
    inputs: &[&Path],
    parameters: &P,
    toolchains: BTreeMap<String, String>,
    outputs: &[&Path],
) -> Result<StageReceipt, ProjectError> {
    let receipt = StageReceipt::build(
        stage,
        env!("CARGO_PKG_VERSION"),
        inputs,
        parameters,
        toolchains,
        outputs,
    )
    .map_err(|error| ProjectError::InvalidState(format!("stage receipt for {stage}: {error}")))?;
    write_json_atomic(receipt_path, &receipt)?;
    Ok(receipt)
}

/// The result of re-verifying one receipt's bindings against live bytes.
#[derive(Debug, Clone, Serialize)]
pub struct ReceiptCheck {
    pub receipt_path: String,
    pub stage: String,
    pub status: &'static str,
    pub failures: Vec<String>,
}

/// Full report from `videoctl receipts verify`.
#[derive(Debug, Clone, Serialize)]
pub struct ReceiptVerificationReport {
    pub schema_version: u32,
    pub status: &'static str,
    pub checked: usize,
    pub results: Vec<ReceiptCheck>,
}

/// Walk `project_path` for every `*.receipt.json` / `*.artifact-receipt.json`
/// file, re-hash each recorded input/output against the bytes currently on
/// disk, and report any binding that no longer holds.
pub fn verify_receipts(project_path: &Path) -> Result<ReceiptVerificationReport, ProjectError> {
    let project_path = project_path.canonicalize()?;
    let mut receipt_paths = Vec::new();
    collect_receipts(&project_path, &mut receipt_paths)?;
    receipt_paths.sort();

    let mut results = Vec::with_capacity(receipt_paths.len());
    for receipt_path in &receipt_paths {
        let receipt: StageReceipt = match read_json(receipt_path) {
            Ok(receipt) => receipt,
            Err(error) => {
                results.push(ReceiptCheck {
                    receipt_path: relative_artifact_path(&project_path, receipt_path),
                    stage: "unknown".into(),
                    status: "fail",
                    failures: vec![format!("receipt could not be parsed: {error}")],
                });
                continue;
            }
        };
        let mut failures = Vec::new();
        for input in &receipt.inputs {
            check_binding(
                Path::new(&input.path),
                &input.blake3,
                None,
                "input",
                &mut failures,
            );
        }
        for output in &receipt.outputs {
            check_binding(
                Path::new(&output.path),
                &output.blake3,
                Some(output.size),
                "output",
                &mut failures,
            );
        }
        let status = if failures.is_empty() { "pass" } else { "fail" };
        results.push(ReceiptCheck {
            receipt_path: relative_artifact_path(&project_path, receipt_path),
            stage: receipt.stage,
            status,
            failures,
        });
    }
    let overall = if results.iter().all(|result| result.status == "pass") {
        "pass"
    } else {
        "fail"
    };
    Ok(ReceiptVerificationReport {
        schema_version: 1,
        status: overall,
        checked: results.len(),
        results,
    })
}

fn check_binding(
    path: &Path,
    recorded_hash: &str,
    recorded_size: Option<u64>,
    label: &str,
    failures: &mut Vec<String>,
) {
    let actual_hash = match hash_file(path) {
        Ok(hash) => hash,
        Err(error) => {
            failures.push(format!("{label} {} unreadable: {error}", path.display()));
            return;
        }
    };
    if actual_hash != recorded_hash {
        failures.push(format!(
            "{label} {} blake3 mismatch: recorded {recorded_hash} live {actual_hash}",
            path.display()
        ));
    }
    if let Some(expected_size) = recorded_size {
        match fs::metadata(path) {
            Ok(metadata) if metadata.len() == expected_size => {}
            Ok(metadata) => failures.push(format!(
                "{label} {} size mismatch: recorded {expected_size} live {}",
                path.display(),
                metadata.len()
            )),
            Err(error) => failures.push(format!("{label} {} unreadable: {error}", path.display())),
        }
    }
}

fn collect_receipts(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), ProjectError> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_receipts(&path, out)?;
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.ends_with(".receipt.json") || name.ends_with("artifact-receipt.json")
            })
        {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn unique_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cutright-receipts-test-{label}-{unique}"));
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn receipt_path_sits_beside_its_artifact() {
        let artifact = Path::new("/project/edit/cut-plan-tight.json");
        assert_eq!(
            receipt_path_for(artifact),
            PathBuf::from("/project/edit/cut-plan-tight.json.receipt.json")
        );
    }

    #[test]
    fn verify_passes_on_untouched_bytes_and_fails_after_tampering() {
        let dir = unique_dir("verify");
        let input_path = dir.join("input.json");
        let output_path = dir.join("output.json");
        fs::write(&input_path, b"{\"a\":1}").unwrap();
        fs::write(&output_path, b"{\"b\":2}").unwrap();

        let receipt_path = receipt_path_for(&output_path);
        write_stage_receipt(
            &receipt_path,
            "test.stage",
            &[input_path.as_path()],
            &serde_json::json!({"k": "v"}),
            BTreeMap::new(),
            &[output_path.as_path()],
        )
        .unwrap();

        let report = verify_receipts(&dir).unwrap();
        assert_eq!(report.status, "pass");
        assert_eq!(report.checked, 1);
        assert_eq!(report.results[0].stage, "test.stage");

        // Tamper with the output after the receipt was written.
        fs::write(&output_path, b"{\"b\":999}").unwrap();
        let report = verify_receipts(&dir).unwrap();
        assert_eq!(report.status, "fail");
        assert!(report.results[0]
            .failures
            .iter()
            .any(|failure| failure.contains("blake3 mismatch")));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn identical_parameters_produce_identical_parameters_hash() {
        let dir = unique_dir("params");
        let input_path = dir.join("input.json");
        fs::write(&input_path, b"same-bytes").unwrap();
        let params = serde_json::json!({"variant": "tight", "gap_threshold_ms": 220});

        let first = write_stage_receipt(
            &dir.join("first.receipt.json"),
            "edit.cut_plan",
            &[input_path.as_path()],
            &params,
            BTreeMap::new(),
            &[],
        )
        .unwrap();
        let second = write_stage_receipt(
            &dir.join("second.receipt.json"),
            "edit.cut_plan",
            &[input_path.as_path()],
            &params,
            BTreeMap::new(),
            &[],
        )
        .unwrap();

        assert_eq!(first.parameters_blake3, second.parameters_blake3);
        fs::remove_dir_all(&dir).ok();
    }
}
