fn quiet_cut_in(samples: &[f32], lo: usize, hi: usize) -> Option<usize> {
    let win = ms_to_samples(PADDED_WINDOW_SILENCE_MS);
    let hop = ms_to_samples(PADDED_WINDOW_SILENCE_HOP_MS);
    if hi <= lo || hi.saturating_sub(lo) < win {
        return None;
    }
    let mut best = None;
    let mut i = lo;
    while i + win <= hi {
        let value = rms(&samples[i..i + win]);
        if best.map(|(_, best)| value < best).unwrap_or(true) {
            best = Some((i + win / 2, value));
        }
        i += hop;
    }
    best.map(|(cut, _)| cut)
}

/// Text-only padded-window decode, for backends whose decode returns a string
/// with no token timestamps (Whisper).
///
/// Same contract as `transcribe_padded_window` — see `docs/ASR_DECODE_CONTRACT.md`
/// — with the token bookkeeping dropped: decode `[start, cut)`, commit whatever
/// comes back, join seams with the shared overlap rule. `window_samples` is the
/// BACKEND's native window (15 s Parakeet, 30 s Whisper); the trailing search
/// region stays `PADDED_WINDOW_PADDING_MS` for both, since 2.24 s is already
/// ample to contain an inter-word pause and a larger region would waste more of
/// each window.
pub(crate) fn transcribe_padded_window_text<F>(
    samples: &[f32],
    window_samples: usize,
    mut decode: F,
) -> Result<String, String>
where
    F: FnMut(&[f32]) -> Result<String, String>,
{
    if samples.len() <= window_samples {
        return decode(samples);
    }

    let padding = ms_to_samples(PADDED_WINDOW_PADDING_MS);
    let target = window_samples.saturating_sub(padding);
    let mut text = String::new();
    let mut start = 0usize;

    while start < samples.len() {
        let window_end = (start + window_samples).min(samples.len());
        if window_end >= samples.len() {
            break;
        }
        let target_end = (start + target).min(samples.len());
        let cut = quiet_cut_in(samples, target_end, window_end).unwrap_or(window_end);
        let segment: &[f32] = if start < cut { &samples[start..cut] } else { &[] };
        append_with_overlap(&mut text, &decode(segment)?);
        start = cut.max(start + 1);
    }
    if start < samples.len() {
        append_with_overlap(&mut text, &decode(&samples[start..])?);
    }
    Ok(text.trim().to_string())
}

/// Join the next window's text onto `buffer`, de-duplicating the seam.
///
/// The `overlap == 0` branch matters: with disjoint windows (decode exactly to
/// the quiet cut) there is never a character overlap to consume, and a raw
/// `push_str` welds the last word of one window to the first of the next —
/// "AndersonCounty", "Star Trekhas". Each weld costs a deletion AND a
/// substitution, which measured as +1.24 WER points on the canonical corpus.
/// Swift's `appendWithOverlap` always had this separator; Rust did not.
fn append_with_overlap(buffer: &mut String, next: &str) {
    if next.is_empty() {
        return;
    }
    if buffer.is_empty() {
        buffer.push_str(next);
        return;
    }
    let overlap = suffix_prefix_overlap(buffer, next, PADDED_WINDOW_OVERLAP_CHARS);
    let remainder = &next[overlap..];
    if remainder.is_empty() {
        return;
    }
    // Only insert a separator when the seam would otherwise fuse two words.
    let needs_space = overlap == 0
        && !buffer.ends_with(char::is_whitespace)
        && !remainder.starts_with(char::is_whitespace);
    if needs_space {
        buffer.push(' ');
    }
    buffer.push_str(remainder);
}

fn suffix_prefix_overlap(a: &str, b: &str, max_chars: usize) -> usize {
    let max = a.len().min(b.len()).min(max_chars);
    let mut best = 0usize;
    for len in 1..=max {
        if !a.is_char_boundary(a.len() - len) || !b.is_char_boundary(len) {
            continue;
        }
        if a[a.len() - len..].eq_ignore_ascii_case(&b[..len]) {
            best = len;
        }
    }
    best
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

fn padded_window_samples() -> usize {
    ms_to_samples(PADDED_WINDOW_MS)
}

fn ms_to_samples(ms: u64) -> usize {
    (SAMPLE_RATE as u64)
        .saturating_mul(ms)
        .saturating_div(1_000) as usize
}

#[cfg(target_os = "macos")]
fn try_load_coreml_parakeet(models_dir: &Path) -> Result<Option<AsrRuntime>, String> {
    let backend = configured_backend();
    if !matches!(
        backend.as_str(),
        "parakeet-unified" | "parakeet" | "parakeet-rnnt" | "rnnt" | "parakeet-tdt"
    ) {
        return Ok(None);
    }

    let dir = coreml_parakeet_bundle_dir(models_dir);
    if !dir.join("pipeline.json").exists() {
        tracing::warn!("CoreML Parakeet bundle unavailable at {}", dir.display());
        return Ok(None);
    }

    let mut model = crate::coreml_asr::CoreMlParakeet::load(&dir)
        .map_err(|e| format!("coreml parakeet load ({}): {e}", dir.display()))?;
    apply_coreml_context_bias(&mut model);
    tracing::info!("CoreML Parakeet model loaded from {}", dir.display());
    Ok(Some(AsrRuntime::CoreMlParakeet(model)))
}

#[cfg(target_os = "macos")]
fn try_load_coreml_whisper(models_dir: &Path, lang: &str) -> Result<AsrRuntime, String> {
    let role = "whisper-multi";
    let dir = whisper_coreml_dir(models_dir);
    if !dir.join("AudioEncoder.mlmodelc").exists() || !dir.join("TextDecoder.mlmodelc").exists() {
        return Err(format!(
            "native whisper bundle missing role={role} dir={}",
            dir.display()
        ));
    }
    let engine = crate::whisper_coreml::WhisperCoreMl::load(&dir)?;
    let lang_tok = engine.lang_token(lang);
    tracing::info!(
        "Whisper CoreML model loaded role={} lang={} dir={}",
        role,
        lang,
        dir.display()
    );
    Ok(AsrRuntime::WhisperCoreMl { engine, lang_tok })
}

#[cfg(target_os = "windows")]
fn try_load_whisper_win(models_dir: &Path, lang: String) -> Result<AsrRuntime, String> {
    let engine = crate::whisper_win::WhisperWin::load(models_dir)?;
    tracing::info!("Windows Whisper model loaded lang={}", lang);
    Ok(AsrRuntime::WhisperWin { engine, lang })
}

#[cfg(target_os = "macos")]
fn coreml_parakeet_bundle_dir(models_dir: &Path) -> PathBuf {
    if let Some(raw) = std::env::var_os("HR_COREML_MODEL_DIR") {
        return PathBuf::from(raw);
    }
    let backend = configured_backend();
    if backend == "parakeet-tdt" {
        models_dir.join(COREML_TDT_MODEL_SUBDIR)
    } else {
        models_dir.join(COREML_UNIFIED_MODEL_SUBDIR)
    }
}

#[cfg(target_os = "macos")]
fn whisper_coreml_dir(models_dir: &Path) -> PathBuf {
    // Whisper is NOT bundled — it's a Pro on-demand download to app-data. The
    // supervisor sets HR_WHISPER_COREML_MODEL to a *bundled* Resources path that
    // doesn't exist for this case, so the on-disk downloaded copy MUST win first
    // (mirror of the dictation_language env precedence fix). Only if it's absent
    // do we honor the env override, then the bundled fallback.
    let downloaded = crate::settings::app_data_root()
        .join("models")
        .join("whisper-multi");
    if downloaded.join("AudioEncoder.mlmodelc").exists() {
        return downloaded;
    }
    if let Some(raw) = std::env::var_os("HR_WHISPER_COREML_MODEL") {
        return PathBuf::from(raw);
    }
    models_dir.join("coreml").join("whisper-multi")
}

fn configured_backend() -> String {
    std::env::var("HR_ASR_BACKEND")
        .ok()
        .or_else(|| Some(crate::settings::asr_backend()))
        .unwrap_or_else(|| heardright_core::settings::DEFAULT_ASR_BACKEND.to_string())
        .trim()
        .to_ascii_lowercase()
}

fn model_subdir_for(alias: &str) -> String {
    if let Some(raw) = std::env::var_os("HR_ASR_MODEL_SUBDIR") {
        let value = raw.to_string_lossy().trim().to_string();
        if !value.is_empty() {
            return value;
        }
    }
    model_subdir_for_alias(alias).to_string()
}

fn model_subdir_for_alias(alias: &str) -> &'static str {
    match alias.trim().to_ascii_lowercase().as_str() {
        "parakeet-unified" | "parakeet" | "parakeet-rnnt" | "rnnt" => UNIFIED_MODEL_SUBDIR,
        "parakeet-tdt" => TDT_MODEL_SUBDIR,
        _ => UNIFIED_MODEL_SUBDIR,
    }
}

/// Re-install decode-time bias for ONE utterance, folding in the current
/// screen-context harvest (screen_vocab). Called from the worker right before
/// each final decode. This is intentionally unconditional: command probes use
/// a stronger score on the same runtime, so hash-only caching could leave probe
/// bias installed for final dictation.
const DEFAULT_CONTEXT_BIAS_SCORE: f32 = 1.0;
const PROBE_CONTEXT_BIAS_SCORE: f32 = 5.0;

pub fn apply_utterance_bias(model: &mut AsrRuntime) {
    let Some(score) = context_bias_score() else {
        return;
    };
    let screen = crate::screen_vocab::current_terms();
    let mut terms = context_bias_terms();
    // Base terms outrank screen terms at the 512 ceiling; dedupe casefolded.
    let existing: std::collections::HashSet<String> =
        terms.iter().map(|t| t.to_lowercase()).collect();
    terms.extend(
        screen
            .into_iter()
            .filter(|t| !existing.contains(&t.to_lowercase())),
    );
    terms.truncate(512);
    let requested = terms.len();
    let installed = match model {
        AsrRuntime::Parakeet(m) => m.set_context_bias_phrases(terms.iter(), score),
        #[cfg(target_os = "macos")]
        AsrRuntime::CoreMlParakeet(m) => m.set_context_bias_phrases(terms.iter(), score),
        // Whisper lanes have no phrase-bias mechanism.
        #[allow(unreachable_patterns)]
        _ => return,
    };
    tracing::info!(
        "utterance bias refreshed: requested={} installed={} score={:.2}",
        requested,
        installed,
        score
    );
}

/// Give command/trigger probes maximum vocabulary bias. Final work restores its
/// default score through `apply_utterance_bias` before decoding.
pub fn apply_probe_context_bias(model: &mut AsrRuntime) {
    if context_bias_score().is_none() {
        return;
    }
    let terms = context_bias_terms();
    let requested = terms.len();
    let installed = match model {
        AsrRuntime::Parakeet(m) => {
            m.set_context_bias_phrases(terms.iter(), PROBE_CONTEXT_BIAS_SCORE)
        }
        #[cfg(target_os = "macos")]
        AsrRuntime::CoreMlParakeet(m) => {
            m.set_context_bias_phrases(terms.iter(), PROBE_CONTEXT_BIAS_SCORE)
        }
        #[allow(unreachable_patterns)]
        _ => return,
    };
    tracing::info!(
        "ASR probe context bias configured: requested={} installed={} score={:.2}",
        requested,
        installed,
        PROBE_CONTEXT_BIAS_SCORE
    );
}

fn apply_context_bias(model: &mut HrTransducer) {
    let Some(score) = context_bias_score() else {
        return;
    };
    let terms = context_bias_terms();
    let requested = terms.len();
    let installed = model.set_context_bias_phrases(terms.iter(), score);
    tracing::info!(
        "ASR context bias configured: requested={} installed={} score={:.2}",
        requested,
        installed,
        score
    );
}

#[cfg(target_os = "macos")]
fn apply_coreml_context_bias(model: &mut crate::coreml_asr::CoreMlParakeet) {
    let Some(score) = context_bias_score() else {
        return;
    };
    let terms = context_bias_terms();
    let requested = terms.len();
    let installed = model.set_context_bias_phrases(terms.iter(), score);
    tracing::info!(
        "ASR CoreML context bias configured: requested={} installed={} score={:.2}",
        requested,
        installed,
        score
    );
}

fn context_bias_score() -> Option<f32> {
    if matches!(
        std::env::var("HR_DISABLE_ASR_CONTEXT_BIAS").ok().as_deref(),
        Some("1") | Some("true") | Some("on")
    ) {
        return None;
    }
    let score = std::env::var("HR_ASR_CONTEXT_BIAS_SCORE")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(DEFAULT_CONTEXT_BIAS_SCORE);
    score
        .is_finite()
        .then_some(score)
        .filter(|s| *s > 0.0 && *s <= 5.0)
}

fn context_bias_terms() -> Vec<String> {
    // Intentional product vocabulary: bias the decoder toward the HeardRight
    // brand instead of homophones such as "herd right" when users name the app.
    let mut terms = vec![
        "Heard Right".to_string(),
        "HeardRight".to_string(),
        "Zephyr".to_string(),
        "Zephyr stop".to_string(),
        "Zephyr send".to_string(),
        "Zephyr cancel".to_string(),
    ];
    terms.extend(crate::vocabulary::terms());
    let max_terms = std::env::var("HR_ASR_CONTEXT_BIAS_MAX_TERMS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(512);
    terms.truncate(max_terms);
    terms
}

#[cfg(target_os = "windows")]
fn probe_directml_embedded() -> bool {
    // The vendored crate registers DirectML with a silent CPU fallback; a
    // real probe requires an ORT session. We use the same lightweight
    // heuristic the sidecar uses: trust the env override or default
    // to DML on Windows and let model load fail loudly if the device is absent.
    !matches!(
        std::env::var("HR_ASR_EP").ok().as_deref(),
        Some("cpu") | Some("CPU")
    )
}

/// Resolve the models base directory. Models live in EXACTLY ONE place — the
/// app-data models dir — on every machine, dev or installed. There is no dev /
/// sibling-of-exe / cwd lookup. Precedence:
///  1. `HR_MODELS_DIR` — the shell→sidecar channel. The shell computes the
///     app-data models dir (honoring `HR_APP_DATA_DIR`) and passes it here, so
///     the sidecar uses the exact path the shell resolved.
///  2. The optional `hint` (a positional arg, for running the sidecar standalone).
///  3. `<app-data>/models` — the sidecar's own app-data fallback (NOT cwd/exe).
pub fn models_base(hint: Option<&Path>) -> PathBuf {
    if let Some(env) = std::env::var_os("HR_MODELS_DIR") {
        return PathBuf::from(env);
    }
    if let Some(hint) = hint {
        return hint.to_path_buf();
    }
    crate::settings::app_data_root().join("models")
}
