use std::time::Instant;

/// Warm the deterministic polish path (one throwaway pass) so the FIRST real
/// dictation doesn't pay any cold cost. Called from the worker warmup alongside
/// the model warmup — off the critical path. (No spellchecker to load: ASR
/// output is real words, not typos — see `harper_correct` removal, dispatch #6.)
pub fn warm() {
    let _ = polish_local("Warm up one two three.");
    let _ = polish_local(
        "Please send support at sign example dot com the project invoice \
         for four thousand two hundred and fifty dollars tomorrow morning.",
    );
}

pub fn polish(input: &str) -> String {
    polish_local(input)
}

pub fn polish_local_only(input: &str) -> String {
    polish_local(input)
}

/// Local polish with explicit first-letter casing control. `capitalize_start:
/// false` = the dictation continues an unfinished sentence already in the
/// focused field, so the first word must stay lowercase (field text is
/// preserved as-is; we adapt to it, never the reverse).
pub fn polish_local_only_with(input: &str, capitalize_start: bool) -> String {
    polish_local_with(input, capitalize_start)
}

/// True when the focused field already holds text whose tail is an unfinished
/// sentence — the insertion point is assumed at the end of the field (the
/// overwhelmingly common dictate-after-typing case). Trailing quotes/brackets
/// are skipped so `He said "stop."` still ends the sentence.
pub fn continues_mid_sentence(field_text: Option<&str>) -> bool {
    let Some(text) = field_text else {
        return false;
    };
    let trimmed = text.trim_end();
    if trimmed.is_empty() {
        return false;
    }
    let last = trimmed
        .chars()
        .rev()
        .find(|c| !matches!(c, '"' | '\'' | ')' | ']' | '}' | '\u{201D}' | '\u{2019}'));
    !matches!(last, None | Some('.' | '!' | '?' | ':' | '\n'))
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DictationPolishContext<'a> {
    pub audio_secs: Option<f32>,
    pub app_name: Option<&'a str>,
    pub window_title: Option<&'a str>,
    /// Focused-field text captured at record start (macOS AX) — on-device
    /// context that grounds app-aware polish; sanitized before leaving device.
    pub field_text: Option<&'a str>,
    pub selected_text: Option<&'a str>,
    pub field_context_available: bool,
    pub writing_region: Option<&'a str>,
}

pub fn polish_dictation(input: &str, context: DictationPolishContext<'_>) -> String {
    let total_started = Instant::now();
    let local_started = Instant::now();
    // Preserve the user's typed field text as-is: if it ends mid-sentence, the
    // dictation is a continuation and must not be force-capitalized.
    let continuation = continues_mid_sentence(context.field_text);
    let local = polish_local_only_with(input, !continuation);
    let local_ms = local_started.elapsed().as_millis() as u64;
    if !should_try_l1_app_polish(input, context) {
        trace_polish_timing(local_ms, None, false, "local_only", total_started);
        return local;
    }
    let l1_started = Instant::now();
    match l1_app_polish_outcome(&local, &context) {
        crate::l3_cleanup::CleanupOutcome::Cleaned(cleaned) => {
            // Field bug 2026-07-16: the model recapitalized a continuation's
            // first word despite the prompt rule (payload-verified: lowercase
            // in, capitalized out). Reconcile deterministically instead of
            // trusting compliance: if this is a continuation and the model
            // returned the SAME first word merely capitalized, downcase it,
            // then re-run vocabulary restore so saved proper-noun terms keep
            // their exact casing.
            let cleaned = if continuation {
                reconcile_continuation_casing(cleaned, &local)
            } else {
                cleaned
            };
            trace_polish_timing(
                local_ms,
                Some(l1_started.elapsed().as_millis() as u64),
                true,
                "l1_cleaned",
                total_started,
            );
            return cleaned;
        }
        crate::l3_cleanup::CleanupOutcome::Failed {
            error_class,
            circuit_open,
        } => {
            crate::l3_cleanup::record_local_fallback();
            tracing::warn!(
                error_class,
                circuit_open,
                "l1_app_polish_fallback_to_local_polish"
            );
            trace_polish_timing(
                local_ms,
                Some(l1_started.elapsed().as_millis() as u64),
                false,
                "l1_failed",
                total_started,
            );
        }
        crate::l3_cleanup::CleanupOutcome::Skipped { .. } => {
            trace_polish_timing(
                local_ms,
                Some(l1_started.elapsed().as_millis() as u64),
                false,
                "l1_skipped",
                total_started,
            );
        }
    }
    local
}

fn l1_app_polish_outcome(
    input: &str,
    context: &DictationPolishContext<'_>,
) -> crate::l3_cleanup::CleanupOutcome {
    let context = crate::l3_cleanup::PolishContext {
        app_name: context.app_name.map(str::to_string),
        window_title: context.window_title.map(str::to_string),
        field_text: context.field_text.map(str::to_string),
        selected_text: context.selected_text.map(str::to_string),
        field_context_available: context.field_context_available,
        vocabulary: crate::vocabulary::terms(),
        writing_region: context.writing_region.map(str::to_string),
        sound_alikes: crate::vocabulary::sound_alike_pairs(),
    };
    crate::l3_cleanup::app_polish_outcome(input, &context)
}

fn trace_polish_timing(
    local_ms: u64,
    l1_ms: Option<u64>,
    ai_used: bool,
    outcome: &'static str,
    total_started: Instant,
) {
    tracing::info!(
        local_polish_ms = local_ms,
        l1_app_polish_ms = l1_ms,
        ai_used,
        outcome,
        total_polish_ms = total_started.elapsed().as_millis() as u64,
        "final_transcript_polish_timing"
    );
}

fn trace_local_polish_timing(
    deterministic_ms: u64,
    cleanup_ms: u64,
    domain_ms: u64,
    vocabulary_ms: u64,
    replacements_ms: u64,
    total_started: Instant,
) {
    tracing::info!(
        deterministic_ms,
        local_cleanup_ms = cleanup_ms,
        domain_spacing_ms = domain_ms,
        vocabulary_restore_ms = vocabulary_ms,
        replacements_ms,
        total_local_ms = total_started.elapsed().as_millis() as u64,
        "l0_local_polish_timing"
    );
}

fn should_try_l1_app_polish(input: &str, context: DictationPolishContext) -> bool {
    let text = input.trim();
    if text.split_whitespace().count() < l3_min_words() {
        return false;
    }
    if context
        .audio_secs
        .is_some_and(|secs| secs < l3_min_audio_secs())
    {
        return false;
    }
    if heardright_core::command_recognition::recognize_command(text).is_some()
        || heardright_core::command_recognition::app_launch_query(text).is_some()
    {
        return false;
    }
    true
}

fn l3_min_audio_secs() -> f32 {
    std::env::var("HEARDRIGHT_L3_MIN_AUDIO_SECS")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(3.0)
        .clamp(0.0, 30.0)
}

fn l3_min_words() -> usize {
    std::env::var("HEARDRIGHT_L3_MIN_WORDS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(8)
        .clamp(1, 200)
}

fn polish_local(input: &str) -> String {
    polish_local_with(input, true)
}

/// Post-L1 continuation reconciliation: the local pass sent the model a
/// lowercase first word (continuation); if the model returned the same first
/// word merely recapitalized, downcase it back and re-run vocabulary restore
/// so saved terms (e.g. "HeardRight") keep their exact casing. If the model
/// changed the first WORD itself (restructure, proper noun it knows), trust it.
fn reconcile_continuation_casing(cleaned: String, local: &str) -> String {
    let first_word = |s: &str| -> String {
        s.chars()
            .take_while(|c| !c.is_whitespace())
            .collect::<String>()
    };
    let local_first = first_word(local);
    let cleaned_first = first_word(&cleaned);
    let local_starts_lower = local_first.chars().next().is_some_and(|c| c.is_lowercase());
    let same_word_recapitalized = local_starts_lower
        && cleaned_first.chars().next().is_some_and(|c| c.is_uppercase())
        && cleaned_first.to_lowercase() == local_first.to_lowercase();
    if !same_word_recapitalized {
        return cleaned;
    }
    let fixed = lowercase_continuation_start(cleaned);
    heardright_core::text_pipeline::restore_vocabulary_casing(&fixed, &crate::vocabulary::terms())
}

/// Lowercase the first letter of a continuation, exempting the pronoun "I"
/// and its contractions ("I'm", "I've", "I'll", "I'd", "I're" is not a word
/// but the regex family is cheap). Proper nouns are restored downstream by
/// vocabulary restore and the L1 CONTINUATION CASING clause.
fn lowercase_continuation_start(mut text: String) -> String {
    let Some(first) = text.chars().next() else {
        return text;
    };
    if !first.is_uppercase() {
        return text;
    }
    // Exempt the pronoun-I family: "I", "I'm", "I've", "I'll", "I'd".
    if first == 'I' {
        let rest: String = text.chars().skip(1).take(3).collect();
        let is_pronoun = rest.is_empty()
            || rest.starts_with(' ')
            || rest.starts_with('\'')
            || rest.starts_with(',')
            || rest.starts_with('.');
        if is_pronoun {
            return text;
        }
    }
    let lowered = first.to_lowercase().to_string();
    text.replace_range(0..first.len_utf8(), &lowered);
    text
}

fn polish_local_with(input: &str, capitalize_start: bool) -> String {
    let total_started = Instant::now();
    let deterministic_started = Instant::now();
    // `deterministic_polish_tail` is the same pass with first-letter
    // capitalization off — used when the dictation continues a sentence the
    // user already typed in the field (see `continues_mid_sentence`).
    let deterministic = if capitalize_start {
        heardright_core::text_pipeline::deterministic_polish(input)
    } else {
        // ASR itself emits sentence-cased text ("Continuing a sentence..."),
        // so merely REFRAINING from capitalizing is not enough for a
        // continuation — actively lowercase the first letter (field-tested
        // 2026-07-16: the refrain-only version still delivered a capital).
        // The pronoun "I" family is exempt; saved vocabulary and the L1
        // model's proper-noun clause restore genuine proper nouns after.
        lowercase_continuation_start(heardright_core::text_pipeline::deterministic_polish_tail(
            input,
        ))
    };
    let deterministic_ms = deterministic_started.elapsed().as_millis() as u64;
    let cleanup_started = Instant::now();
    let mut polished = polish_local_after_deterministic(&deterministic);
    // Harper capitalizes sentence starts unconditionally, undoing the
    // continuation casing chosen above. Restore the lowercase first letter
    // BEFORE vocabulary restore, so proper-noun terms still get their casing.
    if !capitalize_start {
        let restore = match (deterministic.chars().next(), polished.chars().next()) {
            (Some(d), Some(p)) => {
                d.is_lowercase() && p.is_uppercase() && p.to_lowercase().next() == Some(d)
            }
            _ => false,
        };
        if restore {
            let first = polished.chars().next().expect("checked above");
            polished.replace_range(
                0..first.len_utf8(),
                &first.to_lowercase().to_string(),
            );
        }
    }
    let cleanup_ms = cleanup_started.elapsed().as_millis() as u64;
    // Harper (spelling) re-inserts a space after the domain dot ("example.com" ->
    // "example. com"), undoing the deterministic domain fix. Domain spacing is a fixed
    // rule (L0), so re-tighten it as the final step after Harper.
    let domain_started = Instant::now();
    let tightened = heardright_core::text_pipeline::tighten_domain_spacing(&polished);
    let domain_ms = domain_started.elapsed().as_millis() as u64;
    // Saved vocabulary belongs at L0/L1: restore each term's exact spelling/casing that
    // Harper may have lowercased or "corrected" (local only — never sent to L2/L3).
    let vocabulary_started = Instant::now();
    let restored = heardright_core::text_pipeline::restore_vocabulary_casing(
        &tightened,
        &crate::vocabulary::terms(),
    );
    let vocabulary_ms = vocabulary_started.elapsed().as_millis() as u64;
    // User replacements last — the deterministic "always fix X to Y" layer wins
    // over everything upstream (local only, part of the raw/undo-AI text).
    let replacements = crate::settings::replacements();
    let replacements_started = Instant::now();
    let output = if replacements.is_empty() {
        restored
    } else {
        heardright_core::text_pipeline::apply_replacements(&restored, &replacements)
    };
    trace_local_polish_timing(
        deterministic_ms,
        cleanup_ms,
        domain_ms,
        vocabulary_ms,
        replacements_started.elapsed().as_millis() as u64,
        total_started,
    );
    output
}

fn polish_local_after_deterministic(input: &str) -> String {
    match crate::settings::transcript_cleanup().as_str() {
        "clean" => polish_clean(input),
        // "aggressive" is the default.
        _ => polish_aggressive(input),
    }
}

// `clean` = the deterministic transcript normalizer, no spell-correction step.
// ASR (Parakeet) emits real dictionary words; its errors are word-choice
// mis-recognitions ("form" for "from"), which a spellchecker can't fix and
// would only risk corrupting valid rare/technical words (mic->mac). Contextual
// correction is the opt-in L1 app-aware AI layer's job; L0 deterministic handles the rest
// (dispatch #6 — removed harper-core + its burn/cubecl ML tree entirely).
fn polish_clean(input: &str) -> String {
    input.to_string()
}

// `aggressive` = extra speech-clutter removal wrapped around Clean.
fn polish_aggressive(input: &str) -> String {
    capitalize_first_alpha(&polish_aggressive_uncapitalized(input))
}

fn polish_aggressive_uncapitalized(input: &str) -> String {
    let cleaned = heardright_core::text_pipeline::aggressive_speech_cleanup(input);
    let polished = polish_clean(&cleaned);
    heardright_core::text_pipeline::aggressive_speech_cleanup(&polished)
}

fn capitalize_first_alpha(input: &str) -> String {
    let Some((idx, first)) = input.char_indices().find(|(_, ch)| ch.is_alphabetic()) else {
        return input.to_string();
    };
    let upper = first.to_uppercase().to_string();
    if upper == first.to_string() {
        return input.to_string();
    }
    let mut out = input.to_string();
    out.replace_range(idx..idx + first.len_utf8(), &upper);
    out
}
