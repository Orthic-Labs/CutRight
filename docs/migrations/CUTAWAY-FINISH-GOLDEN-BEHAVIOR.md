# Cutaway / Finish — Golden Behavior Migration

Status: Book 1 (CR-V2-B1-014). Provenance material lives at
`imports/provenance/cutaway-finish/` and is hash-bound by
`imports/provenance/cutaway-finish/hash-manifest.json` (receipt:
`imports/v2/receipts/cutaway-finish.json`). Disposition: `provenance_only`.

**No provenance script may ever be called by release code.** Python, Bash,
FFmpeg/ffprobe, SoX, auto-editor, WhisperX, faster-whisper, and the DaVinci
Resolve scripting API are provenance-only dependencies. Every live behavior
below gets a named native (Rust) stage and a golden fixture that native
tests must recreate.

Notation: `provenance path -> native stage` followed by the golden
input/output contract.

## 1. Rough-cut pipeline (cutaway)

### 1.1 Transcript understanding
`cutaway/scripts/transcribe.py -> video-project::transcript::understand_span`

- Behavior: transcribe 16 kHz mono audio for editorial UNDERSTANDING only;
  emit segments + words plus a human-readable memo. Its segment boundaries
  are explicitly NOT cut boundaries (segments smooth over pauses).
- Golden input: pinned 16 kHz mono WAV + expected memo facts.
- Golden output: transcript span set (text, approximate times); assertion
  that the native pipeline never consumes these boundaries as cut edges
  (negative fixture: cut list derived from transcript segments must be
  rejected).

### 1.2 Forced alignment
`cutaway/scripts/whisperx_align.py -> speech pack alignment stage (video-project::alignment::force_align_words)`

- Behavior: transcribe then force-align every word to the waveform so each
  word has an exact start/end second. This is the breakthrough the whole
  pipeline depends on; naive ASR word times drift around pauses and are
  forbidden as cut sources.
- Golden input: pinned 16 kHz mono WAV.
- Golden output: word list `{s, e, w}` JSON; native alignment must match the
  frozen reference within a declared tolerance per word edge, and every
  kept word must remain whole.

### 1.3 Word-safe cuts (live builder)
`cutaway/scripts/build_wx.py -> video-project::boundary_consensus::compile_word_safe_segments`

- Behavior: given aligned words + editorial beats (`START END label` red
  thread) + GAP dial, start a new clip at each beat boundary or word gap >
  GAP; bound each clip at `word.start - LEAD` / `word.end + TAIL` so no word
  is clipped and no clip starts in dead air; drop beats containing no
  words.
- Golden input: frozen aligned-words JSON + `beats.example.txt`-format beat
  file + GAP/LEAD/TAIL parameters.
- Golden output: exact clip list (start/end pairs + labels). The native
  stage is deterministic; golden comparison is exact, not tolerant.

### 1.4 Speech-region intersection (documented variants)
`cutaway/scripts/build_cut.py -> video-project::boundary_consensus::intersect_speech_regions_with_word_edges`
`cutaway/scripts/build_wordcut.py -> video-project::boundary_consensus::snap_cuts_to_word_boundaries`

- Behavior: split only where verified silence/speech energy says so, then
  snap in/out points to word edges; word positions are never trusted to
  PLACE cuts. These upstream builders are documented dead ends relative to
  `build_wx.py`, but their invariant (audio-energy region ∩ word edges) is
  retained as a cross-check inside boundary consensus.
- Golden input: frozen audio-energy mask + aligned words + beats.
- Golden output: clip list identical to the reference builder for the
  golden sample; divergence fixtures record why `build_wx.py` won.

### 1.5 Cut-list export
`cutaway/scripts/build_clipinfos.py -> video-project::cut_list::export_timeline_clips`

- Behavior: turn kept spans into NLE-landing records
  (`media_pool_item_id / start_frame / end_frame / record_frame`) with fps
  and breath-margin parameters.
- Golden input: kept-span list + fps + margin.
- Golden output: frame-exact clip records (deterministic integer math).
- CutRight v2 lands cuts in its own typed timeline, so Resolve/Premiere/
  Remotion landing branches become provenance-only knowledge.

### 1.6 Rejected rough-cut behaviors (negative goldens)
`cutaway/scripts/flow_cut.py`, fixed-dB silencedetect, amplitude-margin
cutting, and raw Whisper-segment cutting are recorded as rejected:

- Fixed-dB silence detection fails when mic distance varies.
- Amplitude margins clip whole words ("This", "ASAP").
- Whisper/ASR-guessed word times place cuts inside silence.
- Over-aggressive auto-cut fragments quiet words mid-sentence.
Native tests carry one negative golden per rejected behavior: the naive
strategy must NOT reproduce the golden cut list.

## 2. Evidence aids (cutaway)

### 2.1 Motion scoring
`cutaway/scripts/motion_score.py -> video-evidence::motion::score_span`

- Behavior: deterministic camera-move detector — mean absolute difference
  between consecutive 160×90 grayscale frames at 6 fps; per-beat score or
  dense scan to find a calm sub-window. Rule of thumb: mean < 6 calm,
  spikes > 15 camera move.
- Golden input: pinned frame span + beat windows.
- Golden output: per-span motion scores and spike counts (exact for the
  frozen decoder output; declared tolerance for alternate decoders).

### 2.2 Storyboard extraction
`cutaway/scripts/storyboard.sh -> video-evidence::storyboard::extract_beat_frames`

- Behavior: one representative frame per beat for framing/eye-contact/take
  review.
- Golden input: pinned source + beat list.
- Golden output: one frame per beat at deterministic beat timestamps.

### 2.3 Rough-cut preview render
`cutaway/scripts/preview.sh -> video-project::preview::render_rough_cut`

- Behavior: render the editorial keeplist to a 9:16 center-crop rough cut,
  then a silence-tightened variant.
- Golden input: pinned source + keeplist.
- Golden output: cut list + declared frame-accuracy assertions against the
  native renderer (pixel-exact comparison is out of scope; cut math is
  exact).

## 3. Finishing pipeline (finish)

All finish behaviors operate on a LOCKED cut and never move cut points or
re-pick takes; the native stages must refuse cut mutation (typed contract).

### 3.1 Hook pull-back
`finish/SKILL.md Technique 1 (zoom-OUT hook) -> video-project::motion::PullbackHookNode`

- Behavior: scale 1.3 → 1.0 front-loaded ease-out (`[0]=1.3 [6]=1.13
  [14]=1.05 [26]=1.01 [40]=1.0` at 24 fps), CENTER LOCKED at 0.5,0.5;
  motion blur gated to the move only.
- Golden input: node parameters + frame indices.
- Golden output: exact sampled scale/center/blur values per frame.
  Negative golden: any center offset once scale ≤ 1.0 must be refused
  (black-edge exposure).

### 3.2 Punch wave
`finish/SKILL.md Technique 1 (punch-IN/OUT wave) -> video-project::motion::PunchWaveNode`

- Behavior: per-clip stretch curve (`[0]=1.5 [3]=1.15 [8]=1.04 [16]=1.005
  [24]=1.0`), peak zoom at the cut, mirrored punch-in/out sides, zoom blur
  during the fast part only.
- Golden input: clip boundaries + curve parameters.
- Golden output: per-frame scale/blur samples; peak alignment at cuts.

### 3.3 Push-in with biased center
`finish/SKILL.md Technique 1 (zoom-IN center) -> video-project::motion::PushInNode`

- Behavior: center bias allowed only while scale never drops below 1.0.
- Golden output: refusal when the sampled curve dips below 1.0 with an
  off-center anchor (negative golden).

### 3.4 Text bloom / authority stack
`finish/SKILL.md Techniques 2–3 -> video-project::text::BloomRevealNode (+ stacked-line layout rules)`

- Behavior: exponential ease-out opacity `1 - e^(-k·t)` paired with upward
  drift and blur-to-sharp; exits mirror; authority-stack lines rise out of
  fog with speed ramp, tight tracking, stacked depth; editor-voice accent
  color.
- Golden input: reveal parameters + frame indices.
- Golden output: sampled opacity/offset/blur on the declared curve; flat
  linear fades must fail the golden comparison.

### 3.5 On-screen asset rules
`finish/SKILL.md asset rules 0–5 -> video-project::layout::AssetPlacementPolicy`

- Behavior: asset meaning check before use, exact-line lifetime, small
  negative-space 2×2 clusters popping in ~3 frames apart, re-text-at-source,
  mandatory parallax over zooming footage (footage zoom + slightly larger
  graphic zoom, same frames; no parallax for tracked elements), per-asset
  SFX.
- Golden input: shot plan + asset list + zooming-clip flag.
- Golden output: placement/lifetime/parallax decisions; violation fixtures
  (scattered layout, filename-only selection, graphic-only zoom) must be
  refused.

### 3.6 SFX peak alignment
`finish/SKILL.md Technique 4 -> video-media::audio::SfxPeakAlignNode`

- Behavior: choose SFX by motion type (whoosh = movement, click = snap,
  tick = toggle), read the waveform not the duration, trim leading silence
  so the transient peak lands exactly on the visual event, symmetric-peak
  placement trick, whooshes dialed down.
- Golden input: pinned SFX waveform + visual event time.
- Golden output: placement offset that centers the detected transient peak
  on the event (deterministic peak detection on the frozen waveform).

### 3.7 Reverb throw
`finish/scripts/reverb_throw.sh -> video-media::audio::ReverbThrowNode`

- Behavior: dry body, only the final THROW_SEC blooms into a wet reverb
  tail (SoX Freeverb, wet-only on the padded ending), tail pad, normalize
  to −3 dB.
- Golden input: pinned trimmed WAV + THROW_SEC/REVERB/TAIL_PAD/WET.
- Golden output: rendered WAV compared sample-by-sample against the frozen
  SoX reference within a declared epsilon; dry region before the throw must
  be bit-identical to the input.

### 3.8 Editor-takeover b-roll (optional signature)
`finish/SKILL.md Technique 5 -> video-project::overlay::EditorTakeoverPlan`

- Behavior: cut to a narrated motion-graphics explainer whose text is
  speech-synced to VO word timestamps; parasitic placement only.
- Golden input: VO word timestamps + slide text plan.
- Golden output: text spans aligned to word times (exact start/end from the
  alignment stage contract in §1.2).

## 4. Cross-cutting golden rules

1. Cut math (word edges → clip list) is deterministic and compared exactly.
2. Signal-derived values (alignment, motion, audio peaks) carry declared
   tolerances recorded beside each fixture.
3. Every rejected upstream strategy (§1.6) ships as a negative golden so a
   regression to the dead end fails loudly.
4. Finish stages receive a locked cut and must refuse mutation requests
   (typed refusal, not silent ignore).
5. Fixtures derive from pinned sample media audited under the
   `attached-cutaway-finish-material` licence row; no fixture may pull
   bytes from the network.
