fn normalize_numbers(text: &str) -> String {
    let hundreds = re(
        r"(?i)\b(one|two|three|four|five|six|seven|eight|nine)\s+hundred(?:\s+(twenty|thirty|forty|fifty|sixty|seventy|eighty|ninety)(?:\s+(one|two|three|four|five|six|seven|eight|nine))?|\s+(one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen))?\b",
    );
    let out = hundreds.replace_all(text, |caps: &Captures| {
        let base = word_value(&caps[1]).unwrap_or(0) * 100;
        let tail = caps
            .get(2)
            .and_then(|m| word_value(m.as_str()))
            .unwrap_or(0)
            + caps
                .get(3)
                .and_then(|m| word_value(m.as_str()))
                .unwrap_or(0)
            + caps
                .get(4)
                .and_then(|m| word_value(m.as_str()))
                .unwrap_or(0);
        (base + tail).to_string()
    });

    let spoken_hundreds = re(
        r"(?i)\b(one|two|three|four|five|six|seven|eight|nine)\s+(twenty|thirty|forty|fifty|sixty|seventy|eighty|ninety)(?:\s+(one|two|three|four|five|six|seven|eight|nine))\b",
    );
    let out = spoken_hundreds.replace_all(&out, |caps: &Captures| {
        let n = word_value(&caps[1]).unwrap_or(0) * 100
            + word_value(&caps[2]).unwrap_or(0)
            + word_value(&caps[3]).unwrap_or(0);
        n.to_string()
    });

    let tens = re(
        r"(?i)\b(twenty|thirty|forty|fifty|sixty|seventy|eighty|ninety)(?:[\s-]+(one|two|three|four|five|six|seven|eight|nine))?\b",
    );
    tens.replace_all(&out, |caps: &Captures| {
        let n = word_value(&caps[1]).unwrap_or(0)
            + caps
                .get(2)
                .and_then(|m| word_value(m.as_str()))
                .unwrap_or(0);
        n.to_string()
    })
    .into_owned()
}

/// Spoken integers 0–19 → digits (larger numbers are handled by normalize_numbers).
/// "one" is guarded because it doubles as a pronoun ("no one", "the one", "one of");
/// the other small words aren't idiomatic, so they always become digits. Bias is
/// toward KEEPING "one" as a word when ambiguous (a stray "1" reads worse than a
/// spelled "one").
fn normalize_small_numbers(text: &str) -> String {
    let re_small = re(
        r"(?i)\b(zero|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen)\b",
    );
    re_small
        .replace_all(text, |caps: &Captures| {
            let m = caps.get(0).unwrap();
            let word = caps[1].to_ascii_lowercase();
            if word == "one" && one_is_idiomatic(text, m.start(), m.end()) {
                return caps[0].to_string();
            }
            word_value(&word)
                .map(|v| v.to_string())
                .unwrap_or_else(|| caps[0].to_string())
        })
        .into_owned()
}

/// Whether a "one" at `text[start..end]` reads as the pronoun rather than a count.
fn one_is_idiomatic(text: &str, start: usize, end: usize) -> bool {
    let word_split = |c: char| !c.is_alphanumeric() && c != '\'';
    let prev = text[..start]
        .split(word_split).rfind(|s| !s.is_empty())
        .unwrap_or("")
        .to_ascii_lowercase();
    let next = text[end..]
        .split(word_split)
        .find(|s| !s.is_empty())
        .unwrap_or("")
        .to_ascii_lowercase();
    // Determiners/pronoun-makers before, or "of"/"another" after.
    const PROTECT_PREV: [&str; 12] = [
        "no", "the", "this", "that", "these", "those", "which", "any", "every", "each", "only",
        "another",
    ];
    const PROTECT_NEXT: [&str; 2] = ["of", "another"];
    PROTECT_PREV.contains(&prev.as_str()) || PROTECT_NEXT.contains(&next.as_str())
}

fn apply_lexicon(text: &str) -> String {
    let replacements = [
        (r"(?i)\btsf\b", "TSF"),
        (r"(?i)\bt\s*s\s*f\b", "TSF"),
        (r"(?i)\bu\s*i\b", "UI"),
        (r"(?i)\bu\s*x\b", "UX"),
        (r"(?i)\ba\s*p\s*i\b", "API"),
        (r"(?i)\bo\s*n\s*n\s*x\b", "ONNX"),
        (r"(?i)\bo\s*r\s*t\b", "ORT"),
        (r"(?i)\brnnt\b", "RNN-T"),
        (r"(?i)\br\s*n\s*n\s*t\b", "RNN-T"),
        (r"(?i)\brnn[\s-]?t\b", "RNN-T"),
        (r"(?i)\btdt\b", "TDT"),
        (r"(?i)\bt\s*d\s*t\b", "TDT"),
        (r"(?i)\bdml\b", "DML"),
        (r"(?i)\basr\b", "ASR"),
        (r"(?i)\bkws\b", "KWS"),
        (r"(?i)\bgpu\b", "GPU"),
        (r"(?i)\bcpu\b", "CPU"),
        (r"(?i)\bapi\b", "API"),
        (r"(?i)\bui\b", "UI"),
        (r"(?i)\bux\b", "UX"),
        (r"(?i)\bonnx\b", "ONNX"),
        (r"(?i)\bort\b", "ORT"),
        (r"(?i)\bjson\b", "JSON"),
        (r"(?i)\bwav\b", "WAV"),
        (r"(?i)\bnew york\b", "New York"),
        (r"(?i)\btuesday\b", "Tuesday"),
        // "heard right" and its ASR-corrupted homophones handled by
        // apply_heardright_homophones (per-form guarded) below — the verb idiom
        // "I/you heard right" and other literal readings must NOT become the brand.
        (r"(?i)\bwispr flow\b", "Wispr Flow"),
        (r"(?i)\bsuperwhisper\b", "Superwhisper"),
        (r"(?i)\bbandra\b", "Bandra"),
        (r"(?i)\bmumbai\b", "Mumbai"),
        (r"(?i)\bkoregaon\b", "Koregaon"),
        (r"(?i)\bdji mic mini\b", "DJI Mic Mini"),
        (r"(?i)\brtx\s+4070\b", "RTX 4070"),
        (r"(?i)\bwhisper\b", "Whisper"),
        (r"(?i)\bllama\b", "Llama"),
        (r"(?i)\bollama\b", "Ollama"),
        (r"(?i)\bgemma\b", "Gemma"),
        (r"(?i)\bparakeet\b", "Parakeet"),
        (r"(?i)\bzephyr\b", "Zephyr"),
        // Unambiguous suite-wide brand proper nouns (no common-word collision).
        (r"(?i)\bsquarespace\b", "Squarespace"),
        (r"(?i)\bsquare space\b", "Squarespace"),
        (r"(?i)\btiktok\b", "TikTok"),
        (r"(?i)\btik tok\b", "TikTok"),
        (r"(?i)\byoutube\b", "YouTube"),
        (r"(?i)\binstagram\b", "Instagram"),
        (r"(?i)\boracle\b", "Oracle"),
        (r"(?i)\bnetflix\b", "Netflix"),
        (r"(?i)\bhuawei\b", "Huawei"),
    ];
    // Cow-threaded: ~40 narrow lexicon patterns, almost none of which match any
    // given utterance — only reassign (and allocate) on an actual hit instead of
    // cloning the whole string on every no-op pass.
    let mut out: Cow<str> = Cow::Borrowed(text);
    for (pattern, replacement) in replacements {
        if let Cow::Owned(s) = re(pattern).replace_all(&out, replacement) {
            out = Cow::Owned(s);
        }
    }
    apply_heardright_homophones(&out)
}

/// Greedy-decoder ASR corruptions of the brand name "HeardRight" that the
/// context-bias layer (per-token logit bonus in `coreml_asr_sections/decode_window.rs`)
/// cannot repair because it can only nudge single tokens, not a multi-token
/// phrase. Observed in production transcripts. Each entry is a lowercase,
/// word-boundary-anchored two-word phrase to repair to the brand spelling
/// "HeardRight" (the replacement is always exactly that string, regardless of
/// the input's casing).
///
/// Greppable/extendable: add a new corruption here, then add a guard clause in
/// `heardright_homophone_guard` below. EVERY entry needs a guard decision — do
/// not assume a phrase "has no literal reading" without checking. An earlier
/// draft of this table replaced "her drive", "her right" and "hurt right"
/// unconditionally, which would have corrupted "her drive to succeed", "her
/// right hand" and "it hurt right there" — all far more common in ordinary
/// dictation than the brand.
const HEARDRIGHT_HOMOPHONES: [&str; 7] = [
    "her drive",
    "herd right",
    "heard right",
    "heard write",
    "hurt right",
    "her right",
    "heard rite",
];

/// Brand-context evidence used by the possessive-ambiguous forms below. These
/// are words that co-occur with someone talking ABOUT the app and essentially
/// never with a possessive reading ("her drive to succeed" contains none of
/// them; "stop her drive from launching with the other apps" contains two).
const BRAND_CONTEXT_TERMS: [&str; 18] = [
    "app",
    "apps",
    "launch",
    "launching",
    "startup",
    "start-up",
    "install",
    "installed",
    "update",
    "dictation",
    "dictate",
    "transcription",
    "transcribe",
    "pill",
    "hub",
    "wake",
    "recording",
    "zephyr",
];

/// PRECISION HEURISTIC. Returns true to SKIP the repair. The governing rule is
/// that a missed repair is far cheaper than corrupting a correct sentence, so
/// every form is guarded and ambiguous ones must clear a higher bar.
///
/// Two classes:
///
/// 1. NEGATIVE-EVIDENCE forms — the corruption is rare in ordinary English, so
///    repair by default and skip only on an obvious literal cue. "heard right"
///    keeps its pronoun-idiom guard ("I/you/we heard right"); "herd right" and
///    "heard write" skip when the next word forces the literal reading.
///    "heard rite" needs no cue: "rite" is rare and never follows "heard".
///
/// 2. POSITIVE-EVIDENCE forms — "her drive", "her right" and "hurt right" are
///    all ordinary English (possessive + noun, or "hurt" + adverbial "right").
///    The literal reading is the COMMON one, so adjacent-word cues are the
///    wrong tool: no local cue distinguishes "her drive from launching" from
///    "her drive from the airport". These repair only when the surrounding
///    sentence carries independent brand evidence (`BRAND_CONTEXT_TERMS`).
///    That deliberately misses a bare "her drive" with no context — correct,
///    because with no context there is no reason to believe it is the brand.
fn heardright_homophone_guard(phrase: &str, prev: &str, next: &str, sentence: &str) -> bool {
    const IDIOM_PREV: [&str; 7] = ["i", "you", "we", "they", "he", "she", "who"];
    const HERD_LITERAL_NEXT: [&str; 5] = ["there", "here", "now", "along", "away"];
    const WRITE_LITERAL_NEXT: [&str; 4] = ["it", "that", "this", "down"];
    match phrase {
        "heard right" => IDIOM_PREV.contains(&prev),
        "herd right" => HERD_LITERAL_NEXT.contains(&next),
        "heard write" => WRITE_LITERAL_NEXT.contains(&next),
        "her drive" | "her right" | "hurt right" => !has_brand_context(sentence),
        _ => false,
    }
}

/// True when the sentence contains independent evidence that the speaker is
/// talking about the app. Word-boundary matched so "happier" cannot satisfy
/// "app".
fn has_brand_context(sentence: &str) -> bool {
    sentence
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|w| !w.is_empty())
        .any(|word| {
            let lowered = word.to_ascii_lowercase();
            BRAND_CONTEXT_TERMS.contains(&lowered.as_str())
        })
}

fn apply_heardright_homophones(text: &str) -> String {
    let word_split = |c: char| !c.is_alphanumeric() && c != '\'';
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    // Collect all candidate matches across every homophone form, then process
    // them in source order so overlapping/adjacent phrases never double-fire.
    let mut matches: Vec<(usize, usize, &'static str)> = Vec::new();
    for phrase in HEARDRIGHT_HOMOPHONES {
        let pattern = format!(r"(?i)\b{}\b", regex::escape(phrase));
        for m in re(&pattern).find_iter(text) {
            matches.push((m.start(), m.end(), phrase));
        }
    }
    matches.sort_by_key(|&(start, _, _)| start);
    for (start, end, phrase) in matches {
        if start < last {
            continue; // already covered by an earlier, overlapping match
        }
        out.push_str(&text[last..start]);
        let prev = text[..start]
            .split(word_split)
            .rfind(|s| !s.is_empty())
            .unwrap_or("")
            .to_ascii_lowercase();
        let next = text[end..]
            .split(word_split)
            .find(|s| !s.is_empty())
            .unwrap_or("")
            .to_ascii_lowercase();
        // Sentence containing the match, used by the positive-evidence forms.
        // Bounded by terminal punctuation on either side so brand evidence from
        // a neighbouring sentence cannot license a repair here.
        let sentence_start = text[..start]
            .rfind(['.', '!', '?', '\n'])
            .map_or(0, |i| i + 1);
        let sentence_end = text[end..]
            .find(['.', '!', '?', '\n'])
            .map_or(text.len(), |i| end + i);
        let sentence = &text[sentence_start..sentence_end];
        if heardright_homophone_guard(phrase, &prev, &next, sentence) {
            out.push_str(&text[start..end]); // keep the literal reading verbatim
        } else {
            out.push_str("HeardRight");
        }
        last = end;
    }
    out.push_str(&text[last..]);
    out
}

fn casing_rules(text: &str, capitalize_start: bool) -> String {
    let out = re(r"\bi\b").replace_all(text, "I");
    let out = re(r"(?i)\bi('(?:m|ve|ll|d|s|re))\b").replace_all(&out, "I$1");
    let mut out = re(r"([.!?]\s+)([a-z])")
        .replace_all(&out, |caps: &Captures| {
            format!("{}{}", &caps[1], caps[2].to_ascii_uppercase())
        })
        .into_owned();
    if capitalize_start {
        let starts_with_email = out
            .split_whitespace()
            .next()
            .is_some_and(|token| token.contains('@'));
        if !starts_with_email {
            if let Some(first) = out.chars().next() {
                if first.is_ascii_lowercase() {
                    out.replace_range(0..first.len_utf8(), &first.to_ascii_uppercase().to_string());
                }
            }
        }
    }
    out
}

pub fn deterministic_polish(text: &str) -> String {
    deterministic_polish_inner(text, true)
}

/// Restore saved vocabulary terms to their exact spelling/casing. Vocabulary biases
/// ASR recognition, but the local polish (deterministic + Harper spelling) can still
/// lowercase or mis-case a known term ("cloudflare" when the user saved "Cloudflare").
/// Replace whole-word, case-insensitive matches with the saved form. Longest terms
/// first so a multi-word term ("Damned Designs") wins over its sub-words. Local only —
/// this never leaves the device (belongs at L0/L1, not the cloud L2/L3 context).
pub fn restore_vocabulary_casing(text: &str, terms: &[String]) -> String {
    if text.is_empty() {
        return text.to_string();
    }
    let mut sorted: Vec<&str> = terms
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .collect();
    sorted.sort_by_key(|t| std::cmp::Reverse(t.len()));
    let keys: Vec<String> = sorted.iter().map(|term| (*term).to_string()).collect();
    let regexes = regexes_for_keys(&VOCABULARY_REGEX_CACHE, &keys);
    // Cow-threaded: most saved vocab terms don't appear in any given utterance —
    // only reassign (and allocate) when a term actually matched.
    let mut out: Cow<str> = Cow::Borrowed(text);
    for (term, pattern) in sorted.into_iter().zip(regexes) {
        if let Cow::Owned(s) = pattern.replace_all(&out, |caps: &regex::Captures| {
            // Only rewrite when the match differs (avoid needless allocations and
            // leave an already-correct term untouched).
            if &caps[0] == term {
                caps[0].to_string()
            } else {
                term.to_string()
            }
        }) {
            out = Cow::Owned(s);
        }
    }
    out.into_owned()
}

/// Apply the user's deterministic text replacements: case-insensitive,
/// whole-word/phrase, longest key first (so a multi-word key wins over a
/// shorter overlapping key). The repeated-mishear lever — "always fix X to Y" — separate from
/// vocabulary (spelling guidance only) and snippets (trigger expansion).
/// Local only; runs at L0/L1 before anything reaches an LLM.
pub fn apply_replacements(
    text: &str,
    replacements: &std::collections::HashMap<String, String>,
) -> String {
    if text.is_empty() || replacements.is_empty() {
        return text.to_string();
    }
    let mut sorted: Vec<(&str, &str)> = replacements
        .iter()
        .map(|(k, v)| (k.trim(), v.as_str()))
        .filter(|(k, _)| !k.is_empty())
        .collect();
    sorted.sort_by_key(|(k, _)| std::cmp::Reverse(k.len()));
    let keys: Vec<String> = sorted.iter().map(|(from, _)| (*from).to_string()).collect();
    let regexes = regexes_for_keys(&REPLACEMENT_REGEX_CACHE, &keys);
    // Cow-threaded: same reasoning as restore_vocabulary_casing — most saved
    // replacement keys don't appear in any given utterance.
    let mut out: Cow<str> = Cow::Borrowed(text);
    for ((_, to), pattern) in sorted.into_iter().zip(regexes) {
        // NoExpand: the replacement is literal text, `$` must not expand.
        if let Cow::Owned(s) = pattern.replace_all(&out, regex::NoExpand(to)) {
            out = Cow::Owned(s);
        }
    }
    out.into_owned()
}

#[cfg(test)]
mod heardright_homophone_tests {
    use super::apply_heardright_homophones as repair;

    #[test]
    fn repairs_all_positive_forms() {
        // These are all DICTATION PROSE — the user writing *about* the app.
        // Deliberately not "open <app>" phrasing: spoken app-open is a
        // standalone command resolved by app_launch::resolve (live app scan +
        // fuzzy match), a different lane entirely. Mixing command-shaped
        // sentences in here muddies what this repair is responsible for.
        //
        // "her drive" is possessive-ambiguous, so it repairs only with brand
        // evidence in the sentence — "dictation" here.
        assert_eq!(
            repair("her drive handles dictation offline"),
            "HeardRight handles dictation offline"
        );
        assert_eq!(repair("switch to herd right please"), "switch to HeardRight please");
        assert_eq!(repair("we shipped heard right for me"), "we shipped HeardRight for me");
        assert_eq!(repair("launch heard write today"), "launch HeardRight today");
        assert_eq!(
            repair("hurt right ships dictation next week"),
            "HeardRight ships dictation next week"
        );
        assert_eq!(
            repair("her right is the app I use"),
            "HeardRight is the app I use"
        );
        assert_eq!(repair("try heard rite instead"), "try HeardRight instead");
    }

    #[test]
    fn leaves_literal_readings_untouched() {
        // Verb idiom, not the brand.
        assert_eq!(repair("I heard right about the meeting"), "I heard right about the meeting");
        assert_eq!(repair("you heard right, we won"), "you heard right, we won");
        // Literal cattle sense: "right" as a direction/location adverb.
        assert_eq!(
            repair("the herd right there is grazing"),
            "the herd right there is grazing"
        );
        assert_eq!(repair("the herd right here now"), "the herd right here now");
        // Literal "heard, [now] write it down".
        assert_eq!(
            repair("I heard write it down please"),
            "I heard write it down please"
        );
        assert_eq!(repair("heard write that memo"), "heard write that memo");
    }

    /// Regression: an earlier draft replaced "her drive", "her right" and
    /// "hurt right" unconditionally. Every sentence here is ordinary English
    /// with no brand evidence, and every one of them would have been corrupted.
    #[test]
    fn possessive_ambiguous_forms_need_brand_evidence() {
        for text in [
            "I admired her drive to succeed",
            "her drive was full of photos",
            "she raised her right hand",
            "it is her right to refuse",
            "it hurt right there when I moved",
            "my knee hurt right after the run",
        ] {
            assert_eq!(repair(text), text, "must not rewrite: {text}");
        }
    }

    /// The real production failure, verbatim from the 2026-07-27 transcript.
    /// "launching" and "apps" are the brand evidence that licenses the repair.
    #[test]
    fn repairs_the_observed_production_sentence() {
        assert_eq!(
            repair("How can we stop her drive from launching at the same time as all the other apps?"),
            "How can we stop HeardRight from launching at the same time as all the other apps?"
        );
    }

    /// Brand evidence must not leak across a sentence boundary.
    #[test]
    fn brand_evidence_does_not_cross_sentences() {
        assert_eq!(
            repair("I love this app. I admired her drive."),
            "I love this app. I admired her drive."
        );
    }

    #[test]
    fn mixed_case_and_mid_sentence_placement() {
        assert_eq!(
            repair("Please install HER DRIVE for the demo"),
            "Please install HeardRight for the demo"
        );
        assert_eq!(
            repair("She said the notes on Herd Right were saved"),
            "She said the notes on HeardRight were saved"
        );
        assert_eq!(
            repair("Wait, Heard Write just crashed"),
            "Wait, HeardRight just crashed"
        );
    }

    #[test]
    fn true_brand_and_literal_herd_in_same_sentence() {
        // The literal "herd right there" is grazing (untouched); the true brand
        // mention "her drive" later in the same sentence still repairs, licensed
        // by "dictation".
        assert_eq!(
            repair("the herd right there is grazing, and her drive does dictation"),
            "the herd right there is grazing, and HeardRight does dictation"
        );
    }
}

#[cfg(test)]
mod vocab_casing_tests {
    use super::restore_vocabulary_casing;

    #[test]
    fn restores_casing_and_prefers_longer_terms() {
        let terms = vec!["Cloudflare".to_string(), "Damned Designs".to_string()];
        assert_eq!(
            restore_vocabulary_casing("deploy on cloudflare for damned designs", &terms),
            "deploy on Cloudflare for Damned Designs",
        );
        // Whole-word only: don't touch a substring inside another word.
        assert_eq!(
            restore_vocabulary_casing("cloudflarey", &terms),
            "cloudflarey",
        );
        // No terms → unchanged.
        assert_eq!(restore_vocabulary_casing("hello", &[]), "hello");
    }

    #[test]
    fn replacements_apply_whole_word_case_insensitive_longest_first() {
        use super::apply_replacements;
        let mut map = std::collections::HashMap::new();
        map.insert("in voice".to_string(), "invoice".to_string());
        map.insert("voice".to_string(), "Voice".to_string());
        assert_eq!(
            apply_replacements("send the In Voice today", &map),
            "send the invoice today",
        );
        // Whole-word only; literal replacement (no $-expansion).
        let mut lit = std::collections::HashMap::new();
        lit.insert("price".to_string(), "$99".to_string());
        assert_eq!(apply_replacements("the price is right", &lit), "the $99 is right");
        assert_eq!(apply_replacements("priceless", &lit), "priceless");
        assert_eq!(
            apply_replacements("nothing", &std::collections::HashMap::new()),
            "nothing",
        );
    }
}

pub fn deterministic_polish_tail(text: &str) -> String {
    deterministic_polish_inner(text, false)
}

fn deterministic_polish_inner(text: &str, capitalize_start: bool) -> String {
    if text.trim().is_empty() {
        return text.to_string();
    }
    let mut out = collapse_horizontal_space(text.trim());
    // NOTE: boundary wake-word stripping ("zephyr" at the start/end) is deliberately
    // OFF while the acoustic wake word is unwired — it deleted the real word "zephyr"
    // from dictation. Real control tails ("… zephyr stop/send/cancel") are still
    // handled by parse_control_command before polish. Re-add when wake ships.
    out = apply_inline_edits(&out);
    out = collapse_repetitions(&out);
    out = reduce_fillers(&out);
    out = apply_inline_formatting(&out);
    out = fix_punctuation_spacing(&out);
    out = normalize_decimals(&out);
    out = normalize_large_numbers(&out);
    out = normalize_time_money_percent(&out);
    out = normalize_dates(&out);
    out = normalize_units(&out);
    out = normalize_numbers(&out);
    out = normalize_small_numbers(&out);
    out = casing_rules(&out, capitalize_start);
    out = apply_lexicon(&out);
    out = tighten_domain_spacing(&out);
    out.trim().to_string()
}
