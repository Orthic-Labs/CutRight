// Studio snapshot/project types shared across App, hooks, modes, and
// components. Moved out of main.tsx per REV2 §14.4 — pure move, no
// behavior change.

export type Mode = "sources" | "compare" | "finals" | "qa";

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
