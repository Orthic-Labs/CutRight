# CutRight Studio — from-scratch redesign spec

Mode: **ship**. Surface: **app UI** (Tauri / WKWebView on macOS). Motion register: **product**.
Ordered by Adrian 2026-08-02 after rejecting the current UI outright.

## Phase 0 — Task truth

> CutRight Studio helps Adrian review machine-made edits by comparing variants
> word-locked at the same spoken moment and binding verdicts to exact artifact
> hashes — and the interface must make **"which variant wins, and what is
> blocking ship"** obvious at every glance.

Not a marketing surface. Repeated-use, keyboard-first, hour-long sessions, dark
room next to an editing monitor. Fatigue profile dominates: low-luminance base,
no pure white, accent reserved for state.

## Phase 1 — Workspace signature (the mechanism the app already earns)

**The Word-Locked Bench.** The one interaction no other app has: swapping
between NATURAL and TIGHT **at the same spoken word**, with the cut-count delta
("3 words cut here") as the evidence. The redesign promotes this from a small
toggle row to the bench itself:

- A/B swap is the largest interactive element after the video — a two-pane
  split handle with the active variant lit and the delta badge at the lock
  point.
- The transcript is not a sidebar afterthought; it is the **spine** — the
  word cursor is the shared clock between video, waveform, segments and ledger.
- Verdicts read as **receipts**: approve/reject stamps show the bound hash
  fragment, because that is literally what the backend persists.

Signature test: task-true (the product's core compare loop), non-transplant
(meaningless without word-locked variants), stateful (variant, delta, verdict,
bench status), efficient (one keystroke swap), nameable (word-locked bench).

## Phase 2 — Three registers (PARKED for Adrian's eyes as rendered screenshots)

All three share the signature, the IA, and the motion plan; they differ on
base/accent/density/type (≥2 axes each). Suite constraint: may not impersonate
sibling accents — taken: VR blue, SR gold, HR ember, CR crimson, MR red,
VoiceRight violet. Free hue space: **teal/cyan family and green family**.

| | R1 · Cutting Room (technical/console) | R2 · Bench (operational/dense) | R3 · Screening Room (calm/editorial) |
|---|---|---|---|
| Base | near-black neutral `#101012` | slate-graphite split `#131720` canvas / `#191C24` panels | deep warm gray `#17161A` |
| Accent | signal teal `#3FD8C7` dark / `#0E7E72` light | electric cyan `#4DC8F0` dark / `#0A6E9E` light | jade `#5BD federated…` → jade `#57C79A` dark / `#177452` light |
| Accent meaning | selection + the active variant only | state + keyboard focus + live playhead | verdicts and proof only |
| Density | dense, 12px data mono, tight leading | densest — editor-grade, dual-row status | roomy, 14px body, generous transcript measure |
| Type | Tanker display / Spline Sans Mono data / Hanken Grotesk UI | Tanker / JetBrains Mono / General Sans | Tanker / Hanken Grotesk / Spline Sans Mono labels |
| Feel | color-suite console | Resolve-adjacent operator deck | quiet screening room with instruments |
| Risk | reads cold if teal overused | dense = intimidating first-run | too calm for QA triage |

Wordmark in all three: `CutRight`, Tanker, "Right" in accent — suite convention.
Whichever register wins gets a locked entry proposed for `brands.md` (Adrian
approves the lock; nothing is written to the brand system before that).

## Phase 3 — IA and state model

Modes: **sources · compare · finals · qa · settings** (settings landing from the
concurrent worker). Navigation: left-anchored mode rail with real weight +
keycap hints (1–5), not floating lowercase text. Every mode declares
empty/loading/error/success/selected/focus states; the happy path alone fails.

- Sources: rail with real thumbnails (first-frame extract exists in evidence
  dirs; fall back to duration-labeled placeholder tiles, never a bare glyph).
  Integrity badges (BLOCKED/UNVERIFIED) keep their semantics.
- Compare: the bench. Video center, A/B split handle, transcript spine right,
  segment strip under transport, verdict receipt bar bottom.
- Finals: preset cards with probed-vs-expected facts and the hash-bound
  "use for final" selection.
- QA: report table per preset, check statuses with evidence loci, acknowledge
  binds the report hash.
- Status strip: two-zone — pipeline/bench/QA left, decisions/session right,
  13px minimum, AA contrast, no orphaned unlabeled values (the "05:56" gets a
  label or dies).

## Motion plan (product register, Precision language)

Persistent objects: **the word cursor** and **the playhead** — they are the
same clock rendered twice and must never desync visibly.

| Scene | Motion | Engine | Purpose |
|---|---|---|---|
| Variant swap | 140ms crossfade + 2% scale settle on the incoming pane; delta badge counts up once | CSS transition + rAF | communicates "same moment, different cut" |
| Word cursor | transform-only underline slide between words; reduced-motion: instant | CSS transform | the shared clock made visible |
| Verdict stamp | 120ms ease-out scale 0.95→1 on the receipt chip | CSS | confirms the ledger write |
| Mode switch | 100ms opacity, no slide | CSS | modes are places, not slides |
| Banner (provisional review) | none — static presence | — | warnings do not dance |

Floors: `prefers-reduced-motion` variant on all four; transform/opacity only;
no `ease-in` entrances; no `transition: all`; zero added JS animation weight
(CSS + existing rAF only — WKWebView and WebView2 both fine). Restraint test:
every row above encodes state; nothing decorates.

## QA gates before Adrian sees it

Typecheck/test/build + headless QA action suites green in all three registers;
screenshots of every mode × register (15 shots) + reduced-motion capture;
axe pass on the new nav rail and receipts; then PARK for the pick.
