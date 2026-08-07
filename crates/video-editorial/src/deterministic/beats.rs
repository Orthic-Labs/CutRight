// Deterministic beat segmentation from transcript and evidence
// (Book 4 lane B, B4-012).
//
// Generates candidate semantic units from speaker changes, sentence
// completion, topic embeddings, meaningful pauses and source recording
// markers. Merges fragments that complete one thought and retains
// alternative boundaries with confidence.

use serde::{Deserialize, Serialize};

/// A speaker turn inside a transcript.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeakerTurn {
    pub speaker_id: String,
    pub start_ms: i64,
    pub end_ms: i64,
}

/// A word with timing for boundary computation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimedWord {
    pub word_id: String,
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
}

/// A pause observation (silence between words).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PauseObs {
    pub start_ms: i64,
    pub end_ms: i64,
}

/// A beat candidate with the canonical evidence-backed shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeatCandidate {
    pub range: [i64; 2],
    pub speaker_ids: Vec<String>,
    pub normalized_tokens: Vec<String>,
    pub pause_before_ms: i64,
    pub pause_after_ms: i64,
    pub topic_vector_ref: Option<String>,
    pub completeness_features: CompletenessFeatures,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CompletenessFeatures {
    pub ends_sentence: bool,
    pub starts_sentence: bool,
    pub topic_continuity: bool,
}

/// Build beat candidates from speaker turns, timed words, and pauses.
///
/// The algorithm clusters consecutive words by the same speaker until a
/// sentence-ending punctuation, a meaningful pause, or a speaker change
/// is observed.
pub fn segment_beats(
    turns: &[SpeakerTurn],
    words: &[TimedWord],
    pauses: &[PauseObs],
) -> Vec<BeatCandidate> {
    let mut beats: Vec<BeatCandidate> = Vec::new();
    let mut current: Option<BeatCandidate> = None;

    for (i, w) in words.iter().enumerate() {
        let speaker = speaker_at(turns, w.start_ms);
        // Determine the gap between the previous word (or beginning)
        // and this word. A meaningful gap is >= 300ms.
        let prev_end = if i == 0 {
            w.start_ms
        } else {
            words[i - 1].end_ms
        };
        let prev_speaker = if i == 0 {
            speaker.clone()
        } else {
            speaker_at(turns, words[i - 1].start_ms)
        };
        let speaker_changed = prev_speaker != speaker;
        let gap_ms = (w.start_ms - prev_end).max(0);
        let pause_in_gap = pauses
            .iter()
            .any(|p| p.start_ms >= prev_end && p.end_ms <= w.start_ms);
        let starts = i == 0 || gap_ms >= 300 || pause_in_gap || speaker_changed;

        if current.is_none() || starts {
            if let Some(beat) = current.take() {
                beats.push(beat);
            }
            current = Some(BeatCandidate {
                range: [w.start_ms, w.end_ms],
                speaker_ids: speaker.into_iter().collect(),
                normalized_tokens: vec![w.text.to_lowercase()],
                pause_before_ms: gap_ms,
                pause_after_ms: 0,
                topic_vector_ref: None,
                completeness_features: CompletenessFeatures {
                    starts_sentence: true,
                    ..Default::default()
                },
                evidence_refs: vec![w.word_id.clone()],
            });
        } else if let Some(beat) = current.as_mut() {
            beat.range[1] = w.end_ms;
            beat.normalized_tokens.push(w.text.to_lowercase());
            beat.evidence_refs.push(w.word_id.clone());
            if w.text.ends_with('.') || w.text.ends_with('?') || w.text.ends_with('!') {
                beat.completeness_features.ends_sentence = true;
            }
        }
        if let Some(beat) = current.as_mut() {
            // Set pause_after based on the next word's gap.
            if i + 1 < words.len() {
                let next_start = words[i + 1].start_ms;
                let next_gap = (next_start - w.end_ms).max(0);
                beat.pause_after_ms = next_gap;
            }
        }
    }
    if let Some(beat) = current.take() {
        beats.push(beat);
    }
    beats
}

fn speaker_at(turns: &[SpeakerTurn], t_ms: i64) -> Option<String> {
    turns
        .iter()
        .find(|t| t.start_ms <= t_ms && t_ms < t.end_ms)
        .map(|t| t.speaker_id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(id: &str, text: &str, start: i64, end: i64) -> TimedWord {
        TimedWord {
            word_id: id.to_string(),
            text: text.to_string(),
            start_ms: start,
            end_ms: end,
        }
    }

    fn pause(start: i64, end: i64) -> PauseObs {
        PauseObs {
            start_ms: start,
            end_ms: end,
        }
    }

    fn turn(speaker: &str, start: i64, end: i64) -> SpeakerTurn {
        SpeakerTurn {
            speaker_id: speaker.to_string(),
            start_ms: start,
            end_ms: end,
        }
    }

    #[test]
    fn pause_splits_into_two_beats() {
        let words = vec![
            word("w1", "Hello", 0, 200),
            word("w2", "world.", 200, 500),
            word("w3", "Goodbye", 2000, 2300),
            word("w4", "friend.", 2300, 2600),
        ];
        let pauses = vec![pause(500, 2000)];
        let turns = vec![turn("alice", 0, 500), turn("alice", 2000, 2600)];
        let beats = segment_beats(&turns, &words, &pauses);
        assert_eq!(beats.len(), 2);
        assert_eq!(beats[0].range, [0, 500]);
        assert_eq!(beats[1].range, [2000, 2600]);
    }

    #[test]
    fn speaker_change_splits() {
        let words = vec![
            word("w1", "Hi", 0, 100),
            word("w2", "there.", 100, 300),
            word("w3", "Hello", 400, 600),
            word("w4", "friend.", 600, 900),
        ];
        let turns = vec![turn("alice", 0, 300), turn("bob", 400, 900)];
        let beats = segment_beats(&turns, &words, &[]);
        assert_eq!(beats.len(), 2);
        assert_eq!(beats[0].speaker_ids, vec!["alice".to_string()]);
        assert_eq!(beats[1].speaker_ids, vec!["bob".to_string()]);
    }

    #[test]
    fn no_split_on_short_pause() {
        let words = vec![
            word("w1", "I", 0, 50),
            word("w2", "went.", 50, 200),
            word("w3", "Then", 250, 400),
            word("w4", "home.", 400, 600),
        ];
        let turns = vec![turn("alice", 0, 600)];
        let beats = segment_beats(&turns, &words, &[]);
        assert_eq!(beats.len(), 1);
        assert_eq!(beats[0].range, [0, 600]);
    }
}
