// Studio snapshot/project types shared across App, hooks, modes, and
// components. Moved out of main.tsx per REV2 §14.4 — pure move, no
// behavior change.

export type Mode = "sources" | "compare" | "finals" | "qa" | "settings";

// The three parked register variants (redesign spec Phase 2) — base/accent/
// density/type token themes over one shared component structure, applied
// via `data-register` on the document root. R1 is the default until Adrian
// picks one; the QA-only RegisterSwitch (visible behind `?qa=1`) exists so
// all three can be screenshotted from one running instance.
export type Register = "cutting-room" | "bench" | "screening-room";
export const REGISTER_ORDER: Register[] = [
  "cutting-room",
  "bench",
  "screening-room",
];
export const REGISTER_LABEL: Record<Register, string> = {
  "cutting-room": "R1 · Cutting Room",
  bench: "R2 · Bench",
  "screening-room": "R3 · Screening Room",
};

// Single source of truth for mode order/labels so the mode tabs, the
// command palette, and the ⌘1-5 keyboard shortcuts can't drift from one
// another (previously each hardcoded its own copy of this list).
export const MODE_ORDER: Mode[] = [
  "sources",
  "compare",
  "finals",
  "qa",
  "settings",
];
export const MODE_LABEL: Record<Mode, string> = {
  sources: "sources",
  compare: "compare",
  finals: "finals",
  qa: "QA",
  settings: "settings",
};

// The true state of an optional filesystem artifact (REV2 §12.1): a `Ready`
// artifact is distinct from `Missing` (never generated), which is distinct
// from `Invalid` (generated but failed to parse — corruption, not absence)
// and `Stale` (parsed fine but superseded by a newer render/edit). The
// backend still sends the original `qa`/`bench`/`cut_plan` fields for
// compatibility; these `*_artifact` fields carry the corrected state.
export type ArtifactState<T> =
  | { state: "missing" }
  | { state: "ready"; data: T }
  | { state: "invalid"; data: { path: string; error: string } }
  | { state: "stale"; data: { path: string; reason: string } };

export function artifactIssue<T>(state?: ArtifactState<T> | null): string | null {
  if (!state) return null;
  if (state.state === "invalid") return `corrupt — ${state.data.error}`;
  if (state.state === "stale") return `stale — ${state.data.reason}`;
  return null;
}

export type Variant = {
  id: string;
  mp4?: string | null;
  fps?: number;
  duration_ms?: number;
  cut_plan?: { segments?: Segment[] } | null;
  cut_plan_artifact?: ArtifactState<{ segments?: Segment[] }> | null;
};
export type Segment = {
  id?: string;
  source_id?: string;
  source_start_ms?: number;
  source_end_ms?: number;
  output_start_ms?: number;
  output_end_ms?: number;
  label?: string;
};
// Outcome of the backend's exact-file asset-scope grant for one registered
// source (REV2 §12.4): `granted` means playback scope was extended to this
// file; `verified` means its current BLAKE3 matches the manifest. A source
// can be granted-but-unverified (still playable, flagged) or ungranted
// entirely (not a regular file, or fails to probe as supported media).
export type SourceIntegrity = {
  source_id: string;
  path: string;
  granted: boolean;
  verified: boolean;
  error?: string | null;
} | null;
export type Source = {
  source_id: string;
  path?: string;
  display_name?: string;
  duration_ms?: number;
  width?: number;
  height?: number;
  is_hdr?: boolean;
  file_present?: boolean;
  stages?: Record<string, boolean>;
  transcript?: string | null;
  poster_jpg?: string | null;
  waveform_png?: string | null;
  integrity?: SourceIntegrity;
};
export type Snapshot = {
  project_path: string;
  generated_at: string;
  project_revision?: string;
  project_instance_id?: string;
  manifest: { project_id?: string; title?: string };
  sources: Source[];
  stages: Record<string, boolean>;
  variants: Variant[];
  finals: Array<{
    preset: string;
    aspect?: string;
    mp4?: string | null;
    duration_ms?: number;
    width?: number;
    height?: number;
  }>;
  qa?: {
    status?: "pass" | "fail";
    checks?: Array<{ id: string; status: "pass" | "fail"; evidence?: string }>;
  } | null;
  qa_artifact?: ArtifactState<Snapshot["qa"]> | null;
  bench?: { decision?: string };
  bench_artifact?: ArtifactState<{ decision?: string; report?: string }> | null;
  decisions_path?: string;
};

// Per-project optional-cloud-analysis settings (REV2 §15.6). Mirrors
// `src-tauri/src/settings.rs::CloudSettings` — see that module's docs for
// why `credential_env_var` is a NAME, never a credential value, and why
// `provider` cannot yet be anything but "disabled".
export type UploadPolicy = "proxy" | "source";
export type CloudSettings = {
  schema_version: number;
  consent: boolean;
  hard_budget_usd: number;
  upload_policy: UploadPolicy;
  provider: string;
  credential_env_var?: string | null;
  updated_at?: string | null;
};
export const DEFAULT_CLOUD_SETTINGS: CloudSettings = {
  schema_version: 1,
  consent: false,
  hard_budget_usd: 0,
  upload_policy: "proxy",
  provider: "disabled",
  credential_env_var: null,
  updated_at: null,
};

export type EngineCapabilities = {
  has_zscale: boolean;
  has_h264_videotoolbox: boolean;
  has_prores_ks: boolean;
  has_lut3d: boolean;
  has_colortemperature: boolean;
};
export type EngineStatus = {
  resolved: boolean;
  toolchain_identity?: string | null;
  ffmpeg_version?: string | null;
  ffmpeg_path?: string | null;
  ffprobe_path?: string | null;
  capabilities?: EngineCapabilities | null;
  error?: string | null;
  note: string;
};
