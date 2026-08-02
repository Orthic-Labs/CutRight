// Zod prop schemas mirroring `schemas/effects/registry.json`'s
// `props_schema` for every effect this package renders. This is a second,
// independent validation layer inside Remotion Studio/render — the
// authoritative gate is Rust's `EffectRegistry::validate_props`, called
// before Node is ever launched (see ../README.md). Keep these in sync with
// the registry document by hand; there are only four of them.
//
// `reducedMotion` is not part of any registry `props_schema` — it is an
// internal flag `scripts/render.mjs` injects for the reduced-motion render
// pass only, never a prop a caller can set through the registry.
import { z } from "zod";

const reducedMotion = z.boolean().optional().default(false);

// `.strict()` everywhere: the registry's `props_schema` for each of these
// effects declares `additionalProperties: false`, so an unknown key must be
// rejected here too, not silently stripped (Zod's plain `z.object()`
// default) — a schema that accepts what the registry rejects is a second
// layer that lies.
export const lowerThirdIdentityCardSchema = z
  .object({
    name: z.string(),
    title: z.string(),
    accent_color: z.string(),
    reducedMotion,
  })
  .strict();
export type LowerThirdIdentityCardProps = z.infer<typeof lowerThirdIdentityCardSchema>;

export const statCounterSchema = z
  .object({
    label: z.string(),
    value: z.number(),
    accent_color: z.string(),
    reducedMotion,
  })
  .strict();
export type StatCounterProps = z.infer<typeof statCounterSchema>;

export const quoteCardSchema = z
  .object({
    quote: z.string(),
    attribution: z.string().optional(),
    accent_color: z.string(),
    reducedMotion,
  })
  .strict();
export type QuoteCardProps = z.infer<typeof quoteCardSchema>;

export const ctaEndCardSchema = z
  .object({
    headline: z.string(),
    subtext: z.string().optional(),
    accent_color: z.string(),
    reducedMotion,
  })
  .strict();
export type CtaEndCardProps = z.infer<typeof ctaEndCardSchema>;

/// Registry `effect_id` -> Remotion composition id. Composition ids reject
/// `.`, so this is the one place the mapping is defined; `scripts/render.mjs`
/// re-derives the same mapping mechanically (`effectId.replace(/\./g, "-")`)
/// rather than importing this table, so it stays usable from plain Node
/// without a TypeScript build step.
export const EFFECT_ID_TO_COMPOSITION_ID: Record<string, string> = {
  "lower-third.identity-card.v1": "lower-third-identity-card-v1",
  "stat-counter.v1": "stat-counter-v1",
  "quote-card.v1": "quote-card-v1",
  "cta-end-card.v1": "cta-end-card-v1",
};
