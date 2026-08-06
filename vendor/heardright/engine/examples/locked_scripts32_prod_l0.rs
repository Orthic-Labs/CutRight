use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use heardright_engine::asr::{AsrEp, AsrRuntime};
use heardright_engine::text_polish::{self, DictationPolishContext};
use serde::{Deserialize, Serialize};

const DEFAULT_MANIFEST: &str = r"D:\Claude\heardright\tauri-app-next\.cache\asr-eval\20260626-size-regate\sound_gt32_manifest_v2.json";
const DEFAULT_MODELS_ROOT: &str = r"D:\Claude\heardright\model_registry\final\asr";
const DEFAULT_OUT_DIR: &str =
    r"D:\Claude\heardright\tauri-app-next\.cache\asr-eval\20260627-locked-scripts32-prod-l0";
const DEFAULT_WHISPER_EXE: &str =
    r"D:\Claude\heardright\.wcpp_vulkan\whisper.cpp\build_cl\bin\Release\whisper-cli.exe";
const DEFAULT_UNIFIED_PRODUCT_JSON: &str = r"D:\Claude\heardright\tauri-app-next\.cache\asr-eval\20260627-unified-sw-style-bakeoff\unified_padded_window_vs_buffer_allclips_20260627.json";
const DEFAULT_TDT_HARD_JSON: &str =
    r"D:\Claude\heardright\tauri-app-next\.cache\asr-eval\20260627-tdt15-nochime\static15_b64.json";
const DEFAULT_TDT_SW_JSONL: &str = r"D:\Claude\heardright\tauri-app-next\.cache\asr-eval\20260627-sw-silence-static15\sw_silence_static15_gt15_full.jsonl";

#[derive(Debug, Deserialize)]
struct ManifestItem {
    clip: String,
    group: String,
    path: String,
    gt: String,
}

#[derive(Debug, Serialize)]
struct ModelSummary {
    name: String,
    artifact: String,
    clips: usize,
    ref_words: usize,
    errors: usize,
    wer: f64,
    mean_ms: f64,
    p95_ms: f64,
    by_group: BTreeMap<String, GroupSummary>,
}

#[derive(Debug, Clone, Serialize)]
struct GroupSummary {
    clips: usize,
    ref_words: usize,
    errors: usize,
    wer: f64,
    mean_ms: f64,
}

#[derive(Debug, Serialize)]
struct ClipRow {
    model: String,
    clip: String,
    group: String,
    duration_s: f64,
    ref_words: usize,
    errors: usize,
    wer: f64,
    decode_ms: f64,
    raw_hyp: String,
    l0_hyp: String,
    l0_ref: String,
}

#[derive(Debug, Serialize)]
struct Report {
    generated: String,
    manifest: String,
    scoring: String,
    summaries: Vec<ModelSummary>,
    rows: Vec<ClipRow>,
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let manifest = flag_value(&args, "--manifest").unwrap_or_else(|| DEFAULT_MANIFEST.into());
    let models_root = PathBuf::from(
        flag_value(&args, "--models-root").unwrap_or_else(|| DEFAULT_MODELS_ROOT.into()),
    );
    let out_dir =
        PathBuf::from(flag_value(&args, "--out-dir").unwrap_or_else(|| DEFAULT_OUT_DIR.into()));
    let whisper_exe = PathBuf::from(
        flag_value(&args, "--whisper-exe").unwrap_or_else(|| DEFAULT_WHISPER_EXE.into()),
    );
    let limit = flag_value(&args, "--limit").and_then(|v| v.parse::<usize>().ok());
    let only = flag_value(&args, "--only");
    let cache_parakeet = args.iter().any(|a| a == "--cache-parakeet");

    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let manifest_path = PathBuf::from(&manifest);
    let mut items: Vec<ManifestItem> =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if let Some(n) = limit {
        items.truncate(n);
    }

    let mut rows = Vec::new();
    if cache_parakeet {
        rows.extend(run_cached_parakeet(&items, only.as_deref())?);
    } else if only
        .as_deref()
        .map(|s| s == "unified" || s == "all")
        .unwrap_or(true)
    {
        rows.extend(run_parakeet_model(
            "unified_locked_padded_l0",
            "parakeet-unified",
            &models_root,
            &items,
        )?);
    }
    if !cache_parakeet
        && only
            .as_deref()
            .map(|s| s == "tdt" || s == "all")
            .unwrap_or(true)
    {
        rows.extend(run_parakeet_model(
            "tdt_locked_padded_l0",
            "parakeet-tdt",
            &models_root,
            &items,
        )?);
    }
    if only
        .as_deref()
        .map(|s| s == "whisper" || s == "all")
        .unwrap_or(true)
    {
        rows.extend(run_whisper_model(
            "whisper_locked_l0",
            &models_root.join("whisper_turbo_q5_k.bin"),
            &whisper_exe,
            &items,
        )?);
    }

    let summaries = summarize(&rows, &models_root);
    let report = Report {
        generated: chrono::Utc::now().to_rfc3339(),
        manifest,
        scoring: "Production-shaped local L0 on both reference script and hypothesis; Parakeet via AsrRuntime padded-window, Whisper via locked whisper.cpp Vulkan artifact.".into(),
        summaries,
        rows,
    };

    let json_path = out_dir.join("locked_scripts32_prod_l0_sweep_20260627.json");
    std::fs::write(
        &json_path,
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    write_markdown(
        &out_dir.join("locked_scripts32_prod_l0_sweep_20260627.md"),
        &report,
    )?;
    println!("wrote {}", json_path.display());
    Ok(())
}

fn run_cached_parakeet(items: &[ManifestItem], only: Option<&str>) -> Result<Vec<ClipRow>, String> {
    let mut rows = Vec::new();
    if only.map(|s| s == "unified" || s == "all").unwrap_or(true) {
        rows.extend(run_cached_unified(items)?);
    }
    if only.map(|s| s == "tdt" || s == "all").unwrap_or(true) {
        rows.extend(run_cached_tdt(items)?);
    }
    Ok(rows)
}

#[derive(Debug, Deserialize)]
struct UnifiedProduct {
    rows: Vec<UnifiedProductRow>,
}

#[derive(Debug, Deserialize)]
struct UnifiedProductRow {
    clip: String,
    padded_window_text: String,
    padded_window_lat_ms: f64,
}

fn run_cached_unified(items: &[ManifestItem]) -> Result<Vec<ClipRow>, String> {
    let path = PathBuf::from(DEFAULT_UNIFIED_PRODUCT_JSON);
    let data: UnifiedProduct =
        serde_json::from_str(&std::fs::read_to_string(&path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let by_idx: BTreeMap<usize, UnifiedProductRow> = data
        .rows
        .into_iter()
        .filter_map(|r| scripts_md_index(&r.clip).map(|idx| (idx, r)))
        .collect();
    let mut rows = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        let source_idx = idx + 1;
        let cached = by_idx
            .get(&source_idx)
            .ok_or_else(|| format!("missing unified scripts_md_{source_idx:03}"))?;
        let (_, duration_s) = load_wav(Path::new(&item.path))?;
        rows.push(score_row(
            "unified_locked_padded_l0",
            item,
            duration_s,
            cached.padded_window_text.clone(),
            cached.padded_window_lat_ms,
        ));
    }
    Ok(rows)
}

#[derive(Debug, Deserialize)]
struct TdtEvalRun {
    rows: Vec<TdtHardRow>,
}

#[derive(Debug, Deserialize)]
struct TdtHardRow {
    clip: String,
    duration_s: f64,
    decode_ms: f64,
    text: String,
}

#[derive(Debug, Deserialize)]
struct TdtSwClipRow {
    clip: String,
    committed_text: Option<String>,
    final_ms: Option<f64>,
}

fn run_cached_tdt(items: &[ManifestItem]) -> Result<Vec<ClipRow>, String> {
    let hard_path = PathBuf::from(DEFAULT_TDT_HARD_JSON);
    let hard_runs: Vec<TdtEvalRun> =
        serde_json::from_str(&std::fs::read_to_string(&hard_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let hard_rows = hard_runs
        .into_iter()
        .next()
        .ok_or_else(|| "missing TDT hard run".to_string())?
        .rows;
    let hard_by_idx: BTreeMap<usize, TdtHardRow> = hard_rows
        .into_iter()
        .filter_map(|r| scripts_md_index(&r.clip).map(|idx| (idx, r)))
        .collect();

    let mut sw_by_idx = BTreeMap::new();
    let sw_path = PathBuf::from(DEFAULT_TDT_SW_JSONL);
    for line in std::fs::read_to_string(&sw_path)
        .map_err(|e| e.to_string())?
        .lines()
    {
        let value: serde_json::Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
        if value.get("kind").and_then(|v| v.as_str()) == Some("clip") {
            let row: TdtSwClipRow = serde_json::from_value(value).map_err(|e| e.to_string())?;
            if let Some(idx) = scripts_md_index(&row.clip) {
                sw_by_idx.insert(idx, row);
            }
        }
    }

    let mut rows = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        let source_idx = idx + 1;
        let hard = hard_by_idx
            .get(&source_idx)
            .ok_or_else(|| format!("missing tdt scripts_md_{source_idx:03}"))?;
        let (raw, ms, duration_s) = if hard.duration_s <= 15.0 {
            (hard.text.clone(), hard.decode_ms, hard.duration_s)
        } else {
            let sw = sw_by_idx
                .get(&source_idx)
                .ok_or_else(|| format!("missing tdt sw scripts_md_{source_idx:03}"))?;
            (
                sw.committed_text.clone().unwrap_or_default(),
                sw.final_ms.unwrap_or(hard.decode_ms),
                hard.duration_s,
            )
        };
        rows.push(score_row("tdt_locked_padded_l0", item, duration_s, raw, ms));
    }
    Ok(rows)
}

fn scripts_md_index(clip: &str) -> Option<usize> {
    let stem = clip.strip_prefix("scripts_md_")?;
    let digits: String = stem.chars().take_while(|ch| ch.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn run_parakeet_model(
    name: &str,
    backend: &str,
    models_root: &Path,
    items: &[ManifestItem],
) -> Result<Vec<ClipRow>, String> {
    println!("loading {name} backend={backend}");
    std::env::set_var("HR_ASR_BACKEND", backend);
    let ep = AsrEp::resolve_default();
    let mut model = AsrRuntime::load(models_root, ep)?;
    if let Some(first) = items.first() {
        let (samples, _) = load_wav(Path::new(&first.path))?;
        let _ = model.transcribe_result(&samples);
    }

    let mut rows = Vec::new();
    for item in items {
        let (samples, duration_s) = load_wav(Path::new(&item.path))?;
        let t0 = Instant::now();
        let raw_hyp = model.transcribe_result(&samples)?.text;
        let decode_ms = t0.elapsed().as_secs_f64() * 1000.0;
        rows.push(score_row(name, item, duration_s, raw_hyp, decode_ms));
        println!("{name} {} {:.0}ms", item.clip, decode_ms);
    }
    Ok(rows)
}

fn run_whisper_model(
    name: &str,
    model_path: &Path,
    exe: &Path,
    items: &[ManifestItem],
) -> Result<Vec<ClipRow>, String> {
    println!("running {name}");
    if !exe.exists() {
        return Err(format!("missing whisper exe: {}", exe.display()));
    }
    if !model_path.exists() {
        return Err(format!("missing whisper model: {}", model_path.display()));
    }
    if let Some(first) = items.first() {
        let _ = whisper_transcribe(exe, model_path, Path::new(&first.path));
    }

    let mut rows = Vec::new();
    for item in items {
        let (_, duration_s) = load_wav(Path::new(&item.path))?;
        let t0 = Instant::now();
        let raw_hyp = whisper_transcribe(exe, model_path, Path::new(&item.path))?;
        let decode_ms = t0.elapsed().as_secs_f64() * 1000.0;
        rows.push(score_row(name, item, duration_s, raw_hyp, decode_ms));
        println!("{name} {} {:.0}ms", item.clip, decode_ms);
    }
    Ok(rows)
}

fn score_row(
    model: &str,
    item: &ManifestItem,
    duration_s: f64,
    raw_hyp: String,
    decode_ms: f64,
) -> ClipRow {
    let audio_secs = Some(duration_s as f32);
    let l0_ref = production_l0(&item.gt, audio_secs);
    let l0_hyp = production_l0(&raw_hyp, audio_secs);
    let ref_words = wer_words(&l0_ref);
    let hyp_words = wer_words(&l0_hyp);
    let errors = edit_distance(&ref_words, &hyp_words);
    let wer = if ref_words.is_empty() {
        0.0
    } else {
        errors as f64 / ref_words.len() as f64
    };
    ClipRow {
        model: model.into(),
        clip: item.clip.clone(),
        group: item.group.clone(),
        duration_s,
        ref_words: ref_words.len(),
        errors,
        wer,
        decode_ms,
        raw_hyp,
        l0_hyp,
        l0_ref,
    }
}

fn production_l0(input: &str, audio_secs: Option<f32>) -> String {
    let mut transcript = input.trim().to_string();
    if let Some(command) = heardright_core::text_pipeline::parse_control_command(&transcript) {
        transcript = command.clean_text;
    }
    let ai_transform = if let Some(command) =
        heardright_core::text_pipeline::parse_ai_transform_command(&transcript)
    {
        transcript = command.clean_text;
        Some(command.intent)
    } else {
        None
    };
    let polished = if ai_transform.is_some() {
        text_polish::polish_local_only(&transcript)
    } else {
        text_polish::polish_dictation(
            &transcript,
            DictationPolishContext {
                audio_secs,
                app_name: None,
                window_title: None,
                field_text: None,
                selected_text: None,
                field_context_available: false,
                writing_region: None,
            },
        )
    };
    let snippets = heardright_engine::settings::snippets();
    if snippets.is_empty() {
        polished
    } else {
        heardright_core::text_pipeline::expand_snippets(&polished, &snippets)
    }
}

fn load_wav(path: &Path) -> Result<(Vec<f32>, f64), String> {
    let mut reader =
        hound::WavReader::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let spec = reader.spec();
    let channels = spec.channels.max(1) as usize;
    let rate = spec.sample_rate.max(1) as f64;
    let mut mono = Vec::new();
    match spec.sample_format {
        hound::SampleFormat::Float => {
            let vals: Vec<f32> = reader.samples::<f32>().filter_map(Result::ok).collect();
            for frame in vals.chunks(channels) {
                mono.push(frame.iter().sum::<f32>() / frame.len() as f32);
            }
        }
        hound::SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample.saturating_sub(1))) as f32;
            let vals: Vec<i32> = reader.samples::<i32>().filter_map(Result::ok).collect();
            for frame in vals.chunks(channels) {
                mono.push(frame.iter().map(|&v| v as f32 / max).sum::<f32>() / frame.len() as f32);
            }
        }
    }
    let duration_s = mono.len() as f64 / rate;
    Ok((mono, duration_s))
}

fn whisper_transcribe(exe: &Path, model: &Path, wav: &Path) -> Result<String, String> {
    let out = Command::new(exe)
        .args([
            "-m",
            &model.to_string_lossy(),
            "-f",
            &wav.to_string_lossy(),
            "-l",
            "en",
            "-bs",
            "1",
            "-bo",
            "1",
        ])
        .output()
        .map_err(|e| format!("whisper launch: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut pieces = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('[') || !trimmed.contains("-->") {
            continue;
        }
        if let Some(end) = trimmed.find(']') {
            pieces.push(trimmed[end + 1..].trim().to_string());
        }
    }
    Ok(pieces.join(" ").trim().to_string())
}

fn wer_words(text: &str) -> Vec<String> {
    let mut out = String::new();
    for ch in text.chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() || ch == '\'' {
            out.push(ch);
        } else {
            out.push(' ');
        }
    }
    out.split_whitespace().map(|s| s.to_string()).collect()
}

fn edit_distance(a: &[String], b: &[String]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0; b.len() + 1];
    for (i, aw) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, bw) in b.iter().enumerate() {
            let cost = if aw == bw { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

fn summarize(rows: &[ClipRow], models_root: &Path) -> Vec<ModelSummary> {
    let mut out = Vec::new();
    let mut by_model: BTreeMap<&str, Vec<&ClipRow>> = BTreeMap::new();
    for row in rows {
        by_model.entry(&row.model).or_default().push(row);
    }
    for (model, rows) in by_model {
        let mut lat: Vec<f64> = rows.iter().map(|r| r.decode_ms).collect();
        lat.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let ref_words: usize = rows.iter().map(|r| r.ref_words).sum();
        let errors: usize = rows.iter().map(|r| r.errors).sum();
        let mut group_map: BTreeMap<String, Vec<&ClipRow>> = BTreeMap::new();
        for row in &rows {
            group_map.entry(row.group.clone()).or_default().push(row);
        }
        let by_group = group_map
            .into_iter()
            .map(|(group, rs)| {
                let ref_words: usize = rs.iter().map(|r| r.ref_words).sum();
                let errors: usize = rs.iter().map(|r| r.errors).sum();
                let mean_ms = rs.iter().map(|r| r.decode_ms).sum::<f64>() / rs.len() as f64;
                (
                    group,
                    GroupSummary {
                        clips: rs.len(),
                        ref_words,
                        errors,
                        wer: pct(errors, ref_words),
                        mean_ms,
                    },
                )
            })
            .collect();
        out.push(ModelSummary {
            name: model.to_string(),
            artifact: artifact_for(model, models_root),
            clips: rows.len(),
            ref_words,
            errors,
            wer: pct(errors, ref_words),
            mean_ms: rows.iter().map(|r| r.decode_ms).sum::<f64>() / rows.len() as f64,
            p95_ms: percentile(&lat, 0.95),
            by_group,
        });
    }
    out
}

fn pct(errors: usize, ref_words: usize) -> f64 {
    if ref_words == 0 {
        0.0
    } else {
        errors as f64 * 100.0 / ref_words as f64
    }
}

fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn artifact_for(model: &str, root: &Path) -> String {
    match model {
        "unified_locked_padded_l0" => root
            .join("unified_static15_b128_sym_bits4_timestamp_hybrid")
            .display()
            .to_string(),
        "tdt_locked_padded_l0" => root
            .join("parakeet_tdt_v3_b64_sym_bits4_protect")
            .display()
            .to_string(),
        "whisper_locked_l0" => root.join("whisper_turbo_q5_k.bin").display().to_string(),
        _ => root.display().to_string(),
    }
}

fn write_markdown(path: &Path, report: &Report) -> Result<(), String> {
    let mut md = String::new();
    md.push_str("# Locked ASR 32-Clip Script Sweep: Padding + Production L0\n\n");
    md.push_str(&format!(
        "Date: 2026-06-27\n\nManifest: `{}`\n\n",
        report.manifest
    ));
    md.push_str("Scoring: each model hypothesis and the script reference are passed through the same production-shaped local L0 sequence before WER: control tail parse, AI transform tail parse, deterministic local polish/cleanup, snippets. No cloud/L3 summary or prompt rewrite is applied.\n\n");
    md.push_str("Runtime lanes:\n\n");
    md.push_str("- Unified: locked `unified_static15_b128_sym_bits4_timestamp_hybrid`, DirectML/CPU hybrid through `AsrRuntime::transcribe_result`, including the production 15s padded-window quiet-cut policy.\n");
    md.push_str("- TDT: locked `parakeet_tdt_v3_b64_sym_bits4_protect`, DirectML/CPU hybrid through the same production padded-window path.\n");
    md.push_str("- Whisper: locked `whisper_turbo_q5_k.bin` through `whisper-cli.exe` Vulkan flags `-bs 1 -bo 1`, then the same production L0 scorer.\n\n");
    md.push_str("## Summary\n\n");
    md.push_str("| model | WER | errors/ref | mean ms | p95 ms | artifact |\n");
    md.push_str("|---|---:|---:|---:|---:|---|\n");
    for s in &report.summaries {
        md.push_str(&format!(
            "| {} | {:.2}% | {}/{} | {:.1} | {:.1} | `{}` |\n",
            s.name, s.wer, s.errors, s.ref_words, s.mean_ms, s.p95_ms, s.artifact
        ));
    }
    md.push_str("\n## By Group\n\n");
    md.push_str("| model | group | clips | WER | errors/ref | mean ms |\n");
    md.push_str("|---|---|---:|---:|---:|---:|\n");
    for s in &report.summaries {
        for (group, g) in &s.by_group {
            md.push_str(&format!(
                "| {} | {} | {} | {:.2}% | {}/{} | {:.1} |\n",
                s.name, group, g.clips, g.wer, g.errors, g.ref_words, g.mean_ms
            ));
        }
    }
    md.push_str("\n## Clip Table\n\n");
    md.push_str(
        "| clip | group | dur | ref words | unified WER/ms | TDT WER/ms | Whisper WER/ms |\n",
    );
    md.push_str("|---|---|---:|---:|---:|---:|---:|\n");
    let mut clips: BTreeMap<&str, Vec<&ClipRow>> = BTreeMap::new();
    for row in &report.rows {
        clips.entry(&row.clip).or_default().push(row);
    }
    for (clip, rows) in clips {
        let first = rows[0];
        let mut cells: BTreeMap<&str, String> = BTreeMap::new();
        for row in rows {
            cells.insert(
                row.model.as_str(),
                format!("{:.2}% / {:.0}", row.wer * 100.0, row.decode_ms),
            );
        }
        md.push_str(&format!(
            "| `{}` | {} | {:.1}s | {} | {} | {} | {} |\n",
            clip,
            first.group,
            first.duration_s,
            first.ref_words,
            cells
                .get("unified_locked_padded_l0")
                .map(String::as_str)
                .unwrap_or("-"),
            cells
                .get("tdt_locked_padded_l0")
                .map(String::as_str)
                .unwrap_or("-"),
            cells
                .get("whisper_locked_l0")
                .map(String::as_str)
                .unwrap_or("-")
        ));
    }
    md.push_str("\n## Full Rows\n\nFull raw hypotheses, L0 hypotheses, and L0 references are in `locked_scripts32_prod_l0_sweep_20260627.json` beside this report.\n");
    std::fs::write(path, md).map_err(|e| e.to_string())
}

fn flag_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone())
}
