use crate::io::*;
use crate::receipts;
use crate::PipelineArtifact;
use crate::ProjectError;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use video_core::{
    models::SCHEMA_VERSION, Candidate, CandidateManifest, DropReason, FillerPolicy, Word,
};

pub fn build_candidates(
    project_path: &Path,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    build_candidates_with_policy(project_path, FillerPolicy::default(), dry_run)
}

/// Single-token filler words / disfluencies.
const FILLER_WORDS: &[&str] = &[
    "um",
    "uh",
    "like",
    "so",
    "right",
    "basically",
    "actually",
    "literally",
];

fn normalize_token(text: &str) -> String {
    text.trim()
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

/// Whether a single word is a filler disfluency.
pub fn is_filler_word(text: &str) -> bool {
    FILLER_WORDS.contains(&normalize_token(text).as_str())
}

/// Count filler words in a span, including the two-word "you know" phrase. Each
/// word is counted at most once.
pub fn count_fillers(words: &[Word]) -> usize {
    let tokens: Vec<String> = words
        .iter()
        .map(|word| normalize_token(&word.text))
        .collect();
    let mut flagged = vec![false; tokens.len()];
    for i in 0..tokens.len().saturating_sub(1) {
        if tokens[i] == "you" && tokens[i + 1] == "know" {
            flagged[i] = true;
            flagged[i + 1] = true;
        }
    }
    for (i, token) in tokens.iter().enumerate() {
        if FILLER_WORDS.contains(&token.as_str()) {
            flagged[i] = true;
        }
    }
    flagged.iter().filter(|flagged| **flagged).count()
}

/// Detect a repeated false start: an immediately repeated token ("go go") or an
/// immediately repeated bigram ("I want to I want to").
pub fn has_false_start(words: &[Word]) -> bool {
    let tokens: Vec<String> = words
        .iter()
        .map(|word| normalize_token(&word.text))
        .filter(|token| !token.is_empty())
        .collect();
    for pair in tokens.windows(2) {
        if pair[0] == pair[1] {
            return true;
        }
    }
    for i in 0..tokens.len().saturating_sub(3) {
        if tokens[i] == tokens[i + 2] && tokens[i + 1] == tokens[i + 3] {
            return true;
        }
    }
    false
}

/// Deterministic take quality used to pick the best take per beat. Ordered so
/// that the maximum is the best take: a complete sentence wins, then fewer
/// fillers, then higher mean confidence, then more words.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TakeScore {
    complete: bool,
    filler_penalty: i64,
    confidence_millis: i64,
    word_count: usize,
}

fn take_score(words: &[Word]) -> TakeScore {
    let filler_count = count_fillers(words);
    let confidence_sum: f64 = words.iter().map(|word| word.confidence as f64).sum();
    let confidence_millis = if words.is_empty() {
        0
    } else {
        (confidence_sum / words.len() as f64 * 1000.0).round() as i64
    };
    let complete = words
        .last()
        .is_some_and(|word| word.text.trim_end().ends_with(['.', '?', '!']));
    TakeScore {
        complete,
        filler_penalty: -(filler_count as i64),
        confidence_millis,
        word_count: words.len(),
    }
}

struct RawCandidate {
    candidate: Candidate,
    words: Vec<Word>,
}

pub fn build_candidates_with_policy(
    project_path: &Path,
    policy: FillerPolicy,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let transcripts = load_transcripts(project_path)?;
    let mut raws: Vec<RawCandidate> = Vec::new();
    for transcript in &transcripts {
        for (index, words) in group_words(&transcript.words, 900).into_iter().enumerate() {
            let first = words
                .first()
                .ok_or_else(|| ProjectError::InvalidState("candidate group was empty".into()))?;
            let last = words.last().expect("nonempty candidate group");
            // Ordinal position is the beat identity: the Nth group of each
            // source covers the same beat, so multiple sources become multiple
            // takes of that beat.
            let beat_label = if index == 0 {
                "hook".to_string()
            } else {
                format!("beat-{index:03}")
            };
            let filler_count = match policy {
                FillerPolicy::Preserve => 0,
                FillerPolicy::SuggestOnly | FillerPolicy::Automatic => count_fillers(&words),
            };
            raws.push(RawCandidate {
                candidate: Candidate {
                    id: format!("candidate-{:03}", raws.len() + 1),
                    source_id: transcript.source_id.clone(),
                    start_ms: first.start_ms,
                    end_ms: last.end_ms,
                    text: words
                        .iter()
                        .map(|word| word.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                    beat_label,
                    take_rank: (index + 1) as u32,
                    drop_reason: None,
                    filler_count,
                },
                words,
            });
        }
    }

    // Best-take-per-beat: when several takes cover the same beat, keep the
    // highest-scoring one and mark the rest as duplicates.
    let scores: Vec<TakeScore> = raws.iter().map(|raw| take_score(&raw.words)).collect();
    let mut beats: Vec<(String, Vec<usize>)> = Vec::new();
    for (index, raw) in raws.iter().enumerate() {
        let position = beats
            .iter()
            .position(|(label, _)| *label == raw.candidate.beat_label);
        match position {
            Some(position) => beats[position].1.push(index),
            None => beats.push((raw.candidate.beat_label.clone(), vec![index])),
        }
    }
    for (_, members) in &beats {
        if members.len() < 2 {
            continue;
        }
        let mut ranked = members.clone();
        ranked.sort_by(|&a, &b| {
            scores[b]
                .cmp(&scores[a])
                .then_with(|| {
                    raws[a]
                        .candidate
                        .source_id
                        .cmp(&raws[b].candidate.source_id)
                })
                .then_with(|| raws[a].candidate.start_ms.cmp(&raws[b].candidate.start_ms))
        });
        for &loser in &ranked[1..] {
            raws[loser].candidate.drop_reason = Some(DropReason::Duplicate);
        }
    }

    // Filler / false-start policy. SuggestOnly (default) records but drops
    // nothing; Automatic drops pure-filler and repeated false-start candidates.
    if policy == FillerPolicy::Automatic {
        for raw in raws.iter_mut() {
            if raw.candidate.drop_reason.is_some() {
                continue;
            }
            if has_false_start(&raw.words) {
                raw.candidate.drop_reason = Some(DropReason::FalseStart);
            } else if !raw.words.is_empty() && raw.candidate.filler_count >= raw.words.len() {
                raw.candidate.drop_reason = Some(DropReason::Filler);
            }
        }
    }

    let candidates: Vec<Candidate> = raws.into_iter().map(|raw| raw.candidate).collect();
    let path = project_path.join("edit/candidates.json");
    if !dry_run {
        write_json_atomic(
            &path,
            &CandidateManifest {
                schema_version: SCHEMA_VERSION,
                candidates: candidates.clone(),
            },
        )?;
        let transcript_paths = transcript_file_paths(project_path)?;
        let inputs: Vec<&Path> = transcript_paths.iter().map(PathBuf::as_path).collect();
        receipts::write_stage_receipt(
            &receipts::receipt_path_for(&path),
            "edit.candidates",
            &inputs,
            &serde_json::json!({ "policy": policy }),
            BTreeMap::new(),
            &[path.as_path()],
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: candidates.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_project;
    use video_core::Transcript;

    #[test]
    fn filler_detection_flags_disfluencies_and_false_starts() {
        assert!(is_filler_word("um"));
        assert!(is_filler_word("Uh,"));
        assert!(is_filler_word("literally"));
        assert!(!is_filler_word("hello"));
        assert!(!is_filler_word("you"));

        let w = |text: &str, start: i64| Word {
            id: format!("w_{start}"),
            source_word_id: None,
            text: text.into(),
            start_ms: start,
            end_ms: start + 100,
            confidence: 1.0,
            speaker: None,
            kind: "word".into(),
        };
        let fillers = vec![w("um", 0), w("you", 100), w("know", 200), w("hello", 300)];
        assert_eq!(count_fillers(&fillers), 3);
        assert_eq!(count_fillers(&[w("hello", 0), w("world", 100)]), 0);

        assert!(has_false_start(&[w("go", 0), w("go", 100), w("home", 200)]));
        assert!(has_false_start(&[
            w("I", 0),
            w("want", 100),
            w("I", 200),
            w("want", 300),
            w("it", 400)
        ]));
        assert!(!has_false_start(&[w("hello", 0), w("world", 100)]));
    }

    #[test]
    fn build_candidates_selects_one_take_per_beat_deterministically() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        let transcript = |source_id: &str, confidence: f32| Transcript {
            schema_version: SCHEMA_VERSION,
            provider: "fixture".into(),
            source_id: source_id.into(),
            language: "en".into(),
            words: vec![Word {
                id: "w_000000".into(),
                source_word_id: None,
                text: "hello.".into(),
                start_ms: 0,
                end_ms: 500,
                confidence,
                speaker: None,
                kind: "word".into(),
            }],
            events: Vec::new(),
        };
        // source-a is the stronger take (higher confidence) than source-b.
        write_json_atomic(
            &temp.path().join("analysis/transcripts/source-a.json"),
            &transcript("source-a", 0.9),
        )
        .unwrap();
        write_json_atomic(
            &temp.path().join("analysis/transcripts/source-b.json"),
            &transcript("source-b", 0.5),
        )
        .unwrap();

        build_candidates(temp.path(), false).unwrap();
        let first: CandidateManifest =
            read_json(&temp.path().join("edit/candidates.json")).unwrap();
        assert_eq!(first.candidates.len(), 2);
        let winners = first
            .candidates
            .iter()
            .filter(|candidate| candidate.drop_reason.is_none())
            .collect::<Vec<_>>();
        assert_eq!(winners.len(), 1);
        assert_eq!(winners[0].source_id, "source-a");
        let loser = first
            .candidates
            .iter()
            .find(|candidate| candidate.source_id == "source-b")
            .unwrap();
        assert_eq!(loser.drop_reason, Some(DropReason::Duplicate));

        // Deterministic: a second pass produces the identical manifest.
        build_candidates(temp.path(), false).unwrap();
        let second: CandidateManifest =
            read_json(&temp.path().join("edit/candidates.json")).unwrap();
        assert_eq!(first, second);
    }
}
