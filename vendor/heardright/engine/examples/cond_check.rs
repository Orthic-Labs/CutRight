//! Minimal raw-vs-conditioner ASR check (no sonora): for each wav, print RMS and
//! the Parakeet transcript of the RAW audio and the `condition_for_asr` output.
//!   HR_COREML_CU=cpuOnly cargo run --release --example cond_check -- \
//!     ../../model_registry/coreml/parakeet-tdt-v3  ~/hr-quiet-clips
#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn load(path: &std::path::Path) -> Vec<f32> {
    let mut r = hound::WavReader::open(path).unwrap();
    let s = r.spec();
    let ch = s.channels.max(1) as usize;
    let mut m = Vec::new();
    match s.sample_format {
        hound::SampleFormat::Int => {
            let max = (1i64 << (s.bits_per_sample - 1)) as f32;
            let v: Vec<i32> = r.samples::<i32>().filter_map(Result::ok).collect();
            for f in v.chunks(ch) {
                m.push(f.iter().map(|&x| x as f32 / max).sum::<f32>() / f.len() as f32);
            }
        }
        hound::SampleFormat::Float => {
            let v: Vec<f32> = r.samples::<f32>().filter_map(Result::ok).collect();
            for f in v.chunks(ch) {
                m.push(f.iter().sum::<f32>() / f.len() as f32);
            }
        }
    }
    m
}

#[cfg(target_os = "macos")]
fn main() {
    use heardright_core::audio_conditioning::{condition_for_asr, rms};
    let a: Vec<String> = std::env::args().collect();
    let model =
        heardright_engine::coreml_asr::CoreMlParakeet::load(std::path::Path::new(&a[1])).unwrap();
    let mut wavs: Vec<_> = std::fs::read_dir(&a[2])
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "wav").unwrap_or(false))
        .collect();
    wavs.sort();
    for w in &wavs {
        let raw = load(w);
        if raw.is_empty() {
            continue;
        }
        let def = condition_for_asr(&raw, 16_000, "default");
        let tx = |x: &[f32]| model.transcribe(x).unwrap_or_else(|e| format!("<err {e}>"));
        println!(
            "\n=== {} ({:.1}s) ===",
            w.file_name().unwrap().to_string_lossy(),
            raw.len() as f32 / 16_000.0
        );
        println!(
            "  rms raw={:.4} default={:.4} (gain {:.1}x)",
            rms(&raw),
            rms(&def),
            rms(&def) / rms(&raw).max(1e-9)
        );
        println!("  RAW    : {}", tx(&raw));
        println!("  DEFAULT: {}", tx(&def));
    }
}
