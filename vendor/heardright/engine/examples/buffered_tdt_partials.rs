use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use heardright_engine::file_transcribe;
use parakeet_rs::{ExecutionConfig, ExecutionProvider, HrTransducer, TimedToken};
use serde_json::json;

const SAMPLE_RATE: usize = 16_000;

#[derive(Debug, Clone, Copy)]
struct BufferedConfig {
    chunk_samples: usize,
    left_samples: usize,
    right_samples: usize,
    hold_words: usize,
    max_windows: Option<usize>,
}

#[derive(Debug, Clone)]
struct CommittedWindow {
    text: String,
    tokens: usize,
}

#[derive(Debug, Clone)]
struct StableCommitter {
    stable: String,
    mutable_tail: String,
    hold_words: usize,
}

impl StableCommitter {
    fn new(hold_words: usize) -> Self {
        Self {
            stable: String::new(),
            mutable_tail: String::new(),
            hold_words,
        }
    }

    fn push(&mut self, chunk: &str) -> String {
        append_with_overlap(&mut self.mutable_tail, chunk);
        let (new_stable, tail) = split_stable_tail(&self.mutable_tail, self.hold_words);
        append_with_overlap(&mut self.stable, &new_stable);
        self.mutable_tail = tail;
        self.stable_text().to_string()
    }

    fn finish(mut self) -> String {
        append_with_overlap(&mut self.stable, &self.mutable_tail);
        self.stable.trim().to_string()
    }

    fn stable_text(&self) -> &str {
        self.stable.trim()
    }

    fn display_text(&self) -> String {
        let mut display = self.stable.clone();
        append_with_overlap(&mut display, &self.mutable_tail);
        display.trim().to_string()
    }
}

fn main() -> Result<(), String> {
    configure_ort_dylib();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        return Err(
            "usage: buffered_tdt_partials <model_dir> <clip.wav|clip_dir> [more clips/dirs...]"
                .to_string(),
        );
    }
    let model_dir = PathBuf::from(&args[1]);
    let mut clips = Vec::new();
    for arg in &args[2..] {
        expand_input(Path::new(arg), &mut clips)?;
    }
    clips.sort();
    if clips.is_empty() {
        return Err("no audio clips found".to_string());
    }

    let config = BufferedConfig {
        chunk_samples: ms_to_samples(env_usize("HEARDRIGHT_BUFFERED_CHUNK_MS", 560).max(80)),
        left_samples: ms_to_samples(env_usize("HEARDRIGHT_BUFFERED_LEFT_MS", 2_800)),
        right_samples: ms_to_samples(env_usize("HEARDRIGHT_BUFFERED_RIGHT_MS", 560)),
        hold_words: env_usize("HEARDRIGHT_BUFFERED_HOLD_WORDS", 3).clamp(0, 12),
        max_windows: std::env::var("HEARDRIGHT_BUFFERED_MAX_WINDOWS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0),
    };
    let audio_policy =
        std::env::var("HR_AUDIO_CONDITIONING_POLICY").unwrap_or_else(|_| "default".into());

    eprintln!(
        "buffered: loading {} on {:?}; chunk={}ms left={}ms right={}ms hold_words={} clips={}",
        model_dir.display(),
        provider_from_env(),
        samples_to_ms(config.chunk_samples),
        samples_to_ms(config.left_samples),
        samples_to_ms(config.right_samples),
        config.hold_words,
        clips.len()
    );
    let load_started = Instant::now();
    let mut model = HrTransducer::load(
        &model_dir,
        Some(ExecutionConfig::new().with_execution_provider(provider_from_env())),
    )
    .map_err(|e| format!("model load {}: {e}", model_dir.display()))?;
    eprintln!(
        "buffered: model loaded in {} ms",
        load_started.elapsed().as_millis()
    );

    let mut all_window_ms = Vec::new();
    for clip in clips {
        let raw = file_transcribe::decode_to_16k_mono(&clip)?;
        if raw.is_empty() {
            continue;
        }
        let audio = heardright_core::audio_conditioning::condition_for_asr(
            &raw,
            SAMPLE_RATE as u32,
            &audio_policy,
        );

        let final_started = Instant::now();
        let final_result = model
            .transcribe_result(&audio)
            .map_err(|e| format!("final decode {}: {e}", clip.display()))?;
        let final_ms = final_started.elapsed().as_millis() as u64;
        let final_text = final_result.text.trim().to_string();

        let mut committer = StableCommitter::new(config.hold_words);
        let mut clip_window_ms = Vec::new();
        let mut window_index = 0usize;
        let mut chunk_start = 0usize;
        while chunk_start < audio.len() {
            if config
                .max_windows
                .map(|max| window_index >= max)
                .unwrap_or(false)
            {
                break;
            }
            let stable_end = (chunk_start + config.chunk_samples).min(audio.len());
            let window_start = chunk_start.saturating_sub(config.left_samples);
            let window_end =
                (chunk_start + config.chunk_samples + config.right_samples).min(audio.len());
            if window_end <= window_start {
                break;
            }

            let decode_started = Instant::now();
            let result = model
                .transcribe_result(&audio[window_start..window_end])
                .map_err(|e| format!("window decode {}: {e}", clip.display()))?;
            let decode_ms = decode_started.elapsed().as_millis() as u64;
            clip_window_ms.push(decode_ms);
            all_window_ms.push(decode_ms);

            let committed_window = commit_tokens_in_range(
                &result.tokens,
                sample_to_secs(window_start),
                sample_to_secs(chunk_start),
                sample_to_secs(stable_end),
            );
            let stable_text = committer.push(&committed_window.text);
            println!(
                "{}",
                json!({
                    "kind": "window",
                    "clip": file_name(&clip),
                    "window_index": window_index,
                    "decode_ms": decode_ms,
                    "window_start_s": sample_to_secs(window_start),
                    "window_end_s": sample_to_secs(window_end),
                    "stable_start_s": sample_to_secs(chunk_start),
                    "stable_end_s": sample_to_secs(stable_end),
                    "window_words": result.text.split_whitespace().count(),
                    "committed_tokens": committed_window.tokens,
                    "committed_text": committed_window.text.trim(),
                    "stable_text": stable_text,
                    "mutable_tail": committer.mutable_tail.trim(),
                    "display_text": committer.display_text(),
                    "window_text": result.text,
                })
            );

            window_index += 1;
            chunk_start += config.chunk_samples;
        }

        let committed_text = committer.finish();
        println!(
            "{}",
            json!({
                "kind": "clip",
                "clip": file_name(&clip),
                "secs": sample_to_secs(audio.len()),
                "final_ms": final_ms,
                "windows": window_index,
                "avg_window_ms": avg(&clip_window_ms),
                "p95_window_ms": percentile(clip_window_ms.clone(), 95.0),
                "word_overlap_vs_final": word_overlap(&committed_text, &final_text),
                "committed_words": committed_text.split_whitespace().count(),
                "final_words": final_text.split_whitespace().count(),
                "committed_text": committed_text,
                "final_text": final_text,
            })
        );
    }

    println!(
        "{}",
        json!({
            "kind": "summary",
            "windows": all_window_ms.len(),
            "avg_window_ms": avg(&all_window_ms),
            "p50_window_ms": percentile(all_window_ms.clone(), 50.0),
            "p95_window_ms": percentile(all_window_ms.clone(), 95.0),
            "max_window_ms": all_window_ms.iter().copied().max().unwrap_or(0),
        })
    );
    Ok(())
}

fn configure_ort_dylib() {
    #[cfg(not(target_os = "windows"))]
    {
        return;
    }

    #[cfg(target_os = "windows")]
    {
        if std::env::var("ORT_DYLIB_PATH")
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        {
            return;
        }

        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.to_path_buf()));
        let candidates = [
            exe_dir.as_ref().map(|dir| dir.join("onnxruntime.dll")),
            exe_dir
                .as_ref()
                .map(|dir| dir.join("resources/runtime/onnxruntime.dll")),
            Some(
                manifest_dir.join("../src-tauri/target/release/resources/runtime/onnxruntime.dll"),
            ),
            Some(manifest_dir.join("../src-tauri/target/debug/resources/runtime/onnxruntime.dll")),
            Some(manifest_dir.join("../src-tauri/resources/runtime/onnxruntime.dll")),
            Some(
                manifest_dir.join(
                    "../../.venv-build-dml/Lib/site-packages/onnxruntime/capi/onnxruntime.dll",
                ),
            ),
        ];

        if let Some(dll) = candidates.into_iter().flatten().find(|path| path.exists()) {
            std::env::set_var("ORT_DYLIB_PATH", &dll);
            eprintln!("buffered: ORT_DYLIB_PATH={}", dll.display());
        } else {
            eprintln!(
                "buffered: ORT_DYLIB_PATH unresolved; ORT may fall back to System32 onnxruntime.dll"
            );
        }
    }
}

fn commit_tokens_in_range(
    tokens: &[TimedToken],
    window_offset_s: f32,
    stable_start_s: f32,
    stable_end_s: f32,
) -> CommittedWindow {
    let mut text = String::new();
    let mut count = 0usize;
    for token in tokens {
        let start_s = window_offset_s + token.start;
        if start_s >= stable_start_s && start_s < stable_end_s {
            text.push_str(&token.text);
            count += 1;
        }
    }
    CommittedWindow {
        text,
        tokens: count,
    }
}

fn append_with_overlap(buffer: &mut String, next: &str) {
    if next.is_empty() {
        return;
    }
    let overlap = suffix_prefix_overlap(buffer, next, 16);
    buffer.push_str(&next[overlap..]);
}

fn split_stable_tail(input: &str, hold_words: usize) -> (String, String) {
    if hold_words == 0 {
        return (input.to_string(), String::new());
    }
    let ranges = word_ranges(input);
    if ranges.len() <= hold_words {
        return (String::new(), input.to_string());
    }
    let split_at = ranges[ranges.len() - hold_words].0;
    (input[..split_at].to_string(), input[split_at..].to_string())
}

fn word_ranges(input: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = None;
    for (idx, ch) in input.char_indices() {
        if ch.is_whitespace() {
            if let Some(s) = start.take() {
                ranges.push((s, idx));
            }
        } else if start.is_none() {
            start = Some(idx);
        }
    }
    if let Some(s) = start {
        ranges.push((s, input.len()));
    }
    ranges
}

fn suffix_prefix_overlap(a: &str, b: &str, max_chars: usize) -> usize {
    let max = a.len().min(b.len()).min(max_chars);
    let mut best = 0usize;
    for len in 1..=max {
        if !a.is_char_boundary(a.len() - len) || !b.is_char_boundary(len) {
            continue;
        }
        let suffix = &a[a.len() - len..];
        let prefix = &b[..len];
        if suffix.eq_ignore_ascii_case(prefix) {
            best = len;
        }
    }
    best
}

fn provider_from_env() -> ExecutionProvider {
    let value = std::env::var("HEARDRIGHT_BUFFERED_EP")
        .ok()
        .or_else(|| std::env::var("HR_ASR_EP").ok())
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if value == "cpu" {
        return ExecutionProvider::Cpu;
    }
    #[cfg(target_os = "windows")]
    {
        if value.is_empty() || matches!(value.as_str(), "dml" | "directml") {
            return ExecutionProvider::DirectML;
        }
    }
    #[cfg(target_os = "macos")]
    {
        if matches!(value.as_str(), "coreml" | "ane") {
            return ExecutionProvider::CoreML;
        }
    }
    ExecutionProvider::Cpu
}

fn expand_input(input: &Path, clips: &mut Vec<PathBuf>) -> Result<(), String> {
    if input.is_dir() {
        for entry in
            std::fs::read_dir(input).map_err(|e| format!("read {}: {e}", input.display()))?
        {
            let path = entry.map_err(|e| e.to_string())?.path();
            if is_audio_file(&path) {
                clips.push(path);
            }
        }
    } else if is_audio_file(input) {
        clips.push(input.to_path_buf());
    }
    Ok(())
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "wav" | "m4a" | "mp3"))
        .unwrap_or(false)
}

fn word_overlap(a: &str, b: &str) -> f32 {
    let a_words: HashSet<String> = words(a).collect();
    let b_words: HashSet<String> = words(b).collect();
    if a_words.is_empty() {
        return 0.0;
    }
    a_words.intersection(&b_words).count() as f32 / a_words.len() as f32
}

fn words(input: &str) -> impl Iterator<Item = String> + '_ {
    input
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|c: char| !c.is_alphanumeric())
                .to_ascii_lowercase()
        })
        .filter(|word| !word.is_empty())
}

fn avg(values: &[u64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<u64>() as f64 / values.len() as f64
}

fn percentile(mut values: Vec<u64>, pct: f64) -> u64 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    let idx = ((values.len() - 1) as f64 * pct / 100.0).ceil() as usize;
    values[idx.min(values.len() - 1)]
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

fn ms_to_samples(ms: usize) -> usize {
    SAMPLE_RATE * ms / 1_000
}

fn samples_to_ms(samples: usize) -> usize {
    samples * 1_000 / SAMPLE_RATE
}

fn sample_to_secs(sample: usize) -> f32 {
    sample as f32 / SAMPLE_RATE as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(text: &str, start: f32) -> TimedToken {
        TimedToken {
            text: text.to_string(),
            start,
            end: start + 0.01,
        }
    }

    #[test]
    fn commit_tokens_uses_absolute_stable_range() {
        let tokens = vec![
            token(" before", 0.05),
            token(" keep", 0.20),
            token(" also", 0.54),
            token(" after", 0.57),
        ];
        let committed = commit_tokens_in_range(&tokens, 1.0, 1.20, 1.56);
        assert_eq!(committed.text, " keep also");
        assert_eq!(committed.tokens, 2);
    }

    #[test]
    fn word_overlap_is_based_on_committed_words() {
        assert_eq!(word_overlap("hello world", "hello brave world"), 1.0);
        assert_eq!(word_overlap("", "hello"), 0.0);
    }

    #[test]
    fn append_with_overlap_removes_boundary_repeats() {
        let mut text = "photo descrip".to_string();
        append_with_overlap(&mut text, "ription should");
        assert_eq!(text, "photo description should");

        append_with_overlap(&mut text, " start");
        append_with_overlap(&mut text, "art now");
        assert_eq!(text, "photo description should start now");
    }

    #[test]
    fn stable_committer_keeps_tail_mutable() {
        let mut c = StableCommitter::new(2);
        c.push("The one click drive photo descrip");
        assert_eq!(c.stable_text(), "The one click drive");
        assert_eq!(c.mutable_tail.trim(), "photo descrip");
        c.push("ription should link");
        assert_eq!(
            c.finish(),
            "The one click drive photo description should link"
        );
    }
}
