//! Optional cloud analysis (REV2 plan §15.6 Phase 8).
//!
//! Cloud analysis is off by default, stays off unless a project explicitly
//! turns it on (`cloud_policy::ProjectCloudConfig::consent`), and even then
//! is bounded by a hard per-project budget (`cloud_policy::authorize_spend`).
//! This module owns the provider boundary and the orchestration around one
//! request: content hashing, cache/dedupe, authorization, the provider call
//! itself, outage fallback, and the receipt/retention/ledger bookkeeping a
//! successful call leaves behind. `cloud_policy` owns the policy state and
//! the pure authorization decision this module calls before ever touching a
//! provider.
//!
//! **Anticipated adapters — not built here.** `ARCHITECTURE-2026-07-26.md`
//! names the two providers this envelope is being built for: a Gemini-style
//! general multimodal model and Twelve Labs' video-native search/indexing
//! product. They have different call shapes, so [`CloudProvider`] is shaped
//! for both up front (see [`CloudCapability`] and the trait's two lifecycles
//! below) even though only [`DisabledProvider`] and [`FakeProvider`] ship in
//! this pass — no vendor SDK, endpoint, or credential handling exists yet.
//!
//! **Advisory only, never timestamp authority.** [`CloudSemanticEvidence`],
//! the one payload shape every provider call returns, carries no time-
//! position field of any kind — no `start_ms`/`end_ms`, no frame or sample
//! index, no `Word`/`TimelineSegment` id. There is no `From`/`Into`
//! conversion from it to any `video_core` timing type, and none should ever
//! be added: a caller that wants to place cloud evidence on the timeline
//! must derive the actual timing independently from local, hash-bound
//! signals (VAD, transcript words).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use video_core::models::{ProviderCost, ProviderResponseEnvelope};

use crate::cloud_policy::{
    authorize_spend, load_cloud_config, load_cloud_ledger, load_cloud_retention, save_cloud_ledger,
    save_cloud_retention, CloudEnvelopePolicy, CloudPolicyError, CloudRetentionRecord,
    CloudSpendRecord, ProjectCloudConfig, UploadPolicy,
};
use crate::io::{hash_file, relative_artifact_path, write_json_atomic};
use crate::receipts::{receipt_path_for, write_stage_receipt};
use crate::ProjectError;

// ---------------------------------------------------------------------------
// Capability + provider identity
// ---------------------------------------------------------------------------

/// What kind of cloud-side operation a request or response is for. Two call
/// shapes are anticipated:
///
/// - **One-shot** (`FrameSemantics`, `SegmentSemantics`): a Gemini-style
///   general multimodal model is sent one frame or one short proxy segment
///   and returns semantic labels/description in a single request/response —
///   [`CloudProvider::analyze`].
/// - **Two-phase** (`VideoIndex`): a Twelve-Labs-style video-native
///   search/indexing provider first ingests a proxy into a persistent,
///   provider-side index ([`CloudProvider::index`]), then answers
///   natural-language queries against that index later, possibly many
///   times, without re-uploading ([`CloudProvider::query`]).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CloudCapability {
    FrameSemantics,
    SegmentSemantics,
    VideoIndex,
}

impl CloudCapability {
    pub fn as_str(&self) -> &'static str {
        match self {
            CloudCapability::FrameSemantics => "frame_semantics",
            CloudCapability::SegmentSemantics => "segment_semantics",
            CloudCapability::VideoIndex => "video_index",
        }
    }

    /// Parse a CLI/config-supplied capability name. Accepts both
    /// `snake_case` (the JSON wire form) and `kebab-case` (nicer on a
    /// command line).
    pub fn parse(value: &str) -> Result<Self, CloudError> {
        match value {
            "frame_semantics" | "frame-semantics" => Ok(Self::FrameSemantics),
            "segment_semantics" | "segment-semantics" => Ok(Self::SegmentSemantics),
            "video_index" | "video-index" => Ok(Self::VideoIndex),
            other => Err(CloudError::InvalidCapability(other.to_string())),
        }
    }
}

/// A provider's identity, recorded alongside every response for provenance:
/// which provider, which model, and which build/version of it answered.
/// Never carries a credential.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderIdentity {
    pub name: String,
    pub model: String,
    pub version: String,
}

// ---------------------------------------------------------------------------
// Advisory evidence — structurally not timestamp authority
// ---------------------------------------------------------------------------

/// Advisory semantic evidence from a cloud provider. See the module-level
/// doc for why this type can never become timestamp/cut-boundary authority:
/// it simply has no field that could serve as one.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloudSemanticEvidence {
    pub labels: Vec<String>,
    pub summary: String,
    pub confidence: f64,
    #[serde(default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Provider boundary
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CloudError {
    #[error("cloud provider '{provider}' refused capability {capability:?}: {reason}")]
    Disabled {
        provider: String,
        capability: CloudCapability,
        reason: String,
    },
    #[error("provider '{provider}' does not support capability {capability:?}")]
    UnsupportedCapability {
        provider: String,
        capability: CloudCapability,
    },
    #[error("no fixture found for key '{0}'")]
    FixtureMissing(String),
    #[error("cloud transport failed: {0}")]
    Transport(String),
    #[error(
        "unknown cloud capability '{0}'; expected frame-semantics|segment-semantics|video-index"
    )]
    InvalidCapability(String),
}

pub struct CloudAnalysisRequest {
    pub capability: CloudCapability,
    pub content_hash: String,
    pub bytes_path: PathBuf,
    pub bytes_kind: UploadPolicy,
}

pub struct CloudIndexRequest {
    pub content_hash: String,
    pub bytes_path: PathBuf,
    pub bytes_kind: UploadPolicy,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CloudIndexHandle {
    pub index_id: String,
    pub identity: ProviderIdentity,
    pub content_hash: String,
    pub created_at: DateTime<Utc>,
}

pub struct CloudQueryRequest {
    pub index_id: String,
    pub query: String,
}

/// The provider boundary. Exactly two implementations ship in this pass:
/// [`DisabledProvider`] (the default — refuses everything with a clear
/// reason) and [`FakeProvider`] (a fixture-driven test adapter). No real
/// vendor, SDK, or network call exists behind this trait yet.
///
/// `index`/`query` default to [`CloudError::UnsupportedCapability`] so a
/// one-shot-only adapter (the anticipated Gemini shape) only has to
/// implement `analyze`; a two-phase adapter (the anticipated Twelve Labs
/// shape) overrides `index` and `query` as well.
pub trait CloudProvider: Send + Sync {
    fn identity(&self) -> ProviderIdentity;
    fn capabilities(&self) -> &[CloudCapability];

    fn analyze(&self, request: &CloudAnalysisRequest) -> Result<CloudSemanticEvidence, CloudError>;

    fn index(&self, request: &CloudIndexRequest) -> Result<CloudIndexHandle, CloudError> {
        let _ = request;
        Err(CloudError::UnsupportedCapability {
            provider: self.identity().name,
            capability: CloudCapability::VideoIndex,
        })
    }

    fn query(&self, request: &CloudQueryRequest) -> Result<CloudSemanticEvidence, CloudError> {
        let _ = request;
        Err(CloudError::UnsupportedCapability {
            provider: self.identity().name,
            capability: CloudCapability::VideoIndex,
        })
    }
}

/// The default provider: refuses every capability with a clear reason. This
/// is what every project gets until it names a real provider *and* that
/// provider ships (none does yet).
pub struct DisabledProvider {
    requested: String,
}

impl DisabledProvider {
    pub fn new(requested: impl Into<String>) -> Self {
        Self {
            requested: requested.into(),
        }
    }

    fn refusal(&self, capability: CloudCapability) -> CloudError {
        let reason = if self.requested.is_empty() || self.requested == "disabled" {
            "cloud analysis is off by default; no provider is configured for this project"
                .to_string()
        } else {
            format!(
                "provider '{}' is not implemented in this build; only the 'fake' test-fixture \
                 adapter ships alongside the default 'disabled' provider",
                self.requested
            )
        };
        CloudError::Disabled {
            provider: self.requested.clone(),
            capability,
            reason,
        }
    }
}

impl CloudProvider for DisabledProvider {
    fn identity(&self) -> ProviderIdentity {
        ProviderIdentity {
            name: "disabled".to_string(),
            model: "none".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    fn capabilities(&self) -> &[CloudCapability] {
        &[]
    }

    fn analyze(&self, request: &CloudAnalysisRequest) -> Result<CloudSemanticEvidence, CloudError> {
        Err(self.refusal(request.capability))
    }

    fn index(&self, _request: &CloudIndexRequest) -> Result<CloudIndexHandle, CloudError> {
        Err(self.refusal(CloudCapability::VideoIndex))
    }

    fn query(&self, _request: &CloudQueryRequest) -> Result<CloudSemanticEvidence, CloudError> {
        Err(self.refusal(CloudCapability::VideoIndex))
    }
}

/// A fixture-driven test adapter. Exercises both call shapes
/// (`analyze`, and the two-phase `index`/`query`) so the trait's shape is
/// proven end-to-end without any real network dependency. Ships only for
/// local/dev/test use — `resolve_provider` is the only place a caller picks
/// it by name.
#[derive(Debug, Clone, Deserialize, Default)]
struct FakeFixtureSet {
    #[serde(default)]
    analyze: BTreeMap<String, CloudSemanticEvidence>,
    #[serde(default)]
    index: BTreeMap<String, String>,
    #[serde(default)]
    query: BTreeMap<String, CloudSemanticEvidence>,
}

pub struct FakeProvider {
    identity: ProviderIdentity,
    fixtures: FakeFixtureSet,
}

const EMBEDDED_FAKE_FIXTURES_JSON: &str =
    include_str!("../../../fixtures/cloud/analysis-fixtures.json");

impl FakeProvider {
    pub fn from_fixtures_str(json: &str) -> Result<Self, CloudError> {
        let fixtures: FakeFixtureSet = serde_json::from_str(json)
            .map_err(|error| CloudError::Transport(format!("invalid fixture json: {error}")))?;
        Ok(Self {
            identity: ProviderIdentity {
                name: "fake".to_string(),
                model: "fake-fixture-v1".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            fixtures,
        })
    }

    /// The fixture set shipped in `fixtures/cloud/analysis-fixtures.json`.
    /// Its content hashes are placeholders, not real blake3 digests of any
    /// real asset — an `analyze_cloud` call against real project bytes will
    /// almost always miss and correctly fall back (see `analyze_cloud`'s
    /// outage-fallback path), which is a deliberately honest default: this
    /// adapter exists to prove the plumbing, not to answer real requests.
    pub fn embedded_default() -> Self {
        Self::from_fixtures_str(EMBEDDED_FAKE_FIXTURES_JSON)
            .expect("embedded fixtures/cloud/analysis-fixtures.json must be valid")
    }
}

impl CloudProvider for FakeProvider {
    fn identity(&self) -> ProviderIdentity {
        self.identity.clone()
    }

    fn capabilities(&self) -> &[CloudCapability] {
        &[
            CloudCapability::FrameSemantics,
            CloudCapability::SegmentSemantics,
            CloudCapability::VideoIndex,
        ]
    }

    fn analyze(&self, request: &CloudAnalysisRequest) -> Result<CloudSemanticEvidence, CloudError> {
        self.fixtures
            .analyze
            .get(&request.content_hash)
            .cloned()
            .ok_or_else(|| CloudError::FixtureMissing(request.content_hash.clone()))
    }

    fn index(&self, request: &CloudIndexRequest) -> Result<CloudIndexHandle, CloudError> {
        let index_id = self
            .fixtures
            .index
            .get(&request.content_hash)
            .cloned()
            .ok_or_else(|| CloudError::FixtureMissing(request.content_hash.clone()))?;
        Ok(CloudIndexHandle {
            index_id,
            identity: self.identity(),
            content_hash: request.content_hash.clone(),
            created_at: Utc::now(),
        })
    }

    fn query(&self, request: &CloudQueryRequest) -> Result<CloudSemanticEvidence, CloudError> {
        let key = format!("{}::{}", request.index_id, request.query);
        self.fixtures
            .query
            .get(&key)
            .cloned()
            .ok_or(CloudError::FixtureMissing(key))
    }
}

/// Resolve a provider by name. `"fake"` (case-insensitive) is the one test
/// adapter that ships; every other name — including any future real vendor
/// name — resolves to [`DisabledProvider`], which refuses with a reason
/// naming exactly what was requested and why it did not run.
pub fn resolve_provider(name: &str) -> Box<dyn CloudProvider> {
    if name.eq_ignore_ascii_case("fake") {
        Box::new(FakeProvider::embedded_default())
    } else {
        Box::new(DisabledProvider::new(name.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Orchestration
// ---------------------------------------------------------------------------

/// A conservative, capability-keyed pre-call cost estimate used only to gate
/// spend *before* a request is issued. A provider that reports its own
/// actual cost would update the ledger with that number instead; neither
/// shipped adapter does, so the estimate is what gets recorded on success.
fn estimated_cost_usd(capability: CloudCapability) -> f64 {
    match capability {
        CloudCapability::FrameSemantics => 0.01,
        CloudCapability::SegmentSemantics => 0.05,
        CloudCapability::VideoIndex => 0.10,
    }
}

fn cache_key(
    content_hash: &str,
    capability: CloudCapability,
    identity: &ProviderIdentity,
) -> String {
    let seed = format!(
        "{content_hash}|{}|{}|{}|{}",
        capability.as_str(),
        identity.name,
        identity.model,
        identity.version
    );
    blake3::hash(seed.as_bytes()).to_hex().to_string()
}

/// Whether a project's named environment variable for a credential is
/// currently set. Reads the variable fresh at call time and reports only
/// its presence — never its value — so nothing here can ever leak a
/// credential into a receipt, a retention record, an error message, or this
/// function's own return type.
fn credential_present(config: &ProjectCloudConfig) -> bool {
    config
        .credential_env_var
        .as_deref()
        .is_some_and(|name| std::env::var(name).is_ok())
}

/// One request/response record as persisted on disk, combining the
/// provider-envelope provenance shape every other provider boundary in this
/// crate already uses with the capability and the advisory evidence itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CloudAnalysisRecord {
    envelope: ProviderResponseEnvelope,
    capability: CloudCapability,
    evidence: CloudSemanticEvidence,
}

/// Full outcome of one `analyze_cloud` call. Every status is a normal,
/// expected steady state of an optional feature — none of them is the CLI
/// contract's `status: "error"` sentinel, so none of them exits nonzero on
/// its own; only a genuine local I/O failure (e.g. the project directory
/// itself is unreadable) surfaces as `Err(ProjectError)`.
#[derive(Debug, Clone, Serialize)]
pub struct CloudAnalysisOutcome {
    /// `"sent"` | `"cached"` | `"refused"` | `"fallback_local"` | `"dry_run"`.
    pub status: &'static str,
    pub reason: Option<String>,
    pub capability: CloudCapability,
    pub content_hash: String,
    pub bytes_kind: UploadPolicy,
    pub identity: Option<ProviderIdentity>,
    pub cost_usd: f64,
    pub budget_usd_limit: f64,
    pub budget_spent_usd: f64,
    pub credential_present: bool,
    pub evidence: Option<CloudSemanticEvidence>,
}

/// Pick a default analysis target when the caller did not name one: the
/// first file under `cache/proxies/`, sorted, so the choice is deterministic.
/// Proxies are the safe default everywhere in this module — a caller must
/// explicitly ask for source bytes (`use_source: true` in `analyze_cloud`,
/// which additionally requires the project's own `upload_policy` to allow
/// it) to ever have this function's result overridden with a source path.
pub fn resolve_default_target(project_path: &Path) -> Result<PathBuf, ProjectError> {
    let proxies_dir = project_path.join("cache/proxies");
    let mut entries: Vec<PathBuf> = if proxies_dir.is_dir() {
        std::fs::read_dir(&proxies_dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect()
    } else {
        Vec::new()
    };
    entries.sort();
    entries.into_iter().next().ok_or_else(|| {
        ProjectError::InvalidState(
            "no proxy found under cache/proxies; pass an explicit --target or generate a proxy first"
                .to_string(),
        )
    })
}

/// Run one optional cloud-analysis request against `target` (or the default
/// proxy, if `target` is `None`). Enforces the full safety envelope before
/// ever calling `provider`: consent, upload policy, hard budget, and a
/// content-hash-plus-capability-plus-provider-identity cache that avoids
/// both a redundant provider call (a cache hit) and a redundant upload of
/// the same bytes for a different capability (retention dedupe). A provider
/// failure of any kind — refusal, missing fixture, transport error — never
/// propagates as an `Err`; it degrades to `status: "fallback_local"` so the
/// pipeline this is called from is never blocked by an optional feature.
#[allow(clippy::too_many_arguments)]
pub fn analyze_cloud(
    project_path: &Path,
    provider: &dyn CloudProvider,
    capability: CloudCapability,
    target: Option<&Path>,
    use_source: bool,
    dry_run: bool,
) -> Result<CloudAnalysisOutcome, ProjectError> {
    let target = match target {
        Some(path) => project_path.join(path),
        None => resolve_default_target(project_path)?,
    };
    let bytes_kind = if use_source {
        UploadPolicy::Source
    } else {
        UploadPolicy::Proxy
    };
    let content_hash = hash_file(&target)?;
    let config = load_cloud_config(project_path);
    let credential_present = credential_present(&config);

    let identity = provider.identity();
    let key = cache_key(&content_hash, capability, &identity);
    let normalised_path = project_path.join(format!("analysis/cloud-analysis/{key}.json"));

    // Cache check: a hit means neither a new provider call nor any new
    // spend/retention bookkeeping, regardless of dry_run — reading an
    // existing artifact is not a mutation.
    if let Some(record) = crate::io::read_json_if_file::<CloudAnalysisRecord>(&normalised_path) {
        let ledger = load_cloud_ledger(project_path);
        return Ok(CloudAnalysisOutcome {
            status: "cached",
            reason: None,
            capability,
            content_hash,
            bytes_kind,
            identity: Some(identity),
            cost_usd: 0.0,
            budget_usd_limit: config.budget_usd_limit,
            budget_spent_usd: ledger.total_spent(),
            credential_present,
            evidence: Some(record.evidence),
        });
    }

    let ledger = load_cloud_ledger(project_path);
    let spent = ledger.total_spent();

    if dry_run {
        return Ok(CloudAnalysisOutcome {
            status: "dry_run",
            reason: None,
            capability,
            content_hash,
            bytes_kind,
            identity: Some(identity),
            cost_usd: 0.0,
            budget_usd_limit: config.budget_usd_limit,
            budget_spent_usd: spent,
            credential_present,
            evidence: None,
        });
    }

    let policy = CloudEnvelopePolicy::v1();
    let estimated = estimated_cost_usd(capability);
    if let Err(policy_error) = authorize_spend(&config, &policy, spent, estimated, bytes_kind) {
        return Ok(CloudAnalysisOutcome {
            status: "refused",
            reason: Some(describe_policy_refusal(&policy_error)),
            capability,
            content_hash,
            bytes_kind,
            identity: Some(identity),
            cost_usd: 0.0,
            budget_usd_limit: config.budget_usd_limit,
            budget_spent_usd: spent,
            credential_present,
            evidence: None,
        });
    }

    let request = CloudAnalysisRequest {
        capability,
        content_hash: content_hash.clone(),
        bytes_path: target.clone(),
        bytes_kind,
    };
    let evidence = match provider.analyze(&request) {
        Ok(evidence) => evidence,
        Err(cloud_error) => {
            // Outage fallback (requirement 6): any provider failure degrades
            // to the local result rather than blocking the caller. Nothing
            // was sent, so nothing is charged.
            return Ok(CloudAnalysisOutcome {
                status: "fallback_local",
                reason: Some(cloud_error.to_string()),
                capability,
                content_hash,
                bytes_kind,
                identity: Some(identity),
                cost_usd: 0.0,
                budget_usd_limit: config.budget_usd_limit,
                budget_spent_usd: spent,
                credential_present,
                evidence: None,
            });
        }
    };

    // Persist raw + normalised artifacts, mirroring the shared
    // `ProviderResponseEnvelope` provenance shape every other provider
    // boundary in this crate uses.
    let raw_response_path =
        project_path.join(format!("cache/provider-responses/cloud-{key}.raw.json"));
    write_json_atomic(&raw_response_path, &evidence)?;
    let envelope = ProviderResponseEnvelope {
        provider: identity.name.clone(),
        provider_model: identity.model.clone(),
        request_hash: format!(
            "blake3:{}",
            blake3::hash(format!("{content_hash}|{}", capability.as_str()).as_bytes()).to_hex()
        ),
        created_at: Utc::now(),
        cost: ProviderCost {
            currency: "USD".to_string(),
            estimated: Some(estimated),
        },
        raw_response_path: relative_artifact_path(project_path, &raw_response_path),
        normalised_output_path: relative_artifact_path(project_path, &normalised_path),
        warnings: Vec::new(),
    };
    let record = CloudAnalysisRecord {
        envelope: envelope.clone(),
        capability,
        evidence: evidence.clone(),
    };
    write_json_atomic(&normalised_path, &record)?;

    // Retention dedupe (requirement 7): only record a new upload if this
    // exact content hash + bytes kind is not already retained. A second
    // capability against the same bytes is a new billable request, not a
    // new upload.
    let mut retention = load_cloud_retention(project_path);
    let already_uploaded = retention.records.iter().any(|existing| {
        existing.content_hash == content_hash
            && existing.bytes_kind == bytes_kind
            && existing.deleted_at.is_none()
    });
    if !already_uploaded {
        retention.records.push(CloudRetentionRecord {
            content_hash: content_hash.clone(),
            bytes_kind,
            capability: capability.as_str().to_string(),
            provider: identity.name.clone(),
            uploaded_at: Utc::now(),
            raw_response_path: relative_artifact_path(project_path, &raw_response_path),
            normalised_output_path: relative_artifact_path(project_path, &normalised_path),
            deleted_at: None,
        });
        save_cloud_retention(project_path, &retention)?;
    }

    let mut ledger = ledger;
    ledger.records.push(CloudSpendRecord {
        content_hash: content_hash.clone(),
        capability: capability.as_str().to_string(),
        provider: identity.name.clone(),
        model: identity.model.clone(),
        cost_usd: estimated,
        at: Utc::now(),
    });
    save_cloud_ledger(project_path, &ledger)?;
    let new_spent = ledger.total_spent();

    write_stage_receipt(
        &receipt_path_for(&normalised_path),
        "analyze.cloud",
        &[target.as_path()],
        &serde_json::json!({
            "capability": capability,
            "content_hash": content_hash,
            "provider": identity.name,
            "model": identity.model,
            "provider_version": identity.version,
            "bytes_kind": bytes_kind,
            "budget_usd_limit": config.budget_usd_limit,
            "budget_spent_before_usd": spent,
            "budget_spent_after_usd": new_spent,
            "cost_usd": estimated,
        }),
        BTreeMap::new(),
        &[normalised_path.as_path()],
    )?;

    Ok(CloudAnalysisOutcome {
        status: "sent",
        reason: None,
        capability,
        content_hash,
        bytes_kind,
        identity: Some(identity),
        cost_usd: estimated,
        budget_usd_limit: config.budget_usd_limit,
        budget_spent_usd: new_spent,
        credential_present,
        evidence: Some(evidence),
    })
}

fn describe_policy_refusal(error: &CloudPolicyError) -> String {
    error.to_string()
}

// ---------------------------------------------------------------------------
// Retention deletion
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct CloudDeletionOutcome {
    /// `"deleted"` | `"dry_run"` | `"nothing_to_delete"`.
    pub status: &'static str,
    pub deleted_records: usize,
    pub removed_files: usize,
}

/// Delete every currently-retained cloud artifact for a project: the raw
/// and normalised response files (and their receipts) are removed from
/// disk, and each retention record gets a `deleted_at` tombstone rather
/// than being erased outright, so "what was uploaded and when" stays
/// answerable even after deletion. The spend ledger is never touched here —
/// deleting retained content must never erase the fact that money was
/// already spent (see `cloud_policy`'s module doc).
pub fn delete_cloud_retention(
    project_path: &Path,
    dry_run: bool,
) -> Result<CloudDeletionOutcome, ProjectError> {
    let mut retention = load_cloud_retention(project_path);
    let active: Vec<usize> = retention
        .records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.deleted_at.is_none())
        .map(|(index, _)| index)
        .collect();

    if active.is_empty() {
        return Ok(CloudDeletionOutcome {
            status: "nothing_to_delete",
            deleted_records: 0,
            removed_files: 0,
        });
    }
    if dry_run {
        return Ok(CloudDeletionOutcome {
            status: "dry_run",
            deleted_records: active.len(),
            removed_files: 0,
        });
    }

    let now = Utc::now();
    let mut removed_files = 0usize;
    for index in &active {
        let (raw_rel, normalised_rel) = {
            let record = &retention.records[*index];
            (
                record.raw_response_path.clone(),
                record.normalised_output_path.clone(),
            )
        };
        for relative in [raw_rel, normalised_rel] {
            let path = project_path.join(&relative);
            if path.is_file() {
                std::fs::remove_file(&path)?;
                removed_files += 1;
            }
            let receipt = receipt_path_for(&path);
            if receipt.is_file() {
                std::fs::remove_file(&receipt)?;
            }
        }
        retention.records[*index].deleted_at = Some(now);
    }
    save_cloud_retention(project_path, &retention)?;

    Ok(CloudDeletionOutcome {
        status: "deleted",
        deleted_records: active.len(),
        removed_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud_policy;
    use crate::project_init::init_project;
    use std::fs;

    fn unique_project(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cutright-cloud-test-{label}-{unique}"));
        init_project(&dir, false).expect("init test project");
        dir
    }

    fn write_proxy(project_path: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = project_path.join("cache/proxies").join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }

    fn enable_cloud(project_path: &Path, budget: f64, upload_policy: UploadPolicy) {
        let mut config = load_cloud_config(project_path);
        config.consent = true;
        config.budget_usd_limit = budget;
        config.upload_policy = upload_policy;
        cloud_policy::save_cloud_config(project_path, &config).unwrap();
    }

    fn fake_with(fixture_json: &str) -> FakeProvider {
        FakeProvider::from_fixtures_str(fixture_json).unwrap()
    }

    #[test]
    fn default_config_sends_nothing() {
        let dir = unique_project("default-sends-nothing");
        write_proxy(&dir, "clip.mp4", b"proxy-bytes");
        let provider = fake_with(r#"{"analyze":{}}"#);
        let outcome = analyze_cloud(
            &dir,
            &provider,
            CloudCapability::FrameSemantics,
            None,
            false,
            false,
        )
        .unwrap();
        assert_eq!(outcome.status, "refused");
        assert!(outcome.evidence.is_none());
        assert_eq!(load_cloud_ledger(&dir).total_spent(), 0.0);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn consent_absent_blocks_even_with_budget() {
        let dir = unique_project("consent-absent");
        write_proxy(&dir, "clip.mp4", b"proxy-bytes");
        let mut config = load_cloud_config(&dir);
        config.budget_usd_limit = 100.0; // budget alone is not consent
        cloud_policy::save_cloud_config(&dir, &config).unwrap();
        let provider = fake_with(r#"{"analyze":{}}"#);
        let outcome = analyze_cloud(
            &dir,
            &provider,
            CloudCapability::FrameSemantics,
            None,
            false,
            false,
        )
        .unwrap();
        assert_eq!(outcome.status, "refused");
        assert!(outcome.reason.unwrap().contains("consent"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn zero_budget_blocks_every_call() {
        let dir = unique_project("zero-budget");
        write_proxy(&dir, "clip.mp4", b"proxy-bytes");
        enable_cloud(&dir, 0.0, UploadPolicy::Proxy);
        let provider = fake_with(r#"{"analyze":{}}"#);
        let outcome = analyze_cloud(
            &dir,
            &provider,
            CloudCapability::FrameSemantics,
            None,
            false,
            false,
        )
        .unwrap();
        assert_eq!(outcome.status, "refused");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn budget_exhaustion_refuses_the_next_call() {
        let dir = unique_project("budget-exhaustion");
        let proxy_a = write_proxy(&dir, "a.mp4", b"aaa");
        let hash_a = hash_file(&proxy_a).unwrap();
        // FrameSemantics costs $0.01 per estimated_cost_usd; allow exactly one call.
        enable_cloud(&dir, 0.01, UploadPolicy::Proxy);
        let fixtures = format!(
            r#"{{"analyze":{{"{hash_a}":{{"labels":["hook"],"summary":"s","confidence":0.9,"extra":{{}}}}}}}}"#
        );
        let provider = fake_with(&fixtures);

        let first = analyze_cloud(
            &dir,
            &provider,
            CloudCapability::FrameSemantics,
            Some(Path::new("cache/proxies/a.mp4")),
            false,
            false,
        )
        .unwrap();
        assert_eq!(first.status, "sent");

        // A different capability over the same bytes forces a second real
        // request (the cache is keyed on capability too), which should now
        // be refused because the $0.01 budget is spent.
        let second = analyze_cloud(
            &dir,
            &provider,
            CloudCapability::SegmentSemantics,
            Some(Path::new("cache/proxies/a.mp4")),
            false,
            false,
        )
        .unwrap();
        assert_eq!(second.status, "refused");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn source_upload_refused_when_proxy_policy_set() {
        let dir = unique_project("source-refused");
        write_proxy(&dir, "clip.mp4", b"proxy-bytes");
        enable_cloud(&dir, 100.0, UploadPolicy::Proxy);
        let provider = fake_with(r#"{"analyze":{}}"#);
        let outcome = analyze_cloud(
            &dir,
            &provider,
            CloudCapability::FrameSemantics,
            None,
            true, // use_source
            false,
        )
        .unwrap();
        assert_eq!(outcome.status, "refused");
        assert!(outcome.reason.unwrap().contains("Source"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cache_hit_avoids_a_second_call() {
        let dir = unique_project("cache-hit");
        let proxy = write_proxy(&dir, "clip.mp4", b"proxy-bytes");
        let hash = hash_file(&proxy).unwrap();
        enable_cloud(&dir, 10.0, UploadPolicy::Proxy);
        let fixtures = format!(
            r#"{{"analyze":{{"{hash}":{{"labels":["hook"],"summary":"s","confidence":0.9,"extra":{{}}}}}}}}"#
        );
        let provider = fake_with(&fixtures);

        let first = analyze_cloud(
            &dir,
            &provider,
            CloudCapability::FrameSemantics,
            None,
            false,
            false,
        )
        .unwrap();
        assert_eq!(first.status, "sent");

        let second = analyze_cloud(
            &dir,
            &provider,
            CloudCapability::FrameSemantics,
            None,
            false,
            false,
        )
        .unwrap();
        assert_eq!(second.status, "cached");
        assert_eq!(second.evidence, first.evidence);

        // Only one billable request happened despite two calls.
        assert_eq!(load_cloud_ledger(&dir).records.len(), 1);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn dedupe_prevents_duplicate_uploads_across_capabilities() {
        let dir = unique_project("dedupe-upload");
        let proxy = write_proxy(&dir, "clip.mp4", b"proxy-bytes");
        let hash = hash_file(&proxy).unwrap();
        enable_cloud(&dir, 10.0, UploadPolicy::Proxy);
        let fixtures = format!(
            r#"{{"analyze":{{"{hash}":{{"labels":["hook"],"summary":"s","confidence":0.9,"extra":{{}}}}}}}}"#
        );
        let provider = fake_with(&fixtures);

        analyze_cloud(
            &dir,
            &provider,
            CloudCapability::FrameSemantics,
            None,
            false,
            false,
        )
        .unwrap();
        analyze_cloud(
            &dir,
            &provider,
            CloudCapability::SegmentSemantics,
            None,
            false,
            false,
        )
        .unwrap();

        // Two distinct billable requests (capability differs → cache misses
        // both times)...
        assert_eq!(load_cloud_ledger(&dir).records.len(), 2);
        // ...but exactly one retained upload for this content hash + bytes
        // kind, since the same bytes were never re-uploaded.
        let retention = load_cloud_retention(&dir);
        let matching = retention
            .records
            .iter()
            .filter(|record| {
                record.content_hash == hash && record.bytes_kind == UploadPolicy::Proxy
            })
            .count();
        assert_eq!(matching, 1);
        fs::remove_dir_all(&dir).ok();
    }

    struct AlwaysFailProvider;
    impl CloudProvider for AlwaysFailProvider {
        fn identity(&self) -> ProviderIdentity {
            ProviderIdentity {
                name: "always-fail".to_string(),
                model: "n/a".to_string(),
                version: "0".to_string(),
            }
        }
        fn capabilities(&self) -> &[CloudCapability] {
            &[CloudCapability::FrameSemantics]
        }
        fn analyze(
            &self,
            _request: &CloudAnalysisRequest,
        ) -> Result<CloudSemanticEvidence, CloudError> {
            Err(CloudError::Transport(
                "simulated network outage".to_string(),
            ))
        }
    }

    #[test]
    fn outage_falls_back_to_local_without_erroring_the_pipeline() {
        let dir = unique_project("outage-fallback");
        write_proxy(&dir, "clip.mp4", b"proxy-bytes");
        enable_cloud(&dir, 10.0, UploadPolicy::Proxy);
        let provider = AlwaysFailProvider;
        let outcome = analyze_cloud(
            &dir,
            &provider,
            CloudCapability::FrameSemantics,
            None,
            false,
            false,
        )
        .unwrap();
        assert_eq!(outcome.status, "fallback_local");
        assert!(outcome.reason.unwrap().contains("outage"));
        assert!(outcome.evidence.is_none());
        // No spend was recorded for a call that never succeeded.
        assert_eq!(load_cloud_ledger(&dir).total_spent(), 0.0);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn delete_action_removes_retention_records_but_keeps_spend_history() {
        let dir = unique_project("delete-retention");
        let proxy = write_proxy(&dir, "clip.mp4", b"proxy-bytes");
        let hash = hash_file(&proxy).unwrap();
        enable_cloud(&dir, 10.0, UploadPolicy::Proxy);
        let fixtures = format!(
            r#"{{"analyze":{{"{hash}":{{"labels":["hook"],"summary":"s","confidence":0.9,"extra":{{}}}}}}}}"#
        );
        let provider = fake_with(&fixtures);
        let sent = analyze_cloud(
            &dir,
            &provider,
            CloudCapability::FrameSemantics,
            None,
            false,
            false,
        )
        .unwrap();
        assert_eq!(sent.status, "sent");
        let spent_before_delete = load_cloud_ledger(&dir).total_spent();
        assert!(spent_before_delete > 0.0);

        let deletion = delete_cloud_retention(&dir, false).unwrap();
        assert_eq!(deletion.status, "deleted");
        assert_eq!(deletion.deleted_records, 1);
        assert!(deletion.removed_files >= 1);

        let retention = load_cloud_retention(&dir);
        assert!(retention.records[0].deleted_at.is_some());
        // Spend history survives deletion — budget enforcement cannot be
        // bypassed by an upload/delete/re-upload cycle.
        assert_eq!(load_cloud_ledger(&dir).total_spent(), spent_before_delete);

        let again = delete_cloud_retention(&dir, false).unwrap();
        assert_eq!(again.status, "nothing_to_delete");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn cloud_semantic_evidence_carries_no_timing_field() {
        let evidence = CloudSemanticEvidence {
            labels: vec!["hook".to_string()],
            summary: "advisory only".to_string(),
            confidence: 0.75,
            extra: serde_json::Map::new(),
        };
        let json = serde_json::to_value(&evidence).unwrap();
        let object = json.as_object().unwrap();
        for forbidden in [
            "start_ms",
            "end_ms",
            "output_start_ms",
            "output_end_ms",
            "frame_index",
            "sample_index",
            "word_id",
            "source_word_id",
        ] {
            assert!(
                !object.contains_key(forbidden),
                "CloudSemanticEvidence must never carry a {forbidden} field"
            );
        }
    }

    #[test]
    fn two_phase_index_then_query_round_trips_through_the_fake_provider() {
        let provider = fake_with(
            r#"{
                "analyze": {},
                "index": { "content-a": "index-a" },
                "query": { "index-a::best hook": { "labels": ["hook"], "summary": "q", "confidence": 0.8, "extra": {} } }
            }"#,
        );
        let handle = provider
            .index(&CloudIndexRequest {
                content_hash: "content-a".to_string(),
                bytes_path: PathBuf::from("irrelevant"),
                bytes_kind: UploadPolicy::Proxy,
            })
            .unwrap();
        assert_eq!(handle.index_id, "index-a");

        let evidence = provider
            .query(&CloudQueryRequest {
                index_id: handle.index_id,
                query: "best hook".to_string(),
            })
            .unwrap();
        assert_eq!(evidence.labels, vec!["hook".to_string()]);
    }

    #[test]
    fn disabled_provider_refuses_analyze_index_and_query() {
        let provider = DisabledProvider::new("gemini");
        assert!(provider
            .analyze(&CloudAnalysisRequest {
                capability: CloudCapability::FrameSemantics,
                content_hash: "x".to_string(),
                bytes_path: PathBuf::from("x"),
                bytes_kind: UploadPolicy::Proxy,
            })
            .is_err());
        assert!(provider
            .index(&CloudIndexRequest {
                content_hash: "x".to_string(),
                bytes_path: PathBuf::from("x"),
                bytes_kind: UploadPolicy::Proxy,
            })
            .is_err());
        assert!(provider
            .query(&CloudQueryRequest {
                index_id: "x".to_string(),
                query: "x".to_string(),
            })
            .is_err());
    }

    #[test]
    fn resolve_provider_maps_fake_and_defaults_everything_else_to_disabled() {
        assert_eq!(resolve_provider("fake").identity().name, "fake");
        assert_eq!(resolve_provider("gemini").identity().name, "disabled");
        assert_eq!(resolve_provider("").identity().name, "disabled");
    }

    #[test]
    fn resolve_default_target_picks_the_first_proxy_or_errors_when_none() {
        let dir = unique_project("default-target");
        assert!(resolve_default_target(&dir).is_err());
        write_proxy(&dir, "z.mp4", b"z");
        write_proxy(&dir, "a.mp4", b"a");
        let target = resolve_default_target(&dir).unwrap();
        assert!(target.ends_with("a.mp4"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn credential_env_var_presence_is_reported_without_exposing_its_value() {
        let dir = unique_project("credential-presence");
        write_proxy(&dir, "clip.mp4", b"proxy-bytes");
        // No credential_env_var configured: presence is false.
        let provider = fake_with(r#"{"analyze":{}}"#);
        let outcome = analyze_cloud(
            &dir,
            &provider,
            CloudCapability::FrameSemantics,
            None,
            false,
            false,
        )
        .unwrap();
        assert!(!outcome.credential_present);

        cloud_policy::set_cloud_provider(
            &dir,
            Some("fake".to_string()),
            Some("CUTRIGHT_TEST_CLOUD_CREDENTIAL".to_string()),
            false,
        )
        .unwrap();
        std::env::set_var("CUTRIGHT_TEST_CLOUD_CREDENTIAL", "shhh-not-in-any-output");
        let outcome = analyze_cloud(
            &dir,
            &provider,
            CloudCapability::FrameSemantics,
            None,
            false,
            false,
        )
        .unwrap();
        assert!(outcome.credential_present);
        let json = serde_json::to_string(&outcome).unwrap();
        assert!(!json.contains("shhh-not-in-any-output"));
        std::env::remove_var("CUTRIGHT_TEST_CLOUD_CREDENTIAL");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn capability_parses_snake_and_kebab_case_and_rejects_unknown() {
        assert_eq!(
            CloudCapability::parse("frame-semantics").unwrap(),
            CloudCapability::FrameSemantics
        );
        assert_eq!(
            CloudCapability::parse("segment_semantics").unwrap(),
            CloudCapability::SegmentSemantics
        );
        assert_eq!(
            CloudCapability::parse("video-index").unwrap(),
            CloudCapability::VideoIndex
        );
        assert!(CloudCapability::parse("nonsense").is_err());
    }
}
