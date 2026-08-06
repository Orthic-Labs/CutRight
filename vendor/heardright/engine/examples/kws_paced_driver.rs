//! Production-cadence KWS load generator for concurrent ASR benchmarks.
//!
//! Usage: kws_paced_driver <model-dir> <wav> <duration-seconds> [cadence-ms]

use heardright_engine::sherpa_kws::SherpaKws;
use std::path::Path;
use std::time::{Duration, Instant};

const SAMPLE_RATE: usize = 16_000;

fn read_wav(path: &Path) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|error| error.to_string())?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.sample_rate != SAMPLE_RATE as u32 {
        return Err(format!("expected mono 16 kHz WAV: {}", path.display()));
    }
    reader
        .samples::<i16>()
        .map(|sample| {
            sample
                .map(|value| value as f32 / 32_768.0)
                .map_err(|error| error.to_string())
        })
        .collect()
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 || args.len() > 5 {
        return Err(
            "usage: kws_paced_driver <model-dir> <wav> <duration-seconds> [cadence-ms]".into(),
        );
    }
    let duration = Duration::from_secs(args[3].parse::<u64>().map_err(|error| error.to_string())?);
    let cadence = Duration::from_millis(
        args.get(4)
            .map(String::as_str)
            .unwrap_or("100")
            .parse::<u64>()
            .map_err(|error| error.to_string())?,
    );
    let chunk_samples = SAMPLE_RATE * cadence.as_millis() as usize / 1_000;
    if chunk_samples == 0 {
        return Err("cadence must produce at least one sample".into());
    }

    let audio = read_wav(Path::new(&args[2]))?;
    if audio.is_empty() {
        return Err("input WAV is empty".into());
    }
    let mut kws = SherpaKws::load(Path::new(&args[1]))?;
    let started = Instant::now();
    let mut next_tick = started;
    let mut offset = 0usize;
    let mut calls = 0usize;
    let mut detections = 0usize;
    let mut compute = Duration::ZERO;
    let mut call_ms = Vec::new();

    while started.elapsed() < duration {
        let mut chunk = Vec::with_capacity(chunk_samples);
        while chunk.len() < chunk_samples {
            let take = (chunk_samples - chunk.len()).min(audio.len() - offset);
            chunk.extend_from_slice(&audio[offset..offset + take]);
            offset = (offset + take) % audio.len();
        }
        let call_started = Instant::now();
        if !kws.transcribe_result(&chunk)?.text.is_empty() {
            detections += 1;
            kws.reset_stream()?;
        }
        let elapsed = call_started.elapsed();
        compute += elapsed;
        call_ms.push(elapsed.as_secs_f64() * 1_000.0);
        calls += 1;
        next_tick += cadence;
        if let Some(remaining) = next_tick.checked_duration_since(Instant::now()) {
            std::thread::sleep(remaining);
        }
    }

    call_ms.sort_by(f64::total_cmp);
    let p95_index = ((call_ms.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(call_ms.len().saturating_sub(1));
    let p95_ms = call_ms.get(p95_index).copied().unwrap_or(0.0);
    let max_ms = call_ms.last().copied().unwrap_or(0.0);
    println!(
        "calls={calls} detections={detections} compute_ms={:.3} duty_pct={:.3} p95_ms={p95_ms:.3} max_ms={max_ms:.3}",
        compute.as_secs_f64() * 1_000.0,
        compute.as_secs_f64() / started.elapsed().as_secs_f64() * 100.0,
    );
    Ok(())
}
