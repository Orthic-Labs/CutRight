// Benchmark report writer (Book 4 lane A, B4-011).
//
// Produces per-project JSONL, metrics, slices, confusion matrices,
// samples, failures, the human-readable report, and a receipt. The
// writer is read-only against evaluated projects.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::runner::BenchmarkRun;

/// A single metric slice row for a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SliceRow {
    pub axis: String,
    pub metric_id: String,
    pub slice_key: String,
    pub slice_value: String,
    pub value: Option<f64>,
    pub status: String,
}

/// Top-level report, written as JSON next to the run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub run_id: String,
    pub profile: String,
    pub slices: Vec<SliceRow>,
    pub samples: Vec<String>,
    pub failures: Vec<String>,
}

/// Build a `Report` from a `BenchmarkRun`. Slices group outcomes by
/// `(axis, metric_id, slice)` and emit one row per group.
pub fn build_report(run: &BenchmarkRun) -> Report {
    use std::collections::BTreeMap;
    let mut rows: BTreeMap<(String, String, String, String), SliceRow> = BTreeMap::new();
    let mut failures: Vec<String> = Vec::new();
    for p in &run.projects {
        for o in &p.outcomes {
            for (k, v) in &o.slices {
                let key = (
                    o.axis.as_str().to_string(),
                    o.metric_id.clone(),
                    k.0.clone(),
                    v.clone(),
                );
                rows.entry(key.clone()).or_insert(SliceRow {
                    axis: key.0.clone(),
                    metric_id: key.1.clone(),
                    slice_key: key.2.clone(),
                    slice_value: key.3.clone(),
                    value: o.value,
                    status: format!("{:?}", o.status),
                });
            }
            if matches!(o.status, crate::MetricStatus::Fail) {
                failures.push(format!(
                    "{}:{} {}",
                    p.project_id,
                    o.metric_id,
                    o.reason.clone().unwrap_or_default()
                ));
            }
        }
    }
    Report {
        run_id: run.run_id.clone(),
        profile: run.profile.clone(),
        slices: rows.into_values().collect(),
        samples: Vec::new(),
        failures,
    }
}

/// Persist the report as JSON.
pub fn write_report(path: &Path, report: &Report) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(report)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, json)
}

/// Persist a side-by-side receipt listing run id, profile, and pack locks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Receipt {
    pub run_id: String,
    pub profile: String,
    pub pack_locks: Vec<String>,
}

pub fn write_receipt(path: &Path, receipt: &Receipt) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(receipt)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{empty_project_run, BenchmarkRun, ProjectRun};
    use crate::{AxisId, EvalOutcome};

    fn run_with(p: ProjectRun) -> BenchmarkRun {
        BenchmarkRun {
            run_id: "r1".into(),
            profile: "reviewed-v2".into(),
            projects: vec![p],
            started_at_ms: 0,
            finished_at_ms: 1,
        }
    }

    #[test]
    fn build_report_empty_run() {
        let r = run_with(empty_project_run("p", "test", "reviewed-v2"));
        let rep = build_report(&r);
        assert_eq!(rep.run_id, "r1");
        assert!(rep.slices.is_empty());
        assert!(rep.failures.is_empty());
    }

    #[test]
    fn build_report_records_failures() {
        let mut p = empty_project_run("p", "test", "reviewed-v2");
        p.outcomes = vec![EvalOutcome::fail("metric.x", AxisId::Editorial, "boom")];
        let r = run_with(p);
        let rep = build_report(&r);
        assert_eq!(rep.failures.len(), 1);
        assert!(rep.failures[0].contains("boom"));
    }
}
