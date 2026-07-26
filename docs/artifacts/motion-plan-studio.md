# Motion plan — CutRight Studio review workspace

Produced by `/motion` for `docs/PHASE-3-STUDIO-REVIEW-SPEC.md`. Contract: `motion/SKILL.md` §Output.

## 0. Platform

`apps/studio/src-tauri/` + React → **embedded WebView**. Loads `stack.md` **+ `webview.md`**.
Shipping target is macOS only today → **WKWebView**. Web hard rules apply in full (this is a
browser); per-OS engine gates also apply.

## 1. Register

**product.** App UI, repeated daily use, judgment tool. Calibration bar: Linear, Nothing.
Not showpiece — nothing here is a narrative surface, and the restraint test governs.

## 2. Motion language

**Precision.** 120–220ms, sharp ease-out `cubic-bezier(0.22, 1, 0.36, 1)`, tiny distances, high
density, tight rhythm.

Chosen because the app's entire job is a fast repeated judgment. Every millisecond of animation sits
between Adrian and a verdict he will make hundreds of times. Authority (600–900ms) or Luxury
(800–1500ms) would tax the core loop; Playfulness would trivialise an irreversible decision.

## 3. Persistent objects

1. **The word cursor** — survives seeks, variant swaps, and mode changes. The one element that is
   never re-created; it is the thread through the whole session.
2. **The variant chip pair** (`natural` / `tight`) — both always mounted, fixed width, only the fill
   changes. Never reflows.
3. **The verdict badge** — per artifact, persists across mode switches and app restarts (replayed
   from `decisions.jsonl`).

## 4. Scenes (views, not scroll sections)

| View | Entrance | Persistence | Exit |
|---|---|---|---|
| Sources | instant | playhead loop | instant |
| Compare | instant | playhead loop + word cursor | instant |
| Finals | instant | verdict badges | instant |
| QA | instant | — | instant |

Mode switches are **instant by design**. A cross-view transition would add latency to a keyboard-
driven workflow (`⌘1`–`⌘4`) and communicate nothing the segmented control does not already show.
This is the restraint test applied at the view level.

## 5. Patterns (exactly 3 — budget is ≤3 per viewport)

| # | Pattern | Category | Spec |
|---|---|---|---|
| 1 | Variant crossfade | `state.md` | outgoing `opacity 1→0`, incoming `0→1`, 220ms, ease-out, overlapping; both `<video>` elements stay mounted and stacked; inactive rests at `opacity: 0` + `pointer-events: none` (never `visibility: hidden` — a hidden element cannot crossfade) |
| 2 | Verdict commit | `state.md` | badge `scale(0.97)→1` + `opacity 0→1`, 180ms; reason row `height 0→auto`, 180ms |
| 3 | Selection settle | `state.md` | `background-color` + `box-shadow`, 120ms, **no transform** |

All three are state-category. There is deliberately no entrance, exit, spatial, layout, attention, or
continuous pattern in this app.

## 6. Engine per scene

**CSS transitions only. Zero animation JS.** Budget used: 0KB of the 50KB product allowance.

Three simple state transitions do not justify a library, and the minimal-implementation ladder says
stdlib/platform before dependency. The single `requestAnimationFrame` loop that drives the playhead
and word cursor is not animation code — it is a clock reader — and is cancelled on pause.

## 7. Restraint test

> If every animation were removed, would the experience worsen?

| Pattern | Verdict |
|---|---|
| Variant crossfade | **Worsens materially.** A hard cut between two videos parked on the same word is visually identical to a playback stutter. The fade is the only signal that the swap occurred — and the swap is the app's signature interaction. Kept at full strength. |
| Verdict commit | **Worsens.** Approve/reject feels consequential and is recorded permanently; without confirmation feedback users re-click and file duplicate decisions. Kept. |
| Selection settle | **Barely worsens.** Survived only after being cut down from a slide to a colour-and-shadow change. Kept in minimal form as evidence the test was applied honestly, not as decoration. |

Everything else considered was cut: view transitions, rail row stagger, waveform draw-on,
segment-strip build-in, playhead easing, badge pulse. Each communicated nothing.

## 8. Reduced motion + engine fallback

```css
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { transition-duration: 0ms !important; animation-duration: 0ms !important; }
}
```

With motion at 0ms every state change is still fully legible:
- variant swap → chip fill changes, transcript cursor jumps, timecode changes
- verdict → icon, colour, **and text label** change
- selection → accent bar and background change

**Motion is never the sole carrier of a state change.** Per `webview.md` §2 this same reduced path is
the engine-gated fallback, so there is no third code path to maintain.

## 9. Deliberate non-use

| Feature | Why not |
|---|---|
| View Transitions API | Safari 18+ gate; the fallback (instant swap) is what we already want, so the branch buys nothing |
| `backdrop-filter` | High cost on WKWebView; this design has no translucent surfaces by choice |
| Native vibrancy | Same — the single-owner rule is satisfied trivially because neither system is used |
| `will-change` / `translateZ(0)` | Not added as ritual. Only after a measured first-frame hitch, then removed |
| Scroll-driven animation | No scroll narrative exists in this app |

## 10. Performance notes

- One `requestAnimationFrame` loop app-wide, reading `video.currentTime`; it drives the playhead
  line, the word cursor, AND the segment-strip current-segment highlight, and is the only per-frame
  DOM writer. Do **not** use `timeupdate` (~4Hz; the playhead visibly stutters). Cancel on pause and
  unmount.
- The segment strip is static DOM; only the current-segment class changes. Never re-render the strip
  per frame.
- Transcript auto-scroll uses `scrollIntoView({ block: "nearest" })` throttled to word changes, not
  per frame, and disengages on manual scroll.
- Frame budget: 16.7ms at 60Hz, 8.3ms at 120Hz (ProMotion). Profile in the **built app**; a Chrome
  run is not WKWebView evidence.

## 11. Gate status

`docs/artifacts/motion-gate-studio.json` is **not written yet, deliberately.**

Per `motion/SKILL.md`, a `pass` verdict may only be written after prototype evidence exists — the
variant crossfade observed working in the built app, and the reduced-motion variant captured. Writing
it from this plan alone is the documented 2026-07-17 dead-pin failure mode (gate said pass, surface
had zero working motion).

The implementation agent writes it at build-order step 6, recording in the JSON:
- the swap observed at a held word position (before/after frame captures),
- `reduced-variant.png`,
- measured frame timing during a swap on the built macOS app.
