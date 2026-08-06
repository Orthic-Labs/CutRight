
/// WhisperKit's LogitsFilter chain for one content step, in order (matches
/// TextDecoder.createLogitsFilters): SuppressBlank → SuppressTokens → TimestampRules.
/// `current` is the full token list (prompt + sampled); `init_prompt` is the prompt
/// length (4 in timestamp mode), which is both the SuppressBlank trigger point and
/// the TimestampRules sampleBegin (= max(transcribeIdx+1, initialPromptIndex)).
fn apply_filters(logits: &mut [f32], current: &[i32], init_prompt: usize, ts_mode: bool) {
    // NOTE: WhisperKit's default `suppressTokens` is EMPTY (the non-speech list is an
    // unimplemented item in DecodingOptions), and the cli ships []. Suppressing the 88
    // generation_config ids here corrupted real words (e.g. "metro" → "mê trông"), so
    // we match WhisperKit and suppress nothing beyond blank + timestamp rules.
    // SuppressBlankFilter — at the first sampled token, suppress [whitespace, eot].
    if current.len() == init_prompt {
        for &t in BEGIN_SUPPRESS {
            logits[t] = NEG;
        }
    }
    // TimestampRulesFilter — only added when decoding WITH timestamps.
    if ts_mode {
        apply_timestamp_rules(logits, &current[init_prompt..]);
    } else {
        // no-ts: model is prompted with <|notimestamps|>; forbid timestamps explicitly.
        for t in (TS_BEGIN as usize)..VOCAB {
            logits[t] = NEG;
        }
    }
}

/// WhisperKit `TimestampRulesFilter` (port of openai/whisper decoding.py): timestamps
/// appear in pairs and never decrease; if Σ P(timestamp) exceeds the best text token,
/// force a timestamp. Initial timestamp is NOT forced here — it is forced via the
/// prompt's <|0.00|> (WhisperKit disabled the in-filter init-ts block).
fn apply_timestamp_rules(logits: &mut [f32], sampled: &[i32]) {
    logits[NO_TS as usize] = NEG; // never emit <|notimestamps|>
    let n = sampled.len();
    if n > 0 {
        let last_ts = sampled[n - 1] >= TS_BEGIN;
        // WhisperKit: count < 2 ⇒ penultimate-was-timestamp = true.
        let penult_ts = n < 2 || sampled[n - 2] >= TS_BEGIN;
        if last_ts {
            if penult_ts {
                // has to be non-timestamp: suppress timestamps
                for t in (TS_BEGIN as usize)..VOCAB {
                    logits[t] = NEG;
                }
            } else {
                // cannot be normal text: suppress text (0..eot)
                for t in 0..(EOT as usize) {
                    logits[t] = NEG;
                }
            }
        }
        // non-decreasing + nonzero segment length: closing ts ≥ last; else strictly >
        if let Some(&lt) = sampled.iter().rev().find(|&&t| t >= TS_BEGIN) {
            let lo = if last_ts && !penult_ts { lt } else { lt + 1 };
            for t in (TS_BEGIN as usize)..(lo as usize) {
                logits[t] = NEG;
            }
        }
    }
    // if Σ P(timestamp) > max P(text), force a timestamp
    let m = logits.iter().cloned().fold(NEG, f32::max);
    if !m.is_finite() {
        return;
    }
    let mut z = 0f32;
    for &x in logits.iter() {
        if x.is_finite() {
            z += (x - m).exp();
        }
    }
    let logz = m + z.ln();
    let mut ts_se = 0f32;
    for t in (TS_BEGIN as usize)..VOCAB {
        if logits[t].is_finite() {
            ts_se += (logits[t] - m).exp();
        }
    }
    let ts_lp = if ts_se > 0.0 {
        m + ts_se.ln() - logz
    } else {
        NEG
    };
    let mut max_text = NEG;
    for t in 0..(TS_BEGIN as usize) {
        if logits[t].is_finite() {
            max_text = max_text.max(logits[t] - logz);
        }
    }
    if ts_lp > max_text {
        for t in 0..(TS_BEGIN as usize) {
            logits[t] = NEG;
        }
    }
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

// ── Whisper log-mel front-end (canonical openai/whisper spec) ───────────────
// Replaces the WhisperKit MelSpectrogram CoreML stage. Spec (pinned): n_fft=400,
// hop=160, periodic Hann, reflect-pad n_fft/2, 128 slaney mel filters, log10
// clamped to (max-8), then (x+4)/4. The filterbank bytes ARE whisper's shipped
// mel_128 (assets/mel_filters.npz). Verified by numerical diff against the numpy
// oracle (scripts/whisper_mel.py) to ≤1e-4/element BEFORE trusting CER — a subtly
// wrong mel transcribes fine then degrades silently (root cause on both platforms).
const MEL_N_FFT: usize = 400;
const MEL_HOP: usize = 160;
const MEL_N_MELS: usize = 128;
const MEL_N_FREQ: usize = 201; // n_fft/2 + 1
static MEL_FILTERS_BIN: &[u8] = include_bytes!("../whisper_mel_128.bin"); // 128*201 f32 LE

fn mel_filters() -> Vec<f32> {
    MEL_FILTERS_BIN
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// Canonical whisper log-mel. `audio` = 16 kHz mono f32; returns `[128 × frames]`
/// C-order (mel-major), `frames = audio_len/hop` (3000 for a 30 s window) — the exact
/// layout the AudioEncoder's `melspectrogram_features [1,128,1,3000]` input expects.
pub fn log_mel(audio: &[f32]) -> Vec<f32> {
    use realfft::RealFftPlanner;
    let pad = MEL_N_FFT / 2; // 200
    let n = audio.len();
    let mut a = vec![0f32; n + 2 * pad];
    for i in 0..pad {
        a[i] = audio[pad - i]; // numpy 'reflect' (no edge repeat)
    }
    a[pad..pad + n].copy_from_slice(audio);
    for i in 0..pad {
        a[pad + n + i] = audio[n - 2 - i];
    }
    // periodic Hann (== torch.hann_window(400) == np.hanning(401)[:-1])
    let win: Vec<f32> = (0..MEL_N_FFT)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / MEL_N_FFT as f32).cos())
        .collect();
    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(MEL_N_FFT);
    let mut inbuf = r2c.make_input_vec();
    let mut outbuf = r2c.make_output_vec();
    let frames = (a.len() - MEL_N_FFT) / MEL_HOP; // 1 + ... then drop last → audio_len/hop
    let mut power = vec![0f32; MEL_N_FREQ * frames];
    for f in 0..frames {
        for i in 0..MEL_N_FFT {
            inbuf[i] = a[f * MEL_HOP + i] * win[i];
        }
        r2c.process(&mut inbuf, &mut outbuf).expect("rfft");
        for k in 0..MEL_N_FREQ {
            power[k * frames + f] = outbuf[k].norm_sqr(); // |stft|^2
        }
    }
    let filt = mel_filters();
    let mut mel = vec![0f32; MEL_N_MELS * frames];
    for m in 0..MEL_N_MELS {
        let frow = &filt[m * MEL_N_FREQ..(m + 1) * MEL_N_FREQ];
        for f in 0..frames {
            let mut s = 0f32;
            for k in 0..MEL_N_FREQ {
                s += frow[k] * power[k * frames + f];
            }
            mel[m * frames + f] = s;
        }
    }
    let mut mx = f32::NEG_INFINITY;
    for v in mel.iter_mut() {
        *v = v.max(1e-10).log10();
        if *v > mx {
            mx = *v;
        }
    }
    let floor = mx - 8.0;
    for v in mel.iter_mut() {
        *v = (v.max(floor) + 4.0) / 4.0;
    }
    mel
}

/// Default tokenizer path is the bundled `tokenizer.json` beside the CoreML
/// bundle. `HR_WHISPER_TOKENIZER` remains as a QA override only.
pub fn tokenizer_path(model_dir: &Path) -> PathBuf {
    if let Ok(p) = std::env::var("HR_WHISPER_TOKENIZER") {
        return PathBuf::from(p);
    }
    model_dir.join("tokenizer.json")
}
