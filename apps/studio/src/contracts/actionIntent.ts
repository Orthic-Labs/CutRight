// apps/studio/src/contracts/actionIntent.ts
//
// Frozen Studio action-intent contract for Book 6 task CR-V2-B6-003.
// Mirrors `schemas/studio/action-intent.schema.v1.json` and
// `docs/product/V2-STUDIO-ACTIONS.md`.
//
// Every UI mutation must become an ActionIntent. The backend then builds
// a typed ActionBatch against the observed revision, performs a dry-run,
// applies the policy gate, executes against a staged clone, and persists
// the ActionResult. The UI patches only from the persisted result.

export const ACTION_INTENT_SCHEMA = "cutright.studio.action_intent/v1" as const;

export type ActionKind =
  | "timeline"
  | "transcript"
  | "story"
  | "beats"
  | "design"
  | "motion_sound"
  | "selection"
  | "settings"
  | "qa";

export const ACTION_KINDS: readonly ActionKind[] = [
  "timeline",
  "transcript",
  "story",
  "beats",
  "design",
  "motion_sound",
  "selection",
  "settings",
  "qa",
] as const;

export type RiskBand = "low" | "medium" | "high";

export const RISK_BANDS: readonly RiskBand[] = ["low", "medium", "high"] as const;

export type TargetKind =
  | "clip"
  | "track"
  | "beat"
  | "take"
  | "graphic"
  | "caption"
  | "effect"
  | "asset"
  | "transcript_word"
  | "reframe_anchor"
  | "project";

export const TARGET_KINDS: readonly TargetKind[] = [
  "clip",
  "track",
  "beat",
  "take",
  "graphic",
  "caption",
  "effect",
  "asset",
  "transcript_word",
  "reframe_anchor",
  "project",
] as const;

// Discriminated union: target.kind picks the companion ID field.
export type ActionTarget =
  | { kind: "clip"; clip_id: string }
  | { kind: "track"; track_id: string }
  | { kind: "beat"; beat_id: string }
  | { kind: "take"; take_id: string }
  | { kind: "graphic"; graphic_id: string }
  | { kind: "caption"; caption_id: string }
  | { kind: "effect"; effect_id: string }
  | { kind: "asset"; asset_id: string }
  | { kind: "transcript_word"; word_id: string }
  | { kind: "reframe_anchor"; anchor_id: string }
  | { kind: "project"; project_id: string };

export type ActionIntent = {
  readonly schema: typeof ACTION_INTENT_SCHEMA;
  readonly intent_id: string;
  readonly kind: ActionKind;
  readonly risk?: RiskBand;
  readonly target: ActionTarget;
  readonly verb: string;
  readonly params?: Readonly<Record<string, unknown>>;
  readonly evidence_refs?: readonly string[];
  readonly client_request_id?: string;
};

// Helper: pick the discriminator ID field name. Used to assemble a fresh
// intent without manually setting the right `_id` key.
export function targetIdField(kind: TargetKind):
  | "clip_id"
  | "track_id"
  | "beat_id"
  | "take_id"
  | "graphic_id"
  | "caption_id"
  | "effect_id"
  | "asset_id"
  | "word_id"
  | "anchor_id"
  | "project_id" {
  switch (kind) {
    case "clip":
      return "clip_id";
    case "track":
      return "track_id";
    case "beat":
      return "beat_id";
    case "take":
      return "take_id";
    case "graphic":
      return "graphic_id";
    case "caption":
      return "caption_id";
    case "effect":
      return "effect_id";
    case "asset":
      return "asset_id";
    case "transcript_word":
      return "word_id";
    case "reframe_anchor":
      return "anchor_id";
    case "project":
      return "project_id";
  }
}

// Builder. Centralises intent_id generation so the frontend never hand-rolls
// a UUID per call.
export function buildActionIntent(input: {
  kind: ActionKind;
  target: ActionTarget;
  verb: string;
  risk?: RiskBand;
  params?: Record<string, unknown>;
  evidence_refs?: readonly string[];
  client_request_id?: string;
}): ActionIntent {
  const intent_id = cryptoRandomUUID();
  const intent: {
    schema: typeof ACTION_INTENT_SCHEMA;
    intent_id: string;
    kind: ActionKind;
    risk?: RiskBand;
    target: ActionTarget;
    verb: string;
    params?: Record<string, unknown>;
    evidence_refs?: readonly string[];
    client_request_id?: string;
  } = {
    schema: ACTION_INTENT_SCHEMA,
    intent_id,
    kind: input.kind,
    target: input.target,
    verb: input.verb,
  };
  if (input.risk !== undefined) intent.risk = input.risk;
  if (input.params !== undefined) intent.params = input.params;
  if (input.evidence_refs !== undefined) intent.evidence_refs = input.evidence_refs;
  if (input.client_request_id !== undefined) intent.client_request_id = input.client_request_id;
  return intent;
}

// RFC4122 v4 UUID via the platform's WebCrypto. The browser provides it
// in Tauri; tests use a deterministic fallback below.
function cryptoRandomUUID(): string {
  const g = globalThis as { crypto?: { randomUUID?: () => string } };
  if (g.crypto && typeof g.crypto.randomUUID === "function") {
    return g.crypto.randomUUID();
  }
  // Deterministic-ish fallback for jsdom-only test runs. Not used in the
  // packaged app.
  const rnd = () => Math.floor(Math.random() * 0xffff).toString(16).padStart(4, "0");
  return `${rnd()}${rnd()}-${rnd()}-4${rnd().slice(1)}-${rnd()}-${rnd()}${rnd()}${rnd()}`;
}

// Type guard for responses from the IPC boundary. Used by the diff modal
// before it renders.
export function isActionIntent(value: unknown): value is ActionIntent {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  if (v.schema !== ACTION_INTENT_SCHEMA) return false;
  if (typeof v.intent_id !== "string") return false;
  if (typeof v.kind !== "string") return false;
  if (!(ACTION_KINDS as readonly string[]).includes(v.kind)) return false;
  if (typeof v.verb !== "string") return false;
  if (typeof v.target !== "object" || v.target === null) return false;
  const t = v.target as Record<string, unknown>;
  if (typeof t.kind !== "string") return false;
  if (!(TARGET_KINDS as readonly string[]).includes(t.kind)) return false;
  return true;
}

// Risk policy. Mirrors `docs/product/V2-STUDIO-ACTIONS.md` §7. The
// frontend uses this to decide whether to show a modal before submit.
export function requiresConfirmation(risk: RiskBand): boolean {
  return risk === "medium" || risk === "high";
}

export function requiresReason(risk: RiskBand): boolean {
  return risk === "high";
}