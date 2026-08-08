//! Vertical-delivery reframe planning (REV2 plan §15.5, Phase 7 — temporal
//! visual perception and reframing).
//!
//! `reframe_plan` used to sample exactly one Vision face box at each
//! timeline segment's midpoint and use it, unmodified, as that whole
//! segment's crop center. This module now samples a bounded, resumable
//! series of frames across each segment, runs every frame through the
//! embedded Vision worker (faces, bodies, OCR text boxes, saliency in one
//! call), adds active-speaker evidence from the already-aligned word-level
//! transcript, and hands the resulting samples to
//! [`crate::reframe_track::build_temporal_track`] — the pure fusion/
//! smoothing/safe-zone core — to produce a full temporal track. The full
//! track is written to `analysis/reframe/<variant>/reframe-track.json`; the
//! existing `analysis/reframe/<variant>/reframe-plan.json` keeps its
//! original per-segment `anchors[]` shape (required by
//! `final_render.rs::load_approved_reframe_anchors`, which this module does
//! not own) but each anchor is now the confidence-weighted collapse of that
//! segment's slice of the temporal track instead of one single-point Vision
//! call.

use crate::io::*;
use crate::receipts;
use crate::reframe_track::{
    build_temporal_track, collapse_segment_anchor, Modality, ModalityObservation, OcrBox,
    SmoothingParams, TrackSample,
};
use crate::PipelineArtifact;
use crate::ProjectError;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};
use video_core::process_runner::{ManagedChild, ProcessSpec};
use video_core::{models::SCHEMA_VERSION, SourceManifest, Timeline, TimelineSegment, Transcript};
use video_media::extract_frame;
use video_media::native::{
    AnalyzeFramesRequest, MacMediaBackend, MacMediaWorker, MacNativeMode, NativeFrameAnalysis,
    NativeFrameRequest, NativeRationalTime, NativeRequestContext,
};

/// Target spacing between samples within one segment. This is a target, not
/// a guarantee: [`effective_sample_interval_ms`] widens it as needed so the
/// whole plan never exceeds [`MAX_TOTAL_SAMPLES`], keeping a long source's
/// Vision-worker cost bounded rather than growing without limit. The chosen
/// interval for a given run is always recorded in the emitted track/plan
/// artifacts and the stage receipt, per REV2 plan §15.5.
const DEFAULT_SAMPLE_INTERVAL_MS: i64 = 500;
/// Floor under [`effective_sample_interval_ms`]'s widening so a pathological
/// (near-zero) total duration can never produce a zero or negative interval.
const MIN_SAMPLE_INTERVAL_MS: i64 = 50;
/// Hard ceiling on samples for one `reframe_plan` run, across every segment
/// combined. A two-hour source at the default 500ms spacing would be 14,400
/// samples; this caps the actual worst case at this many Vision-worker
/// invocations (each individually timeout-bounded — see
/// [`VISION_WORKER_TIMEOUT`]) regardless of source length.
const MAX_TOTAL_SAMPLES: i64 = 3_000;

/// Per-invocation bound on the embedded Vision worker (§10.1: no command
/// spawned by this crate may wait indefinitely). A single-frame multi-request
/// Vision call is normally sub-second; this is generous headroom for a slow
/// disk or cold model load, not an expected steady-state duration.
const VISION_WORKER_TIMEOUT: Duration = Duration::from_secs(30);
const VISION_STDOUT_CAP_BYTES: usize = 2 * 1024 * 1024;
const VISION_STDERR_CAP_BYTES: usize = 1024 * 1024;

/// Half-extents (normalized source-frame units) of a representative 9:16 crop
/// window, used only to score the safe-zone cost function's OCR-avoidance
/// nudge. This is deliberately an approximation, not the exact render-time
/// crop geometry — that geometry is computed once, precisely, from real
/// source/output dimensions by `video_media::reframe_filter` at render time.
/// Getting this constant slightly wrong only makes the OCR nudge slightly
/// more/less eager; it never affects what actually gets rendered.
const APPROX_CROP_HALF_WIDTH: f64 = 0.16;
const APPROX_CROP_HALF_HEIGHT: f64 = 0.16;

#[derive(Debug, Deserialize)]
struct VisionDetectionBox {
    center_x: f64,
    center_y: f64,
    area: f64,
    confidence: f64,
}

#[derive(Debug, Deserialize)]
struct VisionOcrBox {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    #[allow(dead_code)] // recorded on the wire; not yet used to weight boxes.
    confidence: f64,
}

#[derive(Debug, Deserialize)]
struct VisionFrameResponse {
    faces: Vec<VisionDetectionBox>,
    bodies: Vec<VisionDetectionBox>,
    ocr_boxes: Vec<VisionOcrBox>,
    saliency: Option<VisionDetectionBox>,
}

#[derive(Debug, Clone)]
struct ReframeFrameSample {
    output_ms: i64,
    source_ms: i64,
    offset: i64,
    frame: PathBuf,
    source_frame_index: i32,
    sequence_id: String,
}

fn parse_native_vision_mode(value: Option<&str>) -> Result<MacNativeMode, ProjectError> {
    match value.unwrap_or("legacy") {
        "legacy" => Ok(MacNativeMode::Legacy),
        "shadow" => Ok(MacNativeMode::Shadow),
        "native" => Ok(MacNativeMode::Native),
        value => Err(ProjectError::InvalidState(format!(
            "CUTRIGHT_MACOS_NATIVE must be legacy, shadow, or native; got {value}"
        ))),
    }
}

fn native_vision_mode_from_env() -> Result<MacNativeMode, ProjectError> {
    parse_native_vision_mode(std::env::var("CUTRIGHT_MACOS_NATIVE").ok().as_deref())
}

fn native_vision_response(analysis: NativeFrameAnalysis) -> VisionFrameResponse {
    VisionFrameResponse {
        faces: analysis
            .faces
            .into_iter()
            .map(|box_| VisionDetectionBox {
                center_x: box_.center_x,
                center_y: box_.center_y,
                area: box_.area,
                confidence: box_.confidence,
            })
            .collect(),
        bodies: analysis
            .bodies
            .into_iter()
            .map(|box_| VisionDetectionBox {
                center_x: box_.center_x,
                center_y: box_.center_y,
                area: box_.area,
                confidence: box_.confidence,
            })
            .collect(),
        ocr_boxes: analysis
            .ocr_boxes
            .into_iter()
            .map(|box_| VisionOcrBox {
                x0: box_.x0,
                y0: box_.y0,
                x1: box_.x1,
                y1: box_.y1,
                confidence: box_.confidence,
            })
            .collect(),
        saliency: analysis.saliency.map(|box_| VisionDetectionBox {
            center_x: box_.center_x,
            center_y: box_.center_y,
            area: box_.area,
            confidence: box_.confidence,
        }),
    }
}

fn vision_discrepancy(
    legacy: &VisionFrameResponse,
    native: &VisionFrameResponse,
) -> serde_json::Value {
    serde_json::json!({
        "legacy": { "faces": legacy.faces.len(), "bodies": legacy.bodies.len(), "ocr_boxes": legacy.ocr_boxes.len(), "saliency": legacy.saliency.is_some() },
        "native": { "faces": native.faces.len(), "bodies": native.bodies.len(), "ocr_boxes": native.ocr_boxes.len(), "saliency": native.saliency.is_some() },
        "equal_counts": legacy.faces.len() == native.faces.len() && legacy.bodies.len() == native.bodies.len() && legacy.ocr_boxes.len() == native.ocr_boxes.len() && legacy.saliency.is_some() == native.saliency.is_some(),
    })
}

/// A human-authored override for an exact output-time range, optionally
/// dropped at `analysis/reframe/<variant>/manual-anchors.json`. When present
/// it silently takes precedence over every detector for any sample whose
/// `output_ms` falls in `[output_start_ms, output_end_ms)` — the human
/// overrides, the tracker never second-guesses it (REV2 plan §15.5: "manual
/// anchors where confidence is low").
#[derive(Debug, Clone, Deserialize)]
struct ManualAnchor {
    output_start_ms: i64,
    output_end_ms: i64,
    center_x: f64,
    center_y: f64,
}

pub fn reframe_plan(
    project_path: &Path,
    variant: Option<&str>,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let variant = resolve_variant(project_path, variant)?;
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    if sources.sources.is_empty() {
        return Err(ProjectError::NoSources);
    }
    let timeline_path = variant_timeline_path(project_path, &variant);
    require_variant_artifact(project_path, &timeline_path, &variant, "reframe.plan").map_err(
        |_| {
            ProjectError::InvalidState(format!(
                "reframe planning requires edit/timeline-{variant}.json; run `videoctl edit render <project> --variant {variant}` first"
            ))
        },
    )?;
    let timeline: Timeline = read_json(&timeline_path)?;
    let timeline_segments = &timeline
        .tracks
        .first()
        .ok_or_else(|| ProjectError::InvalidState("timeline has no main track".into()))?
        .segments;
    let plan_path = variant_reframe_path(project_path, &variant);
    let track_path = project_path.join(format!("analysis/reframe/{variant}/reframe-track.json"));
    if !dry_run {
        let native_mode = native_vision_mode_from_env()?;
        let worker = if native_mode == MacNativeMode::Native {
            None
        } else {
            Some(vision_anchor_worker()?)
        };
        let mut shadow_discrepancies = Vec::new();
        let native_worker = if native_mode == MacNativeMode::Legacy {
            None
        } else {
            match MacMediaWorker::new() {
                Ok(worker) => Some(worker),
                Err(error) if native_mode == MacNativeMode::Shadow => {
                    shadow_discrepancies.push(serde_json::json!({
                        "native_error": { "kind": "unsupported", "message": error.to_string() },
                        "returned_backend": "legacy",
                    }));
                    None
                }
                Err(error) => {
                    return Err(ProjectError::InvalidState(format!(
                        "native vision unsupported: {error}"
                    )))
                }
            }
        };
        let sample_interval_ms = effective_sample_interval_ms(timeline_segments);
        let transcripts = load_transcripts(project_path).unwrap_or_default();
        let manual_anchors: Vec<ManualAnchor> = read_json_if_file(
            &project_path.join(format!("analysis/reframe/{variant}/manual-anchors.json")),
        )
        .unwrap_or_default();

        let mut speaker_positions: BTreeMap<String, (f64, f64)> = BTreeMap::new();
        let mut samples: Vec<TrackSample> = Vec::new();
        let mut segment_windows: Vec<(i64, i64)> = Vec::with_capacity(timeline_segments.len());

        for segment in timeline_segments {
            segment_windows.push((segment.output_start_ms, segment.output_end_ms));
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
            let mut frame_samples = Vec::new();
            for offset in sample_offsets_for_segment(segment, sample_interval_ms) {
                let output_ms = segment.output_start_ms + offset;
                let source_ms = segment.source_start_ms
                    + (offset as f64 * segment.speed.max(0.0)).round() as i64;
                let source_ms = source_ms.clamp(segment.source_start_ms, segment.source_end_ms);

                let frame =
                    project_path.join(format!("cache/frames/reframe-{}-{offset}.jpg", segment.id));
                extract_frame(Path::new(&source.path), source_ms, &frame)?;
                frame_samples.push(ReframeFrameSample {
                    output_ms,
                    source_ms,
                    offset,
                    frame,
                    source_frame_index: samples.len() as i32 + frame_samples.len() as i32,
                    sequence_id: format!("{}:{}", segment.source_id, segment.id),
                });
            }

            let legacy_vision = match worker.as_ref() {
                Some(worker) => frame_samples
                    .iter()
                    .map(|sample| run_vision_worker(worker, &sample.frame))
                    .collect::<Result<Vec<_>, _>>()?,
                None => Vec::new(),
            };
            let native_vision = match native_worker.as_ref() {
                Some(worker) => {
                    let request = AnalyzeFramesRequest {
                        frames: frame_samples
                            .iter()
                            .map(|sample| NativeFrameRequest {
                                source_path: sample.frame.clone(),
                                source_frame_index: sample.source_frame_index,
                                timestamp: NativeRationalTime {
                                    numerator: sample.source_ms,
                                    denominator: 1_000,
                                },
                                sequence_id: Some(sample.sequence_id.clone()),
                                orientation: Some("up".into()),
                            })
                            .collect(),
                        allowed_roots: vec![project_path.join("cache/frames")],
                    };
                    let context = NativeRequestContext {
                        request_id: format!("reframe:{}:{}", variant, segment.id),
                        timeout: VISION_WORKER_TIMEOUT,
                    };
                    match worker.analyze_frames(&context, &request) {
                        Ok(observations) if observations.len() == frame_samples.len() => {
                            observations
                                .into_iter()
                                .map(native_vision_response)
                                .map(Some)
                                .collect::<Vec<_>>()
                        }
                        Ok(observations) => {
                            return Err(ProjectError::InvalidState(format!(
                                "native vision returned {} observations for {} requested frames",
                                observations.len(),
                                frame_samples.len()
                            )))
                        }
                        Err(error) if native_mode == MacNativeMode::Shadow => {
                            shadow_discrepancies.push(serde_json::json!({
                                "segment_id": segment.id, "source_id": segment.source_id,
                                "native_error": { "kind": "unsupported_or_failed", "message": error.to_string() },
                                "returned_backend": "legacy",
                            }));
                            (0..frame_samples.len()).map(|_| None).collect()
                        }
                        Err(error) => {
                            return Err(ProjectError::InvalidState(format!(
                                "native vision unsupported: {error}"
                            )))
                        }
                    }
                }
                None if native_mode == MacNativeMode::Shadow => {
                    (0..frame_samples.len()).map(|_| None).collect()
                }
                None => Vec::new(),
            };
            for (index, sample) in frame_samples.iter().enumerate() {
                let vision = match native_mode {
                    MacNativeMode::Legacy => &legacy_vision[index],
                    MacNativeMode::Native => native_vision[index].as_ref().ok_or_else(|| {
                        ProjectError::InvalidState("native vision returned no observation".into())
                    })?,
                    MacNativeMode::Shadow => {
                        if let Some(native) = native_vision[index].as_ref() {
                            shadow_discrepancies.push(serde_json::json!({
                                "segment_id": segment.id, "source_id": segment.source_id,
                                "output_ms": sample.output_ms, "source_ms": sample.source_ms,
                                "frame": relative_artifact_path(project_path, &sample.frame),
                                "comparison": vision_discrepancy(&legacy_vision[index], native),
                            }));
                        }
                        &legacy_vision[index]
                    }
                };

                let mut observations = Vec::new();
                for face in &vision.faces {
                    observations.push(ModalityObservation {
                        modality: Modality::Face,
                        center_x: face.center_x,
                        center_y: face.center_y,
                        confidence: face.confidence,
                        extent: face.area,
                    });
                }
                for body in &vision.bodies {
                    observations.push(ModalityObservation {
                        modality: Modality::BodyHands,
                        center_x: body.center_x,
                        center_y: body.center_y,
                        confidence: body.confidence,
                        extent: body.area,
                    });
                }
                if let Some(saliency) = &vision.saliency {
                    observations.push(ModalityObservation {
                        modality: Modality::Saliency,
                        center_x: saliency.center_x,
                        center_y: saliency.center_y,
                        confidence: saliency.confidence,
                        extent: saliency.area,
                    });
                }
                if let Some(active_speaker) = active_speaker_observation(
                    &transcripts,
                    &segment.source_id,
                    sample.source_ms,
                    &vision.faces,
                    &mut speaker_positions,
                ) {
                    observations.push(active_speaker);
                }

                let ocr_boxes = vision
                    .ocr_boxes
                    .iter()
                    .map(|ocr| OcrBox {
                        x0: ocr.x0,
                        y0: ocr.y0,
                        x1: ocr.x1,
                        y1: ocr.y1,
                    })
                    .collect();

                let manual_anchor = manual_anchors
                    .iter()
                    .find(|anchor| {
                        sample.output_ms >= anchor.output_start_ms
                            && sample.output_ms < anchor.output_end_ms
                    })
                    .map(|anchor| (anchor.center_x, anchor.center_y));

                samples.push(TrackSample {
                    output_ms: sample.output_ms,
                    source_id: segment.source_id.clone(),
                    source_frame_index: Some(sample.source_frame_index as u64),
                    sequence_id: Some(sample.sequence_id.clone()),
                    shot_boundary: sample.offset == 0,
                    observations,
                    ocr_boxes,
                    manual_anchor,
                });
            }
        }

        let shadow_path = project_path.join(format!(
            "analysis/reframe/{variant}/native-vision-shadow.json"
        ));
        let shadow_discrepancy_count = shadow_discrepancies.len();
        if native_mode == MacNativeMode::Shadow {
            write_json_atomic(
                &shadow_path,
                &serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "backend": "shadow",
                    "returned_backend": "legacy",
                    "discrepancies": shadow_discrepancies,
                }),
            )?;
        }

        let track = build_temporal_track(
            &samples,
            &SmoothingParams::default(),
            APPROX_CROP_HALF_WIDTH,
            APPROX_CROP_HALF_HEIGHT,
        );

        let track_artifact = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "variant": variant,
            "sample_interval_ms": sample_interval_ms,
            "points": track,
        });
        write_json_atomic(&track_path, &track_artifact)?;

        let anchors: Vec<serde_json::Value> = timeline_segments
            .iter()
            .zip(&segment_windows)
            .map(|(segment, (output_start_ms, output_end_ms))| {
                let anchor = collapse_segment_anchor(&track, *output_start_ms, *output_end_ms);
                serde_json::json!({
                    "source_id": segment.source_id,
                    "output_start_ms": output_start_ms,
                    "output_end_ms": output_end_ms,
                    "center_x": anchor.center_x,
                    "center_y": anchor.center_y,
                    "strategy": anchor.source,
                    "confidence": anchor.confidence,
                    "gap": anchor.gap,
                    "approved": false
                })
            })
            .collect();

        let plan = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "kind": "timeline_reframe_plan",
            "variant": variant,
            "target_aspect": "9:16",
            "approved": false,
            "requires_review": true,
            "sample_interval_ms": sample_interval_ms,
            "track_path": relative_artifact_path(project_path, &track_path),
            "anchors": anchors
        });
        // §6.1: `plan_path` (the variant-scoped location) is the ONLY
        // authority. A generic `analysis/reframe-plan.json` alias used to be
        // written here on every call regardless of variant — the exact
        // defect this fix removes, since it meant the same generic file the
        // read side could silently fall back to held whichever variant last
        // ran reframe planning, not necessarily the one being read.
        write_json_atomic(&plan_path, &plan)?;

        let mut toolchains = BTreeMap::new();
        if let Some(worker) = worker.as_ref() {
            if let Ok(worker_hash) = hash_file(worker) {
                toolchains.insert("vision_anchor_worker".to_string(), worker_hash);
            }
        }
        let mut receipt_outputs = vec![plan_path.as_path(), track_path.as_path()];
        if native_mode == MacNativeMode::Shadow {
            receipt_outputs.push(shadow_path.as_path());
        }
        receipts::write_stage_receipt(
            &receipts::receipt_path_for(&plan_path),
            "reframe.plan",
            &[timeline_path.as_path()],
            &serde_json::json!({
                "variant": variant,
                "target_aspect": "9:16",
                "sample_interval_ms": sample_interval_ms,
                "native_mode": format!("{:?}", native_mode).to_lowercase(),
                "shadow_discrepancy_count": shadow_discrepancy_count,
            }),
            toolchains,
            &receipt_outputs,
        )?;
    }
    Ok(PipelineArtifact {
        status: if dry_run { "dry-run" } else { "created" },
        path: plan_path,
        count: timeline_segments.len(),
    })
}

/// Widen [`DEFAULT_SAMPLE_INTERVAL_MS`] as needed so the total sample count
/// across every segment never exceeds [`MAX_TOTAL_SAMPLES`]. Pure and cheap
/// so a caller (or a test) can predict the exact interval a given timeline
/// will use before running the (expensive, Vision-worker-driven) plan.
fn effective_sample_interval_ms(segments: &[TimelineSegment]) -> i64 {
    let total_output_ms: i64 = segments
        .iter()
        .map(|segment| (segment.output_end_ms - segment.output_start_ms).max(0))
        .sum();
    if total_output_ms <= 0 {
        return DEFAULT_SAMPLE_INTERVAL_MS;
    }
    // `sample_offsets_for_segment` always includes offset `0` (that
    // segment's shot-boundary sample) and, whenever the segment doesn't
    // divide evenly by the interval, one extra tail sample so its span is
    // never left with an uncovered gap at the end — up to two samples per
    // segment beyond the raw `duration / interval` count. Reserving that
    // per-segment headroom out of the total budget before dividing is what
    // keeps the *actual* total sample count (summed across every segment,
    // including their individual rounding) at or under
    // `MAX_TOTAL_SAMPLES`, not just the naive `total / MAX_TOTAL_SAMPLES`
    // estimate, which undercounts by up to two samples per segment.
    let reserved = (segments.len() as i64 * 2).min(MAX_TOTAL_SAMPLES - 1);
    let budget = (MAX_TOTAL_SAMPLES - reserved).max(1);
    let implied = ceil_div(total_output_ms, budget);
    implied
        .max(DEFAULT_SAMPLE_INTERVAL_MS)
        .max(MIN_SAMPLE_INTERVAL_MS)
}

/// Ceiling division for strictly positive `numerator`/`denominator`.
fn ceil_div(numerator: i64, denominator: i64) -> i64 {
    (numerator + denominator - 1) / denominator
}

/// Sample offsets (milliseconds from the segment's own start) at which to
/// sample this segment, given `interval_ms`. Always includes `0` (so every
/// segment gets at least one, shot-boundary-flagged sample even if shorter
/// than `interval_ms`) and always includes the segment's own final offset,
/// so a segment's temporal coverage never has a trailing gap at its tail.
fn sample_offsets_for_segment(segment: &TimelineSegment, interval_ms: i64) -> Vec<i64> {
    let duration = (segment.output_end_ms - segment.output_start_ms).max(0);
    if duration == 0 {
        return vec![0];
    }
    let mut offsets: Vec<i64> = (0..duration).step_by(interval_ms.max(1) as usize).collect();
    if offsets.last().copied() != Some(duration) {
        offsets.push(duration.saturating_sub(1).max(0));
    }
    offsets.sort_unstable();
    offsets.dedup();
    offsets
}

/// Derive an active-speaker [`ModalityObservation`] for this sample, or
/// `None` if no transcript word is speaking at `source_ms`.
///
/// This pipeline does not have a per-face voice-print or lip-sync model —
/// that would be a genuinely separate detector, out of scope for this phase
/// — so "which face is the labeled speaker" is resolved by a small,
/// deterministic continuity heuristic: the face nearest the speaker's own
/// last known position wins, falling back to the single face present (when
/// exactly one is on screen) or the most confident face (when several are
/// present and none has been associated with this speaker yet). `positions`
/// is updated in place so the heuristic has real continuity across samples
/// within a segment and across segments featuring the same recurring
/// speaker label.
fn active_speaker_observation(
    transcripts: &[Transcript],
    source_id: &str,
    source_ms: i64,
    faces: &[VisionDetectionBox],
    positions: &mut BTreeMap<String, (f64, f64)>,
) -> Option<ModalityObservation> {
    let transcript = transcripts
        .iter()
        .find(|transcript| transcript.source_id == source_id)?;
    let word = transcript
        .words
        .iter()
        .find(|word| word.start_ms <= source_ms && source_ms < word.end_ms)?;
    let speaker = word.speaker.as_ref()?;

    let matched_face = if let Some(&last) = positions.get(speaker) {
        faces
            .iter()
            .min_by(|a, b| distance_to(a, last).total_cmp(&distance_to(b, last)))
    } else if faces.len() == 1 {
        faces.first()
    } else {
        faces
            .iter()
            .max_by(|a, b| a.confidence.total_cmp(&b.confidence))
    };

    match matched_face {
        Some(face) => {
            positions.insert(speaker.clone(), (face.center_x, face.center_y));
            Some(ModalityObservation {
                modality: Modality::ActiveSpeaker,
                center_x: face.center_x,
                center_y: face.center_y,
                confidence: word.confidence.max(face.confidence as f32) as f64,
                extent: face.area,
            })
        }
        None => {
            // Nobody is visibly on camera this sample, but the transcript
            // says this speaker is talking. Hold their last known position
            // at low confidence rather than dropping the evidence entirely
            // — this still lets `build_temporal_track`'s gap handling treat
            // an interval with real audio evidence differently from one
            // with none at all.
            let last = positions.get(speaker).copied()?;
            Some(ModalityObservation {
                modality: Modality::ActiveSpeaker,
                center_x: last.0,
                center_y: last.1,
                confidence: (word.confidence as f64 * 0.3).clamp(0.0, 1.0),
                extent: 1.0,
            })
        }
    }
}

fn distance_to(face: &VisionDetectionBox, point: (f64, f64)) -> f64 {
    let dx = face.center_x - point.0;
    let dy = face.center_y - point.1;
    (dx * dx + dy * dy).sqrt()
}

/// Materialize the embedded Vision anchor worker by CONTENT HASH (§10.2).
///
/// The old version-named temp file meant an edited worker with no crate
/// version bump kept running the stale binary that happened to be on disk.
/// Keying on the bytes makes a changed worker land at a different path, and
/// on-disk bytes are verified before execution.
fn vision_anchor_worker() -> Result<PathBuf, ProjectError> {
    video_core::content_store::materialize_worker(
        include_bytes!(env!("CUTRIGHT_VISION_ANCHOR")),
        "vision-anchor",
    )
    .map_err(|error| ProjectError::InvalidState(error.to_string()))
}

/// Run the embedded Vision worker for one sampled frame through the shared
/// process runner (§10.1): a bounded timeout and kill-tree teardown on
/// timeout/failure, matching every other external process this crate spawns
/// — never a bare `Command::spawn` left to run unbounded.
///
/// [`video_core::process_runner::run_process`] can't drive this call site
/// directly because the worker's protocol needs a writable stdin, which
/// `run_process` never exposes (it pipes stdin to `/dev/null`).
/// [`ManagedChild`] is the shared streaming-process primitive underneath
/// both `run_process` and this function; `video_media::process`'s
/// `run_caption_card_worker` uses the identical pattern for its own
/// stdin-driven sidecar, reimplemented here because that helper is private
/// to `video-media`.
fn run_vision_worker(worker: &Path, frame: &Path) -> Result<VisionFrameResponse, ProjectError> {
    let spec = ProcessSpec {
        executable: worker.to_path_buf(),
        args: Vec::new(),
        env_allow: vision_worker_env_allow(),
        working_dir: None,
        timeout: VISION_WORKER_TIMEOUT,
        stdout_cap_bytes: VISION_STDOUT_CAP_BYTES,
        stderr_cap_bytes: VISION_STDERR_CAP_BYTES,
    };
    let (mut managed, mut stdin, stdout) = ManagedChild::spawn(&spec).map_err(|error| {
        ProjectError::InvalidState(format!("vision worker spawn failed: {error}"))
    })?;

    let stdout_handle = thread::spawn(move || {
        let mut buffer = Vec::new();
        let mut bounded = stdout.take(VISION_STDOUT_CAP_BYTES as u64);
        let _ = bounded.read_to_end(&mut buffer);
        buffer
    });

    let request = serde_json::json!({ "image_path": frame });
    let request_bytes = serde_json::to_vec(&request)?;
    let write_result = stdin.write_all(&request_bytes).and_then(|()| stdin.flush());
    drop(stdin); // close the write end so the worker sees EOF
    if let Err(error) = write_result {
        managed.kill_tree();
        let _ = stdout_handle.join();
        return Err(ProjectError::InvalidState(format!(
            "vision worker stdin write failed: {error}"
        )));
    }

    let poll_interval = Duration::from_millis(20);
    let start = Instant::now();
    loop {
        if managed.has_exited() {
            break;
        }
        if start.elapsed() >= VISION_WORKER_TIMEOUT {
            managed.kill_tree();
            let _ = stdout_handle.join();
            return Err(ProjectError::InvalidState(format!(
                "vision worker timed out after {VISION_WORKER_TIMEOUT:?}"
            )));
        }
        thread::sleep(poll_interval);
    }

    let stdout_bytes = stdout_handle.join().unwrap_or_default();
    if stdout_bytes.is_empty() {
        let (stderr, truncated) = managed.stderr_snapshot();
        let mut message = String::from_utf8_lossy(&stderr).trim().to_string();
        if truncated {
            message.push_str(" ...[stderr truncated]");
        }
        return Err(ProjectError::InvalidState(format!(
            "vision worker produced no output: {message}"
        )));
    }
    serde_json::from_slice(&stdout_bytes).map_err(ProjectError::from)
}

fn vision_worker_env_allow() -> Vec<(String, String)> {
    ["PATH", "HOME", "TMPDIR"]
        .iter()
        .filter_map(|key| {
            std::env::var(key)
                .ok()
                .map(|value| (key.to_string(), value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_project;

    #[test]
    fn native_vision_mode_defaults_to_legacy_and_rejects_unknown_values() {
        assert_eq!(
            parse_native_vision_mode(None).unwrap(),
            MacNativeMode::Legacy
        );
        assert_eq!(
            parse_native_vision_mode(Some("shadow")).unwrap(),
            MacNativeMode::Shadow
        );
        assert!(parse_native_vision_mode(Some("auto")).is_err());
    }

    /// REV2 plan §6.1 regression: `reframe_plan` must write to the exact
    /// same variant-scoped path a later read for that variant resolves to
    /// — `analysis/reframe/<variant>/reframe-plan.json`, never a generic
    /// alias the write side produced from whichever variant ran last. This
    /// exercises the shared path helper both sides of the pipeline use
    /// (`variant_reframe_path`) the same way `reframe_plan`'s write and
    /// `render_final`/`qa_run`'s reads do, without shelling out to the
    /// embedded Vision worker binary.
    #[test]
    fn reframe_plan_path_round_trips_through_the_same_variant_only() {
        let temp = tempfile::tempdir().unwrap();
        init_project(temp.path(), false).unwrap();

        let tight_path = variant_reframe_path(temp.path(), "tight");
        let natural_path = variant_reframe_path(temp.path(), "natural");
        assert_ne!(tight_path, natural_path);
        assert_eq!(
            tight_path,
            temp.path().join("analysis/reframe/tight/reframe-plan.json")
        );

        let tight_plan = serde_json::json!({ "variant": "tight", "anchors": [] });
        write_json_atomic(&tight_path, &tight_plan).unwrap();

        // Reading back through the SAME helper for the SAME variant returns
        // exactly what was written for it.
        let read_back: serde_json::Value =
            read_json(&variant_reframe_path(temp.path(), "tight")).unwrap();
        assert_eq!(read_back, tight_plan);

        // No generic alias is produced by the write, and the other
        // variant's path is untouched — it does not silently resolve to
        // tight's plan.
        assert!(!temp.path().join("analysis/reframe-plan.json").is_file());
        assert!(!natural_path.is_file());
        assert!(
            require_variant_artifact(temp.path(), &natural_path, "natural", "reframe.plan")
                .is_err()
        );
    }

    fn segment(id: &str, output_start_ms: i64, output_end_ms: i64) -> TimelineSegment {
        TimelineSegment {
            id: id.into(),
            source_id: "cam-a".into(),
            source_start_ms: output_start_ms,
            source_end_ms: output_end_ms,
            output_start_ms,
            output_end_ms,
            speed: 1.0,
            reason: "kept".into(),
        }
    }

    /// The sample-rate widening is bounded: an arbitrarily long timeline
    /// never produces more than `MAX_TOTAL_SAMPLES` samples in total, so a
    /// long source cannot mean an unbounded Vision-worker run (REV2 plan
    /// §15.5's "bounded and resumable" requirement).
    #[test]
    fn effective_sample_interval_bounds_total_samples_for_a_long_timeline() {
        let two_hours_ms = 2 * 60 * 60 * 1_000;
        let segments = vec![segment("s0", 0, two_hours_ms)];
        let interval = effective_sample_interval_ms(&segments);
        let total_samples = sample_offsets_for_segment(&segments[0], interval).len() as i64;
        assert!(
            total_samples <= MAX_TOTAL_SAMPLES,
            "expected at most {MAX_TOTAL_SAMPLES} samples, got {total_samples} at interval {interval}ms"
        );
    }

    /// A short timeline keeps the default (denser) sample spacing rather
    /// than being needlessly widened.
    #[test]
    fn effective_sample_interval_keeps_default_for_a_short_timeline() {
        let segments = vec![segment("s0", 0, 4_000)];
        assert_eq!(
            effective_sample_interval_ms(&segments),
            DEFAULT_SAMPLE_INTERVAL_MS
        );
    }

    /// Every segment is sampled at least once (at offset 0, its shot
    /// boundary), even a segment shorter than the sample interval.
    #[test]
    fn sample_offsets_cover_a_segment_shorter_than_the_interval() {
        let short = segment("s0", 0, 120);
        let offsets = sample_offsets_for_segment(&short, DEFAULT_SAMPLE_INTERVAL_MS);
        assert_eq!(offsets.first(), Some(&0));
        assert!(offsets.iter().all(|&offset| offset < 120));
    }

    /// Sampling never leaves a trailing gap at a segment's tail: the last
    /// offset always lands at (or immediately before) the segment's own end.
    #[test]
    fn sample_offsets_cover_the_full_segment_span() {
        let seg = segment("s0", 0, 1_130);
        let offsets = sample_offsets_for_segment(&seg, 500);
        assert_eq!(offsets, vec![0, 500, 1_000, 1_129]);
    }

    /// Active-speaker association: with exactly one face on screen, that
    /// face is assumed to be the speaker regardless of prior continuity.
    #[test]
    fn active_speaker_observation_uses_the_only_face_present() {
        let mut transcripts = vec![Transcript {
            schema_version: SCHEMA_VERSION,
            provider: "fixture".into(),
            source_id: "cam-a".into(),
            language: "en".into(),
            words: vec![video_core::Word {
                id: "w0".into(),
                source_word_id: None,
                text: "hi".into(),
                start_ms: 0,
                end_ms: 500,
                confidence: 0.95,
                speaker: Some("S0".into()),
                kind: "word".into(),
            }],
            events: Vec::new(),
        }];
        let faces = vec![VisionDetectionBox {
            center_x: 0.6,
            center_y: 0.4,
            area: 0.1,
            confidence: 0.9,
        }];
        let mut positions = BTreeMap::new();
        let observation =
            active_speaker_observation(&transcripts, "cam-a", 100, &faces, &mut positions)
                .expect("active speaker observation");
        assert_eq!(observation.modality, Modality::ActiveSpeaker);
        assert!((observation.center_x - 0.6).abs() < 1e-9);
        assert_eq!(positions.get("S0"), Some(&(0.6, 0.4)));

        // A later sample with the SAME speaker and two faces present picks
        // the one nearest the recorded continuity position, not just the
        // first/most-confident one.
        let two_faces = vec![
            VisionDetectionBox {
                center_x: 0.6,
                center_y: 0.4,
                area: 0.1,
                confidence: 0.5,
            },
            VisionDetectionBox {
                center_x: 0.1,
                center_y: 0.1,
                area: 0.2,
                confidence: 0.99,
            },
        ];
        transcripts[0].words[0].start_ms = 100;
        transcripts[0].words[0].end_ms = 600;
        let observation =
            active_speaker_observation(&transcripts, "cam-a", 150, &two_faces, &mut positions)
                .expect("continuity observation");
        assert!(
            (observation.center_x - 0.6).abs() < 1e-9,
            "expected continuity to win over the more confident but distant face, got {}",
            observation.center_x
        );
    }

    /// No transcript word is active at this timestamp: no active-speaker
    /// evidence is produced (falls through to face/body/saliency fusion).
    #[test]
    fn active_speaker_observation_is_none_outside_any_word() {
        let transcripts = vec![Transcript {
            schema_version: SCHEMA_VERSION,
            provider: "fixture".into(),
            source_id: "cam-a".into(),
            language: "en".into(),
            words: vec![video_core::Word {
                id: "w0".into(),
                source_word_id: None,
                text: "hi".into(),
                start_ms: 0,
                end_ms: 500,
                confidence: 0.95,
                speaker: Some("S0".into()),
                kind: "word".into(),
            }],
            events: Vec::new(),
        }];
        let mut positions = BTreeMap::new();
        assert!(
            active_speaker_observation(&transcripts, "cam-a", 10_000, &[], &mut positions)
                .is_none()
        );
    }
}
