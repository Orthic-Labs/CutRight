use crate::io::*;
use crate::receipts;
use crate::PipelineArtifact;
use crate::ProjectError;
use chrono::Utc;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use video_core::{
    models::{ProviderCost, ProviderResponseEnvelope, SourceEntry, SCHEMA_VERSION},
    providers::{TranscriptionProvider, TranscriptionRequest},
    SourceManifest, Transcript,
};
use video_providers::{HeardRightProvider, WhisperXProvider};

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
                output.transcript.validate().map_err(|error| {
                    ProjectError::InvalidState(format!(
                        "provider {} returned an invalid transcript for {}: {error}",
                        provider.id(),
                        source.source_id
                    ))
                })?;
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
                let mut toolchains = BTreeMap::new();
                toolchains.insert(provider.id().to_string(), output.provider_model.clone());
                receipts::write_stage_receipt(
                    &receipts::receipt_path_for(&transcript_path),
                    "transcribe",
                    &[Path::new(&source.path)],
                    &serde_json::json!({
                        "provider": provider.id(),
                        "provider_model": output.provider_model,
                        "language_hint": "en",
                    }),
                    toolchains,
                    &[transcript_path.as_path()],
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
        write_project_transcript_pack(project_path, &sources.sources, &transcripts)?;
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

/// Project-level transcript pack (§15.1): one markdown document covering
/// EVERY registered source under its own heading, not just the first. The
/// previous version silently dropped every source after the first, so an
/// agent reading only this file could receive an incomplete multi-source
/// transcript without any signal that it was incomplete. The trailing
/// content hash lets a reader confirm which exact pack they have.
fn write_project_transcript_pack(
    project_path: &Path,
    sources: &[SourceEntry],
    transcripts: &[Transcript],
) -> Result<(), ProjectError> {
    let mut body = String::new();
    body.push_str("# Transcript pack\n\n");
    body.push_str(&format!(
        "sources: {} · generated: {}\n\n",
        sources.len(),
        Utc::now().to_rfc3339()
    ));
    for source in sources {
        let transcript = transcripts
            .iter()
            .find(|transcript| transcript.source_id == source.source_id);
        body.push_str(&format!(
            "## Source `{}` — {}\n\n",
            source.source_id, source.path
        ));
        match transcript {
            Some(transcript) if !transcript.words.is_empty() => {
                for word in &transcript.words {
                    body.push_str(&format!(
                        "- `{}` `{}–{}` {}\n",
                        word.id, word.start_ms, word.end_ms, word.text
                    ));
                }
            }
            Some(_) => body.push_str("_(transcribed; no words)_\n"),
            None => body.push_str("_(not yet transcribed)_\n"),
        }
        body.push('\n');
    }
    let content_hash = blake3::hash(body.as_bytes()).to_hex().to_string();
    body.push_str(&format!("---\ncontent_hash: blake3:{content_hash}\n"));
    write_bytes_atomic(
        &project_path.join("analysis/transcript-packed.md"),
        body.as_bytes(),
    )
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

/// Cache identity for one transcription request (REV2 plan §10.5).
///
/// Deliberately content-addressed: the source's absolute path is NOT part of
/// the identity, because moving a project or relinking a source does not
/// change a single byte of the audio being transcribed. Path stays available
/// as provenance in the envelope; only content and policy decide reuse:
/// source content, provider/model identity, decode and language policy, and
/// the stage implementation version (so a change to how CutRight builds the
/// request invalidates old entries).
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
        "decode_policy": "pcm_f32le/16000/mono",
        "language_hint": "en",
        "stage_implementation_version": env!("CARGO_PKG_VERSION"),
    });
    Ok(blake3::hash(&serde_json::to_vec(&request)?)
        .to_hex()
        .to_string())
}

/// True when `name` names the HeardRight engine (any accepted spelling).
pub(crate) fn is_heardright_provider(name: &str) -> bool {
    matches!(name, "heardright" | "heardright-parakeet-tdt")
}

/// True when `name` names the WhisperX verifier (any accepted spelling).
pub(crate) fn is_whisperx_provider(name: &str) -> bool {
    matches!(name, "whisperx" | "whisperx-alignment")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_project;

    #[test]
    fn transcription_cache_identity_survives_a_moved_source() {
        // §10.5: cache identity is content-addressed. A project copied to
        // another disk, or a source relinked to a new path, must reuse valid
        // analysis instead of paying for a full re-transcription.
        let entry = |path: &str| SourceEntry {
            source_id: "cam-a-001".into(),
            path: path.into(),
            blake3: "blake3:same-bytes".into(),
            duration_ms: Some(1_000),
            width: None,
            height: None,
            rotation_degrees: None,
            is_hdr: None,
            timebase: None,
        };
        let here =
            transcription_request_hash(&entry("/Volumes/A/clip.mov"), "heardright", "tdt-v3")
                .unwrap();
        let moved = transcription_request_hash(
            &entry("/Volumes/B/archive/clip.mov"),
            "heardright",
            "tdt-v3",
        )
        .unwrap();
        assert_eq!(here, moved);

        // Different BYTES must still miss the cache.
        let mut other = entry("/Volumes/A/clip.mov");
        other.blake3 = "blake3:different-bytes".into();
        let different = transcription_request_hash(&other, "heardright", "tdt-v3").unwrap();
        assert_ne!(here, different);

        // So must a different engine.
        let other_engine =
            transcription_request_hash(&entry("/Volumes/A/clip.mov"), "whisperx", "tdt-v3")
                .unwrap();
        assert_ne!(here, other_engine);
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
}
