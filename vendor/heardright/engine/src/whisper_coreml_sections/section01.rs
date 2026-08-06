// Native WhisperKit engine (Track W-C) — runs the 4 precompiled `.mlmodelc`
// stages in-process on the shared CoreML bridge, dropping the `whisperkit-cli`
// subprocess. macOS / ANE only.
//
// Faithful port of WhisperKit's `TextDecoder.decodeText` greedy KV-cache loop.
// Per step:
//   1. force the prompt token while `tokenIndex < prompt.len()`, else use the
//      previously-sampled token as `input_ids`;
//   2. predict → `logits`, `key/value_cache_updates`;
//   3. suppress specials → argmax (greedy);
//   4. write the `[1,5120,1,1]` KV slice into `cache[:,:,:,tokenIndex]`;
//   5. advance the two masks one position.
// The TextDecoderContextPrefill stage (a 3-token KV precompute optimization) is
// intentionally skipped — we run the prompt tokens through the decoder instead.
use std::path::{Path, PathBuf};

use objc2::rc::Retained;
use objc2_core_ml::MLMultiArray;
use tokenizers::Tokenizer;

use crate::coreml::{f16_set, i32_set, ml_f16, ml_i32, read_f16, Stage};

// 30 s window @ 16 kHz; model fixed shapes.
const WINDOW_SAMPLES: usize = 480_000;
const KV_DIM: usize = 5120; // 1280 d_model × 4 turbo decoder layers
const KV_MAX: usize = 448; // max decoded positions (key_cache seq dim) — matches whisper-multi/626 + fp16 large-v3-turbo
const VOCAB: usize = 51866;

// Special token ids (Whisper large-v3, from generation_config.json).
const SOT: i32 = 50258; // <|startoftranscript|>
const EOT: i32 = 50257; // <|endoftext|>  (also pad)
const LANG_EN: i32 = 50259; // <|en|>
const LANG_LAST: i32 = 50358; // <|yue|> — last of the 100 large-v3 language tokens
const TRANSCRIBE: i32 = 50360; // <|transcribe|>
const NO_TS: i32 = 50364; // <|notimestamps|>
const TS_BEGIN: i32 = 50365; // <|0.00|> — first timestamp token (NO_TS + 1)

// `begin_suppress_tokens` ([whitespace, eot]) applied at the first sampled token.
// NOTE: WhisperKit's `suppressTokens` (the 88-id non-speech list from
// generation_config) defaults to EMPTY — it is an unimplemented item in
// DecodingOptions, and the cli ships []. Suppressing those 88 ids corrupted real
// words (e.g. "metro" → "mê trông"), so we match WhisperKit and suppress nothing
// beyond this begin-suppress + the timestamp rules. Decode runs in TIMESTAMP mode
// (initial <|0.00|> forced via the prompt) so the model's trained timestamp
// structure terminates segments — without it greedy loops/truncates.
const BEGIN_SUPPRESS: &[usize] = &[220, 50257];

// Sentinel for "detect the language" — Whisper predicts the language token at
// the SOT step; passing this instead of a real `<|xx|>` reads that prediction.
const AUTO_LANG: i32 = -1;

pub struct WhisperCoreMl {
    encoder: Stage,
    decoder: Stage,
    tokenizer: Tokenizer,
}

impl WhisperCoreMl {
    /// Load the WhisperKit `.mlmodelc` bundle dir + the Whisper tokenizer.
    /// `HR_WHISPER_TOKENIZER` overrides the bundled tokenizer path for QA only.
    pub fn load(model_dir: &Path) -> Result<Self, String> {
        // MelSpectrogram.mlmodelc is intentionally NOT loaded — the mel is computed in
        // Rust (canonical whisper log_mel), which beat the WK mel stage on the ANE.
        let encoder = Stage::load(&model_dir.join("AudioEncoder.mlmodelc"))?;
        let decoder = Stage::load(&model_dir.join("TextDecoder.mlmodelc"))?;
        let tok_path = tokenizer_path(model_dir);
        let tokenizer = Tokenizer::from_file(&tok_path)
            .map_err(|e| format!("load tokenizer {}: {e}", tok_path.display()))?;
        Ok(Self {
            encoder,
            decoder,
            tokenizer,
        })
    }

    /// Look up a Whisper language token id, e.g. `lang_token("ja")` → `<|ja|>`.
    /// `"auto"` returns the AUTO sentinel (-1): Whisper detects the language
    /// itself from the SOT step instead of being forced to one.
    pub fn lang_token(&self, lang: &str) -> i32 {
        if lang == "auto" {
            return AUTO_LANG;
        }
        self.tokenizer
            .token_to_id(&format!("<|{lang}|>"))
            .map(|x| x as i32)
            .unwrap_or(LANG_EN)
    }

    fn detok(&self, toks: &[u32]) -> Result<String, String> {
        self.tokenizer
            .decode(toks, true)
            .map(|s| s.trim().to_string())
            .map_err(|e| format!("detokenize: {e}"))
    }

    /// Transcribe 16 kHz mono f32 samples (English, no timestamps). Windows at 30 s.
    pub fn transcribe(&self, audio: &[f32]) -> Result<String, String> {
        self.transcribe_lang_windowed(audio, LANG_EN)
    }

    /// Transcribe 16 kHz mono f32 samples with an explicit Whisper language token.
    ///
    /// Follows the padded-window contract (`docs/ASR_DECODE_CONTRACT.md`) with
    /// Whisper's native 30 s window. This previously advanced by hard
    /// `start += WINDOW_SAMPLES` cuts and joined the pieces with a blind space:
    /// a word straddling a 30 s boundary was split across two decodes with
    /// nothing to repair it. The quiet cut moves each boundary onto a low-energy
    /// span and the shared seam rule de-duplicates overlap.
    pub fn transcribe_lang_windowed(&self, audio: &[f32], lang_tok: i32) -> Result<String, String> {
        // For AUTO, detect on the first window then lock that language for the
        // rest so the transcript can't flip language mid-dictation.
        let mut active = lang_tok;
        crate::asr::transcribe_padded_window_text(audio, WINDOW_SAMPLES, |seg| {
            let window_started = std::time::Instant::now();
            let enc = self.encode_audio(seg)?;
            let (toks, detected) = self.decode_from_enc(&enc, active)?;
            tracing::info!(
                "whisper window decoded: len_s={:.1} ms={} toks={} lang_tok={}",
                seg.len() as f32 / 16_000.0,
                window_started.elapsed().as_millis(),
                toks.len(),
                detected
            );
            active = detected;
            if toks.is_empty() {
                return Ok(String::new());
            }
            self.detok(&toks)
        })
    }

    /// Single-window transcribe with an explicit language token (clips ≤ 30 s).
    pub fn transcribe_lang(&self, audio: &[f32], lang_tok: i32) -> Result<String, String> {
        let seg = &audio[..audio.len().min(WINDOW_SAMPLES)];
        let enc = self.encode_audio(seg)?;
        let (toks, _) = self.decode_from_enc(&enc, lang_tok)?;
        self.detok(&toks)
    }

    /// CROSS-FEED: decode from an externally-computed log-mel (`128×3000`, C-order),
    /// bypassing the WhisperKit MelSpectrogram stage. Used to prove whether the
    /// WhisperKit-vs-whisper.cpp gap lives in the mel front-end.
    pub fn transcribe_mel_lang(&self, mel: &[f32], lang_tok: i32) -> Result<String, String> {
        if mel.len() != 128 * 3000 {
            return Err(format!("mel must be 128*3000, got {}", mel.len()));
        }
        let mel_arr = ml_f16(&[1, 128, 1, 3000], mel)?;
        let enc: Retained<MLMultiArray> = self.encoder.predict(
            &[("melspectrogram_features", &mel_arr)],
            "encoder_output_embeds",
        )?;
        let (toks, _) = self.decode_from_enc(&enc, lang_tok)?;
        self.detok(&toks)
    }

    /// 16 kHz f32 (≤30 s window) → encoder embeddings via the Rust whisper log-mel.
    fn encode_audio(&self, seg: &[f32]) -> Result<Retained<MLMultiArray>, String> {
        let mut padded = vec![0f32; WINDOW_SAMPLES];
        padded[..seg.len()].copy_from_slice(seg);
        let mel = log_mel(&padded); // [128 × 3000] C-order
        let mel_arr = ml_f16(&[1, MEL_N_MELS, 1, mel.len() / MEL_N_MELS], &mel)?;
        self.encoder.predict(
            &[("melspectrogram_features", &mel_arr)],
            "encoder_output_embeds",
        )
    }

    /// Encoder embeddings → content token ids (greedy KV-cache decode), `lang_tok`
    /// selects the Whisper language in the forced prompt. Pass `AUTO_LANG` (-1) to
    /// let the model detect the language at the SOT step. Returns the decoded
    /// tokens plus the resolved language token (the detected one when AUTO).
    fn decode_from_enc(
        &self,
        enc_arr: &Retained<MLMultiArray>,
        lang_tok: i32,
    ) -> Result<(Vec<u32>, i32), String> {
        // Timestamp mode (WhisperKit default `withoutTimestamps=false`): the initial
        // timestamp <|0.00|> is FORCED via the prompt (not sampled — matches the
        // disabled in-filter init-ts path in WhisperKit's TimestampRulesFilter).
        let ts_mode = std::env::var("HR_WHISPER_NOTS").is_err();
        // AUTO (-1): seed the prompt with English as a placeholder; the real
        // language is read from the SOT-step prediction at ti==0 below and
        // written back into prompt[1] before it is forced at ti==1.
        let mut resolved_lang = lang_tok;
        let prompt_lang = if lang_tok == AUTO_LANG {
            LANG_EN
        } else {
            lang_tok
        };
        let mut prompt: Vec<i32> = if ts_mode {
            vec![SOT, prompt_lang, TRANSCRIBE, TS_BEGIN]
        } else {
            vec![SOT, prompt_lang, TRANSCRIBE, NO_TS]
        };
        let init_prompt = prompt.len(); // 4 (SOT, lang, transcribe, <|0.00|> | <|notimestamps|>)
                                        // Allocate decoder inputs ONCE and mutate in place each step (CoreML reads at
                                        // predict time). `current` is the full token list (prompt + sampled), exactly
                                        // the `currentTokens` WhisperKit hands to every LogitsFilter.
        let ids = ml_i32(&[1], &[prompt[0]])?;
        let clen = ml_i32(&[1], &[0])?;
        let kc = ml_f16(&[1, KV_DIM, 1, KV_MAX], &vec![0f32; KV_DIM * KV_MAX])?;
        let vc = ml_f16(&[1, KV_DIM, 1, KV_MAX], &vec![0f32; KV_DIM * KV_MAX])?;
        let kpad = ml_f16(&[1, KV_MAX], &vec![-10_000f32; KV_MAX])?;
        f16_set(&kpad, 0, 0.0);
        let kupd = ml_i32(&[1, KV_MAX], &vec![0i32; KV_MAX])?;
        i32_set(&kupd, 0, 1);

        let mut current: Vec<i32> = prompt.to_vec();
        let mut next = prompt[0];
        let cap = KV_MAX - 1;
        for ti in 0..cap {
            if ti < init_prompt {
                next = prompt[ti]; // force the prompt prefix (incl. the <|0.00|>)
            }
            i32_set(&ids, 0, next);
            i32_set(&clen, 0, ti as i32);
            let outs = self.decoder.predict_multi(
                &[
                    ("input_ids", &ids),
                    ("cache_length", &clen),
                    ("key_cache", &kc),
                    ("value_cache", &vc),
                    ("decoder_key_padding_mask", &kpad),
                    ("kv_cache_update_mask", &kupd),
                    ("encoder_output_embeds", enc_arr),
                ],
                &["logits", "key_cache_updates", "value_cache_updates"],
            )?;
            // AUTO detection: the SOT step (ti==0) predicts the language token.
            // Argmax over the language-token range, lock it into the forced
            // prompt so ti==1 forces the detected language (not the placeholder).
            if resolved_lang == AUTO_LANG && ti == 0 {
                let logits = read_f16(&outs[0]);
                // Rank ALL language tokens so we can see the picked lang AND the
                // runners-up + their logits (diagnostic for mis-detection).
                let mut ranked: Vec<(i32, f32)> = (LANG_EN..=LANG_LAST)
                    .map(|t| (t, logits[t as usize]))
                    .collect();
                ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                let best = ranked[0].0;
                let top: Vec<String> = ranked
                    .iter()
                    .take(5)
                    .map(|(t, v)| {
                        format!(
                            "{}={:.2}",
                            self.tokenizer.id_to_token(*t as u32).unwrap_or_default(),
                            v
                        )
                    })
                    .collect();
                tracing::info!(
                    "whisper auto-detect: picked {} · top5: {}",
                    self.tokenizer.id_to_token(best as u32).unwrap_or_default(),
                    top.join("  ")
                );
                resolved_lang = best;
                prompt[1] = best;
                current[1] = best;
            }
            // Sampling begins at the last prompt token (ti == init_prompt-1); earlier
            // steps just prefill the KV cache with the forced prompt.
            if ti >= init_prompt - 1 {
                let mut logits = read_f16(&outs[0]); // [1,1,VOCAB]
                apply_filters(&mut logits, &current, init_prompt, ts_mode);
                let sampled = argmax(&logits) as i32;
                if sampled == EOT {
                    break;
                }
                current.push(sampled);
                next = sampled;
            }
            // write the [1,5120,1,1] update slice into cache[:,:,:,ti] in place
            let ku = read_f16(&outs[1]);
            let vu = read_f16(&outs[2]);
            for c in 0..KV_DIM {
                f16_set(&kc, c * KV_MAX + ti, ku[c]);
                f16_set(&vc, c * KV_MAX + ti, vu[c]);
            }
            f16_set(&kpad, ti + 1, 0.0);
            i32_set(&kupd, ti, 0);
            i32_set(&kupd, ti + 1, 1);
        }
        if std::env::var("HR_WHISPER_DUMP").is_ok() {
            let s: String = current[init_prompt..]
                .iter()
                .map(|&t| {
                    if t >= TS_BEGIN {
                        format!("⟨{:.2}⟩", (t - TS_BEGIN) as f32 * 0.02)
                    } else {
                        self.tokenizer.id_to_token(t as u32).unwrap_or_default()
                    }
                })
                .collect();
            let n_ts = current[init_prompt..]
                .iter()
                .filter(|&&t| t >= TS_BEGIN)
                .count();
            eprintln!(
                "[dump] {} sampled, {} timestamps:\n{s}",
                current.len() - init_prompt,
                n_ts
            );
        }
        // keep only sampled text tokens (drop the prompt and all timestamps ≥ TS_BEGIN)
        let toks = current[init_prompt..]
            .iter()
            .filter(|&&t| t < EOT)
            .map(|&t| t as u32)
            .collect();
        Ok((toks, resolved_lang))
    }
}

const NEG: f32 = f32::NEG_INFINITY;
