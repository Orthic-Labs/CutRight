//! Versioned project migrations with backups and dry-run.
//!
//! Implements the v2 contract frozen by `27d5682..50f76d5`:
//! - `docs/architecture/V2-IDENTITY-TIME-REVISION.md` §4 — explicit
//!   migrations from v1 string IDs / millisecond fields to v2 stable IDs and
//!   rational time.
//!
//! ## Migration descriptors
//!
//! Each step is a JSON file under `migrations/v{N}-to-v{M}/`. The schema is
//! intentionally minimal so the runner can both plan and report touched
//! fields without loading a runtime:
//!
//! ```json
//! {
//!   "from": "v1",
//!   "to": "v2",
//!   "step": 1,
//!   "name": "identity-map",
//!   "requires_backup": true,
//!   "touched_fields": ["records[*].id"],
//!   "description": "..."
//! }
//! ```
//!
//! ## Backups
//!
//! Any step with `requires_backup: true` triggers a backup of `.state/`
//! into `.state/backups/<timestamp>.tar` before the step mutates anything.
//! The archive is a POSIX ustar stream with no compression; the spec wire
//! name is `.tar.zst` but the compression layer is pluggable and out of
//! scope for lane P-C. The runner restores a backup by extracting its
//! entries back over `.state/` with temp-dir + rename per entry.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Directory under the project root that holds the migration descriptors.
pub const MIGRATIONS_DIR: &str = "migrations";
/// Subdirectory under `STATE_DIR` that holds backup archives.
pub const BACKUPS_SUBDIR: &str = "backups";
/// Directory under the project root that holds the live state.
pub const STATE_DIR: &str = ".state";

/// A single migration step, loaded from a JSON descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationStep {
    /// Source schema version the step consumes.
    pub from: String,
    /// Target schema version the step produces.
    pub to: String,
    /// 1-based ordering within the from→to plan.
    pub step: u32,
    /// Stable name (snake_case).
    pub name: String,
    /// Whether the step is destructive and must be preceded by a backup.
    pub requires_backup: bool,
    /// JSON paths the step will mutate (planning visibility only).
    pub touched_fields: Vec<String>,
    /// Human-readable description; preserved for receipts and dry-run output.
    pub description: String,
}

/// A plan grouped by from→to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationPlan {
    /// Source schema version.
    pub from: String,
    /// Target schema version.
    pub to: String,
    /// Ordered steps in apply order.
    pub steps: Vec<MigrationStep>,
    /// Aggregated touched fields across all steps.
    pub touched_fields: Vec<String>,
    /// Number of steps that require a backup.
    pub backup_count: usize,
}

/// Outcome of a single migration apply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationOutcome {
    /// Plan that was applied.
    pub plan: MigrationPlan,
    /// Absolute path to the backup archive created (if any step required one).
    pub backup_path: Option<PathBuf>,
    /// BLAKE3 hex digest of the post-apply state directory.
    pub post_state_hash: String,
    /// Per-step receipts (one per step, in apply order).
    pub step_receipts: Vec<MigrationStepReceipt>,
}

/// Receipt for a single applied step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationStepReceipt {
    /// Step index in the plan.
    pub step: u32,
    /// Step name.
    pub name: String,
    /// Whether a backup was created before this step.
    pub backup_created: bool,
    /// Path to the backup archive (if any).
    pub backup_path: Option<PathBuf>,
}

/// Verification report for a backup restore.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationVerifyReport {
    /// Path to the backup that was restored.
    pub backup_path: PathBuf,
    /// BLAKE3 hex digest of the restored state directory.
    pub restored_state_hash: String,
    /// Number of entries the archive wrote.
    pub entries_restored: usize,
}

/// Errors raised by the migration runner.
#[derive(Debug, Error)]
pub enum MigrationError {
    /// I/O error during a migration file system operation.
    #[error("migration I/O error at {path}: {source}")]
    Io {
        /// Path the operation was acting on.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// A migration descriptor was structurally invalid.
    #[error("invalid migration descriptor at {path}: {message}")]
    InvalidDescriptor {
        /// Path of the offending descriptor.
        path: PathBuf,
        /// Human-readable description.
        message: String,
    },
    /// The on-disk archive cannot be parsed or read.
    #[error("backup archive error at {path}: {message}")]
    BadArchive {
        /// Path of the archive.
        path: PathBuf,
        /// Human-readable description.
        message: String,
    },
    /// The plan could not be built because the from→to range is unsupported.
    #[error("no migration path from `{from}` to `{to}`")]
    NoPath {
        /// Source schema version.
        from: String,
        /// Target schema version.
        to: String,
    },
    /// A descriptor's from/to fields disagreed with the requested plan.
    #[error("migration step `{name}` has from=`{step_from}` to=`{step_to}` but plan requested from=`{plan_from}` to=`{plan_to}`")]
    MismatchedStep {
        /// Step name.
        name: String,
        /// Step's declared source.
        step_from: String,
        /// Step's declared target.
        step_to: String,
        /// Plan's source.
        plan_from: String,
        /// Plan's target.
        plan_to: String,
    },
}

/// A single in-memory entry extracted from a backup archive.
#[derive(Debug, Clone, PartialEq)]
pub struct ArchiveEntry {
    /// Path relative to the archive root (typically `.state/...`).
    pub relative_path: PathBuf,
    /// File contents (empty for directory entries).
    pub contents: Vec<u8>,
    /// Declared size in bytes (zero for directories).
    pub size: u64,
}

/// Discover every migration step under `migrations_dir`. Walks one level
/// down so a `migrations/v1-to-v2/` layout is supported alongside a flat
/// `migrations/` layout.
pub fn discover_steps(migrations_dir: &Path) -> Result<Vec<MigrationStep>, MigrationError> {
    if !migrations_dir.exists() {
        return Ok(Vec::new());
    }
    let mut steps: Vec<MigrationStep> = Vec::new();
    let root_read = fs::read_dir(migrations_dir).map_err(|source| MigrationError::Io {
        path: migrations_dir.to_path_buf(),
        source,
    })?;
    for entry in root_read {
        let entry = entry.map_err(|source| MigrationError::Io {
            path: migrations_dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let meta = entry.metadata().map_err(|source| MigrationError::Io {
            path: path.clone(),
            source,
        })?;
        if meta.is_dir() {
            collect_descriptors(&path, &mut steps)?;
        } else if meta.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some("json")
        {
            collect_descriptor(&path, &mut steps)?;
        }
    }
    steps.sort_by_key(|s| s.step);
    Ok(steps)
}

fn collect_descriptors(
    dir: &Path,
    out: &mut Vec<MigrationStep>,
) -> Result<(), MigrationError> {
    let read = fs::read_dir(dir).map_err(|source| MigrationError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in read {
        let entry = entry.map_err(|source| MigrationError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let meta = entry.metadata().map_err(|source| MigrationError::Io {
            path: path.clone(),
            source,
        })?;
        if meta.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            collect_descriptor(&path, out)?;
        } else if meta.is_dir() {
            collect_descriptors(&path, out)?;
        }
    }
    Ok(())
}

fn collect_descriptor(
    path: &Path,
    out: &mut Vec<MigrationStep>,
) -> Result<(), MigrationError> {
    let bytes = fs::read(path).map_err(|source| MigrationError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let step: MigrationStep = serde_json::from_slice(&bytes).map_err(|err| {
        MigrationError::InvalidDescriptor {
            path: path.to_path_buf(),
            message: err.to_string(),
        }
    })?;
    out.push(step);
    Ok(())
}

/// Migration runner state.
#[derive(Debug, Clone)]
pub struct MigrationRunner {
    /// Project root containing `.state/` and `migrations/`.
    project_root: PathBuf,
    /// Path to the migration descriptor directory.
    migrations_dir: PathBuf,
}

impl MigrationRunner {
    /// Build a runner scoped to `project_root`. `migrations_dir` defaults to
    /// `<project_root>/migrations`.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        let root = project_root.into();
        let migrations_dir = root.join(MIGRATIONS_DIR);
        Self {
            project_root: root,
            migrations_dir,
        }
    }

    /// Override the migration descriptor directory.
    pub fn with_migrations_dir(mut self, migrations_dir: impl Into<PathBuf>) -> Self {
        self.migrations_dir = migrations_dir.into();
        self
    }

    /// Project root the runner is scoped to.
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Path to the migration descriptor directory.
    pub fn migrations_dir(&self) -> &Path {
        &self.migrations_dir
    }

    /// Path to the live state directory.
    pub fn state_dir(&self) -> PathBuf {
        self.project_root.join(STATE_DIR)
    }

    /// Path to the backups directory.
    pub fn backups_dir(&self) -> PathBuf {
        self.state_dir().join(BACKUPS_SUBDIR)
    }

    /// Build a plan from `from` → `to` by selecting the steps whose
    /// `from`/`to` fields match the request and whose `step` numbers are
    /// monotonic.
    pub fn plan(&self, from: &str, to: &str) -> Result<MigrationPlan, MigrationError> {
        let all = discover_steps(&self.migrations_dir)?;
        let mut steps: Vec<MigrationStep> = all
            .into_iter()
            .filter(|s| s.from == from && s.to == to)
            .collect();
        if steps.is_empty() {
            return Err(MigrationError::NoPath {
                from: from.to_string(),
                to: to.to_string(),
            });
        }
        steps.sort_by_key(|s| s.step);
        for (i, step) in steps.iter().enumerate() {
            if step.step as usize != i + 1 {
                return Err(MigrationError::InvalidDescriptor {
                    path: self.migrations_dir.join(format!("{}.json", step.name)),
                    message: format!(
                        "step number {} is not contiguous in from={} to={} plan",
                        step.step, from, to
                    ),
                });
            }
        }
        let touched_fields: Vec<String> = steps
            .iter()
            .flat_map(|s| s.touched_fields.iter().cloned())
            .collect();
        let backup_count = steps.iter().filter(|s| s.requires_backup).count();
        Ok(MigrationPlan {
            from: from.to_string(),
            to: to.to_string(),
            steps,
            touched_fields,
            backup_count,
        })
    }

    /// Walk the plan in dry-run mode. Reports every step that would run,
    /// every touched field, and whether a backup would be created. No
    /// filesystem state is modified.
    pub fn dry_run(&self, plan: &MigrationPlan) -> Result<MigrationPlan, MigrationError> {
        self.validate_plan(plan)?;
        Ok(plan.clone())
    }

    /// Apply the plan. Creates a backup archive before each destructive step
    /// (and exactly once before the first destructive step if several are
    /// contiguous). The apply itself does not perform the schema transform;
    /// it produces a receipt that records the human decision to apply and
    /// the per-step backup decision. Schema transform mechanics are owned by
    /// the lane that owns the new schema (P-A / P-B / orchestrator).
    pub fn apply(&self, plan: &MigrationPlan) -> Result<MigrationOutcome, MigrationError> {
        self.validate_plan(plan)?;
        let mut receipts: Vec<MigrationStepReceipt> = Vec::new();
        let mut backup_path: Option<PathBuf> = None;
        for step in &plan.steps {
            let mut backup_created = false;
            if step.requires_backup && backup_path.is_none() {
                let path = self.create_backup(&receipts)?;
                backup_path = Some(path);
                backup_created = true;
            }
            receipts.push(MigrationStepReceipt {
                step: step.step,
                name: step.name.clone(),
                backup_created,
                backup_path: backup_path.clone(),
            });
        }
        let post_state_hash = hash_directory(&self.state_dir())?;
        Ok(MigrationOutcome {
            plan: plan.clone(),
            backup_path,
            post_state_hash,
            step_receipts: receipts,
        })
    }

    /// Restore `backup_path` over the current `.state/` directory. Returns a
    /// verification report whose `restored_state_hash` is the BLAKE3 over
    /// the restored tree.
    pub fn restore_backup(
        &self,
        backup_path: &Path,
    ) -> Result<MigrationVerifyReport, MigrationError> {
        let state_dir = self.state_dir();
        let entries = read_archive(backup_path)?;
        let mut entries_restored = 0usize;
        for entry in entries {
            let target = state_dir.join(&entry.relative_path);
            if entry.contents.is_empty() && entry.size == 0 {
                fs::create_dir_all(&target).map_err(|source| MigrationError::Io {
                    path: target.clone(),
                    source,
                })?;
                entries_restored += 1;
                continue;
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|source| MigrationError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            let temp = unique_temp_file(&target);
            fs::write(&temp, &entry.contents).map_err(|source| MigrationError::Io {
                path: temp.clone(),
                source,
            })?;
            fs::rename(&temp, &target).map_err(|source| MigrationError::Io {
                path: target.clone(),
                source,
            })?;
            entries_restored += 1;
        }
        let restored_state_hash = hash_directory(&state_dir)?;
        Ok(MigrationVerifyReport {
            backup_path: backup_path.to_path_buf(),
            restored_state_hash,
            entries_restored,
        })
    }

    /// Validate that every step in `plan` belongs to the plan's `from`/`to`
    /// pair. Used by both `dry_run` and `apply`.
    fn validate_plan(&self, plan: &MigrationPlan) -> Result<(), MigrationError> {
        for step in &plan.steps {
            if step.from != plan.from || step.to != plan.to {
                return Err(MigrationError::MismatchedStep {
                    name: step.name.clone(),
                    step_from: step.from.clone(),
                    step_to: step.to.clone(),
                    plan_from: plan.from.clone(),
                    plan_to: plan.to.clone(),
                });
            }
        }
        Ok(())
    }

    /// Create a backup archive of the current `.state/` directory.
    /// `prior_receipts` is the list of receipts already produced for this
    /// apply; they are written into the archive so the backup is a complete
    /// snapshot of the project's pre-apply state.
    fn create_backup(
        &self,
        prior_receipts: &[MigrationStepReceipt],
    ) -> Result<PathBuf, MigrationError> {
        let backups_dir = self.backups_dir();
        fs::create_dir_all(&backups_dir).map_err(|source| MigrationError::Io {
            path: backups_dir.clone(),
            source,
        })?;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let backup_path = backups_dir.join(format!("backup-{nonce}.tar"));
        let mut archive = write_archive(&backup_path)?;
        let state_dir = self.state_dir();
        if state_dir.exists() {
            append_dir_to_archive(&mut archive, &state_dir, &state_dir)?;
        }
        let receipts_bytes = serde_json::to_vec_pretty(prior_receipts).map_err(|err| {
            MigrationError::BadArchive {
                path: backup_path.clone(),
                message: err.to_string(),
            }
        })?;
        write_archive_entry(&mut archive, Path::new("receipts.json"), &receipts_bytes)?;
        finish_archive(&mut archive)?;
        Ok(backup_path)
    }
}

/// Compute a content hash of a directory tree by walking its files in
/// deterministic order and feeding each `(relative_path, contents)` to a
/// single BLAKE3 hasher.
fn hash_directory(root: &Path) -> Result<String, MigrationError> {
    if !root.exists() {
        return Ok(blake3::hash(&[]).to_hex().to_string());
    }
    let mut paths: Vec<PathBuf> = Vec::new();
    collect_files(root, root, &mut paths)?;
    paths.sort();
    let mut hasher = blake3::Hasher::new();
    for path in paths {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        hasher.update(rel.as_bytes());
        hasher.update(b"\0");
        let bytes = fs::read(&path).map_err(|source| MigrationError::Io {
            path: path.clone(),
            source,
        })?;
        hasher.update(&bytes);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), MigrationError> {
    let read = fs::read_dir(dir).map_err(|source| MigrationError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in read {
        let entry = entry.map_err(|source| MigrationError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let meta = entry.metadata().map_err(|source| MigrationError::Io {
            path: path.clone(),
            source,
        })?;
        if meta.is_dir() {
            collect_files(root, &path, out)?;
        } else if meta.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

/// Write a directory recursively into an archive.
fn append_dir_to_archive(
    archive: &mut ArchiveWriter,
    root: &Path,
    dir: &Path,
) -> Result<(), MigrationError> {
    let read = fs::read_dir(dir).map_err(|source| MigrationError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in read {
        let entry = entry.map_err(|source| MigrationError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let meta = entry.metadata().map_err(|source| MigrationError::Io {
            path: path.clone(),
            source,
        })?;
        if meta.is_dir() {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_path_buf();
            write_archive_dir(archive, &rel)?;
            append_dir_to_archive(archive, root, &path)?;
        } else if meta.is_file() {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_path_buf();
            let bytes = fs::read(&path).map_err(|source| MigrationError::Io {
                path: path.clone(),
                source,
            })?;
            write_archive_entry(archive, &rel, &bytes)?;
        }
    }
    Ok(())
}

/// Lightweight POSIX ustar writer.
///
/// The archive consists of a 512-byte header per entry, then the entry bytes
/// padded to a 512-byte boundary, then two 512-byte zero blocks marking EOF.
/// This is a focused re-implementation of the `tar` crate's writer for files
/// we control; it does not aim to be a general tar implementation.
struct ArchiveWriter {
    file: File,
}

fn write_archive(path: &Path) -> Result<ArchiveWriter, MigrationError> {
    let file = File::create(path).map_err(|source| MigrationError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(ArchiveWriter { file })
}

fn write_archive_entry(
    archive: &mut ArchiveWriter,
    relative_path: &Path,
    bytes: &[u8],
) -> Result<(), MigrationError> {
    let header = build_header(relative_path, bytes.len() as u64, false);
    archive.file.write_all(&header).map_err(|source| MigrationError::Io {
        path: PathBuf::from("<archive>"),
        source,
    })?;
    archive.file.write_all(bytes).map_err(|source| MigrationError::Io {
        path: PathBuf::from("<archive>"),
        source,
    })?;
    let pad = padding_for(bytes.len());
    let zeros = vec![0u8; pad];
    archive.file.write_all(&zeros).map_err(|source| MigrationError::Io {
        path: PathBuf::from("<archive>"),
        source,
    })?;
    Ok(())
}

fn write_archive_dir(
    archive: &mut ArchiveWriter,
    relative_path: &Path,
) -> Result<(), MigrationError> {
    let mut name = relative_path.to_path_buf();
    if !name.to_string_lossy().ends_with('/') {
        name.push("");
    }
    let header = build_header(&name, 0, true);
    archive.file.write_all(&header).map_err(|source| MigrationError::Io {
        path: PathBuf::from("<archive>"),
        source,
    })?;
    Ok(())
}

fn finish_archive(archive: &mut ArchiveWriter) -> Result<(), MigrationError> {
    let zeros = vec![0u8; 1024];
    archive.file.write_all(&zeros).map_err(|source| MigrationError::Io {
        path: PathBuf::from("<archive>"),
        source,
    })?;
    archive.file.flush().map_err(|source| MigrationError::Io {
        path: PathBuf::from("<archive>"),
        source,
    })?;
    Ok(())
}

fn build_header(name: &Path, size: u64, is_dir: bool) -> [u8; 512] {
    let mut header = [0u8; 512];
    let name_bytes = name.to_string_lossy();
    let name_bytes = name_bytes.as_bytes();
    let name_len = name_bytes.len().min(99);
    header[..name_len].copy_from_slice(&name_bytes[..name_len]);
    // mode (octal, ASCII "0000777\0")
    let mode = if is_dir { "0000777\0" } else { "0000644\0" };
    header[100..108].copy_from_slice(mode.as_bytes());
    // uid / gid (ASCII zeros)
    header[108..116].copy_from_slice(b"0000000\0");
    header[116..124].copy_from_slice(b"0000000\0");
    // size (octal, ASCII)
    let size_field = format!("{:011o}\0", size);
    header[124..136].copy_from_slice(size_field.as_bytes());
    // mtime (octal zero)
    header[136..148].copy_from_slice(b"00000000000\0");
    // checksum placeholder (8 spaces, then a NUL + type flag)
    header[148..156].copy_from_slice(b"        ");
    // type flag
    header[156] = if is_dir { b'5' } else { b'0' };
    // ustar magic
    header[257..263].copy_from_slice(b"ustar\0");
    // version
    header[263..265].copy_from_slice(b"00");
    // Compute checksum: sum of all bytes treating the checksum field as 8
    // spaces.
    let checksum = header.iter().map(|b| *b as u32).sum::<u32>();
    let checksum_field = format!("{:06o}\0 ", checksum);
    header[148..156].copy_from_slice(checksum_field.as_bytes());
    header
}

fn padding_for(len: usize) -> usize {
    let rem = len % 512;
    if rem == 0 {
        0
    } else {
        512 - rem
    }
}

fn read_archive(path: &Path) -> Result<Vec<ArchiveEntry>, MigrationError> {
    let mut file = File::open(path).map_err(|source| MigrationError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).map_err(|source| MigrationError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut entries = Vec::new();
    let mut cursor = 0usize;
    while cursor + 512 <= buf.len() {
        let header = &buf[cursor..cursor + 512];
        if header.iter().all(|b| *b == 0) {
            break;
        }
        let name = parse_header_name(header);
        let size = parse_header_size(header);
        let type_flag = header[156];
        cursor += 512;
        let contents = if type_flag == b'5' {
            Vec::new()
        } else {
            let end = cursor + size as usize;
            if end > buf.len() {
                return Err(MigrationError::BadArchive {
                    path: path.to_path_buf(),
                    message: format!("entry `{}` size {} exceeds archive size", name, size),
                });
            }
            let bytes = buf[cursor..end].to_vec();
            cursor = end + padding_for(size as usize);
            bytes
        };
        entries.push(ArchiveEntry {
            relative_path: PathBuf::from(name),
            contents,
            size,
        });
    }
    Ok(entries)
}

fn parse_header_name(header: &[u8]) -> String {
    let name_end = header
        .iter()
        .take(100)
        .position(|b| *b == 0)
        .unwrap_or(100);
    String::from_utf8_lossy(&header[..name_end]).into_owned()
}

fn parse_header_size(header: &[u8]) -> u64 {
    let size_field = &header[124..136];
    let trimmed = std::str::from_utf8(size_field).unwrap_or("0").trim_end_matches('\0');
    let trimmed = trimmed.trim();
    if trimmed.is_empty() {
        return 0;
    }
    u64::from_str_radix(trimmed, 8).unwrap_or(0)
}

fn unique_temp_file(target: &Path) -> PathBuf {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let stem = target
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    parent.join(format!(".tmp-{stem}-{nonce:x}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a directory under `dir` for the test fixture.
    fn ensure_dir(dir: &Path, relative: &str) -> PathBuf {
        let path = dir.join(relative);
        fs::create_dir_all(&path).expect("mkdir");
        path
    }

    fn unique_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cutright-migrate-test-{nanos}-{counter}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    fn write_descriptor(dir: &Path, step: &MigrationStep) -> PathBuf {
        let migrations_subdir = ensure_dir(dir, "migrations/v1-to-v2");
        let path = migrations_subdir.join(format!("{:02}-{}.json", step.step, step.name));
        let bytes = serde_json::to_vec_pretty(step).expect("encode");
        fs::write(&path, bytes).expect("write");
        path
    }

    fn step(step: u32, name: &str, requires_backup: bool) -> MigrationStep {
        MigrationStep {
            from: "v1".to_string(),
            to: "v2".to_string(),
            step,
            name: name.to_string(),
            requires_backup,
            touched_fields: vec![format!("{name}.field")],
            description: format!("Test step {step}"),
        }
    }

    #[test]
    fn plan_selects_steps_for_from_to_range() {
        let dir = unique_dir();
        write_descriptor(&dir, &step(1, "identity-map", true));
        write_descriptor(&dir, &step(2, "ms-to-ns", true));
        write_descriptor(&dir, &step(3, "time-anchor", false));
        let runner = MigrationRunner::new(&dir);
        let plan = runner.plan("v1", "v2").expect("plan");
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.backup_count, 2);
        assert_eq!(plan.touched_fields.len(), 3);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn plan_rejects_unsupported_range() {
        let dir = unique_dir();
        let runner = MigrationRunner::new(&dir);
        let err = runner.plan("v0", "v1").unwrap_err();
        assert!(matches!(err, MigrationError::NoPath { .. }));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dry_run_does_not_write_to_disk() {
        let dir = unique_dir();
        write_descriptor(&dir, &step(1, "identity-map", true));
        write_descriptor(&dir, &step(2, "ms-to-ns", true));
        let runner = MigrationRunner::new(&dir);
        let plan = runner.plan("v1", "v2").expect("plan");
        let snapshot_before = hash_directory(&runner.state_dir()).expect("snapshot");
        let dry = runner.dry_run(&plan).expect("dry-run");
        assert_eq!(dry, plan);
        let snapshot_after = hash_directory(&runner.state_dir()).expect("snapshot");
        assert_eq!(snapshot_before, snapshot_after);
        let backups_dir = runner.backups_dir();
        assert!(!backups_dir.exists() || fs::read_dir(&backups_dir).unwrap().next().is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_creates_backup_before_first_destructive_step() {
        let dir = unique_dir();
        write_descriptor(&dir, &step(1, "identity-map", true));
        write_descriptor(&dir, &step(2, "ms-to-ns", true));
        write_descriptor(&dir, &step(3, "time-anchor", false));
        let runner = MigrationRunner::new(&dir);
        let plan = runner.plan("v1", "v2").expect("plan");
        let outcome = runner.apply(&plan).expect("apply");
        assert!(outcome.backup_path.is_some());
        assert_eq!(outcome.step_receipts.len(), 3);
        assert!(outcome.step_receipts[0].backup_created);
        assert!(!outcome.step_receipts[1].backup_created);
        assert!(!outcome.step_receipts[2].backup_created);
        let backup_path = outcome.backup_path.clone().unwrap();
        assert!(backup_path.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn dry_run_and_apply_diverge_on_filesystem_side_effects() {
        let dir = unique_dir();
        write_descriptor(&dir, &step(1, "identity-map", true));
        let runner_dry = MigrationRunner::new(&dir);
        let plan = runner_dry.plan("v1", "v2").expect("plan");
        let dry_snapshot = hash_directory(&runner_dry.state_dir()).expect("snapshot");
        runner_dry.dry_run(&plan).expect("dry-run");
        let dry_after = hash_directory(&runner_dry.state_dir()).expect("snapshot");
        assert_eq!(dry_snapshot, dry_after);
        assert!(!runner_dry.backups_dir().exists()
            || fs::read_dir(runner_dry.backups_dir()).unwrap().next().is_none());

        let runner_apply = MigrationRunner::new(&dir);
        let outcome = runner_apply.apply(&plan).expect("apply");
        assert!(outcome.backup_path.is_some());
        assert!(runner_apply.backups_dir().exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn backup_restoration_round_trips_state() {
        let dir = unique_dir();
        let state_dir = dir.join(STATE_DIR);
        let rev_dir = state_dir.join("revisions").join("rev_test");
        fs::create_dir_all(&rev_dir).unwrap();
        let payload = b"first-state-bytes";
        fs::write(rev_dir.join("state.json"), payload).unwrap();
        let backups_dir = state_dir.join(BACKUPS_SUBDIR);
        fs::create_dir_all(&backups_dir).unwrap();
        let runner = MigrationRunner::new(&dir);
        let backup_path = runner.create_backup(&[]).expect("backup");
        assert!(backup_path.exists());
        fs::write(rev_dir.join("state.json"), b"corrupted").unwrap();
        let report = runner.restore_backup(&backup_path).expect("restore");
        // The backup at minimum contains the original state and the
        // receipts.json the runner writes at the end of every archive.
        assert!(report.entries_restored > 0);
        let restored_bytes =
            fs::read(state_dir.join("revisions/rev_test/state.json")).expect("read");
        assert_eq!(restored_bytes, payload);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mismatched_step_in_plan_is_rejected() {
        let dir = unique_dir();
        write_descriptor(&dir, &step(1, "identity-map", true));
        let runner = MigrationRunner::new(&dir);
        let plan = runner.plan("v1", "v2").expect("plan");
        let mut bad = plan.clone();
        bad.steps[0].from = "v0".to_string();
        let err = runner.apply(&bad).unwrap_err();
        assert!(matches!(err, MigrationError::MismatchedStep { .. }));
        let _ = fs::remove_dir_all(&dir);
    }
}
