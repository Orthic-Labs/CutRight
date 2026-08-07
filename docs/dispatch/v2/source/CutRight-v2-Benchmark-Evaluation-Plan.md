# CutRight v2 benchmark and evaluation plan

## 1. Purpose

The benchmark programme determines architecture, pack selection, autonomy, and release readiness. It is not an end-of-project demo. No format advances beyond `reviewed` because a model card looks strong or because one showcase edit looks good.

## 2. Golden corpus

Every media item has a rights manifest, source hash, consent/provenance, expected language, camera/audio conditions, and permitted distribution. The minimum corpus contains:

- 40 recorded-footage projects: single take, multi-take, interview, podcast, tutorial, screen recording, multi-camera, difficult room tone, interruptions, camera handling and mixed frame rates.
- 30 repurpose projects: podcasts, talks, tutorials and finished videos with human-ranked standalone shorts.
- 20 explainer projects: local source packages, scripts, charts, narration, product/process and historical topics.
- 20 anchored-creative projects: presenters, product labels, logos, packaging, identity-sensitive photos and A-roll restyling.
- 20 adversarial projects: false chronology risk, contradictory takes, clipped speech, silent spans, captions near UI zones, camera motion, low light, HDR, variable frame rate, corrupt media and interrupted jobs.

At least 25% of the speech corpus is non-English or code-switching before multilingual support is claimed. Projects are split by speaker, recording session and source programme so near-duplicates cannot cross train/calibration and test sets.

## 3. Evaluation axes

### 3.1 Kernel integrity

- Source mutation count: exactly zero.
- Atomic action success: 100% of injected interruption points leave either the old revision or the complete new revision.
- Undo round-trip: canonical timeline hash returns to the pre-action hash for every reversible action.
- Receipt verification: 100% pass before packaging; tampering is detected.
- Stale revision rejection: 100%.
- Cache identity: moved paths preserve cache hits; changed bytes invalidate them.

### 3.2 Speech and boundary quality

- Word/phoneme truncation: zero clipped high-confidence lexical words in the release corpus.
- Boundary onset/offset absolute error: report median, p90 and p95 against human labels.
- Boundary consensus coverage: fraction supported by primary ASR, VAD and verifier.
- Filler removal precision/recall by context class.
- False-start removal precision and replacement-presence proof.
- Transcript preservation: the canonical transcript never deletes speech because a cut drops it.

### 3.3 Audio-visual preservation

- Audio-video sync drift and local onset alignment before/after edit.
- Non-target frame preservation outside declared actions.
- Non-target audio preservation outside declared fades, cuts and mixes.
- Loudness, true peak, clipping, discontinuity, noise and channel-layout checks.
- Identity, label, logo and OCR preservation for anchored projects.

### 3.4 Editorial quality

- Beat segmentation agreement with human editors.
- Duplicate-take clustering precision/recall.
- Best-take acceptance and score margin.
- Narrative-order acceptance.
- Hook, payoff and CTA acceptance.
- Reorder truthfulness violations: zero accepted false-chronology cases.
- Manual correction time and number of corrections per minute of final video.
- Final accept/reject and reason distribution.

### 3.5 Creative quality

- Brand-token compliance.
- Subject, face, gesture, caption and platform-UI collision count: zero unresolved collisions.
- Crop stability and subject loss.
- Graphic, caption, motion, SFX and music acceptance.
- Reduced-motion equivalence.
- Text legibility and OCR match.
- Visual critic agreement with human review, including false-positive and false-negative rates.

### 3.6 Instruction and preservation quality

Each request is decomposed into a checklist. Score:

- target success;
- untouched-content preservation;
- joint success;
- intent fidelity;
- realism/naturalness;
- temporal and spatial consistency;
- evidence traceability;
- escalation correctness.

### 3.7 Reliability and resource quality

- Cold start, stage latency, throughput, peak RSS/VRAM, disk writes and pack load time by supported target.
- Cancellation latency.
- Resume success at every stage boundary.
- Retry amplification and duplicate-cost prevention.
- Clean-machine offline success.
- Zero network attempts in offline mode.

## 4. Evaluator separation

Planner and critic are separate model instances and separate prompts. Deterministic metrics run first. The critic sees the brief, evidence references, semantic action diff, before/after samples, and deterministic findings. It returns a schema-bound verdict with exact frame/time evidence. It cannot execute actions.

A critic disagreement triggers one bounded revision cycle. A second disagreement or low-confidence verdict escalates. The planner's self-assessment is logged but never counted as independent evidence.

## 5. Initial gates

These are starting release floors; the benchmark report must show confidence intervals and may tighten them, never silently loosen them.

| Gate | Reviewed | Review-light | Autonomous |
|---|---:|---:|---:|
| Consecutive human-accepted finals | 0 | 5 | 10 after review-light |
| Projects in current mode | 0 | ≥5 | ≥15 |
| Best-take acceptance | report only | ≥0.85 | ≥0.92 |
| Boundary correction rate | report only | ≤0.15 | ≤0.08 |
| Graphic acceptance | report only | ≥0.80 | ≥0.90 |
| Unresolved escalation rate | allowed, blocks run | ≤0.15 | ≤0.05 |
| False chronology | 0 accepted | 0 accepted | 0 accepted |
| Source mutation / atomicity / receipt failure | 0 | 0 | 0 |
| Unresolved caption/subject collision | 0 | 0 | 0 |
| Independent critic required | advisory | required for finish | required for every final |

A runtime-pack, model, prompt, skill, renderer, or threshold change resets or invalidates affected format evidence according to its compatibility declaration.

## 6. Benchmark artefacts

Each run writes:

```text
benchmarks/runs/<run_id>/manifest.json
benchmarks/runs/<run_id>/per-project.jsonl
benchmarks/runs/<run_id>/metrics.json
benchmarks/runs/<run_id>/confusion-matrices/
benchmarks/runs/<run_id>/samples/
benchmarks/runs/<run_id>/failures/
benchmarks/runs/<run_id>/report.md
benchmarks/runs/<run_id>/receipt.json
```

The report identifies skipped checks, missing labels, unsupported targets and unproven claims. A tool that did not run is `unproven`, never `pass`.

## 7. Human review protocol

Reviewers compare blinded variants where possible, use a fixed reason vocabulary, and annotate exact time ranges. Editorial disagreement is retained; consensus and individual preferences are separate data. The user’s acceptance controls their autonomy profile, while the shared benchmark controls minimum safety and integrity floors.
