mod alignment;

use self::alignment::{align_words, aligned_boundary_checks};
use crate::io::{hash_file, read_json, relative_artifact_path, write_json_atomic};
use crate::transcribe_project;
use crate::transcription::{is_heardright_provider, is_whisperx_provider};
use crate::PipelineArtifact;
use crate::ProjectError;
use std::path::Path;
use std::time::Instant;
use video_core::{
    models::{ProviderResponseEnvelope, SCHEMA_VERSION},
    SourceManifest, Transcript,
};
use video_media::{extract_frame, probe, render_boundary_probe};

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

/// Seek-to-first-decoded-frame latency benchmark. Reuses
/// [`video_media::extract_frame`] — the typed FFmpeg boundary that already
/// runs `-ss <t> -i <input> -frames:v 1` — as the decode primitive, timing
/// each call with [`Instant`] rather than shelling out separately. Offsets
/// are spread evenly across the middle 80% of the media's duration so runs
/// exercise seeks scattered through the file rather than clustering near
/// the start.
pub fn bench_playback(
    project_path: &Path,
    runs: usize,
    dry_run: bool,
) -> Result<serde_json::Value, ProjectError> {
    if runs == 0 {
        return Err(ProjectError::InvalidState(
            "playback benchmark runs must be positive".into(),
        ));
    }
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    let source = sources.sources.first().ok_or(ProjectError::NoSources)?;
    let media_path = Path::new(&source.path).to_path_buf();

    let path = project_path.join("analysis/bench/playback/report.json");
    if dry_run {
        return Ok(serde_json::json!({
            "status": "dry-run",
            "path": relative_artifact_path(project_path, &path),
            "runs": runs,
        }));
    }

    let metadata = probe(&media_path)?;
    let duration_ms = metadata.duration_ms.unwrap_or(10_000).max(1_000);
    let span_start = duration_ms / 10;
    let span_end = duration_ms - span_start;
    let step = if runs > 1 {
        (span_end - span_start).max(0) / (runs as i64 - 1).max(1)
    } else {
        0
    };

    let frame_path = project_path.join("analysis/bench/playback/frame.jpg");
    let mut samples = Vec::with_capacity(runs);
    let mut latencies_ms = Vec::with_capacity(runs);
    for i in 0..runs {
        let seek_ms = (span_start + step * i as i64).min(span_end).max(0);
        let started = Instant::now();
        extract_frame(&media_path, seek_ms, &frame_path)?;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1_000.0;
        latencies_ms.push(elapsed_ms);
        samples.push(serde_json::json!({
            "seek_ms": seek_ms,
            "latency_ms": elapsed_ms,
        }));
    }
    let _ = std::fs::remove_file(&frame_path);

    let mut sorted = latencies_ms.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("latency samples are finite"));
    let percentile = |p: f64| -> f64 {
        let rank = ((p * (sorted.len() as f64 - 1.0)).round()) as usize;
        sorted[rank.min(sorted.len() - 1)]
    };
    let min_ms = sorted.first().copied().unwrap_or(0.0);
    let max_ms = sorted.last().copied().unwrap_or(0.0);
    let p50_ms = percentile(0.50);
    let p95_ms = percentile(0.95);

    let report = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "kind": "playback_benchmark",
        "media_path": media_path.display().to_string(),
        "decode_boundary": "video_media::extract_frame",
        "runs": runs,
        "p50_ms": p50_ms,
        "p95_ms": p95_ms,
        "min_ms": min_ms,
        "max_ms": max_ms,
        "samples": samples,
    });
    write_json_atomic(&path, &report)?;

    Ok(serde_json::json!({
        "status": "created",
        "path": relative_artifact_path(project_path, &path),
        "runs": runs,
        "p50_ms": p50_ms,
        "p95_ms": p95_ms,
        "min_ms": min_ms,
        "max_ms": max_ms,
        "media_path": media_path.display().to_string(),
        "decode_boundary": "video_media::extract_frame",
        "samples": samples,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn bench_playback_dry_run_reports_planned_runs_without_touching_media() {
        use video_core::models::SourceEntry;

        let temp = tempfile::tempdir().unwrap();
        crate::init_project(temp.path(), false).unwrap();
        write_json_atomic(
            &temp.path().join("sources/manifest.json"),
            &SourceManifest {
                schema_version: SCHEMA_VERSION,
                sources: vec![SourceEntry {
                    source_id: "source-a".into(),
                    path: "sources/source-a.mov".into(),
                    blake3: "fixture".into(),
                    duration_ms: Some(10_000),
                    width: Some(1920),
                    height: Some(1080),
                    rotation_degrees: Some(0),
                    is_hdr: Some(false),
                    timebase: None,
                }],
            },
        )
        .unwrap();

        let result = bench_playback(temp.path(), 5, true).unwrap();
        assert_eq!(result["status"], "dry-run");
        assert_eq!(result["runs"], 5);
        assert!(!temp
            .path()
            .join("analysis/bench/playback/report.json")
            .exists());
    }
}
