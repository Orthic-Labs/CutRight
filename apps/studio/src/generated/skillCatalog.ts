// Generated file: deterministic projection of skills/catalog.lock.json
// produced by CR-V2-B1-024. Do not edit by hand; regenerate from the lock.

export interface SkillCatalogResource {
  readonly path: string;
  readonly sha256: string;
  readonly bytes: number;
}

export interface SkillCatalogEntry {
  readonly id: string;
  readonly version: string;
  readonly description: string;
  readonly content_hash: string;
  readonly dependencies: readonly string[];
  readonly permissions: readonly string[];
  readonly resources: readonly SkillCatalogResource[];
  readonly order: number;
}

export interface SkillCatalog {
  readonly schema_version: number;
  readonly pack_id: string;
  readonly pack_hash: string;
  readonly registry_present: boolean;
  readonly capability_count: number;
  readonly skills: readonly SkillCatalogEntry[];
}

export const skillCatalog: SkillCatalog = Object.freeze({
  schema_version: 1,
  pack_id: "cutright-skill-pack-v1",
  pack_hash: "sha256:2f786f9bf312148c460f217ba6b836a3621d8251cfaae96d2fae67dca882ea30",
  registry_present: false,
  capability_count: 0,
  skills: Object.freeze([
    Object.freeze({
      id: "brand",
      version: "0.1.0",
      description: "Load brand voice, visuals, tone, and restrictions before creating branded content or design. Invoked as cutright://skill/brand {\\\"brand_code\\\":\\\"DD\\\"} (DD, RH, HR, TS, SS, VR, SR, CR, or MR), or when work names one of those ventures.",
      content_hash: "sha256:4af533a5df686e63643edeae60df578992808e7bf66bd297ff8b00afa07b084e",
      dependencies: Object.freeze([]),
      permissions: Object.freeze([]),
      resources: Object.freeze([]),
      order: 0,
    }),
    Object.freeze({
      id: "brand-identity",
      version: "0.1.0",
      description: "Create, audit, evolve, or apply brand identities, systems, guidelines, visual identity, voice, naming, logo direction, brand books, rebrands, website or app identity, pitch decks, or social kits.",
      content_hash: "sha256:14758940433e0043db9f1d27e36417c194e4e1140c6c6819e12fa7e41affb23f",
      dependencies: Object.freeze([]),
      permissions: Object.freeze([]),
      resources: Object.freeze([]),
      order: 1,
    }),
    Object.freeze({
      id: "content",
      version: "0.1.0",
      description: "Route media production: images, illustrations, motion or video, avatars, voiceover, Seedance, Remotion, enhancement, and transcription. Use when media is the deliverable; route UI to cutright://skill/designer, prose to cutright://skill/writing, and strategy to cutright://skill/social.",
      content_hash: "sha256:d5c462907b2923601c0f55c5936befea70c94b035ce4020836e1ab557f9def69",
      dependencies: Object.freeze([]),
      permissions: Object.freeze([]),
      resources: Object.freeze([]),
      order: 2,
    }),
    Object.freeze({
      id: "content-video-editor",
      version: "0.1.0",
      description: "Drive CutRight's validated local CLI (videoctl) end-to-end for hands-off captured-footage editing: ingest → transcribe/benchmark → candidate generation → natural+tight cut plans → review → variant selection → finish/captions/color → vertical reframe → QA → export/package. Reads structured project evidence, writes validated plans, records every decision in the project package.",
      content_hash: "sha256:1fe25b1b6a08fa9a0ab74150403161e680afd6461a951775fcdb5be250351f05",
      dependencies: Object.freeze([]),
      permissions: Object.freeze([]),
      resources: Object.freeze([]),
      order: 3,
    }),
    Object.freeze({
      id: "designer",
      version: "0.1.0",
      description: "Create, redesign, or polish websites, app UI, dashboards, components, static creative, print, motion systems, glass materials, illustration direction, and frontend craft. Route review-only work to cutright://skill/qa (visual_review mode) and identity systems to cutright://skill/brand-identity.",
      content_hash: "sha256:c01e5f2ec717fc0021079b89907fa3d90f0b300209789970d06a2fbe1a24fcaa",
      dependencies: Object.freeze([]),
      permissions: Object.freeze([]),
      resources: Object.freeze([]),
      order: 4,
    }),
    Object.freeze({
      id: "qa",
      version: "0.1.0",
      description: "Add, run, or audit app QA for local web or Tauri apps: hidden QA servers, deterministic mocks, functional assertions, viewport or selector captures, visual evidence, app-only captures, and contract-test authoring. Invoked as cutright://skill/qa (modes: functional, visual_review, capture, contract-tests).",
      content_hash: "sha256:f3ffa7c2cacf91bebcd702d69d84120cebeb0b93d746684e24778a33e7e4ebea",
      dependencies: Object.freeze([]),
      permissions: Object.freeze([]),
      resources: Object.freeze([]),
      order: 5,
    }),
    Object.freeze({
      id: "social",
      version: "0.1.0",
      description: "Route Instagram, Pinterest, YouTube, Twitter or X, LinkedIn, Reels, Shorts, pins, threads, calendars, and social growth strategy. Use cutright://skill/social when social strategy or platform-native content is the deliverable. Plan and copy only — no posting, scheduling, or account mutation.",
      content_hash: "sha256:1dd2ed3a9fe15d07f6f0aeda81f8262a3566171894501f685e86f57cdf387311",
      dependencies: Object.freeze([]),
      permissions: Object.freeze([]),
      resources: Object.freeze([]),
      order: 6,
    }),
    Object.freeze({
      id: "video-director",
      version: "0.1.0",
      description: "CutRight-local director that turns a topic or evidence set into a bounded, typed shot plan: narrative arc, beat map, per-shot size/camera/element-motion vocabulary, style bake-off, A/B/C-roll modes, anti-monotony rhythm, and bounded job semantics. Plans only — never renders, never calls a cloud provider, never holds credentials. Adapted from the MIT-licensed Vox Director concepts (see THIRD_PARTY.yml and docs/legal/notices/vox-director.txt); provenance snapshot at imports/provenance/vox-director/.",
      content_hash: "sha256:d45870127dd1766b7ac02f19cdb7b2c22c78ba3f4276032114ed1acd971306c7",
      dependencies: Object.freeze([]),
      permissions: Object.freeze([]),
      resources: Object.freeze([]),
      order: 7,
    }),
    Object.freeze({
      id: "writing",
      version: "0.1.0",
      description: "Route editorial prose, essays, scripts, captions, threads, research articles, conversion copy, hooks, and content repurposing. Use when words are the deliverable. Invoked as cutright://skill/writing.",
      content_hash: "sha256:d371ebde9a49bae26a10915c881fda22d0deec7271efe16c8d82f3d82a965ab5",
      dependencies: Object.freeze([]),
      permissions: Object.freeze([]),
      resources: Object.freeze([]),
      order: 8,
    }),
  ]),
});

export const skillCatalogById: Readonly<Record<string, SkillCatalogEntry>> = Object.freeze(
  Object.fromEntries(skillCatalog.skills.map((entry) => [entry.id, entry])),
);
