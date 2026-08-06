// Native CoreML Parakeet engine for Apple Silicon (the ANE).
//
// Runs the validated 4-stage static-shape `.mlmodelc` / `.mlpackage` bundle
// (MelSpectrogram · AudioEncoder · TextDecoder · Joint) directly on
// CoreML.framework via `objc2-core-ml` — in-process, no Python, no subprocess.
// The `ort` CoreML EP cannot run these graphs (empty output on the static-shape
// Parakeet conformer); this drives `MLModel`/`MLMultiArray` itself.
//
// This started as a 1:1 Rust port of the proven Python harness
// `scripts/coreml/pico_eval.py::Model.transcribe` (ground-truth 2.24% WER for
// `tdt_w15_p6`, summarized in docs/ARCHITECTURE.md). The
// load-bearing details that MUST match the Python:
//   * per-window decoder-state RESET (carrying LSTM state across a hard window
//     boundary makes TDT emit all-blank dead windows — 57% WER bug);
//   * length clamp `ceil(real_samples / 160 / 8)` encoder frames (don't decode
//     the static-window silence padding, or the decoder hallucinates a tail);
//   * TDT dur==0 ⇒ emit another token at the same frame.
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use objc2::rc::autoreleasepool;
use ring::digest::{Context, SHA256};

// Generic CoreML.framework plumbing (stage load, MLMultiArray IO, stride-aware
// read, predict) lives in the shared `coreml` module — reused by whisper_coreml.
use crate::coreml::{ml_f16, ml_i32, read_f16, Stage};

/// Engine config read from the bundle's `pipeline.json`.
#[derive(Debug, Clone)]
pub struct PipelineMeta {
    pub is_tdt: bool,
    pub n_mel: usize,
    pub window_sec: f32,
    pub mel_frames: usize,
    pub enc_frames: usize,
    pub pred_layers: usize,
    pub pred_hidden: usize,
    pub blank_id: i32,
    pub durations: Vec<i64>,
}

impl PipelineMeta {
    fn from_dir(dir: &Path) -> Result<Self, String> {
        let raw = std::fs::read_to_string(dir.join("pipeline.json"))
            .map_err(|e| format!("read pipeline.json: {e}"))?;
        let v: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| format!("parse pipeline.json: {e}"))?;
        let durations = v["tdt_durations"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x.as_i64()).collect::<Vec<_>>())
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| vec![0, 1, 2, 3, 4]);
        Ok(Self {
            is_tdt: v["is_tdt"].as_bool().unwrap_or(true),
            n_mel: v["n_mel"].as_u64().unwrap_or(128) as usize,
            window_sec: v["window_sec"].as_f64().unwrap_or(15.0) as f32,
            mel_frames: v["mel_frames"].as_u64().unwrap_or(1500) as usize,
            enc_frames: v["enc_frames"].as_u64().unwrap_or(187) as usize,
            pred_layers: v["pred_layers"].as_u64().unwrap_or(2) as usize,
            pred_hidden: v["pred_hidden"].as_u64().unwrap_or(640) as usize,
            blank_id: v["blank_id"].as_i64().unwrap_or(8192) as i32,
            durations,
        })
    }
}

/// The loaded engine: 4 stages + vocab + resolved IO names.
/// A word with its start/end time in seconds, for SRT/VTT subtitle output.
#[derive(Clone, Debug)]
pub struct TimedTok {
    pub text: String,
    pub start: f32,
    pub end: f32,
}

/// A decoded display-token piece with its emit time in seconds.
#[derive(Clone, Debug)]
pub struct TimedPiece {
    pub text: String,
    pub start: f32,
    /// Real TDT-predicted duration in seconds. `0.0` means this piece shares a
    /// frame with the piece that follows it; the last piece of that chain
    /// carries the chain's duration.
    pub duration: f32,
}

pub struct CoreMlParakeet {
    meta: PipelineMeta,
    mel: Stage,
    enc: Stage,
    dec: Stage,
    joint: Stage,
    mel_out: String,
    enc_out: String,
    g_name: String,
    state_h_out: String,
    state_c_out: String,
    joint_out: String,
    pieces: Vec<String>,
    win_samples: usize,
    // Contextual biasing (mirrors the ONNX path's ContextBias) — boosts the
    // logit of the token that continues an in-progress match of a bias phrase
    // (dictionary terms + command words). `None` = greedy as before.
    bias: Option<ContextBias>,
    // Beam width for `decode_window` (see `decode_window.rs`). `1` (the
    // default) is the original greedy decode, bit-identical to the pre-beam
    // implementation. Set via `set_beam_width` or the `HR_ASR_BEAM_WIDTH` env
    // override read once at `load()` time.
    beam_width: usize,
    model_fingerprint: String,
    model_dir_sha256: Option<String>,
    coreml_compile_cache: String,
    encoder_compute: String,
    decoder_compute: String,
}

/// Contextual biasing for the greedy decode. 1:1 port of the ONNX path's
/// `parakeet_rs::ContextBias` so Mac and Windows bias identically. Phrases are
/// stored as token-id sequences (encoded against this bundle's vocab).
#[derive(Clone)]
struct ContextBias {
    phrase_token_ids: Vec<Vec<usize>>,
    score: f32,
}

impl ContextBias {
    fn is_empty(&self) -> bool {
        self.phrase_token_ids.is_empty() || self.score == 0.0
    }

    /// For each phrase, if the emitted-token suffix matches a prefix of the
    /// phrase, the next expected token earns `score` (max across phrases).
    fn next_token_bonuses(&self, emitted: &[usize]) -> std::collections::HashMap<usize, f32> {
        let mut bonuses = std::collections::HashMap::new();
        if self.is_empty() {
            return bonuses;
        }
        for phrase in &self.phrase_token_ids {
            let matched = matching_prefix_len(emitted, phrase);
            if matched < phrase.len() {
                bonuses
                    .entry(phrase[matched])
                    .and_modify(|s: &mut f32| *s = s.max(self.score))
                    .or_insert(self.score);
            }
        }
        bonuses
    }
}

fn matching_prefix_len(emitted: &[usize], phrase: &[usize]) -> usize {
    let max = emitted.len().min(phrase.len().saturating_sub(1));
    for len in (0..=max).rev() {
        if len == 0 || emitted[emitted.len() - len..] == phrase[..len] {
            return len;
        }
    }
    0
}

/// Lowercase, keep alphanumerics + apostrophe, collapse whitespace — identical
/// to the ONNX vocab's `normalize_phrase_for_bias` so encodings match.
fn normalize_phrase_for_bias(phrase: &str) -> String {
    phrase
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '\'' {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn stage_path(dir: &Path, stem: &str) -> PathBuf {
    let compiled = dir.join(format!("{stem}.mlmodelc"));
    if compiled.exists() {
        compiled
    } else {
        dir.join(format!("{stem}.mlpackage"))
    }
}

const MODEL_FINGERPRINT_SAMPLE_BYTES: usize = 4_096;
const FNV1A64_OFFSET: u64 = 0xcbf29ce484222325;
const FNV1A64_PRIME: u64 = 0x100000001b3;

fn fnv1a64_update(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV1A64_PRIME);
    }
}

fn collect_model_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|error| format!("read model dir: {error}"))? {
        let path = entry
            .map_err(|error| format!("read model entry: {error}"))?
            .path();
        if path.is_dir() {
            collect_model_files(root, &path, files)?;
        } else if path.is_file() {
            path.strip_prefix(root)
                .map_err(|error| format!("model fingerprint path: {error}"))?;
            files.push(path);
        }
    }
    Ok(())
}

/// Fast, deterministic model identity for diagnostics. Hashes every relative
/// path, file length, and first/last 4 KiB; it never reads transcript/user data.
fn sampled_model_fingerprint(dir: &Path) -> Result<(String, u64, usize), String> {
    let mut files = Vec::new();
    collect_model_files(dir, dir, &mut files)?;
    files.sort();
    let mut hash = FNV1A64_OFFSET;
    let mut total_bytes = 0u64;
    let mut sample = vec![0u8; MODEL_FINGERPRINT_SAMPLE_BYTES];
    for path in &files {
        let relative = path
            .strip_prefix(dir)
            .map_err(|error| format!("model fingerprint path: {error}"))?;
        fnv1a64_update(&mut hash, relative.to_string_lossy().as_bytes());
        let mut file =
            std::fs::File::open(path).map_err(|error| format!("open model file: {error}"))?;
        let len = file
            .metadata()
            .map_err(|error| format!("stat model file: {error}"))?
            .len();
        total_bytes = total_bytes.saturating_add(len);
        fnv1a64_update(&mut hash, &len.to_le_bytes());

        let first_len = usize::try_from(len.min(MODEL_FINGERPRINT_SAMPLE_BYTES as u64))
            .unwrap_or(MODEL_FINGERPRINT_SAMPLE_BYTES);
        file.read_exact(&mut sample[..first_len])
            .map_err(|error| format!("read model file head: {error}"))?;
        fnv1a64_update(&mut hash, &sample[..first_len]);

        if len > MODEL_FINGERPRINT_SAMPLE_BYTES as u64 {
            let tail_len = usize::try_from(len.min(MODEL_FINGERPRINT_SAMPLE_BYTES as u64))
                .unwrap_or(MODEL_FINGERPRINT_SAMPLE_BYTES);
            file.seek(SeekFrom::End(-(tail_len as i64)))
                .map_err(|error| format!("seek model file tail: {error}"))?;
            file.read_exact(&mut sample[..tail_len])
                .map_err(|error| format!("read model file tail: {error}"))?;
            fnv1a64_update(&mut hash, &sample[..tail_len]);
        }
    }
    Ok((format!("{hash:016x}"), total_bytes, files.len()))
}

/// Full deterministic SHA-256 over every relative model path & byte.
fn model_dir_sha256(dir: &Path) -> Result<String, String> {
    static CACHE: OnceLock<Mutex<Option<(PathBuf, String)>>> = OnceLock::new();
    let cache_key = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    if let Some((_, hash)) = CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
        .filter(|(path, _)| path == &cache_key)
    {
        return Ok(hash.clone());
    }
    let mut files = Vec::new();
    collect_model_files(dir, dir, &mut files)?;
    files.sort();
    let mut digest = Context::new(&SHA256);
    let mut buffer = [0u8; 64 * 1024];
    for path in files {
        let relative = path
            .strip_prefix(dir)
            .map_err(|error| format!("model hash path: {error}"))?;
        digest.update(relative.to_string_lossy().as_bytes());
        digest.update(&[0]);
        let mut file = std::fs::File::open(&path)
            .map_err(|error| format!("model hash open {}: {error}", path.display()))?;
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|error| format!("model hash read {}: {error}", path.display()))?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
        }
    }
    let hash = digest
        .finish()
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    *CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some((cache_key, hash.clone()));
    Ok(hash)
}
