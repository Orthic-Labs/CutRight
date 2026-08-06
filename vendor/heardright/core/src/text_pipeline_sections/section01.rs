// Pure post-ASR text pipeline: voice-control suffixes, inline formatting, and
// deterministic cleanup that must run before paste.

use regex::{Captures, Regex};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Compile-once cache for finite built-in patterns. User-controlled vocabulary
/// and replacement patterns use replace-whole-snapshot caches below.
static REGEX_CACHE: OnceLock<Mutex<HashMap<String, Regex>>> = OnceLock::new();
static VOCABULARY_REGEX_CACHE: OnceLock<Mutex<RegexSnapshotCache>> = OnceLock::new();
static REPLACEMENT_REGEX_CACHE: OnceLock<Mutex<RegexSnapshotCache>> = OnceLock::new();

#[derive(Default)]
struct RegexSnapshotCache {
    keys: Vec<String>,
    regexes: Vec<Regex>,
    #[cfg(test)]
    builds: usize,
}

fn regexes_for_keys(
    cache: &OnceLock<Mutex<RegexSnapshotCache>>,
    keys: &[String],
) -> Vec<Regex> {
    let cache = cache.get_or_init(|| Mutex::new(RegexSnapshotCache::default()));
    let mut snapshot = cache.lock().expect("regex snapshot cache mutex");
    if snapshot.keys != keys {
        snapshot.regexes = keys
            .iter()
            .map(|key| {
                Regex::new(&format!(r"(?i)\b{}\b", regex::escape(key)))
                    .expect("escaped term regex")
            })
            .collect();
        snapshot.keys = keys.to_vec();
        #[cfg(test)]
        {
            snapshot.builds += 1;
        }
    }
    snapshot.regexes.clone()
}

fn re(pattern: &str) -> Regex {
    let cache = REGEX_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = cache.lock().expect("regex cache mutex");
    if let Some(r) = map.get(pattern) {
        return r.clone();
    }
    let compiled = Regex::new(pattern).expect("valid pipeline regex");
    map.insert(pattern.to_string(), compiled.clone());
    compiled
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlIntent {
    Stop,
    Send,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlCommand {
    pub clean_text: String,
    pub wake_word: String,
    pub verb: String,
    pub intent: ControlIntent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiTransformIntent {
    Prompt,
    Summarize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiTransformCommand {
    pub clean_text: String,
    pub intent: AiTransformIntent,
}

fn control_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?ix)
            (?:^|[\s,.;:!?-]+)
            (?P<wake>
                zephyr|zephir|zefer|zeffer|zepher|zephar|zephyrs|zeppe|zepper|zeppa|
                zaffer|zapper|zipper|zifr|zeppr|зэфир|зафир|зэфер|зэфэр|завер
            )
            [\s,.;:!?-]+
            (?P<verb>
                stopped|stop|stahp|stab|stuff|stap|step|stock|stuck|
                cancelled|canceled|cancel|cansel|cancle|
                sent|send|sen|sand|said|says|say|sea|entered|enter
            )
            [\s,.;:!?-]*$",
        )
        .expect("valid control regex")
    })
}

fn legacy_stop_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?ix)(?:^|[\s,.;:!?-]+)stop[\s,.;:!?-]+(?P<verb>stop|send|enter)[\s,.;:!?-]*$")
            .expect("valid legacy stop regex")
    })
}

fn legacy_compact_stop_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?ix)(?:^|[\s,.;:!?-]+)stop(?P<verb>send|enter)[\s,.;:!?-]*$")
            .expect("valid compact legacy stop regex")
    })
}

pub fn parse_control_command(text: &str) -> Option<ControlCommand> {
    let caps = control_re()
        .captures(text)
        .or_else(|| legacy_stop_re().captures(text))
        .or_else(|| legacy_compact_stop_re().captures(text));
    let Some(caps) = caps else {
        return parse_fuzzy_zephyr_tail_command(text)
            .or_else(|| parse_compact_fuzzy_zephyr_tail_command(text));
    };
    let m = caps.get(0)?;
    let verb = caps.name("verb")?.as_str().to_ascii_lowercase();
    let wake_word = caps
        .name("wake")
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "stop".to_string());
    let intent = normalized_control_intent(&verb).unwrap_or(ControlIntent::Stop);
    let clean_text = text[..m.start()]
        .trim_matches(|c: char| c.is_whitespace() || ",.;:!?-".contains(c))
        .to_string();
    Some(ControlCommand {
        clean_text,
        wake_word,
        verb,
        intent,
    })
}

pub fn parse_ai_transform_command(text: &str) -> Option<AiTransformCommand> {
    let stripped = text
        .trim()
        .trim_end_matches(|c: char| c.is_whitespace() || ",.;:!?".contains(c));
    if stripped.is_empty() {
        return None;
    }
    const TAILS: &[(&str, AiTransformIntent)] = &[
        ("please summarize this", AiTransformIntent::Summarize),
        ("please summarise this", AiTransformIntent::Summarize),
        ("please summarize", AiTransformIntent::Summarize),
        ("please summarise", AiTransformIntent::Summarize),
        ("summarize this", AiTransformIntent::Summarize),
        ("summarise this", AiTransformIntent::Summarize),
        ("summarize", AiTransformIntent::Summarize),
        ("summarise", AiTransformIntent::Summarize),
        ("prompt", AiTransformIntent::Prompt),
    ];
    let lower = stripped.to_ascii_lowercase();
    for (tail, intent) in TAILS {
        if lower == *tail {
            return None;
        }
        let Some(body_len) = lower.strip_suffix(tail).map(|body| body.len()) else {
            continue;
        };
        let before = lower[..body_len].chars().next_back()?;
        if before.is_alphanumeric() {
            continue;
        }
        let clean_text = stripped[..body_len]
            .trim_matches(|c: char| c.is_whitespace() || ",.;:!?-".contains(c))
            .to_string();
        if clean_text.is_empty() {
            return None;
        }
        return Some(AiTransformCommand {
            clean_text,
            intent: *intent,
        });
    }
    None
}

/// Resolve a standalone transform command against an explicit text selection.
///
/// This is deliberately separate from `parse_ai_transform_command`: a bare
/// "prompt" or "summarize" must remain ordinary dictation when no text is
/// selected. The caller owns selection capture and replacement delivery.
pub fn parse_selected_text_ai_transform_command(
    command_text: &str,
    selected_text: Option<&str>,
) -> Option<AiTransformCommand> {
    let selected_text = selected_text?.trim();
    if selected_text.is_empty() {
        return None;
    }
    let command = command_text
        .trim()
        .trim_end_matches(|c: char| c.is_whitespace() || ",.;:!?".contains(c))
        .to_ascii_lowercase();
    // Selection lane is summarize-ONLY (Adrian, 2026-07-16): nobody selects
    // text and says "prompt" — that intent stays a dictation-tail command.
    let intent = match command.as_str() {
        "summarize" | "summarise" => AiTransformIntent::Summarize,
        _ => return None,
    };
    Some(AiTransformCommand {
        clean_text: selected_text.to_string(),
        intent,
    })
}

/// True when the dictated utterance is exactly the bare selection-transform
/// trigger ("summarize"/"summarise", trailing punctuation tolerated). Used by
/// the engine's copy-fallback: read-only selections (chat transcripts, web
/// pages) are invisible to the focused-control UIA read, so when the utterance
/// is the bare trigger but no selection was captured, the engine fetches the
/// selection via a clipboard-preserving Ctrl+C. Reuses the parser with a dummy
/// selection so the trigger normalization can never drift from it.
pub fn is_bare_summarize_trigger(command_text: &str) -> bool {
    parse_selected_text_ai_transform_command(command_text, Some("x")).is_some()
}

#[derive(Debug)]
struct TailToken {
    text: String,
    start: usize,
}

fn tail_word_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)[a-z]+").expect("valid tail word regex"))
}

fn tail_tokens(text: &str) -> Vec<TailToken> {
    let tail_start = text
        .char_indices()
        .rev()
        .nth(80)
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    tail_word_re()
        .find_iter(&text[tail_start..])
        .map(|m| TailToken {
            text: m.as_str().to_ascii_lowercase(),
            start: tail_start + m.start(),
        })
        .collect()
}

fn normalized_control_intent(verb: &str) -> Option<ControlIntent> {
    match verb {
        // Include common ASR near-homophones — Parakeet flips "send"↔"sent" at a
        // phrase end constantly (confirmed from the autofire log), drops the final
        // consonants ("send"→"sea", observed 2026-07-01), and similarly mangles the
        // past tenses. The "zephyr <verb>$" anchor keeps false positives in normal
        // dictation negligible.
        "send" | "sent" | "sen" | "sand" | "said" | "says" | "say" | "sea" | "enter"
        | "entered" => Some(ControlIntent::Send),
        "cancel" | "canceled" | "cancelled" | "cansel" | "cancle" => Some(ControlIntent::Cancel),
        // "stuff": Parakeet flips "stop"→"stuff" mid-utterance (confirmed from the
        // autofire log 2026-07-01 — "Zephyr stuff" repeated before a clean "Zephyr
        // stop" finally fired, so the user had to say it twice).
        "stop" | "stopped" | "stahp" | "stab" | "stuff" | "stap" | "step" | "stock" | "stuck" => {
            Some(ControlIntent::Stop)
        }
        _ => None,
    }
}

fn is_fuzzy_zephyr_word(word: &str) -> bool {
    if matches!(
        word,
        "zefyr"
            | "zefir"
            | "zeffir"
            | "zeffyr"
            | "zephir"
            | "zefer"
            | "zefe"
            | "zeffer"
            | "zepher"
            | "zephar"
            | "zephier"
            | "zephyer"
            | "zephyr"
            | "zephyrs"
            | "zeppe"
            | "zepper"
            | "zeppr"
            | "zeppa"
            | "zeppi"
            | "zeppy"
            | "zaffer"
            | "zapper"
            | "zipper"
            | "zifr"
    ) {
        return true;
    }
    word.starts_with('z') && (4..=8).contains(&word.len()) && levenshtein_ascii(word, "zephyr") <= 2
}

fn parse_compact_fuzzy_zephyr_tail_command(text: &str) -> Option<ControlCommand> {
    const VERBS: &[&str] = &[
        "cancelled",
        "canceled",
        "stopped",
        "cancel",
        "cansel",
        "cancle",
        "entered",
        "stuff",
        "stock",
        "stuck",
        "stahp",
        "stab",
        "stap",
        "step",
        "stop",
        "sent",
        "send",
        "sand",
        "said",
        "says",
        "enter",
        "sen",
        "say",
        "sea",
    ];

    let tokens = tail_tokens(text);
    let compact = tokens.last()?;
    for verb in VERBS {
        let Some(wake) = compact.text.strip_suffix(verb) else {
            continue;
        };
        if !is_fuzzy_zephyr_word(wake) {
            continue;
        }
        let intent = normalized_control_intent(verb)?;
        let clean_text = text[..compact.start]
            .trim_matches(|c: char| c.is_whitespace() || ",.;:!?-".contains(c))
            .to_string();
        return Some(ControlCommand {
            clean_text,
            wake_word: wake.to_string(),
            verb: (*verb).to_string(),
            intent,
        });
    }
    None
}

/// True when the current ASR tail ends in a bare Zephyr variant and is therefore
/// one word away from becoming a recording control. The streaming worker uses
/// this only as a scheduling hint: it does not stop capture or strip text until
/// `parse_control_command` also recognizes the verb.
pub fn has_trailing_control_wake(text: &str) -> bool {
    tail_tokens(text)
        .last()
        .is_some_and(|token| is_fuzzy_zephyr_word(&token.text))
}

fn levenshtein_ascii(a: &str, b: &str) -> usize {
    let mut costs: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.bytes().enumerate() {
        let mut prev = i;
        costs[0] = i + 1;
        for (j, cb) in b.bytes().enumerate() {
            let old = costs[j + 1];
            costs[j + 1] = if ca == cb {
                prev
            } else {
                1 + prev.min(costs[j]).min(costs[j + 1])
            };
            prev = old;
        }
    }
    costs[b.len()]
}

fn fuzzy_control_intent_after_wake(verb: &str) -> Option<ControlIntent> {
    if let Some(intent) = normalized_control_intent(verb) {
        return Some(intent);
    }

    // Observed/credible Parakeet phrase-end confusions. These deliberately live
    // behind a confirmed Zephyr token; treating these words as controls globally
    // would corrupt ordinary dictation.
    match verb {
        "slop" | "soft" => return Some(ControlIntent::Stop),
        "and" | "zen" => return Some(ControlIntent::Send),
        _ => {}
    }

    // Accept a single edit for short actions and two for the longer "cancel".
    // Require a unique nearest intent so an ambiguous tail never fires.
    let candidates = [
        ("stop", ControlIntent::Stop, 1usize),
        ("send", ControlIntent::Send, 1usize),
        ("cancel", ControlIntent::Cancel, 2usize),
    ];
    let mut best: Option<(usize, ControlIntent)> = None;
    let mut tied = false;
    for (canonical, intent, limit) in candidates {
        let distance = levenshtein_ascii(verb, canonical);
        if distance > limit {
            continue;
        }
        match best {
            None => {
                best = Some((distance, intent));
                tied = false;
            }
            Some((best_distance, _)) if distance < best_distance => {
                best = Some((distance, intent));
                tied = false;
            }
            Some((best_distance, _)) if distance == best_distance => tied = true,
            _ => {}
        }
    }
    (!tied).then_some(best?.1)
}

fn parse_fuzzy_zephyr_tail_command(text: &str) -> Option<ControlCommand> {
    let tokens = tail_tokens(text);
    let verb = tokens.last()?;
    let wake = tokens.get(tokens.len().checked_sub(2)?)?;
    if !is_fuzzy_zephyr_word(&wake.text) {
        return None;
    }
    let intent = fuzzy_control_intent_after_wake(&verb.text)?;
    let clean_text = text[..wake.start]
        .trim_matches(|c: char| c.is_whitespace() || ",.;:!?-".contains(c))
        .to_string();
    Some(ControlCommand {
        clean_text,
        wake_word: wake.text.clone(),
        verb: verb.text.clone(),
        intent,
    })
}

fn trim_command_punct(text: &str) -> String {
    text.trim_matches(|c: char| c.is_whitespace() || ",.;:!?-".contains(c))
        .to_string()
}

/// Strip the trailing "`<wake> <verb>`" control tail when a control was ALREADY
/// recognized upstream — i.e. the streaming probe fired on it and passed us the
/// `intent`. Only identifiable command residue may be removed. The streaming
/// fire proves the audio contained a command, but it does not prove which final
/// transcript tokens represent it: the full-buffer decode can omit or rewrite
/// the trigger. Preserving an unrecognized suffix is therefore mandatory to
/// avoid deleting dictated content.
pub fn strip_fired_control_tail(text: &str, intent: ControlIntent) -> String {
    if let Some(cmd) = parse_control_command(text) {
        if cmd.intent == intent {
            return scrub_command_tokens(&cmd.clean_text);
        }
    }
    let tokens = tail_tokens(text);
    let Some(verb_tok) = tokens.last() else {
        return trim_command_punct(text);
    };
    if is_fuzzy_zephyr_word(&verb_tok.text) {
        let without_bare_wake = trim_command_punct(&text[..verb_tok.start]);
        if let Some(cmd) = parse_control_command(&without_bare_wake) {
            if cmd.intent == intent {
                return scrub_command_tokens(&cmd.clean_text);
            }
        }
        return without_bare_wake;
    }
    if let Some(start) = truncated_verb_tail_start(&tokens) {
        return trim_joiner_punct(&text[..start]);
    }
    trim_command_punct(text)
}

/// Trim only what joined the command onto the sentence — whitespace and a
/// dangling comma/dash — while KEEPING terminal sentence punctuation.
///
/// `trim_command_punct` also eats `.!?`, which is right when the punctuation is
/// command noise but wrong here: in "What is broken? Zephyr s" the question mark
/// is the user's own sentence, and cutting the command must not silently take it.
fn trim_joiner_punct(text: &str) -> String {
    text.trim_end_matches(|c: char| c.is_whitespace() || ",;:-".contains(c))
        .to_string()
}

/// Longest trailing fragment treated as a chopped command verb, in tokens and
/// in characters. "stop"/"send"/"cancel" clipped mid-word give at most a couple
/// of very short tokens; real dictation after the trigger word does not.
const TRUNCATED_VERB_MAX_TOKENS: usize = 2;
const TRUNCATED_VERB_MAX_CHARS: usize = 6;

/// Byte offset at which to cut when the tail is an identifiable TRIGGER word
/// followed by a chopped verb — the "Zephyr s" shape.
///
/// (Terminology: `zephyr` is the trigger word. The `wake`/`wake_word` naming in
/// the surrounding legacy identifiers predates that distinction — nothing here
/// wakes an idle app, because the acoustic KWS path is not wired in this build.)
///
/// Field case 2026-07-27: the user said "zephyr stop". The probe lane (context
/// bias 5.0) recognized it and stopped the recording, but the full-buffer decode
/// (bias 1.0) rendered the same audio as "Zephyr s". `parse_control_command`
/// cannot parse "s" as a verb, and the bare-trigger branch above only fires when
/// the trigger is the LAST token, so neither path matched and "Zephyr s" was
/// delivered into the user's text.
///
/// This is safe precisely because it is narrow. The caller only reaches
/// `strip_fired_control_tail` after a control has ALREADY fired, so the audio is
/// known to end in a command; the trigger token is independently identifiable;
/// and the residue after it is capped at a couple of very short tokens, so
/// ordinary speech that merely mentions the trigger word ("the zephyr
/// constellation is bright") keeps every word — its trailing content is far too
/// long to qualify.
fn truncated_verb_tail_start(tokens: &[TailToken]) -> Option<usize> {
    let (trigger_index, trigger_tok) = tokens
        .iter()
        .enumerate()
        .rev()
        .find(|(_, tok)| is_fuzzy_zephyr_word(&tok.text))?;
    let residue = &tokens[trigger_index + 1..];
    if residue.is_empty() || residue.len() > TRUNCATED_VERB_MAX_TOKENS {
        return None;
    }
    let residue_chars: usize = residue.iter().map(|tok| tok.text.chars().count()).sum();
    if residue_chars > TRUNCATED_VERB_MAX_CHARS {
        return None;
    }
    Some(trigger_tok.start)
}

/// Strip a single trailing BARE wake word ("… zephyr" with no command verb yet).
/// The streaming committer commits this tail BEFORE the verb lands, so without this
/// it would be sent to the L3 LLM (prewarm) and leak the wake word into the polished
/// prefix. Returns the trimmed text when a trailing wake token was removed, else None.
/// Safe: the wake word (and its near-homophones) is never normal dictation.
fn strip_trailing_bare_wake(text: &str) -> Option<String> {
    let tokens = tail_tokens(text);
    let last = tokens.last()?;
    if !is_fuzzy_zephyr_word(&last.text) {
        return None;
    }
    Some(trim_command_punct(&text[..last.start]))
}

/// Strip a STANDALONE bare AI-transform trigger ("prompt" / "summarize" on its own).
/// `parse_ai_transform_command` returns None for a bare trigger (it needs content to
/// transform), but for the L0 scrub a lone trigger word is a command marker, not
/// dictation — so drop it. Content ending in a trigger ("write me a prompt") is
/// already handled by `parse_ai_transform_command`, which keeps the content.
fn strip_trailing_bare_transform(text: &str) -> Option<String> {
    match trim_command_punct(text).to_ascii_lowercase().as_str() {
        "prompt" | "summarize" | "summarise" => Some(String::new()),
        _ => None,
    }
}

/// Canonical **L0** scrub: remove every wake / control / AI-transform token from a
/// transcript (or partial) so they can NEVER reach the L3 LLM or the delivered text.
/// Idempotent; iterates because tokens stack at the tail ("… prompt, zephyr stop").
/// Used by BOTH the streaming-polish prewarm (pre-LLM) and finalize, so the two paths
/// can't drift. Removes: a complete "zephyr <verb>" control tail, a complete
/// "<text> prompt|summarize" transform tail, and a trailing bare wake word. It does
/// NOT strip bare control verbs (e.g. a legitimately dictated "send"/"stop") — those
/// only count as commands when preceded by the wake word.
pub fn scrub_command_tokens(text: &str) -> String {
    let mut current = trim_command_punct(text);
    loop {
        if let Some(cmd) = parse_control_command(&current) {
            if cmd.clean_text.len() < current.len() {
                current = cmd.clean_text;
                continue;
            }
        }
        if let Some(cmd) = parse_ai_transform_command(&current) {
            if cmd.clean_text.len() < current.len() {
                current = cmd.clean_text;
                continue;
            }
        }
        if let Some(stripped) = strip_trailing_bare_wake(&current) {
            if stripped.len() < current.len() {
                current = stripped;
                continue;
            }
        }
        if let Some(stripped) = strip_trailing_bare_transform(&current) {
            if stripped.len() < current.len() {
                current = stripped;
                continue;
            }
        }
        break;
    }
    current
}
