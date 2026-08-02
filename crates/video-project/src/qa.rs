use crate::io::*;
use crate::receipts;
use crate::validate_edit;
use crate::PipelineArtifact;
use crate::ProjectError;
use chrono::Utc;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use video_core::{models::SCHEMA_VERSION, SourceManifest, Timeline};
use video_media::probe;

/// Runs QA for exactly one deliverable preset and writes its own report at
/// `qa/<preset>.report.json` (§13.2) — replacing the old single
/// YouTube-shaped `qa/report.json`. Run once per preset (`--preset
/// youtube|reels|tiktok`) to build the full `qa/youtube.report.json` +
/// `qa/reels.report.json` + `qa/tiktok.report.json` set; every run refreshes
/// `qa/summary.json` from whatever reports currently exist on disk.
pub fn qa_run(
    project_path: &Path,
    variant: Option<&str>,
    preset: &str,
    dry_run: bool,
) -> Result<PipelineArtifact, ProjectError> {
    let sources: SourceManifest = read_json(&project_path.join("sources/manifest.json"))?;
    for source in &sources.sources {
        let actual = format!("blake3:{}", hash_file(Path::new(&source.path))?);
        if actual != source.blake3 {
            return Err(ProjectError::SourceChanged(PathBuf::from(&source.path)));
        }
    }
    let manifest = read_project_manifest(&project_path.join("project.json"))?;
    let output_preset = manifest
        .outputs
        .iter()
        .find(|candidate| candidate.id == preset)
        .ok_or_else(|| {
            ProjectError::InvalidState(format!("project is missing the {preset} preset"))
        })?;
    let output = project_path.join(format!("render/finals/{preset}.mp4"));
    if !output.is_file() {
        return Err(ProjectError::InvalidState(format!(
            "QA requires an explicit final render: {}",
            output.display()
        )));
    }
    let provenance_path = project_path.join(format!("render/finals/{preset}.provenance.json"));
    let provenance: serde_json::Value = read_json(&provenance_path).map_err(|_| {
        ProjectError::InvalidState(format!(
            "QA requires render.final provenance for {preset}; render the final first"
        ))
    })?;
    let provenance_variant = provenance
        .get("variant")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ProjectError::InvalidState("final provenance is missing a variant".into()))?
        .to_string();
    // §13.2: QA must reject a mixed-variant artifact graph — if the caller
    // asked for a specific variant (or the currently selected review base
    // differs), that has to match the variant this final was actually
    // rendered from.
    let requested_variant = resolve_variant(project_path, variant)?;
    if requested_variant != provenance_variant {
        return Err(ProjectError::InvalidState(format!(
            "QA rejects a mixed-variant artifact graph: {preset} was rendered from variant \
             {provenance_variant} but variant {requested_variant} is currently selected/requested"
        )));
    }
    let variant = provenance_variant;
    validate_edit(project_path, Some(&variant))?;

    let benchmark = project_path.join("analysis/bench/transcribe/report.json");
    let benchmark_report: serde_json::Value = read_json(&benchmark).map_err(|_| {
        ProjectError::InvalidState(
            "QA requires a resolved `videoctl bench transcribe <project>` report".into(),
        )
    })?;
    let benchmark_decision = benchmark_report
        .get("decision")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unresolved")
        .to_string();
    if benchmark_decision == "unresolved" {
        return Err(ProjectError::InvalidState(
            "QA rejects an unresolved transcription benchmark".into(),
        ));
    }

    let report_path = project_path.join(format!("qa/{preset}.report.json"));
    if dry_run {
        return Ok(PipelineArtifact {
            status: "dry-run",
            path: report_path,
            count: 1,
        });
    }

    let captions = variant_captions_path(project_path, &variant);
    let timeline_path = variant_timeline_path(project_path, &variant);
    let timeline: Timeline = read_json(&timeline_path)?;
    let evidence = project_path.join("analysis/evidence/manifest.json");
    let output_hash = format!("blake3:{}", hash_file(&output)?);
    let recorded_output_hash = provenance
        .get("output_hash")
        .and_then(serde_json::Value::as_str);

    let mut checks = Vec::new();
    checks.push(serde_json::json!({
        "id": "provenance.output_unchanged",
        "status": if recorded_output_hash == Some(output_hash.as_str()) { "pass" } else { "fail" },
        "evidence": { "path": relative_artifact_path(project_path, &output), "live_hash": output_hash, "recorded_hash": recorded_output_hash }
    }));
    for (label, live_path, recorded_key) in [
        ("captions", captions.as_path(), "captions_hash"),
        ("timeline", timeline_path.as_path(), "timeline_hash"),
    ] {
        let live_hash = format!("blake3:{}", hash_file(live_path)?);
        let recorded_hash = provenance
            .get(recorded_key)
            .and_then(serde_json::Value::as_str);
        checks.push(serde_json::json!({
            "id": format!("provenance.{label}_unchanged"),
            "status": if recorded_hash == Some(live_hash.as_str()) { "pass" } else { "fail" },
            "evidence": { "path": relative_artifact_path(project_path, live_path), "live_hash": live_hash, "recorded_hash": recorded_hash }
        }));
    }

    let metadata = probe(&output)?;
    let expected_duration_ms = timeline
        .tracks
        .first()
        .and_then(|track| track.segments.last())
        .map(|segment| segment.output_end_ms)
        .unwrap_or(0);
    let duration_ok = metadata
        .duration_ms
        .is_some_and(|actual| (actual - expected_duration_ms).abs() <= 250);
    checks.push(serde_json::json!({
        "id": "media.duration",
        "status": if duration_ok { "pass" } else { "fail" },
        "evidence": { "expected_ms": expected_duration_ms, "probed_ms": metadata.duration_ms }
    }));
    checks.push(serde_json::json!({
        "id": "media.streams",
        "status": if metadata.has_video && metadata.has_audio { "pass" } else { "fail" },
        "evidence": { "video": metadata.has_video, "audio": metadata.has_audio }
    }));
    let dimensions_ok = metadata.width == Some(output_preset.width)
        && metadata.height == Some(output_preset.height);
    checks.push(serde_json::json!({
        "id": "media.dimensions",
        "status": if dimensions_ok { "pass" } else { "fail" },
        "evidence": {
            "expected": { "width": output_preset.width, "height": output_preset.height },
            "probed": { "width": metadata.width, "height": metadata.height }
        }
    }));
    let expected_fps = fps(&timeline.timebase);
    let probed_fps = metadata.timebase.as_ref().map(fps);
    let fps_ok = probed_fps.is_some_and(|value| (value - expected_fps).abs() < 0.05);
    checks.push(serde_json::json!({
        "id": "media.frame_rate",
        "status": if fps_ok { "pass" } else { "fail" },
        "evidence": { "expected_fps": expected_fps, "probed_fps": probed_fps }
    }));

    let decoded_clean = decode_through_end(&output)?;
    checks.push(serde_json::json!({
        "id": "media.decode_through_end",
        "status": if decoded_clean { "pass" } else { "fail" },
        "evidence": { "path": relative_artifact_path(project_path, &output) }
    }));

    let (tail_black, tail_frozen) =
        detect_tail_black_or_frozen(&output, metadata.duration_ms.unwrap_or(0), 2_000)?;
    checks.push(serde_json::json!({
        "id": "media.tail_not_black_or_frozen",
        "status": if !tail_black && !tail_frozen { "pass" } else { "fail" },
        "evidence": { "black_tail": tail_black, "frozen_tail": tail_frozen }
    }));

    let loudness = measure_loudness(&output)?;
    let loudness_ok = loudness.integrated_lufs.is_some() && loudness.clipped_samples == 0;
    checks.push(serde_json::json!({
        "id": "audio.loudness_true_peak_clipping",
        "status": if loudness_ok { "pass" } else { "fail" },
        "evidence": {
            "integrated_lufs": loudness.integrated_lufs,
            "true_peak_dbtp": loudness.true_peak_dbtp,
            "clipped_samples": loudness.clipped_samples
        }
    }));

    if captions.is_file() {
        let (cue_count, last_end_ms) = caption_timing_coverage(&captions)?;
        let coverage_gap_ms = last_end_ms.map(|end| (expected_duration_ms - end).max(0));
        let coverage_ok = cue_count > 0 && coverage_gap_ms.is_some_and(|gap| gap <= 2_000);
        checks.push(serde_json::json!({
            "id": "captions.timing_coverage",
            "status": if coverage_ok { "pass" } else { "fail" },
            "evidence": { "cue_count": cue_count, "last_cue_end_ms": last_end_ms, "coverage_gap_ms": coverage_gap_ms }
        }));
    } else {
        checks.push(serde_json::json!({
            "id": "captions.timing_coverage",
            "status": "fail",
            "evidence": { "path": relative_artifact_path(project_path, &captions) }
        }));
    }

    checks.push(serde_json::json!({
        "id": "evidence.present",
        "status": if evidence.is_file() { "pass" } else { "fail" },
        "evidence": { "path": relative_artifact_path(project_path, &evidence) }
    }));

    if output_preset.aspect == "9:16" {
        let reframe_path = variant_reframe_path(project_path, &variant);
        let recorded_reframe_hash = provenance
            .get("reframe_plan_hash")
            .and_then(serde_json::Value::as_str);
        let identity_ok = reframe_path.is_file()
            && recorded_reframe_hash.is_some_and(|recorded| {
                hash_file(&reframe_path)
                    .map(|hash| format!("blake3:{hash}") == recorded)
                    .unwrap_or(false)
            });
        checks.push(serde_json::json!({
            "id": "reframe.plan_identity",
            "status": if identity_ok { "pass" } else { "fail" },
            "evidence": { "path": relative_artifact_path(project_path, &reframe_path), "recorded_hash": recorded_reframe_hash }
        }));
    }

    checks.push(serde_json::json!({
        "id": "transcript.benchmark",
        "status": "pass",
        "evidence": { "path": relative_artifact_path(project_path, &benchmark), "decision": benchmark_decision }
    }));

    // §8 taste gate: a deliverable never ships on automated checks alone.
    let verdict_path = project_path.join(format!("feedback/verdict/{preset}.json"));
    let verdict: Option<serde_json::Value> = read_json_if_file(&verdict_path);
    let verdict_ok = verdict
        .as_ref()
        .and_then(|value| value.get("approved"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    checks.push(serde_json::json!({
        "id": "human.final_verdict",
        "status": if verdict_ok { "pass" } else { "pending" },
        "evidence": { "path": relative_artifact_path(project_path, &verdict_path), "present": verdict.is_some() }
    }));

    let overall_pass = checks
        .iter()
        .all(|check| check.get("status").and_then(serde_json::Value::as_str) == Some("pass"));
    let report = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "status": if overall_pass { "pass" } else { "fail" },
        "preset": preset,
        "variant": variant,
        "output": relative_artifact_path(project_path, &output),
        "output_hash": output_hash,
        "source_hashes": sources.sources.iter().map(|source| serde_json::json!({
            "source_id": source.source_id,
            "blake3": source.blake3,
        })).collect::<Vec<_>>(),
        "selected_variant_hash": format!("blake3:{}", hash_file(&timeline_path)?),
        "benchmark_decision": benchmark_decision,
        "checks": checks,
        "generated_at": Utc::now(),
    });
    write_json_atomic(&report_path, &report)?;
    write_qa_summary(project_path)?;
    receipts::write_stage_receipt(
        &receipts::receipt_path_for(&report_path),
        "qa.run",
        &[
            output.as_path(),
            captions.as_path(),
            timeline_path.as_path(),
            benchmark.as_path(),
        ],
        &serde_json::json!({ "preset": preset, "variant": variant }),
        BTreeMap::new(),
        &[report_path.as_path()],
    )?;

    Ok(PipelineArtifact {
        status: if overall_pass { "pass" } else { "fail" },
        path: report_path,
        count: 1,
    })
}

/// Aggregates every per-deliverable QA report into one `qa/summary.json`
/// (§13.2) by scanning `qa/*.report.json` on disk — not just the
/// deliverable the current run touched — so the summary always reflects
/// every report that currently exists, in one place.
fn write_qa_summary(project_path: &Path) -> Result<(), ProjectError> {
    let qa_dir = project_path.join("qa");
    let mut deliverables = Vec::new();
    if qa_dir.is_dir() {
        let mut paths = fs::read_dir(&qa_dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(".report.json"))
            })
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            let report: serde_json::Value = read_json(&path)?;
            deliverables.push(serde_json::json!({
                "preset": report.get("preset").cloned().unwrap_or(serde_json::Value::Null),
                "status": report.get("status").cloned().unwrap_or(serde_json::Value::Null),
                "output_hash": report.get("output_hash").cloned().unwrap_or(serde_json::Value::Null),
                "report_path": relative_artifact_path(project_path, &path),
                "report_hash": format!("blake3:{}", hash_file(&path)?),
            }));
        }
    }
    let overall_status = if deliverables.is_empty() {
        "pending"
    } else if deliverables
        .iter()
        .all(|item| item.get("status").and_then(serde_json::Value::as_str) == Some("pass"))
    {
        "pass"
    } else {
        "fail"
    };
    write_json_atomic(
        &project_path.join("qa/summary.json"),
        &serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "status": overall_status,
            "deliverables": deliverables,
            "generated_at": Utc::now(),
        }),
    )
}

struct LoudnessMeasurement {
    integrated_lufs: Option<f64>,
    true_peak_dbtp: Option<f64>,
    clipped_samples: u64,
}

fn qa_ffmpeg_bin() -> PathBuf {
    std::env::var_os("CUTRIGHT_FFMPEG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("ffmpeg"))
}

/// Decodes the whole file end-to-end and requires a clean exit with no
/// decoder errors on stderr — catches truncated/corrupt renders that probe
/// alone (container metadata only) would not.
fn decode_through_end(path: &Path) -> Result<bool, ProjectError> {
    let output = Command::new(qa_ffmpeg_bin())
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-f", "null", "-"])
        .output()
        .map_err(|error| {
            ProjectError::InvalidState(format!("ffmpeg decode check could not start: {error}"))
        })?;
    Ok(output.status.success() && output.stderr.is_empty())
}

/// Runs blackdetect + freezedetect over just the tail window of the render
/// so a deliverable that silently drops to black or freezes at the very end
/// (a common truncated-render symptom) fails QA instead of shipping.
fn detect_tail_black_or_frozen(
    path: &Path,
    duration_ms: i64,
    tail_ms: i64,
) -> Result<(bool, bool), ProjectError> {
    let start_seconds = (duration_ms - tail_ms).max(0) as f64 / 1000.0;
    let output = Command::new(qa_ffmpeg_bin())
        .args(["-v", "info", "-ss"])
        .arg(format!("{start_seconds:.3}"))
        .arg("-i")
        .arg(path)
        .args([
            "-vf",
            "blackdetect=d=0.5:pic_th=0.98,freezedetect=n=-60dB:d=0.5",
            "-an",
            "-f",
            "null",
            "-",
        ])
        .output()
        .map_err(|error| {
            ProjectError::InvalidState(format!("ffmpeg tail check could not start: {error}"))
        })?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok((
        stderr.contains("black_start"),
        stderr.contains("freeze_start"),
    ))
}

/// Measures integrated loudness (LUFS) and true peak (dBTP) via `ebur128`
/// and sums clipped-sample counts across channels via `astats`, all in one
/// ffmpeg pass over the audio.
fn measure_loudness(path: &Path) -> Result<LoudnessMeasurement, ProjectError> {
    let output = Command::new(qa_ffmpeg_bin())
        .args(["-v", "info", "-i"])
        .arg(path)
        .args(["-af", "astats=reset=1,ebur128=peak=true", "-f", "null", "-"])
        .output()
        .map_err(|error| {
            ProjectError::InvalidState(format!("ffmpeg loudness check could not start: {error}"))
        })?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(LoudnessMeasurement {
        integrated_lufs: parse_last_labeled_f64(&stderr, "I:"),
        true_peak_dbtp: parse_last_labeled_f64(&stderr, "Peak:"),
        clipped_samples: parse_summed_labeled_u64(&stderr, "Number of clipped samples:"),
    })
}

fn parse_last_labeled_f64(text: &str, label: &str) -> Option<f64> {
    text.lines().rev().find_map(|line| {
        line.trim()
            .strip_prefix(label)
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|token| token.parse::<f64>().ok())
    })
}

fn parse_summed_labeled_u64(text: &str, label: &str) -> u64 {
    text.lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix(label)
                .and_then(|rest| rest.split_whitespace().next())
                .and_then(|token| token.parse::<u64>().ok())
        })
        .sum()
}

/// Counts SRT cues and finds the last cue's end timestamp so QA can compare
/// caption coverage against the deliverable's actual duration.
fn caption_timing_coverage(captions_path: &Path) -> Result<(usize, Option<i64>), ProjectError> {
    let text = fs::read_to_string(captions_path)?;
    let mut cue_count = 0_usize;
    let mut last_end_ms = None;
    for line in text.lines() {
        if let Some((_, end)) = line.split_once("-->") {
            cue_count += 1;
            if let Some(ms) = parse_srt_timestamp(end.trim()) {
                last_end_ms = Some(ms);
            }
        }
    }
    Ok((cue_count, last_end_ms))
}

fn parse_srt_timestamp(value: &str) -> Option<i64> {
    let (hms, millis) = value.split_once(',')?;
    let mut parts = hms.split(':');
    let hours: i64 = parts.next()?.parse().ok()?;
    let minutes: i64 = parts.next()?.parse().ok()?;
    let seconds: i64 = parts.next()?.parse().ok()?;
    let millis: i64 = millis.parse().ok()?;
    Some((hours * 3_600 + minutes * 60 + seconds) * 1_000 + millis)
}
