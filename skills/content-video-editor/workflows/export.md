# Export + package + handoffs

Render every platform preset from the one canonical timeline, package the deliverables, write
interchange, and emit the typed handoffs to Writing, Designer, and Social. This is the last stage; it
runs only after [qa](qa.md) passes and Adrian has accepted the visual result.

## Inputs

- `qa/report.json` with `status:"pass"` and Adrian's visual acceptance.
- The selected variant, finished; vertical reframed and anchor-approved (for 9:16 presets).
- `project.json` `outputs[]` declaring the presets (`youtube` 16:9, `reels`/`tiktok` 9:16, with
  width/height).

## Commands (in order)

```bash
# 1. YouTube final is already rendered + QA'd. Render the vertical preset(s).
#    A 9:16 preset REFUSES to render without an approved analysis/reframe-plan.json.
videoctl render final <project> --preset reels
videoctl render final <project> --preset tiktok   # if declared

# 2. Package deliverables into exports/. Requires youtube.mp4 + reels.mp4 + captions.srt to exist.
videoctl package social <project>

# 3. Interchange timeline (reads edit/timeline.json — the selected variant's alias).
videoctl export otio <project>
```

## Evidence to read before deciding

- `render/finals/<preset>.mp4` → each final exists and matches its preset dimensions.
- `qa/report.json` → still `pass` (re-QA if any final was re-rendered after QA).
- `exports/` tree → the packaged deliverables landed.

## What gets written

```text
exports/
├── youtube/youtube.mp4          # 16:9 final
├── vertical/reels.mp4           # 9:16 final
├── captions/youtube.srt         # caption sidecar (16:9)
├── captions/reels.srt           # caption sidecar (9:16)
└── interchange/timeline.otio.json  # OTIO for hand-off to an NLE if ever needed
```

## The handoffs (emit typed records — see docs/HANDOFF-CONTRACTS.md)

The MP4s are not the whole delivery. Write the typed handoff records so the owning skills can finish:

- **→ Writing:** titles, descriptions, and per-platform hooks from the output transcript + brief.
  Writing owns the words; it does not move cut points.
- **→ Designer:** a thumbnail / styleframe request keyed to the finished frame (a strong payoff moment),
  with brand + safe-zone constraints. Designer owns the static layout.
- **→ Social:** the platform-package record per deliverable — aspect, duration, caption burn, hook goal,
  CTA, target audience. Social owns packaging + distribution; it does not choose source cut points.

## Gate

- Every declared preset has a final in `render/finals/` and a packaged copy in `exports/`.
- `qa/report.json` is `pass` for the delivered finals (re-run QA after any post-QA re-render).
- The OTIO interchange reflects the selected variant.
- All three handoff records are written to their contract locations with the locked payload.
- Sources remain untouched (manifest hashes unchanged).

## Handoff outputs

- `exports/**` → the delivered files.
- `brief/platform-brief.json` (from Social, consumed) + the outward handoff records to Writing,
  Designer, Social ([docs/HANDOFF-CONTRACTS.md](../../../docs/HANDOFF-CONTRACTS.md)).
- `exports/interchange/timeline.otio.json` → optional NLE interchange.

## Engine gaps to know

- `render final` is hard-coded to `render/rough-cuts/natural.mp4` + generic `edit/captions.srt`
  (REV2 §3.8/§3.13). If the selected variant is **tight**, every preset still renders from natural until
  variant-aware final lands — select natural or block and escalate. The burned captions come from the
  generic `edit/captions.srt`, so ensure it reflects the selected variant.
- `package social` copies the **generic** `edit/captions.srt` to both the YouTube and vertical caption
  exports; it does not yet use per-variant or per-preset caption profiles (REV2 §13.3). Vertical caption
  burn-in + safe-zone profiles are Phase 4.
- `export otio` reads the shared `edit/timeline.json`, so it exports whichever variant rendered last —
  re-render the selected variant last to keep it correct.
- The handoff **records** to Writing/Designer/Social are a contract this skill writes
  ([docs/HANDOFF-CONTRACTS.md](../../../docs/HANDOFF-CONTRACTS.md)); the engine does not generate them
  yet. `package social` only copies media — author the typed records yourself.
