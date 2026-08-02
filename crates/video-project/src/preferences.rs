//! Preference learning (REV2 plan §15.7 Phase 9).
//!
//! Learns exclusively from **current, hash-bound, target-specific** decision
//! records already on disk at `<project>/feedback/decisions.jsonl` — the
//! ledger Studio appends via its `DecisionRecord` contract
//! (`apps/studio/src-tauri/src/decision_contract.rs` /
//! `decision_ledger.rs`). `video-project` cannot depend on the Studio Tauri
//! binary crate (it lives outside this Cargo workspace), so this module
//! independently parses the same on-disk JSONL contract and independently
//! reimplements Studio's `Current | StaleArtifact | MissingArtifact |
//! Superseded` classification. This is a deliberate, deterministic
//! duplication of a stable *data contract* (the JSONL shape and the classify
//! rule), not a logic fork: any record whose artifact hash no longer matches,
//! or that a later verdict on the same subject has superseded, is excluded
//! from training here exactly as Studio's own replay would exclude it from
//! being treated as the live verdict.
//!
//! # What this module refuses to do
//!
//! - It never emits a recommendation that cannot be traced back to specific
//!   `decision_id`s (binding rule 1).
//! - "Readiness" (REV2's "autonomy") is computed per format, from a versioned
//!   policy file, and never defaults to a global switch (binding rule 2). The
//!   output always carries `auto_apply_allowed: false` — this stage proposes,
//!   it never acts (binding rule 3).
//! - Below the policy's `min_decisions_per_axis` floor, an axis is reported
//!   `insufficient_data`, never a "recommendation" (binding rule 4).
//! - A split verdict (no option meeting `min_agreement_ratio`) is reported
//!   `conflict` with its full distribution, never averaged into a single
//!   number (binding rule 5).
//! - Preference axes the ledger's `reason` vocabulary does not yet capture
//!   (pause policy, effect density, SFX choices, hook/CTA structure) are
//!   reported `unsupported_by_ledger_schema` rather than stretched onto a
//!   loosely related reason. Post-publish retention has no producer anywhere
//!   in this pipeline yet; its *input contract* is defined at
//!   `schemas/retention-sample.schema.json` and joined here by exact
//!   artifact hash when a `<project>/feedback/retention.jsonl` happens to
//!   exist, but nothing here invents retention data.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::io::{hash_file, read_project_manifest, relative_artifact_path, write_json_atomic};
use crate::receipts::{receipt_path_for, write_stage_receipt};
use crate::ProjectError;

pub const PREFERENCE_SCHEMA_VERSION: u32 = 1;

const AUTONOMY_POLICY_V1_JSON: &str =
    include_str!("../../../schemas/preference-autonomy-policy.v1.json");

// ---------------------------------------------------------------------------
// Versioned policy (mirrors the `BenchmarkPolicy` embedded-JSON pattern in
// `video_core::benchmark_policy`, kept local here since this crate owns the
// only consumer and video-core is out of this change's scope).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReadinessTier {
    pub level: String,
    pub min_reviewed_projects: u32,
}

/// Preference-learning thresholds (REV2 plan §15.7). Bumping the policy means
/// adding a new versioned file (`preference-autonomy-policy.v2.json`) and a
/// new loader, never silently editing the numbers behind an existing
/// `policy_version` — same discipline as `BenchmarkPolicy`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreferenceAutonomyPolicy {
    pub policy_version: u32,
    #[serde(default)]
    pub description: String,
    /// Sample-size floor: fewer than this many citable current decisions on
    /// an axis means "not a preference yet" (binding rule 4).
    pub min_decisions_per_axis: u32,
    /// Minimum share the leading option must hold before it is reported as a
    /// recommendation rather than a conflict.
    pub min_agreement_ratio: f64,
    /// Ladder of advisory readiness levels, keyed by how many distinct
    /// reviewed projects have contributed a current decision to a format.
    /// Never consulted to gate an actual render — see `auto_apply_allowed`.
    pub readiness_tiers: Vec<ReadinessTier>,
}

impl PreferenceAutonomyPolicy {
    /// Load the embedded policy version 1. This is the only supported policy
    /// version today.
    pub fn v1() -> Self {
        serde_json::from_str(AUTONOMY_POLICY_V1_JSON)
            .expect("embedded preference-autonomy-policy.v1.json must be valid")
    }

    fn readiness_level(&self, reviewed_projects: u32) -> String {
        self.readiness_tiers
            .iter()
            .filter(|tier| reviewed_projects >= tier.min_reviewed_projects)
            .max_by_key(|tier| tier.min_reviewed_projects)
            .map(|tier| tier.level.clone())
            .unwrap_or_else(|| "review_required".to_string())
    }
}

// ---------------------------------------------------------------------------
// Local mirror of Studio's decision ledger contract (read-only consumer).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordStatus {
    Current,
    StaleArtifact,
    MissingArtifact,
    Superseded,
}

/// Minimal mirror of `apps/studio/src-tauri/src/decision_contract.rs`'s
/// `DecisionRecord`. Unknown/future fields are ignored by serde; fields this
/// module does not read are simply not declared.
#[derive(Debug, Clone, Deserialize)]
struct LedgerRecord {
    decision_id: String,
    ts: String,
    #[serde(default)]
    project_id: String,
    #[serde(default)]
    project_instance_id: String,
    kind: String,
    verdict: String,
    reason: String,
    #[serde(default)]
    subject: String,
    #[serde(default)]
    subject_blake3: Option<String>,
    #[serde(default)]
    variant: Option<String>,
    #[serde(default)]
    preset: Option<String>,
}

impl LedgerRecord {
    /// Studio's own immutable per-project identity when present, else the
    /// legacy folder-derived `project_id` — same fallback Studio uses.
    fn project_identity(&self) -> &str {
        if self.project_instance_id.is_empty() {
            &self.project_id
        } else {
            &self.project_instance_id
        }
    }
}

fn classify(root: &Path, record: &LedgerRecord) -> RecordStatus {
    if record.subject.is_empty() {
        return RecordStatus::MissingArtifact;
    }
    let subject_path = root.join(&record.subject);
    match hash_file(&subject_path) {
        Ok(hash) => {
            let hash = format!("blake3:{hash}");
            match &record.subject_blake3 {
                Some(expected) if *expected == hash => RecordStatus::Current,
                _ => RecordStatus::StaleArtifact,
            }
        }
        Err(_) => RecordStatus::MissingArtifact,
    }
}

/// Per-project ledger load result: only `current` records ever feed
/// learning; every exclusion is still counted so the output can show its
/// work (`sources[].excluded_*`).
struct ProjectLedger {
    decisions_path: Option<PathBuf>,
    ledger_blake3: Option<String>,
    current: Vec<LedgerRecord>,
    excluded_stale: usize,
    excluded_missing: usize,
    excluded_superseded: usize,
    malformed_lines: usize,
}

/// Read `<project>/feedback/decisions.jsonl`, classify every parseable line,
/// and apply Studio's same-subject supersede rule: a later `variant_verdict`
/// or `final_verdict` on the same `subject` supersedes an earlier one.
/// Missing ledger is not an error — a project with zero reviews contributes
/// nothing and is reported as such.
fn load_project_ledger(project_path: &Path) -> Result<ProjectLedger, ProjectError> {
    let path = project_path.join("feedback/decisions.jsonl");
    if !path.is_file() {
        return Ok(ProjectLedger {
            decisions_path: None,
            ledger_blake3: None,
            current: Vec::new(),
            excluded_stale: 0,
            excluded_missing: 0,
            excluded_superseded: 0,
            malformed_lines: 0,
        });
    }
    let ledger_blake3 = format!("blake3:{}", hash_file(&path)?);
    let file = File::open(&path)?;
    let mut parsed: Vec<(LedgerRecord, RecordStatus)> = Vec::new();
    let mut malformed_lines = 0usize;
    for line in BufReader::new(file).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<LedgerRecord>(&line) {
            Ok(record) => {
                let status = classify(project_path, &record);
                parsed.push((record, status));
            }
            Err(_) => malformed_lines += 1,
        }
    }

    // A later verdict on the same subject supersedes earlier ones (mirrors
    // `decision_ledger::replay`).
    let mut last_by_subject: BTreeMap<String, usize> = BTreeMap::new();
    for (index, (record, _)) in parsed.iter().enumerate() {
        if matches!(record.kind.as_str(), "variant_verdict" | "final_verdict") {
            last_by_subject.insert(record.subject.clone(), index);
        }
    }
    for (index, (record, status)) in parsed.iter_mut().enumerate() {
        if matches!(record.kind.as_str(), "variant_verdict" | "final_verdict")
            && last_by_subject.get(&record.subject).copied() != Some(index)
            && *status == RecordStatus::Current
        {
            *status = RecordStatus::Superseded;
        }
    }

    let mut current = Vec::new();
    let mut excluded_stale = 0usize;
    let mut excluded_missing = 0usize;
    let mut excluded_superseded = 0usize;
    for (record, status) in parsed {
        match status {
            RecordStatus::Current => current.push(record),
            RecordStatus::StaleArtifact => excluded_stale += 1,
            RecordStatus::MissingArtifact => excluded_missing += 1,
            RecordStatus::Superseded => excluded_superseded += 1,
        }
    }

    Ok(ProjectLedger {
        decisions_path: Some(path),
        ledger_blake3: Some(ledger_blake3),
        current,
        excluded_stale,
        excluded_missing,
        excluded_superseded,
        malformed_lines,
    })
}

// ---------------------------------------------------------------------------
// Optional retention input (designed, not fabricated: see module docs and
// `schemas/retention-sample.schema.json`).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct RetentionSample {
    #[serde(default)]
    output_blake3: String,
    #[serde(default)]
    metric: String,
    #[serde(default)]
    value: f64,
}

fn load_retention_samples(project_path: &Path) -> Result<Vec<RetentionSample>, ProjectError> {
    let path = project_path.join("feedback/retention.jsonl");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let file = File::open(&path)?;
    let mut samples = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(sample) = serde_json::from_str::<RetentionSample>(&line) {
            if !sample.output_blake3.is_empty() && !sample.metric.is_empty() {
                samples.push(sample);
            }
        }
    }
    Ok(samples)
}

// ---------------------------------------------------------------------------
// Axis tally and decision.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct DistributionEntry {
    pub option: String,
    pub count: usize,
    pub ratio: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AxisResult {
    pub axis: String,
    pub status: &'static str,
    pub sample_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommendation: Option<String>,
    pub distribution: Vec<DistributionEntry>,
    pub cited_decisions: Vec<String>,
}

/// One citation: the option this decision voted for, plus enough to sort
/// citations deterministically and chronologically.
struct Citation<'a> {
    option: String,
    decision_id: &'a str,
    ts: &'a str,
}

/// Tally citations into a distribution and decide the axis's status per
/// policy (binding rules 4 and 5): below the sample floor is
/// `insufficient_data`; at/above floor with a clear leader is
/// `recommendation`; at/above floor with no leader is `conflict` — never
/// silently averaged.
fn decide_axis(
    axis: &'static str,
    citations: Vec<Citation<'_>>,
    policy: &PreferenceAutonomyPolicy,
) -> AxisResult {
    let sample_count = citations.len();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for citation in &citations {
        *counts.entry(citation.option.clone()).or_insert(0) += 1;
    }
    let mut distribution: Vec<DistributionEntry> = counts
        .into_iter()
        .map(|(option, count)| {
            let ratio = count as f64 / sample_count.max(1) as f64;
            DistributionEntry {
                option,
                count,
                ratio,
            }
        })
        .collect();
    // Deterministic order: highest count first, ties broken alphabetically
    // by option name.
    distribution.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.option.cmp(&b.option)));

    let mut cited_decisions: Vec<&str> = citations.iter().map(|c| c.decision_id).collect();
    let ts_by_id: BTreeMap<&str, &str> = citations
        .iter()
        .map(|citation| (citation.decision_id, citation.ts))
        .collect();
    cited_decisions.sort_by(|a, b| ts_by_id.get(a).cmp(&ts_by_id.get(b)).then_with(|| a.cmp(b)));
    cited_decisions.dedup();
    let cited_decisions: Vec<String> = cited_decisions.into_iter().map(str::to_owned).collect();

    if sample_count < policy.min_decisions_per_axis as usize {
        return AxisResult {
            axis: axis.to_string(),
            status: "insufficient_data",
            sample_count,
            confidence: None,
            recommendation: None,
            distribution,
            cited_decisions,
        };
    }

    let leader = distribution.first();
    let leader_ratio = leader.map(|entry| entry.ratio).unwrap_or(0.0);
    if leader_ratio >= policy.min_agreement_ratio {
        let recommendation = leader.map(|entry| entry.option.clone());
        AxisResult {
            axis: axis.to_string(),
            status: "recommendation",
            sample_count,
            confidence: Some(leader_ratio),
            recommendation,
            distribution,
            cited_decisions,
        }
    } else {
        // Binding rule 5: a real disagreement is surfaced, not averaged away.
        AxisResult {
            axis: axis.to_string(),
            status: "conflict",
            sample_count,
            confidence: Some(leader_ratio),
            recommendation: None,
            distribution,
            cited_decisions,
        }
    }
}

fn unsupported_axis(axis: &'static str) -> AxisResult {
    AxisResult {
        axis: axis.to_string(),
        status: "unsupported_by_ledger_schema",
        sample_count: 0,
        confidence: None,
        recommendation: None,
        distribution: Vec::new(),
        cited_decisions: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Output shape.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct SourceSummary {
    pub project: String,
    pub decisions_path: Option<String>,
    pub ledger_blake3: Option<String>,
    pub current_records: usize,
    pub excluded_stale: usize,
    pub excluded_missing: usize,
    pub excluded_superseded: usize,
    pub malformed_lines: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Readiness {
    pub level: String,
    pub reviewed_projects: usize,
    /// Always `false`: this stage proposes recommendations for a human; it
    /// never gates or performs an automatic render decision (binding rule 3).
    pub auto_apply_allowed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FormatRecommendations {
    pub format: String,
    pub reviewed_projects: usize,
    pub total_current_decisions: usize,
    pub readiness: Readiness,
    pub axes: Vec<AxisResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreferenceRecommendations {
    pub schema_version: u32,
    pub policy_version: u32,
    pub generated_at: chrono::DateTime<Utc>,
    pub sources: Vec<SourceSummary>,
    pub formats: Vec<FormatRecommendations>,
}

#[derive(Debug, Serialize)]
pub struct PreferenceRecommendationResult {
    pub status: &'static str,
    pub output_path: PathBuf,
    pub format_count: usize,
    pub recommendation: PreferenceRecommendations,
}

const BOUNDARY_REASONS: &[&str] = &[
    "too_tight",
    "too_loose",
    "bad_boundary",
    "clipped_word",
    "wrong_take",
];

/// `format` here is the deliverable a decision's evidence applies to: the
/// exact `preset` for decisions that carry one (`final_verdict`, whose
/// `subject` is a rendered `render/finals/<preset>.mp4`), and every output
/// preset the *project* declares for decisions that only carry a `variant`
/// (`variant_verdict`, `segment_flag`) — because a variant choice made in
/// review is genuinely evidence for every format later rendered from that
/// variant in this project, not an invented cross-reference. A project's own
/// `project.json` `outputs` is the source for that join, never a guess.
fn record_formats(record: &LedgerRecord, project_output_ids: &[String]) -> Vec<String> {
    if let Some(preset) = &record.preset {
        if !preset.is_empty() {
            return vec![preset.clone()];
        }
    }
    if record.variant.is_some() {
        if !project_output_ids.is_empty() {
            return project_output_ids.to_vec();
        }
        return vec!["unscoped".to_string()];
    }
    vec!["unscoped".to_string()]
}

/// Build recommendations across one or more projects. Read-only: opens each
/// project's ledger and (optionally) retention samples, but writes nothing —
/// callers decide whether/where to persist the result via
/// [`write_recommendations`].
pub fn compute_preference_recommendations(
    project_paths: &[PathBuf],
) -> Result<PreferenceRecommendations, ProjectError> {
    let policy = PreferenceAutonomyPolicy::v1();

    let mut sources = Vec::with_capacity(project_paths.len());
    // (format -> records-with-project-identity) plus a per-format set of
    // retention samples joined in below.
    let mut by_format: BTreeMap<String, Vec<(String, LedgerRecord)>> = BTreeMap::new();
    let mut retention_by_hash: BTreeMap<String, Vec<RetentionSample>> = BTreeMap::new();
    let mut any_retention_file = false;

    for project_path in project_paths {
        let canonical = project_path
            .canonicalize()
            .unwrap_or_else(|_| project_path.clone());
        let ledger = load_project_ledger(&canonical)?;
        let project_output_ids: Vec<String> =
            read_project_manifest(&canonical.join("project.json"))
                .map(|manifest| {
                    manifest
                        .outputs
                        .into_iter()
                        .map(|output| output.id)
                        .collect()
                })
                .unwrap_or_default();

        sources.push(SourceSummary {
            project: canonical.display().to_string(),
            decisions_path: ledger
                .decisions_path
                .as_ref()
                .map(|path| relative_artifact_path(&canonical, path)),
            ledger_blake3: ledger.ledger_blake3.clone(),
            current_records: ledger.current.len(),
            excluded_stale: ledger.excluded_stale,
            excluded_missing: ledger.excluded_missing,
            excluded_superseded: ledger.excluded_superseded,
            malformed_lines: ledger.malformed_lines,
        });

        for record in ledger.current {
            for format in record_formats(&record, &project_output_ids) {
                by_format
                    .entry(format)
                    .or_default()
                    .push((canonical.display().to_string(), record.clone()));
            }
        }

        let retention = load_retention_samples(&canonical)?;
        if !retention.is_empty() {
            any_retention_file = true;
        }
        for sample in retention {
            retention_by_hash
                .entry(sample.output_blake3.clone())
                .or_default()
                .push(sample);
        }
    }

    let mut formats = Vec::with_capacity(by_format.len());
    for (format, records) in by_format {
        let reviewed_projects: std::collections::BTreeSet<&str> = records
            .iter()
            .map(|(_, record)| record.project_identity())
            .collect();
        let reviewed_project_count = reviewed_projects.len();

        let variant_citations: Vec<Citation> = records
            .iter()
            .filter(|(_, record)| record.kind == "variant_verdict" && record.verdict == "approved")
            .filter_map(|(_, record)| {
                record.variant.as_ref().map(|variant| Citation {
                    option: variant.clone(),
                    decision_id: &record.decision_id,
                    ts: &record.ts,
                })
            })
            .collect();

        let boundary_citations: Vec<Citation> = records
            .iter()
            .filter(|(_, record)| {
                record.kind == "segment_flag" && BOUNDARY_REASONS.contains(&record.reason.as_str())
            })
            .map(|(_, record)| Citation {
                option: record.reason.clone(),
                decision_id: &record.decision_id,
                ts: &record.ts,
            })
            .collect();

        let rejection_citations: Vec<Citation> = records
            .iter()
            .filter(|(_, record)| record.verdict == "rejected")
            .map(|(_, record)| Citation {
                option: record.reason.clone(),
                decision_id: &record.decision_id,
                ts: &record.ts,
            })
            .collect();

        let caption_citations: Vec<Citation> = records
            .iter()
            .filter(|(_, record)| record.kind == "final_verdict" && record.reason == "captions")
            .map(|(_, record)| Citation {
                option: record.verdict.clone(),
                decision_id: &record.decision_id,
                ts: &record.ts,
            })
            .collect();

        let framing_citations: Vec<Citation> = records
            .iter()
            .filter(|(_, record)| {
                record.kind == "final_verdict"
                    && matches!(record.reason.as_str(), "framing" | "color")
            })
            .map(|(_, record)| Citation {
                option: record.verdict.clone(),
                decision_id: &record.decision_id,
                ts: &record.ts,
            })
            .collect();

        // Retention: join by exact subject_blake3 against every current
        // final_verdict decision cited under this format. No retention data
        // anywhere in the given projects means an honest `no_data`, not zero
        // averaged into a number.
        let mut retention_matches: Vec<(String, f64)> = Vec::new();
        let mut retention_cited: Vec<&str> = Vec::new();
        for (_, record) in &records {
            if record.kind != "final_verdict" {
                continue;
            }
            let Some(hash) = &record.subject_blake3 else {
                continue;
            };
            if let Some(samples) = retention_by_hash.get(hash) {
                for sample in samples {
                    retention_matches.push((sample.metric.clone(), sample.value));
                    retention_cited.push(&record.decision_id);
                }
            }
        }
        let retention_axis = if retention_matches.is_empty() {
            if any_retention_file {
                AxisResult {
                    axis: "retention".to_string(),
                    status: "insufficient_data",
                    sample_count: 0,
                    confidence: None,
                    recommendation: None,
                    distribution: Vec::new(),
                    cited_decisions: Vec::new(),
                }
            } else {
                AxisResult {
                    axis: "retention".to_string(),
                    status: "no_data",
                    sample_count: 0,
                    confidence: None,
                    recommendation: None,
                    distribution: Vec::new(),
                    cited_decisions: Vec::new(),
                }
            }
        } else if retention_matches.len() < policy.min_decisions_per_axis as usize {
            AxisResult {
                axis: "retention".to_string(),
                status: "insufficient_data",
                sample_count: retention_matches.len(),
                confidence: None,
                recommendation: None,
                distribution: Vec::new(),
                cited_decisions: {
                    let mut ids: Vec<String> =
                        retention_cited.into_iter().map(str::to_owned).collect();
                    ids.sort();
                    ids.dedup();
                    ids
                },
            }
        } else {
            let mut per_metric: BTreeMap<String, (f64, usize)> = BTreeMap::new();
            for (metric, value) in &retention_matches {
                let entry = per_metric.entry(metric.clone()).or_insert((0.0, 0));
                entry.0 += value;
                entry.1 += 1;
            }
            let mut distribution: Vec<DistributionEntry> = per_metric
                .into_iter()
                .map(|(metric, (sum, count))| {
                    let ratio = count as f64 / retention_matches.len() as f64;
                    DistributionEntry {
                        option: format!("{metric}_avg={:.3}", sum / count as f64),
                        count,
                        ratio,
                    }
                })
                .collect();
            distribution.sort_by(|a, b| a.option.cmp(&b.option));
            let mut ids: Vec<String> = retention_cited.into_iter().map(str::to_owned).collect();
            ids.sort();
            ids.dedup();
            AxisResult {
                axis: "retention".to_string(),
                status: "recommendation",
                sample_count: retention_matches.len(),
                confidence: None,
                recommendation: None,
                distribution,
                cited_decisions: ids,
            }
        };

        let axes = vec![
            decide_axis("variant_selection", variant_citations, &policy),
            decide_axis("boundary_correction", boundary_citations, &policy),
            decide_axis("rejection_reasons", rejection_citations, &policy),
            decide_axis("caption_feedback", caption_citations, &policy),
            decide_axis("framing_crop_feedback", framing_citations, &policy),
            retention_axis,
            unsupported_axis("pause_policy"),
            unsupported_axis("effect_density"),
            unsupported_axis("sfx_choices"),
            unsupported_axis("hook_cta_structure"),
        ];

        #[allow(clippy::cast_possible_truncation)]
        let readiness = Readiness {
            level: policy.readiness_level(reviewed_project_count as u32),
            reviewed_projects: reviewed_project_count,
            auto_apply_allowed: false,
        };

        formats.push(FormatRecommendations {
            format,
            reviewed_projects: reviewed_project_count,
            total_current_decisions: records.len(),
            readiness,
            axes,
        });
    }

    Ok(PreferenceRecommendations {
        schema_version: PREFERENCE_SCHEMA_VERSION,
        policy_version: PreferenceAutonomyPolicy::v1().policy_version,
        generated_at: Utc::now(),
        sources,
        formats,
    })
}

/// Persist a computed recommendation set at `output_path` and write a
/// [`crate::receipts`] stage receipt binding it to the exact policy version
/// and every project ledger that was read (REV2 plan §15.7: "trace a
/// recommendation set to exactly the decisions that produced it").
fn write_recommendations(
    output_path: &Path,
    recommendations: &PreferenceRecommendations,
    project_paths: &[PathBuf],
) -> Result<(), ProjectError> {
    write_json_atomic(output_path, recommendations)?;

    let policy_source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/preference-autonomy-policy.v1.json");
    let mut inputs: Vec<PathBuf> = Vec::new();
    if policy_source.is_file() {
        inputs.push(policy_source);
    }
    for project_path in project_paths {
        let path = project_path.join("feedback/decisions.jsonl");
        if path.is_file() {
            inputs.push(path);
        }
        let retention_path = project_path.join("feedback/retention.jsonl");
        if retention_path.is_file() {
            inputs.push(retention_path);
        }
    }
    let input_refs: Vec<&Path> = inputs.iter().map(PathBuf::as_path).collect();

    write_stage_receipt(
        &receipt_path_for(output_path),
        "preferences.recommend",
        &input_refs,
        &serde_json::json!({
            "policy_version": recommendations.policy_version,
            "format_count": recommendations.formats.len(),
        }),
        BTreeMap::new(),
        &[output_path],
    )?;
    Ok(())
}

/// CLI/library entry point (`videoctl preferences recommend`). Computes
/// recommendations across every given project unconditionally (read-only,
/// cheap), and only writes `output_path` (plus its stage receipt) when
/// `dry_run` is false — matching the rest of the pipeline's dry-run
/// convention (compute for real, skip the write).
pub fn recommend_preferences(
    project_paths: &[PathBuf],
    output_path: &Path,
    dry_run: bool,
) -> Result<PreferenceRecommendationResult, ProjectError> {
    if project_paths.is_empty() {
        return Err(ProjectError::InvalidState(
            "preferences.recommend requires at least one project".into(),
        ));
    }
    let recommendations = compute_preference_recommendations(project_paths)?;
    let format_count = recommendations.formats.len();
    if !dry_run {
        write_recommendations(output_path, &recommendations, project_paths)?;
    }
    Ok(PreferenceRecommendationResult {
        status: if dry_run { "dry-run" } else { "ok" },
        output_path: output_path.to_path_buf(),
        format_count,
        recommendation: recommendations,
    })
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
        let dir = std::env::temp_dir().join(format!("cutright-preferences-test-{label}-{unique}"));
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    fn init_minimal_project(root: &Path, outputs: &[&str]) {
        fs::create_dir_all(root.join("feedback")).unwrap();
        let outputs_json: Vec<serde_json::Value> = outputs
            .iter()
            .map(|id| serde_json::json!({ "id": id, "aspect": "16:9", "width": 1920, "height": 1080 }))
            .collect();
        fs::write(
            root.join("project.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version": 1,
                "project_id": "proj",
                "project_instance_id": format!("pin_{}", root.file_name().unwrap().to_string_lossy()),
                "kind": "long_form",
                "created_at": "2026-01-01T00:00:00Z",
                "review_mode": "reviewed",
                "source_policy": "immutable",
                "outputs": outputs_json,
            }))
            .unwrap(),
        )
        .unwrap();
    }

    /// Writes (or overwrites) the artifact at `subject_rel` with fixed bytes
    /// and returns its `blake3:<hex>` hash, so more than one decision line
    /// can be bound to the exact same artifact bytes (needed to exercise the
    /// same-subject supersede rule, which only applies when two verdicts
    /// genuinely review the same rendered bytes).
    fn write_artifact(root: &Path, subject_rel: &str, content: &[u8]) -> String {
        let subject_path = root.join(subject_rel);
        fs::create_dir_all(subject_path.parent().unwrap()).unwrap();
        fs::write(&subject_path, content).unwrap();
        format!("blake3:{}", hash_file(&subject_path).unwrap())
    }

    /// Appends one decision record line bound to an already-known artifact
    /// hash. Does not touch the artifact on disk.
    #[allow(clippy::too_many_arguments)]
    fn append_decision_line(
        root: &Path,
        decision_id: &str,
        ts: &str,
        kind: &str,
        verdict: &str,
        reason: &str,
        subject_rel: &str,
        subject_hash: &str,
        variant: Option<&str>,
        preset: Option<&str>,
    ) {
        let record = serde_json::json!({
            "decision_id": decision_id,
            "schema_version": 1,
            "client_request_id": decision_id,
            "ts": ts,
            "project_id": "proj",
            "project_instance_id": format!("pin_{}", root.file_name().unwrap().to_string_lossy()),
            "kind": kind,
            "verdict": verdict,
            "reason": reason,
            "subject": subject_rel,
            "subject_blake3": subject_hash,
            "subject_size": decision_id.len(),
            "variant": variant,
            "preset": preset,
            "playhead_ms": 0,
            "bench_resolved": true,
            "app_version": "0.1.0",
        });
        let mut line = serde_json::to_string(&record).unwrap();
        line.push('\n');
        let ledger = root.join("feedback/decisions.jsonl");
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&ledger)
            .unwrap();
        file.write_all(line.as_bytes()).unwrap();
    }

    /// Writes one decision record line and the artifact it's bound to (keyed
    /// uniquely off `decision_id`'s bytes), so the record classifies as
    /// `Current`. Each call binds to its own distinct artifact — use
    /// [`write_artifact`] + [`append_decision_line`] directly when two
    /// decisions must review the exact same bytes (e.g. a supersede test).
    #[allow(clippy::too_many_arguments)]
    fn write_current_decision(
        root: &Path,
        decision_id: &str,
        ts: &str,
        kind: &str,
        verdict: &str,
        reason: &str,
        subject_rel: &str,
        variant: Option<&str>,
        preset: Option<&str>,
    ) {
        let hash = write_artifact(root, subject_rel, decision_id.as_bytes());
        append_decision_line(
            root,
            decision_id,
            ts,
            kind,
            verdict,
            reason,
            subject_rel,
            &hash,
            variant,
            preset,
        );
    }

    #[test]
    fn stale_and_superseded_records_are_excluded_from_training() {
        let dir = unique_dir("stale");
        init_minimal_project(&dir, &["youtube"]);

        // Current: artifact bytes match the recorded hash.
        write_current_decision(
            &dir,
            "d_current",
            "2026-01-01T00:00:00Z",
            "variant_verdict",
            "approved",
            "pacing",
            "render/rough-cuts/tight.mp4",
            Some("tight"),
            None,
        );
        // Tamper with the bound artifact after the fact: now StaleArtifact.
        fs::write(dir.join("render/rough-cuts/tight.mp4"), b"tampered-bytes").unwrap();

        let recs = compute_preference_recommendations(std::slice::from_ref(&dir)).unwrap();
        assert_eq!(recs.sources[0].current_records, 0);
        assert_eq!(recs.sources[0].excluded_stale, 1);

        // No axis should cite the stale decision.
        for format in &recs.formats {
            for axis in &format.axes {
                assert!(!axis.cited_decisions.contains(&"d_current".to_string()));
            }
        }

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn superseded_variant_verdict_is_excluded() {
        let dir = unique_dir("superseded");
        init_minimal_project(&dir, &["youtube"]);

        // Both decisions review the exact same rendered artifact bytes (the
        // real-world case: Adrian reviews the same render twice without a
        // re-render in between) — so both hash-match, and only the later
        // verdict on that subject should win.
        let hash = write_artifact(
            &dir,
            "render/rough-cuts/tight.mp4",
            b"identical-render-bytes",
        );
        append_decision_line(
            &dir,
            "d_first",
            "2026-01-01T00:00:00Z",
            "variant_verdict",
            "approved",
            "pacing",
            "render/rough-cuts/tight.mp4",
            &hash,
            Some("tight"),
            None,
        );
        append_decision_line(
            &dir,
            "d_second",
            "2026-01-02T00:00:00Z",
            "variant_verdict",
            "rejected",
            "pacing",
            "render/rough-cuts/tight.mp4",
            &hash,
            Some("tight"),
            None,
        );

        let recs = compute_preference_recommendations(std::slice::from_ref(&dir)).unwrap();
        // Both records hash-match the live artifact, but the earlier verdict
        // on the same subject is superseded by the later one.
        assert_eq!(recs.sources[0].current_records, 1);
        assert_eq!(recs.sources[0].excluded_superseded, 1);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn recommendation_cites_the_decisions_that_caused_it() {
        let dir = unique_dir("citation");
        init_minimal_project(&dir, &["youtube"]);
        for (index, decision_id) in ["d_a", "d_b", "d_c", "d_d"].iter().enumerate() {
            write_current_decision(
                &dir,
                decision_id,
                &format!("2026-01-0{}T00:00:00Z", index + 1),
                "variant_verdict",
                "approved",
                "pacing",
                &format!("render/rough-cuts/tight-{index}.mp4"),
                Some("tight"),
                None,
            );
        }

        let recs = compute_preference_recommendations(std::slice::from_ref(&dir)).unwrap();
        let format = recs.formats.iter().find(|f| f.format == "youtube").unwrap();
        let axis = format
            .axes
            .iter()
            .find(|a| a.axis == "variant_selection")
            .unwrap();
        assert_eq!(axis.status, "recommendation");
        assert_eq!(axis.recommendation.as_deref(), Some("tight"));
        assert_eq!(axis.sample_count, 4);
        let mut cited = axis.cited_decisions.clone();
        cited.sort();
        assert_eq!(cited, vec!["d_a", "d_b", "d_c", "d_d"]);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn below_floor_sample_refuses_to_recommend() {
        let dir = unique_dir("floor");
        init_minimal_project(&dir, &["youtube"]);
        // Policy floor is 4; write only 2.
        write_current_decision(
            &dir,
            "d_a",
            "2026-01-01T00:00:00Z",
            "variant_verdict",
            "approved",
            "pacing",
            "render/rough-cuts/tight-0.mp4",
            Some("tight"),
            None,
        );
        write_current_decision(
            &dir,
            "d_b",
            "2026-01-02T00:00:00Z",
            "variant_verdict",
            "approved",
            "pacing",
            "render/rough-cuts/tight-1.mp4",
            Some("tight"),
            None,
        );

        let recs = compute_preference_recommendations(std::slice::from_ref(&dir)).unwrap();
        let format = recs.formats.iter().find(|f| f.format == "youtube").unwrap();
        let axis = format
            .axes
            .iter()
            .find(|a| a.axis == "variant_selection")
            .unwrap();
        assert_eq!(axis.status, "insufficient_data");
        assert!(axis.recommendation.is_none());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn contradicting_decisions_surface_as_conflict_not_an_average() {
        let dir = unique_dir("conflict");
        init_minimal_project(&dir, &["youtube"]);
        // Two approvals of "tight", two of "natural" — a genuine 50/50 split.
        for (index, variant) in ["tight", "tight", "natural", "natural"].iter().enumerate() {
            write_current_decision(
                &dir,
                &format!("d_{index}"),
                &format!("2026-01-0{}T00:00:00Z", index + 1),
                "variant_verdict",
                "approved",
                "pacing",
                &format!("render/rough-cuts/{variant}-{index}.mp4"),
                Some(variant),
                None,
            );
        }

        let recs = compute_preference_recommendations(std::slice::from_ref(&dir)).unwrap();
        let format = recs.formats.iter().find(|f| f.format == "youtube").unwrap();
        let axis = format
            .axes
            .iter()
            .find(|a| a.axis == "variant_selection")
            .unwrap();
        assert_eq!(axis.status, "conflict");
        assert!(axis.recommendation.is_none());
        assert_eq!(axis.distribution.len(), 2);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn readiness_is_per_format_never_a_global_default() {
        let heavily_reviewed = unique_dir("readiness-a");
        init_minimal_project(&heavily_reviewed, &["youtube"]);
        for index in 0..4 {
            write_current_decision(
                &heavily_reviewed,
                &format!("d_yt_{index}"),
                &format!("2026-01-0{}T00:00:00Z", index + 1),
                "variant_verdict",
                "approved",
                "pacing",
                &format!("render/rough-cuts/tight-{index}.mp4"),
                Some("tight"),
                None,
            );
        }
        let lightly_reviewed = unique_dir("readiness-b");
        init_minimal_project(&lightly_reviewed, &["reels"]);
        write_current_decision(
            &lightly_reviewed,
            "d_reels_0",
            "2026-01-01T00:00:00Z",
            "variant_verdict",
            "approved",
            "pacing",
            "render/rough-cuts/tight-0.mp4",
            Some("tight"),
            None,
        );

        let recs = compute_preference_recommendations(&[
            heavily_reviewed.clone(),
            lightly_reviewed.clone(),
        ])
        .unwrap();
        let youtube = recs.formats.iter().find(|f| f.format == "youtube").unwrap();
        let reels = recs.formats.iter().find(|f| f.format == "reels").unwrap();
        // Both formats have exactly 1 reviewed project each in this fixture
        // (each project declares only its own format), so readiness is
        // identical and low here — the point of this test is that the two
        // formats are computed independently, not derived from a shared
        // global counter.
        assert_eq!(youtube.reviewed_projects, 1);
        assert_eq!(reels.reviewed_projects, 1);
        assert!(!youtube.readiness.auto_apply_allowed);
        assert!(!reels.readiness.auto_apply_allowed);
        assert_eq!(youtube.readiness.level, "review_required");

        fs::remove_dir_all(&heavily_reviewed).ok();
        fs::remove_dir_all(&lightly_reviewed).ok();
    }

    #[test]
    fn deterministic_same_ledger_same_recommendations() {
        let dir = unique_dir("determinism");
        init_minimal_project(&dir, &["youtube"]);
        for (index, (verdict, reason)) in [
            ("rejected", "too_tight"),
            ("rejected", "too_loose"),
            ("rejected", "too_tight"),
            ("rejected", "bad_boundary"),
        ]
        .iter()
        .enumerate()
        {
            write_current_decision(
                &dir,
                &format!("d_{index}"),
                &format!("2026-01-0{}T00:00:00Z", index + 1),
                "segment_flag",
                verdict,
                reason,
                &format!("render/rough-cuts/tight-{index}.mp4"),
                Some("tight"),
                None,
            );
        }

        let first = compute_preference_recommendations(std::slice::from_ref(&dir)).unwrap();
        let second = compute_preference_recommendations(std::slice::from_ref(&dir)).unwrap();
        let first_json = serde_json::to_value(&first.formats).unwrap();
        let second_json = serde_json::to_value(&second.formats).unwrap();
        assert_eq!(first_json, second_json);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unsupported_axes_are_reported_not_fabricated() {
        let dir = unique_dir("unsupported");
        init_minimal_project(&dir, &["youtube"]);
        write_current_decision(
            &dir,
            "d_a",
            "2026-01-01T00:00:00Z",
            "variant_verdict",
            "approved",
            "pacing",
            "render/rough-cuts/tight-0.mp4",
            Some("tight"),
            None,
        );

        let recs = compute_preference_recommendations(std::slice::from_ref(&dir)).unwrap();
        let format = recs.formats.iter().find(|f| f.format == "youtube").unwrap();
        for axis_name in [
            "pause_policy",
            "effect_density",
            "sfx_choices",
            "hook_cta_structure",
        ] {
            let axis = format.axes.iter().find(|a| a.axis == axis_name).unwrap();
            assert_eq!(axis.status, "unsupported_by_ledger_schema");
            assert!(axis.cited_decisions.is_empty());
        }
        let retention = format.axes.iter().find(|a| a.axis == "retention").unwrap();
        assert_eq!(retention.status, "no_data");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn recommend_preferences_writes_output_and_receipt_only_when_not_dry_run() {
        let dir = unique_dir("write");
        init_minimal_project(&dir, &["youtube"]);
        for index in 0..4 {
            write_current_decision(
                &dir,
                &format!("d_{index}"),
                &format!("2026-01-0{}T00:00:00Z", index + 1),
                "variant_verdict",
                "approved",
                "pacing",
                &format!("render/rough-cuts/tight-{index}.mp4"),
                Some("tight"),
                None,
            );
        }
        let output_path = dir.join("feedback/preferences/recommendations.json");

        let dry = recommend_preferences(std::slice::from_ref(&dir), &output_path, true).unwrap();
        assert_eq!(dry.status, "dry-run");
        assert_eq!(dry.format_count, 1);
        assert!(!output_path.is_file());

        let real = recommend_preferences(std::slice::from_ref(&dir), &output_path, false).unwrap();
        assert_eq!(real.status, "ok");
        assert!(output_path.is_file());
        assert!(receipt_path_for(&output_path).is_file());

        fs::remove_dir_all(&dir).ok();
    }
}
