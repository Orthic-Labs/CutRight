/// Resolve the decode beam width: `HR_ASR_BEAM_WIDTH` env override (clamped
/// to `1..=8`, unparseable values fall back to the default) else `1` (the
/// original greedy decode). Read once per `load()` so experiments need no
/// rebuild — set the env var and relaunch.
fn resolve_beam_width() -> (usize, &'static str) {
    match std::env::var("HR_ASR_BEAM_WIDTH")
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
    {
        Some(parsed) => (parsed.clamp(1, 8), "env"),
        None => (1, "default"),
    }
}

impl CoreMlParakeet {
    /// Load a CoreML Parakeet bundle directory. Precompiled `.mlmodelc` stages are
    /// preferred when present; `.mlpackage` stages remain the dev fallback.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let meta = PipelineMeta::from_dir(dir)?;
        let global_compute = crate::settings::asr_compute_profile();
        let encoder_setting = crate::settings::asr_encoder_compute();
        let decoder_setting = crate::settings::asr_decoder_compute();
        let encoder_profile = if encoder_setting == "inherit" {
            global_compute.as_str()
        } else {
            encoder_setting.as_str()
        };
        let decoder_profile = if decoder_setting == "inherit" {
            global_compute.as_str()
        } else {
            decoder_setting.as_str()
        };
        let mel =
            Stage::load_with_compute_profile(&stage_path(dir, "MelSpectrogram"), encoder_profile)?;
        let enc =
            Stage::load_with_compute_profile(&stage_path(dir, "AudioEncoder"), encoder_profile)?;
        let dec =
            Stage::load_with_compute_profile(&stage_path(dir, "TextDecoder"), decoder_profile)?;
        let joint = Stage::load_with_compute_profile(&stage_path(dir, "Joint"), decoder_profile)?;
        let encoder_compute = enc.compute_profile().to_string();
        let decoder_compute = dec.compute_profile().to_string();
        let coreml_compile_cache = [
            mel.compile_cache(),
            enc.compile_cache(),
            dec.compile_cache(),
            joint.compile_cache(),
        ]
        .into_iter()
        .find(|status| *status == "fresh_compile")
        .or_else(|| {
            [
                mel.compile_cache(),
                enc.compile_cache(),
                dec.compile_cache(),
                joint.compile_cache(),
            ]
            .into_iter()
            .find(|status| *status == "cache_hit")
        })
        .unwrap_or("precompiled_bundle")
        .to_string();

        let mel_out = mel
            .output_names()
            .into_iter()
            .next()
            .ok_or("MelSpectrogram has no output")?;
        let enc_out = enc
            .output_names()
            .into_iter()
            .next()
            .ok_or("AudioEncoder has no output")?;
        let joint_out = joint
            .output_names()
            .into_iter()
            .next()
            .ok_or("Joint has no output")?;
        // Decoder: g = [1,1,pred_hidden]; the two LSTM states = [pred_layers,1,pred_hidden].
        let g_name = dec.output_by_shape(&[1, 1, meta.pred_hidden])?;
        let states = dec.outputs_by_shape(&[meta.pred_layers, 1, meta.pred_hidden]);
        if states.len() < 2 {
            return Err(format!(
                "TextDecoder: expected 2 state outputs of shape [{},1,{}], found {}",
                meta.pred_layers,
                meta.pred_hidden,
                states.len()
            ));
        }
        let state_h_out = states[0].clone();
        let state_c_out = states[1].clone();

        let vocab = std::fs::read_to_string(dir.join("vocab.txt"))
            .map_err(|e| format!("read vocab.txt: {e}"))?;
        let pieces: Vec<String> = vocab
            .lines()
            .map(|ln| ln.split('\t').next().unwrap_or("").to_string())
            .collect();

        let win_samples = (meta.window_sec * 16_000.0) as usize;
        let (beam_width, beam_source) = resolve_beam_width();
        eprintln!("asr_beam_width width={beam_width} source={beam_source}");
        tracing::info!(
            encoder_compute,
            decoder_compute,
            "CoreML Parakeet compute routing"
        );
        let model_fingerprint = match sampled_model_fingerprint(dir) {
            Ok((sampled_fingerprint, model_bytes, model_files)) => {
                tracing::info!(
                model_dir = %dir.display(),
                sampled_fingerprint = sampled_fingerprint,
                model_bytes,
                model_files,
                window_sec = meta.window_sec,
                mel_frames = meta.mel_frames,
                enc_frames = meta.enc_frames,
                "CoreML Parakeet model fingerprint"
                );
                sampled_fingerprint
            }
            Err(error) => {
                tracing::warn!(
                    model_dir = %dir.display(),
                    error,
                    "CoreML Parakeet model fingerprint unavailable"
                );
                "unavailable".to_string()
            }
        };
        let model_dir_sha256 = if crate::settings::asr_detailed_telemetry()
            || crate::settings::onboarding_calibration_active()
        {
            match model_dir_sha256(dir) {
                Ok(hash) => Some(hash),
                Err(error) => {
                    tracing::warn!(model_dir = %dir.display(), error, "CoreML model SHA-256 unavailable");
                    None
                }
            }
        } else {
            None
        };
        Ok(Self {
            meta,
            mel,
            enc,
            dec,
            joint,
            mel_out,
            enc_out,
            g_name,
            state_h_out,
            state_c_out,
            joint_out,
            pieces,
            win_samples,
            bias: None,
            beam_width,
            model_fingerprint,
            model_dir_sha256,
            coreml_compile_cache,
            encoder_compute,
            decoder_compute,
        })
    }

    /// Set the decode beam width (clamped to `1..=8`). `1` is bit-identical
    /// to the original greedy decode (see `decode_window`); each additional
    /// width point costs one more decoder/joint call per active hypothesis
    /// per step — this is not optimized, by design (see task rationale in
    /// `decode_window.rs`).
    pub fn set_beam_width(&mut self, width: usize) {
        self.beam_width = width.clamp(1, 8);
    }

    /// Configure contextual biasing (dictionary + command terms). Mirrors
    /// `HrTransducer::set_context_bias_phrases`. Returns the count installed.
    pub fn set_context_bias_phrases<I, S>(&mut self, phrases: I, score: f32) -> usize
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        if !(score.is_finite() && score > 0.0) {
            self.bias = None;
            return 0;
        }
        let encoded: Vec<Vec<usize>> = phrases
            .into_iter()
            .filter_map(|p| self.encode_phrase_greedy(p.as_ref()))
            .filter(|ids| !ids.is_empty())
            .collect();
        let n = encoded.len();
        self.bias = (n > 0).then_some(ContextBias {
            phrase_token_ids: encoded,
            score,
        });
        n
    }

    pub fn clear_context_bias(&mut self) {
        self.bias = None;
    }

    /// Greedy longest-match SentencePiece encode of a phrase against this
    /// bundle's `pieces` — identical to the ONNX vocab's `encode_phrase_greedy`.
    fn encode_phrase_greedy(&self, phrase: &str) -> Option<Vec<usize>> {
        let normalized = normalize_phrase_for_bias(phrase);
        if normalized.is_empty() {
            return None;
        }
        let mut piece_text = String::new();
        for word in normalized.split_whitespace() {
            piece_text.push('\u{2581}');
            piece_text.push_str(word);
        }
        let mut remaining = piece_text.as_str();
        let mut ids = Vec::new();
        while !remaining.is_empty() {
            let mut best: Option<(usize, usize)> = None;
            for (id, token) in self.pieces.iter().enumerate() {
                if token.is_empty() || token.starts_with('<') {
                    continue;
                }
                if remaining.starts_with(token.as_str()) {
                    let len = token.len();
                    if best.map(|(_, bl)| len > bl).unwrap_or(true) {
                        best = Some((id, len));
                    }
                }
            }
            let (id, len) = best?;
            ids.push(id);
            remaining = &remaining[len..];
        }
        Some(ids)
    }

    pub fn is_tdt(&self) -> bool {
        self.meta.is_tdt
    }

    /// Configured CoreML compute profiles permitted for these stages. CoreML
    /// does not disclose actual per-inference device placement.
    pub fn configured_compute_route(&self) -> (&str, &str) {
        (&self.encoder_compute, &self.decoder_compute)
    }

    pub fn model_fingerprint(&self) -> &str {
        &self.model_fingerprint
    }

    pub fn model_dir_sha256(&self) -> Option<&str> {
        self.model_dir_sha256.as_deref()
    }

    pub fn compile_cache(&self) -> &str {
        &self.coreml_compile_cache
    }

    fn predict_decoder_joint(
        &self,
        last_tok: i32,
        h: &[f32],
        c: &[f32],
        enc_step: &[f32],
        pl: usize,
        ph: usize,
        enc_d: usize,
    ) -> Result<(Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>), String> {
        let (g, new_h, new_c) = autoreleasepool(|_| -> Result<_, String> {
            let ids = ml_i32(&[1, 1], &[last_tok])?;
            let hin = ml_f16(&[pl, 1, ph], h)?;
            let cin = ml_f16(&[pl, 1, ph], c)?;
            let outs = self.dec.predict_multi(
                &[
                    ("decoder_input_ids", &ids),
                    ("state_h", &hin),
                    ("state_c", &cin),
                ],
                &[&self.g_name, &self.state_h_out, &self.state_c_out],
            )?;
            Ok((read_f16(&outs[0]), read_f16(&outs[1]), read_f16(&outs[2])))
        })?;
        let logits = autoreleasepool(|_| -> Result<Vec<f32>, String> {
            let enc_in = ml_f16(&[1, enc_d], enc_step)?;
            let dec_in = ml_f16(&[1, ph], &g)?;
            let out = self.joint.predict(
                &[("encoder_step", &enc_in), ("decoder_step", &dec_in)],
                &self.joint_out,
            )?;
            Ok(read_f16(&out))
        })?;
        Ok((g, new_h, new_c, logits))
    }

    /// Transcribe 16 kHz mono f32 samples. Windowed, per-window state reset.
    pub fn transcribe(&self, audio: &[f32]) -> Result<String, String> {
        let hits = self.decode_all(audio)?;
        let ids: Vec<usize> = hits.iter().map(|h| h.0).collect();
        Ok(detok(&ids, &self.pieces))
    }

    /// Transcribe with word-level timestamps (offline file path). Same decode as
    /// [`transcribe`]; additionally groups subword pieces into words on the `▁`
    /// boundary and stamps each with the TDT frame time (80 ms/frame: 160 hop ×
    /// 8 subsample @ 16 kHz, offset by the window start). Returns (full_text, words).
    pub fn transcribe_timed(&self, audio: &[f32]) -> Result<(String, Vec<TimedTok>), String> {
        let hits = self.decode_all(audio)?;
        let ids: Vec<usize> = hits.iter().map(|h| h.0).collect();
        let text = detok(&ids, &self.pieces);

        // Group subword pieces into words. A piece beginning with `▁` opens a new
        // word; the word's start is that piece's frame time.
        let mut words: Vec<TimedTok> = Vec::new();
        for &(id, start, _dur) in &hits {
            let piece = self.pieces.get(id).map(|s| s.as_str()).unwrap_or("");
            let starts_word = piece.starts_with('\u{2581}');
            let clean = piece.replace('\u{2581}', "");
            if starts_word || words.is_empty() {
                words.push(TimedTok {
                    text: clean,
                    start,
                    end: start,
                });
            } else if let Some(last) = words.last_mut() {
                last.text.push_str(&clean);
                last.end = start;
            }
        }
        // End of word N = start of word N+1 (last word gets a 0.4 s tail). Clamp
        // to a minimum 80 ms cue so zero-width cues never reach the SRT.
        let n = words.len();
        for i in 0..n {
            let end = if i + 1 < n {
                words[i + 1].start
            } else {
                words[i].end.max(words[i].start) + 0.4
            };
            words[i].end = end.max(words[i].start + 0.08);
            words[i].text = words[i].text.trim().to_string();
        }
        words.retain(|w| !w.text.is_empty());
        Ok((text, words))
    }

    /// Transcribe to display-token pieces with timestamps. Used by buffered seam
    /// experiments that must match the ONNX `TimedToken` commit behavior.
    pub fn transcribe_pieces_timed(&self, audio: &[f32]) -> Result<Vec<TimedPiece>, String> {
        let hits = self.decode_all(audio)?;
        let mut full_text = String::new();
        let mut pieces = Vec::with_capacity(hits.len());
        for (id, start, dur_frames) in hits {
            let token_text = self.pieces.get(id).map(|s| s.as_str()).unwrap_or("");
            if token_text.starts_with('<') && token_text.ends_with('>') && token_text != "<unk>" {
                continue;
            }
            let mut display_text = token_text.replace('\u{2581}', " ");
            if !full_text.is_empty()
                && !display_text.starts_with(' ')
                && display_text.chars().all(|c| c.is_ascii_digit())
            {
                display_text.insert(0, ' ');
            }
            full_text.push_str(&display_text);
            pieces.push(TimedPiece {
                text: display_text,
                start,
                duration: dur_frames as f32 * 0.08,
            });
        }
        Ok(pieces)
    }

    /// Windowed greedy decode over the whole clip. Returns (piece_id, start_sec)
    /// per emitted token. Shared by [`transcribe`] and [`transcribe_timed`].
    ///
    /// SINGLE WINDOW ONLY. This used to loop `ws += win`, splitting long audio at
    /// hard 15 s boundaries with no quiet cut and no seam de-duplication — cuts
    /// landed mid-word and the pieces were concatenated blind. Nothing in the app
    /// ever reached it (every production caller short-circuits to <= one window
    /// and long audio goes through `transcribe_padded_window`), but it was a
    /// public API that silently produced bad transcripts for long input.
    ///
    /// Long audio has exactly one correct path: `AsrRuntime::transcribe*`, which
    /// applies the padded-window contract in `docs/ASR_DECODE_CONTRACT.md`.
    fn decode_all(&self, audio: &[f32]) -> Result<Vec<(usize, f32, usize)>, String> {
        let max_symbols = 10usize;
        if audio.len() > self.win_samples {
            return Err(format!(
                "audio is {:.1}s but this decode primitive handles at most one {:.1}s window; \
                 use AsrRuntime::transcribe/transcribe_file for long audio (padded-window contract)",
                audio.len() as f32 / 16_000.0,
                self.win_samples as f32 / 16_000.0,
            ));
        }
        let mut hits: Vec<(usize, f32, usize)> = Vec::new();
        self.decode_window(audio, max_symbols, 0.0, &mut hits)?;
        Ok(hits)
    }
}
