use crate::io::*;
use crate::receipts;
use crate::PipelineArtifact;
use crate::ProjectError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use video_core::{models::SCHEMA_VERSION, CutPlan, Transcript, Word};

/// The reviewed-base selection that gates final rendering (§6.2). A final
/// render resolves its inputs from this record unless a variant is passed
/// explicitly. Persisted at `feedback/variant-selection.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectionRecord {
    pub schema_version: u32,
    pub variant: String,
    pub rough_cut_path: String,
    pub rough_cut_blake3: String,
    pub rough_cut_size: u64,
    pub selected_at: DateTime<Utc>,
    pub selected_by: String,
}

pub fn remap_transcript(
    project_path: &Path,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    remap_transcript_with_variant(project_path, None, dry_run)
}

pub fn remap_transcript_for_variant(
    project_path: &Path,
    variant: &str,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    validate_variant(variant)?;
    remap_transcript_with_variant(project_path, Some(variant), dry_run)
}

pub fn remap_transcript_with_variant(
    project_path: &Path,
    variant: Option<&str>,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    if let Some(variant) = variant {
        validate_variant(variant)?;
    }
    let plan_path = match variant {
        Some(variant) => project_path.join(format!("edit/cut-plan-{variant}.json")),
        None => project_path.join("edit/cut-plan.json"),
    };
    let plan: CutPlan = read_json(&plan_path)?;
    let transcripts = load_transcripts(project_path)?;
    let mut output_words = Vec::new();
    let mut output_cursor = 0;
    for segment in &plan.segments {
        let output_start = output_cursor;
        for transcript in transcripts
            .iter()
            .filter(|item| item.source_id == segment.source_id)
        {
            for word in &transcript.words {
                if word.end_ms <= segment.source_start_ms || word.start_ms >= segment.source_end_ms
                {
                    continue;
                }
                let source_start = word.start_ms.max(segment.source_start_ms);
                let source_end = word.end_ms.min(segment.source_end_ms);
                output_words.push(Word {
                    id: format!("ow_{:06}", output_words.len()),
                    source_word_id: Some(format!("{}:{}", transcript.source_id, word.id)),
                    text: word.text.clone(),
                    start_ms: output_start + source_start - segment.source_start_ms,
                    end_ms: output_start + source_end - segment.source_start_ms,
                    confidence: word.confidence,
                    speaker: word.speaker.clone(),
                    kind: word.kind.clone(),
                });
            }
        }
        output_cursor += segment.source_end_ms - segment.source_start_ms;
    }
    let transcript = Transcript {
        schema_version: SCHEMA_VERSION,
        provider: "cutright-timeline-remap".into(),
        source_id: "timeline".into(),
        language: "en".into(),
        words: output_words,
        events: Vec::new(),
    };
    let path = match variant {
        Some(variant) => project_path.join(format!("edit/output-transcript-{variant}.json")),
        None => project_path.join("edit/output-transcript.json"),
    };
    if !dry_run {
        write_json_atomic(&path, &transcript)?;
        let captions_path = match variant {
            Some(variant) => project_path.join(format!("edit/captions-{variant}.srt")),
            None => project_path.join("edit/captions.srt"),
        };
        write_srt(&captions_path, &transcript.words)?;

        let mut inputs: Vec<PathBuf> = vec![plan_path.clone()];
        inputs.extend(transcript_file_paths(project_path)?);
        let input_refs: Vec<&Path> = inputs.iter().map(PathBuf::as_path).collect();
        receipts::write_stage_receipt(
            &receipts::receipt_path_for(&path),
            "edit.transcript_remap",
            &input_refs,
            &serde_json::json!({ "variant": variant }),
            BTreeMap::new(),
            &[path.as_path(), captions_path.as_path()],
        )?;

        // §6.1: once a variant's plan, timeline, remapped transcript,
        // captions, and rough cut all exist, bind them together into one
        // per-variant package receipt. A no-op until the rough cut and
        // timeline for this variant have both been rendered.
        if let Some(variant) = variant {
            write_variant_package_receipt(
                project_path,
                variant,
                &plan_path,
                &path,
                &captions_path,
            )?;
        }
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: transcript.words.len(),
    })
}

/// The §6.1 per-variant package receipt: binds the cut plan, timeline,
/// remapped transcript, captions, and rough cut for one variant together as
/// `render/rough-cuts/<variant>.artifact-receipt.json`. A no-op (not an
/// error) until the timeline and rough cut for that variant both exist,
/// since transcript remap can otherwise run before the rough cut render step
/// in some call orders.
fn write_variant_package_receipt(
    project_path: &Path,
    variant: &str,
    plan_path: &Path,
    output_transcript_path: &Path,
    captions_path: &Path,
) -> Result<(), ProjectError> {
    let timeline_path = project_path.join(format!("edit/timeline-{variant}.json"));
    let rough_cut_path = project_path.join(format!("render/rough-cuts/{variant}.mp4"));
    if !timeline_path.is_file() || !rough_cut_path.is_file() {
        return Ok(());
    }
    let inputs = [
        plan_path,
        timeline_path.as_path(),
        output_transcript_path,
        captions_path,
        rough_cut_path.as_path(),
    ];
    let receipt_path =
        project_path.join(format!("render/rough-cuts/{variant}.artifact-receipt.json"));
    receipts::write_stage_receipt(
        &receipt_path,
        "edit.variant_package",
        &inputs,
        &serde_json::json!({ "variant": variant }),
        BTreeMap::new(),
        &[],
    )?;
    Ok(())
}

/// Record an explicit reviewed-base selection (§6.2). Validates the variant,
/// confirms the rough cut exists, binds it by BLAKE3, and persists the record.
pub fn select_variant(
    project_path: &Path,
    variant: &str,
    selected_by: &str,
) -> Result<SelectionRecord, ProjectError> {
    validate_variant(variant)?;
    let rough_cut_rel = format!("render/rough-cuts/{variant}.mp4");
    let rough_cut = project_path.join(&rough_cut_rel);
    if !rough_cut.is_file() {
        return Err(ProjectError::InvalidState(format!(
            "variant selection requires a rendered rough cut: {rough_cut_rel}"
        )));
    }
    let digest = hash_file(&rough_cut)?;
    let record = SelectionRecord {
        schema_version: SCHEMA_VERSION,
        variant: variant.to_string(),
        rough_cut_path: rough_cut_rel,
        rough_cut_blake3: format!("blake3:{digest}"),
        rough_cut_size: fs::metadata(&rough_cut)?.len(),
        selected_at: Utc::now(),
        selected_by: selected_by.to_string(),
    };
    write_json_atomic(
        &project_path.join("feedback/variant-selection.json"),
        &record,
    )?;
    Ok(record)
}

/// Read the current reviewed-base selection, if any.
pub fn read_variant_selection(
    project_path: &Path,
) -> Result<Option<SelectionRecord>, ProjectError> {
    let path = project_path.join("feedback/variant-selection.json");
    if !path.is_file() {
        return Ok(None);
    }
    Ok(Some(read_json(&path)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_project;
    use video_core::CutSegment;

    #[test]
    fn variant_remap_preserves_unique_compound_source_word_ids() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        let source_word = |source_id: &str| Transcript {
            schema_version: SCHEMA_VERSION,
            provider: "fixture".into(),
            source_id: source_id.into(),
            language: "en".into(),
            words: vec![Word {
                id: "w_000000".into(),
                source_word_id: None,
                text: source_id.into(),
                start_ms: 0,
                end_ms: 100,
                confidence: 1.0,
                speaker: None,
                kind: "word".into(),
            }],
            events: Vec::new(),
        };
        write_json_atomic(
            &temp.path().join("analysis/transcripts/source-a.json"),
            &source_word("source-a"),
        )
        .unwrap();
        write_json_atomic(
            &temp.path().join("analysis/transcripts/source-b.json"),
            &source_word("source-b"),
        )
        .unwrap();
        write_json_atomic(
            &temp.path().join("edit/cut-plan-tight.json"),
            &CutPlan {
                schema_version: SCHEMA_VERSION,
                variant: "tight".into(),
                gap_threshold_ms: 0,
                head_margin_ms: 0,
                tail_margin_ms: 0,
                segments: vec![
                    CutSegment {
                        id: "one".into(),
                        source_id: "source-a".into(),
                        source_start_ms: 0,
                        source_end_ms: 100,
                        reason: "fixture".into(),
                    },
                    CutSegment {
                        id: "two".into(),
                        source_id: "source-b".into(),
                        source_start_ms: 0,
                        source_end_ms: 100,
                        reason: "fixture".into(),
                    },
                ],
            },
        )
        .unwrap();

        let result = remap_transcript_for_variant(temp.path(), "tight", false).unwrap();
        assert_eq!(
            result.path,
            temp.path().join("edit/output-transcript-tight.json")
        );
        assert!(temp.path().join("edit/captions-tight.srt").is_file());
        let output: Transcript = read_json(&result.path).unwrap();
        let ids = output
            .words
            .iter()
            .map(|word| word.source_word_id.clone().unwrap())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains("source-a:w_000000"));
        assert!(ids.contains("source-b:w_000000"));
    }

    #[test]
    fn select_variant_binds_rough_cut_hash_and_reads_back() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        fs::write(
            temp.path().join("render/rough-cuts/natural.mp4"),
            b"natural-bytes",
        )
        .unwrap();

        let record = select_variant(temp.path(), "natural", "cli").unwrap();
        assert_eq!(record.schema_version, SCHEMA_VERSION);
        assert_eq!(record.variant, "natural");
        assert_eq!(record.rough_cut_path, "render/rough-cuts/natural.mp4");
        assert_eq!(record.rough_cut_size, b"natural-bytes".len() as u64);
        assert_eq!(
            record.rough_cut_blake3,
            format!("blake3:{}", blake3::hash(b"natural-bytes").to_hex())
        );
        assert_eq!(record.selected_by, "cli");

        let read = read_variant_selection(temp.path())
            .unwrap()
            .expect("selection persists");
        assert_eq!(read.variant, "natural");
        assert_eq!(read.rough_cut_blake3, record.rough_cut_blake3);

        assert!(select_variant(temp.path(), "wide", "cli").is_err());
        assert!(select_variant(temp.path(), "tight", "cli").is_err());

        let fresh = tempfile::tempdir().unwrap();
        init_project(fresh.path(), false).unwrap();
        assert!(read_variant_selection(fresh.path()).unwrap().is_none());
    }
}
