//! Real shorts extraction (REV2 plan §15.4 Phase 6).
//!
//! Replaces duration-then-take-rank ranking with a nine-stage pipeline that
//! works entirely from artifacts already produced upstream in this project
//! — the transcript, VAD regions, and the editorial candidate manifest — so
//! it never makes a cloud call and never depends on a new model. Where a
//! judgment genuinely needs semantics those artifacts cannot provide (most
//! notably visual support: whether a clip has anything worth watching), the
//! score is deliberately conservative and its `confidence` is recorded as
//! `Low` rather than inventing certainty.
//!
//! Pipeline stages, in order:
//! 1. [`segment_semantic_units`] — semantic segmentation of accepted
//!    candidates into contiguous, source-scoped units.
//! 2. [`validate_standalone`] — standalone-context validation (rejects a
//!    unit that opens on a dangling reference).
//! 3. [`score_hook`], [`score_payoff`], [`score_proof`], [`score_value`] —
//!    scored independently and recorded separately, never collapsed into
//!    one number before the caller sees the breakdown.
//! 4. [`score_duration_fit`] — fit against the platform's target window.
//! 5. [`score_visual_support`] — always `Low` confidence today; there is no
//!    Phase 7 visual-perception artifact for this stage to read yet.
//! 6. [`score_platform_fit`] / [`score_brand_fit`] — platform and brand
//!    fit.
//! 7. [`cluster_and_select`] — diversity clustering so near-paraphrases of
//!    the same point cannot occupy every slot.
//! 8. [`build_preview`] — a cheap preview (poster timestamp + text
//!    snippet) derived from already-loaded data, never a render call.
//! 9. Human selection: [`ShortsCandidateRecord::selected_id`] semantics
//!    live in `shorts.rs`'s output artifact — this module never marks a
//!    candidate selected. The pipeline proposes; a human picks.
//!
//! Every candidate this pipeline proposes also carries a
//! [`TruthfulnessNote`]: whether assembling it reordered or omitted source
//! material in a way that could change what was said, per [`build_shorts`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::io::{read_json_if_file, write_json_atomic};
use crate::ProjectError;
use video_core::models::{Candidate, OutputPreset};
use video_core::{Transcript, VadSignal};

pub const SHORTS_PROFILE_SCHEMA_VERSION: u32 = 1;
pub const SHORTS_SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------
// Versioned scoring profile (weights/thresholds live here, not scattered
// constants — REV2 plan §15.4).
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlatformWindow {
    pub id: String,
    pub min_duration_ms: i64,
    pub ideal_min_ms: i64,
    pub ideal_max_ms: i64,
    pub max_duration_ms: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ScoreWeights {
    pub hook: f64,
    pub payoff: f64,
    pub proof: f64,
    pub value: f64,
    pub duration_fit: f64,
    pub visual_support: f64,
    pub platform_fit: f64,
    pub brand_fit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShortsScoringProfile {
    pub schema_version: u32,
    /// Bumped whenever the defaults below change; recorded on every shorts
    /// proposal so a reprocessed manifest never silently reuses a
    /// candidate set built under different weights/thresholds.
    pub profile_version: u32,
    pub platform: PlatformWindow,
    pub weights: ScoreWeights,
    /// Token-Jaccard similarity at/above which two candidate units are
    /// treated as near-duplicates for diversity clustering.
    pub diversity_similarity_threshold: f64,
    /// Max gap (ms) between two adjacent accepted candidates on the same
    /// source that still lets them merge into one semantic unit.
    pub max_group_gap_ms: i64,
    /// Lower-cased opener tokens that mark a dangling reference to
    /// preceding context ("and that's why it matters" fails standalone
    /// validation because "and" refers back to something this clip does
    /// not contain).
    pub dangling_openers: Vec<String>,
    pub hook_cues: Vec<String>,
    pub payoff_cues: Vec<String>,
    pub proof_cues: Vec<String>,
    /// Empty by default: brand-safety terms are per-project, not invented
    /// here. An empty list means brand fit cannot be verified from local
    /// data and is scored neutrally with `Low` confidence.
    pub brand_banned_terms: Vec<String>,
}

impl Default for ShortsScoringProfile {
    fn default() -> Self {
        Self {
            schema_version: SHORTS_PROFILE_SCHEMA_VERSION,
            profile_version: 1,
            platform: PlatformWindow {
                id: "vertical-short".into(),
                min_duration_ms: 5_000,
                ideal_min_ms: 15_000,
                ideal_max_ms: 45_000,
                max_duration_ms: 90_000,
            },
            weights: ScoreWeights {
                hook: 0.20,
                payoff: 0.15,
                proof: 0.10,
                value: 0.20,
                duration_fit: 0.15,
                visual_support: 0.05,
                platform_fit: 0.10,
                brand_fit: 0.05,
            },
            diversity_similarity_threshold: 0.6,
            max_group_gap_ms: 600,
            dangling_openers: [
                "and",
                "but",
                "so",
                "also",
                "then",
                "because",
                "which",
                "this",
                "that",
                "it",
                "he",
                "she",
                "they",
                "therefore",
                "however",
                "plus",
                "meanwhile",
                "otherwise",
                "anyway",
                "besides",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            hook_cues: [
                "never",
                "always",
                "secret",
                "mistake",
                "stop",
                "wait",
                "here's",
                "imagine",
                "what if",
                "did you know",
                "nobody tells you",
                "truth is",
                "biggest",
                "worst",
                "best",
                "no one",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            payoff_cues: [
                "so",
                "that's how",
                "that's why",
                "which means",
                "the result",
                "in the end",
                "here's why",
                "bottom line",
                "turns out",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            proof_cues: [
                "because",
                "study",
                "data",
                "results",
                "for example",
                "case",
                "percent",
                "research",
                "proven",
                "tested",
                "measured",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            brand_banned_terms: Vec::new(),
        }
    }
}

pub(crate) fn shorts_profile_path(project_path: &Path) -> PathBuf {
    project_path.join("edit/shorts-profile.json")
}

/// Load the project's persisted shorts scoring profile, or initialize and
/// persist the versioned default the first time this stage runs. Once
/// written, a profile is never silently rewritten by a later run with
/// different defaults — only an explicit edit (or a code change bumping
/// `profile_version` for a fresh project) changes it.
pub(crate) fn load_or_init_shorts_profile(
    project_path: &Path,
) -> Result<ShortsScoringProfile, ProjectError> {
    let path = shorts_profile_path(project_path);
    if let Some(profile) = read_json_if_file::<ShortsScoringProfile>(&path) {
        return Ok(profile);
    }
    let profile = ShortsScoringProfile::default();
    write_json_atomic(&path, &profile)?;
    Ok(profile)
}

// ---------------------------------------------------------------------
// Output record shapes
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Score {
    pub value: f64,
    pub confidence: Confidence,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoreBreakdown {
    pub hook: Score,
    pub payoff: Score,
    pub proof: Score,
    pub value: Score,
    pub duration_fit: Score,
    pub visual_support: Score,
    pub platform_fit: Score,
    pub brand_fit: Score,
    pub composite: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StandaloneCheck {
    pub passed: bool,
    pub confidence: Confidence,
    pub reasons: Vec<String>,
}

/// A correctness obligation, not decoration (REV2 plan §15.4): states
/// whether assembling this candidate from the underlying candidates
/// reordered or omitted source material in a way that could change what
/// was said.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TruthfulnessNote {
    pub reorders_or_omits_material: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Preview {
    pub poster_source_id: String,
    pub poster_ms: i64,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceMappingEntry {
    pub candidate_id: String,
    pub source_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShortsCandidateRecord {
    pub id: String,
    pub source_mapping: Vec<SourceMappingEntry>,
    pub start_ms: i64,
    pub end_ms: i64,
    pub duration_ms: i64,
    pub transcript: String,
    pub rationale: String,
    pub scores: ScoreBreakdown,
    pub standalone: StandaloneCheck,
    pub truthfulness: TruthfulnessNote,
    pub preview: Preview,
    pub diversity_cluster: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RejectedUnit {
    pub unit_id: String,
    pub source_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub stage: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShortsBuildResult {
    pub variants: Vec<ShortsCandidateRecord>,
    pub rejected: Vec<RejectedUnit>,
    pub candidates_considered: usize,
}

// ---------------------------------------------------------------------
// Internal working type: one semantic unit before scoring.
// ---------------------------------------------------------------------

pub(crate) struct SemanticUnit {
    unit_id: String,
    source_id: String,
    members: Vec<Candidate>,
}

impl SemanticUnit {
    fn start_ms(&self) -> i64 {
        self.members.first().map(|c| c.start_ms).unwrap_or(0)
    }

    fn end_ms(&self) -> i64 {
        self.members.last().map(|c| c.end_ms).unwrap_or(0)
    }

    fn duration_ms(&self) -> i64 {
        (self.end_ms() - self.start_ms()).max(0)
    }

    fn text(&self) -> String {
        self.members
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// The VAD-confirmed gap (ms) between two adjacent candidates, snapped to
/// real detected speech boundaries rather than trusted transcript word
/// timestamps. Word timing can understate a true pause (e.g. a word's
/// recorded `end_ms` overshoots where speech actually stopped); this looks
/// for the VAD speech region active at/around `prev_end_ms` and, if it
/// actually ended earlier, uses that earlier true end. Symmetrically for
/// `next_start_ms`. The result can therefore exceed the nominal
/// `next_start_ms - prev_end_ms` word gap, which is exactly the case this
/// exists to catch; it is never used to shrink a gap the words already show.
fn vad_confirmed_gap_ms(vad: &VadSignal, prev_end_ms: i64, next_start_ms: i64) -> i64 {
    let true_prev_end = vad
        .regions
        .iter()
        .filter(|region| region.start_ms <= prev_end_ms)
        .map(|region| region.end_ms)
        .max()
        .filter(|end| *end < prev_end_ms)
        .unwrap_or(prev_end_ms);
    let true_next_start = vad
        .regions
        .iter()
        .filter(|region| region.end_ms >= next_start_ms)
        .map(|region| region.start_ms)
        .min()
        .filter(|start| *start > next_start_ms)
        .unwrap_or(next_start_ms);
    (true_next_start - true_prev_end).max(next_start_ms - prev_end_ms)
}

/// Stage 1: semantic segmentation. Groups accepted (non-dropped) candidates
/// per source, in time order, into contiguous units. A unit grows while the
/// gap to the next candidate stays within `max_group_gap_ms` (optionally
/// corroborated by a genuine VAD silence of at least that length) and while
/// growing would not exceed the platform's `max_duration_ms`; otherwise the
/// next candidate starts a new unit. Deterministic: the same candidate
/// manifest always produces the same units in the same order.
pub(crate) fn segment_semantic_units(
    candidates: &[Candidate],
    vad_by_source: &BTreeMap<String, VadSignal>,
    profile: &ShortsScoringProfile,
) -> Vec<SemanticUnit> {
    let mut by_source: BTreeMap<String, Vec<Candidate>> = BTreeMap::new();
    for candidate in candidates {
        if candidate.drop_reason.is_some() {
            continue;
        }
        by_source
            .entry(candidate.source_id.clone())
            .or_default()
            .push(candidate.clone());
    }

    let mut units = Vec::new();
    let mut unit_index = 0usize;
    for (source_id, mut source_candidates) in by_source {
        source_candidates.sort_by_key(|c| c.start_ms);
        let vad = vad_by_source.get(&source_id);
        let mut current: Vec<Candidate> = Vec::new();
        for candidate in source_candidates {
            if let Some(previous) = current.last() {
                let gap_ms = candidate.start_ms - previous.end_ms;
                let unit_start = current
                    .first()
                    .map(|c| c.start_ms)
                    .unwrap_or(candidate.start_ms);
                let would_be_duration = candidate.end_ms - unit_start;
                let confirmed_gap_ms = vad
                    .map(|signal| vad_confirmed_gap_ms(signal, previous.end_ms, candidate.start_ms))
                    .unwrap_or(gap_ms);
                let breaks = gap_ms > profile.max_group_gap_ms
                    || confirmed_gap_ms > profile.max_group_gap_ms
                    || would_be_duration > profile.platform.max_duration_ms;
                if breaks {
                    unit_index += 1;
                    units.push(SemanticUnit {
                        unit_id: format!("unit-{unit_index:03}"),
                        source_id: source_id.clone(),
                        members: std::mem::take(&mut current),
                    });
                }
            }
            current.push(candidate);
        }
        if !current.is_empty() {
            unit_index += 1;
            units.push(SemanticUnit {
                unit_id: format!("unit-{unit_index:03}"),
                source_id: source_id.clone(),
                members: current,
            });
        }
    }
    units
}

/// First non-empty, punctuation-stripped, lower-cased token of `text`.
fn first_token(text: &str) -> Option<String> {
    text.split_whitespace()
        .next()
        .map(|token| {
            token
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|token| !token.is_empty())
}

/// Stage 2: standalone-context validation. A unit whose opening token is a
/// known dangling reference (e.g. "and", "that's why") fails outright —
/// that is a deterministic lexicon match, recorded at `High` confidence.
/// Passing is recorded at `Medium` confidence: absence of a known bad
/// opener does not prove the clip truly stands alone, which is a semantic
/// judgment this lexicon cannot fully make.
pub(crate) fn validate_standalone(text: &str, profile: &ShortsScoringProfile) -> StandaloneCheck {
    match first_token(text) {
        Some(opener) if profile.dangling_openers.contains(&opener) => StandaloneCheck {
            passed: false,
            confidence: Confidence::High,
            reasons: vec![format!(
                "opens on \"{opener}\", a dangling reference to context this clip does not contain"
            )],
        },
        Some(_) => StandaloneCheck {
            passed: true,
            confidence: Confidence::Medium,
            reasons: vec!["no known dangling-reference opener detected".into()],
        },
        None => StandaloneCheck {
            passed: false,
            confidence: Confidence::High,
            reasons: vec!["empty transcript text".into()],
        },
    }
}

fn contains_cue(lower_text: &str, cues: &[String]) -> Vec<String> {
    cues.iter()
        .filter(|cue| lower_text.contains(cue.as_str()))
        .cloned()
        .collect()
}

/// Stage 3a: hook score. Rewards an opening question, a lexicon hook cue
/// anywhere in the unit, and the upstream editorial `beat_label == "hook"`
/// signal recorded when candidates were built. `High` confidence when a
/// strong structural signal (question mark, or the upstream hook label) is
/// present; `Medium` otherwise, since a lexicon match alone is a heuristic.
pub(crate) fn score_hook(text: &str, beat_label: &str, profile: &ShortsScoringProfile) -> Score {
    let lower = text.to_lowercase();
    let has_question = text.contains('?');
    let cues = contains_cue(&lower, &profile.hook_cues);
    let is_labeled_hook = beat_label == "hook";

    let mut value: f64 = 0.0;
    let mut signals = Vec::new();
    if has_question {
        value += 0.4;
        signals.push("opens with a question".to_string());
    }
    if !cues.is_empty() {
        value += 0.4;
        signals.push(format!("hook cue(s): {}", cues.join(", ")));
    }
    if is_labeled_hook {
        value += 0.2;
        signals.push("upstream beat_label is \"hook\"".to_string());
    }
    let value = value.min(1.0);

    // Any lexicon/cue match still leaves the score a heuristic, so both the
    // "cue found" and "no signal at all" cases land at `Medium` — only the
    // strong structural signals above earn `High`.
    let confidence = if has_question || is_labeled_hook {
        Confidence::High
    } else {
        Confidence::Medium
    };
    let rationale = if signals.is_empty() {
        "no hook signal detected (no question mark, hook-cue lexicon match, or upstream hook label)"
            .to_string()
    } else {
        signals.join("; ")
    };
    Score {
        value,
        confidence,
        rationale,
    }
}

/// Stage 3b: payoff score. Rewards ending on terminal punctuation
/// (structural completeness), a payoff-cue phrase near the end, and not
/// trailing on a coordinating conjunction. `High` confidence when the
/// ending punctuation is unambiguous; `Medium` otherwise.
pub(crate) fn score_payoff(text: &str, profile: &ShortsScoringProfile) -> Score {
    let trimmed = text.trim();
    let lower = trimmed.to_lowercase();
    let ends_complete = trimmed.ends_with(['.', '!', '?']);
    let tail: String = lower
        .split_whitespace()
        .rev()
        .take(12)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" ");
    let cues = contains_cue(&tail, &profile.payoff_cues);
    let last_word = lower
        .split_whitespace()
        .last()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .unwrap_or_default();
    let trails_on_conjunction =
        ["and", "but", "so", "because", "which"].contains(&last_word.as_str());

    let mut value: f64 = 0.0;
    let mut signals = Vec::new();
    if ends_complete {
        value += 0.4;
        signals.push("ends on terminal punctuation".to_string());
    }
    if !cues.is_empty() {
        value += 0.4;
        signals.push(format!("payoff cue(s): {}", cues.join(", ")));
    }
    if !trails_on_conjunction {
        value += 0.2;
    } else {
        signals.push(format!("trails on \"{last_word}\" — likely mid-clause"));
    }
    let value = value.min(1.0);
    let confidence = if ends_complete {
        Confidence::High
    } else {
        Confidence::Medium
    };
    let rationale = if signals.is_empty() {
        "no payoff signal detected".to_string()
    } else {
        signals.join("; ")
    };
    Score {
        value,
        confidence,
        rationale,
    }
}

/// Stage 3c: proof score. Rewards digits and proof-cue phrases. Kept at
/// `Low` confidence: whether a clip actually substantiates its claim is a
/// semantic judgment a lexicon cannot make reliably, so this stage
/// deliberately under-claims certainty even when it finds a signal.
pub(crate) fn score_proof(text: &str, profile: &ShortsScoringProfile) -> Score {
    let lower = text.to_lowercase();
    let has_digit = text.chars().any(|c| c.is_ascii_digit());
    let cues = contains_cue(&lower, &profile.proof_cues);

    let mut value: f64 = 0.0;
    let mut signals = Vec::new();
    if has_digit {
        value += 0.5;
        signals.push("contains a number".to_string());
    }
    if !cues.is_empty() {
        value += 0.5;
        signals.push(format!("proof cue(s): {}", cues.join(", ")));
    }
    let value = value.min(1.0);
    let rationale = if signals.is_empty() {
        "no proof signal detected (lexicon-only heuristic; may under-detect real evidence)"
            .to_string()
    } else {
        format!(
            "{} (lexicon-only heuristic; does not verify the claim is actually substantiated)",
            signals.join("; ")
        )
    };
    Score {
        value,
        confidence: Confidence::Low,
        rationale,
    }
}

/// Stage 3d: value/information-density score, computed from real
/// transcript word data (unique-word ratio, speaking rate, average word
/// confidence, filler ratio) — all deterministic statistics, so this is
/// recorded at `High` confidence even though the underlying judgment of
/// "value" is itself a proxy.
pub(crate) fn score_value(
    words: &[&video_core::Word],
    filler_count: usize,
    duration_ms: i64,
) -> Score {
    if words.is_empty() {
        return Score {
            value: 0.0,
            confidence: Confidence::High,
            rationale: "no transcript words found in this unit's range".into(),
        };
    }
    let word_count = words.len();
    let unique_count = words
        .iter()
        .map(|w| w.text.to_lowercase())
        .collect::<std::collections::HashSet<_>>()
        .len();
    let unique_ratio = unique_count as f64 / word_count as f64;
    let avg_confidence =
        words.iter().map(|w| f64::from(w.confidence)).sum::<f64>() / word_count as f64;
    let seconds = (duration_ms.max(1)) as f64 / 1000.0;
    let wps = word_count as f64 / seconds;
    // Ideal conversational speaking rate band; linearly decays outside it.
    let rate_score = if (2.0..=3.3).contains(&wps) {
        1.0
    } else if wps < 2.0 {
        (wps / 2.0).clamp(0.0, 1.0)
    } else {
        (1.0 - ((wps - 3.3) / 3.0)).clamp(0.0, 1.0)
    };
    let filler_ratio = filler_count as f64 / word_count as f64;
    let filler_score = (1.0 - filler_ratio).clamp(0.0, 1.0);

    let value =
        (unique_ratio * 0.3 + avg_confidence * 0.3 + rate_score * 0.25 + filler_score * 0.15)
            .clamp(0.0, 1.0);
    Score {
        value,
        confidence: Confidence::High,
        rationale: format!(
            "unique-word ratio {unique_ratio:.2}, avg word confidence {avg_confidence:.2}, \
             speaking rate {wps:.2} wps, filler ratio {filler_ratio:.2}"
        ),
    }
}

/// Stage 4: duration fit against the platform's target window. Fully
/// deterministic arithmetic, so always `High` confidence. Zero outside the
/// hard `[min_duration_ms, max_duration_ms]` bounds — that is a hard gate
/// upstream callers should treat as a rejection, not just a low score.
pub(crate) fn score_duration_fit(duration_ms: i64, platform: &PlatformWindow) -> Score {
    if duration_ms < platform.min_duration_ms || duration_ms > platform.max_duration_ms {
        return Score {
            value: 0.0,
            confidence: Confidence::High,
            rationale: format!(
                "{duration_ms}ms is outside the hard platform window [{}, {}]ms",
                platform.min_duration_ms, platform.max_duration_ms
            ),
        };
    }
    let value = if duration_ms >= platform.ideal_min_ms && duration_ms <= platform.ideal_max_ms {
        1.0
    } else if duration_ms < platform.ideal_min_ms {
        let span = (platform.ideal_min_ms - platform.min_duration_ms).max(1) as f64;
        ((duration_ms - platform.min_duration_ms) as f64 / span).clamp(0.0, 1.0)
    } else {
        let span = (platform.max_duration_ms - platform.ideal_max_ms).max(1) as f64;
        (1.0 - (duration_ms - platform.ideal_max_ms) as f64 / span).clamp(0.0, 1.0)
    };
    Score {
        value,
        confidence: Confidence::High,
        rationale: format!(
            "{duration_ms}ms against ideal window [{}, {}]ms",
            platform.ideal_min_ms, platform.ideal_max_ms
        ),
    }
}

/// Stage 5: visual support. There is no Phase 7 temporal-visual-perception
/// artifact in this project yet (that phase is built separately), so this
/// stage never invents a judgment about whether the clip has anything worth
/// watching. It always returns a neutral score at `Low` confidence.
pub(crate) fn score_visual_support() -> Score {
    Score {
        value: 0.5,
        confidence: Confidence::Low,
        rationale: "no visual-perception artifact available to this stage yet (REV2 plan §15.5 \
                    Phase 7 is separate work); scored neutrally pending that data"
            .into(),
    }
}

/// Stage 6a: platform fit — does this project declare an output preset
/// suited to short-form vertical delivery? `High` confidence when at least
/// one preset is configured (a literal, deterministic check); `Low` when
/// none are configured, since fit cannot be assessed at all.
pub(crate) fn score_platform_fit(output_presets: &[OutputPreset]) -> Score {
    if output_presets.is_empty() {
        return Score {
            value: 0.5,
            confidence: Confidence::Low,
            rationale: "project declares no output presets; platform fit cannot be determined"
                .into(),
        };
    }
    let has_vertical = output_presets.iter().any(|preset| preset.aspect == "9:16");
    Score {
        value: if has_vertical { 1.0 } else { 0.6 },
        confidence: Confidence::High,
        rationale: if has_vertical {
            "project declares a 9:16 output preset".into()
        } else {
            "project declares output preset(s) but none are 9:16".into()
        },
    }
}

/// Stage 6b: brand fit — a configurable banned-term lexicon check. An empty
/// list (the default) means brand fit cannot be verified from local data;
/// that is recorded at `Low` confidence rather than assumed safe.
pub(crate) fn score_brand_fit(text: &str, profile: &ShortsScoringProfile) -> Score {
    if profile.brand_banned_terms.is_empty() {
        return Score {
            value: 0.5,
            confidence: Confidence::Low,
            rationale: "no brand banned-term list configured for this project; brand fit cannot be verified"
                .into(),
        };
    }
    let lower = text.to_lowercase();
    let hits = contains_cue(&lower, &profile.brand_banned_terms);
    if hits.is_empty() {
        Score {
            value: 1.0,
            confidence: Confidence::High,
            rationale: "no configured banned terms present".into(),
        }
    } else {
        Score {
            value: 0.0,
            confidence: Confidence::High,
            rationale: format!("matched banned term(s): {}", hits.join(", ")),
        }
    }
}

pub(crate) fn composite_score(breakdown: &ScoreBreakdown, weights: &ScoreWeights) -> f64 {
    let total_weight = weights.hook
        + weights.payoff
        + weights.proof
        + weights.value
        + weights.duration_fit
        + weights.visual_support
        + weights.platform_fit
        + weights.brand_fit;
    if total_weight <= 0.0 {
        return 0.0;
    }
    (breakdown.hook.value * weights.hook
        + breakdown.payoff.value * weights.payoff
        + breakdown.proof.value * weights.proof
        + breakdown.value.value * weights.value
        + breakdown.duration_fit.value * weights.duration_fit
        + breakdown.visual_support.value * weights.visual_support
        + breakdown.platform_fit.value * weights.platform_fit
        + breakdown.brand_fit.value * weights.brand_fit)
        / total_weight
}

/// Token-set Jaccard similarity between two texts, lower-cased and split on
/// whitespace. Deterministic and dependency-free.
pub(crate) fn jaccard_similarity(left: &str, right: &str) -> f64 {
    let left_tokens: std::collections::HashSet<String> = left
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();
    let right_tokens: std::collections::HashSet<String> = right
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();
    if left_tokens.is_empty() && right_tokens.is_empty() {
        return 1.0;
    }
    let intersection = left_tokens.intersection(&right_tokens).count() as f64;
    let union = left_tokens.union(&right_tokens).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

/// Stage 7: diversity clustering. `scored` must already be sorted by
/// composite score descending (ties broken deterministically). Greedily
/// assigns each unit to the first existing cluster whose representative
/// (the cluster's first, i.e. highest-scored, member) is similar enough;
/// otherwise it starts a new cluster. Returns, for each input index, the
/// cluster id it was assigned to, plus the cluster membership order used
/// for diverse top-N selection.
pub(crate) fn cluster_and_select(
    texts: &[&str],
    threshold: f64,
    count: usize,
) -> (Vec<usize>, Vec<usize>) {
    let mut clusters: Vec<Vec<usize>> = Vec::new();
    let mut cluster_of = vec![0usize; texts.len()];
    for (index, text) in texts.iter().enumerate() {
        let mut joined = None;
        for (cluster_index, cluster) in clusters.iter().enumerate() {
            let representative = texts[cluster[0]];
            if jaccard_similarity(representative, text) >= threshold {
                joined = Some(cluster_index);
                break;
            }
        }
        match joined {
            Some(cluster_index) => {
                clusters[cluster_index].push(index);
                cluster_of[index] = cluster_index;
            }
            None => {
                cluster_of[index] = clusters.len();
                clusters.push(vec![index]);
            }
        }
    }

    let mut selected = Vec::new();
    let mut cursors = vec![0usize; clusters.len()];
    loop {
        let mut progressed = false;
        for (cluster_index, cluster) in clusters.iter().enumerate() {
            if selected.len() >= count {
                break;
            }
            let cursor = cursors[cluster_index];
            if cursor < cluster.len() {
                selected.push(cluster[cursor]);
                cursors[cluster_index] += 1;
                progressed = true;
            }
        }
        if selected.len() >= count || !progressed {
            break;
        }
    }
    (cluster_of, selected)
}

/// Stage 8: a cheap preview — a poster timestamp (the highest-confidence
/// word in the first two seconds, falling back to the unit start) and a
/// short text snippet. Derived entirely from already-loaded transcript
/// data; never invokes a render.
pub(crate) fn build_preview(
    source_id: &str,
    start_ms: i64,
    words: &[&video_core::Word],
    text: &str,
) -> Preview {
    let poster_ms = words
        .iter()
        .filter(|w| w.start_ms - start_ms <= 2_000)
        .max_by(|a, b| a.confidence.total_cmp(&b.confidence))
        .map(|w| w.start_ms)
        .unwrap_or(start_ms);
    let snippet = text
        .split_whitespace()
        .take(12)
        .collect::<Vec<_>>()
        .join(" ");
    Preview {
        poster_source_id: source_id.to_string(),
        poster_ms,
        snippet,
    }
}

/// Builds the [`TruthfulnessNote`] for one unit. Grounded entirely in the
/// full candidate list: a unit assembled from more than one underlying
/// accepted candidate omits whatever original material (silence, or
/// explicitly dropped takes) fell in the gap between them, which changes
/// what the resulting clip appears to say relative to the raw source.
pub(crate) fn build_truthfulness_note(
    unit: &SemanticUnit,
    all_candidates: &[Candidate],
) -> TruthfulnessNote {
    if unit.members.len() <= 1 {
        return TruthfulnessNote {
            reorders_or_omits_material: false,
            note: "single contiguous accepted candidate; no reordering or internal omission".into(),
        };
    }
    let mut omissions = Vec::new();
    for pair in unit.members.windows(2) {
        let (left, right) = (&pair[0], &pair[1]);
        let gap_ms = right.start_ms - left.end_ms;
        if gap_ms <= 0 {
            continue;
        }
        let dropped_in_gap: Vec<&str> = all_candidates
            .iter()
            .filter(|c| {
                c.source_id == unit.source_id
                    && c.drop_reason.is_some()
                    && c.start_ms >= left.end_ms
                    && c.end_ms <= right.start_ms
            })
            .map(|c| c.id.as_str())
            .collect();
        if dropped_in_gap.is_empty() {
            omissions.push(format!(
                "{gap_ms}ms gap between {} and {}",
                left.id, right.id
            ));
        } else {
            omissions.push(format!(
                "{gap_ms}ms gap between {} and {} covers dropped candidate(s) {}",
                left.id,
                right.id,
                dropped_in_gap.join(", ")
            ));
        }
    }
    if omissions.is_empty() {
        TruthfulnessNote {
            reorders_or_omits_material: false,
            note: "multiple candidates merged with no internal time gap; no omission".into(),
        }
    } else {
        TruthfulnessNote {
            reorders_or_omits_material: true,
            note: format!(
                "this clip merges {} candidates from the same source; it omits: {}",
                unit.members.len(),
                omissions.join("; ")
            ),
        }
    }
}

/// The full nine-stage pipeline (segmentation → standalone validation →
/// scoring → duration/visual/platform/brand fit → diversity clustering →
/// preview → returned for human selection). `count` is the number of
/// diverse candidates to propose (stage 8/9); every unit that fails a hard
/// gate (standalone validation or the duration hard bounds) is recorded in
/// `rejected` instead of scored further.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_shorts(
    candidates: &[Candidate],
    transcripts: &[Transcript],
    vad_by_source: &BTreeMap<String, VadSignal>,
    output_presets: &[OutputPreset],
    profile: &ShortsScoringProfile,
    count: usize,
) -> ShortsBuildResult {
    let transcripts_by_source: BTreeMap<&str, &Transcript> = transcripts
        .iter()
        .map(|t| (t.source_id.as_str(), t))
        .collect();

    let units = segment_semantic_units(candidates, vad_by_source, profile);
    let candidates_considered = units.len();

    let mut rejected = Vec::new();
    let mut scored: Vec<(ShortsCandidateRecord, TruthfulnessNote)> = Vec::new();

    for unit in &units {
        let text = unit.text();
        let standalone = validate_standalone(&text, profile);
        if !standalone.passed {
            rejected.push(RejectedUnit {
                unit_id: unit.unit_id.clone(),
                source_id: unit.source_id.clone(),
                start_ms: unit.start_ms(),
                end_ms: unit.end_ms(),
                stage: "standalone_context".into(),
                reason: standalone.reasons.join("; "),
            });
            continue;
        }

        let duration_fit = score_duration_fit(unit.duration_ms(), &profile.platform);
        if duration_fit.value <= 0.0 {
            rejected.push(RejectedUnit {
                unit_id: unit.unit_id.clone(),
                source_id: unit.source_id.clone(),
                start_ms: unit.start_ms(),
                end_ms: unit.end_ms(),
                stage: "duration_fit".into(),
                reason: duration_fit.rationale,
            });
            continue;
        }

        let words: Vec<&video_core::Word> = transcripts_by_source
            .get(unit.source_id.as_str())
            .map(|transcript| {
                transcript
                    .words
                    .iter()
                    .filter(|w| w.start_ms >= unit.start_ms() && w.end_ms <= unit.end_ms())
                    .collect()
            })
            .unwrap_or_default();
        let filler_total: usize = unit.members.iter().map(|c| c.filler_count).sum();
        let beat_label = unit
            .members
            .first()
            .map(|c| c.beat_label.as_str())
            .unwrap_or("");

        let breakdown_partial = ScoreBreakdown {
            hook: score_hook(&text, beat_label, profile),
            payoff: score_payoff(&text, profile),
            proof: score_proof(&text, profile),
            value: score_value(&words, filler_total, unit.duration_ms()),
            duration_fit,
            visual_support: score_visual_support(),
            platform_fit: score_platform_fit(output_presets),
            brand_fit: score_brand_fit(&text, profile),
            composite: 0.0,
        };
        let composite = composite_score(&breakdown_partial, &profile.weights);
        let breakdown = ScoreBreakdown {
            composite,
            ..breakdown_partial
        };

        let preview = build_preview(&unit.source_id, unit.start_ms(), &words, &text);
        let truthfulness = build_truthfulness_note(unit, candidates);
        let source_mapping = unit
            .members
            .iter()
            .map(|c| SourceMappingEntry {
                candidate_id: c.id.clone(),
                source_id: c.source_id.clone(),
                start_ms: c.start_ms,
                end_ms: c.end_ms,
            })
            .collect();
        let rationale = format!(
            "hook {:.2}, payoff {:.2}, proof {:.2}, value {:.2}, duration_fit {:.2}, \
             visual_support {:.2}, platform_fit {:.2}, brand_fit {:.2} -> composite {composite:.2}",
            breakdown.hook.value,
            breakdown.payoff.value,
            breakdown.proof.value,
            breakdown.value.value,
            breakdown.duration_fit.value,
            breakdown.visual_support.value,
            breakdown.platform_fit.value,
            breakdown.brand_fit.value,
        );

        scored.push((
            ShortsCandidateRecord {
                id: unit.unit_id.clone(),
                source_mapping,
                start_ms: unit.start_ms(),
                end_ms: unit.end_ms(),
                duration_ms: unit.duration_ms(),
                transcript: text,
                rationale,
                scores: breakdown,
                standalone,
                truthfulness: truthfulness.clone(),
                preview,
                diversity_cluster: String::new(),
            },
            truthfulness,
        ));
    }

    // Deterministic ranking: composite desc, then unit id asc as a stable
    // tiebreaker so identical scores never depend on hash/iteration order.
    scored.sort_by(|(a, _), (b, _)| {
        b.scores
            .composite
            .partial_cmp(&a.scores.composite)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });

    let texts: Vec<&str> = scored
        .iter()
        .map(|(record, _)| record.transcript.as_str())
        .collect();
    let (cluster_of, selected_indices) =
        cluster_and_select(&texts, profile.diversity_similarity_threshold, count);

    let mut variants = Vec::with_capacity(selected_indices.len());
    for index in selected_indices {
        let mut record = scored[index].0.clone();
        record.diversity_cluster = format!("cluster-{:03}", cluster_of[index]);
        variants.push(record);
    }

    ShortsBuildResult {
        variants,
        rejected,
        candidates_considered,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use video_core::{VadRegion, Word};

    fn candidate(id: &str, source_id: &str, start_ms: i64, end_ms: i64, text: &str) -> Candidate {
        Candidate {
            id: id.into(),
            source_id: source_id.into(),
            start_ms,
            end_ms,
            text: text.into(),
            beat_label: "beat-001".into(),
            take_rank: 1,
            drop_reason: None,
            filler_count: 0,
        }
    }

    fn word(id: &str, start_ms: i64, end_ms: i64, text: &str, confidence: f32) -> Word {
        Word {
            id: id.into(),
            source_word_id: None,
            text: text.into(),
            start_ms,
            end_ms,
            confidence,
            speaker: None,
            kind: "word".into(),
        }
    }

    #[test]
    fn standalone_rejects_dangling_reference_opener() {
        let profile = ShortsScoringProfile::default();
        let check = validate_standalone("and that's why it matters so much to everyone.", &profile);
        assert!(!check.passed);
        assert_eq!(check.confidence, Confidence::High);
    }

    #[test]
    fn standalone_accepts_a_clean_opener() {
        let profile = ShortsScoringProfile::default();
        let check = validate_standalone("Here is the one mistake everyone makes.", &profile);
        assert!(check.passed);
    }

    #[test]
    fn duration_fit_respects_platform_window() {
        let platform = ShortsScoringProfile::default().platform;
        let inside = score_duration_fit(20_000, &platform);
        assert_eq!(inside.value, 1.0);
        let too_short = score_duration_fit(1_000, &platform);
        assert_eq!(too_short.value, 0.0);
        let too_long = score_duration_fit(120_000, &platform);
        assert_eq!(too_long.value, 0.0);
    }

    #[test]
    fn diversity_clustering_suppresses_near_duplicates() {
        let texts = vec![
            "the biggest mistake people make with money is spending first and saving last",
            "the biggest mistake people make with cash is spending first then saving last",
            "here is a completely different topic about growing tomatoes in your backyard",
            "another totally unrelated point about learning to play the guitar quickly",
        ];
        // Ask for only 3 of the 4 texts: with 3 distinct clusters (the two
        // money sentences share one), round-robin selection must fill all 3
        // slots with one representative per cluster before ever taking a
        // second member from the money cluster.
        let (_clusters, selected) = cluster_and_select(&texts, 0.6, 3);
        let money_indices: std::collections::HashSet<usize> = [0usize, 1].into_iter().collect();
        let selected_money = selected
            .iter()
            .filter(|index| money_indices.contains(index))
            .count();
        assert_eq!(
            selected_money, 1,
            "near-duplicate candidates must not both occupy top slots: {selected:?}"
        );
        assert_eq!(selected.len(), 3, "requested count is 3: {selected:?}");
    }

    #[test]
    fn scoring_pipeline_is_deterministic() {
        let candidates = vec![
            candidate(
                "candidate-001",
                "cam-a",
                0,
                3_000,
                "Here is the one mistake everyone makes.",
            ),
            candidate(
                "candidate-002",
                "cam-a",
                3_200,
                8_000,
                "It costs people thousands of dollars every year.",
            ),
        ];
        let transcripts = vec![Transcript {
            schema_version: 1,
            provider: "fixture".into(),
            source_id: "cam-a".into(),
            language: "en".into(),
            words: vec![
                word("w1", 0, 400, "Here", 0.95),
                word("w2", 400, 800, "is", 0.95),
                word("w3", 800, 1_400, "the", 0.9),
                word("w4", 1_400, 2_200, "one", 0.9),
                word("w5", 2_200, 3_000, "mistake", 0.9),
                word("w6", 3_200, 4_000, "everyone", 0.9),
                word("w7", 4_000, 5_000, "makes", 0.9),
                word("w8", 5_000, 8_000, "here", 0.9),
            ],
            events: Vec::new(),
        }];
        let profile = ShortsScoringProfile::default();
        let vad = BTreeMap::new();
        let first = build_shorts(&candidates, &transcripts, &vad, &[], &profile, 4);
        let second = build_shorts(&candidates, &transcripts, &vad, &[], &profile, 4);
        assert_eq!(first, second);
    }

    #[test]
    fn truthfulness_note_flags_a_gap_between_merged_candidates() {
        let all_candidates = vec![
            candidate(
                "candidate-001",
                "cam-a",
                0,
                3_000,
                "Here is the one mistake everyone makes.",
            ),
            candidate(
                "candidate-002",
                "cam-a",
                3_200,
                8_000,
                "It costs people thousands of dollars.",
            ),
        ];
        let unit = SemanticUnit {
            unit_id: "unit-001".into(),
            source_id: "cam-a".into(),
            members: all_candidates.clone(),
        };
        let note = build_truthfulness_note(&unit, &all_candidates);
        assert!(note.reorders_or_omits_material);
        assert!(note.note.contains("200ms gap"));
    }

    #[test]
    fn truthfulness_note_clean_for_a_single_contiguous_candidate() {
        let all_candidates = vec![candidate(
            "candidate-001",
            "cam-a",
            0,
            3_000,
            "Here is the one mistake everyone makes.",
        )];
        let unit = SemanticUnit {
            unit_id: "unit-001".into(),
            source_id: "cam-a".into(),
            members: all_candidates.clone(),
        };
        let note = build_truthfulness_note(&unit, &all_candidates);
        assert!(!note.reorders_or_omits_material);
    }

    #[test]
    fn segmentation_breaks_on_a_vad_confirmed_silence_even_with_a_small_word_gap() {
        let candidates = vec![
            candidate(
                "candidate-001",
                "cam-a",
                0,
                3_000,
                "First clean thought here.",
            ),
            // Word-level gap is only 100ms (well under max_group_gap_ms),
            // but the transcript's word timestamps overshoot/undershoot the
            // real speech boundaries: VAD shows speech actually stopped at
            // 2000 and did not resume until 4000 — a real 2000ms pause the
            // word timing alone would miss.
            candidate(
                "candidate-002",
                "cam-a",
                3_100,
                6_000,
                "Second unrelated thought here.",
            ),
        ];
        let mut vad = BTreeMap::new();
        vad.insert(
            "cam-a".to_string(),
            VadSignal {
                schema_version: 1,
                source_id: "cam-a".into(),
                sample_rate: 16_000,
                provider: "fixture".into(),
                regions: vec![
                    VadRegion {
                        start_ms: 0,
                        end_ms: 2_000,
                        mean_probability: 0.9,
                    },
                    VadRegion {
                        start_ms: 4_000,
                        end_ms: 6_000,
                        mean_probability: 0.9,
                    },
                ],
            },
        );
        let profile = ShortsScoringProfile {
            max_group_gap_ms: 1_000, // word gap alone (100ms) would NOT break
            ..ShortsScoringProfile::default()
        };
        let units = segment_semantic_units(&candidates, &vad, &profile);
        assert_eq!(
            units.len(),
            2,
            "VAD-confirmed silence must force a break even though the word gap alone would not: {:?}",
            units.iter().map(|u| u.members.len()).collect::<Vec<_>>()
        );
    }
}
