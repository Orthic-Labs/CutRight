//! Beam/lattice gate test: dump the Parakeet greedy 1-best + the top-K acoustic
//! alternatives per emitted token, and check whether the *correct* word is even
//! present in the alternatives. If it is, beam + LM-rescoring can recover it; if
//! not, no post-decode method can — only a learned L2 (world knowledge) or
//! decode-time biasing could. This is the cheap experiment that decides whether
//! building a full beam is worth it.
//!
//!   HR_COREML_CU=cpuOnly cargo run --release --example beam_eval -- \
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
    use heardright_engine::coreml_asr::CoreMlParakeet;
    use std::path::PathBuf;
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("usage: beam_eval <bundle> <clips_dir>");
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
    // the content words Parakeet got wrong on these clips
    let targets = [
        "invoice",
        "logo",
        "footer",
        "whisper",
        "kerning",
        "squarespace",
        "socials",
    ];
    let k = 4usize;
    for w in &wavs {
        let audio = load_wav(w);
        if audio.is_empty() {
            continue;
        }
        let (greedy, rows) = match model.transcribe_topk(&audio, k) {
            Ok(x) => x,
            Err(e) => {
                println!("{}: <err {e}>", w.display());
                continue;
            }
        };
        println!("\n=== {} ===", w.file_name().unwrap().to_string_lossy());
        println!("GREEDY: {greedy}");
        // alternatives that DIFFER from the chosen (the interesting ones)
        let mut interesting = 0;
        for (i, alts) in rows.iter().enumerate() {
            if alts.len() > 1 && alts[1].trim() != alts[0].trim() {
                println!("  pos[{i}] {} | alts: {}", alts[0], alts[1..].join(" "));
                interesting += 1;
                if interesting > 40 {
                    println!("  …(truncated)");
                    break;
                }
            }
        }
        // gate check: is any target word reachable from the alternatives?
        let flat: String = rows
            .iter()
            .flatten()
            .map(|p| p.replace('▁', " "))
            .collect::<Vec<_>>()
            .join("")
            .to_lowercase();
        for t in targets {
            if flat.contains(t) {
                println!(
                    "  >>> GATE: '{t}' IS PRESENT in acoustic alternatives (beam could recover it)"
                );
            }
        }
    }
}
