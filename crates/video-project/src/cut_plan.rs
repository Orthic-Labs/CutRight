use crate::io::*;
use crate::receipts;
use crate::PipelineArtifact;
use crate::ProjectError;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use video_core::{
    models::SCHEMA_VERSION, CandidateManifest, CutPlan, CutSegment, SourceManifest, VadSignal, Word,
};

pub fn build_cut_plan(
    project_path: &Path,
    variant: &str,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let (gap_threshold_ms, head_margin_ms, tail_margin_ms) = match variant {
        "tight" => (220, 90, 130),
        "natural" => (400, 140, 220),
        other => {
            return Err(ProjectError::InvalidState(format!(
                "unknown edit variant {other}; use tight or natural"
            )))
        }
    };
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    let candidates: CandidateManifest = read_json(&project_path.join("edit/candidates.json"))?;
    if candidates.candidates.is_empty() {
        return Err(ProjectError::InvalidState(
            "candidate pass must produce at least one candidate before rendering".into(),
        ));
    }
    let vad_by_source = sources
        .sources
        .iter()
        .map(|source| {
            let path = project_path.join(format!("analysis/vad-{}.json", source.source_id));
            let signal: VadSignal = read_json(&path).map_err(|_| {
                ProjectError::InvalidState(format!(
                    "cut planning requires VAD for {}; run `videoctl analyze local <project>` first",
                    source.source_id
                ))
            })?;
            Ok((source.source_id.clone(), signal))
        })
        .collect::<Result<std::collections::HashMap<_, _>, ProjectError>>()?;
    let transcripts = load_transcripts(project_path)?;
    let mut segments = Vec::new();
    for candidate in candidates
        .candidates
        .iter()
        .filter(|candidate| candidate.drop_reason.is_none())
    {
        let source_duration = sources
            .sources
            .iter()
            .find(|source| source.source_id == candidate.source_id)
            .and_then(|source| source.duration_ms);
        let mut candidate_words: Vec<&Word> = transcripts
            .iter()
            .filter(|transcript| transcript.source_id == candidate.source_id)
            .flat_map(|transcript| transcript.words.iter())
            .filter(|word| word.end_ms > candidate.start_ms && word.start_ms < candidate.end_ms)
            .collect();
        candidate_words.sort_by_key(|word| word.start_ms);
        let chunks = candidate_chunks(
            &candidate_words,
            gap_threshold_ms,
            head_margin_ms,
            tail_margin_ms,
            source_duration,
        );
        if chunks.is_empty() {
            // No word evidence for this candidate: fall back to VAD-expanded bounds.
            let vad = vad_by_source.get(&candidate.source_id).ok_or_else(|| {
                ProjectError::InvalidState(format!("missing VAD for {}", candidate.source_id))
            })?;
            let (speech_start, speech_end) =
                vad_adjusted_bounds(candidate.start_ms, candidate.end_ms, vad);
            let start = speech_start.saturating_sub(head_margin_ms).max(0);
            let mut end = speech_end.saturating_add(tail_margin_ms);
            if let Some(duration) = source_duration {
                end = end.min(duration);
            }
            if end - start < 600 {
                end = start.saturating_add(600);
                if let Some(duration) = source_duration {
                    end = end.min(duration);
                }
            }
            if end > start {
                segments.push(CutSegment {
                    id: format!("segment-{:03}", segments.len() + 1),
                    source_id: candidate.source_id.clone(),
                    source_start_ms: start,
                    source_end_ms: end,
                    reason: format!("{}:{}", variant, candidate.beat_label),
                });
            }
        } else {
            for (start, end) in chunks {
                segments.push(CutSegment {
                    id: format!("segment-{:03}", segments.len() + 1),
                    source_id: candidate.source_id.clone(),
                    source_start_ms: start,
                    source_end_ms: end,
                    reason: format!("{}:{}", variant, candidate.beat_label),
                });
            }
        }
    }
    if segments.is_empty() {
        return Err(ProjectError::InvalidState(
            "all editorial candidates are dropped; no cut plan can be rendered".into(),
        ));
    }
    let path = project_path.join(format!("edit/cut-plan-{variant}.json"));
    let segment_count = segments.len();
    if !dry_run {
        // §6.1: the variant-scoped path is the ONLY authority. A generic
        // `edit/cut-plan.json` alias used to be overwritten here on every
        // build regardless of which variant produced it, so building
        // `tight` then `natural` left the generic file holding whichever
        // variant ran last — and any stage reading it via a silent fallback
        // would silently mix variants. There is no in-crate reader of a
        // generic alias left, so none is written.
        write_json_atomic(
            &path,
            &CutPlan {
                schema_version: SCHEMA_VERSION,
                variant: variant.into(),
                gap_threshold_ms,
                head_margin_ms,
                tail_margin_ms,
                segments,
            },
        )?;
        let candidates_path = project_path.join("edit/candidates.json");
        let mut input_paths: Vec<PathBuf> = vec![candidates_path];
        for source in &sources.sources {
            input_paths.push(project_path.join(format!("analysis/vad-{}.json", source.source_id)));
        }
        input_paths.extend(transcript_file_paths(project_path)?);
        let inputs: Vec<&Path> = input_paths.iter().map(PathBuf::as_path).collect();
        receipts::write_stage_receipt(
            &receipts::receipt_path_for(&path),
            "edit.cut_plan",
            &inputs,
            &serde_json::json!({
                "variant": variant,
                "gap_threshold_ms": gap_threshold_ms,
                "head_margin_ms": head_margin_ms,
                "tail_margin_ms": tail_margin_ms,
            }),
            BTreeMap::new(),
            &[path.as_path()],
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: if dry_run { 0 } else { segment_count },
    })
}

fn vad_adjusted_bounds(start_ms: i64, end_ms: i64, vad: &VadSignal) -> (i64, i64) {
    const ADJACENCY_MS: i64 = 300;
    let mut start = start_ms;
    let mut end = end_ms;
    for region in &vad.regions {
        if region.start_ms <= start_ms && region.end_ms + ADJACENCY_MS >= start_ms {
            start = start.min(region.start_ms);
        }
        if region.end_ms >= end_ms && region.start_ms - ADJACENCY_MS <= end_ms {
            end = end.max(region.end_ms);
        }
    }
    (start, end)
}

/// Split a candidate's words into render chunks, compacting any inter-word pause
/// longer than `gap_threshold_ms` down to a retained residual of that size. The
/// first chunk keeps the head margin and the last chunk keeps the tail margin;
/// internal chunk boundaries keep only the residual pause. Words are never
/// clipped or overlapped, and because a tighter variant uses a smaller
/// threshold, it produces more, shorter chunks — removing more silence. Returns
/// no chunks when there are no words, so the caller can fall back to VAD bounds.
fn candidate_chunks(
    words: &[&Word],
    gap_threshold_ms: i64,
    head_margin_ms: i64,
    tail_margin_ms: i64,
    source_duration: Option<i64>,
) -> Vec<(i64, i64)> {
    if words.is_empty() {
        return Vec::new();
    }
    let clamp = |value: i64| -> i64 {
        let mut clamped = value.max(0);
        if let Some(duration) = source_duration {
            clamped = clamped.min(duration);
        }
        clamped
    };
    let mut chunks: Vec<(i64, i64)> = Vec::new();
    let mut chunk_start = words[0].start_ms - head_margin_ms;
    for pair in words.windows(2) {
        let current = pair[0];
        let next = pair[1];
        let gap = next.start_ms - current.end_ms;
        if gap > gap_threshold_ms {
            let tail_pad = gap_threshold_ms / 2;
            let head_pad = gap_threshold_ms - tail_pad;
            let out_start = clamp(chunk_start);
            let out_end = clamp(current.end_ms + tail_pad);
            if out_end > out_start {
                chunks.push((out_start, out_end));
            }
            chunk_start = next.start_ms - head_pad;
        }
    }
    let last_end = words[words.len() - 1].end_ms;
    let out_start = clamp(chunk_start);
    let out_end = clamp(last_end + tail_margin_ms);
    if out_end > out_start {
        chunks.push((out_start, out_end));
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_project;
    use crate::test_support::word;
    use std::fs;
    use video_core::models::SourceEntry;
    use video_core::{Candidate, Transcript};

    #[test]
    fn vad_expands_candidate_bounds_to_enclosing_speech() {
        let signal = VadSignal {
            schema_version: SCHEMA_VERSION,
            source_id: "source-a".into(),
            sample_rate: 16_000,
            provider: "heardright-silero".into(),
            regions: vec![video_core::VadRegion {
                start_ms: 900,
                end_ms: 2_100,
                mean_probability: 0.9,
            }],
        };
        assert_eq!(vad_adjusted_bounds(1_000, 2_000, &signal), (900, 2_100));
        assert_eq!(vad_adjusted_bounds(3_000, 3_200, &signal), (3_000, 3_200));
    }

    #[test]
    fn tight_variant_compacts_a_pause_that_natural_keeps() {
        // Two words separated by a 300ms internal pause.
        let w0 = word(0, 100);
        let w1 = word(400, 500);
        let words = vec![&w0, &w1];
        // tight threshold 220 < 300 gap -> split into two chunks.
        let tight = candidate_chunks(&words, 220, 90, 130, Some(10_000));
        assert_eq!(tight.len(), 2);
        // natural threshold 400 > 300 gap -> one chunk keeps the pause.
        let natural = candidate_chunks(&words, 400, 140, 220, Some(10_000));
        assert_eq!(natural.len(), 1);
        let span = |chunks: &[(i64, i64)]| chunks.iter().map(|(s, e)| e - s).sum::<i64>();
        assert!(span(&tight) < span(&natural));
    }

    #[test]
    fn candidate_chunks_never_clip_words() {
        let owned = vec![word(0, 100), word(400, 500), word(900, 1000)];
        let words: Vec<&Word> = owned.iter().collect();
        for threshold in [100, 220, 400, 1000] {
            let chunks = candidate_chunks(&words, threshold, 90, 130, Some(10_000));
            for w in &owned {
                let contained = chunks
                    .iter()
                    .any(|(s, e)| *s <= w.start_ms && w.end_ms <= *e);
                assert!(
                    contained,
                    "word {}..{} not contained at threshold {threshold}",
                    w.start_ms, w.end_ms
                );
            }
        }
    }

    #[test]
    fn build_cut_plan_makes_tight_remove_more_silence_than_natural() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
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
        write_json_atomic(
            &temp.path().join("analysis/vad-source-a.json"),
            &VadSignal {
                schema_version: SCHEMA_VERSION,
                source_id: "source-a".into(),
                sample_rate: 16_000,
                provider: "heardright-silero".into(),
                regions: vec![video_core::VadRegion {
                    start_ms: 0,
                    end_ms: 500,
                    mean_probability: 0.9,
                }],
            },
        )
        .unwrap();
        write_json_atomic(
            &temp.path().join("analysis/transcripts/source-a.json"),
            &Transcript {
                schema_version: SCHEMA_VERSION,
                provider: "fixture".into(),
                source_id: "source-a".into(),
                language: "en".into(),
                words: vec![word(0, 100), word(400, 500)],
                events: Vec::new(),
            },
        )
        .unwrap();
        write_json_atomic(
            &temp.path().join("edit/candidates.json"),
            &CandidateManifest {
                schema_version: SCHEMA_VERSION,
                candidates: vec![Candidate {
                    id: "candidate-1".into(),
                    source_id: "source-a".into(),
                    start_ms: 0,
                    end_ms: 500,
                    text: "two words".into(),
                    beat_label: "hook".into(),
                    take_rank: 1,
                    drop_reason: None,
                    filler_count: 0,
                }],
            },
        )
        .unwrap();

        build_cut_plan(temp.path(), "tight", false).unwrap();
        build_cut_plan(temp.path(), "natural", false).unwrap();
        let tight: CutPlan = read_json(&temp.path().join("edit/cut-plan-tight.json")).unwrap();
        let natural: CutPlan = read_json(&temp.path().join("edit/cut-plan-natural.json")).unwrap();
        // The 300ms pause exceeds tight's 220ms threshold (split) but not natural's 400ms.
        assert_eq!(tight.segments.len(), 2);
        assert_eq!(natural.segments.len(), 1);
        assert_eq!(tight.gap_threshold_ms, 220);
        assert_eq!(natural.gap_threshold_ms, 400);
    }

    /// REV2 plan §6.1 regression: building `natural` after `tight` must
    /// leave `tight`'s cut plan byte-for-byte unchanged. Before the fix,
    /// `build_cut_plan` additionally overwrote a shared generic
    /// `edit/cut-plan.json` on every call regardless of variant, so a
    /// downstream consumer that fell back to the generic alias when its own
    /// variant-scoped file was absent could pick up the OTHER variant's
    /// plan. The variant-scoped file itself was always correct, so this
    /// test targets the exact generic-alias write that made the fallback
    /// dangerous: `edit/cut-plan-tight.json` must be identical before and
    /// after `natural` is built, and no `edit/cut-plan.json` may exist.
    #[test]
    fn building_natural_after_tight_leaves_tight_byte_identical_and_writes_no_generic_alias() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
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
        write_json_atomic(
            &temp.path().join("analysis/vad-source-a.json"),
            &VadSignal {
                schema_version: SCHEMA_VERSION,
                source_id: "source-a".into(),
                sample_rate: 16_000,
                provider: "heardright-silero".into(),
                regions: vec![video_core::VadRegion {
                    start_ms: 0,
                    end_ms: 500,
                    mean_probability: 0.9,
                }],
            },
        )
        .unwrap();
        write_json_atomic(
            &temp.path().join("analysis/transcripts/source-a.json"),
            &Transcript {
                schema_version: SCHEMA_VERSION,
                provider: "fixture".into(),
                source_id: "source-a".into(),
                language: "en".into(),
                words: vec![word(0, 100), word(400, 500)],
                events: Vec::new(),
            },
        )
        .unwrap();
        write_json_atomic(
            &temp.path().join("edit/candidates.json"),
            &CandidateManifest {
                schema_version: SCHEMA_VERSION,
                candidates: vec![Candidate {
                    id: "candidate-1".into(),
                    source_id: "source-a".into(),
                    start_ms: 0,
                    end_ms: 500,
                    text: "two words".into(),
                    beat_label: "hook".into(),
                    take_rank: 1,
                    drop_reason: None,
                    filler_count: 0,
                }],
            },
        )
        .unwrap();

        build_cut_plan(temp.path(), "tight", false).unwrap();
        let tight_path = temp.path().join("edit/cut-plan-tight.json");
        let tight_bytes_before = fs::read(&tight_path).unwrap();

        build_cut_plan(temp.path(), "natural", false).unwrap();

        let tight_bytes_after = fs::read(&tight_path).unwrap();
        assert_eq!(
            tight_bytes_before, tight_bytes_after,
            "building natural must not mutate tight's cut plan"
        );
        assert!(
            !temp.path().join("edit/cut-plan.json").is_file(),
            "no generic edit/cut-plan.json alias should be written"
        );
    }
}
