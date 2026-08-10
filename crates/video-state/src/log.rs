//! Hash-chained, append-only event logs.
//!
//! Three log kinds live under `.state/logs/`:
//!
//! - `actions.jsonl` — every action batch and its terminal result.
//! - `decisions.jsonl` — user decisions (accept / reject / defer) on
//!   capability output, critic verdicts, and skill proposals.
//! - `audit.jsonl` — security-relevant events (permission denials, cross
//!   project write attempts, lock contention, retention resets).
//!
//! Each log line is a JSON object:
//!
//! ```json
//! {"seq": 7, "prev_hash": "<hex>", "record_hash": "<hex>", "payload": {...}}
//! ```
//!
//! `record_hash` is the BLAKE3 digest of `prev_hash || seq || payload_json`.
//! The first record's `prev_hash` is the 32-byte zero hash. Verifying the log
//! is a single linear pass over the lines and re-derives each `record_hash`
//! from the previous one. Any tampering, truncation, insertion, or replay
//! breaks the chain at a deterministic line.

use std::fs;
use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Schema tag emitted on every log record. Kept stable across rule changes so
/// out-of-band parsers can fail closed on a mismatched tag.
pub const LOG_RECORD_SCHEMA: &str = "cutright.log_record/v1";

/// Directory under the project root that holds every log file.
pub const STATE_DIR: &str = ".state";
/// Subdirectory under `STATE_DIR` that holds the three log files.
pub const LOGS_SUBDIR: &str = "logs";

/// The three append-only log kinds the project requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogKind {
    /// Action batches and their terminal results.
    Actions,
    /// User decisions and critic verdicts.
    Decisions,
    /// Security-relevant events.
    Audit,
}

impl LogKind {
    /// File name (without extension) for this log.
    pub fn file_stem(self) -> &'static str {
        match self {
            LogKind::Actions => "actions",
            LogKind::Decisions => "decisions",
            LogKind::Audit => "audit",
        }
    }

    /// Resolve the file name (including `.jsonl`) for this log.
    pub fn file_name(self) -> String {
        format!("{}.jsonl", self.file_stem())
    }

    /// Iterate over every log kind. Order matches the on-disk layout.
    pub fn all() -> [LogKind; 3] {
        [LogKind::Actions, LogKind::Decisions, LogKind::Audit]
    }
}

/// A single log record. The fields are serialized as one JSON line on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogRecord {
    /// Schema tag — always `cutright.log_record/v1`.
    pub schema: String,
    /// Monotonic sequence number starting at 1.
    pub seq: u64,
    /// BLAKE3 hash of the previous record (32 zero bytes for seq 1).
    pub prev_hash: String,
    /// BLAKE3 hash of `prev_hash || seq || payload_json`.
    pub record_hash: String,
    /// Arbitrary JSON payload carrying the event-specific data.
    pub payload: serde_json::Value,
}

/// Errors raised by the log writer / reader / verifier.
#[derive(Debug, Error)]
pub enum LogError {
    /// I/O error during a log file operation.
    #[error("log I/O error at {path}: {source}")]
    Io {
        /// Path the operation was acting on.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// JSON serialization or deserialization of a record failed.
    #[error("could not encode log record: {0}")]
    Json(#[from] serde_json::Error),
    /// The on-disk log line could not be decoded into a `LogRecord`.
    #[error("malformed log line at {path}:{line_number}: {message}")]
    Malformed {
        /// Path of the log file.
        path: PathBuf,
        /// 1-based line number where the failure occurred.
        line_number: u64,
        /// Human-readable description.
        message: String,
    },
    /// The file flipped mode (e.g. an empty trailing line) in a way that
    /// breaks the parser.
    #[error("truncated log at {path}:{line_number}: expected more bytes than were available")]
    Truncated {
        /// Path of the log file.
        path: PathBuf,
        /// 1-based line number where the truncation was observed.
        line_number: u64,
    },
}

/// Result of a full verification pass over a log file.
#[derive(Debug, Clone, PartialEq)]
pub struct LogVerifierReport {
    /// Log kind the report is about.
    pub kind: LogKind,
    /// Number of records successfully verified.
    pub record_count: u64,
    /// Outcome of the verification pass.
    pub outcome: LogVerifierOutcome,
}

/// Outcome enum for a verification pass.
#[derive(Debug, Clone, PartialEq)]
pub enum LogVerifierOutcome {
    /// The log is structurally valid and hash-chained.
    Ok,
    /// A record's `record_hash` did not match its declared fields.
    HashMismatch {
        /// Sequence number of the broken record (1-based).
        seq: u64,
        /// Declared hash.
        declared: String,
        /// Re-derived hash.
        computed: String,
    },
    /// A record's `prev_hash` did not match the previous record's
    /// `record_hash`.
    ChainBroken {
        /// Sequence number of the broken record (1-based).
        seq: u64,
        /// Declared prev hash.
        declared_prev: String,
        /// Previous record's hash.
        expected_prev: String,
    },
    /// A record's `seq` was not exactly the previous seq + 1.
    OutOfOrder {
        /// Sequence number of the broken record.
        seq: u64,
        /// Expected next sequence number.
        expected: u64,
    },
    /// A record line could not be parsed.
    MalformedLine {
        /// Sequence number of the broken record (1-based).
        seq: u64,
        /// Underlying parse error.
        message: String,
    },
    /// The file ended mid-record.
    TruncatedTail {
        /// 1-based line number where the truncation was observed.
        line_number: u64,
    },
}

/// File locking helper. On unix we use `flock(2)`; this is a small, focused
/// inline binding so the log crate stays dependency-free.
#[cfg(unix)]
#[allow(unsafe_code)]
fn exclusive_lock(file: &fs::File, lock_path: &Path) -> Result<(), LogError> {
    extern "C" {
        fn flock(fd: i32, op: i32) -> i32;
    }
    use std::os::unix::io::AsRawFd;
    const LOCK_EX: i32 = 2;
    const LOCK_NB: i32 = 4;
    let result = unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) };
    if result != 0 {
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::WouldBlock {
            return Err(LogError::Io {
                path: lock_path.to_path_buf(),
                source: err,
            });
        }
        return Err(LogError::Io {
            path: lock_path.to_path_buf(),
            source: err,
        });
    }
    Ok(())
}

#[cfg(not(unix))]
fn exclusive_lock(_file: &fs::File, _lock_path: &Path) -> Result<(), LogError> {
    // Cross-process locking is not implemented on non-unix builds.
    Ok(())
}

/// Append-only log writer.
#[derive(Debug)]
pub struct LogWriter {
    project_root: PathBuf,
}

impl LogWriter {
    /// Build a writer scoped to `project_root`. The `.state/logs/` directory
    /// is created lazily on the first append.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    /// Resolve the on-disk path for `kind`.
    pub fn log_path(&self, kind: LogKind) -> PathBuf {
        self.project_root
            .join(STATE_DIR)
            .join(LOGS_SUBDIR)
            .join(kind.file_name())
    }

    /// Ensure the `.state/logs/` directory exists.
    fn ensure_logs_dir(&self) -> Result<PathBuf, LogError> {
        let dir = self.project_root.join(STATE_DIR).join(LOGS_SUBDIR);
        fs::create_dir_all(&dir).map_err(|source| LogError::Io {
            path: dir.clone(),
            source,
        })?;
        Ok(dir)
    }

    /// Read the last `(seq, record_hash)` from the file. Returns `(0,
    /// zero_hash)` for an empty or missing file.
    fn tail(&self, path: &Path) -> Result<(u64, String), LogError> {
        if !path.exists() {
            return Ok((0, zero_hash()));
        }
        let file = fs::File::open(path).map_err(|source| LogError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut reader = io::BufReader::new(file);
        let mut line = String::new();
        let mut last_seq: u64 = 0;
        let mut last_hash = zero_hash();
        let mut line_number: u64 = 0;
        loop {
            line.clear();
            let n = reader.read_line(&mut line).map_err(|source| LogError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            if n == 0 {
                break;
            }
            line_number += 1;
            let trimmed = line.trim_end_matches(['\n', '\r']);
            if trimmed.is_empty() {
                continue;
            }
            let record: LogRecord =
                serde_json::from_str(trimmed).map_err(|err| LogError::Malformed {
                    path: path.to_path_buf(),
                    line_number,
                    message: err.to_string(),
                })?;
            last_seq = record.seq;
            last_hash = record.record_hash;
        }
        Ok((last_seq, last_hash))
    }

    /// Append a single record to `kind`. The file is opened with an exclusive
    /// OS lock so concurrent appends cannot interleave partial lines.
    ///
    /// `payload` is any `Serialize` value. The chain hash is computed from
    /// the previous tail hash (or the zero hash, for the first record) and
    /// the JSON encoding of the payload. The seq is `prev_seq + 1`.
    pub fn append<T: Serialize>(&self, kind: LogKind, payload: &T) -> Result<LogRecord, LogError> {
        let _ = self.ensure_logs_dir()?;
        let path = self.log_path(kind);
        let (prev_seq, prev_hash) = self.tail(&path)?;
        let payload_value = serde_json::to_value(payload)?;
        let seq = prev_seq + 1;
        let record_hash = chain_hash(&prev_hash, seq, &payload_value);
        let record = LogRecord {
            schema: LOG_RECORD_SCHEMA.to_string(),
            seq,
            prev_hash,
            record_hash: record_hash.clone(),
            payload: payload_value,
        };
        let mut line = serde_json::to_string(&record)?;
        line.push('\n');
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)
            .map_err(|source| LogError::Io {
                path: path.clone(),
                source,
            })?;
        exclusive_lock(&file, &path)?;
        file.write_all(line.as_bytes())
            .map_err(|source| LogError::Io {
                path: path.clone(),
                source,
            })?;
        file.flush().map_err(|source| LogError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(record)
    }
}

/// Streaming reader over a log file.
#[derive(Debug)]
pub struct LogReader {
    project_root: PathBuf,
}

impl LogReader {
    /// Build a reader scoped to `project_root`.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    /// Path to the on-disk file for `kind`.
    pub fn log_path(&self, kind: LogKind) -> PathBuf {
        self.project_root
            .join(STATE_DIR)
            .join(LOGS_SUBDIR)
            .join(kind.file_name())
    }

    /// Read every record in `kind`. Returns an empty vector if the file does
    /// not exist.
    pub fn read_all(&self, kind: LogKind) -> Result<Vec<LogRecord>, LogError> {
        let path = self.log_path(kind);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let bytes = fs::read(&path).map_err(|source| LogError::Io {
            path: path.clone(),
            source,
        })?;
        let mut records = Vec::new();
        for (idx, line) in bytes.split(|b| *b == b'\n').enumerate() {
            if line.is_empty() {
                continue;
            }
            let record: LogRecord =
                serde_json::from_slice(line).map_err(|err| LogError::Malformed {
                    path: path.clone(),
                    line_number: (idx as u64) + 1,
                    message: err.to_string(),
                })?;
            records.push(record);
        }
        Ok(records)
    }
}

/// Hash-chain verifier.
#[derive(Debug)]
pub struct LogVerifier {
    project_root: PathBuf,
}

impl LogVerifier {
    /// Build a verifier scoped to `project_root`.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
        }
    }

    /// Verify the on-disk log for `kind`. Returns a `LogVerifierReport` whose
    /// `outcome` is `Ok` on success, or a precise failure variant otherwise.
    pub fn verify(&self, kind: LogKind) -> Result<LogVerifierReport, LogError> {
        let path = self
            .project_root
            .join(STATE_DIR)
            .join(LOGS_SUBDIR)
            .join(kind.file_name());
        if !path.exists() {
            return Ok(LogVerifierReport {
                kind,
                record_count: 0,
                outcome: LogVerifierOutcome::Ok,
            });
        }
        let bytes = fs::read(&path).map_err(|source| LogError::Io {
            path: path.clone(),
            source,
        })?;
        let mut expected_prev = zero_hash();
        let mut expected_seq: u64 = 1;
        let mut record_count: u64 = 0;
        for (idx, line) in bytes.split(|b| *b == b'\n').enumerate() {
            if line.is_empty() {
                continue;
            }
            let _line_number = (idx as u64) + 1;
            let record: LogRecord = match serde_json::from_slice(line) {
                Ok(r) => r,
                Err(err) => {
                    return Ok(LogVerifierReport {
                        kind,
                        record_count,
                        outcome: LogVerifierOutcome::MalformedLine {
                            seq: expected_seq,
                            message: err.to_string(),
                        },
                    });
                }
            };
            if record.seq != expected_seq {
                return Ok(LogVerifierReport {
                    kind,
                    record_count,
                    outcome: LogVerifierOutcome::OutOfOrder {
                        seq: record.seq,
                        expected: expected_seq,
                    },
                });
            }
            if record.prev_hash != expected_prev {
                return Ok(LogVerifierReport {
                    kind,
                    record_count,
                    outcome: LogVerifierOutcome::ChainBroken {
                        seq: record.seq,
                        declared_prev: record.prev_hash.clone(),
                        expected_prev: expected_prev.clone(),
                    },
                });
            }
            let computed = chain_hash(&record.prev_hash, record.seq, &record.payload);
            if computed != record.record_hash {
                return Ok(LogVerifierReport {
                    kind,
                    record_count,
                    outcome: LogVerifierOutcome::HashMismatch {
                        seq: record.seq,
                        declared: record.record_hash.clone(),
                        computed,
                    },
                });
            }
            expected_prev = record.record_hash.clone();
            expected_seq += 1;
            record_count += 1;
        }
        Ok(LogVerifierReport {
            kind,
            record_count,
            outcome: LogVerifierOutcome::Ok,
        })
    }

    /// Detect a truncated tail. The file ends with a non-terminated partial
    /// line (no trailing newline). Returns `LogVerifierOutcome::TruncatedTail`
    /// if the last byte is not `\n`, and `Ok` otherwise.
    pub fn detect_truncated_tail(&self, kind: LogKind) -> Result<LogVerifierReport, LogError> {
        let path = self
            .project_root
            .join(STATE_DIR)
            .join(LOGS_SUBDIR)
            .join(kind.file_name());
        if !path.exists() {
            return Ok(LogVerifierReport {
                kind,
                record_count: 0,
                outcome: LogVerifierOutcome::Ok,
            });
        }
        let mut file = fs::File::open(&path).map_err(|source| LogError::Io {
            path: path.clone(),
            source,
        })?;
        let len = file
            .metadata()
            .map_err(|source| LogError::Io {
                path: path.clone(),
                source,
            })?
            .len();
        if len == 0 {
            return Ok(LogVerifierReport {
                kind,
                record_count: 0,
                outcome: LogVerifierOutcome::Ok,
            });
        }
        file.seek(SeekFrom::End(-1))
            .map_err(|source| LogError::Io {
                path: path.clone(),
                source,
            })?;
        let mut buf = [0u8; 1];
        file.read_exact(&mut buf).map_err(|source| LogError::Io {
            path: path.clone(),
            source,
        })?;
        let line_number = byte_count_lines(&path).unwrap_or(0);
        if buf[0] != b'\n' {
            return Ok(LogVerifierReport {
                kind,
                record_count: 0,
                outcome: LogVerifierOutcome::TruncatedTail { line_number },
            });
        }
        Ok(LogVerifierReport {
            kind,
            record_count: 0,
            outcome: LogVerifierOutcome::Ok,
        })
    }
}

fn byte_count_lines(path: &Path) -> Result<u64, LogError> {
    let bytes = fs::read(path).map_err(|source| LogError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(bytes.iter().filter(|b| **b == b'\n').count() as u64)
}

fn zero_hash() -> String {
    blake3::hash(&[0u8; 32]).to_hex().to_string()
}

fn chain_hash(prev_hash: &str, seq: u64, payload: &serde_json::Value) -> String {
    let payload_bytes = serde_json::to_vec(payload).expect("payload serialization");
    let mut hasher = blake3::Hasher::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(&seq.to_le_bytes());
    hasher.update(&payload_bytes);
    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cutright-log-test-{nanos}-{counter}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn empty_log_verifies() {
        let dir = unique_dir();
        let verifier = LogVerifier::new(&dir);
        for kind in LogKind::all() {
            let report = verifier.verify(kind).expect("verify");
            assert_eq!(report.outcome, LogVerifierOutcome::Ok);
            assert_eq!(report.record_count, 0);
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_chains_records() {
        let dir = unique_dir();
        let writer = LogWriter::new(&dir);
        let reader = LogReader::new(&dir);
        let verifier = LogVerifier::new(&dir);

        let r1 = writer
            .append(LogKind::Actions, &serde_json::json!({"event": "a"}))
            .expect("append 1");
        let r2 = writer
            .append(LogKind::Actions, &serde_json::json!({"event": "b"}))
            .expect("append 2");
        let r3 = writer
            .append(LogKind::Actions, &serde_json::json!({"event": "c"}))
            .expect("append 3");

        assert_eq!(r1.seq, 1);
        assert_eq!(r2.seq, 2);
        assert_eq!(r3.seq, 3);
        assert_eq!(r2.prev_hash, r1.record_hash);
        assert_eq!(r3.prev_hash, r2.record_hash);

        let records = reader.read_all(LogKind::Actions).expect("read");
        assert_eq!(records.len(), 3);
        let report = verifier.verify(LogKind::Actions).expect("verify");
        assert_eq!(report.outcome, LogVerifierOutcome::Ok);
        assert_eq!(report.record_count, 3);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn tampered_record_is_detected() {
        let dir = unique_dir();
        let writer = LogWriter::new(&dir);
        let verifier = LogVerifier::new(&dir);

        writer
            .append(LogKind::Actions, &serde_json::json!({"event": "a"}))
            .expect("a");
        writer
            .append(LogKind::Actions, &serde_json::json!({"event": "b"}))
            .expect("b");
        writer
            .append(LogKind::Actions, &serde_json::json!({"event": "c"}))
            .expect("c");

        // Mutate the second record's payload in place.
        let path = writer.log_path(LogKind::Actions);
        let bytes = fs::read(&path).expect("read");
        let mut lines: Vec<Vec<u8>> = bytes
            .split(|b| *b == b'\n')
            .filter(|l| !l.is_empty())
            .map(|l| l.to_vec())
            .collect();
        let record: LogRecord = serde_json::from_slice(&lines[1]).expect("parse");
        let mut mutated = record.clone();
        mutated.payload = serde_json::json!({"event": "TAMPERED"});
        let mut mutated_bytes = serde_json::to_vec(&mutated).expect("re-serialize");
        mutated_bytes.push(b'\n');
        lines[1] = mutated_bytes;
        let mut new_bytes = Vec::new();
        for line in &lines {
            new_bytes.extend_from_slice(line);
            new_bytes.push(b'\n');
        }
        fs::write(&path, new_bytes).expect("write tampered");

        let report = verifier.verify(LogKind::Actions).expect("verify");
        assert!(matches!(
            report.outcome,
            LogVerifierOutcome::HashMismatch { .. }
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn truncated_log_is_detected() {
        let dir = unique_dir();
        let writer = LogWriter::new(&dir);
        let verifier = LogVerifier::new(&dir);

        writer
            .append(
                LogKind::Decisions,
                &serde_json::json!({"decision": "accept"}),
            )
            .expect("a");
        writer
            .append(
                LogKind::Decisions,
                &serde_json::json!({"decision": "reject"}),
            )
            .expect("b");

        let path = writer.log_path(LogKind::Decisions);
        let bytes = fs::read(&path).expect("read");
        // Truncate the trailing newline so the next read sees an unterminated
        // tail.
        let trimmed = &bytes[..bytes.len() - 1];
        fs::write(&path, trimmed).expect("write truncated");

        // The structural verifier still walks the closed records; the
        // truncated-tail detector picks up the missing newline.
        let report = verifier.verify(LogKind::Decisions).expect("verify");
        assert_eq!(report.outcome, LogVerifierOutcome::Ok);
        let tail = verifier
            .detect_truncated_tail(LogKind::Decisions)
            .expect("tail");
        assert!(matches!(
            tail.outcome,
            LogVerifierOutcome::TruncatedTail { .. }
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replayed_record_is_detected() {
        let dir = unique_dir();
        let writer = LogWriter::new(&dir);
        let verifier = LogVerifier::new(&dir);

        writer
            .append(LogKind::Audit, &serde_json::json!({"event": "first"}))
            .expect("first");
        writer
            .append(LogKind::Audit, &serde_json::json!({"event": "second"}))
            .expect("second");

        let path = writer.log_path(LogKind::Audit);
        let bytes = fs::read(&path).expect("read");
        let lines: Vec<&[u8]> = bytes
            .split(|b| *b == b'\n')
            .filter(|l| !l.is_empty())
            .collect();
        // Replay the first record as a third line. The seq field still says
        // 1, so the verifier must catch it as out-of-order.
        let mut replayed = lines[0].to_vec();
        replayed.push(b'\n');
        // The original file already ends with a newline; no extra newline is
        // needed. The replayed line still has seq=1, so when the verifier
        // expects seq=3 it will report OutOfOrder.
        let mut final_bytes = bytes.clone();
        final_bytes.extend_from_slice(&replayed);
        fs::write(&path, final_bytes).expect("write replayed");

        let report = verifier.verify(LogKind::Audit).expect("verify");
        assert!(matches!(
            report.outcome,
            LogVerifierOutcome::OutOfOrder { .. }
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn separate_log_kinds_do_not_share_a_chain() {
        let dir = unique_dir();
        let writer = LogWriter::new(&dir);
        let reader = LogReader::new(&dir);

        writer
            .append(LogKind::Actions, &serde_json::json!({"k": "a"}))
            .expect("a");
        writer
            .append(LogKind::Decisions, &serde_json::json!({"k": "d"}))
            .expect("d");
        let actions = reader.read_all(LogKind::Actions).expect("read");
        let decisions = reader.read_all(LogKind::Decisions).expect("read");
        assert_eq!(actions.len(), 1);
        assert_eq!(decisions.len(), 1);
        assert_ne!(actions[0].record_hash, decisions[0].record_hash);
        let _ = fs::remove_dir_all(&dir);
    }
}
