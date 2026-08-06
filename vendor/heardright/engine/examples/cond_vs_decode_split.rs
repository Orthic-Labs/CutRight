//! Measurement only — decomposes the app's reported "transcribe finished in N ms"
//! into its parts. `worker_sections/section03.rs` starts its timer at line 361,
//! BEFORE conditioning and the two pre-decode gates, then calls transcribe. The
//! shipping eval harness times transcribe alone, which is why the app looks ~3x
//! slower than the bench on the same audio.
//!
//! Usage: cond_vs_decode_split <model_dir> <wav_dir>

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: cond_vs_decode_split <model_dir> <wav_dir>");
        std::process::exit(2);
    }
    let model = heardright_engine::coreml_asr::CoreMlParakeet::load(std::path::Path::new(&args[1]))
        .expect("load model");
    let mut runtime = heardright_engine::asr::AsrRuntime::CoreMlParakeet(model);

    let mut wavs: Vec<_> = std::fs::read_dir(&args[2])
        .expect("read wav dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "wav").unwrap_or(false))
        .collect();
    wavs.sort();

    println!(
        "{:<16} {:>7} {:>9} {:>9} {:>9} {:>9}",
        "clip", "audio", "cond", "gates", "decode", "total"
    );
    for path in wavs {
        let mut reader = hound::WavReader::open(&path).expect("open wav");
        let samples: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| s.expect("sample") as f32 / 32768.0)
            .collect();
        let secs = samples.len() as f32 / 16_000.0;

        // Warm the model so the first clip does not absorb lazy-init cost.
        let _ = runtime.transcribe(&samples);

        let t0 = std::time::Instant::now();
        let conditioned =
            heardright_core::audio_conditioning::condition_for_asr(&samples, 16_000, "default");
        let cond_ms = t0.elapsed().as_secs_f64() * 1000.0;

        // The two pre-decode gates, exactly as section03.rs runs them.
        let t1 = std::time::Instant::now();
        let _global = heardright_core::audio_conditioning::rms(&conditioned);
        let n_frames = conditioned.len() / 160;
        let _voiced = conditioned
            .chunks(160)
            .filter(|f| heardright_core::audio_conditioning::rms(f) > 0.004)
            .count();
        let _ = n_frames;
        let gates_ms = t1.elapsed().as_secs_f64() * 1000.0;

        let t2 = std::time::Instant::now();
        let _ = runtime.transcribe(&conditioned);
        let decode_ms = t2.elapsed().as_secs_f64() * 1000.0;

        println!(
            "{:<16} {:>6.1}s {:>8.1}ms {:>8.1}ms {:>8.1}ms {:>8.1}ms",
            path.file_name().unwrap().to_string_lossy(),
            secs,
            cond_ms,
            gates_ms,
            decode_ms,
            cond_ms + gates_ms + decode_ms
        );
    }
}
