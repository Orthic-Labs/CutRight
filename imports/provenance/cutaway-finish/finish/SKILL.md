---
name: shortform-finish
description: "STAGE-2 finishing pass for SHORT-FORM 9:16 clips — runs AFTER shortform-cutaway has locked the rough cut (best takes, whole words, red thread). This is the STYLING / ANIMATION layer: 9:16 reframe, animated zooms & punch-ins, lens effects (chromatic aberration, zoom blur), an exponential text-fade-in look, captions, on-screen 'editor voice' text, music and SFX. The cut is sacred and decided upstream; this skill never re-picks takes or moves cut points. Use whenever a short's cut is LOCKED and you want it styled: 'add a zoom', 'punch in here', 'make it pop', 'add the hook zoom', 'finish this short'. NOT long-form."
---

# SHORTFORM FINISH — the styling / animation stage (Stage 2)

> **THE JOB:** take a LOCKED short-form rough cut (from `shortform-cutaway`) and make it *look*
> finished — 9:16 framing, animated zooms / punch-ins, lens effects, an exponential text fade-in,
> captions / on-screen text, music + SFX. The cut itself is already decided upstream; this skill never
> re-picks takes or moves cut points.

**This skill teaches the METHOD, not a pile of asset files.** The looks below are *techniques you
recreate* with your own footage, your own SFX, and your own style assets. Build your own small library
once and reuse it — that library is *yours*. Nothing here ships you finished templates.

---

## ⭐ FIRST RUN — which editor do you use?

**Ask the user this once, then follow the matching branch.** The finishing *principles* (the zoom
curve, the parallax rule, the exponential fade) are identical everywhere — only the mechanism differs.

- **(a) DaVinci Resolve** — zooms/effects are built as Fusion node graphs with animation injected as
  splines (see the Resolve method below). Captions/text as alpha overlays.
- **(b) Premiere Pro** — zooms via keyframed Scale on the clip's Motion; lens FX via the built-in
  effects (Lens Distortion, Transform with shutter-angle blur); text via Essential Graphics. The
  *curves and timings* below port directly onto Premiere keyframes.
- **(c) Remotion** — everything is code: `scale`/`translate` driven by `interpolate` + spring curves,
  blur via CSS/SVG filters, text by animating `opacity`/`transform`. This is the most flexible path and
  the exponential-fade math below is literally a Remotion `interpolate` curve.
- **(d) Claude-Code-only (ffmpeg, no NLE)** — bake zooms with a crop-zoom expression, captions burned
  in, audio mixed in ffmpeg. Simplest, fully scriptable, no other software.

> If the user doesn't know, default to **(c) Remotion** for full styling control, or **(d)** if they
> want zero extra software.

---

## BRING YOUR OWN STYLE LIBRARY & SFX (the model)

This skill does **not** ship motion-graphics templates, SFX packs, or grade LUTs. You supply those:

- **Style assets / motion graphics** — your own money/count-up cards, toggles, checkboxes, IG-comment
  mocks, lower-thirds, etc. Build them once (Remotion, After Effects, or even still PNGs animated with a
  zoom) and keep them as re-textable TEMPLATES. The *rules* for using them well are below.
- **SFX** — your own whooshes, clicks, ticks, risers. Any decent SFX pack works. The *philosophy* for
  matching sound to motion is below; the sounds are yours.
- **Grade / LUTs** — your own look. Grading is out of scope for this skill; do it in your NLE.

The value here is the **method**: how to build a zoom-out hook, a punch-in/out wave, an exponential
text fade-in, and the authority-stack text look — and how to time SFX to motion. Recreate them with
your assets.

---

## WORK ON A DUPLICATE — never the locked cut
The cutaway skill hands off a locked cut. **Duplicate it** and do all styling on the copy (in Resolve:
`timeline duplicate`; in Premiere: duplicate the sequence; in Remotion/ffmpeg: a new comp/output). The
pristine cut stays untouched and reproducible.

## REFRAME TO 9:16 — check the source first
Set the canvas to **1080×1920**. Then check the *source* dimensions before any fill math:
- **If the source is already shot vertical** (e.g. 2160×3840), a 9:16 canvas fills perfectly — no crop,
  no bars. Scale 1.0 = the clean full frame; scale > 1 punches in.
- **If the source is 16:9**, you must fill-scale (crop the sides) to avoid pillarbox bars.

---

## TECHNIQUE 1 — ANIMATED ZOOMS (hook, push, punch)

The whole short stays alive by always being subtly in motion. Three moves do most of the work.

### The zoom-OUT "hook" (opening pull-back)
A quick-then-slow pull-back at the very start grabs attention. **Scale `1.3 → 1.0`** with a **front-loaded
ease-out**: most of the move happens in the first ~0.6s, then it crawls to a stop. As keyframes (at 24fps):
`[frame 0]=1.3  [6]=1.13  [14]=1.05  [26]=1.01  [40]=1.0`. Add a touch of motion blur **only during the
move** (shutter ~180°). Knobs: start scale (drama), keyframe spacing (snap vs linger), settle frame.

> **⛔ CENTER LOCK on any zoom that settles to ≤ 1.0 (hook / pull-back).** Keep the zoom **center at
> 0.5,0.5 (frame center). NEVER offset it.** The instant scale reaches ≤ 1.0, an off-center zoom exposes
> BLACK at the frame edge and shoves the subject out of view. Only the *scale* animates on a pull-back.

### The punch-IN / punch-OUT "wave" (mid-video energy)
Keeps motion constant so viewers don't tire. The rhythm:
- **Intros = punch OUT** (start zoomed, pull out — the hook).
- **Mid-video cuts = a wave:** each clip **punches IN at its END** (zoom accelerates into the cut) → the
  cut lands at peak zoom → the next clip **starts zoomed and punches OUT at its start** (settles), then
  punches in at ITS end → repeat. The cut sits where both sides are at peak, hiding it inside continuous
  motion.
- **The "stretch" curve:** more zoom + a HARD ease-out — very fast off the line then a long slow crawl.
  Example scale values for a punch-out: **`[0]=1.5 [3]=1.15 [8]=1.04 [16]=1.005 [24]=1.0`** (≈70% of the
  move in the first 3 frames); mirror for punch-in. Pair with a strong zoom-blur during the fast part.
- **Do it PER-CLIP** (a separate zoom on each clip), so each side of a cut can have its own move.

### Zoom CENTER on a pure zoom-IN
On a move that stays **≥ 1.0** (a push-in, never settling below full frame), you *may* bias the zoom
center toward whatever you're emphasizing (e.g. center 0.5,~0.6 to "lean the camera up" toward a graphic
in the top strip). This is only allowed when scale never drops below 1.0 — otherwise the center-lock rule
above applies.

### Lens FX that intensify DURING the move
- **Zoom blur** — a directional/zoom blur whose strength peaks during the fast part of the zoom and
  returns to 0 at rest.
- **Chromatic aberration** — a radial RGB-channel offset whose strength peaks during the move.
- **GATE motion blur to the MOVE, never the whole clip.** If you blur every frame, moving hands/objects
  smear the entire clip. After the move settles, the clip MUST be crisp. (If baking in ffmpeg, render
  only the hook frames with frame-blend and the rest sharp, then concat.)
- **Every zoom punch/hook wants a SOUND** — a riser/whoosh timed to the move (peak-aligned). A
  motion-blurred zoom with no audio swell feels empty.

### How each editor builds the zoom
- **Resolve:** the scripting API can't create keyframes directly. Build a Fusion graph (Transform for
  zoom; DirectionalBlur for zoom blur), export the clip's `.comp`, edit the text to add a `BezierSpline`
  modifier with your keyframes and point the animated input at it, re-import, load. Verify by sampling
  the input at two times (a spline-driven input reads back correctly even when raw keyframe writes don't).
  **After ANY Fusion change, open/load the comp once or it won't render.**
- **Premiere:** keyframe Scale on the clip's Motion with the values above; ease the keyframes.
- **Remotion:** `interpolate(frame, [0,3,8,16,24], [1.5,1.15,1.04,1.005,1.0])` (or a spring) on `scale`.
- **ffmpeg:** an alpha-safe crop-zoom, e.g.
  `crop=w='1080/(1+min(n*k,0.085))':h='1920/(1+min(n*k,0.085))':x='(in_w-out_w)/2':y='(in_h-out_h)/2',scale=1080:1920`.

---

## TECHNIQUE 2 — EXPONENTIAL TEXT FADE-IN (best-practice, teachable)

The signature on-screen text look. **Never a flat linear fade.** Text should *bloom* in — fast at the
start, settling slow — so it feels like it materializes rather than dissolves.

- **Opacity** rides an exponential/ease-out curve: `opacity = 1 - e^(-k·t)` (or an ease-out cubic). It
  jumps most of the way up immediately, then eases to full.
- **Pair it with a small upward drift + a blur-to-sharp**: start ~10px low and slightly blurred, settle
  to position and sharp on the same ease-out. (This is the "blur-to-sharp" hero reveal.)
- **Exit** mirrors it — a quick lift-and-blur-out, not a flat fade.
- **In Remotion** this is one `interpolate` with `easing: Easing.out(Easing.exp)` on `opacity`,
  `translateY`, and `blur`. **In Premiere/AE** it's eased opacity + position + a small Gaussian-blur
  keyframe. **In ffmpeg** it's a `fade` with a curve plus a `boxblur` ramp.

This is a *principle* you apply to every text element — captions emphasis, lower-thirds, the editor-voice
text. Once it's in your head you'll never ship a flat fade again.

---

## TECHNIQUE 3 — THE "AUTHORITY STACK" TEXT LOOK (recreate it)

The on-screen "editor voice" / pull-quote signature: **words rise out of a soft fog and speed-ramp to a
settle**, stacked, bold, tight tracking. It reads as authored and deliberate, not like a default caption.

How to recreate it (any tool):
- **Bold, tight letter-spacing** (≈ -0.04em), high contrast, comfortably large.
- **Each line/word enters with a fog-rise**: starts low + blurred + faint, then rises into place while
  sharpening — on the exponential ease-out from Technique 2, with a slight **speed ramp** (fast-in, slow
  settle).
- **Stack the lines** so later lines sit slightly behind/under earlier ones (a hint of depth/overlap).
- **Exit = lift + blur-out.** Never a flat cut-off.
- **For the "editor voice" character** (the on-screen text that speaks as your AI editor), give it a warm
  accent color so it reads as a distinct voice, and animate it in exactly like the authority stack — never
  a flat fade.

This is the brand-signature text move; build it once in your tool of choice and reuse it. It is a
*technique*, not a file you receive.

---

## ON-SCREEN ASSET RULES (when you DO add your own graphics)

These rules make any motion graphic land — apply them to *your* assets.

- **RULE 0 — LOOK at the asset before using it.** Never pick a graphic by its filename. Know what it
  actually shows; a graphic whose meaning contradicts the spoken line is a bug.
- **RULE 1 — Match the EXACT line it sits over.** The graphic appears ONLY while that thing is being said
  and is GONE the instant the topic passes. Re-text it to the exact words/number being spoken.
- **RULE 2 — User-provided images (screenshots/photos):** keep them **small**, clustered in the **negative
  space around the face** (a tidy **2×2 grid in the bottom strip, above the captions** is the reliable
  default), **pop them in as-is** (no slide/scale), in **rapid succession (~3 frames apart)**, on screen
  **only for the relevant section**, never over the face, never overlapping each other.
- **RULE 3 — Re-text at the SOURCE and re-render.** Every graphic is a re-textable TEMPLATE; change the
  text in the source comp and re-render with alpha — never overlay text on a baked render.
- **RULE 4 — PARALLAX is mandatory over a zooming clip.** When a graphic sits over a clip that zooms, the
  **footage zooms** AND the **graphic zooms slightly MORE** (foreground leans forward), synced to the same
  frames. If only the graphic zooms, it doesn't read. (Tracked elements like a finger reticle get NO
  parallax — a zoom breaks the lock.)
- **RULE 5 — Bake SFX into the asset, per-asset.** Each asset/section gets its OWN subtle SFX timed to its
  reveal keyframes — not one giant SFX bed over the whole timeline. Put a subtle SFX on basically anything
  that animates in.
- **LAYOUT DISCIPLINE:** don't scatter elements top/center/bottom — keep them clustered so the viewer's
  eye stays in one zone.

---

## TECHNIQUE 4 — SOUND DESIGN (match the SFX to the MOTION)

**The sound type IS the motion** — pick by what's moving, then peak-align it.

- **WHOOSH = a ZOOM / movement.** Whoosh **length must match the zoom length** (slow long zoom → long
  whoosh; fast zoom → short snappy whoosh).
- **CLICK / SNAP = a sudden change with NO movement** (a card that snaps in without moving → click, not
  whoosh).
- **TICK = a checkmark / toggle flick** (subtle).
- **NOT on plain b-roll source cuts** — whooshes don't fit those.
- **ALIGN TO THE PEAK, not the file start.** Analyze the waveform, find the transient peak, and trim the
  leading silence so the **SFX peak lands exactly on the visual event**. A reusable trick: make the SFX
  symmetric with the **peak dead-center**, then place the clip centered on the cut — the peak lands every
  time. Dial whooshes DOWN; they're usually too loud.
- **ALWAYS read the WAVEFORM, not the duration** — length lies. Inspect attack sharpness, peak position,
  and depth before choosing/placing a sound.

> See `scripts/reverb_throw.sh` for the one portable audio move shipped with this skill (below).

### REVERB THROW — `scripts/reverb_throw.sh`
The signature audio move: the clip plays **clean/dry**, then as it cuts the **last moment blooms into a
reverb tail** and drags out into silence — NOT reverb on the whole clip, only the ending. Use it for depth
at cut points (stack it under a riser/whoosh on a transition).
- **Engine = SoX Freeverb** (`brew install sox`) + ffmpeg. (ffmpeg's `afir` convolution is broken in many
  builds — outputs silence; SoX wet-only is the keeper.)
- `reverb_throw.sh IN.wav OUT.wav [THROW_SEC=0.7] [REVERB=78] [TAIL_PAD=4] [WET=1.0]`. Knobs: THROW_SEC
  (how much of the end blooms), REVERB (tail length/lushness), WET (bloom level). Output normalized to −3 dB.

---

## TECHNIQUE 5 — "EDITOR TAKEOVER" B-ROLL (optional signature)

A fun mid-video move: the speaker says *"let my editor explain this"* → cut to a short motion-graphics
explainer narrated by an AI voice → speaker returns. The speaker is the hook; the editor is the clarity
layer. Build it from text-slide templates you own, narrate with any TTS, and **speech-sync the text to the
VO word timestamps**. Use the exponential fade / authority-stack look for the on-screen text. Keep it
parasitic — add it only where a real gap in the explanation needs it.

---

## AUDITION WORKFLOW (the way to finish)
- **Render 4–5 ALTERNATIVES per styled moment** (different zoom amounts, different graphics, different
  SFX) and compare, rather than committing to one. This is the reliable way to land taste.
- **Place every created asset onto the timeline** — never leave a render as just a file on disk to drag in
  later. Import it, put it on its own track at the right spot, enabled.

## SCOPE
**DOES:** 9:16 reframe · animated zooms / punch-ins · lens FX (chromatic aberration, zoom/radial blur,
gated motion blur) · exponential text fade-in · authority-stack text look · captions / editor-voice text ·
SFX timing + the reverb-throw move · (optional) editor-takeover b-roll.
**DOES NOT (→ that's `shortform-cutaway`):** pick takes · find/remove silences · choose the red thread ·
move cut points. The cut is locked before this skill runs.
