// apps/studio/src/contracts/projectIndex.ts
//
// Frozen Studio project index contract for Book 6 task CR-V2-B6-002.
// Mirrors `schemas/studio/project-index.schema.v1.json` and
// `docs/architecture/V2-PROJECT-INDEX.md`.
//
// The index is a *disposable projection*. Deleting the file loses no
// project truth. Two rows may share a `title`; identity is `project_instance_id`.

export type LaneId =
  | "recorded_footage"
  | "repurpose"
  | "explainer"
  | "anchored_creative";

export const LANE_IDS: readonly LaneId[] = [
  "recorded_footage",
  "repurpose",
  "explainer",
  "anchored_creative",
] as const;

export type RunStatus =
  | "idle"
  | "running"
  | "ready"
  | "needs_review"
  | "failed"
  | "stale"
  | "missing";

export const RUN_STATUSES: readonly RunStatus[] = [
  "idle",
  "running",
  "ready",
  "needs_review",
  "failed",
  "stale",
  "missing",
] as const;

export const PROJECT_INDEX_SCHEMA = "cutright.studio.project_index/v1" as const;

export type ProjectIndexRow = {
  readonly project_instance_id: string;
  readonly package_path: string;
  readonly title: string;
  readonly lane: LaneId;
  readonly active_revision: string;
  readonly run_status: RunStatus;
  readonly ready_count: number;
  readonly needs_review_count: number;
  readonly failed_count: number;
  readonly updated_at: string;
  readonly thumbnail_hash?: string;
};

export type ProjectIndex = {
  readonly schema: typeof PROJECT_INDEX_SCHEMA;
  readonly version: number;
  readonly rows: readonly ProjectIndexRow[];
  readonly watch_folder_import_enabled: boolean;
};

// Type guard used at the IPC boundary. The Rust backend always returns the
// canonical shape; this guard exists so the frontend never crashes on a
// stale file when the schema version is older than the running app.
export function isProjectIndex(value: unknown): value is ProjectIndex {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  if (v.schema !== PROJECT_INDEX_SCHEMA) return false;
  if (typeof v.version !== "number") return false;
  if (!Array.isArray(v.rows)) return false;
  if (typeof v.watch_folder_import_enabled !== "boolean") return false;
  return true;
}

export function isProjectIndexRow(value: unknown): value is ProjectIndexRow {
  if (typeof value !== "object" || value === null) return false;
  const r = value as Record<string, unknown>;
  if (typeof r.project_instance_id !== "string") return false;
  if (typeof r.package_path !== "string") return false;
  if (typeof r.title !== "string") return false;
  if (typeof r.lane !== "string") return false;
  if (!(LANE_IDS as readonly string[]).includes(r.lane)) return false;
  if (typeof r.active_revision !== "string") return false;
  if (typeof r.run_status !== "string") return false;
  if (!(RUN_STATUSES as readonly string[]).includes(r.run_status)) return false;
  for (const k of ["ready_count", "needs_review_count", "failed_count"] as const) {
    if (typeof r[k] !== "number") return false;
  }
  if (typeof r.updated_at !== "string") return false;
  return true;
}

// Display helpers. Frontends must never invent a `run_status` from
// free-form strings; the backend is the only source.
export function runStatusLabel(status: RunStatus): string {
  switch (status) {
    case "idle":
      return "Idle";
    case "running":
      return "Running";
    case "ready":
      return "Ready";
    case "needs_review":
      return "Needs review";
    case "failed":
      return "Failed";
    case "stale":
      return "Stale";
    case "missing":
      return "Missing";
  }
}

// Stable sort key for the Home grid. Most recently updated first; ties
// break on `project_instance_id` so the order is deterministic.
export function compareProjectIndexRows(
  a: ProjectIndexRow,
  b: ProjectIndexRow,
): number {
  const aT = Date.parse(a.updated_at);
  const bT = Date.parse(b.updated_at);
  if (aT !== bT) return bT - aT;
  return a.project_instance_id.localeCompare(b.project_instance_id);
}