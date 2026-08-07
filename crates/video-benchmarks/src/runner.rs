// Benchmark runner (Book 4 lane A, B4-011).
//
// Reads a corpus manifest, a profile, and pack locks; for each
// project runs the deterministic evaluators. Never mutates the
// evaluated projects — the runner is read-only against completed
// project revisions.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::AxisId;
use crate::EvalOutcome;
use crate::MetricStatus;

/// A single benchmark run summary for one project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectRun {
    pub project_id: String,
    pub split: String,
    pub profile: String,
    pub outcomes: Vec<EvalOutcome>,
    pub failures: u32,
    pub passes: u32,
}

/// Top-level run result for a runner invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkRun {
    pub run_id: String,
    pub profile: String,
    pub projects: Vec<ProjectRun>,
    pub started_at_ms: i64,
    pub finished_at_ms: i64,
}

/// Build an empty project-run record.
pub fn empty_project_run(project_id: &str, split: &str, profile: &str) -> ProjectRun {
    ProjectRun {
        project_id: project_id.to_string(),
        split: split.to_string(),
        profile: profile.to_string(),
        outcomes: Vec::new(),
        failures: 0,
        passes: 0,
    }
}

/// Tally a project run after outcomes have been collected.
pub fn tally(pr: &mut ProjectRun) {
    pr.passes = 0;
    pr.failures = 0;
    for o in &pr.outcomes {
        match o.status {
            MetricStatus::Pass => pr.passes += 1,
            MetricStatus::Fail => pr.failures += 1,
            _ => {}
        }
    }
}

/// Group outcomes by axis for slice reporting.
pub fn outcomes_by_axis(run: &ProjectRun) -> BTreeMap<String, Vec<&EvalOutcome>> {
    let mut out: BTreeMap<String, Vec<&EvalOutcome>> = BTreeMap::new();
    for o in &run.outcomes {
        out.entry(axis_key(o.axis).to_string()).or_default().push(o);
    }
    out
}

fn axis_key(a: AxisId) -> &'static str {
    a.as_str()
}

/// Persist a `BenchmarkRun` as JSON; private fixture paths are redacted.
pub fn write_run(path: &Path, run: &BenchmarkRun) -> std::io::Result<()> {
    let redacted = redact_paths(run);
    let json = serde_json::to_string_pretty(&redacted)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, json)
}

fn redact_paths(run: &BenchmarkRun) -> BenchmarkRun {
    let mut r = run.clone();
    for p in &mut r.projects {
        for o in &mut p.outcomes {
            for ev in &mut o.evidence {
                for f in &mut ev.frame_refs {
                    if f.contains("private-fixtures/") {
                        *f = "[redacted]".to_string();
                    }
                }
            }
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(metric: &str, pass: bool) -> EvalOutcome {
        let mut o = if pass {
            EvalOutcome::pass(metric, AxisId::Editorial, 1.0, "ratio")
        } else {
            EvalOutcome::fail(metric, AxisId::Editorial, "test")
        };
        o.metric_id = metric.to_string();
        o
    }

    #[test]
    fn empty_tally_is_zero() {
        let mut p = empty_project_run("p", "test", "reviewed-v2");
        tally(&mut p);
        assert_eq!(p.passes, 0);
        assert_eq!(p.failures, 0);
    }

    #[test]
    fn tally_counts_passes_and_failures() {
        let mut p = empty_project_run("p", "test", "reviewed-v2");
        p.outcomes = vec![outcome("a", true), outcome("b", false), outcome("c", true)];
        tally(&mut p);
        assert_eq!(p.passes, 2);
        assert_eq!(p.failures, 1);
    }

    #[test]
    fn group_by_axis() {
        let mut p = empty_project_run("p", "test", "reviewed-v2");
        p.outcomes = vec![
            outcome("a", true),
            EvalOutcome::pass("b", AxisId::AudioVisual, 1.0, "ratio"),
        ];
        let g = outcomes_by_axis(&p);
        assert!(g.contains_key("editorial"));
        assert!(g.contains_key("audio_visual"));
    }

    #[test]
    fn redact_strips_private_paths() {
        let mut p = empty_project_run("p", "test", "reviewed-v2");
        let mut o = EvalOutcome::pass("a", AxisId::Editorial, 1.0, "ratio");
        o.evidence.push(crate::EvidenceRef {
            source_range: [0, 0],
            output_range: [0, 0],
            frame_refs: vec!["private-fixtures/foo.mp4#t=0".to_string()],
            word_ids: vec![],
        });
        p.outcomes = vec![o];
        let run = BenchmarkRun {
            run_id: "r1".into(),
            profile: "reviewed-v2".into(),
            projects: vec![p],
            started_at_ms: 0,
            finished_at_ms: 1,
        };
        let red = redact_paths(&run);
        assert!(red.projects[0].outcomes[0].evidence[0]
            .frame_refs
            .iter()
            .all(|f| f != "private-fixtures/foo.mp4#t=0"));
    }
}
