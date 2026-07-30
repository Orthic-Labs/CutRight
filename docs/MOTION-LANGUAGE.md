# Motion Language + B-roll

**Spec for the implementation agent.** Written 2026-07-31 against the `a8d4584` tree + REV2.
Design authority: `/motion` owns the cinematic motion language; this spec adapts its principles into
parameterized rules the CutRight engine can apply automatically. Adrian's eyes approve the result.

## 0. The contract (lead)

The editorial motion grammar is a **small set of meaning-driven, parameterized moves** the engine
applies at evidence-backed moments: punch-in on emphasis, cutaway to mask a jump cut, motivated
transitions, and B-roll at payoff moments to hide cuts and reinforce meaning. It adapts the Motion
skill's core doctrine — *motion serves meaning, not decoration*; persistent objects; clear hierarchy;
one motion language; restraint — into rules a deterministic engine can execute.

The governing test is the Motion skill's **restraint test**, transposed to video:

> If you removed this move, would the edit be worse? If no — or "only slightly" — cut it.

A move that exists to show that we can animate is a defect. The best motion is the motion not added.

## 1. Scope — and the hard line from reframe

Motion grammar and reframe are **separate systems that compose**. Do not conflate them.

| | Motion grammar (this spec) | Reframe (crop-tracking) |
|---|---|---|
| What moves | Discrete editorial moves **on the program** (punch-in, cutaway, transition, B-roll) | The **spatial crop window** for a vertical deliverable |
| When | At evidence-backed moments (emphasis, jump, scene change, payoff) | Continuously, per segment, to keep the subject framed |
| Owner stage | [finish](../skills/content-video-editor/workflows/finish.md) slots | [reframe](../skills/content-video-editor/workflows/reframe.md) |
| Engine today | Phase 5 (effect renderers not wired) | Single midpoint face anchor (Phase 7 = temporal tracking) |

Reframe decides *where the 9:16 window points*; motion decides *what editorial move plays*. A punch-in
is applied **within** the reframed frame and must respect the reframe anchor so the subject stays framed.

## 2. Declare one motion language per project

Adapt the Motion skill's language menu (Authority / Playfulness / Luxury / Precision / Energy / Calm /
Technical / Editorial) to editorial video. Declare exactly one in `brief/motion-plan.md`; it sets the
default scale, easing, duration, and distance for every move. Mixing languages in one video is
incoherent and is a finding.

| Format | Default language | Feel |
|---|---|---|
| YouTube talking-head / tutorial | **Editorial / Precision** | restrained, motivated, invisible |
| Vertical short | **Energy** (restrained) | faster hook punch-ins, tighter cooldowns |
| Launch / story film | **Showpiece register** | choreography test replaces restraint; one anchor + persistent objects |

**Persistent objects** (Motion §2): 1–3 elements survive across beats — the lower-third style, the
caption treatment, a brand shape/color. They are what makes the cuts feel like one video, not a stack of
effects. A showpiece video without a persistent object is a slideshow of effects and fails.

## 3. The grammar (parameterized rules)

Each rule: **trigger** (evidence-backed), **parameters** (defaults, calibrated by preferences), and
**constraints**. Triggers come from package artifacts, never from vibes.

### 3.1 Punch-in on emphasis

A small scale-up that lands on a spoken emphasis to underline a point or mark a beat boundary.

- **Trigger:** an emphasis/payoff word (from `edit/output-transcript-<variant>.json` emphasis + the
  `beat_label:"payoff"` in `edit/editorial-plan.json`) **or** a beat boundary.
- **Parameters:** scale `1.00 → 1.06–1.12` (product); up to `1.15` for shorts. Easing **ease-out /
  exponential ease-out** — never `ease-in` on an entrance, never a flat fade. Duration `180–320 ms`.
  Centre on the subject (reframe anchor / saliency box). Snap the landing to the spoken payoff word.
- **Constraints:** no subject/caption occlusion; **cooldown** ≥ `2500 ms` between punch-ins (product),
  `≥1500 ms` (shorts); density cap (§4). Motion blur **gated to actual movement** — none on a static
  punch-in. A punch-in never starts from `scale(0)`; it grows from the live frame.

### 3.2 Jump-cut masking with cutaways

Hide the visual discontinuity where two same-angle segments meet.

- **Trigger:** a cut whose **jump distance** exceeds threshold (from `analysis/` visual-quality flags —
  the "jump distance at proposed cuts" signal) — i.e. adjacent segments, same source/angle, similar
  scale, that would read as a jarring jump.
- **Response (pick one):** (a) lay a **cutaway** (B-roll, graphic, or alternate angle) over the join for
  its duration; or (b) a **motivated punch-in** (§3.1) on the downstream segment so the scale change
  disguises the jump. Prefer a real cutaway when footage exists; use the punch-in when it does not.
- **Parameters:** cutaway covers the join ± `200–400 ms`; enters/exits ease-out; full-screen or
  picture-in-picture per §6 placement.
- **Constraints:** the cutaway must be *about* the spoken content (not decorative); never mask a cut that
  is intentionally a jump cut for style (respect the preference profile).

### 3.3 Motivated transitions

A transition is allowed only when something motivates it. **The default cut is a hard cut.**

- **Trigger (motivation):** a scene/topic change (`analysis/scenes.json` boundary or a `beat_label`
  change), a time passage, or a location change. No motivation → hard cut, no transition.
- **Vocabulary:** hard cut (default); flash/whip (Energy language, at a beat, sparingly); fade/dip
  (time passage only). **No transition on every cut** and **no whoosh on every transition** — SFX are
  functional, one sound per meaningful event.
- **Constraints:** a transition must not overlap a spoken word; it must not delay the hook past the
  format's time-to-value (≤1 s product, ≤3 s short to first value).

### 3.4 B-roll / cutaway at payoff moments

Reinforce the payoff and cover the cut that lands it.

- **Trigger:** a `beat_label:"payoff"` (or a high hook/payoff-strength score) **and** visual support
  available (real B-roll in the sources, or a registered graphic that clarifies the point).
- **Rule:** prefer **real evidence** over an invented graphic. If footage of the thing exists, show it;
  if a number is spoken, a registered stat effect may clarify; otherwise stay on the speaker.
- **Parameters:** duration `1200–3000 ms` (enough to read, not so long the speaker is forgotten); enter
  on the payoff word, ease-out; duck the speaker audio under any B-roll SFX/music.
- **Constraints:** caption-safe and subject-safe (§6); the B-roll must be truthful to the speech
  (misleading B-roll is a QA failure); density capped (§4).

## 4. Density + restraint

- **Pattern budget:** ≤ 3 distinct move types active per minute (product); a move on every cut is a
  finding. Shorts may run denser but still obey the cooldowns.
- **Restraint test on every move** (§0). Apply it per move and to the whole pass: if stripping all moves
  would not worsen the edit, the pass was decoration.
- **Register:** talking-head/tutorial = **product** register (restraint doctrine). A launch/story film =
  **showpiece** register, where the **choreography test** replaces restraint ("does every move advance
  the one narrative?") — density allowed, incoherence not. The floors (§8) hold in both.
- Density, cooldowns, and language are learned per format in `feedback/preferences.json`
  (`effect_density`, `broll_frequency`, `music_sfx`).

## 5. Parameter defaults (starting candidates, calibrated by preference)

| Move | Scale / size | Easing | Duration | Cooldown |
|---|---|---|---|---|
| Punch-in (product) | 1.06–1.12 | exponential ease-out | 180–320 ms | ≥2500 ms |
| Punch-in (short) | 1.10–1.15 | exponential ease-out | 140–240 ms | ≥1500 ms |
| Cutaway (jump mask) | full-frame or PiP | ease-out in/out | join ±200–400 ms | — |
| Transition (motivated) | n/a | per type | 200–400 ms | scene change only |
| B-roll (payoff) | full-frame | ease-out | 1200–3000 ms | ≥ one beat apart |

These are configuration, not validated truths; the preference learner tunes them per format.

## 6. Placement + collision

Every graphic/cutaway slot computes placement over its **whole interval** (not one frame) using the
vision plan §12.6 cost function: subject + face + hand/gesture + existing-text + caption + platform-UI +
saliency overlap, plus edge proximity and temporal jitter. Choose the lowest-cost **stable** anchor; if
every anchor is poor, in order: full-screen cutaway → delay the graphic → reduce/remove it → ask for
human placement. Never shrink important text to unreadable to satisfy collision. Motion moves reuse the
reframe anchor so a punch-in stays centred on the tracked subject.

## 7. Integration (finish-plan slots)

Moves are authored as slots in `finish/finish-plan.json` (schema in
[finish](../skills/content-video-editor/workflows/finish.md)), triggered by evidence:

```json
{
  "id": "slot-011",
  "kind": "punch-in",
  "renderer": "remotion",
  "effect_id": "punchin.emphasis.v1",
  "output_start_ms": 41800,
  "output_end_ms": 42120,
  "anchor": "subject",
  "collision_policy": "avoid-subject-and-platform-ui",
  "trigger": {"source": "editorial-plan", "beat_label": "payoff", "word_id": "ow_000312"},
  "props": {"scale_to": 1.1, "ease": "exponential-out", "motion_blur": false}
}
```

`kind` ∈ `punch-in | cutaway | transition | broll`. Triggers reference real evidence: emphasis words
(output transcript), jump distance (visual-quality flags), scene boundaries (`analysis/scenes.json`),
payoff beats (`edit/editorial-plan.json`). A **new signature move** not in this grammar is handed to
Motion via [HANDOFF-CONTRACTS.md](HANDOFF-CONTRACTS.md) §4 — the engine applies stock grammar, Motion
authors new language.

## 8. Floors (survive every register)

- A **reduced-motion variant** for every move: a punch-in reduces to a hard cut or a minimal `≤1.03`
  nudge; a transition reduces to a hard cut. Honour `prefers-reduced-motion` at delivery.
- No `scale(0)` (nothing appears from nothing); no `ease-in` on entrances; prefer transform/opacity.
- Caption-safe and subject-safe, always; a move that occludes the payoff word or the subject is a
  failure.
- Motion blur only where there is real movement; pull-backs stay centre-locked; text eases out
  exponentially rather than flat-fading (the existing finish doctrine, preserved).

## Engine gaps to know

- **Effect renderers are not wired.** `slot render` accepts only `renderer:"render.final"` today; the
  Remotion/HyperFrames/ASS effect library (punch-in, cutaway, stat, caption components) is Phase 5.
  Until then this grammar is authored into `finish-plan.json` + a reference render, and the engine
  composes it when the slot renderers land. Do not report a move as "applied" until it actually renders.
- **Triggers depend on signals not all produced yet:** jump distance + visual-quality flags and
  `analysis/scenes.json` are part of local visual perception (Phase 7); emphasis words rely on the
  output transcript + `editorial-plan.json` payoff labels (EDITORIAL-BRAIN). Where a trigger signal is
  unavailable, the move is **not** applied — the engine fails closed (no move) rather than guessing a
  moment.
- Temporal tracking for subject-centred punch-ins is Phase 7; until then a punch-in centres on the
  single reframe anchor, which is approximate for moving subjects.
- The 15 starter effects + effect registry (vision §12.2) are Phase 5; `effect_id`s above are the target
  naming, not shipped assets.
