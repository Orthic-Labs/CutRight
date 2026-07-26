# Phase 3 — CutRight Studio Review Workspace

**Build spec for the implementation agent.** Written 2026-07-26 against `6dc5124`.
Design authority: `/designer` (app-UI branch + Tauri shell) and `/motion` (product register, webview).
Adrian's eyes approve the rendered result; nothing here overrides the taste gate.

---

## 0. Scope

Build the review surface that turns a finished `.video-project/` into something a human can judge:
every source clip in one place, playable; the two rough-cut variants comparable; the finals and QA
verdict visible; and an approve/reject decision recorded as canonical JSON.

**In scope:** `apps/studio/` frontend + its Tauri command layer, and three small engine additions
(§1) that the UI cannot correctly do in TypeScript.

**Out of scope — do not build:**
- Any button that runs `videoctl`. Studio reads artifacts and writes verdicts. The agent drives the
  engine. A "Re-render" button would make Studio a second engine driver and break the layer contract
  in `ARCHITECTURE-2026-07-26.md` §4.
- Timeline editing, boundary dragging, trimming. Read + judge only in this phase.
- Chat, LLM calls, network of any kind. Studio is offline, local-file only.
- Effects, captions styling, colour grading UI.

**Definition of done:** Adrian opens a real project, watches every clip, compares tight vs natural,
approves one, rejects a bad boundary, and `feedback/decisions.jsonl` contains those decisions —
without touching a terminal.

---

## 1. Engine prerequisites (do these first, in Rust)

Studio must not reimplement timeline arithmetic. Three additions make the UI a pure reader.

### P1 — Per-variant output transcripts

`remap_transcript` currently reads `edit/cut-plan.json` (whichever variant was built last) and writes
a single `edit/output-transcript.json`. The compare view needs both.

- Add `--variant <tight|natural>` to `videoctl transcript remap`.
- With the flag: read `edit/cut-plan-{variant}.json`, write `edit/output-transcript-{variant}.json`
  and `edit/captions-{variant}.srt`.
- Without the flag: current behaviour unchanged (compatibility).
- `videoctl edit render` should remap its own variant automatically after compiling the timeline, so
  the artifacts always exist alongside the MP4.

### P2 — Source-word provenance through the remap

`Word` loses its source identity during remap (`id` is regenerated as `ow_000142`), so the same
spoken word cannot be joined across two variants. Add one optional field:

```rust
// video-core/src/models.rs — Word
#[serde(default, skip_serializing_if = "Option::is_none")]
pub source_word_id: Option<String>,
```

Populate it in `remap_transcript` as **`{source_id}:{word_id}`** (e.g. `source-8f2c1a2b3c4d:w_000138`).
The compound form is mandatory: word ids restart at `w_000000` per source transcript
([video-providers/src/lib.rs:487](../crates/video-providers/src/lib.rs)), so the bare id collides
across clips in any multi-source project and would conflate two unrelated spoken words — silently
breaking the word-locked join. Leave it `None` everywhere else.
Tests: (1) round-trip keeps the compound id; (2) a legacy transcript without the field still
deserializes; (3) a two-source project produces no duplicate `source_word_id` values.

### P3 — A read-only project snapshot command

One Tauri-callable Rust function that walks a project and returns everything Studio needs in a single
call, so the frontend never globs the filesystem itself:

```rust
pub fn project_snapshot(project_path: &Path) -> Result<ProjectSnapshot, ProjectError>
```

Returns (all paths absolute, all optional members `None` when the pipeline stage has not run):

```jsonc
{
  "schema_version": 1,
  "project_path": "/abs/path/My.video-project",
  "manifest": { /* project.json */ },
  "generated_at": "2026-07-26T09:14:22Z",
  "sources": [ { /* SourceEntry */, "file_present": true,
                 "transcript": "analysis/transcripts/<source_id>.json|null",
                 "stages": { "transcribed": true, "analyzed": true,
                             "in_candidates": true, "in_cut": true },
                 "waveform_png": "…|null", "poster_jpg": "…|null" } ],
  "stages": { "ingested": true, "transcribed": true, "analyzed": true,
              "candidates": true, "rough_cut": true, "final": false, "qa": false },
  "variants": [ { "id": "natural", "mp4": "…", "mp4_mtime": "RFC3339", "fps": 29.97,
                  "cut_plan": {…}, "output_transcript": "…", "srt": "…",
                  "segment_count": 42, "duration_ms": 512340 } ],
  "finals": [ { "preset": "youtube", "aspect": "16:9", "mp4": "…", "mp4_mtime": "RFC3339",
                "fps": 29.97, "duration_ms": 511900 } ],
  "qa": { /* qa/report.json */ } ,
  "bench": { "decision": "unresolved|heardright|whisperx", "report": "…" },
  "reframe_plan": { /* analysis/reframe-plan.json */ },
  "decisions_path": "/abs/path/My.video-project/feedback/decisions.jsonl"
}
```

Missing files are `null`, never an error — a half-built project must open. Add a test that a project
containing only `project.json` returns a snapshot with every pipeline stage `false`.

Path convention (used by the snapshot AND `decisions.jsonl` `subject`): "project-relative" means
relative to the `.video-project/` root, forward slashes, no leading `./`. Snapshot media paths are
absolute (they feed `convertFileSrc`); decision `subject` values are project-relative.

Per-source `stages` are computed in Rust (transcript file exists; `vad-<id>.json` exists; any
candidate references the source; any cut-plan segment references the source) — the ledger pips
(§7.2) read them directly and never derive pipeline state in TypeScript. Removed-gap data needs no
new field: the output timeline is contiguous by construction, so the segment strip derives source
gaps from consecutive `cut_plan` segments' `source_end_ms`→`source_start_ms` plus the source
duration — both already in the snapshot.

Cost rules: `file_present` is a stat, never a hash — the snapshot must never re-hash a multi-GB
source on open (hash verification is the explicit action in §7.2). `fps` is the rendered MP4's
probed frame rate; it drives frame stepping (§11). `mp4_mtime` is the staleness signal (§9.1).

---

## 2. Task truth

> CutRight Studio helps Adrian complete **per-project cut review** by **playing every source and both
> pacing variants against the same spoken word**, and the interface must make **which variant he
> approved, and which boundaries he rejected, obvious and recorded**.

---

## 3. Workspace signature (hard gate) — "Word-Locked Compare"

The invented, product-specific mechanism this app is built around.

**What it is:** the two rough-cut variants play in one viewer with a single shared cursor that is a
*word*, not a timestamp. Press `A` and the video swaps from natural to tight and lands on **the same
spoken word** — not the same clock position. The pacing difference becomes directly perceptible
because the content position is held constant while the timing around it changes.

**Why it passes the gate:**
- *Task-truth* — the only real question about tight-vs-natural is what the pacing does to the same
  sentence. Clock-synced A/B answers a question nobody asked.
- *Non-transplant* — it is impossible without word-level remapped transcripts carrying source
  provenance (P1+P2). No other app in the suite could use this.
- *Stateful* — the cursor shows current word, its source word id, which variant is live, and whether
  that word survives in the other variant.
- *Efficient* — one keystroke, no seeking, no mental arithmetic; repeated hundreds of times per review.
- *Nameable* — "word-locked compare".

**The algorithm (pure lookup, no math in TS):**

1. Playing variant `A` at output time `t`. Find word `w` in `output-transcript-A.json` where
   `w.start_ms <= t < w.end_ms`; if `t` falls in a gap, take the nearest **preceding** word
   (ties round down to the earlier word).
2. On swap, look up variant `B`'s transcript for the word with the same `source_word_id`.
3. Found → seek `B` to that word's `start_ms`, resume playing if it was playing.
4. **Not found** (that content was dropped in `B`) → seek to the nearest **following** word that
   exists in `B`, and show a persistent inline marker in the transcript rail: *"cut in tight"* with
   the dropped word count. This is a feature: it is exactly the pacing difference under review.

Edge rules (each gets a Vitest case, §14; Vitest runs in jsdom with `@tauri-apps/api` `invoke`
mocked and transcripts as fixture JSON — no Tauri runtime, no video element needed for the lookup
logic):
- `t` before the first word → cursor state is `preroll`: no word underlined, swap seeks `B` to 0.
- Step 4 with no following word in `B` (content dropped at the tail) → seek to `B`'s **last** word,
  pause, marker rendered at the end of the rail.
- `B` has zero words (every candidate dropped) → the swap is refused with an inline notice
  "tight has no content"; the variant chip for `B` renders disabled. Never a crash, never a black
  viewer with no cursor.

**Second mechanism — the Clip Ledger.** The left rail is a queue ledger: one row per source, each
showing its pipeline state (ingested → transcribed → analyzed → in cut), duration, resolution, HDR
flag, and whether its hash still matches the manifest. It is the "all clips in one place" surface and
doubles as the integrity display.

---

## 4. Information architecture

Single window. No router, no page navigation — a persistent three-zone shell with a switchable viewer.

```
┌───────────────────────────────────────────────────────────────────────┐
│  TITLEBAR  (overlay traffic lights ⟵ macOS)   project name    ⌘K help │
├────────────┬──────────────────────────────────────┬───────────────────┤
│            │                                      │                   │
│  CLIP      │            VIEWER                    │   INSPECTOR       │
│  LEDGER    │   (video + transport + compare bar)  │   (context rail)  │
│  (rail)    │                                      │                   │
│  240px     │            flexible                  │      320px        │
│            │                                      │                   │
├────────────┴──────────────────────────────────────┴───────────────────┤
│  STATUS STRIP — pipeline progress · bench verdict · QA verdict · gates    │
└───────────────────────────────────────────────────────────────────────┘
```

Naming rule for the codebase: the centre region is the **Viewer** (components `Viewer*`);
"stage(s)" refers ONLY to pipeline progress (`snapshot.stages`, ledger stage pips); "Phase" refers
ONLY to the build-plan phases. Do not mix the three.

**Viewer modes** (segmented control at the top of the viewer, not a nav):

| Mode | Shows | Primary action |
|---|---|---|
| `Sources` | One selected source clip, full player, its waveform, its transcript | Scrub, read |
| `Compare` | Word-locked A/B of tight vs natural + segment strip | Approve a variant |
| `Finals` | Rendered deliverables per preset, side by side with aspect framing | Approve the final |
| `QA` | The QA report as a checklist with evidence links | Read, acknowledge |

**Inspector contents** are mode-dependent: source metadata (Sources), the transcript rail with the
live word cursor + segment list (Compare), preset/loudness/dimension facts (Finals), failing-check
detail (QA).

**Default mode on open = the most-progressed available mode:** finals exist → `Finals`; else rough
cuts exist → `Compare` (the signature surface); else `Sources`. Reopening a project restores the
last active mode if it is still available.

**Empty/incomplete is the default state, not an edge case.** A project with only sources opens on
`Sources`; `Compare` is disabled with the reason ("no rough cuts yet — run `videoctl edit render`")
and the exact command shown as selectable text.

---

## 5. Visual registers — three explored, one locked

Per `/designer` Phase 3, using the same workspace signature.

| | **A · Console** | **B · Screening Room** | **C · Bench** ← **LOCKED** |
|---|---|---|---|
| Base | `#0B0B0C` near-black, dense | `#000` true black, chrome dissolves | `#0B0B0C` chrome, `#000` viewer |
| Density | High — 28px rows, 12px type | Very low — video is 80% of the window | Mixed — dense rails, calm viewer |
| Type | Mono-forward labels | Large editorial display, sparse | Grotesque UI, mono only for data |
| Motion | Near-zero, instant | Slow luxurious fades | Precise, 3 patterns |
| Best fit | Batch triage of many projects | Watching a finished film | Judging a cut against evidence |
| Risk | Reads cold and technical — the exact CodeRight lesson (JetBrains-Mono-as-body made the app feel like an IDE, retired 2026-06-29) | Evidence is a click away; comparison becomes slow | Must hold two densities without looking like two apps |

**Locked: C · Bench.** The task is judgment-by-comparison, which needs the video large *and* the
evidence adjacent. Console makes the video secondary to data it does not need; Screening Room hides
the transcript and segment list that justify the verdict. Bench is the only register that serves the
actual decision. The two densities are reconciled by one rule: **the viewer is black and quiet, the
rails are dark-grey and dense** — the video never shares a surface colour with chrome.

---

## 6. Design tokens

### 6.1 Colour — CutRight identity (PROPOSED, Adrian locks the accent)

CutRight is not yet in the Right Suite identity table in `.claude/rules/brands.md`. This proposal
follows the suite's locked system rules (Tanker wordmark, CamelCase, "Right" in accent, dark+light,
WCAG AA both bases). **Accent hue is Adrian's call — to override, change `--accent` / `--accent-light`
and nothing else needs to move.**

**Accent — marker magenta.** `#FF4FB8` dark / `#B4007C` light.
Rationale: it is the only strong hue not already claimed in the suite (VR blue 202°, VoiceRight
violet 255°, CR red 349°, MR red 357°, HR ember 14°, SR gold 40°), it sits 55° from violet and 30°+
from both reds, and — decisively — **green and red must stay purely semantic in this app** because
approve/reject is the core interaction. A green or red brand accent would collide with the verdict
language. Magenta is also the native marker colour of edit tooling.

Measured contrast (computed, not estimated): `#FF4FB8` on `#0B0B0C` = **6.62:1**; `#B4007C` on
`#F7F3EC` = **5.88:1**; white on `#B4007C` fill = **6.50:1**. Full token sweep also passes AA —
body `17.43:1` dark / `15.71:1` light, muted `7.02:1` dark (`6.56:1` on `--surface`) / `5.31:1`
light, approved `10.23:1`, rejected `7.11:1`. Lowest pair in the system is 5.31:1.

**Base — neutral, by product truth.** One sentence of physical scene, as the App Colour Gate demands:
*Adrian reviews footage at night on a Mac in a dim room, judging whether the picture itself looks
right.* A tinted or bright surround biases perceived exposure and colour of the video — this is the
one app in the suite where dark neutral is a technical requirement, not a mood. The viewer surround is true
black because that is the correct surround for video review; the chrome is lifted off it.

```css
:root {
  /* viewer surround — video only, never chrome */
  --viewer-bg:        #000000;

  /* dark theme (default) */
  --bg:               #0B0B0C;
  --surface:          #141416;   /* rails, cards */
  --elevated:         #1D1D20;   /* hovered card, popover, active row */
  --border:           #2A2A2E;
  --border-strong:    #3A3A40;
  --text:             #F2F1EF;
  --muted:            #9A9A9E;
  --faint:            #6A6A70;
  --accent:           #FF4FB8;
  --accent-ink:       #12000A;   /* text on an accent fill */

  /* semantic — never used as identity */
  --approved:         #34D399;
  --rejected:         #F87171;
  --pending:          #9A9A9E;
  --warn:             #FBBF24;
  --focus:            #FF4FB8;

  /* type */
  --font-display: "Tanker", ui-sans-serif, system-ui, sans-serif;
  --font-ui:      "Geist", ui-sans-serif, -apple-system, system-ui, sans-serif;
  --font-mono:    "Spline Sans Mono", ui-monospace, "SF Mono", monospace;

  /* space — 4px base */
  --s1: 4px;  --s2: 8px;  --s3: 12px; --s4: 16px;
  --s5: 24px; --s6: 32px; --s7: 48px;

  --radius:    8px;
  --radius-sm: 5px;

  /* motion — see §8 */
  --dur-micro: 120ms;
  --dur-state: 180ms;
  --dur-swap:  220ms;
  --ease-out:  cubic-bezier(0.22, 1, 0.36, 1);
}

:root[data-theme="light"] {
  --bg: #F7F3EC; --surface: #FFFDF8; --elevated: #FFFFFF;
  --border: #E2DAD1; --border-strong: #CFC5B9;
  --text: #1A1A1C; --muted: #64646A; --faint: #8A8A90;
  --accent: #B4007C; --accent-ink: #FFFFFF;
  --approved: #047857; --rejected: #B91C1C; --warn: #92400E;
  /* --viewer-bg stays #000000 in both themes: video surround is not a theme choice.
     Light theme adds a 1px inset border (--border) around the viewer so the black
     region reads as a deliberate surface, not a hole — flag to Adrian at taste gate. */
}
```

Both themes ship. Default to dark; honour `prefers-color-scheme` on first run; persist the user's
explicit toggle in `localStorage`.

**Colour rules enforced in review:**
- Accent appears on: selection, focus ring, the live-variant chip, the wordmark's "Right". Nowhere else.
- Verdict state is never colour-only — every verdict carries an icon **and** a text label.
- Repeated controls keep identical dimensions across every state (no border-width jumps; use inset
  box-shadow for selection, not a wider border).

### 6.2 Type

Vendor all three as `woff2` under `apps/studio/src/assets/fonts/` and `@font-face` them locally —
the CSP is `default-src 'self'` and there is no network. Licences differ and `LICENSES.md` must
name each correctly: **Tanker — Fontshare Free Font License (FFL)**, not OFL; **Geist — SIL OFL 1.1**;
**Spline Sans Mono — SIL OFL 1.1**. All three permit app embedding; the FFL forbids reselling
fonts, redistributing them on font platforms, or renaming derivatives — app-bundle embedding is
expressly permitted (verified 2026-07-26 against the FFL and Fontshare usage summaries; the suite
already ships Tanker on five public sites, a more exposed use than a bundle).

| Role | Face | Size / weight | Notes |
|---|---|---|---|
| Wordmark | Tanker | 20px | `CutRight`, "Right" in `--accent`. Titlebar only. |
| Stage mode labels | Geist 500 | 13px, 0.02em | Segmented control |
| Body / UI | Geist 400 | 13px | Default |
| Rail row title | Geist 500 | 13px | |
| Rail row meta | Spline Sans Mono 400 | 11px, `--muted` | Duration, resolution, ids |
| Timecode / word id / hash | Spline Sans Mono 400 | 11–12px, tabular | **Always mono, always tabular-nums** |
| Transcript rail | Geist 400 | 14px / 1.55 | Reading surface — larger than UI |
| Section eyebrow | Geist 600 | 10px, 0.08em, uppercase, `--faint` | |

Tanker is display-only. Never set body copy or a data value in Tanker.

---

## 7. Component specification

### 7.1 Titlebar

- `titleBarStyle: "Overlay"` is already set — keep native traffic lights on macOS, never fake them.
- Height 38px, `data-tauri-drag-region` on the static bar element only (children need their own
  attribute; do not transform the drag region).
- Left: 78px reserved padding for traffic lights, then the `CutRight` wordmark.
- Centre: project name (Geist 500, 13px) + `.video-project` suffix in `--faint`.
- Right: theme toggle, `?` shortcuts button.
- Windows note: if a Windows build is ever added, custom caption buttons go right — per CLAUDE.md §14.

### 7.2 Clip Ledger (left rail, 240px)

One row per source from `snapshot.sources`:

```
┌────────────────────────────────┐
│ ▎ [poster 40×24]  Clip 01      │   ▎ = 2px accent bar when selected
│                   04:12 · 4K   │   mono, --muted
│                   ●●●●○  HDR   │   pipeline pips + flags
└────────────────────────────────┘
```

- **Pipeline pips**: five 4px dots = ingested, transcribed, analyzed, in-candidates, in-cut. Filled
  `--accent` when true, `--border-strong` when false. Tooltip names each.
- **HDR chip** when `is_hdr` — this matters because it changes the render path.
- **Integrity**: two tiers. Default (from `snapshot.sources[].file_present`, a stat): a missing
  file shows a `--rejected` left bar + "file missing". Full verification is an explicit
  **Verify sources** action in the inspector — it calls the `verify_sources` command (§9.1), which
  re-hashes in Rust with per-source progress; a mismatch shows the red bar + "source changed" and
  a banner (§12). Never auto-hash multi-GB files on open.
- Selection: `--elevated` background + 2px `--accent` left bar + `aria-selected`.
- Row height 56px, fixed, identical in every state.
- Header of the rail: `SOURCES` eyebrow + count; footer: total duration (mono).
- Keyboard: `↑`/`↓` move selection, `1`–`9` jump to source N.

### 7.3 Viewer — Sources mode

- `<video>` on `--viewer-bg`, `object-fit: contain`, max height `calc(100vh - 260px)`.
- Below it: the waveform PNG from `cache/waveforms/` or `analysis/evidence/waveforms/`, 48px tall,
  with a 1px `--accent` playhead line positioned by percentage. Click-to-seek on the waveform.
- Transport strip: play/pause, current/total timecode (mono, tabular), 1px scrub track with an 11px
  accent thumb, volume, `⌫` reset.
- If a source has no waveform artifact, render the strip without it — never a broken image.

### 7.4 Viewer — Compare mode (the signature surface)

```
┌──────────────────────────────────────────────────────────────┐
│  [ NATURAL ]  [ tight ]        word-locked ⌥          A ⇄     │  variant chips
├──────────────────────────────────────────────────────────────┤
│                                                              │
│                     ■ video (--viewer-bg) ■                      │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ ▓▓▓▓░▓▓▓▓▓░░▓▓▓▓▓▓░▓▓▓▓  segment strip (kept vs removed)     │
├──────────────────────────────────────────────────────────────┤
│  ▶  00:01:12.480 / 00:08:31.900     [✓ Approve] [✗ Reject]   │
└──────────────────────────────────────────────────────────────┘
```

- **Variant chips**: the live one is an accent fill with `--accent-ink` text; the other is a
  `--border` outline. Both always visible, fixed width, so the swap never reflows.
- **Two `<video>` elements**, stacked in the same grid cell, both mounted for the whole Compare
  session — never destroyed or recreated on swap. The inactive one is `opacity: 0` +
  `pointer-events: none` — NOT `visibility: hidden` (a hidden element cannot take part in an
  opacity crossfade, and WKWebView may not paint it). Swap algorithm, in this order:
  1. Inactive video is always `muted = true`, `paused`, `opacity: 0`, `preload="metadata"` — the
     pre-seek buffers around the target frame without holding a full second buffer (two 4K streams
     fully buffered is a WKWebView memory risk).
  2. On every word-cursor change, pre-seek the inactive video to the word-locked target
     (`currentTime = target`) and let its `seeked` event mark it warm.
  3. On swap: `pause()` the outgoing video FIRST. If the incoming video is warm
     (`seeked` fired and `readyState >= 2`), proceed; otherwise proceed anyway over its last decoded
     frame (never black) and correct `currentTime` on the pending `seeked`.
  4. Crossfade opacities (out 1→0, in 0→1, 220ms) while BOTH are visible, unmute the incoming,
     play if it was playing; on `transitionend`, set the outgoing to `pointer-events: none` and
     re-assert `muted`.
  Assumed ceiling: two simultaneous ≤4K30 rough cuts; beyond that behaviour is untested — say so in
  the README rather than engineering for it.
- **Bench chip, per-variant**: while `snapshot.bench.decision == "unresolved"`, each variant chip
  carries a small `--warn` dot with tooltip "word timestamps unverified (bench unresolved)". This is
  distinct from the global status-strip chip because it changes what an approval MEANS — the judge
  must see it at the point of judgment, and the decision records `bench_resolved: false` (§10).
- **Segment strip**: full output duration as a bar; each `TimelineSegment` a kept block
  (`--border-strong`, or `--accent` when it is the current segment); the source gaps between them
  rendered as thin `--bg` slots. Hovering a block shows `segment-014 · 3.2s · natural:hook`.
  Clicking seeks to it. This is where "what got removed" becomes visible.
- **Word cursor** lives in the inspector transcript rail (§7.6) but is driven from here.
- **Verdict buttons**: `Approve` is an accent-outline button that fills on hover;
  `Reject` is `--border` outline, text `--rejected` on hover. Both require a reason chip before they
  commit (§7.7) — one click opens the reason row, second click commits.

### 7.5 Viewer — Finals mode

- One card per entry in `snapshot.finals`. Cards sit side by side; each renders its video inside a
  frame matching its true aspect (16:9 wide, 9:16 tall and narrower) so the shape is legible at a
  glance.
- Under each: preset id, dimensions, duration, and stream facts from the QA report. Mono, tabular.
  Loudness is NOT displayed — `qa/report.json` does not record it today
  ([lib.rs:1505](../crates/video-project/src/lib.rs)); the −14 LUFS target is enforced at render
  time by `measured_loudnorm_filter`. If the engine later adds a loudness check to QA, surface it
  then. Do not probe audio in the frontend.
- A vertical final whose `reframe_plan` anchor has `"strategy": "manual_anchor_required"` shows a
  `--warn` chip: *"reframe anchor unreviewed"*, because §render_final gates on that.
- Approve/reject per final, same two-step commit.

### 7.6 Inspector — transcript rail (Compare mode)

The reading surface, and the reason word-locked compare is legible.

- Words from `output-transcript-{variant}.json`, rendered inline, wrapping, 14px/1.55.
- The **current word** gets an accent underline (2px, `border-bottom`) — not a background highlight,
  which would strobe at speaking rate.
- Words are click-to-seek.
- Segment boundaries insert a thin `--border` rule with the segment id in mono `--faint`.
- **Cross-variant markers**: where the other variant dropped content, insert an inline chip
  `— 6 words cut in tight —` in `--faint`, mono, 10px. This is the pacing delta made visible.
- Auto-scroll follows the cursor. "Manual scroll" that disengages it = wheel/touchpad/scrollbar
  input inside the rail ONLY. Click-to-seek on a word, `←`/`→` word keys, and Tab focus moves all
  RE-engage follow (the user navigated; following is what they want). A small "follow" button also
  re-engages. Re-engagement uses `scrollIntoView({ block: "nearest" })`, throttled to word changes.
- Under the transcript: the current word's `source_word_id`, `start_ms`, `end_ms` in mono — the raw
  evidence for a boundary complaint.

### 7.7 Verdict control + reason chips

Approve/reject must capture *why*, because `decisions.jsonl` is preference-learning seed data
(ARCHITECTURE §13) and a verdict with no reason teaches nothing.

- Click `Approve` → a reason row expands with chips: `pacing`, `word edges`, `energy`,
  `length`, `other`. Selecting one commits immediately. `Esc` cancels.
- Click `Reject` → chips: `clipped word`, `too tight`, `too loose`, `bad boundary`,
  `wrong take`, `other`. Same commit.
- `other` reveals a single-line text input (max 200 chars).
- **Boundary-level reject**: right-click (or `X`) on a segment block files a `segment_flag` against
  that segment id and the current word — the highest-value correction signal in the whole app.
- **`X` dispatch rule**: `X` targets the focused element. Segment block focused → `segment_flag`;
  otherwise → variant-level reject. One key, focus-scoped, never ambiguous.
- After commit the control becomes a verdict badge showing the chosen state, reason, and time.
  Clicking the badge offers `Change verdict` (appends a new decision; never edits history).

### 7.8 Status strip (bottom, 30px)

Left → right, mono 11px, each a small state dot + label:
- Pipeline: `ingest ✓ transcribe ✓ analyze ✓ cut ✓ final ✓ qa ✓` — filled/hollow dots.
- **Bench**: `bench: unresolved` in `--warn`, or `bench: heardright` in `--approved`. Hovering
  explains: *timestamp authority is not granted until three real capture clips pass the benchmark*.
- **QA**: pass/fail from `qa/report.json`.
- Right: decisions — `this session: N · total: M`. A session = one project-open lifetime (switching
  or reopening a project resets N; M comes from the `read_decisions` replay).

The strip must be internally consistent — a failed QA can never sit next to a healthy headline, and
any failure names the failing subsystem plus what to run.

---

## 8. Motion plan (product register)

Full artifact at `docs/artifacts/motion-plan-studio.md`. Summary:

- **Register:** product. **Language:** Precision — 120–220ms, sharp ease-out
  (`cubic-bezier(0.22,1,0.36,1)`), tiny distances, high density. Calibration: Linear, Nothing.
- **Persistent objects:** (1) the word cursor — survives every seek, swap, and mode change;
  (2) the variant chip pair; (3) the verdict badge on each artifact.
- **Exactly three patterns**, all CSS, **zero animation JS** (budget 0KB of the 50KB allowance):

| # | Pattern | Where | Spec | Earns its keep because |
|---|---|---|---|---|
| 1 | **Variant crossfade** | A/B swap | Outgoing `opacity 1→0`, incoming `0→1`, 220ms, ease-out, overlapping | A hard cut between two videos at the same word is indistinguishable from a stutter. The fade is the only signal that the swap happened. |
| 2 | **Verdict commit** | Approve/reject badge | Badge `scale(0.97)→1` + `opacity 0→1`, 180ms; reason row height 0→auto, 180ms | Confirms an intentional, recorded action landed. Without it users double-click and file duplicate decisions. |
| 3 | **Selection settle** | Ledger rows, segment blocks | `background-color` + `box-shadow` 120ms. **No transform, no slide.** | Deliberately the weakest — it passed the restraint test only in its minimal form. |

- **Restraint test applied:** removing #1 makes the signature interaction ambiguous; removing #2
  makes an irreversible-feeling action feel unregistered; removing #3 costs almost nothing, which is
  precisely why it was cut down to a colour transition. Anything beyond these three is decoration —
  do not add page transitions, stagger, parallax, or scroll animation to this app.
- **Reduced motion:** `@media (prefers-reduced-motion: reduce)` sets all three to `0ms`. The variant
  swap still changes the chip fill and the transcript cursor; the verdict still changes icon, colour,
  and label. **Motion is never the sole carrier of a state change** — this doubles as the
  engine-gated fallback per `motion/webview.md` §2.
- **Not used, deliberately:** View Transitions API (Safari 18+ gate, no fallback worth the branch),
  `backdrop-filter` (WKWebView cost, and this design has no translucent surfaces), `will-change`
  (add only if profiling shows a first-frame hitch, then remove).
- **Frame budget:** the only continuously-updating elements are the playhead line, the word cursor,
  and the segment strip's current-segment highlight. Drive all three from ONE
  `requestAnimationFrame` loop reading `video.currentTime`, not from `timeupdate` (~4Hz, looks
  broken). One loop for the whole app; cancel it on pause and unmount; it is the only per-frame
  DOM writer.

`docs/artifacts/motion-gate-studio.json` must be written **after** prototyping the variant crossfade
in the built app — verdict `pass` requires recorded evidence (the swap observed, the reduced variant
captured). Writing that gate from this plan alone is forbidden.

---

## 9. Tauri integration

### 9.1 Commands (Rust, `apps/studio/src-tauri/src/main.rs`)

Exactly six. Everything else is a filesystem read the frontend must not do.

| Command | Signature | Notes |
|---|---|---|
| `pick_project` | `() -> Result<Option<String>, String>` | Native folder picker (`tauri-plugin-dialog`). Validates `project.json` exists, then extends the asset-protocol scope (§9.2). |
| `read_snapshot` | `(path: String) -> Result<ProjectSnapshot, String>` | Wraps P3. The only project read. Also invoked by the manual refresh (`⌘R`). |
| `read_transcript` | `(path: String, variant: String) -> Result<Transcript, String>` | Reads `edit/output-transcript-{variant}.json`. Separate because it is large and mode-specific. Size assumption: transcripts to ~10k words load whole; no streaming loader. |
| `append_decision` | `(path: String, decision: Decision) -> Result<(), String>` | **The only write.** Appends one line to `feedback/decisions.jsonl`, creating the file if absent. |
| `read_decisions` | `(path: String) -> Result<Vec<Decision>, String>` | Replays existing verdicts so reopening a project shows prior state. Skips malformed lines rather than failing. |
| `verify_sources` | `(path: String) -> Result<Vec<SourceCheck>, String>` | Explicit re-hash (blake3, in Rust) of every registered source vs the manifest; emits per-source progress events. Invoked only by the §7.2 Verify action. |

**Server-side validation in `append_decision`** (the schema lives in §10; the command enforces it —
otherwise the frontend drifts and the audit trail corrupts silently):
`kind ∈ {variant_verdict, final_verdict, segment_flag, qa_ack, session_open}`; `reason` must be in
the §10 vocabulary for that `kind`; `verdict` null iff `kind == "session_open"`; `variant` must be
one of the project's actual variant ids (from the cut plans on disk) when present; `subject` for
`session_open` is the literal string `"project"`; `note` null or ≤200 chars and only with
`reason == "other"`; `ts` RFC3339; `playhead_ms ≥ 0`; `subject` project-relative (no `..`, no
absolute path); `word_id` / `source_word_id` shape-checked when present. Reject with a named field
error. `read_decisions` returns `{ decisions, skipped }` so the UI can surface "N malformed lines
ignored" instead of hiding corruption.

**Hard boundary — all six commands:** every command canonicalizes its `path` argument (resolving
symlinks — a symlinked `feedback/` must not let a write escape the project) and refuses paths whose
canonical form is not a directory containing `project.json` (reads) or not inside that project
directory (writes). `append_decision` writes only `feedback/decisions.jsonl`. Add traversal
tests (`../../etc/x`) for the write AND the read commands. Studio never modifies `sources/`.

**Staleness:** the agent may re-render while Studio is open; Studio does NOT watch the filesystem.
Every decision records `snapshot_generated_at`. On any `read_snapshot`, if a variant/final `mp4_mtime`
is newer than a prior decision's `snapshot_generated_at` for that subject, the verdict badge gains a
`--warn` "artifact changed since verdict" chip. `⌘R` re-reads the snapshot; that is the whole
refresh model.

### 9.2 Asset protocol — the one real trap

Local MP4s will be blocked by the current CSP. Required `tauri.conf.json` changes:

```jsonc
{
  "app": {
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; img-src 'self' asset: http://asset.localhost data:; media-src 'self' asset: http://asset.localhost",
      "assetProtocol": { "enable": true, "scope": { "allow": [], "deny": [] } }
    }
  }
}
```

`style-src 'unsafe-inline'` is required (already shipped in the current conf): React style props and
the rAF loop's `style.transform` playhead writes are inline styles. Keep it; do not add
`script-src 'unsafe-inline'`.

**The static allow list is empty on purpose.** A fixed `$HOME/**` is both too broad (grants the
webview the whole home directory) and too narrow (this workspace itself lives on `/Volumes/D`, and
real projects will too). Instead `pick_project` extends the scope at runtime to exactly (a) the
chosen project directory and (b) the parent directories of each registered source (sources are
absolute paths outside the project). Tauri v2 exposes runtime asset-scope extension on the app
handle (`asset_protocol_scope()` / `allow_directory`); verify the exact API name against the
installed `tauri` 2.x crate and note any deviation in the PR. Prefer exact `allow_file` grants for
each registered source over parent-directory grants where the runtime API supports it. Runtime
grants do not persist across launches: the one-click reopen path (§9.3) must re-grant after
validating the project, exactly as `pick_project` does. Result: the webview can read the opened
project and its media — nothing else.

Build config this section implies (all currently absent from `apps/studio`): the `tauri` crate's
`protocol-asset` feature, `tauri-plugin-dialog`, `tauri-plugin-window-state`, and
`"bundle": { "active": true }` with macOS targets — §14's built-app gates cannot run against a
dev-server-only config.

Frontend must load every media path through `convertFileSrc()` from `@tauri-apps/api/core` —
never a bare `file://` path. The asset URL differs by OS (`asset://localhost` on macOS,
`http://asset.localhost` on Windows), which is why both appear in the CSP.

Verify the exact permission/capability strings against the installed `@tauri-apps/cli` 2.11 rather
than copying them from memory; if the scope shape differs in that version, follow the installed
version's schema and note the deviation in the PR.

### 9.3 Platform

macOS/WKWebView is the only target; Windows and Linux builds are explicitly out of scope for this
phase (do not hedge the code with per-OS branches). Record the minimum macOS in the README. Test in
the built app, not only in `qa:browser` — a Chrome run is a frontend baseline, not WKWebView
evidence. Pin toolchain in `apps/studio/package.json`: `"packageManager": "pnpm@11.16.0"` (already
present) plus an `"engines"` entry for the Node major used at build time.

Persistence: `localStorage` keeps `{ lastProjectPath, lastMode, theme }`; window size/position via
`tauri-plugin-window-state`. On launch with a stored `lastProjectPath`, offer one-click reopen on
the empty state — do not auto-open (the file may be gone or on an unmounted volume).

---

## 10. `feedback/decisions.jsonl` — canonical schema

One JSON object per line, append-only, never rewritten. Every UI adjustment writes here; there is no
hidden UI state.

```jsonc
{
  "schema_version": 1,
  "ts": "2026-07-26T09:14:22.481Z",     // RFC3339 UTC
  "project_id": "project-8f2c…",         // from project.json
  "kind": "variant_verdict",             // see table
  "verdict": "approved",                 // approved | rejected | acknowledged | null
                                          // null ONLY for kind == "session_open"
  "reason": "pacing",                    // controlled vocabulary per kind
  "note": null,                          // free text, ≤200 chars, only when reason == "other"
  "subject": "render/rough-cuts/natural.mp4",  // project-relative (§1 P3 convention)
  "variant": "natural",                  // null when not variant-scoped
  "segment_id": null,                    // set for segment_flag
  "word_id": "ow_000142",                // output word at the playhead
  "source_word_id": "source-8f2c1a2b3c4d:w_000138",  // compound id (§1 P2) — joins across variants
  "playhead_ms": 72480,
  "bench_resolved": true,                // false when bench.decision was "unresolved" at verdict time
  "snapshot_generated_at": "2026-07-26T09:12:00Z",  // staleness join (§9.1)
  "app_version": "0.2.0"                 // single source: apps/studio/package.json "version",
                                          // injected at build via Vite define — never hardcoded
}
```

| `kind` | Subject | Allowed `reason` |
|---|---|---|
| `variant_verdict` | a rough-cut mp4 | `pacing`, `word_edges`, `energy`, `length`, `other` |
| `final_verdict` | a final mp4 | `looks_right`, `captions`, `loudness`, `framing`, `other` |
| `segment_flag` | a segment id | `clipped_word`, `too_tight`, `too_loose`, `bad_boundary`, `wrong_take`, `other` |
| `qa_ack` | `qa/report.json` | `reviewed` |
| `session_open` | project path | `opened` (and `verdict` is `null` — the only kind where it is) |

Changing a verdict appends a new record. History is never edited — that is the audit trail and the
learning signal both.

---

## 11. Keyboard model

Every repeated action has a key. Show them in a `?` sheet.

| Key | Action |
|---|---|
| `Space` | Play / pause |
| `J` `K` `L` | `J` seek −2s (hold repeats) · `K` pause · `L` play; repeat presses cycle 1×→2×→4×. WKWebView does not support negative `playbackRate`, so there is no smooth reverse — do not attempt it |
| `A` | Toggle variant, word-locked |
| `←` `→` | Previous / next word |
| `⇧←` `⇧→` | Previous / next segment |
| `,` `.` | Step one frame back / forward — frame duration = `1 / fps` from the active MP4's snapshot entry, never a hardcoded rate |
| `Y` | Approve (opens reason chips) |
| `X` | Reject (opens reason chips) |
| `1`–`9` | Select source N (sources 10+ via `⌘K` palette — the documented escape hatch) |
| `⌘1`–`⌘4` | Sources / Compare / Finals / QA |
| `⌘K` | Command palette (mode switch, source jump, verdicts) |
| `?` | Shortcut sheet |
| `Esc` | Cancel reason row / close sheet |

Focus must be visible on every interactive element (2px `--focus` ring, 2px offset). No animation may
trap focus or remove an element from the accessibility tree.

---

## 12. Required states (hard stop — all must exist before review)

| State | Requirement |
|---|---|
| Loading | Skeleton rails + "reading project…". Never a spinner over a blank window. |
| Empty — no project | Centred: wordmark, one line, `Open project…` button, and the `videoctl project init` command as selectable text. |
| Empty — no rough cuts | `Compare` disabled with the reason and the exact command to run. |
| Empty — no finals | Same pattern in `Finals`. |
| Error — unreadable project | Name the file that failed and the parse error; offer `Open another`. |
| Error — missing media | Card shows "file missing" with the expected path; the rest of the app still works. |
| Error — unplayable media | `<video>` fired `error` although the file exists: card shows "unsupported codec" + the path + a note that WKWebView decodes fewer formats than Chrome and the source may need transcoding. Distinct from missing. |
| Error — source hash changed | (Only after an explicit Verify run, §7.2.) Red ledger row + a full-width banner at the top of the viewer, dismissible, persisting until the next snapshot; names the file and explains re-ingest. |
| Success | Verdict badge with state, reason, timestamp. |
| Disabled | 40% opacity + `aria-disabled` + tooltip giving the reason. Never a silent dead control. |
| Focus | Visible ring, every control, both themes. |
| Hover / press | `--elevated` on hover; press is a 40ms opacity dip, no transform. |
| Selected | Accent left bar + `--elevated` + `aria-selected`. |
| Long content | 200-word segment text truncates with a middle ellipsis and a title tooltip; the transcript rail scrolls independently. |
| Bench unresolved | `--warn` chip in the status strip AND on each variant chip (§7.4); approving stays allowed but the decision records `"bench_resolved": false`. |

No offline/auth states — the app is local-only by design.

---

## 13. Build order

0. **Pin the ground.** Confirm `ARCHITECTURE-2026-07-26.md` and this spec exist at the working
   commit; record that SHA in the PR description — the spec is downstream of both.
1. **P1–P3 engine changes** + their tests. `cargo test --workspace` green.
2. **Tauri command layer** + the five commands + the traversal test.
3. **Shell**: titlebar, three zones, status strip, tokens, fonts, themes. No data yet.
4. **Clip Ledger + Sources mode** — first real value: all clips in one place, playable.
5. **Compare mode** without word-lock (plain A/B) — prove video swap and segment strip.
6. **Word-lock** on top, using P1+P2. Prototype the crossfade here; write `motion-gate-studio.json`
   from observed evidence.
7. **Verdict controls + decisions.jsonl** round-trip (write, reopen, replay).
8. **Finals + QA modes.**
9. **States sweep** (§12) + persistence (§9.3) — every state actually reachable in the built app.
10. **QA + gates** (§14).

Ship 1–4 as a first reviewable slice; do not wait for 10 to show Adrian something.

---

## 14. Acceptance gates

- `cargo test --workspace` green; `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `pnpm typecheck` and `pnpm build` clean in `apps/studio`.
- `/qa` hidden-browser evidence via the existing `qa:browser` contract:
  `qa-functional.mjs` covering open → select source → play → switch mode → swap variant → approve →
  reopen and see the verdict replayed; `qa-shot.mjs` screenshots of every §12 state.
- `axe` clean — run via the `qa:browser` Chrome context (axe-core cannot be injected into the built
  WKWebView shell); WKWebView accessibility is then spot-checked manually with VoiceOver. Do not
  spend a day automating axe inside Tauri.
- Contrast figures are reproducible: WCAG 2.1 relative-luminance formula (the §6.1 numbers were
  computed that way in review); re-verify any token change with the same method.
- `docs/artifacts/motion-plan-studio.md` present; `motion-gate-studio.json` verdict `pass` with
  concrete evidence fields: before/after frame captures of a swap held on one word, the measured
  swap frame timing on the built macOS app, and `reduced-variant.png`. A gate without those three
  artifacts is invalid regardless of verdict.
- The built macOS app opens a real project and plays real media (WKWebView evidence, not Chrome).
- `/audit-visual` run with app context (repeated-use product surface: weight task clarity, keyboard,
  focus, state completeness over first-impression drama).
- **Adrian's eyes approve** the rendered result — `node tools/lib/open-for-review.mjs` the built app
  and the state screenshots. The taste gate is the last gate and it is not delegable.
- Docs synced in the same turn the phase lands: `README.md`, `docs/PHASE-3.md`, `PIPELINES.md`, and a
  MemRight note. When Adrian approves the accent at the taste gate, register the CutRight identity
  row (accent "marker magenta" `#FF4FB8`/`#B4007C`, Tanker/Geist/Spline Sans Mono, both bases) in
  `.claude/rules/brands.md` in that same turn — not before approval; the accent is his call.

---

## 15. Non-goals, restated

No engine invocation. No timeline editing. No network. No chat. No LLM. No effects UI. No
`backdrop-filter`. No animation library. No fourth motion pattern. No undo/redo (`decisions.jsonl`
is append-only; a changed mind appends a new verdict). No filesystem watching (refresh is `⌘R`).
No reverse/negative-rate playback. No Windows or Linux build. If a requirement seems to need one of
these, stop and raise it rather than building it.

---

## 16. Must-hold invariants (assertion form — each maps to a test or review check)

1. `source_word_id` is globally unique per project (compound `{source_id}:{word_id}`); a two-source
   project has zero duplicates. *(Rust test, §1 P2)*
2. After a word-locked swap, the active video's current word has the same `source_word_id` as before
   the swap, OR the cursor is on the nearest following word and the rail shows the cut marker with
   the correct dropped count. *(Vitest, §3)*
3. The three §3 edge rules (preroll, tail-miss, empty variant) never crash and never leave the
   viewer without a defined cursor state. *(Vitest)*
4. Every Tauri command refuses a path outside the opened project; `append_decision` writes only
   `feedback/decisions.jsonl`. *(Rust tests, §9.1)*
5. Every appended decision validates against the §10 schema server-side; an invalid record is
   rejected with a named field, never written. *(Rust test)*
6. Reopening a project replays prior verdicts; badge state survives restart. *(qa-functional)*
7. A verdict on an artifact whose `mp4_mtime` postdates the verdict's `snapshot_generated_at` shows
   the stale-verdict warning chip. *(Vitest on the comparison; qa screenshot)*
8. The inactive Compare video is always muted+paused+`opacity:0`; no audio ever leaks from it, and
   both videos are paintable during a crossfade. *(qa-functional)*
9. Reduced-motion mode: every state change remains legible with all transitions at 0ms.
   *(reduced-variant.png)*
10. No animation JS ships; the single rAF loop is the only per-frame DOM writer and stops on pause.
    *(bundle check + code review)*
11. Verdict state is never colour-only — icon + label accompany every colour change. *(axe/review)*
12. The snapshot never hashes source files; hashing happens only in `verify_sources`. *(Rust test)*
