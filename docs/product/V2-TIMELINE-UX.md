# CutRight v2 Timeline Authoring UX and Corrective Operation Scope

## 1. Purpose

This document freezes the timeline authoring surface for v2 Studio. It
defines the data model (`StudioTimelineView`), the corrective operation
vocabulary, and the explicit non-NLE breadth promise.

This contract is owned by the serial freeze task `CR-V2-B6-004` and Lane B
(`CR-V2-B6-012` through `CR-V2-B6-016`). Lanes A and C must consume this
contract; they must not redefine it.

## 2. Authoritative schema

`schemas/studio/timeline-view.schema.v1.json` is the wire shape of the
timeline view projection. The schema is closed (`additionalProperties: false`)
and the time fields are `RationalTime` (`{ num, den }`).

- `source_in` and `source_out` are positions in the **source timebase**.
- `timeline_start` and `duration` are positions in the **project timebase**.
- Conversion between timebases happens **only in the kernel**. The
  frontend never converts between timebases on its own.

## 3. Tracks, clips, linked media

```ts
type TrackKind = "video" | "audio" | "overlay" | "caption" | "music" | "sfx";
```

Audio tracks are *linked* to a video track via `linked_track_id`. Linked
behaviour is **not** inferred from track indexes. Splitting a clip on the
video track leaves the audio untouched unless the user explicitly splits
the linked audio clip too.

A clip carries:

- `clip_id` (stable, monotonic per timeline revision),
- `source_id` (immutable source reference),
- `media_revision` (the source's media revision when the clip was placed),
- `timeline_start`, `duration`, `source_in`, `source_out` (rational-time
  positions),
- optional `volume`, `fade_in`, `fade_out`, `crop_anchor`, `effect_ids`,
  `caption_ids`, `overlay_ids`, `keyframes`.

## 4. v2 corrective operation vocabulary

The v2 timeline supports exactly these operations. Each operation has an
`action_id`, an `acceptance_case`, and a registered `risk_band`. No other
timeline edit is allowed through the UI.

| Action ID              | Verb            | Risk | Notes                                        |
| ---------------------- | --------------- | ---- | -------------------------------------------- |
| `trim_clip`            | trim            | med  | adjust `source_in`/`source_out`              |
| `split_clip`           | split           | med  | at playhead or explicit frame                |
| `remove_clip`          | remove          | med  | leaves a gap (use `ripple_clip` for close-up)|
| `ripple_clip`          | remove + shift  | med  | close gap, shift subsequent clips            |
| `restore_clip`         | restore         | med  | from the removed-passage history             |
| `move_clip`            | move            | med  | change `timeline_start` and/or track         |
| `swap_take`            | swap            | med  | replace a clip with a different take/media   |
| `reorder_beat`         | reorder         | med  | moves the underlying beats                   |
| `set_volume`           | volume          | low  | set a clip's `volume`                        |
| `set_fade`             | fade            | low  | set `fade_in`/`fade_out`                     |
| `change_crop_anchor`   | crop            | low  | set `crop_anchor.x/y`                        |
| `edit_caption`         | caption         | low  | edit caption text/style                      |
| `edit_graphic`         | graphic         | low  | edit graphic text/layout                     |
| `enable_effect`        | enable          | low  | mark an effect_id active                     |
| `disable_effect`       | disable         | low  | mark an effect_id inactive                   |
| `set_keyframe`         | keyframe        | med  | add / update / delete one keyframe            |
| `undo` / `redo`        | undo            | low  | round-trip through the executor              |

Anything outside this list is rejected by the executor. The UI may show a
greyed-out affordance that explains why; it must not silently widen the
vocabulary.

## 5. Composited inspection vs source inspection

- **Source inspection** reads from the immutable source bytes. It answers
  "what did the source say / show at time T?" and is the basis for
  transcript corrections, beat labels, and refra me anchors.
- **Composited inspection** reads from the rendered timeline through the
  native graph. It answers "what does the timeline show at frame F?" and
  is the basis for QA, critic findings, and visual sample sheets.

The two are distinct surfaces. The UI never conflates them; the
embedded agent never conflates them.

## 6. Non-NLE breadth promise

v2 does not promise full Premiere / Resolve / FCPX parity. In particular:

- No multi-cam live switching.
- No nested compound clips.
- No nested sequences.
- No native motion-graphics editor (Design mode handles style
  generation; corrective graphics edits handle one-off edits).
- No plug-in marketplace.

Anything not listed in §4 is out of scope and is rejected by the executor.

## 7. Lane ownership

Lane B (`CR-V2-B6-012` through `CR-V2-B6-016`) owns the timeline mode,
timeline components, the timeline hook, and the corrective operation
workflows. Lane A may consume the read model from the Timeline mode; Lane
C may compose agent plans that emit timeline actions but must never own
UI state. The wire schema and the kernel conversion rules are shared and
not editable by any lane.

## 8. Anti-promises

- The UI never converts between timebases itself.
- The UI never infers linked-media behaviour from track indexes.
- The UI never lets the user bypass the corrective operation vocabulary
  by editing JSON.
- The UI never persists UI state into the canonical project JSON.