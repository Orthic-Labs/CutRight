// Lane D smoke: exercise `WhisperWin::load` + `transcribe_lang` through the
// real engine code path (not just the raw CLI), since `cargo test` is
// currently blocked crate-wide by a pre-existing, out-of-scope macOS-only
// test-helper gate (see l3_cleanup_sections/tests/section05*.rs). Windows only.
//
// Usage: cargo run --example whisper_win_smoke -- <models_dir> <wav_path> <lang>
#[cfg(target_os = "windows")]
fn main() {
    let mut args = std::env::args().skip(1);
    let models_dir = args.next().expect("usage: <models_dir> <wav_path> <lang>");
    let wav_path = args.next().expect("usage: <models_dir> <wav_path> <lang>");
    let lang = args.next().unwrap_or_else(|| "en".to_string());

    let engine =
        heardright_engine::whisper_win::WhisperWin::load(std::path::Path::new(&models_dir))
            .expect("load WhisperWin engine");

    let mut reader = hound::WavReader::open(&wav_path).expect("open wav");
    let spec = reader.spec();
    println!("wav spec: {spec:?}");
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / i16::MAX as f32)
            .collect(),
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap()).collect(),
    };

    let started = std::time::Instant::now();
    let text = engine.transcribe_lang(&samples, &lang).expect("transcribe");
    println!("latency_ms={}", started.elapsed().as_millis());
    println!("text={text:?}");
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("whisper_win_smoke is Windows-only");
}
