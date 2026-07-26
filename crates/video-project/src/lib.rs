use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use thiserror::Error;
use video_core::{
    models::{ProviderCost, ProviderResponseEnvelope, SourceEntry, SCHEMA_VERSION},
    providers::{TranscriptionProvider, TranscriptionRequest, VadProvider, VadRequest},
    Candidate, CandidateManifest, CutPlan, CutSegment, OutputPreset, ProjectManifest, ReviewMode,
    SourceManifest, SourcePolicy, Timebase, Timeline, TimelineSegment, Track, Transcript,
    VadSignal, Word,
};
use video_media::{
    compose_decision_evidence, extract_audio_f32, extract_frame, probe, render_boundary_probe,
    render_preset_with_captions, render_preset_with_captions_and_reframe, render_segments,
    render_source_segments, render_waveform, render_waveform_range, AudioError, ProbeError,
    ReframeAnchor, RenderError, RenderSegment, SourceRenderSegment,
};
use video_providers::{HeardRightProvider, ProviderError, SileroVadProvider, WhisperXProvider};

const PROJECT_DIRS: &[&str] = &[
    "brief",
    "sources",
    "cache/audio",
    "cache/proxies",
    "cache/frames",
    "cache/waveforms",
    "cache/provider-responses",
    "analysis/cloud-analysis",
    "analysis/bench/transcribe",
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
    #[error(transparent)]
    Audio(#[from] AudioError),
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

/// Read-only, Studio-facing summary of a project. Optional artifacts are omitted when their
/// producing pipeline stage has not run (or the artifact is unreadable).
#[derive(Debug, Serialize)]
pub struct ProjectSnapshot {
    pub schema_version: u32,
    pub project_path: PathBuf,
    pub manifest: ProjectManifest,
    pub generated_at: DateTime<Utc>,
    pub sources: Vec<SourceSnapshot>,
    pub stages: PipelineStages,
    pub variants: Vec<VariantSnapshot>,
    pub finals: Vec<FinalSnapshot>,
    pub qa: Option<serde_json::Value>,
    pub bench: Option<BenchSnapshot>,
    pub reframe_plan: Option<serde_json::Value>,
    pub decisions_path: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct SourceSnapshot {
    #[serde(flatten)]
    pub source: SourceEntry,
    pub file_present: bool,
    pub transcript: Option<PathBuf>,
    pub stages: SourceStages,
    pub waveform_png: Option<PathBuf>,
    pub poster_jpg: Option<PathBuf>,
}

#[derive(Debug, Serialize, Default)]
pub struct SourceStages {
    pub ingested: bool,
    pub transcribed: bool,
    pub analyzed: bool,
    pub in_candidates: bool,
    pub in_cut: bool,
}

#[derive(Debug, Serialize, Default)]
pub struct PipelineStages {
    pub ingested: bool,
    pub transcribed: bool,
    pub analyzed: bool,
    pub candidates: bool,
    pub rough_cut: bool,
    #[serde(rename = "final")]
    pub final_render: bool,
    pub qa: bool,
}

#[derive(Debug, Serialize)]
pub struct VariantSnapshot {
    pub id: String,
    pub mp4: Option<PathBuf>,
    pub mp4_mtime: Option<DateTime<Utc>>,
    pub fps: Option<f64>,
    pub cut_plan: Option<CutPlan>,
    pub output_transcript: Option<PathBuf>,
    pub srt: Option<PathBuf>,
    pub segment_count: Option<usize>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct FinalSnapshot {
    pub preset: String,
    pub aspect: String,
    pub width: u32,
    pub height: u32,
    pub mp4: PathBuf,
    pub mp4_mtime: Option<DateTime<Utc>>,
    pub fps: Option<f64>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct BenchSnapshot {
    pub decision: Option<String>,
    pub report: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ReframePlan {
    approved: bool,
    anchors: Vec<ReframePlanAnchor>,
}

#[derive(Debug, Deserialize)]
struct ReframePlanAnchor {
    output_start_ms: i64,
    output_end_ms: i64,
    source_id: String,
    center_x: f64,
    center_y: f64,
    approved: bool,
}

#[derive(Debug, Deserialize)]
struct VisionAnchorResponse {
    found: bool,
    center_x: f64,
    center_y: f64,
    confidence: f64,
}

/// Returns the filesystem-backed state Studio needs without mutating or hashing the project.
pub fn project_snapshot(project_path: &Path) -> Result<ProjectSnapshot, ProjectError> {
    let project_path = project_path.canonicalize()?;
    let manifest = read_project_manifest(&project_path.join("project.json"))?;
    let source_manifest_path = project_path.join("sources/manifest.json");
    let sources_manifest = read_json_if_file::<SourceManifest>(&source_manifest_path);
    let candidates =
        read_json_if_file::<CandidateManifest>(&project_path.join("edit/candidates.json"));
    let variant_ids = ["tight", "natural"];
    let variant_plans = variant_ids
        .iter()
        .map(|id| {
            (
                *id,
                read_json_if_file::<CutPlan>(
                    &project_path.join(format!("edit/cut-plan-{id}.json")),
                ),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();

    let sources: Vec<SourceSnapshot> = sources_manifest
        .as_ref()
        .map(|manifest| {
            manifest
                .sources
                .iter()
                .cloned()
                .map(|mut source| {
                    let source_path = absolute_path(&project_path, Path::new(&source.path));
                    source.path = source_path.to_string_lossy().into_owned();
                    let transcript = existing_path(
                        project_path
                            .join(format!("analysis/transcripts/{}.json", source.source_id)),
                    );
                    let analyzed = project_path
                        .join(format!("analysis/vad-{}.json", source.source_id))
                        .is_file();
                    let in_candidates = candidates.as_ref().is_some_and(|items| {
                        items
                            .candidates
                            .iter()
                            .any(|candidate| candidate.source_id == source.source_id)
                    });
                    let in_cut = variant_plans.values().flatten().any(|plan| {
                        plan.segments
                            .iter()
                            .any(|segment| segment.source_id == source.source_id)
                    });
                    SourceSnapshot {
                        file_present: source_path.is_file(),
                        transcript,
                        stages: SourceStages {
                            ingested: true,
                            transcribed: project_path
                                .join(format!("analysis/transcripts/{}.json", source.source_id))
                                .is_file(),
                            analyzed,
                            in_candidates,
                            in_cut,
                        },
                        waveform_png: existing_path(
                            project_path.join(format!("cache/waveforms/{}.png", source.source_id)),
                        ),
                        poster_jpg: existing_path(
                            project_path.join(format!("cache/frames/{}.jpg", source.source_id)),
                        ),
                        source,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let variants = variant_ids
        .iter()
        .map(|id| snapshot_variant(&project_path, id, variant_plans.get(id).cloned().flatten()))
        .collect::<Vec<_>>();
    let finals = manifest
        .outputs
        .iter()
        .filter_map(|preset| snapshot_final(&project_path, preset))
        .collect::<Vec<_>>();
    let stages = PipelineStages {
        ingested: !sources.is_empty(),
        transcribed: !sources.is_empty() && sources.iter().all(|source| source.stages.transcribed),
        analyzed: !sources.is_empty() && sources.iter().all(|source| source.stages.analyzed),
        candidates: candidates.is_some(),
        rough_cut: variants.iter().any(|variant| variant.mp4.is_some()),
        final_render: !finals.is_empty(),
        qa: project_path.join("qa/report.json").is_file(),
    };
    let bench_path = project_path.join("analysis/bench/transcribe/report.json");

    Ok(ProjectSnapshot {
        schema_version: SCHEMA_VERSION,
        project_path: project_path.clone(),
        manifest,
        generated_at: Utc::now(),
        sources,
        stages,
        variants,
        finals,
        qa: read_value_if_file(&project_path.join("qa/report.json")),
        bench: read_value_if_file(&bench_path).map(|report| BenchSnapshot {
            decision: report
                .get("decision")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            report: bench_path,
        }),
        reframe_plan: read_value_if_file(&project_path.join("analysis/reframe-plan.json")),
        decisions_path: project_path.join("feedback/decisions.jsonl"),
    })
}

fn snapshot_variant(project_path: &Path, id: &str, cut_plan: Option<CutPlan>) -> VariantSnapshot {
    let mp4_path = project_path.join(format!("render/rough-cuts/{id}.mp4"));
    let metadata = mp4_path.is_file().then(|| probe(&mp4_path).ok()).flatten();
    VariantSnapshot {
        id: id.to_string(),
        mp4: existing_path(mp4_path.clone()),
        mp4_mtime: file_mtime(&mp4_path),
        fps: metadata
            .as_ref()
            .and_then(|value| value.timebase.as_ref())
            .map(fps),
        output_transcript: existing_path(
            project_path.join(format!("edit/output-transcript-{id}.json")),
        ),
        srt: existing_path(project_path.join(format!("edit/captions-{id}.srt"))),
        segment_count: cut_plan.as_ref().map(|plan| plan.segments.len()),
        duration_ms: metadata.and_then(|value| value.duration_ms),
        cut_plan,
    }
}

fn snapshot_final(project_path: &Path, preset: &OutputPreset) -> Option<FinalSnapshot> {
    let mp4 = project_path.join(format!("render/finals/{}.mp4", preset.id));
    if !mp4.is_file() {
        return None;
    }
    let metadata = probe(&mp4).ok();
    Some(FinalSnapshot {
        preset: preset.id.clone(),
        aspect: preset.aspect.clone(),
        width: preset.width,
        height: preset.height,
        mp4: mp4.clone(),
        mp4_mtime: file_mtime(&mp4),
        fps: metadata
            .as_ref()
            .and_then(|value| value.timebase.as_ref())
            .map(fps),
        duration_ms: metadata.and_then(|value| value.duration_ms),
    })
}

fn existing_path(path: PathBuf) -> Option<PathBuf> {
    path.is_file().then_some(path)
}

fn absolute_path(project_path: &Path, path: &Path) -> PathBuf {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_path.join(path)
    };
    path.canonicalize().unwrap_or(path)
}

fn file_mtime(path: &Path) -> Option<DateTime<Utc>> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()
        .map(DateTime::<Utc>::from)
}

fn fps(timebase: &Timebase) -> f64 {
    f64::from(timebase.fps_num) / f64::from(timebase.fps_den)
}

fn read_json_if_file<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    path.is_file().then(|| read_json(path).ok()).flatten()
}

fn read_value_if_file(path: &Path) -> Option<serde_json::Value> {
    read_json_if_file(path)
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
    let provider = if dry_run {
        None
    } else {
        Some(match provider_name {
            "heardright" | "heardright-parakeet-tdt" => {
                Box::new(HeardRightProvider::discover()?) as Box<dyn TranscriptionProvider>
            }
            "whisperx" | "whisperx-alignment" => {
                Box::new(WhisperXProvider::discover()?) as Box<dyn TranscriptionProvider>
            }
            other => {
                return Err(ProjectError::InvalidState(format!(
                    "unsupported local provider {other}; use heardright or whisperx"
                )))
            }
        })
    };
    let output_dir = project_path.join("analysis/transcripts");
    let provider_suffix = match provider_name {
        "heardright" | "heardright-parakeet-tdt" => None,
        "whisperx" | "whisperx-alignment" => Some("whisperx"),
        _ => None,
    };
    let provider_label = provider_suffix.unwrap_or("heardright");
    if !dry_run {
        fs::create_dir_all(&output_dir)?;
    }
    let mut transcripts = Vec::new();
    let mut cache_hits = 0_usize;
    for source in &sources.sources {
        let transcript_path = match provider_suffix {
            Some(suffix) => output_dir.join(format!("{}.{}.json", source.source_id, suffix)),
            None => output_dir.join(format!("{}.json", source.source_id)),
        };
        let transcript = if let Some(provider) = &provider {
            if let Some(cached) = load_cached_transcription(
                project_path,
                source,
                &transcript_path,
                provider.id(),
                provider.model_id(),
                provider_label,
            )? {
                cache_hits += 1;
                cached
            } else {
                let output = provider
                    .transcribe(&TranscriptionRequest {
                        source_id: source.source_id.clone(),
                        source_path: PathBuf::from(&source.path),
                        language_hint: Some("en".into()),
                    })
                    .map_err(|error| ProjectError::InvalidState(error.to_string()))?;
                write_json_atomic(&transcript_path, &output.transcript)?;
                write_transcription_provenance(
                    project_path,
                    source,
                    &transcript_path,
                    TranscriptionProvenance {
                        raw_response: &output.raw_response,
                        provider: provider.id(),
                        provider_model: &output.provider_model,
                        warnings: &output.warnings,
                        provider_label,
                    },
                )?;
                output.transcript
            }
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
        transcripts.push(transcript);
    }
    if !dry_run && provider_suffix.is_none() {
        let first = transcripts.first().expect("sources are nonempty");
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
    let path = project_path.join("analysis/transcripts");
    Ok(PipelineArtifact {
        status: if dry_run {
            "dry-run"
        } else if cache_hits == sources.sources.len() {
            "cached"
        } else {
            "created"
        },
        path,
        count: sources.sources.len(),
    })
}

fn load_cached_transcription(
    project_path: &Path,
    source: &SourceEntry,
    transcript_path: &Path,
    provider: &str,
    provider_model: &str,
    provider_label: &str,
) -> Result<Option<Transcript>, ProjectError> {
    let raw_path = project_path.join(format!(
        "cache/provider-responses/{}.{}.raw.json",
        source.source_id, provider_label
    ));
    let envelope_path = project_path.join(format!(
        "analysis/transcripts/{}.{}.envelope.json",
        source.source_id, provider_label
    ));
    if !transcript_path.is_file() || !raw_path.is_file() || !envelope_path.is_file() {
        return Ok(None);
    }
    let envelope: ProviderResponseEnvelope = read_json(&envelope_path)?;
    if envelope.provider != provider
        || envelope.provider_model != provider_model
        || envelope.request_hash != transcription_request_hash(source, provider, provider_model)?
    {
        return Ok(None);
    }
    Ok(Some(read_json(transcript_path)?))
}

struct TranscriptionProvenance<'a> {
    raw_response: &'a serde_json::Value,
    provider: &'a str,
    provider_model: &'a str,
    warnings: &'a [String],
    provider_label: &'a str,
}

fn write_transcription_provenance(
    project_path: &Path,
    source: &SourceEntry,
    transcript_path: &Path,
    provenance: TranscriptionProvenance<'_>,
) -> Result<(), ProjectError> {
    let raw_path = project_path.join(format!(
        "cache/provider-responses/{}.{}.raw.json",
        source.source_id, provenance.provider_label
    ));
    write_json_atomic(&raw_path, provenance.raw_response)?;
    let envelope = ProviderResponseEnvelope {
        provider: provenance.provider.into(),
        provider_model: provenance.provider_model.into(),
        request_hash: transcription_request_hash(
            source,
            provenance.provider,
            provenance.provider_model,
        )?,
        created_at: Utc::now(),
        cost: ProviderCost {
            currency: "USD".into(),
            estimated: Some(0.0),
        },
        raw_response_path: relative_artifact_path(project_path, &raw_path),
        normalised_output_path: relative_artifact_path(project_path, transcript_path),
        warnings: provenance.warnings.to_vec(),
    };
    let envelope_path = project_path.join(format!(
        "analysis/transcripts/{}.{}.envelope.json",
        source.source_id, provenance.provider_label
    ));
    write_json_atomic(&envelope_path, &envelope)
}

fn transcription_request_hash(
    source: &SourceEntry,
    provider: &str,
    provider_model: &str,
) -> Result<String, ProjectError> {
    let request = serde_json::json!({
        "provider": provider,
        "provider_model": provider_model,
        "source_id": source.source_id,
        "source_blake3": source.blake3,
        "source_path": source.path,
        "language_hint": "en"
    });
    Ok(blake3::hash(&serde_json::to_vec(&request)?)
        .to_hex()
        .to_string())
}

fn relative_artifact_path(project_path: &Path, path: &Path) -> String {
    path.strip_prefix(project_path)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
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
    transcribe_project(project_path, primary, false)?;
    transcribe_project(project_path, verifier, false)?;
    let mut total_primary_non_clean = 0_usize;
    let mut total_verifier_non_clean = 0_usize;
    let mut total_primary_unmatched = 0_usize;
    let mut total_verifier_unmatched = 0_usize;
    let mut clips = Vec::new();
    for source in &sources.sources {
        let primary_path =
            project_path.join(format!("analysis/transcripts/{}.json", source.source_id));
        let verifier_path = project_path.join(format!(
            "analysis/transcripts/{}.whisperx.json",
            source.source_id
        ));
        let primary_transcript: Transcript = read_json(&primary_path)?;
        let verifier_transcript: Transcript = read_json(&verifier_path)?;
        let alignment = align_words(&primary_transcript.words, &verifier_transcript.words);
        let mut primary_checks = aligned_boundary_checks(
            &primary_transcript.words,
            &verifier_transcript.words,
            &alignment.matches,
            true,
            boundaries,
            padding_ms,
        );
        let verifier_checks = aligned_boundary_checks(
            &verifier_transcript.words,
            &primary_transcript.words,
            &alignment.matches,
            false,
            boundaries,
            padding_ms,
        );
        let primary_non_clean = primary_checks
            .iter()
            .filter(|check| check["status"] != "clean")
            .count();
        let verifier_non_clean = verifier_checks
            .iter()
            .filter(|check| check["status"] != "clean")
            .count();
        total_primary_non_clean += primary_non_clean;
        total_verifier_non_clean += verifier_non_clean;
        total_primary_unmatched += alignment.unmatched_primary.len();
        total_verifier_unmatched += alignment.unmatched_verifier.len();
        for (index, check) in primary_checks.iter_mut().enumerate() {
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
        clips.push(serde_json::json!({
            "source_id": source.source_id,
            "source_path": source.path,
            "source_blake3": source.blake3,
            "primary_transcript": primary_path.strip_prefix(project_path).unwrap_or(&primary_path),
            "verifier_transcript": verifier_path.strip_prefix(project_path).unwrap_or(&verifier_path),
            "primary_checks": primary_checks,
            "verifier_checks": verifier_checks,
            "counts": {
                "primary_non_clean": primary_non_clean,
                "verifier_non_clean": verifier_non_clean,
                "primary_unmatched_words": alignment.unmatched_primary.len(),
                "verifier_unmatched_words": alignment.unmatched_verifier.len()
            }
        }));
    }
    let primary_eligible = total_primary_non_clean == 0 && total_primary_unmatched == 0;
    let verifier_eligible = total_verifier_non_clean == 0 && total_verifier_unmatched == 0;
    let decision = benchmark_decision(primary, verifier, primary_eligible, verifier_eligible);
    let path = project_path.join("analysis/bench/transcribe/report.json");
    write_json_atomic(
        &path,
        &serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "kind": "transcription_benchmark",
            "primary": primary,
            "verifier": verifier,
            "boundaries_requested": boundaries,
            "padding_ms": padding_ms,
            "clips": clips,
            "summary": {
                "primary_non_clean": total_primary_non_clean,
                "verifier_non_clean": total_verifier_non_clean,
                "primary_unmatched_words": total_primary_unmatched,
                "verifier_unmatched_words": total_verifier_unmatched,
                "primary_eligible": primary_eligible,
                "verifier_eligible": verifier_eligible
            },
            "decision": decision
        }),
    )?;
    if decision == "unresolved" {
        return Err(ProjectError::InvalidState(format!(
            "transcription benchmark is unresolved; inspect {}",
            path.display()
        )));
    }
    Ok(PipelineArtifact {
        status: "created",
        path,
        count: sources.sources.len(),
    })
}

fn benchmark_decision<'a>(
    primary: &'a str,
    verifier: &'a str,
    primary_eligible: bool,
    verifier_eligible: bool,
) -> &'a str {
    match (primary_eligible, verifier_eligible) {
        (true, false) => primary,
        (false, true) => verifier,
        (true, true) | (false, false) => "unresolved",
    }
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
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    if dry_run {
        return Ok(PipelineArtifact {
            status: "dry-run",
            path: project_path.join("analysis"),
            count: sources.sources.len(),
        });
    }
    let provider = SileroVadProvider::discover()?;
    let mut region_count = 0;
    for source in &sources.sources {
        let audio_path = project_path.join(format!("cache/audio/{}-16k.f32", source.source_id));
        extract_audio_f32(Path::new(&source.path), &audio_path, 16_000)?;
        let signal = provider
            .analyze(&VadRequest {
                source_id: source.source_id.clone(),
                audio_path,
                sample_rate: 16_000,
                threshold: 0.5,
            })
            .map_err(|error| ProjectError::InvalidState(error.to_string()))?;
        region_count += signal.regions.len();
        write_json_atomic(
            &project_path.join(format!("analysis/vad-{}.json", source.source_id)),
            &signal,
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path: project_path.join("analysis"),
        count: region_count,
    })
}

pub fn reframe_plan(project_path: &Path, dry_run: bool) -> Result<PipelineArtifact, ProjectError> {
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    if sources.sources.is_empty() {
        return Err(ProjectError::NoSources);
    }
    let timeline: Timeline = read_json(&project_path.join("edit/timeline.json")).map_err(|_| {
        ProjectError::InvalidState(
            "reframe planning requires edit/timeline.json; run `videoctl edit render <project> --variant natural` first".into(),
        )
    })?;
    let timeline_segments = &timeline
        .tracks
        .first()
        .ok_or_else(|| ProjectError::InvalidState("timeline has no main track".into()))?
        .segments;
    let path = project_path.join("analysis/reframe-plan.json");
    if !dry_run {
        let worker = vision_anchor_worker()?;
        let mut anchors = Vec::with_capacity(timeline_segments.len());
        for segment in timeline_segments {
            let source = sources
                .sources
                .iter()
                .find(|source| source.source_id == segment.source_id)
                .ok_or_else(|| {
                    ProjectError::InvalidState(format!(
                        "reframe segment {} references a missing source",
                        segment.id
                    ))
                })?;
            let frame = project_path.join(format!("cache/frames/reframe-{}.jpg", segment.id));
            extract_frame(
                Path::new(&source.path),
                segment.source_start_ms + (segment.source_end_ms - segment.source_start_ms) / 2,
                &frame,
            )?;
            let vision = detect_vision_anchor(&worker, &frame)?;
            anchors.push(serde_json::json!({
                "source_id": segment.source_id,
                "output_start_ms": segment.output_start_ms,
                "output_end_ms": segment.output_end_ms,
                "center_x": vision.center_x,
                "center_y": vision.center_y,
                "strategy": if vision.found { "vision_face" } else { "manual_anchor_required" },
                "confidence": vision.confidence,
                "approved": false
            }));
        }
        write_json_atomic(
            &path,
            &serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "kind": "timeline_reframe_plan",
                "target_aspect": "9:16",
                "approved": false,
                "requires_review": true,
                "anchors": anchors
            }),
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: timeline_segments.len(),
    })
}

fn vision_anchor_worker() -> Result<PathBuf, ProjectError> {
    let worker = std::env::temp_dir().join(format!(
        "cutright-vision-anchor-{}",
        env!("CARGO_PKG_VERSION")
    ));
    if !worker.is_file() {
        fs::write(&worker, include_bytes!(env!("CUTRIGHT_VISION_ANCHOR")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&worker, fs::Permissions::from_mode(0o700))?;
        }
    }
    Ok(worker)
}

fn detect_vision_anchor(worker: &Path, frame: &Path) -> Result<VisionAnchorResponse, ProjectError> {
    let request = serde_json::json!({ "image_path": frame });
    let mut child = Command::new(worker)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .as_mut()
        .expect("piped vision stdin")
        .write_all(&serde_json::to_vec(&request)?)?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(ProjectError::InvalidState(format!(
            "Vision reframe anchor failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let anchor: VisionAnchorResponse = serde_json::from_slice(&output.stdout)?;
    if !(0.0..=1.0).contains(&anchor.center_x) || !(0.0..=1.0).contains(&anchor.center_y) {
        return Err(ProjectError::InvalidState(
            "Vision reframe anchor returned invalid normalized coordinates".into(),
        ));
    }
    Ok(anchor)
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
    }
    if segments.is_empty() {
        return Err(ProjectError::InvalidState(
            "all editorial candidates are dropped; no cut plan can be rendered".into(),
        ));
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
    let output = project_path.join(format!("render/rough-cuts/{variant}.mp4"));
    if !dry_run {
        if sources.sources.len() == 1 {
            let source = sources.sources.first().expect("sources are nonempty");
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
        } else {
            let inputs = sources
                .sources
                .iter()
                .map(|source| PathBuf::from(&source.path))
                .collect::<Vec<_>>();
            let segments = plan
                .segments
                .iter()
                .map(|segment| {
                    let input_index = sources
                        .sources
                        .iter()
                        .position(|source| source.source_id == segment.source_id)
                        .ok_or_else(|| {
                            ProjectError::InvalidState(format!(
                                "cut segment {} references a missing source",
                                segment.id
                            ))
                        })?;
                    Ok(SourceRenderSegment {
                        input_index,
                        start_ms: segment.source_start_ms,
                        end_ms: segment.source_end_ms,
                    })
                })
                .collect::<Result<Vec<_>, ProjectError>>()?;
            render_source_segments(&inputs, &segments, &output)?;
        }
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
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path,
        count: transcript.words.len(),
    })
}

fn validate_variant(variant: &str) -> Result<(), ProjectError> {
    match variant {
        "tight" | "natural" => Ok(()),
        _ => Err(ProjectError::InvalidState(format!(
            "unknown edit variant {variant}; use tight or natural"
        ))),
    }
}

pub fn render_final(
    project_path: &Path,
    preset: &str,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let manifest = read_project_manifest(&project_path.join("project.json"))?;
    let output_preset = manifest
        .outputs
        .iter()
        .find(|candidate| candidate.id == preset)
        .ok_or_else(|| ProjectError::InvalidState(format!("unknown output preset {preset}")))?;
    let input = project_path.join("render/rough-cuts/natural.mp4");
    let captions = project_path.join("edit/captions.srt");
    let output = project_path.join(format!("render/finals/{preset}.mp4"));
    let reframe_anchors = if output_preset.aspect == "9:16" {
        Some(load_approved_reframe_anchors(project_path)?)
    } else {
        None
    };
    if !dry_run {
        if let Some(anchors) = reframe_anchors.as_deref() {
            render_preset_with_captions_and_reframe(
                &input,
                &captions,
                &output,
                output_preset.width,
                output_preset.height,
                true,
                Some(anchors),
            )?;
        } else {
            render_preset_with_captions(
                &input,
                &captions,
                &output,
                output_preset.width,
                output_preset.height,
                false,
            )?;
        }
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path: output,
        count: 1,
    })
}

fn load_approved_reframe_anchors(project_path: &Path) -> Result<Vec<ReframeAnchor>, ProjectError> {
    let path = project_path.join("analysis/reframe-plan.json");
    let plan: ReframePlan = read_json(&path).map_err(|_| {
        ProjectError::InvalidState(
            "vertical final rendering requires an approved `analysis/reframe-plan.json`".into(),
        )
    })?;
    if !plan.approved
        || plan.anchors.is_empty()
        || plan.anchors.iter().any(|anchor| !anchor.approved)
    {
        return Err(ProjectError::InvalidState(
            "vertical final rendering requires every reframe anchor to be explicitly approved"
                .into(),
        ));
    }
    let timeline: Timeline = read_json(&project_path.join("edit/timeline.json"))?;
    let segments = &timeline
        .tracks
        .first()
        .ok_or_else(|| ProjectError::InvalidState("timeline has no main track".into()))?
        .segments;
    if plan.anchors.len() != segments.len()
        || plan.anchors.iter().zip(segments).any(|(anchor, segment)| {
            anchor.source_id != segment.source_id
                || anchor.output_start_ms != segment.output_start_ms
                || anchor.output_end_ms != segment.output_end_ms
                || !(0.0..=1.0).contains(&anchor.center_x)
                || !(0.0..=1.0).contains(&anchor.center_y)
        })
    {
        return Err(ProjectError::InvalidState(
            "reframe anchors must exactly cover the output timeline with normalized centers".into(),
        ));
    }
    Ok(plan
        .anchors
        .into_iter()
        .map(|anchor| ReframeAnchor {
            output_start_ms: anchor.output_start_ms,
            center_x: anchor.center_x,
            center_y: anchor.center_y,
        })
        .collect())
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
    let benchmark = project_path.join("analysis/bench/transcribe/report.json");
    let manifest = read_project_manifest(&project_path.join("project.json"))?;
    let preset = manifest
        .outputs
        .iter()
        .find(|preset| preset.id == "youtube")
        .ok_or_else(|| {
            ProjectError::InvalidState("project is missing the youtube preset".into())
        })?;
    if !output.is_file() {
        return Err(ProjectError::InvalidState(format!(
            "QA requires an explicit final render: {}",
            output.display()
        )));
    }
    let benchmark_report: serde_json::Value = read_json(&benchmark).map_err(|_| {
        ProjectError::InvalidState(
            "QA requires a resolved `videoctl bench transcribe <project>` report".into(),
        )
    })?;
    if benchmark_report
        .get("decision")
        .and_then(serde_json::Value::as_str)
        == Some("unresolved")
    {
        return Err(ProjectError::InvalidState(
            "QA rejects an unresolved transcription benchmark".into(),
        ));
    }
    let media = &output;
    if !dry_run {
        let metadata = probe(media)?;
        if metadata.duration_ms.unwrap_or(0) <= 0 {
            return Err(ProjectError::InvalidState(
                "rendered output has no duration".into(),
            ));
        }
        if !metadata.has_video || !metadata.has_audio {
            return Err(ProjectError::InvalidState(
                "final output must contain both video and audio streams".into(),
            ));
        }
        if metadata.width != Some(preset.width) || metadata.height != Some(preset.height) {
            return Err(ProjectError::InvalidState(format!(
                "final output dimensions must be {}x{}",
                preset.width, preset.height
            )));
        }
        let captions = project_path.join("edit/captions.srt");
        let evidence = project_path.join("analysis/evidence/manifest.json");
        if !captions.is_file() || !evidence.is_file() {
            return Err(ProjectError::InvalidState(
                "QA requires generated captions and visual evidence".into(),
            ));
        }
        write_json_atomic(
            &project_path.join("qa/report.json"),
            &serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "status": "pass",
                "output": media,
                "duration_ms": metadata.duration_ms,
                "source_hashes": "unchanged",
                "checks": [{
                    "id": "final.explicit",
                    "status": "pass",
                    "evidence": media
                }, {
                    "id": "transcript.benchmark",
                    "status": "pass",
                    "evidence": benchmark
                }, {
                    "id": "media.duration",
                    "status": "pass",
                    "evidence": metadata.duration_ms
                }, {
                    "id": "media.streams",
                    "status": "pass",
                    "evidence": {"video": metadata.has_video, "audio": metadata.has_audio}
                }, {
                    "id": "media.dimensions",
                    "status": "pass",
                    "evidence": {"width": metadata.width, "height": metadata.height}
                }, {
                    "id": "captions.source_and_evidence",
                    "status": "pass",
                    "evidence": {"captions": captions, "evidence": evidence}
                }]
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
    let candidates: CandidateManifest = read_json(&project_path.join("edit/candidates.json"))?;
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    let path = project_path.join("analysis/evidence/manifest.json");
    if !dry_run {
        let boundary_dir = project_path.join("analysis/evidence/boundaries");
        let mut artifacts = Vec::new();
        for candidate in &candidates.candidates {
            let source = sources
                .sources
                .iter()
                .find(|source| source.source_id == candidate.source_id)
                .ok_or_else(|| {
                    ProjectError::InvalidState(format!(
                        "candidate {} references a missing source",
                        candidate.id
                    ))
                })?;
            let decision_start = candidate.start_ms.saturating_sub(750).max(0);
            let decision_end = candidate.end_ms.saturating_add(750);
            let mut frames = Vec::new();
            for (edge, timestamp_ms) in [
                ("before", decision_start),
                ("decision", candidate.start_ms),
                ("after", candidate.end_ms),
            ] {
                let frame = boundary_dir.join(format!("{}-{edge}.jpg", candidate.id));
                extract_frame(Path::new(&source.path), timestamp_ms, &frame)?;
                artifacts.push(serde_json::json!({
                    "kind": "boundary_frame",
                    "candidate_id": candidate.id,
                    "edge": edge,
                    "source_id": source.source_id,
                    "timestamp_ms": timestamp_ms,
                    "path": frame.strip_prefix(project_path).unwrap_or(&frame)
                }));
                frames.push(frame);
            }
            let waveform =
                project_path.join(format!("analysis/evidence/waveforms/{}.png", candidate.id));
            render_waveform_range(
                Path::new(&source.path),
                decision_start,
                decision_end,
                &waveform,
            )?;
            let composite =
                project_path.join(format!("analysis/evidence/filmstrips/{}.png", candidate.id));
            compose_decision_evidence(&frames, &waveform, &composite)?;
            artifacts.push(serde_json::json!({
                "kind": "decision_filmstrip",
                "candidate_id": candidate.id,
                "source_id": source.source_id,
                "start_ms": decision_start,
                "end_ms": decision_end,
                "path": composite.strip_prefix(project_path).unwrap_or(&composite)
            }));
        }
        let waveform = project_path.join("analysis/evidence/waveforms/natural.png");
        render_waveform(
            &project_path.join("render/rough-cuts/natural.mp4"),
            &waveform,
        )?;
        artifacts.push(serde_json::json!({"kind": "waveform", "path": waveform.strip_prefix(project_path).unwrap_or(&waveform)}));
        write_json_atomic(
            &path,
            &serde_json::json!({"schema_version": SCHEMA_VERSION, "artifacts": artifacts}),
        )?;
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
    let mut ranked = candidates
        .candidates
        .iter()
        .filter(|candidate| candidate.drop_reason.is_none())
        .cloned()
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        let left_score = (
            left.end_ms - left.start_ms,
            std::cmp::Reverse(left.take_rank),
        );
        let right_score = (
            right.end_ms - right.start_ms,
            std::cmp::Reverse(right.take_rank),
        );
        right_score.cmp(&left_score)
    });
    let mut source_ids = std::collections::HashSet::new();
    let mut selected = ranked
        .iter()
        .filter(|candidate| source_ids.insert(candidate.source_id.clone()))
        .take(count as usize)
        .cloned()
        .collect::<Vec<_>>();
    if selected.len() < count as usize {
        let selected_ids = selected
            .iter()
            .map(|candidate| candidate.id.clone())
            .collect::<std::collections::HashSet<_>>();
        let remaining = ranked
            .into_iter()
            .filter(|candidate| !selected_ids.contains(&candidate.id))
            .take(count as usize - selected.len())
            .collect::<Vec<_>>();
        selected.extend(remaining);
    }
    let path = project_path.join("edit/shorts.json");
    if !dry_run {
        write_json_atomic(
            &path,
            &serde_json::json!({
                "schema_version": SCHEMA_VERSION,
                "status": "proposed",
                "strategy": "duration_then_take_rank_with_source_diversity",
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
    let manifest = read_project_manifest(&project_path.join("project.json"))?;
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
                "slots": manifest.outputs.iter().map(|preset| serde_json::json!({
                    "id": format!("final-{}", preset.id),
                    "kind": "final_delivery",
                    "renderer": "render.final",
                    "effect_id": "delivery.render_final.v1",
                    "preset": preset.id,
                    "width": preset.width,
                    "height": preset.height,
                    "requires_reframe_approval": preset.aspect == "9:16",
                    "output_start_ms": 0,
                    "output_end_ms": timeline.tracks[0].segments.last().map(|segment| segment.output_end_ms).unwrap_or(0)
                })).collect::<Vec<_>>()
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
    let slot = finish
        .get("slots")
        .and_then(serde_json::Value::as_array)
        .and_then(|slots| {
            slots
                .iter()
                .find(|slot| slot.get("id").and_then(serde_json::Value::as_str) == Some(slot_id))
        })
        .ok_or_else(|| ProjectError::InvalidState(format!("unknown finish slot {slot_id}")))?;
    let renderer = slot
        .get("renderer")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            ProjectError::InvalidState(format!("finish slot {slot_id} has no renderer"))
        })?;
    match renderer {
        "render.final" => {
            let preset = slot
                .get("preset")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    ProjectError::InvalidState(format!("finish slot {slot_id} has no preset"))
                })?;
            render_final(project_path, preset, dry_run)
        }
        other => Err(ProjectError::InvalidState(format!(
            "finish slot {slot_id} has unsupported renderer {other}"
        ))),
    }
}

pub fn package_social(
    project_path: &Path,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let final_video = project_path.join("render/finals/youtube.mp4");
    let vertical_video = project_path.join("render/finals/reels.mp4");
    let caption_file = project_path.join("edit/captions.srt");
    let video_export = project_path.join("exports/youtube/youtube.mp4");
    let vertical_export = project_path.join("exports/vertical/reels.mp4");
    let caption_export = project_path.join("exports/captions/youtube.srt");
    let vertical_caption_export = project_path.join("exports/captions/reels.srt");
    if !dry_run {
        if !final_video.is_file() || !vertical_video.is_file() || !caption_file.is_file() {
            return Err(ProjectError::InvalidState(
                "social packaging requires YouTube, vertical, and caption artifacts".into(),
            ));
        }
        fs::copy(&final_video, &video_export)?;
        fs::copy(&vertical_video, &vertical_export)?;
        fs::copy(&caption_file, &caption_export)?;
        fs::copy(&caption_file, &vertical_caption_export)?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path: video_export,
        count: 4,
    })
}

pub fn export_otio(project_path: &Path, dry_run: bool) -> Result<PipelineArtifact, ProjectError> {
    let timeline: Timeline = read_json(&project_path.join("edit/timeline.json"))?;
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    let rate = timeline.timebase.fps_num as f64 / timeline.timebase.fps_den as f64;
    let children = timeline.tracks[0]
        .segments
        .iter()
        .map(|segment| {
            let source = sources
                .sources
                .iter()
                .find(|source| source.source_id == segment.source_id)
                .ok_or_else(|| ProjectError::InvalidState(format!("missing source {}", segment.source_id)))?;
            let source_duration = segment.source_end_ms - segment.source_start_ms;
            Ok(serde_json::json!({
                "OTIO_SCHEMA": "Clip.2",
                "name": segment.id,
                "media_reference": {
                    "OTIO_SCHEMA": "ExternalReference.1",
                    "target_url": format!("file://{}", source.path.replace(' ', "%20"))
                },
                "source_range": {
                    "OTIO_SCHEMA": "TimeRange.1",
                    "start_time": {"OTIO_SCHEMA": "RationalTime.1", "value": segment.source_start_ms as f64 * rate / 1000.0, "rate": rate},
                    "duration": {"OTIO_SCHEMA": "RationalTime.1", "value": source_duration as f64 * rate / 1000.0, "rate": rate}
                }
            }))
        })
        .collect::<Result<Vec<_>, ProjectError>>()?;
    let path = project_path.join("exports/interchange/timeline.otio.json");
    if !dry_run {
        write_json_atomic(
            &path,
            &serde_json::json!({
                "OTIO_SCHEMA": "Timeline.1",
                "name": "CutRight",
                "tracks": {
                    "OTIO_SCHEMA": "Stack.1",
                    "children": [{ "OTIO_SCHEMA": "Track.1", "kind": "Video", "children": children }]
                }
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
            .filter(|path| {
                path.extension().and_then(|ext| ext.to_str()) == Some("json")
                    && !path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.contains(".whisperx."))
            })
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
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
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

    fn sample_timeline() -> Timeline {
        Timeline {
            schema_version: SCHEMA_VERSION,
            timebase: Timebase {
                fps_num: 30,
                fps_den: 1,
            },
            tracks: vec![Track {
                id: "main".into(),
                track_type: "video".into(),
                segments: vec![TimelineSegment {
                    id: "segment-001".into(),
                    source_id: "source-001".into(),
                    source_start_ms: 1_000,
                    source_end_ms: 3_000,
                    output_start_ms: 0,
                    output_end_ms: 2_000,
                    speed: 1.0,
                    reason: "fixture".into(),
                }],
            }],
        }
    }

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
        assert!(temp.path().join("analysis/bench/transcribe").is_dir());
    }

    #[test]
    fn snapshot_of_manifest_only_project_has_no_completed_stages() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();

        let snapshot = project_snapshot(temp.path()).unwrap();

        assert!(snapshot.sources.is_empty());
        assert!(!snapshot.stages.ingested);
        assert!(!snapshot.stages.transcribed);
        assert!(!snapshot.stages.analyzed);
        assert!(!snapshot.stages.candidates);
        assert!(!snapshot.stages.rough_cut);
        assert!(!snapshot.stages.final_render);
        assert!(!snapshot.stages.qa);
        assert_eq!(
            snapshot.decisions_path,
            temp.path()
                .canonicalize()
                .unwrap()
                .join("feedback/decisions.jsonl")
        );
    }

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

    #[test]
    fn transcription_provenance_writes_raw_response_and_envelope() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        let source = SourceEntry {
            source_id: "cam-a-001".into(),
            path: "sources/cam-a-001.mov".into(),
            blake3: "source-hash".into(),
            duration_ms: Some(1_000),
            width: Some(1920),
            height: Some(1080),
            rotation_degrees: Some(0),
            is_hdr: Some(false),
            timebase: None,
        };
        let transcript_path = temp.path().join("analysis/transcripts/cam-a-001.json");
        write_json_atomic(
            &transcript_path,
            &Transcript {
                schema_version: SCHEMA_VERSION,
                provider: "heardright-parakeet-tdt".into(),
                source_id: source.source_id.clone(),
                language: "en".into(),
                words: Vec::new(),
                events: Vec::new(),
            },
        )
        .unwrap();
        let raw = serde_json::json!({ "engine": "heardright", "words": [{ "text": "hello" }] });
        write_transcription_provenance(
            temp.path(),
            &source,
            &transcript_path,
            TranscriptionProvenance {
                raw_response: &raw,
                provider: "heardright-parakeet-tdt",
                provider_model: "parakeet-tdt-v3-coreml",
                warnings: &["fixture warning".into()],
                provider_label: "heardright",
            },
        )
        .unwrap();
        let stored_raw: serde_json::Value = read_json(
            &temp
                .path()
                .join("cache/provider-responses/cam-a-001.heardright.raw.json"),
        )
        .unwrap();
        assert_eq!(stored_raw, raw);
        let envelope: ProviderResponseEnvelope = read_json(
            &temp
                .path()
                .join("analysis/transcripts/cam-a-001.heardright.envelope.json"),
        )
        .unwrap();
        assert_eq!(envelope.provider, "heardright-parakeet-tdt");
        assert_eq!(envelope.provider_model, "parakeet-tdt-v3-coreml");
        assert_eq!(
            envelope.raw_response_path,
            "cache/provider-responses/cam-a-001.heardright.raw.json"
        );
        assert_eq!(
            envelope.normalised_output_path,
            "analysis/transcripts/cam-a-001.json"
        );
        assert!(!envelope.request_hash.is_empty());
        assert_eq!(envelope.warnings, vec!["fixture warning"]);
        let cached = load_cached_transcription(
            temp.path(),
            &source,
            &transcript_path,
            "heardright-parakeet-tdt",
            "parakeet-tdt-v3-coreml",
            "heardright",
        )
        .unwrap()
        .expect("matching provenance should be reusable");
        assert_eq!(cached.source_id, source.source_id);
    }

    #[test]
    fn finish_validation_creates_one_delivery_slot_per_preset() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        write_json_atomic(&temp.path().join("edit/timeline.json"), &sample_timeline()).unwrap();

        let result = finish_validate(temp.path(), false).unwrap();
        assert_eq!(result.count, 1);
        let plan: video_core::FinishPlan = read_json(&result.path).unwrap();
        assert_eq!(plan.slots.len(), 3);
        assert_eq!(plan.slots[0].id, "final-youtube");
        assert_eq!(plan.slots[0].renderer, "render.final");
        assert_eq!(plan.slots[0].effect_id, "delivery.render_final.v1");
    }

    #[test]
    fn otio_export_has_standard_timeline_clip_and_media_reference_schemas() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();
        write_json_atomic(&temp.path().join("edit/timeline.json"), &sample_timeline()).unwrap();
        write_json_atomic(
            &temp.path().join("sources/manifest.json"),
            &SourceManifest {
                schema_version: SCHEMA_VERSION,
                sources: vec![SourceEntry {
                    source_id: "source-001".into(),
                    path: "/captures/cam one.mov".into(),
                    blake3: "fixture".into(),
                    duration_ms: Some(10_000),
                    width: Some(1920),
                    height: Some(1080),
                    rotation_degrees: Some(0),
                    is_hdr: Some(false),
                    timebase: Some(Timebase {
                        fps_num: 30,
                        fps_den: 1,
                    }),
                }],
            },
        )
        .unwrap();

        let result = export_otio(temp.path(), false).unwrap();
        let otio: serde_json::Value = read_json(&result.path).unwrap();
        assert_eq!(otio["OTIO_SCHEMA"], "Timeline.1");
        let clip = &otio["tracks"]["children"][0]["children"][0];
        assert_eq!(clip["OTIO_SCHEMA"], "Clip.2");
        assert_eq!(
            clip["media_reference"]["OTIO_SCHEMA"],
            "ExternalReference.1"
        );
        assert_eq!(clip["source_range"]["OTIO_SCHEMA"], "TimeRange.1");
        assert_eq!(
            clip["source_range"]["start_time"]["OTIO_SCHEMA"],
            "RationalTime.1"
        );
        assert_eq!(
            clip["media_reference"]["target_url"],
            "file:///captures/cam%20one.mov"
        );
    }

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
    fn vad_expands_candidate_bounds_to_enclosing_speech() {
        let signal = VadSignal {
            schema_version: SCHEMA_VERSION,
            source_id: "source-a".into(),
            sample_rate: 16_000,
            provider: "silero-coreml".into(),
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
    fn benchmark_sampling_spans_the_full_clip() {
        let boundaries = (0..100).collect::<Vec<_>>();
        let sampled = evenly_spaced(&boundaries, 5)
            .into_iter()
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(sampled, vec![0, 24, 49, 74, 99]);
    }

    #[test]
    fn transcription_benchmark_requires_a_decisive_provider() {
        assert_eq!(
            benchmark_decision("heardright", "whisperx", true, false),
            "heardright"
        );
        assert_eq!(
            benchmark_decision("heardright", "whisperx", false, true),
            "whisperx"
        );
        assert_eq!(
            benchmark_decision("heardright", "whisperx", true, true),
            "unresolved"
        );
        assert_eq!(
            benchmark_decision("heardright", "whisperx", false, false),
            "unresolved"
        );
    }
}
