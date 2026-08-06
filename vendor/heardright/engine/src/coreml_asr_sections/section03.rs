
/// Group word-level [`TimedTok`]s into subtitle cues and render SRT + WebVTT.
/// A cue closes on sentence-final punctuation, after ~12 words, after ~7 s, or
/// on a >1 s silence gap — whichever comes first.
pub fn render_subtitles(words: &[TimedTok]) -> (String, String) {
    let mut cues: Vec<TimedTok> = Vec::new();
    let mut cur: Option<TimedTok> = None;
    let mut cur_words = 0usize;
    for w in words {
        match cur.as_mut() {
            None => {
                cur = Some(w.clone());
                cur_words = 1;
            }
            Some(c) => {
                let gap = w.start - c.end;
                if gap > 1.0 || cur_words >= 12 || (c.end - c.start) >= 7.0 {
                    cues.push(c.clone());
                    cur = Some(w.clone());
                    cur_words = 1;
                } else {
                    c.text.push(' ');
                    c.text.push_str(&w.text);
                    c.end = w.end;
                    cur_words += 1;
                }
            }
        }
        if let Some(c) = cur.as_ref() {
            if c.text.ends_with(['.', '?', '!']) && cur_words >= 3 {
                cues.push(c.clone());
                cur = None;
                cur_words = 0;
            }
        }
    }
    if let Some(c) = cur {
        cues.push(c);
    }

    let mut srt = String::new();
    let mut vtt = String::from("WEBVTT\n\n");
    for (i, c) in cues.iter().enumerate() {
        let end = c.end.max(c.start + 0.08);
        srt.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            subtitle_timestamp(c.start, true),
            subtitle_timestamp(end, true),
            c.text.trim()
        ));
        vtt.push_str(&format!(
            "{} --> {}\n{}\n\n",
            subtitle_timestamp(c.start, false),
            subtitle_timestamp(end, false),
            c.text.trim()
        ));
    }
    (srt, vtt)
}

fn subtitle_timestamp(seconds: f32, comma: bool) -> String {
    let total_ms = (seconds.max(0.0) * 1000.0).round() as u64;
    let hours = total_ms / 3_600_000;
    let minutes = (total_ms % 3_600_000) / 60_000;
    let secs = (total_ms % 60_000) / 1000;
    let millis = total_ms % 1000;
    let sep = if comma { ',' } else { '.' };
    format!("{hours:02}:{minutes:02}:{secs:02}{sep}{millis:03}")
}

// ---- helpers ----------------------------------------------------------------

fn stats(v: &[f32]) -> (f32, f32, f32) {
    let mut mn = f32::INFINITY;
    let mut mx = f32::NEG_INFINITY;
    let mut sm = 0f32;
    for &x in v {
        mn = mn.min(x);
        mx = mx.max(x);
        sm += x;
    }
    (mn, mx, sm)
}

fn argmax(v: &[f32]) -> usize {
    let mut bi = 0usize;
    let mut bv = f32::NEG_INFINITY;
    for (i, &x) in v.iter().enumerate() {
        if x > bv {
            bv = x;
            bi = i;
        }
    }
    bi
}

fn detok(ids: &[usize], pieces: &[String]) -> String {
    let mut s = String::new();
    for &i in ids {
        if let Some(p) = pieces.get(i) {
            s.push_str(p);
        }
    }
    s.replace('\u{2581}', " ").trim().to_string()
}

/// Resolve the shipped bundle dir for a CoreML Parakeet model.
/// `HR_COREML_MODEL_DIR` overrides; else `{base}/{name}`.
pub fn bundle_dir(base: &Path, name: &str) -> PathBuf {
    if let Ok(p) = std::env::var("HR_COREML_MODEL_DIR") {
        return PathBuf::from(p);
    }
    base.join(name)
}

#[cfg(test)]
mod model_fingerprint_tests {
    use super::sampled_model_fingerprint;

    #[test]
    fn sampled_model_fingerprint_is_stable_and_content_sensitive() {
        let unique = format!(
            "heardright-model-fingerprint-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        let stage = root.join("AudioEncoder.mlmodelc").join("weights");
        std::fs::create_dir_all(&stage).unwrap();
        std::fs::write(root.join("pipeline.json"), b"{\"window_sec\":15}").unwrap();
        std::fs::write(stage.join("weight.bin"), vec![7u8; 12_000]).unwrap();

        let first = sampled_model_fingerprint(&root).unwrap();
        let second = sampled_model_fingerprint(&root).unwrap();
        assert_eq!(first, second);

        let mut changed = vec![7u8; 12_000];
        *changed.last_mut().unwrap() = 8;
        std::fs::write(stage.join("weight.bin"), changed).unwrap();
        let third = sampled_model_fingerprint(&root).unwrap();
        assert_ne!(first.0, third.0);
        assert_eq!(first.1, third.1);
        assert_eq!(first.2, third.2);

        std::fs::remove_dir_all(root).unwrap();
    }
}
