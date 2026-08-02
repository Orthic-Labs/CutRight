use crate::io::*;
use crate::transcribe_project;
use crate::transcription::{is_heardright_provider, is_whisperx_provider};
use crate::PipelineArtifact;
use crate::ProjectError;
use std::path::Path;
use video_core::{
    models::{ProviderResponseEnvelope, SCHEMA_VERSION},
    SourceManifest, Transcript, Word,
};
use video_media::render_boundary_probe;

/// HeardRight is always the transcript authority and WhisperX is always the
/// verifier (REV2 plan §8.1), regardless of which CLI flag (`--primary` or
/// `--verifier`) named which engine — the roles are fixed by product
/// architecture, not by argument order. Returns `(heardright_name,
/// whisperx_name)` using whichever of the two input strings actually named
/// each engine, preserving the caller's exact spelling.
fn benchmark_provider_roles<'a>(
    primary: &'a str,
    verifier: &'a str,
) -> Result<(&'a str, &'a str), ProjectError> {
    if is_heardright_provider(primary) && is_whisperx_provider(verifier) {
        return Ok((primary, verifier));
    }
    if is_heardright_provider(verifier) && is_whisperx_provider(primary) {
        return Ok((verifier, primary));
    }
    Err(ProjectError::InvalidState(
        "transcription benchmark requires exactly one heardright provider and one whisperx \
         provider (as --primary/--verifier, in either order); HeardRight is always the \
         transcript authority and WhisperX is always the verifier"
            .into(),
    ))
}

/// Classifies why a word failed to align to its counterpart, distinguishing
/// framing noise that normalization cannot fully absorb (punctuation-only
/// tokens, contractions, split tokens) from genuine ASR content disagreement
/// (REV2 plan §8.2). `token_key` already strips punctuation before matching,
/// so most punctuation-only differences never reach this classifier; it
/// exists for the residue that does.
fn classify_word_disagreement(text: &str) -> &'static str {
    let trimmed = text.trim();
    let alnum: String = trimmed.chars().filter(|c| c.is_alphanumeric()).collect();
    if alnum.is_empty() {
        "punctuation"
    } else if trimmed.contains('\'') {
        "contraction"
    } else if alnum.chars().count() <= 2 {
        "token_split"
    } else {
        "content"
    }
}

/// Summary statistics over a set of boundary-check `delta_ms` values, used
/// to record start/end delta distributions in the benchmark report (REV2
/// plan §8.2) instead of only pass/fail counts.
fn delta_stats(checks: &[serde_json::Value]) -> serde_json::Value {
    let deltas: Vec<f64> = checks
        .iter()
        .filter_map(|check| check.get("delta_ms").and_then(serde_json::Value::as_i64))
        .map(|delta| delta as f64)
        .collect();
    if deltas.is_empty() {
        return serde_json::json!({"count": 0, "min_ms": null, "max_ms": null, "mean_ms": null});
    }
    let min = deltas.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = deltas.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mean = deltas.iter().sum::<f64>() / deltas.len() as f64;
    serde_json::json!({"count": deltas.len(), "min_ms": min, "max_ms": max, "mean_ms": mean})
}

/// Best-effort protocol/engine identity extracted from a provider's raw
/// response envelope (REV2 plan §8.3). Fields that the provider did not echo
/// back are recorded as `null` rather than guessed.
fn extract_protocol_identity(raw_response: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "schema_name": raw_response.get("schema_name").cloned().unwrap_or(serde_json::Value::Null),
        "schema_version": raw_response.get("schema_version").cloned().unwrap_or(serde_json::Value::Null),
        "protocol_major": raw_response.get("protocol_major").cloned().unwrap_or(serde_json::Value::Null),
        "protocol_minor": raw_response.get("protocol_minor").cloned().unwrap_or(serde_json::Value::Null),
        "engine_version": raw_response.get("engine_version").cloned().unwrap_or(serde_json::Value::Null),
    })
}

/// Binding provenance for one engine's contribution to one clip (REV2 plan
/// §8.3): the normalized transcript hash, the raw response/envelope hashes,
/// and the engine/model/protocol identity. Any input change invalidates the
/// decision because every one of these hashes is recomputed from disk, not
/// cached.
fn engine_binding(
    project_path: &Path,
    source_id: &str,
    provider_label: &str,
    transcript_path: &Path,
) -> Result<serde_json::Value, ProjectError> {
    let envelope_path = project_path.join(format!(
        "analysis/transcripts/{source_id}.{provider_label}.envelope.json"
    ));
    let raw_path = project_path.join(format!(
        "cache/provider-responses/{source_id}.{provider_label}.raw.json"
    ));
    let envelope: ProviderResponseEnvelope = read_json(&envelope_path)?;
    let raw_response: serde_json::Value = read_json(&raw_path)?;
    Ok(serde_json::json!({
        "provider": envelope.provider,
        "provider_model": envelope.provider_model,
        "protocol": extract_protocol_identity(&raw_response),
        "transcript_hash": hash_file(transcript_path)?,
        "raw_response_hash": hash_file(&raw_path)?,
        "envelope_hash": hash_file(&envelope_path)?,
        "request_hash": envelope.request_hash,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BenchmarkDecision {
    status: &'static str,
    transcript_authority: &'static str,
    timestamp_authority: &'static str,
}

/// The HeardRight-primary validation policy (REV2 plan §8.1). This replaces
/// the old symmetric "whichever provider is eligible wins, both eligible is
/// unresolved" election: HeardRight is the transcript authority whenever its
/// own content and coverage are acceptable, even when WhisperX is also
/// clean. The product transcript engine never changes merely because the
/// verifier happened to produce one cleaner sample.
fn benchmark_decision(
    verifier_unavailable: bool,
    verifier_coverage_sufficient: bool,
    heardright_content_clean: bool,
    heardright_edges_clean: bool,
    whisperx_edges_clean: bool,
) -> BenchmarkDecision {
    if verifier_unavailable {
        return BenchmarkDecision {
            status: "verifier_unavailable",
            transcript_authority: "heardright",
            timestamp_authority: "heardright",
        };
    }
    if heardright_content_clean && heardright_edges_clean && verifier_coverage_sufficient {
        return BenchmarkDecision {
            status: "primary_accepted",
            transcript_authority: "heardright",
            timestamp_authority: "heardright",
        };
    }
    if heardright_content_clean
        && verifier_coverage_sufficient
        && !heardright_edges_clean
        && whisperx_edges_clean
    {
        return BenchmarkDecision {
            status: "verifier_edges_required",
            transcript_authority: "heardright",
            timestamp_authority: "whisperx",
        };
    }
    BenchmarkDecision {
        status: "manual_review_required",
        transcript_authority: "heardright",
        timestamp_authority: "unresolved",
    }
}

pub fn bench_transcribe(
    project_path: &Path,
    primary: &str,
    verifier: &str,
    boundaries: usize,
    padding_ms: i64,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    if boundaries == 0 || padding_ms < 0 {
        return Err(ProjectError::InvalidState(
            "benchmark boundaries must be positive and padding must be nonnegative".into(),
        ));
    }
    let (heardright_name, whisperx_name) = benchmark_provider_roles(primary, verifier)?;
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    if sources.sources.len() < 3 {
        return Err(ProjectError::InvalidState(
            "transcription benchmark requires at least three immutable source clips".into(),
        ));
    }
    if dry_run {
        return Ok(PipelineArtifact {
            status: "dry-run",
            path: project_path.join("analysis/bench/transcribe/report.json"),
            count: sources.sources.len(),
        });
    }
    let policy = video_core::BenchmarkPolicy::v1();

    // HeardRight is the transcript authority: it must run cleanly or the
    // benchmark cannot proceed at all. There is no fallback to WhisperX as
    // the product engine (REV2 §8.1).
    transcribe_project(project_path, heardright_name, false)?;

    // WhisperX is a verifier, not a co-equal engine: its failure degrades
    // the decision to `verifier_unavailable` rather than aborting the whole
    // benchmark (REV2 §8.1 case 4 — the HeardRight transcript stays
    // viewable even when verification could not run).
    let verifier_unavailable_reason = match transcribe_project(project_path, whisperx_name, false) {
        Ok(_) => None,
        Err(error) => Some(error.to_string()),
    };

    let mut total_heardright_words = 0_usize;
    let mut total_matched_words = 0_usize;
    let mut total_content_unmatched_words = 0_usize;
    let mut total_heardright_non_clean = 0_usize;
    let mut total_whisperx_non_clean = 0_usize;
    let mut all_heardright_checks: Vec<serde_json::Value> = Vec::new();
    let mut all_whisperx_checks: Vec<serde_json::Value> = Vec::new();
    let mut clips = Vec::new();

    for source in &sources.sources {
        let heardright_path =
            project_path.join(format!("analysis/transcripts/{}.json", source.source_id));
        let whisperx_path = project_path.join(format!(
            "analysis/transcripts/{}.whisperx.json",
            source.source_id
        ));
        let heardright_transcript: Transcript = read_json(&heardright_path)?;
        heardright_transcript.validate().map_err(|error| {
            ProjectError::InvalidState(format!(
                "heardright transcript for {} failed semantic validation: {error}",
                source.source_id
            ))
        })?;
        total_heardright_words += heardright_transcript.words.len();

        let whisperx_transcript: Option<Transcript> = if whisperx_path.is_file() {
            let transcript: Transcript = read_json(&whisperx_path)?;
            transcript.validate().map_err(|error| {
                ProjectError::InvalidState(format!(
                    "whisperx transcript for {} failed semantic validation: {error}",
                    source.source_id
                ))
            })?;
            Some(transcript)
        } else {
            None
        };

        let (
            heardright_checks,
            whisperx_checks,
            unmatched_words,
            whisperx_unmatched_words,
            matched_count,
        ) = match &whisperx_transcript {
            Some(whisperx_transcript) => {
                let alignment =
                    align_words(&heardright_transcript.words, &whisperx_transcript.words);
                let mut heardright_checks = aligned_boundary_checks(
                    &heardright_transcript.words,
                    &whisperx_transcript.words,
                    &alignment.matches,
                    true,
                    boundaries,
                    padding_ms,
                );
                let whisperx_checks = aligned_boundary_checks(
                    &whisperx_transcript.words,
                    &heardright_transcript.words,
                    &alignment.matches,
                    false,
                    boundaries,
                    padding_ms,
                );
                for (index, check) in heardright_checks.iter_mut().enumerate() {
                    let boundary_ms = check
                        .get("boundary_ms")
                        .and_then(serde_json::Value::as_i64)
                        .ok_or_else(|| {
                            ProjectError::InvalidState("benchmark check has no boundary".into())
                        })?;
                    let probe = project_path.join(format!(
                        "analysis/bench/transcribe/probes/{}/{index:03}-{}.mp4",
                        source.source_id,
                        check["side"].as_str().unwrap_or("boundary")
                    ));
                    render_boundary_probe(Path::new(&source.path), boundary_ms, &probe)?;
                    check
                        .as_object_mut()
                        .expect("benchmark check is an object")
                        .insert(
                            "render_probe".into(),
                            serde_json::Value::String(relative_artifact_path(project_path, &probe)),
                        );
                }
                let unmatched_words: Vec<serde_json::Value> = alignment
                    .unmatched_primary
                    .iter()
                    .map(|&index| {
                        let word = &heardright_transcript.words[index];
                        serde_json::json!({
                            "word_id": word.id,
                            "text": word.text,
                            "class": classify_word_disagreement(&word.text)
                        })
                    })
                    .collect();
                let whisperx_unmatched_words: Vec<serde_json::Value> = alignment
                    .unmatched_verifier
                    .iter()
                    .map(|&index| {
                        let word = &whisperx_transcript.words[index];
                        serde_json::json!({
                            "word_id": word.id,
                            "text": word.text,
                            "class": classify_word_disagreement(&word.text)
                        })
                    })
                    .collect();
                (
                    heardright_checks,
                    whisperx_checks,
                    unmatched_words,
                    whisperx_unmatched_words,
                    alignment.matches.len(),
                )
            }
            None => (Vec::new(), Vec::new(), Vec::new(), Vec::new(), 0),
        };

        let heardright_non_clean = heardright_checks
            .iter()
            .filter(|check| check["status"] != "clean")
            .count();
        let whisperx_non_clean = whisperx_checks
            .iter()
            .filter(|check| check["status"] != "clean")
            .count();
        let content_unmatched = unmatched_words
            .iter()
            .filter(|word| word["class"] == "content")
            .count();

        total_matched_words += matched_count;
        total_content_unmatched_words += content_unmatched;
        total_heardright_non_clean += heardright_non_clean;
        total_whisperx_non_clean += whisperx_non_clean;
        all_heardright_checks.extend(heardright_checks.iter().cloned());
        all_whisperx_checks.extend(whisperx_checks.iter().cloned());

        let heardright_binding = engine_binding(
            project_path,
            &source.source_id,
            "heardright",
            &heardright_path,
        )?;
        let whisperx_binding = if whisperx_transcript.is_some() {
            Some(engine_binding(
                project_path,
                &source.source_id,
                "whisperx",
                &whisperx_path,
            )?)
        } else {
            None
        };

        clips.push(serde_json::json!({
            "source_id": source.source_id,
            "source_path": source.path,
            "source_blake3": source.blake3,
            "heardright_transcript": heardright_path.strip_prefix(project_path).unwrap_or(&heardright_path),
            "whisperx_transcript": if whisperx_transcript.is_some() {
                serde_json::Value::String(
                    relative_artifact_path(project_path, &whisperx_path),
                )
            } else {
                serde_json::Value::Null
            },
            "heardright_checks": heardright_checks,
            "whisperx_checks": whisperx_checks,
            "unmatched_words": unmatched_words,
            "whisperx_unmatched_words": whisperx_unmatched_words,
            "counts": {
                "heardright_non_clean": heardright_non_clean,
                "whisperx_non_clean": whisperx_non_clean,
                "matched_words": matched_count,
                "unmatched_words": unmatched_words.len(),
                "content_unmatched_words": content_unmatched,
                "whisperx_unmatched_words": whisperx_unmatched_words.len()
            },
            "binding": {
                "heardright": heardright_binding,
                "whisperx": whisperx_binding
            }
        }));
    }

    let alignment_coverage = if total_heardright_words == 0 {
        0.0
    } else {
        total_matched_words as f64 / total_heardright_words as f64
    };
    let unmatched_content_rate = if total_heardright_words == 0 {
        0.0
    } else {
        total_content_unmatched_words as f64 / total_heardright_words as f64
    };
    let verifier_unavailable = verifier_unavailable_reason.is_some();
    let verifier_coverage_sufficient = alignment_coverage >= policy.min_alignment_coverage;
    let heardright_content_clean = unmatched_content_rate <= policy.max_unmatched_content_rate;
    let heardright_edges_clean = total_heardright_non_clean == 0;
    let whisperx_edges_clean = total_whisperx_non_clean == 0;

    let decision = benchmark_decision(
        verifier_unavailable,
        verifier_coverage_sufficient,
        heardright_content_clean,
        heardright_edges_clean,
        whisperx_edges_clean,
    );

    let path = project_path.join("analysis/bench/transcribe/report.json");
    write_json_atomic(
        &path,
        &serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "kind": "transcription_benchmark",
            "policy_version": policy.policy_version,
            "heardright_provider": heardright_name,
            "whisperx_provider": whisperx_name,
            "boundaries_requested": boundaries,
            "padding_ms": padding_ms,
            "verifier_unavailable_reason": verifier_unavailable_reason,
            "clips": clips,
            "summary": {
                "heardright_words": total_heardright_words,
                "matched_words": total_matched_words,
                "content_unmatched_words": total_content_unmatched_words,
                "alignment_coverage": alignment_coverage,
                "unmatched_content_rate": unmatched_content_rate,
                "heardright_non_clean": total_heardright_non_clean,
                "whisperx_non_clean": total_whisperx_non_clean,
                "heardright_start_end_delta_ms": delta_stats(&all_heardright_checks),
                "whisperx_start_end_delta_ms": delta_stats(&all_whisperx_checks)
            },
            "decision": {
                "transcript_authority": decision.transcript_authority,
                "timestamp_authority": decision.timestamp_authority,
                "verifier": "whisperx",
                "status": decision.status
            }
        }),
    )?;

    if decision.status == "manual_review_required" {
        return Err(ProjectError::InvalidState(format!(
            "transcription benchmark requires manual review; destructive word-edge cuts are \
             blocked until a human resolves it; inspect {}",
            path.display()
        )));
    }
    Ok(PipelineArtifact {
        status: match decision.status {
            "verifier_unavailable" => "verifier_unavailable",
            _ => "created",
        },
        path,
        count: sources.sources.len(),
    })
}

#[derive(Debug)]
struct Alignment {
    matches: Vec<(usize, usize)>,
    unmatched_primary: Vec<usize>,
    unmatched_verifier: Vec<usize>,
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

fn align_words(primary: &[Word], verifier: &[Word]) -> Alignment {
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

fn aligned_boundary_checks(
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

    #[test]
    fn benchmark_decision_accepts_heardright_even_when_whisperx_is_also_clean() {
        // REV2 §8.1 case 1: this is the exact bug the old symmetric election
        // had — both providers eligible used to mean "unresolved". Now
        // HeardRight wins outright, because switching engines because the
        // verifier also looked clean is exactly what §8.1 forbids.
        let decision = benchmark_decision(false, true, true, true, true);
        assert_eq!(decision.status, "primary_accepted");
        assert_eq!(decision.transcript_authority, "heardright");
        assert_eq!(decision.timestamp_authority, "heardright");
    }

    #[test]
    fn benchmark_decision_accepts_heardright_when_whisperx_is_unclean() {
        let decision = benchmark_decision(false, true, true, true, false);
        assert_eq!(decision.status, "primary_accepted");
    }

    #[test]
    fn benchmark_decision_requires_verifier_edges_when_heardright_edges_fail() {
        // REV2 §8.1 case 2: keep HeardRight's text, borrow WhisperX's clean
        // edge timings.
        let decision = benchmark_decision(false, true, true, false, true);
        assert_eq!(decision.status, "verifier_edges_required");
        assert_eq!(decision.transcript_authority, "heardright");
        assert_eq!(decision.timestamp_authority, "whisperx");
    }

    #[test]
    fn benchmark_decision_requires_manual_review_on_material_disagreement() {
        // REV2 §8.1 case 3: content is unclean and WhisperX's own edges are
        // also unclean, so nobody's timestamps can be trusted automatically.
        let unclean_content = benchmark_decision(false, true, false, false, false);
        assert_eq!(unclean_content.status, "manual_review_required");
        assert_eq!(unclean_content.timestamp_authority, "unresolved");

        let insufficient_coverage = benchmark_decision(false, false, true, true, true);
        assert_eq!(insufficient_coverage.status, "manual_review_required");
    }

    #[test]
    fn benchmark_decision_marks_verifier_unavailable_without_blocking_the_transcript() {
        // REV2 §8.1 case 4: HeardRight stays the transcript authority and
        // its own timestamps stay usable for viewing, but the status flags
        // that destructive automation is unverified.
        let decision = benchmark_decision(true, false, false, false, false);
        assert_eq!(decision.status, "verifier_unavailable");
        assert_eq!(decision.transcript_authority, "heardright");
        assert_eq!(decision.timestamp_authority, "heardright");
    }

    #[test]
    fn benchmark_provider_roles_resolves_heardright_and_whisperx_in_either_argument_order() {
        assert_eq!(
            benchmark_provider_roles("heardright", "whisperx").unwrap(),
            ("heardright", "whisperx")
        );
        assert_eq!(
            benchmark_provider_roles("whisperx-alignment", "heardright-parakeet-tdt").unwrap(),
            ("heardright-parakeet-tdt", "whisperx-alignment")
        );
    }

    #[test]
    fn benchmark_provider_roles_rejects_two_of_the_same_engine() {
        assert!(benchmark_provider_roles("heardright", "heardright-parakeet-tdt").is_err());
        assert!(benchmark_provider_roles("whisperx", "whisperx-alignment").is_err());
        assert!(benchmark_provider_roles("heardright", "unknown-provider").is_err());
    }

    #[test]
    fn classify_word_disagreement_distinguishes_trivial_from_content_disagreement() {
        assert_eq!(classify_word_disagreement("--"), "punctuation");
        assert_eq!(classify_word_disagreement("don't"), "contraction");
        assert_eq!(classify_word_disagreement("re"), "token_split");
        assert_eq!(classify_word_disagreement("mountain"), "content");
    }
}
