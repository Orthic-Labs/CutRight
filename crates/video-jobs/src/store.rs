//! Atomic, project-backed job state.
//!
//! The JSON file is the source of truth.  The in-memory index is only a
//! disposable lookup cache and can always be rebuilt by scanning `jobs/`.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::dag::{JobId, StageId};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
const MAX_LOCK_WAIT: usize = 250;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageState {
    Pending,
    Ready,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

pub const MAX_ATTEMPTS_HISTORY: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageRecord {
    pub stage_id: StageId,
    pub state: StageState,
    pub fingerprint: Option<[u8; 32]>,
    pub attempts: Vec<AttemptRecord>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttemptRecord {
    pub attempt: u32,
    pub outcome: AttemptOutcome,
    pub started_at_unix_ms: u64,
    pub finished_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptOutcome {
    Succeeded,
    Retryable,
    Permanent,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobEvent {
    pub sequence: u64,
    pub revision: u64,
    pub kind: String,
    pub stage_id: Option<StageId>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointRecord {
    pub checkpoint_id: String,
    pub state_hash: [u8; 32],
    pub state_size_bytes: u64,
    pub created_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalWinner {
    Completion,
    Cancellation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalLoser {
    Completion,
    Cancellation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalRace {
    pub winner: TerminalWinner,
    pub loser: TerminalLoser,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageReceipt {
    pub stage_id: StageId,
    pub attempt: u32,
    pub terminal_state: StageState,
    pub output_fingerprint: Option<[u8; 32]>,
    pub revision: u64,
    pub race: Option<TerminalRace>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputRequired {
    pub stage_id: StageId,
    pub reason: String,
}

impl StageRecord {
    pub fn pending(stage_id: impl Into<StageId>) -> Self {
        Self {
            stage_id: stage_id.into(),
            state: StageState::Pending,
            fingerprint: None,
            attempts: Vec::new(),
            last_error: None,
        }
    }

    pub fn succeeded(stage_id: impl Into<StageId>, fingerprint: [u8; 32]) -> Self {
        Self {
            stage_id: stage_id.into(),
            state: StageState::Succeeded,
            fingerprint: Some(fingerprint),
            attempts: Vec::new(),
            last_error: None,
        }
    }

    pub fn transition(&mut self, target: StageState) -> Result<(), StoreError> {
        let ok = matches!(
            (self.state, target),
            (StageState::Pending, StageState::Ready)
                | (StageState::Ready, StageState::Running)
                | (StageState::Running, StageState::Ready)
                | (StageState::Running, StageState::Succeeded)
                | (StageState::Running, StageState::Failed)
                | (StageState::Running, StageState::Cancelled)
                | (StageState::Pending, StageState::Cancelled)
                | (StageState::Ready, StageState::Cancelled)
        );
        if ok {
            self.state = target;
            Ok(())
        } else {
            Err(StoreError::IllegalTransition {
                from: self.state,
                to: target,
            })
        }
    }

    pub fn record_attempt(&mut self, attempt: AttemptRecord) {
        self.attempts.push(attempt);
        if self.attempts.len() > MAX_ATTEMPTS_HISTORY {
            let drop = self.attempts.len() - MAX_ATTEMPTS_HISTORY;
            self.attempts.drain(0..drop);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobRecord {
    pub job_id: JobId,
    pub dag_fingerprint: [u8; 32],
    pub stages: BTreeMap<StageId, StageRecord>,
    pub created_at_unix_ms: u64,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub events: Vec<JobEvent>,
    #[serde(default)]
    pub checkpoints: BTreeMap<StageId, CheckpointRecord>,
    #[serde(default)]
    pub receipts: Vec<StageReceipt>,
    #[serde(default)]
    pub input_required: Option<InputRequired>,
}

impl JobRecord {
    pub fn pending_count(&self) -> usize {
        self.stages
            .values()
            .filter(|s| matches!(s.state, StageState::Pending | StageState::Ready))
            .count()
    }

    pub fn has_receipt(&self, stage_id: &StageId) -> bool {
        self.receipts.iter().any(|r| &r.stage_id == stage_id)
    }

    pub fn push_event(
        &mut self,
        kind: impl Into<String>,
        stage_id: Option<StageId>,
        detail: impl Into<String>,
    ) {
        self.events.push(JobEvent {
            sequence: self.events.len() as u64 + 1,
            revision: self.revision,
            kind: kind.into(),
            stage_id,
            detail: detail.into(),
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreError {
    IllegalTransition { from: StageState, to: StageState },
    UnknownJob(JobId),
    UnknownStage(StageId),
    RevisionConflict { expected: u64, actual: u64 },
    MissingReceipt(StageId),
    InvalidJobId(JobId),
    Io(String),
    Corrupt(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IllegalTransition { from, to } => {
                write!(f, "illegal transition {from:?} -> {to:?}")
            }
            Self::UnknownJob(id) => write!(f, "unknown job {id}"),
            Self::UnknownStage(id) => write!(f, "unknown stage {id}"),
            Self::RevisionConflict { expected, actual } => {
                write!(f, "revision conflict: expected {expected}, actual {actual}")
            }
            Self::MissingReceipt(id) => write!(f, "stage {id} reports success without a receipt"),
            Self::InvalidJobId(id) => write!(f, "invalid job id {id}"),
            Self::Io(message) | Self::Corrupt(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for StoreError {}

fn validate_record(record: &JobRecord) -> Result<(), StoreError> {
    for stage in record.stages.values() {
        if stage.state == StageState::Succeeded && !record.has_receipt(&stage.stage_id) {
            return Err(StoreError::MissingReceipt(stage.stage_id.clone()));
        }
    }
    Ok(())
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Project-backed store. `index` is deliberately disposable; `open` and
/// `rebuild_index` reconstruct it from canonical job files.
pub struct ProjectJobStore {
    project_root: PathBuf,
    index: BTreeMap<JobId, PathBuf>,
}

impl ProjectJobStore {
    pub fn open(project_root: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let project_root = project_root.into();
        fs::create_dir_all(project_root.join("jobs")).map_err(io_error)?;
        let mut store = Self {
            project_root,
            index: BTreeMap::new(),
        };
        store.rebuild_index()?;
        Ok(store)
    }

    pub fn rebuild_index(&mut self) -> Result<(), StoreError> {
        self.index.clear();
        let dir = self.jobs_dir();
        for entry in fs::read_dir(&dir).map_err(io_error)? {
            let path = entry.map_err(io_error)?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let record: JobRecord = read_json(&path)?;
            self.index.insert(record.job_id, path);
        }
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<JobRecord>, StoreError> {
        let mut records = self
            .index
            .values()
            .map(|path| read_json(path))
            .collect::<Result<Vec<JobRecord>, _>>()?;
        records.sort_by(|a, b| a.job_id.cmp(&b.job_id));
        Ok(records)
    }

    pub fn create(&mut self, mut record: JobRecord) -> Result<(), StoreError> {
        if !valid_id(&record.job_id) {
            return Err(StoreError::InvalidJobId(record.job_id));
        }
        let path = self.path_for(&record.job_id);
        let _lock = FileLock::acquire(&path.with_extension("lock"))?;
        if path.exists() {
            return Err(StoreError::Corrupt(format!(
                "job already exists: {}",
                record.job_id
            )));
        }
        record.revision = 0;
        validate_record(&record)?;
        write_json_atomic(&path, &record)?;
        self.index.insert(record.job_id, path);
        Ok(())
    }

    pub fn load(&self, job_id: &str) -> Result<JobRecord, StoreError> {
        if !valid_id(job_id) {
            return Err(StoreError::InvalidJobId(job_id.to_string()));
        }
        let path = self.path_for(job_id);
        if !path.is_file() {
            return Err(StoreError::UnknownJob(job_id.to_string()));
        }
        read_json(&path)
    }

    /// Atomically replace a record only when both caller and next revisions
    /// are exact. The lock closes the read/compare/write race between daemons.
    pub fn compare_and_swap(
        &mut self,
        job_id: &str,
        expected_revision: u64,
        next: JobRecord,
    ) -> Result<JobRecord, StoreError> {
        if next.job_id != job_id {
            return Err(StoreError::Corrupt("job id changed during CAS".into()));
        }
        if next.revision != expected_revision.saturating_add(1) {
            return Err(StoreError::RevisionConflict {
                expected: expected_revision + 1,
                actual: next.revision,
            });
        }
        let path = self.path_for(job_id);
        let _lock = FileLock::acquire(&path.with_extension("lock"))?;
        let current: JobRecord = read_json(&path)?;
        if current.revision != expected_revision {
            return Err(StoreError::RevisionConflict {
                expected: expected_revision,
                actual: current.revision,
            });
        }
        validate_record(&next)?;
        write_json_atomic(&path, &next)?;
        self.index.insert(job_id.to_string(), path);
        Ok(next)
    }

    /// Mutate a record under the same exact-revision CAS used by daemons.
    pub fn transact<F>(
        &mut self,
        job_id: &str,
        expected_revision: u64,
        mutate: F,
    ) -> Result<JobRecord, StoreError>
    where
        F: FnOnce(&mut JobRecord) -> Result<(), StoreError>,
    {
        let mut next = self.load(job_id)?;
        if next.revision != expected_revision {
            return Err(StoreError::RevisionConflict {
                expected: expected_revision,
                actual: next.revision,
            });
        }
        mutate(&mut next)?;
        next.revision += 1;
        self.compare_and_swap(job_id, expected_revision, next)
    }

    fn jobs_dir(&self) -> PathBuf {
        self.project_root.join("jobs")
    }
    fn path_for(&self, job_id: &str) -> PathBuf {
        self.jobs_dir().join(format!("{job_id}.json"))
    }
}

fn io_error(error: std::io::Error) -> StoreError {
    StoreError::Io(error.to_string())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, StoreError> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(io_error)?
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    serde_json::from_slice(&bytes)
        .map_err(|e| StoreError::Corrupt(format!("{}: {e}", path.display())))
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), StoreError> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| StoreError::Corrupt(e.to_string()))?;
    let temp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = File::create(&temp).map_err(io_error)?;
        file.write_all(&bytes).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
        fs::rename(&temp, path).map_err(io_error)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

struct FileLock {
    path: PathBuf,
}

impl FileLock {
    fn acquire(path: &Path) -> Result<Self, StoreError> {
        for _ in 0..MAX_LOCK_WAIT {
            match OpenOptions::new().write(true).create_new(true).open(path) {
                Ok(_) => {
                    return Ok(Self {
                        path: path.to_path_buf(),
                    })
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if let Ok(meta) = fs::metadata(path) {
                        if let Ok(age) = meta
                            .modified()
                            .and_then(|t| t.elapsed().map_err(std::io::Error::other))
                        {
                            if age > Duration::from_secs(5) {
                                let _ = fs::remove_file(path);
                            }
                        }
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(error) => return Err(io_error(error)),
            }
        }
        Err(StoreError::Io(format!(
            "timed out acquiring {}",
            path.display()
        )))
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn pending_to_ready_is_legal() {
        let mut stage = StageRecord::pending("a");
        stage.transition(StageState::Ready).unwrap();
        assert_eq!(stage.state, StageState::Ready);
    }

    #[test]
    fn running_can_return_to_ready_after_restart() {
        let mut stage = StageRecord::pending("a");
        stage.transition(StageState::Ready).unwrap();
        stage.transition(StageState::Running).unwrap();
        stage.transition(StageState::Ready).unwrap();
        assert_eq!(stage.state, StageState::Ready);
    }

    #[test]
    fn attempt_history_is_bounded() {
        let mut stage = StageRecord::pending("a");
        for i in 0..(MAX_ATTEMPTS_HISTORY + 3) {
            stage.record_attempt(AttemptRecord {
                attempt: i as u32,
                outcome: AttemptOutcome::Retryable,
                started_at_unix_ms: 0,
                finished_at_unix_ms: 0,
            });
        }
        assert_eq!(stage.attempts.len(), MAX_ATTEMPTS_HISTORY);
    }

    #[test]
    fn store_round_trip_and_exact_cas() {
        let dir = tempdir().unwrap();
        let mut store = ProjectJobStore::open(dir.path()).unwrap();
        let record = JobRecord {
            job_id: "j".into(),
            dag_fingerprint: [1; 32],
            stages: BTreeMap::from([("a".into(), StageRecord::pending("a"))]),
            created_at_unix_ms: 0,
            revision: 0,
            events: vec![],
            checkpoints: BTreeMap::new(),
            receipts: vec![],
            input_required: None,
        };
        store.create(record).unwrap();
        let next = store
            .transact("j", 0, |job| {
                job.push_event("created", None, "pending");
                Ok(())
            })
            .unwrap();
        assert_eq!(next.revision, 1);
        assert!(matches!(
            store.transact("j", 0, |_| Ok(())),
            Err(StoreError::RevisionConflict { .. })
        ));
        let mut rebuilt = ProjectJobStore::open(dir.path()).unwrap();
        assert_eq!(rebuilt.list().unwrap()[0].events.len(), 1);
        rebuilt.rebuild_index().unwrap();
    }
}
