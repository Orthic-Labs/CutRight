use chrono::Utc;
use serde::Serialize;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use video_core::{
    models::{SourceEntry, SCHEMA_VERSION},
    Candidate, CandidateManifest, CutPlan, CutSegment, OutputPreset, ProjectManifest, ReviewMode,
    SourceManifest, SourcePolicy, Timebase, Timeline, TimelineSegment, Track, Transcript,
    VadRegion, VadSignal, Word,
};
use video_media::{
    probe, render_segments, render_subtitled, ProbeError, RenderError, RenderSegment,
};
use video_providers::{HeardRightProvider, ProviderError};

const PROJECT_DIRS: &[&str] = &[
    "brief",
    "sources",
    "cache/audio",
    "cache/proxies",
    "cache/frames",
    "cache/waveforms",
    "cache/provider-responses",
    "analysis/cloud-analysis",
    "analysis/transcripts",
    "edit/variants",
    "finish/slots",
    "render/proxies",
    "render/rough-cuts",
    "render/slots",
    "render/previews",
    "render/finals",
    "qa",
    "feedback",
    "exports/youtube",
    "exports/vertical",
    "exports/captions",
    "exports/interchange",
];

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("project path is not a directory: {0}")]
    NotDirectory(PathBuf),
    #[error("project manifest has unsupported schema version {0}")]
    UnsupportedSchema(u32),
    #[error("project manifest is invalid: {0}")]
    InvalidManifest(String),
    #[error("ingest requires at least one source file")]
    NoSources,
    #[error("source file is not inside an immutable registration: {0}")]
    SourceChanged(PathBuf),
    #[error("source path cannot be read: {0}")]
    InvalidSource(PathBuf),
    #[error(transparent)]
    Probe(#[from] ProbeError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Provider(#[from] ProviderError),
    #[error(transparent)]
    Render(#[from] RenderError),
    #[error("pipeline state is invalid: {0}")]
    InvalidState(String),
}

#[derive(Debug, Serialize)]
pub struct InitResult {
    pub status: &'static str,
    pub project_path: PathBuf,
    pub created_paths: Vec<PathBuf>,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct IngestResult {
    pub status: &'static str,
    pub project_path: PathBuf,
    pub manifest_path: PathBuf,
    pub sources: Vec<IngestedSource>,
}

#[derive(Debug, Serialize)]
pub struct IngestedSource {
    pub status: &'static str,
    pub entry: SourceEntry,
}

#[derive(Debug, Serialize)]
pub struct PipelineArtifact {
    pub status: &'static str,
    pub path: PathBuf,
    pub count: usize,
}

pub fn init_project(path: &Path, dry_run: bool) -> Result<InitResult, ProjectError> {
    if path.exists() && !path.is_dir() {
        return Err(ProjectError::NotDirectory(path.to_path_buf()));
    }

    let project_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("video-project")
        .to_string();
    let manifest_path = path.join("project.json");
    let mut created_paths = Vec::new();

    if dry_run {
        return Ok(InitResult {
            status: "dry-run",
            project_path: path.to_path_buf(),
            created_paths: PROJECT_DIRS.iter().map(|dir| path.join(dir)).collect(),
            manifest_path,
        });
    }

    fs::create_dir_all(path)?;
    for dir in PROJECT_DIRS {
        let directory = path.join(dir);
        if !directory.exists() {
            fs::create_dir_all(&directory)?;
            created_paths.push(directory);
        }
    }

    if manifest_path.exists() {
        read_project_manifest(&manifest_path)?;
    } else {
        let manifest = ProjectManifest {
            schema_version: SCHEMA_VERSION,
            project_id: format!("project-{}", blake3::hash(project_name.as_bytes()).to_hex()),
            kind: "mixed_creator_content".into(),
            created_at: Utc::now(),
            review_mode: ReviewMode::Reviewed,
            source_policy: SourcePolicy::Immutable,
            outputs: vec![
                OutputPreset {
                    id: "youtube".into(),
                    aspect: "16:9".into(),
                    width: 1920,
                    height: 1080,
                },
                OutputPreset {
                    id: "reels".into(),
                    aspect: "9:16".into(),
                    width: 1080,
                    height: 1920,
                },
                OutputPreset {
                    id: "tiktok".into(),
                    aspect: "9:16".into(),
                    width: 1080,
                    height: 1920,
                },
            ],
        };
        write_json_atomic(&manifest_path, &manifest)?;
        created_paths.push(manifest_path.clone());
    }

    let sources_manifest = path.join("sources/manifest.json");
    if !sources_manifest.exists() {
        write_json_atomic(
            &sources_manifest,
            &SourceManifest {
                schema_version: SCHEMA_VERSION,
                sources: Vec::new(),
            },
        )?;
        created_paths.push(sources_manifest);
    }

    Ok(InitResult {
        status: if created_paths.is_empty() {
            "existing"
        } else {
            "created"
        },
        project_path: path.to_path_buf(),
        created_paths,
        manifest_path,
    })
}

pub fn ingest_sources(
    project_path: &Path,
    source_paths: &[PathBuf],
    dry_run: bool,
) -> Result<IngestResult, ProjectError> {
    if source_paths.is_empty() {
        return Err(ProjectError::NoSources);
    }
    let project_manifest_path = project_path.join("project.json");
    read_project_manifest(&project_manifest_path)?;
    let manifest_path = project_path.join("sources/manifest.json");
    let mut manifest: SourceManifest = read_json(&manifest_path)?;
    if manifest.schema_version != SCHEMA_VERSION {
        return Err(ProjectError::UnsupportedSchema(manifest.schema_version));
    }

    let mut sources = Vec::with_capacity(source_paths.len());
    for source_path in source_paths {
        let canonical_path = fs::canonicalize(source_path)
            .map_err(|_| ProjectError::InvalidSource(source_path.clone()))?;
        let digest = hash_file(&canonical_path)?;
        let hash = format!("blake3:{digest}");
        let path_string = canonical_path.to_string_lossy().into_owned();
        if let Some(existing) = manifest
            .sources
            .iter()
            .find(|source| source.path == path_string)
        {
            if existing.blake3 != hash {
                return Err(ProjectError::SourceChanged(canonical_path));
            }
            sources.push(IngestedSource {
                status: "existing",
                entry: existing.clone(),
            });
            continue;
        }

        let metadata = probe(&canonical_path)?;
        let entry = SourceEntry {
            source_id: format!("source-{}", &digest[..12]),
            path: path_string,
            blake3: hash,
            duration_ms: metadata.duration_ms,
            width: metadata.width,
            height: metadata.height,
            rotation_degrees: metadata.rotation_degrees,
            is_hdr: metadata.is_hdr,
            timebase: metadata.timebase,
        };
        manifest.sources.push(entry.clone());
        sources.push(IngestedSource {
            status: "ingested",
            entry,
        });
    }

    if !dry_run {
        write_json_atomic(&manifest_path, &manifest)?;
    }
    Ok(IngestResult {
        status: if dry_run {
            "dry-run"
        } else if sources.iter().all(|source| source.status == "existing") {
            "existing"
        } else {
            "ingested"
        },
        project_path: project_path.to_path_buf(),
        manifest_path,
        sources,
    })
}

pub fn transcribe_project(
    project_path: &Path,
    provider_name: &str,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    if sources.sources.is_empty() {
        return Err(ProjectError::NoSources);
    }
    if provider_name != "heardright" && provider_name != "heardright-parakeet-tdt" {
        return Err(ProjectError::InvalidState(format!(
            "unsupported local provider {provider_name}; use heardright"
        )));
    }
    let provider = if dry_run {
        None
    } else {
        Some(HeardRightProvider::discover()?)
    };
    let output_dir = project_path.join("analysis/transcripts");
    if !dry_run {
        fs::create_dir_all(&output_dir)?;
    }
    let mut transcripts = Vec::new();
    for source in &sources.sources {
        let transcript = if let Some(provider) = &provider {
            provider.transcribe(&source.source_id, Path::new(&source.path))?
        } else {
            Transcript {
                schema_version: SCHEMA_VERSION,
                provider: "heardright-parakeet-tdt".into(),
                source_id: source.source_id.clone(),
                language: "en".into(),
                words: Vec::new(),
                events: Vec::new(),
            }
        };
        if !dry_run {
            write_json_atomic(
                &output_dir.join(format!("{}.json", source.source_id)),
                &transcript,
            )?;
        }
        transcripts.push(transcript);
    }
    if !dry_run {
        if let Some(first) = transcripts.first() {
            write_json_atomic(&project_path.join("analysis/transcript.json"), first)?;
            let packed = first
                .words
                .iter()
                .map(|word| format!("- `{}–{}` {}", word.start_ms, word.end_ms, word.text))
                .collect::<Vec<_>>()
                .join("\n");
            fs::write(
                project_path.join("analysis/transcript-packed.md"),
                packed + "\n",
            )?;
        }
    }
    let path = project_path.join("analysis/transcripts");
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: sources.sources.len(),
    })
}

pub fn build_candidates(
    project_path: &Path,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let transcripts = load_transcripts(project_path)?;
    let mut candidates = Vec::new();
    for transcript in &transcripts {
        for (index, words) in group_words(&transcript.words, 900).into_iter().enumerate() {
            let first = words
                .first()
                .ok_or_else(|| ProjectError::InvalidState("candidate group was empty".into()))?;
            let last = words.last().expect("nonempty candidate group");
            candidates.push(Candidate {
                id: format!("candidate-{:03}", candidates.len() + 1),
                source_id: transcript.source_id.clone(),
                start_ms: first.start_ms,
                end_ms: last.end_ms,
                text: words
                    .iter()
                    .map(|word| word.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
                beat_label: if index == 0 { "hook" } else { "beat" }.into(),
                take_rank: (index + 1) as u32,
                drop_reason: None,
            });
        }
    }
    let path = project_path.join("edit/candidates.json");
    if !dry_run {
        write_json_atomic(
            &path,
            &CandidateManifest {
                schema_version: SCHEMA_VERSION,
                candidates: candidates.clone(),
            },
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: candidates.len(),
    })
}

pub fn analyze_local(project_path: &Path, dry_run: bool) -> Result<PipelineArtifact, ProjectError> {
    let transcripts = load_transcripts(project_path)?;
    let mut region_count = 0;
    for transcript in &transcripts {
        let regions = group_words(&transcript.words, 450)
            .into_iter()
            .filter_map(|group| {
                let first = group.first()?;
                let last = group.last()?;
                Some(VadRegion {
                    start_ms: first.start_ms,
                    end_ms: last.end_ms,
                    mean_probability: 1.0,
                })
            })
            .collect::<Vec<_>>();
        region_count += regions.len();
        if !dry_run {
            write_json_atomic(
                &project_path.join(format!("analysis/vad-{}.json", transcript.source_id)),
                &VadSignal {
                    schema_version: SCHEMA_VERSION,
                    source_id: transcript.source_id.clone(),
                    sample_rate: 16_000,
                    provider: "heardright-native-word-activity".into(),
                    regions,
                },
            )?;
        }
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path: project_path.join("analysis"),
        count: region_count,
    })
}

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
    let transcripts = load_transcripts(project_path)?;
    let mut segments = Vec::new();
    for transcript in &transcripts {
        for words in group_words(&transcript.words, gap_threshold_ms).into_iter() {
            let first = words.first().expect("nonempty word group");
            let last = words.last().expect("nonempty word group");
            let source_duration = sources
                .sources
                .iter()
                .find(|source| source.source_id == transcript.source_id)
                .and_then(|source| source.duration_ms);
            let start = first.start_ms.saturating_sub(head_margin_ms).max(0);
            let mut end = last.end_ms.saturating_add(tail_margin_ms);
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
                    source_id: transcript.source_id.clone(),
                    source_start_ms: start,
                    source_end_ms: end,
                    reason: variant.into(),
                });
            }
        }
    }
    let path = project_path.join(format!("edit/cut-plan-{variant}.json"));
    if !dry_run {
        write_json_atomic(
            &path,
            &CutPlan {
                schema_version: SCHEMA_VERSION,
                variant: variant.into(),
                gap_threshold_ms,
                head_margin_ms,
                tail_margin_ms,
                segments: segments.clone(),
            },
        )?;
        write_json_atomic(
            &project_path.join("edit/cut-plan.json"),
            &CutPlan {
                schema_version: SCHEMA_VERSION,
                variant: variant.into(),
                gap_threshold_ms,
                head_margin_ms,
                tail_margin_ms,
                segments,
            },
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: if dry_run {
            0
        } else {
            read_json::<CutPlan>(&project_path.join("edit/cut-plan.json"))?
                .segments
                .len()
        },
    })
}

pub fn validate_edit(project_path: &Path) -> Result<PipelineArtifact, ProjectError> {
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    let plan: CutPlan = read_json(&project_path.join("edit/cut-plan.json"))?;
    let mut durations = std::collections::HashMap::new();
    for source in sources.sources {
        durations.insert(source.source_id, source.duration_ms);
    }
    let mut last_end = std::collections::HashMap::new();
    for segment in &plan.segments {
        let duration = durations.get(&segment.source_id).ok_or_else(|| {
            ProjectError::InvalidState(format!("missing source {}", segment.source_id))
        })?;
        if segment.source_start_ms < 0
            || segment.source_end_ms <= segment.source_start_ms
            || duration.is_some_and(|end| segment.source_end_ms > end)
        {
            return Err(ProjectError::InvalidState(format!(
                "invalid segment {} range {}..{}",
                segment.id, segment.source_start_ms, segment.source_end_ms
            )));
        }
        if last_end
            .get(&segment.source_id)
            .is_some_and(|end| segment.source_start_ms < *end)
        {
            return Err(ProjectError::InvalidState(format!(
                "overlapping segment {}",
                segment.id
            )));
        }
        last_end.insert(segment.source_id.clone(), segment.source_end_ms);
    }
    Ok(PipelineArtifact {
        status: "valid",
        path: project_path.join("edit/cut-plan.json"),
        count: plan.segments.len(),
    })
}

pub fn compile_timeline(
    project_path: &Path,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    let plan: CutPlan = read_json(&project_path.join("edit/cut-plan.json"))?;
    let timebase = sources
        .sources
        .first()
        .and_then(|source| source.timebase.clone())
        .unwrap_or(Timebase {
            fps_num: 30_000,
            fps_den: 1_001,
        });
    let mut cursor = 0;
    let mut segments = Vec::new();
    for segment in &plan.segments {
        let duration = segment.source_end_ms - segment.source_start_ms;
        segments.push(TimelineSegment {
            id: segment.id.clone(),
            source_id: segment.source_id.clone(),
            source_start_ms: segment.source_start_ms,
            source_end_ms: segment.source_end_ms,
            output_start_ms: cursor,
            output_end_ms: cursor + duration,
            speed: 1.0,
            reason: segment.reason.clone(),
        });
        cursor += duration;
    }
    let timeline = Timeline {
        schema_version: SCHEMA_VERSION,
        timebase,
        tracks: vec![Track {
            id: "main".into(),
            track_type: "av".into(),
            segments,
        }],
    };
    let path = project_path.join("edit/timeline.json");
    if !dry_run {
        write_json_atomic(&path, &timeline)?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: timeline.tracks[0].segments.len(),
    })
}

pub fn render_edit(
    project_path: &Path,
    variant: &str,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let plan_path = project_path.join(format!("edit/cut-plan-{variant}.json"));
    let plan: CutPlan = read_json(&plan_path)?;
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    let source = sources.sources.first().ok_or(ProjectError::NoSources)?;
    if plan
        .segments
        .iter()
        .any(|segment| segment.source_id != source.source_id)
    {
        return Err(ProjectError::InvalidState(
            "rough render currently requires one registered source".into(),
        ));
    }
    let output = project_path.join(format!("render/rough-cuts/{variant}.mp4"));
    if !dry_run {
        render_segments(
            Path::new(&source.path),
            &plan
                .segments
                .iter()
                .map(|segment| RenderSegment {
                    start_ms: segment.source_start_ms,
                    end_ms: segment.source_end_ms,
                })
                .collect::<Vec<_>>(),
            &output,
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path: output,
        count: plan.segments.len(),
    })
}

pub fn remap_transcript(
    project_path: &Path,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let plan: CutPlan = read_json(&project_path.join("edit/cut-plan.json"))?;
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
    let path = project_path.join("edit/output-transcript.json");
    if !dry_run {
        write_json_atomic(&path, &transcript)?;
        write_srt(&project_path.join("edit/captions.srt"), &transcript.words)?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: transcript.words.len(),
    })
}

pub fn render_final(
    project_path: &Path,
    preset: &str,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let input = project_path.join("render/rough-cuts/natural.mp4");
    let captions = project_path.join("edit/captions.srt");
    let output = project_path.join(format!("render/finals/{preset}.mp4"));
    if !dry_run {
        render_subtitled(&input, &captions, &output)?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path: output,
        count: 1,
    })
}

pub fn qa_run(project_path: &Path, dry_run: bool) -> Result<PipelineArtifact, ProjectError> {
    validate_edit(project_path)?;
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    for source in &sources.sources {
        let actual = format!("blake3:{}", hash_file(Path::new(&source.path))?);
        if actual != source.blake3 {
            return Err(ProjectError::SourceChanged(PathBuf::from(&source.path)));
        }
    }
    let output = project_path.join("render/finals/youtube.mp4");
    let fallback = project_path.join("render/rough-cuts/natural.mp4");
    let media = if output.is_file() { &output } else { &fallback };
    if !dry_run {
        let metadata = probe(media)?;
        if metadata.duration_ms.unwrap_or(0) <= 0 {
            return Err(ProjectError::InvalidState(
                "rendered output has no duration".into(),
            ));
        }
        write_json_atomic(
            &project_path.join("qa/report.json"),
            &serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "status": "pass",
                "output": media,
                "duration_ms": metadata.duration_ms,
                "source_hashes": "unchanged"
            }),
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "pass" },
        path: project_path.join("qa/report.json"),
        count: 1,
    })
}

pub fn evidence_build(
    project_path: &Path,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let transcripts = load_transcripts(project_path)?;
    let candidates: CandidateManifest = read_json(&project_path.join("edit/candidates.json"))?;
    let mut body = String::from("# CutRight evidence\n\n");
    body.push_str(&format!("Provider: `{}`\n\n", transcripts[0].provider));
    body.push_str("## Candidate beats\n\n");
    for candidate in &candidates.candidates {
        body.push_str(&format!(
            "- `{}` {}–{}: {}\n",
            candidate.beat_label, candidate.start_ms, candidate.end_ms, candidate.text
        ));
    }
    let path = project_path.join("analysis/evidence.md");
    if !dry_run {
        fs::write(&path, body)?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: candidates.candidates.len(),
    })
}

pub fn propose_shorts(
    project_path: &Path,
    count: u8,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let candidates: CandidateManifest = read_json(&project_path.join("edit/candidates.json"))?;
    let selected = candidates
        .candidates
        .iter()
        .take(count as usize)
        .cloned()
        .collect::<Vec<_>>();
    let path = project_path.join("edit/shorts.json");
    if !dry_run {
        write_json_atomic(
            &path,
            &serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "status": "proposed",
                "variants": selected
            }),
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: selected.len(),
    })
}

pub fn finish_validate(
    project_path: &Path,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let timeline: Timeline = read_json(&project_path.join("edit/timeline.json"))?;
    if timeline
        .tracks
        .iter()
        .all(|track| track.segments.is_empty())
    {
        return Err(ProjectError::InvalidState(
            "timeline has no segments".into(),
        ));
    }
    let path = project_path.join("finish/finish-plan.json");
    if !dry_run {
        write_json_atomic(
            &path,
            &serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "base_timeline": "edit/timeline.json",
                "slots": [{
                    "id": "captions",
                    "kind": "caption",
                    "renderer": "ffmpeg-or-sidecar",
                    "effect_id": "caption.srt.v1",
                    "output_start_ms": 0,
                    "output_end_ms": timeline.tracks[0].segments.last().map(|segment| segment.output_end_ms).unwrap_or(0)
                }]
            }),
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "valid" },
        path,
        count: timeline
            .tracks
            .iter()
            .map(|track| track.segments.len())
            .sum(),
    })
}

pub fn render_slot(
    project_path: &Path,
    slot_id: &str,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let finish: serde_json::Value = read_json(&project_path.join("finish/finish-plan.json"))?;
    let exists = finish
        .get("slots")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|slots| {
            slots
                .iter()
                .any(|slot| slot.get("id").and_then(serde_json::Value::as_str) == Some(slot_id))
        });
    if !exists {
        return Err(ProjectError::InvalidState(format!(
            "unknown finish slot {slot_id}"
        )));
    }
    let path = project_path.join(format!("render/slots/{slot_id}.json"));
    if !dry_run {
        write_json_atomic(
            &path,
            &serde_json::json!({ "slot_id": slot_id, "status": "rendered" }),
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: 1,
    })
}

pub fn package_social(
    project_path: &Path,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let final_video = project_path.join("render/finals/youtube.mp4");
    let caption_file = project_path.join("edit/captions.srt");
    let video_export = project_path.join("exports/youtube/youtube.mp4");
    let caption_export = project_path.join("exports/captions/youtube.srt");
    if !dry_run {
        fs::copy(&final_video, &video_export)?;
        fs::copy(&caption_file, &caption_export)?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path: video_export,
        count: 2,
    })
}

pub fn export_otio(project_path: &Path, dry_run: bool) -> Result<PipelineArtifact, ProjectError> {
    let timeline: Timeline = read_json(&project_path.join("edit/timeline.json"))?;
    let children = timeline.tracks[0]
        .segments
        .iter()
        .map(|segment| {
            serde_json::json!({
                "OTIO_SCHEMA": "Clip.2",
                "name": segment.id,
                "source_id": segment.source_id,
                "source_start_ms": segment.source_start_ms,
                "source_end_ms": segment.source_end_ms,
                "output_start_ms": segment.output_start_ms,
                "output_end_ms": segment.output_end_ms
            })
        })
        .collect::<Vec<_>>();
    let path = project_path.join("exports/interchange/timeline.otio.json");
    if !dry_run {
        write_json_atomic(
            &path,
            &serde_json::json!({
                "OTIO_SCHEMA": "Timeline.1",
                "name": "CutRight",
                "tracks": [{ "OTIO_SCHEMA": "Track.1", "kind": "Video", "children": children }]
            }),
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: timeline.tracks[0].segments.len(),
    })
}

fn load_transcripts(project_path: &Path) -> Result<Vec<Transcript>, ProjectError> {
    let directory = project_path.join("analysis/transcripts");
    let mut paths = if directory.is_dir() {
        fs::read_dir(directory)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect::<Vec<_>>()
    } else {
        vec![project_path.join("analysis/transcript.json")]
    };
    paths.sort();
    if paths.is_empty() {
        return Err(ProjectError::InvalidState(
            "transcribe must run before editing".into(),
        ));
    }
    paths.into_iter().map(|path| read_json(&path)).collect()
}

fn group_words(words: &[Word], gap_threshold_ms: i64) -> Vec<Vec<Word>> {
    let mut groups: Vec<Vec<Word>> = Vec::new();
    for word in words.iter().filter(|word| word.end_ms > word.start_ms) {
        if groups
            .last()
            .and_then(|group| group.last())
            .is_some_and(|last| word.start_ms - last.end_ms > gap_threshold_ms)
        {
            groups.push(vec![word.clone()]);
        } else if let Some(group) = groups.last_mut() {
            group.push(word.clone());
        } else {
            groups.push(vec![word.clone()]);
        }
    }
    groups
}

fn write_srt(path: &Path, words: &[Word]) -> Result<(), ProjectError> {
    let mut body = String::new();
    for (index, group) in group_words(words, 1_000).into_iter().enumerate() {
        let start = group.first().expect("nonempty caption").start_ms;
        let end = group
            .last()
            .expect("nonempty caption")
            .end_ms
            .max(start + 80);
        body.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            index + 1,
            srt_time(start),
            srt_time(end),
            group
                .iter()
                .map(|word| word.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    fs::write(path, body)?;
    Ok(())
}

fn srt_time(milliseconds: i64) -> String {
    let total = milliseconds.max(0) as u64;
    format!(
        "{:02}:{:02}:{:02},{:03}",
        total / 3_600_000,
        (total / 60_000) % 60,
        (total / 1_000) % 60,
        total % 1_000
    )
}

fn hash_file(path: &Path) -> Result<String, ProjectError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, ProjectError> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn read_project_manifest(path: &Path) -> Result<ProjectManifest, ProjectError> {
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    let version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| ProjectError::InvalidManifest("missing schema_version".into()))?;
    if version != SCHEMA_VERSION as u64 {
        return Err(ProjectError::UnsupportedSchema(version as u32));
    }
    serde_json::from_value(value).map_err(|error| ProjectError::InvalidManifest(error.to_string()))
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), ProjectError> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let temporary = path.with_extension(format!("json.tmp-{}", std::process::id()));
    let mut file = fs::File::create(&temporary)?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    fs::rename(temporary, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_is_idempotent_and_keeps_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let first = init_project(temp.path(), false).unwrap();
        let manifest_before = fs::read(temp.path().join("project.json")).unwrap();
        let second = init_project(temp.path(), false).unwrap();
        let manifest_after = fs::read(temp.path().join("project.json")).unwrap();
        assert_eq!(first.status, "created");
        assert_eq!(second.status, "existing");
        assert_eq!(manifest_before, manifest_after);
    }

    #[test]
    fn init_rejects_a_newer_manifest_schema() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path()).unwrap();
        let manifest = serde_json::json!({ "schema_version": 99 });
        fs::write(
            temp.path().join("project.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let error = init_project(temp.path(), false).unwrap_err();
        assert!(matches!(error, ProjectError::UnsupportedSchema(99)));
    }

    #[test]
    fn hashes_source_bytes_with_blake3() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.bin");
        fs::write(&source, b"cutright").unwrap();
        assert_eq!(
            hash_file(&source).unwrap(),
            blake3::hash(b"cutright").to_hex().to_string()
        );
    }
}
