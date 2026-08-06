//! Our SHIPPED int4 Parakeet TDT v3 (CoreML) on a dir of clips — to diff against
//! stock bf16 Parakeet (parakeet-mlx) and isolate quantization damage vs inherent error.
//!   HR_COREML_CU=cpuOnly cargo run --release --example parakeet_eval -- \
//!     /Users/adrdsouza/claude/heardright/model_registry/coreml/parakeet-tdt-v3  ~/hr-bias-clips
#[cfg(not(target_os = "macos"))]
fn main() {}
#[cfg(target_os = "macos")]
fn load_wav(path: &std::path::Path) -> Vec<f32> {
    let mut r = hound::WavReader::open(path).unwrap();
    let s = r.spec();
    let ch = s.channels.max(1) as usize;
    let mut m = Vec::new();
    match s.sample_format {
        hound::SampleFormat::Int => {
            let mx = (1i64 << (s.bits_per_sample - 1)) as f32;
            let v: Vec<i32> = r.samples::<i32>().filter_map(Result::ok).collect();
            for f in v.chunks(ch) {
                m.push(f.iter().map(|&x| x as f32 / mx).sum::<f32>() / f.len() as f32);
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
    use heardright_engine::coreml_asr::CoreMlParakeet;
    use std::path::PathBuf;
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("usage: parakeet_eval <bundle> <clips_dir>");
        std::process::exit(2);
    }
    let model = CoreMlParakeet::load(std::path::Path::new(&a[1])).unwrap();
    let mut wavs: Vec<PathBuf> = std::fs::read_dir(&a[2])
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "wav").unwrap_or(false))
        .collect();
    wavs.sort();
    if let Some(w) = wavs.first() {
        let a = load_wav(w);
        if !a.is_empty() {
            let _ = model.transcribe(&a);
        }
    } // warm
    let (mut tot_audio, mut tot_dec) = (0f64, 0f64);
    for w in &wavs {
        let audio = load_wav(w);
        if audio.is_empty() {
            continue;
        }
        let secs = audio.len() as f64 / 16000.0;
        // MANDATORY per the ASR frontend lock: train==serve. The shipping path
        // (worker_sections/section03.rs) runs `condition_for_asr` before the
        // model; this example did not, so it was scoring RAW audio and silently
        // under-reporting quality. Env-switchable so the effect is measurable.
        let audio = if std::env::var_os("HR_EVAL_RAW_AUDIO").is_some() {
            audio
        } else {
            let policy =
                std::env::var("HR_AUDIO_CONDITIONING_POLICY").unwrap_or_else(|_| "default".into());
            heardright_core::audio_conditioning::condition_for_asr(&audio, 16_000, &policy)
        };
        let t0 = std::time::Instant::now();
        let t = model
            .transcribe(&audio)
            .unwrap_or_else(|e| format!("<err {e}>"));
        let dt = t0.elapsed().as_secs_f64();
        tot_audio += secs;
        tot_dec += dt;
        println!(
            "\n{} [{:.1}s audio, {:.0}ms decode, RTF {:.3}]:\n  {t}",
            w.file_name().unwrap().to_string_lossy(),
            secs,
            dt * 1000.0,
            dt / secs
        );
    }
    println!(
        "\n=== TOTAL: {:.1}s audio, {:.2}s decode, RTF {:.3} ===",
        tot_audio,
        tot_dec,
        tot_dec / tot_audio
    );
}
