# Transcribe + benchmark + VAD

Produce the word-level transcript (on the **original source timebase**), verify its word edges with an
independent aligner, and lay down the VAD signal the cut plan needs. All audio inference is
HeardRight's; CutRight only calls it. This skill never bundles or downloads models.

## Inputs

- `<project>/sources/manifest.json` from [ingest](ingest.md).
- HeardRight reachable as CutRight's local-audio service (transcription + VAD). WhisperX available as
  the alignment verifier.

## Commands (in order)

```bash
# 1. Transcribe every source with the primary provider (HeardRight).
videoctl transcribe <project> --provider heardright

# 2. Verify word-edge safety against the independent aligner (WhisperX).
#    HeardRight is the AUTHORITY; WhisperX verifies timestamps, it does not compete.
videoctl bench transcribe <project> --primary heardright --verifier whisperx --boundaries 20 --padding-ms 40

# 3. VAD signal per source (HeardRight-owned Silero, delivered through CutRight).
#    Required before `edit render` — the cut plan reads analysis/vad-<source>.json.
videoctl analyze local <project>
```

Optional, only with explicit consent and a budget (`cloud.allowed:true` in the project config):
`videoctl analyze cloud <project> --provider gemini|twelvelabs` for semantic segmentation / B-roll
themes. Cloud is off by default; a cloud call never overwrites local timestamps.

## Evidence to read before deciding

- `analysis/transcripts/<source_id>.json` → per-source word transcript: `words[]` with `id`, `text`,
  `start_ms`, `end_ms`, `confidence`, `speaker`, plus non-speech `events[]`. Fillers and false starts
  are **preserved**, not stripped. Word ids restart per source, so join across sources only via the
  compound `source_word_id` (`<source_id>:<word_id>`) when present.
- `analysis/transcripts/<source_id>.heardright.envelope.json` → provider provenance (provider, model,
  request hash, raw response path). Keep it; do not edit.
- `analysis/transcript-packed.md` → compact phrase view split on speaker changes / meaningful pauses.
  **This is what you read to reason editorially** in [rough-cut](rough-cut.md), not the raw word list.
- `analysis/bench/transcribe/report.json` → the `decision` field (`primary` / `verifier` / `unresolved`)
  and the boundary disagreement detail.
- `analysis/vad-<source_id>.json` → `regions[]` (`start_ms`, `end_ms`, `mean_probability`) on the source
  timebase. A signal for the cut plan, never a destructive pre-cut.

## Gate

- A transcript exists for **every** source in the manifest.
- `analysis/bench/transcribe/report.json` has a resolved `decision` (not `unresolved`). Word-edge cuts
  are destructive; do not make them until the benchmark clears the primary transcript's timestamps.
- A VAD file exists for every source.

If `decision` is `unresolved`: read the disagreement detail first. The current bench policy is known to
return `unresolved` when **both** providers are good (REV2 §3.11 inverts the intended HeardRight-primary
policy). That is an engine defect to escalate, not a reason to force a cut or to silently promote
WhisperX over HeardRight. Do not proceed to destructive cutting on an unresolved benchmark in
`reviewed` mode; in a calibrated `autonomous` format, fall back to conservative (wider) cut margins and
record the override in `feedback/decisions.jsonl`.

## Handoff outputs

- `analysis/transcripts/*.json` + `analysis/transcript-packed.md` → [rough-cut](rough-cut.md)
  candidate generation and editorial reasoning.
- `analysis/bench/transcribe/report.json` → required later by [qa](qa.md) (QA rejects `unresolved`).
- `analysis/vad-<source>.json` → required by `edit render` (cut-plan boundary expansion).

## Engine gaps to know

- The packed transcript is currently built from the **first source only** (REV2 §2.2); multi-source
  projects lose later sources from the packed view. Read the per-source `analysis/transcripts/*.json`
  directly when the project has more than one source.
- `analyze local` still runs CutRight's own bundled Silero worker; REV2 §9 migrates VAD behind the
  HeardRight service boundary. Treat the VAD output as HeardRight-owned regardless of which worker
  produced it, and do not add a second audio stack.
- The benchmark report is not yet hash-bound to the exact transcript it verified (REV2 P1 §8.3); record
  the report path in your handoff so a later re-transcription cannot silently invalidate an old cut.
