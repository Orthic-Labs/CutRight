use video_core::Word;

#[derive(Debug)]
pub(crate) struct Alignment {
    pub(crate) matches: Vec<(usize, usize)>,
    pub(crate) unmatched_primary: Vec<usize>,
    pub(crate) unmatched_verifier: Vec<usize>,
}

fn token_key(text: &str) -> String {
    let compact: String = text
        .trim()
        .to_lowercase()
        .chars()
        .filter(|character| character.is_alphanumeric())
        .collect();
    if compact.is_empty() {
        text.trim().to_lowercase()
    } else {
        compact
    }
}

pub(crate) fn align_words(primary: &[Word], verifier: &[Word]) -> Alignment {
    let mut matrix = vec![vec![0_usize; verifier.len() + 1]; primary.len() + 1];
    for primary_index in (0..primary.len()).rev() {
        for verifier_index in (0..verifier.len()).rev() {
            matrix[primary_index][verifier_index] = if token_key(&primary[primary_index].text)
                == token_key(&verifier[verifier_index].text)
            {
                matrix[primary_index + 1][verifier_index + 1] + 1
            } else {
                matrix[primary_index + 1][verifier_index]
                    .max(matrix[primary_index][verifier_index + 1])
            };
        }
    }
    let mut matches = Vec::new();
    let mut unmatched_primary = Vec::new();
    let mut unmatched_verifier = Vec::new();
    let (mut primary_index, mut verifier_index) = (0, 0);
    while primary_index < primary.len() && verifier_index < verifier.len() {
        if token_key(&primary[primary_index].text) == token_key(&verifier[verifier_index].text) {
            matches.push((primary_index, verifier_index));
            primary_index += 1;
            verifier_index += 1;
        } else if matrix[primary_index + 1][verifier_index]
            >= matrix[primary_index][verifier_index + 1]
        {
            unmatched_primary.push(primary_index);
            primary_index += 1;
        } else {
            unmatched_verifier.push(verifier_index);
            verifier_index += 1;
        }
    }
    unmatched_primary.extend(primary_index..primary.len());
    unmatched_verifier.extend(verifier_index..verifier.len());
    Alignment {
        matches,
        unmatched_primary,
        unmatched_verifier,
    }
}

pub(crate) fn aligned_boundary_checks(
    candidate: &[Word],
    reference: &[Word],
    matches: &[(usize, usize)],
    candidate_is_primary: bool,
    limit: usize,
    padding_ms: i64,
) -> Vec<serde_json::Value> {
    let boundaries = matches
        .iter()
        .flat_map(|(primary, verifier)| {
            let (candidate_index, reference_index) = if candidate_is_primary {
                (*primary, *verifier)
            } else {
                (*verifier, *primary)
            };
            [
                (candidate_index, reference_index, "start"),
                (candidate_index, reference_index, "end"),
            ]
        })
        .collect::<Vec<_>>();
    evenly_spaced(&boundaries, limit)
        .into_iter()
        .map(|&(candidate_index, reference_index, side)| {
            let candidate_word = &candidate[candidate_index];
            let reference_word = &reference[reference_index];
            let boundary_ms = if side == "start" {
                candidate_word.start_ms
            } else {
                candidate_word.end_ms
            };
            let expected = if side == "start" {
                reference_word.start_ms
            } else {
                reference_word.end_ms
            };
            let status = if (boundary_ms - expected).abs() <= padding_ms {
                "clean"
            } else if boundary_ms > reference_word.start_ms + padding_ms
                && boundary_ms < reference_word.end_ms - padding_ms
            {
                "clipped_word"
            } else if side == "start" && boundary_ms < reference_word.start_ms {
                "early_start"
            } else if side == "start" {
                "late_start"
            } else if boundary_ms < reference_word.start_ms {
                // An end boundary landing before the reference word even starts
                // is an EARLY cut, not a late one. Without this branch every
                // non-clean end boundary reported `late_end`, which points a
                // reviewer at the opposite edit.
                "early_end"
            } else {
                "late_end"
            };
            serde_json::json!({
                "side": side,
                "boundary_ms": boundary_ms,
                "word_id": candidate_word.id,
                "word_text": candidate_word.text,
                "status": status,
                "matched_word_id": reference_word.id,
                "matched_word_start_ms": reference_word.start_ms,
                "matched_word_end_ms": reference_word.end_ms,
                "delta_ms": boundary_ms - expected
            })
        })
        .collect()
}

fn evenly_spaced<T>(items: &[T], limit: usize) -> Vec<&T> {
    if limit == 0 || items.is_empty() {
        return Vec::new();
    }
    if items.len() <= limit {
        return items.iter().collect();
    }
    if limit == 1 {
        return vec![&items[items.len() / 2]];
    }
    (0..limit)
        .map(|index| &items[index * (items.len() - 1) / (limit - 1)])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_marks_only_inter_word_boundaries_as_clean() {
        let candidate = vec![Word {
            id: "candidate".into(),
            source_word_id: None,
            text: "hello".into(),
            start_ms: 1_000,
            end_ms: 1_500,
            confidence: 1.0,
            speaker: None,
            kind: "word".into(),
        }];
        let reference = vec![Word {
            id: "reference".into(),
            source_word_id: None,
            text: "hello".into(),
            start_ms: 1_030,
            end_ms: 1_470,
            confidence: 0.0,
            speaker: None,
            kind: "word".into(),
        }];
        let alignment = align_words(&candidate, &reference);
        let checks =
            aligned_boundary_checks(&candidate, &reference, &alignment.matches, true, 2, 40);
        assert_eq!(checks[0]["status"], "clean");
        assert_eq!(checks[1]["status"], "clean");
        let clipped_candidate = vec![Word {
            start_ms: 1_200,
            ..candidate[0].clone()
        }];
        let clipped_alignment = align_words(&clipped_candidate, &reference);
        let clipped = aligned_boundary_checks(
            &clipped_candidate,
            &reference,
            &clipped_alignment.matches,
            true,
            2,
            40,
        );
        assert_eq!(clipped[0]["status"], "clipped_word");
        let repeated = vec![
            Word {
                text: "go,".into(),
                ..candidate[0].clone()
            },
            Word {
                text: "go".into(),
                ..candidate[0].clone()
            },
            Word {
                text: "home".into(),
                ..candidate[0].clone()
            },
        ];
        let repeated_verifier = vec![
            Word {
                text: "go".into(),
                ..reference[0].clone()
            },
            Word {
                text: "go".into(),
                ..reference[0].clone()
            },
            Word {
                text: "home.".into(),
                ..reference[0].clone()
            },
        ];
        let repeated_alignment = align_words(&repeated, &repeated_verifier);
        assert_eq!(repeated_alignment.matches.len(), 3);
        assert!(repeated_alignment.unmatched_primary.is_empty());
        assert!(repeated_alignment.unmatched_verifier.is_empty());
    }

    #[test]
    fn benchmark_sampling_spans_the_full_clip() {
        let boundaries = (0..100).collect::<Vec<_>>();
        let sampled = evenly_spaced(&boundaries, 5)
            .into_iter()
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(sampled, vec![0, 24, 49, 74, 99]);
    }
}
