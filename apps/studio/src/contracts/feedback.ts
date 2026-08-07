/**
 * apps/studio/src/contracts/feedback.ts
 *
 * Studio contract for the v2 feedback loop. Mirrors the Rust types in
 * crates/video-feedback/src/decision.rs. The Studio never invents a
 * category; enums match the JSON schema exactly.
 */

export type DecisionTarget =
  | "segment"
  | "beat"
  | "take"
  | "boundary"
  | "caption"
  | "graphic"
  | "effect"
  | "audio"
  | "crop"
  | "final";

export type DecisionAction =
  | "approve"
  | "reject"
  | "replace"
  | "trim"
  | "extend"
  | "reorder"
  | "mute"
  | "revoice"
  | "restyle"
  | "regenerate"
  | "reframe"
  | "rerender"
  | "note";

export type DecisionReason =
  | "take_choice"
  | "boundary_choice"
  | "filler_choice"
  | "pause_choice"
  | "hook_choice"
  | "cta_choice"
  | "beat_order"
  | "crop_choice"
  | "caption_choice"
  | "graphic_choice"
  | "effect_density"
  | "broll_choice"
  | "sfx_choice"
  | "music_choice"
  | "color_choice"
  | "audio_choice"
  | "identity_choice"
  | "final_verdict"
  | "unknown_reason";

export type DecisionAxis =
  | "take"
  | "boundary"
  | "filler"
  | "pause"
  | "hook"
  | "cta"
  | "beat_order"
  | "crop"
  | "caption"
  | "graphic"
  | "motion"
  | "broll"
  | "sfx"
  | "music"
  | "color"
  | "audio"
  | "identity"
  | "final"
  | "unsupported_axis";

export type UserOrigin =
  | "user_reviewed"
  | "user_rejected"
  | "user_replaced"
  | "user_noted"
  | "model_suggested"
  | "system";

export type SessionOrigin =
  | "external_session"
  | "studio_review"
  | "studio_autonomous"
  | "headless";

export type ReviewMode = "reviewed" | "review_light" | "autonomous";

export interface FormatKey {
  content_type: string;
  platform: string;
  variant: string;
}

export interface DecisionRecord {
  schema_version: "v2";
  decision_id: string;
  prev_hash: string;
  record_hash: string;
  project_instance_id: string;
  project_revision: string;
  subject_hash: string;
  decision_target: DecisionTarget;
  decision_action: DecisionAction;
  decision_reason: DecisionReason;
  decision_axis: DecisionAxis;
  delta: unknown;
  format_key: FormatKey;
  pack_set_id: string;
  pack_set_fingerprint: string;
  app_version: string;
  user_origin: UserOrigin;
  session_origin: SessionOrigin;
  asset_hash: string | null;
  effect_id: string | null;
  final_hash: string | null;
  review_mode: ReviewMode;
  sample_count: number;
  confidence: number;
  stale_subject: boolean;
  malformed: boolean;
  note: string | null;
  created_at: string;
}

export const DECISION_REASONS: readonly DecisionReason[] = [
  "take_choice",
  "boundary_choice",
  "filler_choice",
  "pause_choice",
  "hook_choice",
  "cta_choice",
  "beat_order",
  "crop_choice",
  "caption_choice",
  "graphic_choice",
  "effect_density",
  "broll_choice",
  "sfx_choice",
  "music_choice",
  "color_choice",
  "audio_choice",
  "identity_choice",
  "final_verdict",
  "unknown_reason",
];

export const DECISION_AXES: readonly DecisionAxis[] = [
  "take",
  "boundary",
  "filler",
  "pause",
  "hook",
  "cta",
  "beat_order",
  "crop",
  "caption",
  "graphic",
  "motion",
  "broll",
  "sfx",
  "music",
  "color",
  "audio",
  "identity",
  "final",
  "unsupported_axis",
];
