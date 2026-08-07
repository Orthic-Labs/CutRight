// apps/studio/src/contracts/navigation.ts
//
// Frozen Studio navigation contract for Book 6 task CR-V2-B6-001.
// Mirrors `schemas/studio/navigation.schema.v1.json` and `docs/product/V2-STUDIO-IA.md`.
// UI state (selection, playhead, focused panel, expanded sections) lives in a
// sibling slice and is dropped on route change. This contract describes only
// the shallow deterministic route state.

export type StudioMode =
  | "home"
  | "sources"
  | "transcript"
  | "story"
  | "beats"
  | "timeline"
  | "design"
  | "motion-sound"
  | "compare"
  | "finals"
  | "qa"
  | "settings";

export const STUDIO_MODES: readonly StudioMode[] = [
  "home",
  "sources",
  "transcript",
  "story",
  "beats",
  "timeline",
  "design",
  "motion-sound",
  "compare",
  "finals",
  "qa",
  "settings",
] as const;

// Modes that require a project_id + revision to enter.
export const PROJECT_BOUND_MODES: readonly StudioMode[] = [
  "sources",
  "transcript",
  "story",
  "beats",
  "timeline",
  "design",
  "motion-sound",
  "compare",
  "finals",
  "qa",
] as const;

// Modes that additionally require a timeline_id.
export const TIMELINE_BOUND_MODES: readonly StudioMode[] = [
  "timeline",
  "design",
  "motion-sound",
  "compare",
  "finals",
  "qa",
] as const;

export const STUDIO_NAVIGATION_SCHEMA = "cutright.studio.navigation/v1" as const;

export type RouteState = {
  readonly schema: typeof STUDIO_NAVIGATION_SCHEMA;
  readonly mode: StudioMode;
  readonly project_id?: string;
  readonly revision?: string;
  readonly timeline_id?: string;
  readonly evidence_id?: string;
  readonly object_id?: string;
  readonly capability_unavailable?: boolean;
};

// Type guard for a value coming back from JSON.parse / IPC. Used at the
// router boundary so unknown shapes degrade visibly without crashing.
export function isStudioMode(value: unknown): value is StudioMode {
  return (
    typeof value === "string" &&
    (STUDIO_MODES as readonly string[]).includes(value)
  );
}

export function routeRequiresProject(mode: StudioMode): boolean {
  return (PROJECT_BOUND_MODES as readonly StudioMode[]).includes(mode);
}

export function routeRequiresTimeline(mode: StudioMode): boolean {
  return (TIMELINE_BOUND_MODES as readonly StudioMode[]).includes(mode);
}

// Deep-link parser. Accepts only `cutright://` hosts; everything else is
// rejected and never reaches the router. Returns null on parse failure so
// callers can fall back to the home route.
export function parseDeepLink(
  href: string,
): { route: RouteState } | null {
  if (!href.startsWith("cutright://")) return null;
  const tail = href.slice("cutright://".length);
  const [pathPart, queryPart] = tail.split("?", 2);
  const pathSegments = pathPart.split("/").filter((s) => s.length > 0);
  if (pathSegments.length < 2) return null;

  const projectId = pathSegments[0];
  const mode = pathSegments[1];
  if (!isStudioMode(mode)) return null;

  const params = new URLSearchParams(queryPart ?? "");
  const route: {
    schema: typeof STUDIO_NAVIGATION_SCHEMA;
    mode: StudioMode;
    project_id: string;
    revision?: string;
    timeline_id?: string;
    evidence_id?: string;
    object_id?: string;
  } = {
    schema: STUDIO_NAVIGATION_SCHEMA,
    mode,
    project_id: projectId,
  };
  const revision = params.get("revision");
  const timeline = params.get("timeline");
  const evidence = params.get("evidence");
  const object = params.get("object");
  if (revision !== null) route.revision = revision;
  if (timeline !== null) route.timeline_id = timeline;
  if (evidence !== null) route.evidence_id = evidence;
  if (object !== null) route.object_id = object;

  return { route: route as RouteState };
}