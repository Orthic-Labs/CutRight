use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

const SAMPLE_RATE: usize = 16_000;
const PREROLL_SAMPLES: usize = 4_000;
const CUT_GUARD_SAMPLES: usize = 5_120;

#[derive(Deserialize)]
struct SourceReport {
    rows: Vec<SourceRow>,
}

#[derive(Deserialize)]
struct SourceRow {
    clip: PathBuf,
    speech_segments: Vec<SpeechSegment>,
    #[serde(default)]
    segment_base_sample: Option<usize>,
    #[serde(default)]
    stream_samples: Option<usize>,
    #[serde(default)]
    onset_sample: Option<usize>,
    #[serde(default)]
    cut_sample_320ms_guard: Option<usize>,
    #[serde(default, alias = "keyword")]
    expected_keyword: Option<String>,
}

#[derive(Deserialize)]
struct SpeechSegment {
    start: usize,
    samples: usize,
}

#[derive(Serialize)]
struct ResultRow {
    clip: PathBuf,
    keyword: String,
    stream_base_sample: Option<usize>,
    keyword_start_s: Option<f32>,
    keyword_end_s: Option<f32>,
    onset_sample: Option<usize>,
    cut_sample_320ms_guard: Option<usize>,
    expected_onset_sample: Option<usize>,
    expected_cut_sample_320ms_guard: Option<usize>,
    expected_keyword: Option<String>,
    exact_expected_keyword: Option<bool>,
    exact_expected_cut: Option<bool>,
    cut_guard_is_exact: Option<bool>,
    error: Option<String>,
    call_ms: Vec<f64>,
}

#[derive(Serialize)]
struct Report {
    schema: u32,
    runtime: String,
    compute_units: String,
    cadence_ms: usize,
    manifest: PathBuf,
    clips: usize,
    detections: usize,
    rejected_results: usize,
    load_ms: f64,
    step_mean_ms: f64,
    step_p95_ms: f64,
    step_max_ms: f64,
    total_compute_ms: f64,
    wall_ms: f64,
    rows: Vec<ResultRow>,
}

fn read_wav(path: &Path) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|error| error.to_string())?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.sample_rate != SAMPLE_RATE as u32 {
        return Err(format!(
            "{}: expected 16 kHz mono, got {} Hz/{} ch",
            path.display(),
            spec.sample_rate,
            spec.channels
        ));
    }
    match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string()),
        hound::SampleFormat::Int => {
            let scale = ((1_i64 << (spec.bits_per_sample - 1)) - 1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|value| value as f32 / scale))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())
        }
    }
}

fn percentile(values: &[f64], quantile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() - 1) as f64 * quantile).ceil() as usize;
    sorted[index]
}

fn main() -> Result<(), String> {
    let usage =
        "usage: sherpa_kws_runtime_ab <model-dir> <source-report.json> <output.json> [cadence-ms]";
    let mut args = std::env::args_os().skip(1);
    let model_dir = args.next().map(PathBuf::from).ok_or(usage)?;
    let manifest = args.next().map(PathBuf::from).ok_or(usage)?;
    let output = args.next().map(PathBuf::from).ok_or(usage)?;
    let cadence_ms = args
        .next()
        .map(|value| value.to_string_lossy().parse::<usize>())
        .transpose()
        .map_err(|error| error.to_string())?
        .unwrap_or(100);
    if args.next().is_some() || cadence_ms == 0 {
        return Err(usage.into());
    }
    let chunk_samples = SAMPLE_RATE * cadence_ms / 1_000;
    if chunk_samples == 0 {
        return Err("cadence must produce at least one sample".into());
    }
    let source: SourceReport =
        serde_json::from_slice(&std::fs::read(&manifest).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    #[cfg(target_os = "windows")]
    let _ort = unsafe {
        libloading::Library::new(
            model_dir
                .parent()
                .ok_or("model directory has no resource parent")?
                .join("runtime/onnxruntime.dll"),
        )
    }
    .map_err(|error| format!("preload bundled ONNX Runtime: {error}"))?;
    let load_started = Instant::now();
    let mut kws = heardright_engine::sherpa_kws::SherpaKws::load(&model_dir)?;
    let _ = kws.transcribe_result(&vec![0.0; SAMPLE_RATE])?;
    kws.reset_stream()?;
    let load_ms = load_started.elapsed().as_secs_f64() * 1_000.0;
    let run_started = Instant::now();
    let mut all_call_ms = Vec::new();
    let mut rows = Vec::with_capacity(source.rows.len());

    for source_row in source.rows {
        let audio = read_wav(&source_row.clip)?;
        let mut keyword = String::new();
        let mut stream_base_sample = None;
        let mut keyword_start_s = None;
        let mut keyword_end_s = None;
        let mut onset_sample = None;
        let mut cut_sample_320ms_guard = None;
        let mut row_error = None;
        let mut row_call_ms = Vec::new();
        let stream_windows = match (source_row.segment_base_sample, source_row.stream_samples) {
            (Some(base), Some(samples)) => {
                let end = base.saturating_add(samples);
                if samples == 0 || end > audio.len() {
                    return Err(format!(
                        "{}: invalid manifest-owned KWS stream {base}..{end} for {} samples",
                        source_row.clip.display(),
                        audio.len()
                    ));
                }
                vec![(base, end)]
            }
            _ => source_row
                .speech_segments
                .iter()
                .map(|segment| {
                    let base = segment.start.saturating_sub(PREROLL_SAMPLES);
                    let end = audio
                        .len()
                        .min(segment.start.saturating_add(segment.samples));
                    (base, end)
                })
                .collect(),
        };
        for (base, end) in stream_windows {
            kws.reset_stream()?;
            let mut stream_audio = audio[base..end].to_vec();
            stream_audio.resize(stream_audio.len() + SAMPLE_RATE, 0.0);
            for chunk in stream_audio.chunks(chunk_samples) {
                let started = Instant::now();
                let result = match kws.transcribe_result(chunk) {
                    Ok(result) => result,
                    Err(error) => {
                        row_error = Some(error);
                        break;
                    }
                };
                let call_ms = started.elapsed().as_secs_f64() * 1_000.0;
                row_call_ms.push(call_ms);
                all_call_ms.push(call_ms);
                if !result.text.is_empty() {
                    keyword = result.text;
                    stream_base_sample = Some(base);
                    keyword_start_s = result.tokens.first().map(|token| token.start);
                    keyword_end_s = result.tokens.last().map(|token| token.end);
                    onset_sample = keyword_start_s
                        .map(|seconds| base + (seconds * SAMPLE_RATE as f32).round() as usize);
                    cut_sample_320ms_guard =
                        onset_sample.map(|sample| sample.saturating_sub(CUT_GUARD_SAMPLES));
                    break;
                }
            }
            if !keyword.is_empty() || row_error.is_some() {
                break;
            }
        }
        let expected_onset_sample = source_row.onset_sample;
        let expected_cut_sample_320ms_guard = source_row.cut_sample_320ms_guard;
        let expected_keyword = source_row
            .expected_keyword
            .map(|keyword| keyword.replace('_', " ").to_ascii_lowercase());
        let exact_expected_keyword = expected_keyword
            .as_deref()
            .map(|expected| expected == keyword);
        rows.push(ResultRow {
            clip: source_row.clip,
            keyword,
            stream_base_sample,
            keyword_start_s,
            keyword_end_s,
            onset_sample,
            cut_sample_320ms_guard,
            expected_onset_sample,
            expected_cut_sample_320ms_guard,
            exact_expected_keyword,
            expected_keyword,
            exact_expected_cut: expected_cut_sample_320ms_guard
                .zip(cut_sample_320ms_guard)
                .map(|(expected, actual)| expected == actual),
            cut_guard_is_exact: onset_sample
                .zip(cut_sample_320ms_guard)
                .map(|(onset, cut)| onset.saturating_sub(cut) == CUT_GUARD_SAMPLES),
            error: row_error,
            call_ms: row_call_ms,
        });
    }

    let report = Report {
        schema: 4,
        runtime: std::env::var("HR_SHERPA_LIBRARY_PATH")
            .unwrap_or_else(|_| "bundled-production".into()),
        compute_units: std::env::var("HR_SHERPA_COREML_CU").unwrap_or_else(|_| "onnx-cpu".into()),
        cadence_ms,
        manifest,
        clips: rows.len(),
        detections: rows.iter().filter(|row| !row.keyword.is_empty()).count(),
        rejected_results: rows.iter().filter(|row| row.error.is_some()).count(),
        load_ms,
        step_mean_ms: if all_call_ms.is_empty() {
            0.0
        } else {
            all_call_ms.iter().sum::<f64>() / all_call_ms.len() as f64
        },
        step_p95_ms: percentile(&all_call_ms, 0.95),
        step_max_ms: all_call_ms.iter().copied().fold(0.0, f64::max),
        total_compute_ms: all_call_ms.iter().sum(),
        wall_ms: run_started.elapsed().as_secs_f64() * 1_000.0,
        rows,
    };
    let encoded = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
    let temporary = output.with_extension(format!(
        "{}.tmp",
        output
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("json")
    ));
    std::fs::write(&temporary, [encoded, b"\n".to_vec()].concat())
        .map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, &output).map_err(|error| error.to_string())?;
    println!(
        "clips={} detections={} rejected={} exact_keywords={} exact_cuts={} load_ms={:.3} mean_ms={:.3} p95_ms={:.3} max_ms={:.3} total_ms={:.3}",
        report.clips,
        report.detections,
        report.rejected_results,
        report
            .rows
            .iter()
            .filter(|row| row.exact_expected_keyword == Some(true))
            .count(),
        report
            .rows
            .iter()
            .filter(|row| row.exact_expected_cut == Some(true))
            .count(),
        report.load_ms,
        report.step_mean_ms,
        report.step_p95_ms,
        report.step_max_ms,
        report.total_compute_ms,
    );
    Ok(())
}
