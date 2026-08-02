//! Per-project cloud-analysis consent, budget, and upload-policy envelope
//! (REV2 plan §15.6 Phase 8).
//!
//! Cloud analysis is off by default everywhere. Nothing in this module ever
//! enables it: [`ProjectCloudConfig::default`] is `consent: false`,
//! `budget_usd_limit: 0.0`, `upload_policy: Proxy` — the same defaults a
//! project with no `cloud-config.json` on disk gets from [`load_cloud_config`].
//! A global default can never turn cloud on; only an explicit, per-project
//! `cloud-config.json` with `consent: true` and a positive budget can, and
//! even then [`CloudEnvelopePolicy`] caps the budget at a hard ceiling no
//! project config can raise past.
//!
//! This module owns policy *decisions* (pure functions, no I/O beyond
//! loading/saving the small JSON documents that carry state between calls);
//! `cloud.rs` owns the provider boundary and orchestrates a request against
//! the decisions made here.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::io::{read_json_if_file, write_json_atomic};
use crate::ProjectError;

/// Schema version shared by the three small on-disk documents this module
/// owns (`cloud-config.json`, `spend-ledger.json`, `retention.json`) — they
/// always version together since they are read and written as one unit by
/// `cloud.rs`.
pub const CLOUD_SCHEMA_VERSION: u32 = 1;

const CLOUD_ENVELOPE_POLICY_V1_JSON: &str = include_str!("../../../schemas/cloud-policy.v1.json");

/// Which bytes a cloud request is allowed to carry off the machine. `Proxy`
/// is the safe default everywhere in this module; `Source` requires an
/// explicit, separate per-project opt-in (`ProjectCloudConfig::upload_policy
/// == Source`) *and* an explicit per-request choice — a project defaulted to
/// `Proxy` never uploads a registered source merely because a proxy happens
/// to be unavailable; the request is refused instead (see
/// [`authorize_spend`]).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum UploadPolicy {
    #[default]
    Proxy,
    Source,
}

/// The embedded, versioned safety ceiling (mirrors the `PreferenceAutonomyPolicy`
/// / `BenchmarkPolicy` embedded-JSON idiom: bumping it means adding
/// `cloud-policy.v2.json` and a new loader, never silently editing the
/// numbers behind an existing `policy_version`). This is a hard ceiling, not
/// a switch — it only ever narrows what a per-project [`ProjectCloudConfig`]
/// can authorize, never widens it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloudEnvelopePolicy {
    pub policy_version: u32,
    #[serde(default)]
    pub description: String,
    pub default_consent: bool,
    pub default_budget_usd_limit: f64,
    pub default_upload_policy: UploadPolicy,
    /// No project's `budget_usd_limit`, however it was set, is ever
    /// honored past this ceiling.
    pub max_budget_usd_limit_ceiling: f64,
    pub allowed_upload_policies: Vec<UploadPolicy>,
}

impl CloudEnvelopePolicy {
    /// Load the embedded policy version 1. This is the only supported
    /// policy version today.
    pub fn v1() -> Self {
        serde_json::from_str(CLOUD_ENVELOPE_POLICY_V1_JSON)
            .expect("embedded cloud-policy.v1.json must be valid")
    }
}

/// Per-project consent, budget, and upload-policy state. Persisted at
/// `<project>/analysis/cloud-analysis/cloud-config.json`. A project with no
/// such file gets [`ProjectCloudConfig::default`] — cloud stays off.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectCloudConfig {
    pub schema_version: u32,
    /// Explicit per-project consent. `false` unless a human set it — never
    /// implied by any other setting (a nonzero budget with `consent: false`
    /// still authorizes nothing).
    #[serde(default)]
    pub consent: bool,
    /// Hard spend ceiling in USD. `0.0` (the default) means no request can
    /// ever be authorized, regardless of its estimated cost.
    #[serde(default)]
    pub budget_usd_limit: f64,
    #[serde(default)]
    pub upload_policy: UploadPolicy,
    /// Provider name this project intends to use once one is configured.
    /// Purely informational here — `cloud.rs` resolves the actual provider
    /// instance from the CLI/caller-supplied name, not from this field.
    #[serde(default)]
    pub provider: Option<String>,
    /// The name of an environment variable an eventual real provider adapter
    /// would read its credential from at call time. Never a credential
    /// value itself — this module never reads, stores, or logs one.
    #[serde(default)]
    pub credential_env_var: Option<String>,
}

impl Default for ProjectCloudConfig {
    fn default() -> Self {
        Self {
            schema_version: CLOUD_SCHEMA_VERSION,
            consent: false,
            budget_usd_limit: 0.0,
            upload_policy: UploadPolicy::Proxy,
            provider: None,
            credential_env_var: None,
        }
    }
}

pub(crate) fn cloud_config_path(project_path: &Path) -> PathBuf {
    project_path.join("analysis/cloud-analysis/cloud-config.json")
}

/// Load a project's cloud config, or the disabled-by-default value if none
/// has ever been written.
pub fn load_cloud_config(project_path: &Path) -> ProjectCloudConfig {
    read_json_if_file(&cloud_config_path(project_path)).unwrap_or_default()
}

pub(crate) fn save_cloud_config(
    project_path: &Path,
    config: &ProjectCloudConfig,
) -> Result<(), ProjectError> {
    write_json_atomic(&cloud_config_path(project_path), config)
}

/// Set explicit per-project consent. This is the only way `consent` ever
/// becomes `true` — there is no global switch and no implicit inheritance
/// from another project.
pub fn set_cloud_consent(
    project_path: &Path,
    enable: bool,
    dry_run: bool,
) -> Result<ProjectCloudConfig, ProjectError> {
    let mut config = load_cloud_config(project_path);
    config.consent = enable;
    if !dry_run {
        save_cloud_config(project_path, &config)?;
    }
    Ok(config)
}

/// Set a project's hard budget ceiling. Rejects negative or non-finite
/// values outright — a malformed budget must never be silently coerced to
/// "unlimited" or "zero".
pub fn set_cloud_budget(
    project_path: &Path,
    usd_limit: f64,
    dry_run: bool,
) -> Result<ProjectCloudConfig, ProjectError> {
    if !usd_limit.is_finite() || usd_limit < 0.0 {
        return Err(ProjectError::InvalidState(format!(
            "cloud budget must be a finite, non-negative USD amount, got {usd_limit}"
        )));
    }
    let mut config = load_cloud_config(project_path);
    config.budget_usd_limit = usd_limit;
    if !dry_run {
        save_cloud_config(project_path, &config)?;
    }
    Ok(config)
}

/// Set which provider a project intends to use and, optionally, which
/// environment variable an eventual real adapter would read a credential
/// name from. Never accepts or stores a credential value.
pub fn set_cloud_provider(
    project_path: &Path,
    provider: Option<String>,
    credential_env_var: Option<String>,
    dry_run: bool,
) -> Result<ProjectCloudConfig, ProjectError> {
    let mut config = load_cloud_config(project_path);
    config.provider = provider;
    config.credential_env_var = credential_env_var;
    if !dry_run {
        save_cloud_config(project_path, &config)?;
    }
    Ok(config)
}

// ---------------------------------------------------------------------------
// Spend ledger — append-only record of every billable cloud request. Never
// pruned by `delete_cloud_retention` (deleting a project's retained content
// must never erase the fact that money was already spent, or budget
// enforcement could be bypassed by upload/delete/re-upload cycling).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloudSpendRecord {
    pub content_hash: String,
    pub capability: String,
    pub provider: String,
    pub model: String,
    pub cost_usd: f64,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CloudSpendLedger {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub records: Vec<CloudSpendRecord>,
}

impl CloudSpendLedger {
    pub fn total_spent(&self) -> f64 {
        self.records.iter().map(|record| record.cost_usd).sum()
    }
}

pub(crate) fn cloud_ledger_path(project_path: &Path) -> PathBuf {
    project_path.join("analysis/cloud-analysis/spend-ledger.json")
}

pub fn load_cloud_ledger(project_path: &Path) -> CloudSpendLedger {
    read_json_if_file(&cloud_ledger_path(project_path)).unwrap_or_default()
}

pub(crate) fn save_cloud_ledger(
    project_path: &Path,
    ledger: &CloudSpendLedger,
) -> Result<(), ProjectError> {
    write_json_atomic(&cloud_ledger_path(project_path), ledger)
}

// ---------------------------------------------------------------------------
// Retention log — what was uploaded, when, and (once deleted) when it
// stopped being retained. This is what `delete_cloud_retention` in `cloud.rs`
// acts on; the spend ledger above is deliberately untouched by deletion.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloudRetentionRecord {
    pub content_hash: String,
    pub bytes_kind: UploadPolicy,
    pub capability: String,
    pub provider: String,
    pub uploaded_at: DateTime<Utc>,
    pub raw_response_path: String,
    pub normalised_output_path: String,
    #[serde(default)]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CloudRetentionLog {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub records: Vec<CloudRetentionRecord>,
}

pub(crate) fn cloud_retention_path(project_path: &Path) -> PathBuf {
    project_path.join("analysis/cloud-analysis/retention.json")
}

pub fn load_cloud_retention(project_path: &Path) -> CloudRetentionLog {
    read_json_if_file(&cloud_retention_path(project_path)).unwrap_or_default()
}

pub(crate) fn save_cloud_retention(
    project_path: &Path,
    log: &CloudRetentionLog,
) -> Result<(), ProjectError> {
    write_json_atomic(&cloud_retention_path(project_path), log)
}

// ---------------------------------------------------------------------------
// Authorization — the pure decision at the center of the safety envelope.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CloudPolicyError {
    #[error(
        "cloud analysis requires explicit per-project consent; set it with \
         `videoctl cloud consent <project> --enable`"
    )]
    ConsentRequired,
    #[error(
        "cloud budget is ${limit:.4} (already spent ${spent:.4}); this request is estimated \
         at ${estimated:.4} and would exceed it"
    )]
    BudgetExceeded {
        limit: f64,
        spent: f64,
        estimated: f64,
    },
    #[error(
        "upload policy is {policy:?}; refusing to send {requested:?} bytes off this machine \
         for this request"
    )]
    UploadPolicyRefused {
        policy: UploadPolicy,
        requested: UploadPolicy,
    },
}

/// Decide whether a cloud request may proceed. Pure policy state in, a
/// decision out — no I/O, no network, no provider call, so every gate below
/// is enforced *before* a single byte is ever considered for transmission,
/// not after the fact.
///
/// Gates, in order:
/// 1. **Consent** — `false` by default; nothing is ever sent unless a
///    project's own `cloud-config.json` explicitly says otherwise.
/// 2. **Upload policy** — `Source` bytes are refused unless the project has
///    explicitly opted into `Source`; `Proxy` is always allowed.
/// 3. **Budget** — a budget of `0.0` (the default) refuses *every* request
///    outright, independent of that request's estimated cost. A nonzero
///    budget still refuses any request whose estimated cost would push
///    total spend past the limit, clamped to
///    [`CloudEnvelopePolicy::max_budget_usd_limit_ceiling`] — a ceiling no
///    project config can raise past.
pub fn authorize_spend(
    config: &ProjectCloudConfig,
    policy: &CloudEnvelopePolicy,
    already_spent_usd: f64,
    estimated_cost_usd: f64,
    requested_bytes: UploadPolicy,
) -> Result<(), CloudPolicyError> {
    if !config.consent {
        return Err(CloudPolicyError::ConsentRequired);
    }
    if requested_bytes == UploadPolicy::Source && config.upload_policy != UploadPolicy::Source {
        return Err(CloudPolicyError::UploadPolicyRefused {
            policy: config.upload_policy,
            requested: requested_bytes,
        });
    }
    let effective_limit = config
        .budget_usd_limit
        .min(policy.max_budget_usd_limit_ceiling);
    if effective_limit <= 0.0 || already_spent_usd + estimated_cost_usd > effective_limit {
        return Err(CloudPolicyError::BudgetExceeded {
            limit: effective_limit,
            spent: already_spent_usd,
            estimated: estimated_cost_usd,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cutright-cloud-policy-test-{label}-{unique}"));
        std::fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn default_config_is_disabled_zero_budget_proxy_only() {
        let config = ProjectCloudConfig::default();
        assert!(!config.consent);
        assert_eq!(config.budget_usd_limit, 0.0);
        assert_eq!(config.upload_policy, UploadPolicy::Proxy);
    }

    #[test]
    fn missing_config_file_loads_the_disabled_default() {
        let dir = unique_dir("missing");
        let config = load_cloud_config(&dir);
        assert_eq!(config, ProjectCloudConfig::default());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn consent_absent_blocks_every_request() {
        let policy = CloudEnvelopePolicy::v1();
        // generous budget, still no consent
        let config = ProjectCloudConfig {
            budget_usd_limit: 100.0,
            ..Default::default()
        };
        let error = authorize_spend(&config, &policy, 0.0, 0.01, UploadPolicy::Proxy).unwrap_err();
        assert_eq!(error, CloudPolicyError::ConsentRequired);
    }

    #[test]
    fn zero_budget_blocks_every_call_even_with_consent() {
        let policy = CloudEnvelopePolicy::v1();
        // budget_usd_limit stays at the default 0.0
        let config = ProjectCloudConfig {
            consent: true,
            ..Default::default()
        };
        let error = authorize_spend(&config, &policy, 0.0, 0.0, UploadPolicy::Proxy).unwrap_err();
        assert!(matches!(error, CloudPolicyError::BudgetExceeded { .. }));
    }

    #[test]
    fn budget_exhaustion_refuses_the_next_call() {
        let policy = CloudEnvelopePolicy::v1();
        let config = ProjectCloudConfig {
            consent: true,
            budget_usd_limit: 0.05,
            ..Default::default()
        };
        assert!(authorize_spend(&config, &policy, 0.0, 0.05, UploadPolicy::Proxy).is_ok());
        let error = authorize_spend(&config, &policy, 0.05, 0.01, UploadPolicy::Proxy).unwrap_err();
        assert!(matches!(error, CloudPolicyError::BudgetExceeded { .. }));
    }

    #[test]
    fn source_upload_refused_when_policy_is_proxy() {
        let policy = CloudEnvelopePolicy::v1();
        // upload_policy stays at the default Proxy
        let config = ProjectCloudConfig {
            consent: true,
            budget_usd_limit: 100.0,
            ..Default::default()
        };
        let error = authorize_spend(&config, &policy, 0.0, 0.01, UploadPolicy::Source).unwrap_err();
        assert!(matches!(
            error,
            CloudPolicyError::UploadPolicyRefused {
                policy: UploadPolicy::Proxy,
                requested: UploadPolicy::Source
            }
        ));
    }

    #[test]
    fn source_upload_allowed_once_explicitly_opted_in() {
        let policy = CloudEnvelopePolicy::v1();
        let config = ProjectCloudConfig {
            consent: true,
            budget_usd_limit: 100.0,
            upload_policy: UploadPolicy::Source,
            ..Default::default()
        };
        assert!(authorize_spend(&config, &policy, 0.0, 0.01, UploadPolicy::Source).is_ok());
    }

    #[test]
    fn project_budget_cannot_exceed_the_policy_ceiling() {
        let policy = CloudEnvelopePolicy::v1();
        let config = ProjectCloudConfig {
            consent: true,
            budget_usd_limit: policy.max_budget_usd_limit_ceiling + 1_000.0,
            ..Default::default()
        };
        // A request estimated just over the real ceiling must still be refused,
        // proving the project-set budget was clamped rather than trusted.
        let error = authorize_spend(
            &config,
            &policy,
            0.0,
            policy.max_budget_usd_limit_ceiling + 1.0,
            UploadPolicy::Proxy,
        )
        .unwrap_err();
        assert!(matches!(error, CloudPolicyError::BudgetExceeded { .. }));
    }

    #[test]
    fn consent_and_budget_round_trip_through_disk() {
        let dir = unique_dir("roundtrip");
        set_cloud_consent(&dir, true, false).unwrap();
        set_cloud_budget(&dir, 2.5, false).unwrap();
        let config = load_cloud_config(&dir);
        assert!(config.consent);
        assert_eq!(config.budget_usd_limit, 2.5);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn dry_run_never_persists_consent_or_budget_changes() {
        let dir = unique_dir("dry-run");
        set_cloud_consent(&dir, true, true).unwrap();
        set_cloud_budget(&dir, 9.0, true).unwrap();
        let config = load_cloud_config(&dir);
        assert_eq!(config, ProjectCloudConfig::default());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn negative_or_non_finite_budget_is_rejected() {
        let dir = unique_dir("bad-budget");
        assert!(set_cloud_budget(&dir, -1.0, false).is_err());
        assert!(set_cloud_budget(&dir, f64::NAN, false).is_err());
        assert!(set_cloud_budget(&dir, f64::INFINITY, false).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ledger_totals_sum_every_record() {
        let ledger = CloudSpendLedger {
            schema_version: CLOUD_SCHEMA_VERSION,
            records: vec![
                CloudSpendRecord {
                    content_hash: "a".into(),
                    capability: "frame_semantics".into(),
                    provider: "fake".into(),
                    model: "fake-fixture-v1".into(),
                    cost_usd: 0.01,
                    at: Utc::now(),
                },
                CloudSpendRecord {
                    content_hash: "b".into(),
                    capability: "segment_semantics".into(),
                    provider: "fake".into(),
                    model: "fake-fixture-v1".into(),
                    cost_usd: 0.05,
                    at: Utc::now(),
                },
            ],
        };
        assert!((ledger.total_spent() - 0.06).abs() < 1e-9);
    }
}
