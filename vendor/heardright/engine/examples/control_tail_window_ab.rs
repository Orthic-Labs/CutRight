//! Compare rolling control-tail window lengths through the shipped ASR runtime.
//!
//! Usage: control_tail_window_ab <models_dir> <wav> [<wav>...]

use std::path::Path;
use std::time::Instant;

use heardright_engine::asr::{AsrEp, AsrRuntime};

const SAMPLE_RATE: usize = 16_000;

fn load_wav(path: &Path) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|error| error.to_string())?;
    let spec = reader.spec();
    if spec.sample_rate != SAMPLE_RATE as u32 || spec.channels != 1 {
        return Err(format!(
            "{}: expected 16 kHz mono WAV, got {} Hz/{} channels",
            path.display(),
            spec.sample_rate,
            spec.channels
        ));
    }
    match spec.sample_format {
        hound::SampleFormat::Int => {
            let scale = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| value as f32 / scale)
                        .map_err(|error| error.to_string())
                })
                .collect()
        }
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| sample.map_err(|error| error.to_string()))
            .collect(),
    }
}

fn transcribe_window(
    runtime: &mut AsrRuntime,
    audio: &[f32],
    window_ms: usize,
) -> Result<(String, u128), String> {
    let window_samples = window_ms * SAMPLE_RATE / 1_000;
    let start = audio.len().saturating_sub(window_samples);
    let conditioned = heardright_core::audio_conditioning::condition_for_asr(
        &audio[start..],
        SAMPLE_RATE as u32,
        "default",
    );
    let started = Instant::now();
    let text = runtime.transcribe(&conditioned)?;
    Ok((text, started.elapsed().as_millis()))
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        return Err("usage: control_tail_window_ab <models_dir> <wav> [<wav>...]".into());
    }
    let mut runtime = AsrRuntime::load(Path::new(&args[0]), AsrEp::resolve_default())?;
    let _ = runtime.transcribe(&vec![0.0; SAMPLE_RATE]);

    for wav in &args[1..] {
        let audio = load_wav(Path::new(wav))?;
        let (short, short_ms) = transcribe_window(&mut runtime, &audio, 1_750)?;
        let (full, full_ms) = transcribe_window(&mut runtime, &audio, 3_000)?;
        println!(
            "{}\n  1750ms {:>4}ms: {:?}\n  3000ms {:>4}ms: {:?}",
            wav, short_ms, short, full_ms, full
        );
    }
    Ok(())
}
