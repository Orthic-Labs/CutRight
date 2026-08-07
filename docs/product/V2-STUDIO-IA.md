# CutRight v2 Studio Information Architecture

This document freezes the v2 Studio navigation graph, route state shape, and the
read model each mode consumes. It is the authoritative reference for Book 6
freeze tasks `CR-V2-B6-001` through `CR-V2-B6-006` and for the lane merges that
follow. Lanes A, B, and C must not redefine the IA inside their exclusive paths.

## 1. Scope

Studio is the only user-facing surface for v2. It is a deterministic authoring
shell, not a generic NLE. Every mode below has a frozen name, frozen read model,
frozen deep-link shape, and a registered capability gate. The schema for
navigation state lives at `schemas/studio/navigation.schema.v1.json`; the
schema for the per-mode project view lives at
`schemas/studio/project-view.schema.v1.json`.

## 2. Frozen modes

The Studio mode vocabulary is closed. Names are stable and appear in deep
links, persistence, IPC, and accessibility labels. They must not be translated
in the route state.

```ts
type StudioMode =
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
```

| Mode             | Owner lane | Primary read model            | Notes                                  |
| ---------------- | ---------- | ----------------------------- | -------------------------------------- |
| home             | P-A        | ProjectIndexRow list          | rebuildable index, lane badges         |
| sources          | P-A        | SourceManifest + probe facts  | immutable bytes, hashes                |
| transcript       | P-A        | Transcript + corrected layer  | text correction is separate from cuts  |
| story            | P-A        | StoryPlan arc                 | arc + score components + chronology     |
| beats            | P-A        | BeatPlan + takes              | selected / alternates / signals        |
| timeline         | P-B        | Timeline revision view        | non-destructive editor                 |
| design           | P-B        | CreativePlan + AssetReview    | brand / directions / Designer findings |
| motion-sound     | P-B        | MotionPlan + audio graph      | bounded props + auditions              |
| compare          | P-A        | Variant list + critic findings| A/B sync, swap, sample                 |
| finals           | P-A        | FinalSet + package assets     | selection history preserved            |
| qa               | P-A        | QA report + receipt tree      | critic verdict, tamper / stale states  |
| settings         | shared     | Settings + capability matrix  | enable / disable MCP, autonomy policy  |

## 3. Route state shape

Route state is shallow, deterministic, and never mutates canonical project
state. UI state (selection, playhead, focused panel, expanded sections) lives
in a sibling slice and is dropped on route change.

```ts
type RouteState = {
  schema: "cutright.studio.navigation/v1";
  mode: StudioMode;
  project_id?: string;
  revision?: string;
  timeline_id?: string;
  evidence_id?: string;
  object_id?: string;
  capability_unavailable?: boolean; // unknown / missing capability marker
};
```

Rules:

1. `project_id` and `revision` are required to enter any mode other than
   `home` and `settings`. The router surfaces a degraded state otherwise.
2. `timeline_id` is required for `timeline`, `design`, `motion-sound`,
   `compare`, `finals`, and `qa`.
3. `evidence_id` is set when the user opens a specific evidence node from
   sources, transcript, story, beats, or compare.
4. `object_id` is set when the user opens a composited inspection target from
   the embedded agent or from `timeline`.
5. An unknown or unavailable mode degrades visibly. The router replaces the
   mode body with a `ModeUnavailable` panel that lists the missing capability
   and a pointer to the closest reachable mode.
6. Selection / playhead / focused panel state is never serialized into the
   route and never written into the canonical project JSON.

## 4. Project read model

The per-mode project view is assembled from canonical revision / evidence /
job / decision data plus disposable index metadata. The contract is
`schemas/studio/project-view.schema.v1.json`. The read model is a *projection*;
it is rebuilt from the project package on demand and never authoritative.

```ts
type ProjectView = {
  schema: "cutright.studio.project_view/v1";
  project: {
    project_id: string;
    project_instance_id: string; // Studio-owned immutable identity
    title: string;
    lane: "recorded_footage" | "repurpose" | "explainer" | "anchored_creative";
    active_revision: string;
    updated_at: string; // ISO 8601, monotonic per project
  };
  revision: {
    revision: string;
    parent_revision: string | null;
    digest: {
      ready: number;
      needs_review: number;
      failed: number;
      running: number;
    };
  };
  evidence: {
    sources: SourceSummary[];
    transcript_present: boolean;
    story_plan_present: boolean;
    beat_count: number;
    timeline_present: boolean;
  };
  jobs: {
    active: JobSummary[];
    recent: JobSummary[];
  };
  capabilities: {
    available: CapabilityId[];
    missing: CapabilityId[];
  };
};
```

## 5. Capability gating

Every mode declares a required capability set. Missing capabilities degrade
visibly without crashing the router. The capability list is sourced from the
shared registry; this IA document only freezes the names and the gating rule.

## 6. Deep links

Deep links are URL-shaped strings of the form
`cutright://<project_id>/<mode>?revision=<rev>&timeline=<tl>&evidence=<ev>&object=<obj>`.
Hosts other than `cutright://` are rejected at parse time and never reach the
router.

## 7. Lane ownership

- Lane A (`home`, `sources`, `transcript`, `story`, `beats`, `run`,
  `compare`, `finals`, `qa`): lane A owns the core workflow.
- Lane B (`timeline`, `design`, `motion-sound`, `assets`, `auditions`,
  `corrections`): lane B owns authoring.
- Lane C (embedded agent, tool registry, composited inspection, MCP,
  accessibility, performance): lane C owns agent + inspection + a11y / perf.

No lane may redefine a mode owned by another lane. Shared services (the
executor, the project read model, the capability registry) live outside any
single lane and are wired by the serial merge tasks `CR-V2-B6-022` onwards.

## 8. Anti-promises

- Studio does not promise full NLE breadth beyond registered actions.
- Studio does not mutate canonical project JSON from UI state alone.
- Studio does not embed chain-of-thought, internal planner reasoning, or
  model scaffolding in any visible surface.
