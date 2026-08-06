/// One active beam hypothesis during decode. Carries everything needed to
/// resume decoding independently of every other hypothesis: its own LSTM
/// predictor state, its own TDT time index (hypotheses can and do sit at
/// different frames — see `tdt_step`), and its own emitted-token history
/// (used both for final output and for evaluating context-bias matches).
#[derive(Clone, Debug)]
struct BeamHyp {
    h: Vec<f32>,
    c: Vec<f32>,
    last_tok: i32,
    /// Encoder frame index this hypothesis will read next.
    t: usize,
    /// Symbols already emitted at frame `t` (TDT/RNN-T `max_symbols` cap).
    sym: usize,
    /// Cumulative log-probability (+ context-bias bonuses) of this
    /// hypothesis's token choices so far. Log-softmax based so scores stay
    /// comparable across hypotheses that may sit at different `t`/`sym`
    /// depths. At beam width 1 there is only ever one hypothesis, so this
    /// value never influences which token is chosen — see `decode_window`'s
    /// doc comment for why that keeps width 1 bit-identical to the original
    /// greedy decode.
    score: f32,
    hits: Vec<(usize, f32, usize)>,
}

/// Numerically-stable log-softmax over raw logits.
fn log_softmax(logits: &[f32]) -> Vec<f32> {
    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let sum_exp: f32 = logits.iter().map(|&l| (l - max).exp()).sum();
    let log_sum_exp = max + sum_exp.ln();
    logits.iter().map(|&l| l - log_sum_exp).collect()
}

/// Rank candidate token indices by `logit + context-bias bonus`, descending,
/// returning the top `k`. Ranks on the RAW logit (not a log-softmax
/// transform) plus the bonus, which is exactly the arithmetic the old
/// `argmax`/`biased_argmax` used — so `rank_top_k(_, _, 1)` always picks the
/// same index those functions did. Ties keep the FIRST (lowest) index: the
/// sort is stable and candidates are enumerated in ascending index order, so
/// a tied group keeps its original relative order — same convention as
/// `argmax`'s strict `>` left-to-right scan (a later equal value never
/// displaces an earlier one).
fn rank_top_k(
    logits: &[f32],
    bonuses: &std::collections::HashMap<usize, f32>,
    k: usize,
) -> Vec<usize> {
    let mut scored: Vec<(usize, f32)> = logits
        .iter()
        .enumerate()
        .map(|(i, &l)| (i, l + bonuses.get(&i).copied().unwrap_or(0.0)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(k.max(1)).map(|(i, _)| i).collect()
}

/// Force a hypothesis stuck at its `max_symbols` cap to advance one frame
/// without another decoder/joint call — mirrors the old greedy loop's
/// top-of-loop `if sym >= max_symbols { t += 1; break; }` guard, which never
/// re-queries the model for a step it's about to discard.
fn force_time_advance(hyp: &BeamHyp) -> BeamHyp {
    let mut nh = hyp.clone();
    nh.t += 1;
    nh.sym = 0;
    nh
}

/// Expand one hypothesis's TDT joint output into candidate successors (up to
/// `width`, one per top-ranked next token). Pure — no CoreML calls — so this
/// is the "scoring core" the width-1 equivalence tests below drive directly
/// with synthetic logits; `decode_window` calls the exact same function.
///
/// `new_h`/`new_c` are the decoder's updated LSTM state after consuming
/// `hyp.last_tok` — every candidate that stems from this step's joint output
/// shares this same state, because the predictor network's next state
/// depends only on the token ALREADY consumed (`hyp.last_tok`), never on
/// which token comes next. That's what makes it correct to spend exactly one
/// decoder/joint call per active hypothesis per step, regardless of `width`.
#[allow(clippy::too_many_arguments)]
fn tdt_step(
    hyp: &BeamHyp,
    logits: &[f32],
    nd: usize,
    durations: &[i64],
    blank: i32,
    max_symbols: usize,
    width: usize,
    win_start_sec: f32,
    bonuses: &std::collections::HashMap<usize, f32>,
    new_h: &[f32],
    new_c: &[f32],
) -> Vec<BeamHyp> {
    let tok_logits = &logits[..logits.len() - nd];
    let dur_logits = &logits[logits.len() - nd..];
    // Duration is never biased and never beamed — it's the acoustic timing
    // prediction for THIS step's joint output, independent of which token
    // candidate a hypothesis picks, identical to the pre-beam decode.
    let dur = durations[argmax(dur_logits)] as usize;
    let log_probs = log_softmax(tok_logits);
    rank_top_k(tok_logits, bonuses, width)
        .into_iter()
        .map(|tok| {
            let bonus = bonuses.get(&tok).copied().unwrap_or(0.0);
            let mut nh = hyp.clone();
            nh.score += log_probs[tok] + bonus;
            if tok as i32 != blank {
                nh.hits
                    .push((tok, win_start_sec + hyp.t as f32 * 0.08, dur));
                nh.last_tok = tok as i32;
                nh.h = new_h.to_vec();
                nh.c = new_c.to_vec();
                nh.sym += 1;
                if dur == 0 && nh.sym < max_symbols {
                    // Another symbol is emitted at this same frame — stay put.
                } else {
                    nh.t += dur.max(1);
                    nh.sym = 0;
                }
            } else {
                // Blank: no emission, predictor state does NOT advance (last_tok/h/c
                // stay as they entered this step) — only the time index moves.
                nh.t += dur.max(1);
                nh.sym = 0;
            }
            nh
        })
        .collect()
}

/// Expand one hypothesis's plain RNN-T joint output (no duration head) into
/// candidate successors. Same shape as `tdt_step`, minus duration handling:
/// blank advances one frame with no state update; a non-blank token emits
/// and chains at the SAME frame until blank or the `max_symbols` cap forces
/// (via `force_time_advance`, driven by the caller) an advance.
#[allow(clippy::too_many_arguments)]
fn rnnt_step(
    hyp: &BeamHyp,
    logits: &[f32],
    blank: i32,
    width: usize,
    win_start_sec: f32,
    bonuses: &std::collections::HashMap<usize, f32>,
    new_h: &[f32],
    new_c: &[f32],
) -> Vec<BeamHyp> {
    let log_probs = log_softmax(logits);
    rank_top_k(logits, bonuses, width)
        .into_iter()
        .map(|tok| {
            let bonus = bonuses.get(&tok).copied().unwrap_or(0.0);
            let mut nh = hyp.clone();
            nh.score += log_probs[tok] + bonus;
            if tok as i32 == blank {
                nh.t += 1;
                nh.sym = 0;
            } else {
                nh.hits.push((tok, win_start_sec + hyp.t as f32 * 0.08, 1));
                nh.last_tok = tok as i32;
                nh.h = new_h.to_vec();
                nh.c = new_c.to_vec();
                nh.sym += 1;
            }
            nh
        })
        .collect()
}

impl CoreMlParakeet {
    /// One window: mel → encoder → beam-search TDT/RNN-T decode, one LSTM
    /// state per hypothesis. Pushes (piece_id, absolute_start_sec,
    /// duration_frames) per emitted token onto `hits`, taken from the single
    /// highest-scoring COMPLETE hypothesis. TDT predicts the duration; `0`
    /// frames is legal and means "another token is emitted at this same
    /// frame" -- the last token of such a chain carries the real advance.
    /// RNNT has no duration head, so its tokens are 1 frame.
    ///
    /// Beam width is `self.beam_width` (default 1; see `set_beam_width` /
    /// `HR_ASR_BEAM_WIDTH`). AT WIDTH 1 THIS IS BIT-IDENTICAL TO THE
    /// PRE-BEAM GREEDY DECODE: `rank_top_k` ranks on the raw
    /// `logit + bias-bonus` sum — the exact arithmetic the retired
    /// `argmax`/`biased_argmax` used — never on the log-softmax `score`
    /// field, which only ranks ALTERNATIVE hypotheses against each other
    /// and is a complete no-op when there is only one hypothesis in the
    /// beam (nothing to compare it to). Every branch below (dur==0 chaining,
    /// the max_symbols cap, blank vs non-blank state advance) mirrors the
    /// old single-hypothesis control flow step for step; see the
    /// `beam_width_one_matches_greedy_*` tests in this file for the proof —
    /// this fn calls real CoreML models, so it can't be unit-tested
    /// directly, and per the task's acceptance bar the "scoring core"
    /// (`tdt_step`/`rnnt_step`/`rank_top_k`/`log_softmax`) is factored out
    /// pure and exercised there with synthetic logits instead.
    ///
    /// Context bias applies at the HYPOTHESIS level: `bonuses` is
    /// recomputed per hypothesis from ITS OWN emitted-token history each
    /// step, so a hypothesis that has partially matched a multi-token bias
    /// phrase keeps earning bonuses as it extends the match — letting a
    /// beam width > 1 keep a complete two-token phrase alive long enough to
    /// outscore a locally-better single-token guess, which a per-token
    /// nudge on a single greedy path can never do.
    fn decode_window(
        &self,
        seg: &[f32],
        max_symbols: usize,
        win_start_sec: f32,
        hits: &mut Vec<(usize, f32, usize)>,
    ) -> Result<(), String> {
        window_health::clear_latest_asr_window_stats();
        let m = &self.meta;
        let pl = m.pred_layers;
        let ph = m.pred_hidden;
        let blank = m.blank_id;
        let nd = m.durations.len();
        let duration_head_len = if m.is_tdt { nd } else { 0 };
        let width = self.beam_width.max(1);
        let detailed = crate::settings::asr_detailed_telemetry();
        let collect_diagnostics = detailed || crate::settings::onboarding_calibration_active();
        let decode_started = collect_diagnostics.then(std::time::Instant::now);
        let mut joint_calls = 0usize;
        let mut blank_steps = 0usize;
        let mut forced_advances = 0usize;
        let mut duration_sum_frames = 0usize;
        let mut duration_max_frames = 0usize;
        let mut duration_frames_skipped = 0usize;
        let mut duration_count = 0usize;
        let mut margin_sum = 0.0f64;
        let mut margin_count = 0usize;
        let mut near_tie_count = 0usize;
        let mut bias_applied_count = 0usize;
        let bias_installed_count = collect_diagnostics
            .then(|| {
                self.bias
                    .as_ref()
                    .filter(|bias| !bias.is_empty())
                    .map(|bias| bias.phrase_token_ids.len())
                    .unwrap_or(0)
            })
            .unwrap_or(0);

        // --- mel: pad audio to the static window, run, slice to mel_frames ---
        let mut padded = vec![0f32; self.win_samples];
        padded[..seg.len()].copy_from_slice(seg);
        let mel_full = autoreleasepool(|_| -> Result<Vec<f32>, String> {
            let audio_in = ml_f16(&[1, self.win_samples], &padded)?;
            let len_in = ml_i32(&[1], &[seg.len() as i32])?;
            let out = self.mel.predict(
                &[("audio", &audio_in), ("audio_len", &len_in)],
                &self.mel_out,
            )?;
            Ok(read_f16(&out))
        })?;
        let dbg = std::env::var("HR_COREML_DEBUG").is_ok();
        if dbg {
            let (mn, mx, sm) = stats(&mel_full);
            eprintln!(
                "[dbg] seg={} mel_full.len={} min={mn:.3} max={mx:.3} mean={:.3}",
                seg.len(),
                mel_full.len(),
                sm / mel_full.len() as f32
            );
        }
        // mel_full shape [1, n_mel, mel_frames+?]; slice to [n_mel, mel_frames]
        let mel_t_full = mel_full.len() / m.n_mel;
        let mut mel = vec![0f32; m.n_mel * m.mel_frames];
        for c in 0..m.n_mel {
            for t in 0..m.mel_frames {
                mel[c * m.mel_frames + t] = mel_full[c * mel_t_full + t];
            }
        }

        // --- encoder: [1, n_mel, mel_frames] -> [1, enc_d, enc_T] ---
        let enc_flat = autoreleasepool(|_| -> Result<Vec<f32>, String> {
            let mel_in = ml_f16(&[1, m.n_mel, m.mel_frames], &mel)?;
            let len_in = ml_i32(&[1], &[m.mel_frames as i32])?;
            let out = self.enc.predict(
                &[("melspectrogram_features", &mel_in), ("mel_len", &len_in)],
                &self.enc_out,
            )?;
            Ok(read_f16(&out))
        })?;
        // enc_flat is [enc_d, enc_T] row-major (batch 1 dropped). Transpose to (T, enc_d).
        let enc_t = enc_flat.len() / 1024; // enc_d_model = 1024
        let enc_d = 1024usize;
        let encoded = |t: usize| -> Vec<f32> {
            let mut v = vec![0f32; enc_d];
            for d in 0..enc_d {
                v[d] = enc_flat[d * enc_t + t];
            }
            v
        };

        // length clamp: only decode frames covering REAL audio (not silence pad).
        let nframes = enc_t.min(((seg.len() as f32) / 160.0 / 8.0).ceil() as usize);
        if dbg {
            let (mn, mx, sm) = stats(&enc_flat);
            eprintln!(
                "[dbg] enc_flat.len={} enc_t={enc_t} nframes={nframes} min={mn:.3} max={mx:.3} mean={:.3}",
                enc_flat.len(),
                sm / enc_flat.len() as f32
            );
        }

        // fresh decoder state per window, one hypothesis to start
        let mut beam: Vec<BeamHyp> = vec![BeamHyp {
            h: vec![0f32; pl * ph],
            c: vec![0f32; pl * ph],
            last_tok: blank,
            t: 0,
            sym: 0,
            score: 0.0,
            hits: Vec::new(),
        }];

        // Defensive round cap: every round advances some hypothesis's `t` or
        // `sym`, both bounded (`nframes`, `max_symbols`), so this can never
        // legitimately trigger — it exists so a logic bug degrades to a
        // truncated transcript instead of an infinite loop.
        let max_rounds = nframes.saturating_add(1) * max_symbols.max(1) + 4;
        let mut rounds = 0usize;
        while beam.iter().any(|hyp| hyp.t < nframes) {
            rounds += 1;
            if rounds > max_rounds {
                break;
            }
            let mut next: Vec<BeamHyp> = Vec::with_capacity(beam.len() * width);
            for hyp in beam.drain(..) {
                if hyp.t >= nframes {
                    next.push(hyp);
                    continue;
                }
                if hyp.sym >= max_symbols {
                    if collect_diagnostics {
                        forced_advances += 1;
                    }
                    next.push(force_time_advance(&hyp));
                    continue;
                }
                let enc_step = encoded(hyp.t);
                let (g, new_h, new_c, logits) = self.predict_decoder_joint(
                    hyp.last_tok,
                    &hyp.h,
                    &hyp.c,
                    &enc_step,
                    pl,
                    ph,
                    enc_d,
                )?;
                if collect_diagnostics {
                    joint_calls += 1;
                }
                let emitted: Vec<usize> = hyp.hits.iter().map(|entry| entry.0).collect();
                let bonuses = self
                    .bias
                    .as_ref()
                    .map(|b| b.next_token_bonuses(&emitted))
                    .unwrap_or_default();
                if collect_diagnostics {
                    bias_applied_count += usize::from(!bonuses.is_empty());
                }
                let token_logits = &logits[..logits.len().saturating_sub(duration_head_len)];
                if collect_diagnostics {
                    if rank_top_k(token_logits, &bonuses, 1).first() == Some(&(blank as usize)) {
                        blank_steps += 1;
                    }
                    if let Some(margin) = window_health::top1_top2_margin(token_logits, &bonuses) {
                        margin_sum += f64::from(margin);
                        margin_count += 1;
                        if margin <= 0.1 {
                            near_tie_count += 1;
                        }
                    }
                }
                if collect_diagnostics && m.is_tdt && nd > 0 {
                    let duration_index = argmax(&logits[logits.len() - nd..]);
                    let duration = m.durations[duration_index].max(0) as usize;
                    duration_sum_frames = duration_sum_frames.saturating_add(duration);
                    duration_max_frames = duration_max_frames.max(duration);
                    duration_frames_skipped =
                        duration_frames_skipped.saturating_add(duration.saturating_sub(1));
                    duration_count += 1;
                }

                let children = if m.is_tdt {
                    tdt_step(
                        &hyp,
                        &logits,
                        nd,
                        &m.durations,
                        blank,
                        max_symbols,
                        width,
                        win_start_sec,
                        &bonuses,
                        &new_h,
                        &new_c,
                    )
                } else {
                    rnnt_step(
                        &hyp,
                        &logits,
                        blank,
                        width,
                        win_start_sec,
                        &bonuses,
                        &new_h,
                        &new_c,
                    )
                };
                if dbg && hyp.t < 3 {
                    if let Some(top) = children.first() {
                        eprintln!(
                            "[dbg] t={} sym={} blank={blank} tok={:?} score={:.3} g0={:.3}",
                            hyp.t,
                            hyp.sym,
                            top.hits.last().map(|h| h.0),
                            top.score,
                            g.first().copied().unwrap_or(0.0)
                        );
                    }
                }
                next.extend(children);
            }
            next.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            next.truncate(width);
            beam = next;
        }

        beam.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if let Some(best) = beam.into_iter().next() {
            if collect_diagnostics {
                let mel_health = window_health::tensor_health(&mel_full);
                let encoder_health = window_health::tensor_health(&enc_flat);
                let stats = AsrWindowStats {
                    frames_total: nframes,
                    joint_calls,
                    emitted_tokens: best.hits.len(),
                    blank_steps,
                    forced_advances,
                    duration_mean_frames: if duration_count == 0 {
                        0.0
                    } else {
                        duration_sum_frames as f32 / duration_count as f32
                    },
                    duration_max_frames,
                    duration_frames_skipped,
                    no_emission_gaps_over_2s: window_health::no_emission_gaps_over_2s(
                        &best.hits,
                        win_start_sec,
                        nframes,
                    ),
                    top1_top2_margin_mean: if margin_count == 0 {
                        0.0
                    } else {
                        (margin_sum / margin_count as f64) as f32
                    },
                    near_tie_count,
                    bias_installed_count,
                    bias_applied_count,
                    conditioned_audio_rms_quarters: window_health::quarter_rms(seg),
                    mel_rms: mel_health.rms,
                    mel_min: mel_health.min,
                    mel_max: mel_health.max,
                    mel_nonfinite: mel_health.nonfinite,
                    mel_quarters: window_health::quartered_tensor_health(&mel_full),
                    encoder_rms: encoder_health.rms,
                    encoder_min: encoder_health.min,
                    encoder_max: encoder_health.max,
                    encoder_nonfinite: encoder_health.nonfinite,
                    encoder_quarters: window_health::quartered_tensor_health(&enc_flat),
                };
                window_health::publish_asr_window_stats(stats.clone());
                window_health::publish_calibration_window_stats(stats.clone(), win_start_sec);
                if detailed {
                    let elapsed_ms = decode_started
                        .expect("diagnostic timer exists when detailed telemetry is enabled")
                        .elapsed()
                        .as_millis() as u64;
                    let envelope = window_health::AsrWindowStatsEnvelope::new(
                        &stats,
                        &self.model_fingerprint,
                        self.model_dir_sha256.as_deref(),
                        &self.coreml_compile_cache,
                        self.mel.compute_profile(),
                        self.joint.compute_profile(),
                        &self.encoder_compute,
                        &self.decoder_compute,
                        elapsed_ms,
                    );
                    if let Ok(stats_json) = serde_json::to_string(&envelope) {
                        eprintln!("HR_ASR_WINDOW_STATS_JSON={stats_json}");
                    }
                    tracing::info!(
                        event = "asr_window_stats",
                        stats = ?stats,
                        upstream_inputs_needed = ?ASR_WINDOW_STATS_UPSTREAM_INPUTS_NEEDED,
                        encoder_compute = self.encoder_compute,
                        decoder_compute = self.decoder_compute,
                        elapsed_ms,
                        "asr_window_stats"
                    );
                }
            }
            hits.extend(best.hits);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn empty_bonuses() -> HashMap<usize, f32> {
        HashMap::new()
    }

    /// Drive `tdt_step`/`force_time_advance` for a single (width=1)
    /// hypothesis against a scripted sequence of joint outputs, mirroring
    /// exactly how `decode_window`'s round loop calls them. Returns the
    /// final hypothesis so callers can assert on `hits`/`t`/`sym`.
    fn run_width_one_tdt(
        script: &[Vec<f32>],
        nd: usize,
        durations: &[i64],
        blank: i32,
        max_symbols: usize,
        nframes: usize,
        bonuses_for_step: impl Fn(&[usize]) -> HashMap<usize, f32>,
    ) -> BeamHyp {
        let mut hyp = BeamHyp {
            h: vec![0.0],
            c: vec![0.0],
            last_tok: blank,
            t: 0,
            sym: 0,
            score: 0.0,
            hits: Vec::new(),
        };
        let mut call = 0usize;
        while hyp.t < nframes {
            if hyp.sym >= max_symbols {
                hyp = force_time_advance(&hyp);
                continue;
            }
            let logits = &script[call];
            call += 1;
            // Distinguishable, deterministic "new state" so tests can also
            // assert the predictor state actually advances on emission.
            let new_h = vec![hyp.h[0] + 1.0];
            let new_c = vec![hyp.c[0] + 1.0];
            let emitted: Vec<usize> = hyp.hits.iter().map(|e| e.0).collect();
            let bonuses = bonuses_for_step(&emitted);
            let children = tdt_step(
                &hyp,
                logits,
                nd,
                durations,
                blank,
                max_symbols,
                1,
                0.0,
                &bonuses,
                &new_h,
                &new_c,
            );
            assert_eq!(
                children.len(),
                1,
                "width 1 must always produce exactly one child"
            );
            hyp = children.into_iter().next().unwrap();
        }
        hyp
    }

    /// The core acceptance-bar test: a scripted multi-step TDT decode
    /// exercising every branch of the old greedy control flow — a dur==0
    /// same-frame chain, a normal dur>0 emission, a blank frame-advance, and
    /// a final emission — run through the new beam machinery at width 1.
    /// The expected hits below were hand-derived from the OLD decode_window
    /// semantics (argmax over tok logits, argmax over duration logits,
    /// dur==0 chains at the same frame, blank leaves state untouched and
    /// advances by its own predicted duration) applied to this exact script.
    #[test]
    fn beam_width_one_matches_greedy_tdt_chain_and_blank() {
        let blank = 5i32;
        let durations = vec![0i64, 1i64];
        let nd = durations.len();
        let max_symbols = 3usize;
        let bonuses = empty_bonuses();
        // Every call's candidate state is `current h + 1.0` — a fresh call
        // computed from whatever `hyp.h` is when it runs, so a call whose
        // result never gets applied (blank) leaves the NEXT call's base
        // value unchanged. Driving this by hand (rather than through
        // `run_width_one_tdt`) lets us assert on `hyp.h` right after the
        // blank round, before it gets overwritten by the next emission.
        let mut hyp = BeamHyp {
            h: vec![0.0],
            c: vec![0.0],
            last_tok: blank,
            t: 0,
            sym: 0,
            score: 0.0,
            hits: Vec::new(),
        };

        // Round 1 @ t=0 sym=0: tok=2, dur idx0 -> 0 frames (chain at t=0).
        let step = vec![0.1, 0.2, 5.0, 0.3, 0.4, 0.05, /*dur*/ 9.0, 0.1];
        let new_h = vec![hyp.h[0] + 1.0];
        let new_c = vec![hyp.c[0] + 1.0];
        hyp = tdt_step(
            &hyp,
            &step,
            nd,
            &durations,
            blank,
            max_symbols,
            1,
            0.0,
            &bonuses,
            &new_h,
            &new_c,
        )
        .into_iter()
        .next()
        .unwrap();
        assert_eq!((hyp.t, hyp.sym, hyp.h[0]), (0, 1, 1.0));
        assert_eq!(hyp.hits, vec![(2, 0.0, 0)]);

        // Round 2 @ t=0 sym=1: tok=1, dur idx1 -> 1 frame (advance to t=1).
        let step = vec![0.1, 5.0, 0.2, 0.3, 0.4, 0.05, /*dur*/ 0.1, 9.0];
        let new_h = vec![hyp.h[0] + 1.0];
        let new_c = vec![hyp.c[0] + 1.0];
        hyp = tdt_step(
            &hyp,
            &step,
            nd,
            &durations,
            blank,
            max_symbols,
            1,
            0.0,
            &bonuses,
            &new_h,
            &new_c,
        )
        .into_iter()
        .next()
        .unwrap();
        assert_eq!((hyp.t, hyp.sym, hyp.h[0]), (1, 0, 2.0));
        assert_eq!(hyp.hits, vec![(2, 0.0, 0), (1, 0.0, 1)]);

        // Round 3 @ t=1 sym=0: blank, dur idx1 -> 1 frame (advance to t=2,
        // predictor state must stay at 2.0, NOT jump to this round's 3.0
        // candidate).
        let step = vec![0.1, 0.2, 0.3, 0.4, 0.5, 9.0, /*dur*/ 0.1, 9.0];
        let new_h = vec![hyp.h[0] + 1.0]; // candidate 3.0 — must be discarded
        let new_c = vec![hyp.c[0] + 1.0];
        hyp = tdt_step(
            &hyp,
            &step,
            nd,
            &durations,
            blank,
            max_symbols,
            1,
            0.0,
            &bonuses,
            &new_h,
            &new_c,
        )
        .into_iter()
        .next()
        .unwrap();
        assert_eq!(
            (hyp.t, hyp.sym, hyp.h[0]),
            (2, 0, 2.0),
            "blank must not advance predictor state"
        );
        assert_eq!(hyp.hits.len(), 2, "blank emits nothing");

        // Round 4 @ t=2 sym=0: tok=3, dur idx1 -> 1 frame (advance to t=3, done).
        let step = vec![0.1, 0.2, 0.3, 9.0, 0.4, 0.05, /*dur*/ 0.1, 9.0];
        let new_h = vec![hyp.h[0] + 1.0];
        let new_c = vec![hyp.c[0] + 1.0];
        hyp = tdt_step(
            &hyp,
            &step,
            nd,
            &durations,
            blank,
            max_symbols,
            1,
            0.0,
            &bonuses,
            &new_h,
            &new_c,
        )
        .into_iter()
        .next()
        .unwrap();
        assert_eq!(hyp.t, 3);

        let expected = [(2usize, 0.0f32, 0usize), (1, 0.0, 1), (3, 0.16, 1)];
        assert_eq!(hyp.hits.len(), expected.len());
        for (got, want) in hyp.hits.iter().zip(expected.iter()) {
            assert_eq!(got.0, want.0, "token id mismatch");
            assert!(
                (got.1 - want.1).abs() < 1e-6,
                "timestamp mismatch: {} vs {}",
                got.1,
                want.1
            );
            assert_eq!(got.2, want.2, "duration-frames mismatch");
        }
    }

    /// TDT's `dur==0 but sym has just reached the cap` fallthrough: the old
    /// code emits the token, then advances time by `dur.max(1)` anyway
    /// (rather than chaining) once `sym` would no longer be `< max_symbols`.
    /// (The separate top-of-loop `sym >= max_symbols` guard is dead code on
    /// the TDT path in both the old and new implementations — every TDT
    /// continuation already resets `sym` to 0 whenever it advances `t`, so a
    /// hypothesis can never re-enter a round with a stale `sym` at the cap;
    /// that guard's only live use is the plain RNN-T path, covered by
    /// `rnnt_max_symbols_cap_forces_advance_without_extra_call` below.)
    #[test]
    fn beam_width_one_respects_max_symbols_cap() {
        let blank = 3i32;
        let durations = vec![0i64]; // every step predicts dur=0 (would chain forever without the cap)
        let nd = durations.len();
        let max_symbols = 2usize;
        let nframes = 2usize;

        // Two calls at t=0 (cap reached at sym=2), forced advance to t=1,
        // then two more calls at t=1.
        let script: Vec<Vec<f32>> = vec![
            vec![5.0, 0.1, 0.2, 0.05, /*dur*/ 9.0],
            vec![0.1, 5.0, 0.2, 0.05, /*dur*/ 9.0],
            vec![0.1, 0.2, 5.0, 0.05, /*dur*/ 9.0],
            vec![5.0, 0.1, 0.2, 0.05, /*dur*/ 9.0],
        ];

        let hyp = run_width_one_tdt(
            &script,
            nd,
            &durations,
            blank,
            max_symbols,
            nframes,
            |_emitted| empty_bonuses(),
        );

        // t=0: tok0 (dur=0,sym=1<2 chain), tok1 (dur=0, sym=2 -> NOT <2, so
        // falls through to t+=dur.max(1)=1 forced advance) -- matches old
        // "dur==0 but sym>=max_symbols" fallthrough, not the cap-guard path.
        // t=1: tok2 (dur=0,sym=1<2 chain), tok0 (dur=0,sym=2 -> advance to t=2, done).
        let expected_ids = [0usize, 1, 2, 0];
        let got_ids: Vec<usize> = hyp.hits.iter().map(|h| h.0).collect();
        assert_eq!(got_ids, expected_ids);
    }

    /// Context bias must apply at the hypothesis level and change the
    /// width-1 pick exactly like the retired `biased_argmax` did — proving
    /// `rank_top_k` + bonus reproduces that arithmetic bit for bit.
    #[test]
    fn beam_width_one_applies_context_bias() {
        let blank = 3i32;
        let durations = vec![1i64];
        let nd = durations.len();
        let max_symbols = 2usize;
        let nframes = 1usize;
        // Unbiased argmax would pick token 0 (5.0 beats 4.9), but token 1
        // gets a +0.2 bias bonus, which must flip the pick to token 1.
        let script: Vec<Vec<f32>> = vec![vec![5.0, 4.9, 0.1, 0.05, /*dur*/ 9.0]];

        let mut bonus = HashMap::new();
        bonus.insert(1usize, 0.2f32);
        let hyp = run_width_one_tdt(
            &script,
            nd,
            &durations,
            blank,
            max_symbols,
            nframes,
            move |_emitted| bonus.clone(),
        );

        assert_eq!(
            hyp.hits.first().map(|h| h.0),
            Some(1),
            "bias bonus must flip the pick"
        );
    }

    /// Beam width > 1: a hypothesis-level bias should let a two-token
    /// bias-phrase completion outscore a locally-better single-token guess
    /// once both tokens are accounted for — the property a per-token nudge
    /// on a single greedy path can never reproduce.
    #[test]
    fn beam_width_two_lets_bias_phrase_win_over_two_steps() {
        let blank = 4i32;
        let durations = vec![1i64];
        let nd = durations.len();
        let max_symbols = 2usize;
        let width = 2usize;

        // Phrase tokens: [1, 2] gets bonus 3.0 for the next expected token.
        // Step 1 (t=0): token 0 slightly beats token 1 on raw logits alone
        // (5.0 vs 4.8), but token 1 is the start of the bias phrase.
        let step1 = vec![5.0, 4.8, 0.1, 0.1, /*dur*/ 9.0];
        // Step 2, expanding the "token 0" hypothesis (phrase not started):
        // no bonus applies; best available continuation is token 3.
        let step2_from_0 = vec![0.1, 0.1, 0.1, 6.0, /*dur*/ 9.0];
        // Step 2, expanding the "token 1" hypothesis (phrase prefix [1]
        // matched): token 2 gets +3.0, flipping what would otherwise lose.
        let step2_from_1 = vec![0.1, 0.1, 5.9, 0.1, /*dur*/ 9.0];

        let phrase = vec![1usize, 2usize];
        let bonus_for = |emitted: &[usize]| -> HashMap<usize, f32> {
            let matched = if emitted.last() == Some(&1) { 1 } else { 0 };
            let mut m = HashMap::new();
            if matched < phrase.len() {
                m.insert(phrase[matched], 3.0);
            }
            m
        };

        // Round 1: single hypothesis, expand with width=2 -> top-2 tokens by
        // raw logit are {0: 5.0, 1: 4.8+3.0=7.8} once the bias is applied
        // (bias applies from the start since emitted=[] hasn't matched yet,
        // so token 1 — phrase[0] — already gets +3.0 at step 1).
        let root = BeamHyp {
            h: vec![0.0],
            c: vec![0.0],
            last_tok: blank,
            t: 0,
            sym: 0,
            score: 0.0,
            hits: Vec::new(),
        };
        let bonuses1 = bonus_for(&[]);
        let round1 = tdt_step(
            &root,
            &step1,
            nd,
            &durations,
            blank,
            max_symbols,
            width,
            0.0,
            &bonuses1,
            &[1.0],
            &[1.0],
        );
        assert_eq!(round1.len(), 2);
        // token 1 (phrase start, boosted to 7.8) must outrank token 0 (5.0).
        assert_eq!(round1[0].hits.last().map(|h| h.0), Some(1));
        assert_eq!(round1[1].hits.last().map(|h| h.0), Some(0));

        // Round 2: expand BOTH surviving hypotheses, then prune to width=2
        // by cumulative score exactly like decode_window does.
        let hyp_from_1 = &round1[0];
        let hyp_from_0 = &round1[1];
        let bonuses_from_1 = bonus_for(&[1]);
        let bonuses_from_0 = bonus_for(&[0]);
        let mut next = Vec::new();
        next.extend(tdt_step(
            hyp_from_1,
            &step2_from_1,
            nd,
            &durations,
            blank,
            max_symbols,
            width,
            0.0,
            &bonuses_from_1,
            &[2.0],
            &[2.0],
        ));
        next.extend(tdt_step(
            hyp_from_0,
            &step2_from_0,
            nd,
            &durations,
            blank,
            max_symbols,
            width,
            0.0,
            &bonuses_from_0,
            &[2.0],
            &[2.0],
        ));
        next.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        next.truncate(width);

        let winner = &next[0];
        assert_eq!(
            winner.hits.iter().map(|h| h.0).collect::<Vec<_>>(),
            vec![1, 2],
            "the two-token bias phrase [1,2] must win the beam over two steps"
        );
    }

    /// `rank_top_k` tie-break: equal scores keep the lowest original index
    /// first, same convention as the retired `argmax`'s strict `>` scan.
    #[test]
    fn rank_top_k_ties_keep_lowest_index() {
        let logits = [1.0, 3.0, 3.0, 2.0, 3.0];
        let top = rank_top_k(&logits, &empty_bonuses(), 3);
        assert_eq!(
            top,
            vec![1, 2, 4],
            "tied scores must stay in ascending index order"
        );
    }

    /// `log_softmax` is a monotonic (order-preserving) transform of the raw
    /// logits, so ranking by it alone (no bias) always agrees with ranking
    /// the raw logits directly.
    #[test]
    fn log_softmax_preserves_ranking() {
        let logits = vec![0.5f32, 3.2, -1.0, 3.2, 2.9];
        let lp = log_softmax(&logits);
        let raw_rank = rank_top_k(&logits, &empty_bonuses(), logits.len());
        let mut lp_indexed: Vec<(usize, f32)> = lp.iter().copied().enumerate().collect();
        lp_indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let lp_rank: Vec<usize> = lp_indexed.into_iter().map(|(i, _)| i).collect();
        assert_eq!(raw_rank, lp_rank);
    }

    /// Plain RNN-T (no duration head): non-blank chains at the same frame,
    /// blank advances by exactly one frame with no state update.
    #[test]
    fn rnnt_step_chains_until_blank() {
        let blank = 2i32;
        let mut hyp = BeamHyp {
            h: vec![0.0],
            c: vec![0.0],
            last_tok: blank,
            t: 0,
            sym: 0,
            score: 0.0,
            hits: Vec::new(),
        };
        // Step 1: emit token 0 (chains, t stays 0).
        let step1 = vec![5.0, 0.1, 0.2];
        let children = rnnt_step(
            &hyp,
            &step1,
            blank,
            1,
            0.0,
            &empty_bonuses(),
            &[1.0],
            &[1.0],
        );
        hyp = children.into_iter().next().unwrap();
        assert_eq!(hyp.t, 0);
        assert_eq!(hyp.hits.last().map(|h| h.0), Some(0));

        // Step 2: emit blank (advances to t=1, state must NOT update).
        let step2 = vec![0.1, 0.2, 5.0];
        let children = rnnt_step(
            &hyp,
            &step2,
            blank,
            1,
            0.0,
            &empty_bonuses(),
            &[9.0],
            &[9.0],
        );
        hyp = children.into_iter().next().unwrap();
        assert_eq!(hyp.t, 1);
        assert_eq!(hyp.h[0], 1.0, "blank must not advance predictor state");
        assert_eq!(hyp.hits.len(), 1, "blank emits nothing");
    }

    /// Drive `rnnt_step`/`force_time_advance` for a single (width=1)
    /// hypothesis, mirroring `decode_window`'s round loop for the non-TDT
    /// branch (no duration head: non-blank never advances `t`, only `sym`).
    fn run_width_one_rnnt(
        script: &[Vec<f32>],
        blank: i32,
        max_symbols: usize,
        nframes: usize,
    ) -> BeamHyp {
        let mut hyp = BeamHyp {
            h: vec![0.0],
            c: vec![0.0],
            last_tok: blank,
            t: 0,
            sym: 0,
            score: 0.0,
            hits: Vec::new(),
        };
        let mut call = 0usize;
        while hyp.t < nframes {
            if hyp.sym >= max_symbols {
                hyp = force_time_advance(&hyp);
                continue;
            }
            let logits = &script[call];
            call += 1;
            let new_h = vec![hyp.h[0] + 1.0];
            let new_c = vec![hyp.c[0] + 1.0];
            let children = rnnt_step(
                &hyp,
                logits,
                blank,
                1,
                0.0,
                &empty_bonuses(),
                &new_h,
                &new_c,
            );
            assert_eq!(children.len(), 1);
            hyp = children.into_iter().next().unwrap();
        }
        hyp
    }

    /// Plain RNN-T has no per-step reset of `sym`, so its `max_symbols` cap
    /// is enforced ENTIRELY by the top-of-round `sym >= max_symbols` guard
    /// (`force_time_advance`), which must fire BEFORE spending another
    /// decoder/joint call — proven here by giving the script exactly as
    /// many entries as calls that should actually happen: if the guard
    /// failed to intercept, the driver would index past the end of `script`
    /// and panic.
    #[test]
    fn rnnt_max_symbols_cap_forces_advance_without_extra_call() {
        let blank = 2i32;
        let max_symbols = 2usize;
        let nframes = 2usize;
        let script: Vec<Vec<f32>> = vec![
            vec![5.0, 0.1, 0.2], // t=0 sym=0: tok0 -> sym=1
            vec![0.1, 5.0, 0.2], // t=0 sym=1: tok1 -> sym=2 == cap
            // cap guard must fire here WITHOUT consuming a 3rd script entry
            // at t=0 -- this entry is only reached at t=1, sym=0.
            vec![0.2, 0.1, 5.0], // t=1 sym=0: blank -> t=2, done
        ];
        let hyp = run_width_one_rnnt(&script, blank, max_symbols, nframes);
        assert_eq!(hyp.hits.iter().map(|h| h.0).collect::<Vec<_>>(), vec![0, 1]);
        assert_eq!(hyp.t, 2);
    }
}
