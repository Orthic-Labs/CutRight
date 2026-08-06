//! Run the locked Whisper-multi CoreML model on a dir of clips — to compare a
//! DIFFERENT decoder (encoder-decoder + implicit LM) against Parakeet TDT on the
//! exact content mishears. Does Whisper get invoice/logo/footer right at the source?
//!   HR_COREML_CU=cpuOnly cargo run --release --example whisper_eval -- \
//!     /Users/adrdsouza/claude/heardright/model_registry/coreml/whisper-multi  ~/hr-bias-clips
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
    use heardright_engine::whisper_coreml::WhisperCoreMl;
    use serde::Deserialize;
    use std::path::PathBuf;
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("usage: whisper_eval <whisper-multi-dir> <clips_or_pico_eval_dir> [out.json]");
        std::process::exit(2);
    }
    eprintln!(
        "loading whisper-multi (HR_COREML_CU={}) …",
        std::env::var("HR_COREML_CU").unwrap_or_else(|_| "default(ANE,~6min first compile)".into())
    );
    let model = WhisperCoreMl::load(std::path::Path::new(&a[1])).unwrap_or_else(|e| {
        eprintln!("load failed: {e}");
        std::process::exit(1);
    });
    let input_dir = PathBuf::from(&a[2]);
    if input_dir.join("eval_set.json").exists() {
        #[derive(Deserialize)]
        struct Item {
            wav: String,
            gt: String,
            corpus: String,
            dur_s: f64,
        }
        let items: Vec<Item> = serde_json::from_str(
            &std::fs::read_to_string(input_dir.join("eval_set.json")).unwrap(),
        )
        .unwrap();
        if let Some(first) = items.first() {
            let audio = load_wav(&input_dir.join(&first.wav));
            if !audio.is_empty() {
                let _ = model.transcribe(&audio);
            }
        }
        let started = std::time::Instant::now();
        let mut refs = Vec::new();
        let mut hyps = Vec::new();
        let mut rows = Vec::new();
        let mut total_audio = 0.0;
        for item in &items {
            let audio = load_wav(&input_dir.join(&item.wav));
            let t0 = std::time::Instant::now();
            let hyp = model
                .transcribe(&audio)
                .unwrap_or_else(|e| format!("<err {e}>"));
            let decode_s = t0.elapsed().as_secs_f64();
            let gt = norm(&item.gt);
            let hyp_n = norm(&hyp);
            refs.push(gt.clone());
            hyps.push(hyp_n.clone());
            total_audio += item.dur_s;
            rows.push(serde_json::json!({
                "wav": item.wav,
                "corpus": item.corpus,
                "duration_s": item.dur_s,
                "decode_s": decode_s,
                "wer": wer_one(&gt, &hyp_n),
                "hyp": hyp,
            }));
        }
        let decode_s = started.elapsed().as_secs_f64();
        let over15: Vec<_> = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row["duration_s"].as_f64().unwrap_or(0.0) > 15.0)
            .map(|(i, _)| i)
            .collect();
        let over15_wer = if over15.is_empty() {
            0.0
        } else {
            let r: Vec<_> = over15.iter().map(|&i| refs[i].clone()).collect();
            let h: Vec<_> = over15.iter().map(|&i| hyps[i].clone()).collect();
            wer(&r, &h)
        };
        let report = serde_json::json!({
            "wer": wer(&refs, &hyps),
            "wer_pct": wer(&refs, &hyps) * 100.0,
            "over15_wer": over15_wer,
            "over15_wer_pct": over15_wer * 100.0,
            "decode_s": decode_s,
            "rtf": decode_s / total_audio,
            "total_audio_s": total_audio,
            "rows": rows,
        });
        println!(
            "whisper_coreml WER {:>6.2}%  >15s {:>6.2}%  decode {:>7.2}s  RTF {:.4}",
            report["wer_pct"].as_f64().unwrap(),
            report["over15_wer_pct"].as_f64().unwrap(),
            decode_s,
            decode_s / total_audio
        );
        if let Some(out) = a.get(3) {
            std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
            println!("wrote {out}");
        }
        return;
    }

    let mut wavs: Vec<PathBuf> = std::fs::read_dir(&input_dir)
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

#[cfg(target_os = "macos")]
fn norm(text: &str) -> String {
    text.to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '\'' {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(target_os = "macos")]
fn wer_one(reference: &str, hypothesis: &str) -> f64 {
    wer(&[reference.to_string()], &[hypothesis.to_string()])
}

#[cfg(target_os = "macos")]
fn wer(refs: &[String], hyps: &[String]) -> f64 {
    let r = refs.join(" ");
    let h = hyps.join(" ");
    let rw: Vec<&str> = r.split_whitespace().collect();
    let hw: Vec<&str> = h.split_whitespace().collect();
    if rw.is_empty() {
        return if hw.is_empty() { 0.0 } else { 1.0 };
    }
    let mut prev: Vec<usize> = (0..=hw.len()).collect();
    for (i, rw_i) in rw.iter().enumerate() {
        let mut cur = vec![i + 1; hw.len() + 1];
        for (j, hw_j) in hw.iter().enumerate() {
            cur[j + 1] = (prev[j + 1] + 1)
                .min(cur[j] + 1)
                .min(prev[j] + usize::from(rw_i != hw_j));
        }
        prev = cur;
    }
    prev[hw.len()] as f64 / rw.len() as f64
}
