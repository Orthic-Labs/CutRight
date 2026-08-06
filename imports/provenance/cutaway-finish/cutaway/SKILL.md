---
name: shortform-cutaway
description: "A repeatable SHORT-FORM rough-cut editor for 9:16 talking-head clips, any style. Turns a raw camera ramble — where you talk to the AI, redo takes, pause, point, move the camera — into a TIGHT, FLOWING cut. THE METHOD: WhisperX FORCED ALIGNMENT gives exact per-word timestamps → cut on the precise word edges (every word whole, no early starts, no clipped ends) → remove only the no-word gaps (silences) → the model picks the red thread (one clean take per beat, story hook→…→CTA, drop folds/doubles/ums/meta). Use whenever you hand a recent short-form clip and want it cut: 'remove the silences', 'make the rough cut', 'cut this short', 'make it flow', 'punchy version'. Does NOT do captions/music/grade/reframe (= styles stage: shortform-finish). NOT long-form."
---

# SHORTFORM CUTAWAY — the rough-cut editor

> **THE JOB:** WhisperX tells you EXACTLY where every word starts and ends → cut on those edges →
> keep every word whole → remove only the gaps where no word is spoken → the model arranges the
> kept takes into a flowing red thread. Output = a 9:16 rough-cut **cut list + matching MP4**, which
> you then drop into whichever editor you use.

This skill is the STAGE-1 rough cut: it picks the takes and removes silence. The STAGE-2 polish
(reframe, zooms, captions, music) is the separate `shortform-finish` skill.

---

## ⭐ FIRST RUN — which editor do you use?

**Ask the user this once, then follow the matching branch below.** The cut math is identical for
everyone — WhisperX word edges → a list of `start/end` time pairs. The only difference is how that
list *lands* in your tool. Pick one:

- **(a) DaVinci Resolve** — the cut list becomes a timeline of clips (via the Resolve scripting API / MCP).
- **(b) Premiere Pro** — the cut list becomes an EDL / XML you import as a sequence.
- **(c) Remotion** — the cut list becomes an array of `{from,to}` segments your composition plays in order.
- **(d) Claude-Code-only (no NLE)** — the cut list is rendered straight to an MP4 with ffmpeg. No editor needed.

> If the user doesn't know, default to **(d)** — it needs no other software and produces a finished
> MP4 you can post directly.

### Branch (a) — DaVinci Resolve
`build_wx.py` already emits Resolve `clip_infos` (`media_pool_item_id / start_frame / end_frame /
record_frame`). Import your source into the media pool, get its item id, pass it as the `MEDIA_POOL_ITEM_ID`
arg, then feed the resulting JSON to `create_timeline_from_clips`. Set the source FPS arg if your footage
isn't 23.976 (`24000/1001`). **Don't auto-render — scrub the timeline first.**

### Branch (b) — Premiere Pro
Take the `# start-end` lines `build_wx.py` prints to stderr (seconds) and write them out as a CMX3600 EDL
or an FCP7 XML: one event per clip, source-in/out = the start/end seconds, record-in = cumulative. Import
the EDL/XML in Premiere → it builds the sequence pointing at your source file. (Any "EDL from cut list"
snippet works; the cut list is the only thing that matters.)

### Branch (c) — Remotion
Convert each clip to `{ from: startSec, to: endSec }` and map them to sequential `<Sequence>`s playing an
`<OffthreadVideo>` of your source, trimmed with `startFrom`/`endAt` (× fps). The cut list is the playlist.

### Branch (d) — Claude-Code-only (ffmpeg, no NLE)
Render each clip with `ffmpeg -ss START -i SRC -t DUR`, reframe to 9:16 in the same pass, then concat with
a re-encode. **Drive the per-clip loop from Python, not a bash `while read`** (bash eats stdin on the short
clips). Example reframe filter: `-vf "crop=ih*9/16:ih,scale=1080:1920,fps=30"`. This is the simplest path
and produces a finished, postable MP4.

---

## SETUP (do this once)

WhisperX is the engine. It needs **Python ≤ 3.12** and **torch**. The cleanest setup is a dedicated venv.

```bash
# 1. Create a Python 3.11 venv anywhere you like
python3.11 -m venv ~/wx-env            # <YOUR_VENV> — put it wherever you want
~/wx-env/bin/pip install whisperx

# 2. (optional) point the model cache somewhere with space — WhisperX downloads
#    a faster-whisper model + a ~360MB alignment model the first time.
export HF_HOME=~/wx-cache               # <YOUR_HF_CACHE> — any folder with a few GB free
```

Models download once to `HF_HOME`. On a CPU-only machine, alignment adds ~1–2 min per clip — worth it.
If you have an NVIDIA GPU, WhisperX will use it automatically and run far faster.

Everywhere below, `<YOUR_VENV>` = the venv you just made and `<YOUR_HF_CACHE>` = your `HF_HOME` folder.
The scripts in `scripts/` take all paths as arguments — there is nothing to hardcode.

---

## WHY WhisperX (the breakthrough — read this)
A "silence" = **a stretch where you say no words** — NOT a waveform dip. So the cut must be driven by
WORD positions. But **faster-whisper only *guesses* word times** — it drifts badly around pauses (it
will place a word inside a 2-second silence, or stretch one word to a fake 2.2s). That causes every
naive failure: clips starting too early, last words clipped, clips landing in silence.
**WhisperX transcribes with faster-whisper THEN force-aligns each word to the audio with a phoneme
model → real, exact start/end per word.** With those, cutting is trivial and correct.

---

## THE METHOD — run this

```bash
WORK=~/shortform-cut          # any working folder
mkdir -p "$WORK"/{audio,work}
SRC="/path/to/your/CLIP.MP4"  # your raw 9:16 talking-head clip

# 1. extract mono 16k audio
ffmpeg -y -i "$SRC" -vn -ac 1 -ar 16000 -c:a libmp3lame -q:a 5 "$WORK/audio/CLIP.mp3"

# 2. WhisperX forced alignment → EXACT word timestamps
HF_HOME=<YOUR_HF_CACHE> OMP_NUM_THREADS=4 \
  <YOUR_VENV>/bin/python scripts/whisperx_align.py \
  "$WORK/audio/CLIP.mp3" "$WORK/work/CLIP_wx.json"

# 3. EDITOR step (the model): read the words, write beats.txt — the RED THREAD (see below)

# 4. build the cut: exact word edges, remove no-word gaps > GAP (0.22 = good default)
#    The 3rd arg is your editor id / source ref:
#      Resolve → the media-pool item id;  any other editor → a label like "src".
python3 scripts/build_wx.py \
  "$WORK/work/CLIP_wx.json" beats.txt src 0.22 > "$WORK/work/ci.json"
# build_wx prints the clip list (start-end seconds + text) to stderr — that's your cut list.

# 5. land the cut in your editor — see the FIRST RUN branch you picked.
```

`OMP_NUM_THREADS=4` just keeps WhisperX from hogging every core. Tune freely.

### Step 3 — THE RED THREAD (the editor step; only the model can do this)
Read the WhisperX words. Write `beats.txt`, one beat per line `START END label`, choosing takes so the
story MOVES FORWARD: **hook → what it is → how → result → value → CTA → (fun button).** Rules:
- **One clean take per beat.** People redo lines until right → the **LATER take usually wins**.
- **Drop**: false starts, restarts ("let me start over"), doubles (a fact said twice), pointing/"I can
  point here" shots, pure meta ("grab another coffee", "this is just a test"), ums.
- **KEEP the real content.** "Pure meta" = test/logistics chatter and talking TO the AI about the edit.
  The actual thoughts and opinions = the CONTENT — do not drop those as a tangent. When unsure → keep.
- **Honor to-camera direction.** If the speaker directs the edit out loud (e.g. "alright, you can cut
  here"), that line is often the intended FUN ENDING — keep it if it lands.
- **Fix contradictions / wrong order** — assemble beats so it reads as ONE coherent thought, even if the
  source order was jumbled (put the cleanest hook first even if it was recorded last).
- **GAP dial** (`build_wx` arg): `0.22` is a good punchy-but-flowing default. Smaller = remove more
  pauses (more jump-cuts); larger = breathier. Tune per piece; the word edges are exact, so it's a clean dial.

See `examples/beats.example.txt` for the format (a synthetic, generic example — bring your own footage).

---

## ⛔ WHAT DID NOT WORK — dead ends (DO NOT REPEAT)
1. **Whisper as the silence cutter** — its *segments* smooth over pauses (they merge a long think-pause
   inside one sentence). Whisper/WhisperX READ words; loudness tools read amplitude; neither alone replaces
   the editorial red-thread step.
2. **Tight amplitude margins** — clip whole words off the ends ("This", "ASAP").
3. **faster-whisper WORD timestamps to place cuts** — faster-whisper mistimes words around pauses → silent
   camera-move clips, missed silences. **Word times must be ACCURATE → that's the whole reason for WhisperX.**
4. **Fixed-dB silencedetect** (`-30dB`) — fails when mic distance varies (close mic = room tone stays above
   threshold → pause undetected).
5. **Loudness-based auto-cut alone** — *good* but its edges are loudness-based not word-based, so it still
   starts slightly early / clips some word-ends and leaves some pauses. WhisperX edges fix exactly that.
6. **Over-aggressive auto-cut** — cuts quiet words mid-sentence and fragments into glitchy bits.

> **Not the cause (don't chase):** the NLE, the FPS, or "use Gemini to find the silences". The cut math is
> deterministic from the word edges; if a cut is wrong, it's the beats/GAP, not the tooling.

The live builder is **`build_wx.py`** on **`whisperx_align.py`** words. Other scripts in `scripts/`
(`build_cut.py`, `build_wordcut.py`, `build_clipinfos.py`, `flow_cut.py`) are documented dead ends kept for
reference; `motion_score.py` / `storyboard.sh` are optional aids for camera-move / framing checks.

## SCOPE
**DOES:** clip pick · WhisperX word alignment · word-exact cut (remove no-word gaps) · editor's red-thread
take/story selection · a cut list that lands in any editor + a matching MP4.
**DOES NOT (→ that's the `shortform-finish` skill):** captions · on-screen text · music · SFX · grade ·
9:16 reframe / punch-ins. Lock the rough cut here, then style it there.
