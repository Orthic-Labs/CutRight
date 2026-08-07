# Native Renderer Migration Contract

Status: **frozen** by CR-V2-B1-021. This document is the deletion contract for
the Remotion and HyperFrames render stacks. It exists so that no visual
requirement is lost merely because its old technology is rejected.

- Dispositions + forbidden runtime guards:
  [`imports/v2/dispositions/renderers.json`](../../imports/v2/dispositions/renderers.json)
- Machine-checked migration rows:
  [`fixtures/native-renderer/manifest.json`](../../fixtures/native-renderer/manifest.json)
- Validator:
  `python3 tools/v2-evals/check_renderer_migration_manifest.py fixtures/native-renderer/manifest.json`

## 1. Classification of the rejected stacks

Remotion and HyperFrames enter the v2 corpus only from the pinned workspace
(`source_id` `workspace-capabilities`, revision
`6ee21f03a787e7b57dc412760a8996ea7a235302`, paths `tools/remotion/**` and
`tools/hyperframes/**`). Both are classified
**`provenance_only` / `clean_room_behavior`**:

- **provenance_only** — no binary, package, lockfile, or runtime bytes from
  either stack may enter the repository, a runtime pack, or a release.
- **clean_room_behavior** — only observed behavior (what a composition does,
  its timing intent, its layout constraints) may inform native effects. No
  source, declarations, descriptions, schemas, or comments are copied.

Runtime-pack prohibition (enforced by the release guard lists in
`renderers.json`):

| Stack | Forbidden npm packages | Forbidden binaries / engines |
| --- | --- | --- |
| Remotion | `remotion`, `@remotion/bundler`, `@remotion/cli`, `@remotion/renderer`, `react`, `react-dom` | `node`, `npm`, `npx`, `chromium`, `chrome-headless-shell` |
| HyperFrames | none observed in the pinned project (its `package.json` declares no dependencies); the ban is on the HyperFrames runtime and its `hyperframes.json` document-format tooling | `node`, `npm`, `npx`, `chromium`, `chrome-headless-shell` |

## 2. Inventory of the current CutRight render surface

Everything below is verified against the committed tree; the migration table
in §3 gives every item a native target.

### 2.1 Effect registry

Schema: `schemas/effect-registry.schema.json`. One entry per render effect:
`effect_id`, `renderer` (`ffmpeg` | `ass` | `remotion` | `hyperframes`),
versioned `props_schema`, `safe_zones`, `motion_profile`, `preview_fixture`,
`footprint`, `reduced_motion`. Adding an effect means adding one entry plus a
fixture, never a new branch in the render pipeline.

Registry (`schemas/effects/registry.json`), five effects:

| effect_id | renderer | motion_profile | safe_zones | reduced_motion |
| --- | --- | --- | --- | --- |
| `caption.bold-karaoke.v1` | `ass` | restrained | youtube-lower-third, vertical-bottom | static-fallback |
| `lower-third.identity-card.v1` | `remotion` | restrained | youtube-lower-third | static-fallback |
| `stat-counter.v1` | `remotion` | expressive | youtube-lower-third, vertical-bottom | static-fallback |
| `quote-card.v1` | `remotion` | restrained | youtube-lower-third, vertical-bottom | static-fallback |
| `cta-end-card.v1` | `remotion` | restrained | youtube-lower-third, vertical-bottom | static-fallback |

### 2.2 Timing rules and synchronisation schemas

- `schemas/caption-document.schema.json` + `schemas/caption-profile.schema.json`
  — word-level caption documents and per-profile styling; the karaoke sweep
  timing for `caption.bold-karaoke.v1` is driven by transcript word timings.
- `schemas/transcript.schema.json` — word-level transcripts; the clock every
  audio-synchronised effect is timed against.
- `schemas/cut-plan.schema.json` — cut decisions that hook pull-back and punch
  wave attach to.
- `schemas/reframe-track.schema.json` — reframing keyframe tracks.
- `schemas/audio-finish.schema.json` + `schemas/finish-plan.schema.json` —
  audio finish plans that audio-synchronised effects must lock to.
- `schemas/timeline.schema.json` — rational-FPS timelines; effects render at
  timeline frame indices, never wall-clock time.

### 2.3 Safe zones, motion profiles, reduced motion

- Safe zones: `youtube-lower-third`, `vertical-bottom` (schema enum). Effect
  `footprint` percentages describe the on-screen region an effect occupies;
  safe-zone collision policy is enforced from these fields.
- Motion profiles: `static`, `restrained`, `expressive`.
- Reduced motion: every effect declares one of `not-meaningful`,
  `static-fallback`, `unsupported` with a description. The native
  implementation must honour the same declared fallback.

### 2.4 Input schemas and preview fixtures

- Props validation samples: `fixtures/effects/props-fixtures.json` (valid and
  invalid props per registry effect). **Props schemas are frozen across the
  migration**: a v2 native effect accepts exactly the v1 `props_schema`.
- Preview fixtures: each registry entry declares `still` and `motion` fixture
  paths. Media bytes stay outside Git per `fixtures/README.md`; the checked-in
  contract is the JSON plus the path declarations.
- Project fixture format: `fixtures/README.md` (project package with
  `project.json`, `sources/manifest.json`, word-level transcript, rational-FPS
  `edit/timeline.json`, round-trip assertions).

## 3. Migration table — every renderer/effect has a native target

Rows are frozen in `fixtures/native-renderer/manifest.json` and re-checked by
`check_renderer_migration_manifest.py`, which fails if any registry effect on
the `remotion` or `hyperframes` renderer lacks a row.

| Family | Legacy | Native effect id | Golden fixture | Shipping runtime |
| --- | --- | --- | --- | --- |
| lower-third | `remotion:lower-third.identity-card.v1` | `lower-third.identity-card.v2` | `fixtures/native-renderer/lower-third` | `cutright-native` |
| stat-counter | `remotion:StatCounter` | `stat.counter.v2` | `fixtures/native-renderer/stat-counter` | `cutright-native` |
| quote-card | `remotion:quote-card.v1` | `quote-card.v2` | `fixtures/native-renderer/quote-card` | `cutright-native` |
| cta-card | `remotion:cta-end-card.v1` | `cta-end-card.v2` | `fixtures/native-renderer/cta-card` | `cutright-native` |
| captions | `ass:caption.bold-karaoke.v1` | `caption.bold-karaoke.v2` | `fixtures/native-renderer/captions` | `cutright-native` |
| hook-pullback | none (pre-v2, new native family) | `hook.pullback.v1` | `fixtures/native-renderer/hook-pullback` | `cutright-native` |
| punch-wave | none (pre-v2, new native family) | `punch.wave.v1` | `fixtures/native-renderer/punch-wave` | `cutright-native` |
| text-reveals | none (pre-v2, new native family) | `text.reveal.v1` | `fixtures/native-renderer/text-reveals` | `cutright-native` |
| audio-sync | none (pre-v2, new native family) | `audio.sync.v1` | `fixtures/native-renderer/audio-sync` | `cutright-native` |

Non-effect renderer values: `ffmpeg` and `ass` remain permitted encoding /
subtitle-burn tools invoked by the native runtime (no forbidden runtime
involved). The `remotion` and `hyperframes` enum values stay in the registry
schema only until the deletion criteria in §5 are met, then they are deleted.

Canonical row shape (implementation shape frozen by CR-V2-B1-021):

```json
{"legacy":"remotion:StatCounter","native_effect_id":"stat.counter.v2","golden_fixture":"fixtures/native-renderer/stat-counter","shipping_runtime":"cutright-native"}
```

## 4. Native golden comparisons

Each family gets a golden fixture directory under `fixtures/native-renderer/`
containing, once the native effect exists: a deterministic `props.json` (valid
under the frozen props schema), a declared frame list (timeline frame indices,
rational FPS), and approved reference outputs. Comparison method:

1. Render the native effect at the declared frames with fixed inputs; the
   render must be byte-reproducible across runs (no timestamps, sorted
   iteration, fixed seeds).
2. Compare each rendered frame against the approved reference (exact hash for
   stills; per-frame hash list for motion). Any mismatch fails the gate.
3. Render the declared `reduced_motion` variant and compare against the
   fallback reference. The fallback must match the registry description.

Family requirements (visual intent that must survive the technology change):

- **lower third** — identity card with name/title/accent color, restrained
  entry, `youtube-lower-third` safe-zone compliant footprint.
- **stat counter** — count-up numeral animation (expressive) ending on the
  exact final value; label and accent color from props.
- **quote card** — quote plus optional attribution and accent color,
  restrained fade-in, centred footprint.
- **CTA card** — headline plus optional subtext and accent color, restrained
  fade-in, end-card footprint.
- **captions** — bold karaoke highlight sweep timed to transcript word timings;
  highlight color and emphasis scale from props.
- **hook pull-back** — opening-hook scale/zoom pull-back attached to the cut
  plan's hook beat; must respect reframe-track keyframes.
- **punch wave** — beat-aligned punch/zoom wave on emphasis cuts from the cut
  plan; amplitude bounded by the restrained/expressive profile of the clip.
- **text reveals** — word/line reveal timing families for titles and overlays,
  deterministic per-word stagger derived from frame indices.
- **audio-synchronised effects** — any effect whose trigger or envelope comes
  from `audio-finish`/`finish-plan` or transcript timing; must lock to the
  audio clock, not the render wall clock.

## 5. Deletion criteria

The Remotion and HyperFrames stacks are deleted from CutRight only when **all**
of the following hold, evidenced in one release-audit commit:

1. **Native passes fixtures** — every row in
   `fixtures/native-renderer/manifest.json` has a passing golden comparison,
   including its reduced-motion variant, and
   `check_renderer_migration_manifest.py` exits 0.
2. **Projects migrate** — no project or registry entry renders through the
   `remotion` or `hyperframes` renderer value; `schemas/effects/registry.json`
   contains zero effects on those renderers.
3. **Clean release** — the shipping release and every runtime pack contain no
   Node, Chromium, Remotion, or HyperFrames runtime, verified against the
   forbidden package/binary lists in
   `imports/v2/dispositions/renderers.json`.

After all three: remove `remotion` and `hyperframes` from the
`effect-registry.schema.json` renderer enum, retire the v1 registry effect
ids, and move this document to archive status. Until then, the v1 effects and
their fixtures remain the compatibility contract.
