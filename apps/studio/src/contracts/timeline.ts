// apps/studio/src/contracts/timeline.ts
//
// Frozen Studio timeline view contract for Book 6 task CR-V2-B6-004.
// Mirrors `schemas/studio/timeline-view.schema.v1.json` and
// `docs/product/V2-TIMELINE-UX.md`.
//
// `source_in`/`source_out` are in the source timebase; `timeline_start`/
// `duration` are in the project timebase. Conversion between timebases
// happens only in the kernel; the frontend never converts.

export const TIMELINE_VIEW_SCHEMA = "cutright.studio.timeline_view/v1" as const;

export type RationalTime = { readonly num: number; readonly den: number };
export type Timebase = { readonly num: number; readonly den: number };

export const TRACK_KINDS: readonly TrackKind[] = [
  "video",
  "audio",
  "overlay",
  "caption",
  "music",
  "sfx",
] as const;

export type TrackKind =
  | "video"
  | "audio"
  | "overlay"
  | "caption"
  | "music"
  | "sfx";

export type CropAnchor = { readonly x: number; readonly y: number };

export type Keyframe = {
  readonly kf_id: string;
  readonly at: RationalTime;
  readonly props: Readonly<Record<string, unknown>>;
};

export type Clip = {
  readonly clip_id: string;
  readonly source_id: string;
  readonly media_revision?: string;
  readonly timeline_start: RationalTime;
  readonly duration: RationalTime;
  readonly source_in: RationalTime;
  readonly source_out: RationalTime;
  readonly volume?: number;
  readonly fade_in?: RationalTime;
  readonly fade_out?: RationalTime;
  readonly crop_anchor?: CropAnchor;
  readonly effect_ids?: readonly string[];
  readonly caption_ids?: readonly string[];
  readonly overlay_ids?: readonly string[];
  readonly keyframes?: readonly Keyframe[];
};

export type Track = {
  readonly track_id: string;
  readonly kind: TrackKind;
  readonly linked_track_id?: string;
  readonly clips: readonly Clip[];
};

export type TimelineSelection = {
  readonly clip_ids?: readonly string[];
  readonly playhead?: RationalTime;
};

export type TimelineView = {
  readonly schema: typeof TIMELINE_VIEW_SCHEMA;
  readonly timeline_id: string;
  readonly revision: string;
  readonly duration: { readonly rational: RationalTime; readonly timebase: Timebase };
  readonly tracks: readonly Track[];
  readonly selection?: TimelineSelection;
};

// Frozen corrective operation vocabulary. The UI maps each action verb to
// an `ActionIntent` from `contracts/actionIntent.ts`; nothing outside this
// list is allowed.
export const TIMELINE_ACTIONS = {
  trim_clip: "trim",
  split_clip: "split",
  remove_clip: "remove",
  ripple_clip: "ripple",
  restore_clip: "restore",
  move_clip: "move",
  swap_take: "swap",
  reorder_beat: "reorder",
  set_volume: "volume",
  set_fade: "fade",
  change_crop_anchor: "crop",
  edit_caption: "caption",
  edit_graphic: "graphic",
  enable_effect: "enable",
  disable_effect: "disable",
  set_keyframe: "keyframe",
  undo: "undo",
  redo: "redo",
} as const;

export type TimelineActionId = keyof typeof TIMELINE_ACTIONS;

export const TIMELINE_ACTION_IDS: readonly TimelineActionId[] = [
  "trim_clip",
  "split_clip",
  "remove_clip",
  "ripple_clip",
  "restore_clip",
  "move_clip",
  "swap_take",
  "reorder_beat",
  "set_volume",
  "set_fade",
  "change_crop_anchor",
  "edit_caption",
  "edit_graphic",
  "enable_effect",
  "disable_effect",
  "set_keyframe",
  "undo",
  "redo",
] as const;

// Risk band per action. The executor enforces this; the UI uses it to
// decide whether to show the confirmation modal.
export const TIMELINE_ACTION_RISK: Record<TimelineActionId, "low" | "medium" | "high"> = {
  trim_clip: "medium",
  split_clip: "medium",
  remove_clip: "medium",
  ripple_clip: "medium",
  restore_clip: "medium",
  move_clip: "medium",
  swap_take: "medium",
  reorder_beat: "medium",
  set_volume: "low",
  set_fade: "low",
  change_crop_anchor: "low",
  edit_caption: "low",
  edit_graphic: "low",
  enable_effect: "low",
  disable_effect: "low",
  set_keyframe: "medium",
  undo: "low",
  redo: "low",
};

// Rational-time helpers used by UI selection. The kernel remains the
// source of truth for cross-timebase arithmetic; these helpers only
// format and compare within a single timebase.
export function compareRationalTime(a: RationalTime, b: RationalTime): number {
  // Compare a.num/b.den vs b.num/a.den without floating-point.
  const lhs = a.num * b.den;
  const rhs = b.num * a.den;
  if (lhs < rhs) return -1;
  if (lhs > rhs) return 1;
  return 0;
}

export function formatRationalTime(rt: RationalTime): string {
  // Frame number if den == 1, else fractional seconds.
  if (rt.den === 1) return `${rt.num}f`;
  const seconds = rt.num / rt.den;
  return `${seconds.toFixed(3)}s`;
}

// Type guards for IPC boundary validation.
export function isRationalTime(value: unknown): value is RationalTime {
  if (typeof value !== "object" || value === null) return false;
  const r = value as Record<string, unknown>;
  return typeof r.num === "number" && typeof r.den === "number" && r.den >= 1;
}

export function isClip(value: unknown): value is Clip {
  if (typeof value !== "object" || value === null) return false;
  const c = value as Record<string, unknown>;
  return (
    typeof c.clip_id === "string" &&
    typeof c.source_id === "string" &&
    isRationalTime(c.timeline_start) &&
    isRationalTime(c.duration) &&
    isRationalTime(c.source_in) &&
    isRationalTime(c.source_out)
  );
}

export function isTimelineActionId(value: unknown): value is TimelineActionId {
  return (
    typeof value === "string" &&
    (TIMELINE_ACTION_IDS as readonly string[]).includes(value)
  );
}