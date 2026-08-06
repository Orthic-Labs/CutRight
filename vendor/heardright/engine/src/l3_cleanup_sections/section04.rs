fn openrouter_provider_preferences(model: &str) -> Option<Value> {
    let only = env_csv("HEARDRIGHT_L3_OPENROUTER_PROVIDER_ONLY");
    let order = env_csv("HEARDRIGHT_L3_OPENROUTER_PROVIDER_ORDER");
    let sort = env_string("HEARDRIGHT_L3_OPENROUTER_PROVIDER_SORT");
    let only = if only.is_empty() && model.contains("llama-3.3-70b") {
        vec!["groq".to_string()]
    } else {
        only
    };
    let mut provider = serde_json::Map::new();
    provider.insert("data_collection".to_string(), json!("deny"));
    if !only.is_empty() {
        provider.insert("only".to_string(), json!(only));
        provider.insert("allow_fallbacks".to_string(), json!(false));
    } else if !order.is_empty() {
        provider.insert("order".to_string(), json!(order));
    }
    if let Some(sort) = sort {
        provider.insert("sort".to_string(), json!(sort));
    }
    Some(Value::Object(provider))
}

fn parse_message_content(raw: &str) -> Result<String, &'static str> {
    let response: ChatResponse = serde_json::from_str(raw).map_err(|_| "bad_response")?;
    let choice = response.choices.first().ok_or("bad_response")?;
    // A "length" finish means the completion budget ran out mid-output. The
    // partial text can look valid, so accepting it would type a silently
    // truncated polish into the user's app. Reject and fall to the next lane.
    if choice.finish_reason.as_deref() == Some("length") {
        return Err("truncated_response");
    }
    Some(choice.message.content.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or("bad_response")
}

#[cfg(test)]
fn accept_output(input: &str, output: &str) -> bool {
    normalize_output(input, output).is_some()
}

fn normalize_output(input: &str, output: &str) -> Option<String> {
    normalize_output_with_extra(input, output, 160)
}

/// Remove target-field text that a model prepended to the actual dictation.
/// The output is inserted after that existing field text, so retaining the
/// echo duplicates it for the user. Only exact Unicode or ASCII-insensitive
/// whole-prefix matches are eligible; echo-only output is kept so this guard
/// can never silently erase the user's dictation.
pub(crate) fn strip_field_text_echo(field_text: &str, input: &str, output: &str) -> String {
    fn prefix_len(text: &str, field: &str) -> Option<usize> {
        let matches = text.starts_with(field)
            || (field.is_ascii()
                && text
                    .get(..field.len())
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case(field)));
        if !matches {
            return None;
        }
        let rest = &text[field.len()..];
        if rest
            .chars()
            .next()
            .is_some_and(|c| !c.is_whitespace() && c != ',')
        {
            return None;
        }
        Some(field.len())
    }

    let field = field_text.trim();
    if field.is_empty() {
        return output.to_string();
    }
    if prefix_len(input.trim_start(), field).is_some() {
        return output.to_string();
    }
    let output_start = output.trim_start();
    let Some(prefix_len) = prefix_len(output_start, field) else {
        return output.to_string();
    };
    let rest = output_start[prefix_len..].trim_start();
    let rest = rest.strip_prefix(',').unwrap_or(rest).trim_start();
    if rest.is_empty() {
        return output.to_string();
    }
    rest.to_string()
}

pub(crate) fn normalize_output_with_extra(
    input: &str,
    output: &str,
    extra_chars: usize,
) -> Option<String> {
    let trimmed = output.trim();
    if trimmed.is_empty() || trimmed.starts_with("```") {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("transcript:") || lower.contains("the user wants") {
        return None;
    }
    // Strip an echoed system cue if the model prepends one. The base lane cues
    // "Cleaned transcript:"; the live App lane cues "Polished output:". Both are
    // safe to strip — no genuine dictation output starts with either.
    let trimmed = if lower.starts_with("cleaned transcript:") {
        trimmed["cleaned transcript:".len()..].trim()
    } else if lower.starts_with("polished output:") {
        trimmed["polished output:".len()..].trim()
    } else {
        trimmed
    };
    if trimmed.is_empty() {
        return None;
    }
    let max_len = input.len().saturating_add(extra_chars);
    (trimmed.len() <= max_len).then(|| trimmed.to_string())
}

/// Deterministic dash scrub on accepted LLM output. The prompts ban em/en
/// dashes outright, but payload audits (2026-07-19, Mac + Windows) measured a
/// 5-12% leak rate — a 27B model does not reliably obey a NEVER rule. The ban
/// is mechanical, so enforcement is mechanical: digit-adjacent dashes become
/// "to" (numeric range per the prompt's own rule), everything else becomes a
/// comma, matching the prompt's suggested replacements.
pub(crate) fn scrub_dashes(text: &str) -> String {
    if !text.contains('\u{2014}') && !text.contains('\u{2013}') {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c != '\u{2014}' && c != '\u{2013}' {
            out.push(c);
            i += 1;
            continue;
        }
        // Collapse a run of dashes and the whitespace around it.
        let mut j = i;
        while j < chars.len() && matches!(chars[j], '\u{2014}' | '\u{2013}') {
            j += 1;
        }
        while out.ends_with(' ') {
            out.pop();
        }
        let prev_digit = out.chars().last().is_some_and(|p| p.is_ascii_digit());
        let mut k = j;
        while k < chars.len() && chars[k] == ' ' {
            k += 1;
        }
        let next_digit = k < chars.len() && chars[k].is_ascii_digit();
        if prev_digit && next_digit {
            out.push_str(" to ");
            i = k;
        } else if out.is_empty() || k >= chars.len() {
            // Leading or trailing dash: drop it rather than leaving a
            // dangling comma.
            out.push(' ');
            i = k;
        } else {
            out.push_str(", ");
            i = k;
        }
    }
    let cleaned = out.replace(", ,", ",").replace(",,", ",");
    cleaned.trim().trim_end_matches(',').trim_end().to_string()
}

/// Digit-preservation guard (Adrian, 2026-07-19: number corruption
/// "absolutely cannot happen"). Prompt rules alone measurably fail — payload
/// audits caught "1 5" -> "1.5" (spoken fifteen) and a dictation where a
/// digit vanished with its clause. The guard rejects any LLM output whose
/// digit content differs from the input's:
///   - the concatenated digit sequence must be identical, where standalone
///     spelled cardinals count as their digits (so "the left 1" -> "the left
///     one" stays legal in both directions);
///   - the output may not contain a digit-adjacent decimal point the input
///     did not already have (input side tolerates whitespace after the dot so
///     a legitimate "0. 2 5" -> "0.25" join still passes).
/// Compound spelled numbers ("one hundred five") are intentionally not
/// evaluated as arithmetic — a conversion the signature cannot verify is
/// rejected, and rejection just means the local polisher runs instead.
///
/// One deliberate escape hatch (2026-08-02): a spoken self-correction
/// ("Let's meet at 7, oh no, 8") legitimately DELETES the superseded digit,
/// which strict signature equality vetoed on every provider — so number
/// corrections could never ship no matter what the model did. When strict
/// equality fails, `correction_aware_digit_drop` allows deletion-only
/// mismatches where every deleted number token sits immediately before a
/// spoken correction cue in the input. Invented digits, changed digits,
/// reordering, and deletions with no adjacent cue all still reject.
pub(crate) fn digits_preserved(input: &str, output: &str) -> bool {
    if decimal_points_strict(output) > decimal_points_loose(input) {
        return false;
    }
    digit_signature(input) == digit_signature(output) || correction_aware_digit_drop(input, output)
}

/// Cues a speaker uses to replace what they just said. Mirrors the App-lane
/// prompt's SELF-CORRECTION PRECEDENCE examples. Matched case-insensitively
/// on word boundaries ("factually" never matches "actually").
const CORRECTION_CUES: [&str; 12] = [
    "actually",
    "no wait",
    "no, wait",
    "no no",
    "no, no",
    "oh no",
    "i mean",
    "i meant",
    "scratch that",
    "strike that",
    "make that",
    "sorry",
];

/// A deleted number token must end within this many bytes before a cue.
/// Spoken corrections are immediate ("at 7, oh no, 8"), so the window is
/// short on purpose: it keeps an incidental "sorry" elsewhere in the
/// dictation from licensing an unrelated digit drop.
const CORRECTION_CUE_WINDOW: usize = 32;

/// Byte offsets where a correction cue starts, word-boundary checked. The
/// caller passes lowercased text; `to_ascii_lowercase` preserves byte
/// offsets, and cues are pure ASCII so byte-level boundary checks are safe.
fn correction_cue_starts(lower: &str) -> Vec<usize> {
    let bytes = lower.as_bytes();
    let mut starts = Vec::new();
    for cue in CORRECTION_CUES {
        let mut from = 0;
        while let Some(idx) = lower[from..].find(cue) {
            let abs = from + idx;
            let end = abs + cue.len();
            let left_ok = abs == 0 || !bytes[abs - 1].is_ascii_alphanumeric();
            let right_ok = end >= bytes.len() || !bytes[end].is_ascii_alphanumeric();
            if left_ok && right_ok {
                starts.push(abs);
            }
            from = abs + 1;
        }
    }
    starts
}

/// Number tokens with their end byte offset: maximal ASCII-digit runs plus
/// standalone spelled cardinals mapped through `spelled_number`. Unlike the
/// concatenated signature, token boundaries matter here so a dropped "17"
/// can never be satisfied by a surviving "7".
fn number_tokens(text: &str) -> Vec<(String, usize)> {
    fn flush_word(word: &mut String, end: usize, tokens: &mut Vec<(String, usize)>) {
        if !word.is_empty() {
            if let Some(mapped) = spelled_number(word.to_ascii_lowercase().as_str()) {
                tokens.push((mapped.to_string(), end));
            }
            word.clear();
        }
    }
    fn flush_digits(digits: &mut String, end: usize, tokens: &mut Vec<(String, usize)>) {
        if !digits.is_empty() {
            tokens.push((std::mem::take(digits), end));
        }
    }
    let mut tokens = Vec::new();
    let mut digits = String::new();
    let mut word = String::new();
    let mut digits_end = 0usize;
    let mut word_end = 0usize;
    for (pos, c) in text.char_indices() {
        let end = pos + c.len_utf8();
        if c.is_ascii_digit() {
            flush_word(&mut word, word_end, &mut tokens);
            digits.push(c);
            digits_end = end;
        } else if c.is_alphabetic() {
            flush_digits(&mut digits, digits_end, &mut tokens);
            word.push(c);
            word_end = end;
        } else {
            flush_word(&mut word, word_end, &mut tokens);
            flush_digits(&mut digits, digits_end, &mut tokens);
        }
    }
    flush_word(&mut word, word_end, &mut tokens);
    flush_digits(&mut digits, digits_end, &mut tokens);
    tokens
}

/// Deletion-only digit mismatch check for spoken self-corrections. Output
/// number tokens must match input number tokens in order; an input token may
/// be skipped only when it ends within `CORRECTION_CUE_WINDOW` bytes before
/// a correction cue (the superseded value precedes the cue: "7, oh no, 8").
/// The replacement follows the cue, so it can never be the deleted one.
fn correction_aware_digit_drop(input: &str, output: &str) -> bool {
    let input = strip_list_markers(input);
    let output = strip_list_markers(output);
    let cues = correction_cue_starts(&input.to_ascii_lowercase());
    if cues.is_empty() {
        return false;
    }
    let tokens_in = number_tokens(&input);
    let tokens_out: Vec<String> = number_tokens(&output).into_iter().map(|(t, _)| t).collect();
    let deletable: Vec<bool> = tokens_in
        .iter()
        .map(|(_, end)| {
            cues.iter()
                .any(|&c| c >= *end && c - *end <= CORRECTION_CUE_WINDOW)
        })
        .collect();
    let n = tokens_in.len();
    let m = tokens_out.len();
    // can[i][j]: tokens_out[j..] is derivable from tokens_in[i..] by in-order
    // matches, deleting only correction-adjacent input tokens. DP rather than
    // greedy so equal-valued tokens (one deletable, one not) resolve to a
    // valid assignment when one exists.
    let mut can = vec![vec![false; m + 1]; n + 1];
    can[n][m] = true;
    for i in (0..n).rev() {
        can[i][m] = deletable[i] && can[i + 1][m];
        for j in (0..m).rev() {
            let matched = tokens_in[i].0 == tokens_out[j] && can[i + 1][j + 1];
            let deleted = deletable[i] && can[i + 1][j];
            can[i][j] = matched || deleted;
        }
    }
    can[0][0]
}

fn spelled_number(word: &str) -> Option<&'static str> {
    Some(match word {
        "zero" => "0",
        "one" => "1",
        "two" => "2",
        "three" => "3",
        "four" => "4",
        "five" => "5",
        "six" => "6",
        "seven" => "7",
        "eight" => "8",
        "nine" => "9",
        "ten" => "10",
        "eleven" => "11",
        "twelve" => "12",
        "thirteen" => "13",
        "fourteen" => "14",
        "fifteen" => "15",
        "sixteen" => "16",
        "seventeen" => "17",
        "eighteen" => "18",
        "nineteen" => "19",
        "twenty" => "20",
        "thirty" => "30",
        "forty" => "40",
        "fifty" => "50",
        "sixty" => "60",
        "seventy" => "70",
        "eighty" => "80",
        "ninety" => "90",
        "hundred" => "100",
        "thousand" => "1000",
        _ => return None,
    })
}

/// Remove ordered-list markers from the number signature. The prompt asks the
/// model to turn spoken enumeration into numbered lines, so those formatting
/// digits must not make an otherwise valid polish fail the digit guard. Only
/// one- or two-digit markers at line start followed by punctuation and
/// whitespace are excluded; numbers in list content remain protected.
fn strip_list_markers(text: &str) -> String {
    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            let rest = trimmed.trim_start_matches(|c: char| c.is_ascii_digit());
            let consumed = trimmed.len() - rest.len();
            if consumed > 0 && consumed <= 2 {
                if let Some(after) = rest.strip_prefix(['.', ')']) {
                    if after.starts_with(' ') || after.starts_with('\t') {
                        return after.trim_start().to_string();
                    }
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn digit_signature(text: &str) -> String {
    let text = &strip_list_markers(text);
    fn flush(word: &mut String, sig: &mut String) {
        if word.is_empty() {
            return;
        }
        if let Some(mapped) = spelled_number(word.to_ascii_lowercase().as_str()) {
            sig.push_str(mapped);
        }
        word.clear();
    }
    let mut sig = String::new();
    let mut word = String::new();
    for c in text.chars() {
        if c.is_ascii_digit() {
            flush(&mut word, &mut sig);
            sig.push(c);
        } else if c.is_alphabetic() {
            word.push(c);
        } else {
            flush(&mut word, &mut sig);
        }
    }
    flush(&mut word, &mut sig);
    sig
}

/// '.' with a digit immediately before and after — a decimal point the model
/// actually emitted.
fn decimal_points_strict(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    chars
        .windows(3)
        .filter(|w| w[0].is_ascii_digit() && w[1] == '.' && w[2].is_ascii_digit())
        .count()
}

/// '.' with a digit before and a digit after ignoring whitespace — counts the
/// input's spoken decimals even when ASR spaced them out ("0. 2 5").
fn decimal_points_loose(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut count = 0;
    for i in 1..chars.len() {
        if chars[i] != '.' || !chars[i - 1].is_ascii_digit() {
            continue;
        }
        let mut j = i + 1;
        while j < chars.len() && chars[j].is_whitespace() {
            j += 1;
        }
        if j < chars.len() && chars[j].is_ascii_digit() {
            count += 1;
        }
    }
    count
}

fn max_tokens_for(input: &str) -> u32 {
    let words_budget = input.split_whitespace().count() as u32 + 160;
    let char_budget = (input.chars().count() as u32 / 3) + 160;
    words_budget.max(char_budget).clamp(128, 1200)
}

fn max_input_chars() -> usize {
    env_u64("HEARDRIGHT_L3_MAX_INPUT_CHARS")
        .unwrap_or(4_000)
        .clamp(256, 20_000) as usize
}

fn error_class_from_ureq(error: ureq::Error) -> &'static str {
    match error {
        ureq::Error::StatusCode(401 | 403) => "auth",
        ureq::Error::StatusCode(408 | 429) => "rate_limit",
        ureq::Error::StatusCode(500..=599) => "server",
        ureq::Error::StatusCode(_) => "http",
        ureq::Error::Timeout(_) => "timeout",
        _ => "network",
    }
}

fn total_timeout() -> Duration {
    let timeout_ms = env_u64("HEARDRIGHT_L3_TOTAL_TIMEOUT_MS").unwrap_or(1_500);
    Duration::from_millis(timeout_ms.clamp(300, 15_000))
}

/// Cloud budget scaled to the input: a fixed sub-second budget cannot round-trip a long
/// dictation (field-verified 2026-07-16: a 2,400-char dictation timed out on
/// both providers and fell back to local polish). Base 1.5s + 2ms/char,
/// capped at 8s — a long recording already took seconds to transcribe, so a
/// proportionate polish wait is the right trade. An explicit
/// HEARDRIGHT_L3_TOTAL_TIMEOUT_MS override still wins (clamped like before).
fn total_timeout_for_input(input: &str) -> Duration {
    if env_u64("HEARDRIGHT_L3_TOTAL_TIMEOUT_MS").is_some() {
        return total_timeout();
    }
    let scaled = 1_500 + (input.chars().count() as u64).saturating_mul(2);
    Duration::from_millis(scaled.clamp(1_500, 8_000))
}

fn provider_timeout() -> Duration {
    let timeout_ms = env_u64("HEARDRIGHT_L3_PROVIDER_TIMEOUT_MS").unwrap_or(1_500);
    Duration::from_millis(timeout_ms.clamp(250, 15_000))
}

/// Per-attempt cap matched to the scaled total budget: with a long-input
/// budget of several seconds, a fixed 900ms per-attempt cap would strangle
/// every attempt anyway. Give one attempt up to ~2/3 of the total (so a
/// fallback still gets a real slice), floored above cold TLS/provider jitter. An
/// explicit HEARDRIGHT_L3_PROVIDER_TIMEOUT_MS override still wins.
fn provider_timeout_for_budget(total: Duration) -> Duration {
    if env_u64("HEARDRIGHT_L3_PROVIDER_TIMEOUT_MS").is_some() {
        return provider_timeout();
    }
    let scaled = total.mul_f32(0.67);
    scaled.max(Duration::from_millis(1_500))
}

fn circuit_threshold() -> u32 {
    env_u64("HEARDRIGHT_L3_CIRCUIT_FAILS")
        .unwrap_or(3)
        .clamp(1, 20) as u32
}

fn circuit_cooldown() -> Duration {
    Duration::from_millis(
        env_u64("HEARDRIGHT_L3_CIRCUIT_COOLDOWN_MS")
            .unwrap_or(60_000)
            .clamp(1_000, 600_000),
    )
}

fn provider_circuit_open(provider: Provider) -> bool {
    let circuit = CIRCUIT.get_or_init(|| Mutex::new(Circuit::default()));
    lock_circuit(circuit).is_open(provider, Instant::now())
}

fn record_success(provider: Provider) {
    SUCCESSES.fetch_add(1, Ordering::Relaxed);
    if let Some(circuit) = CIRCUIT.get() {
        let mut guard = lock_circuit(circuit);
        guard.record_success(provider);
    }
}

fn record_failure(provider: Provider) {
    FAILURES.fetch_add(1, Ordering::Relaxed);
    let circuit = CIRCUIT.get_or_init(|| Mutex::new(Circuit::default()));
    let mut guard = lock_circuit(circuit);
    if guard.record_failure(
        provider,
        circuit_threshold(),
        circuit_cooldown(),
        Instant::now(),
    ) {
        CIRCUIT_OPENS.fetch_add(1, Ordering::Relaxed);
        tracing::warn!(
            provider = provider.as_str(),
            cooldown_ms = circuit_cooldown().as_millis() as u64,
            "l3_cleanup_provider_circuit_opened"
        );
    }
}

pub fn health() -> CleanupHealth {
    let (
        circuit_open,
        consecutive_failures,
        groq_circuit_open,
        cerebras_circuit_open,
        nvidia_circuit_open,
        openrouter_circuit_open,
    ) = if let Some(circuit) = CIRCUIT.get() {
        let mut guard = lock_circuit(circuit);
        let now = Instant::now();
        let groq_circuit_open = guard.is_open(Provider::Groq, now);
        let cerebras_circuit_open = guard.is_open(Provider::Cerebras, now);
        let nvidia_circuit_open = guard.is_open(Provider::Nvidia, now);
        let openrouter_circuit_open = guard.is_open(Provider::OpenRouter, now);
        (
            groq_circuit_open
                || cerebras_circuit_open
                || nvidia_circuit_open
                || openrouter_circuit_open,
            guard.consecutive_failures(),
            groq_circuit_open,
            cerebras_circuit_open,
            nvidia_circuit_open,
            openrouter_circuit_open,
        )
    } else {
        (false, 0, false, false, false, false)
    };
    CleanupHealth {
        circuit_open,
        consecutive_failures,
        groq_circuit_open,
        cerebras_circuit_open,
        nvidia_circuit_open,
        openrouter_circuit_open,
        attempts: ATTEMPTS.load(Ordering::Relaxed),
        successes: SUCCESSES.load(Ordering::Relaxed),
        failures: FAILURES.load(Ordering::Relaxed),
        skips: SKIPS.load(Ordering::Relaxed),
        local_fallbacks: LOCAL_FALLBACKS.load(Ordering::Relaxed),
        circuit_opens: CIRCUIT_OPENS.load(Ordering::Relaxed),
    }
}

pub fn record_local_fallback() {
    LOCAL_FALLBACKS.fetch_add(1, Ordering::Relaxed);
}

fn record_skip() {
    SKIPS.fetch_add(1, Ordering::Relaxed);
}

fn lock_circuit(circuit: &Mutex<Circuit>) -> MutexGuard<'_, Circuit> {
    circuit.lock()
}

fn trace_attempt(
    spec: &ProviderSpec,
    total_timeout: Duration,
    fallback_index: usize,
    prompt_version: &'static str,
) {
    tracing::info!(
        product_level = product_level(prompt_version),
        provider = spec.provider.as_str(),
        model = %spec.model,
        deadline_ms = total_timeout.as_millis() as u64,
        fallback_index,
        prompt_version,
        "l3_cleanup_attempt"
    );
}

fn trace_result(
    spec: &ProviderSpec,
    status: &'static str,
    latency: Duration,
    error_class: &'static str,
    circuit_state: &'static str,
    prompt_version: &'static str,
) {
    tracing::info!(
        product_level = product_level(prompt_version),
        provider = spec.provider.as_str(),
        model = %spec.model,
        status,
        latency_ms = latency.as_millis() as u64,
        error_class,
        circuit_state,
        prompt_version,
        "l3_cleanup_result"
    );
}

fn trace_skip(reason: &'static str, circuit_state: &'static str) {
    tracing::debug!(
        reason,
        circuit_state,
        product_level = product_level(PROMPT_VERSION),
        prompt_version = PROMPT_VERSION,
        "l3_cleanup_skip"
    );
}

fn product_level(prompt_version: &str) -> &'static str {
    match prompt_version {
        APP_PROMPT_VERSION => "l1",
        PROMPT_POLISH_VERSION => "l2",
        SUMMARY_PROMPT_VERSION => "l3",
        _ => "l1",
    }
}

fn env_true(key: &str) -> bool {
    env_string(key)
        .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "on" | "yes"))
        .unwrap_or(false)
}

fn env_string(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn env_csv(key: &str) -> Vec<String> {
    env_string(key)
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn env_u64(key: &str) -> Option<u64> {
    env_string(key).and_then(|v| v.parse::<u64>().ok())
}
