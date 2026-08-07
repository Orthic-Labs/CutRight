---
name: brand-identity
description: "Create, audit, evolve, or apply brand identities, systems, guidelines, visual identity, voice, naming, logo direction, brand books, rebrands, website or app identity, pitch decks, or social kits."
---

# Brand Identity

MODE: OUTPUT_ONLY
PRIMARY_DELIVERABLE: Identity decisions, assets, restrictions, & QA evidence.
DISCOVERY_PROFILE: D1_SCOPED_SOURCE
EFFECT_PROFILES: asset_read, asset_delivery_emit
SPECIALIST_REFS_MAX: 1
CHILD_AGENTS_MAX: 0
EXTERNAL_REQUESTS_MAX: 0
MAY_ADD_TASKS: NO
MAY_CALL_SKILLS: NONE
TERMINAL: Identity decisions, assets, restrictions, & QA evidence exist.

## Typed artefacts (CutRight v2)

Identity outputs are emitted as typed artefacts (delivered as AssetDelivery records; no
direct mutation of existing locked assets):

- **BrandSystem** — the full identity decision set: signature mechanism, direction,
  voice, marks, type, color roles, applications, provenance, QA evidence.
- **BrandTokenSet** — the machine-readable `<repo>/.brand/tokens.json` consumed by
  `cutright://skill/designer` and `cutright://skill/qa {"mode":"visual_review"}`.
- **BrandRestrictionSet** — locked invariants and banned defaults bound to the identity
  (never overwritten once locked).
- Color science gates run through the typed capability
  `cutright://capability/brand_identity.color_check` (`scripts/color-check.mjs`; pure
  zero-dependency math).

## Flow

1. Load `cutright://skill/brand {"brand_code":"<code>"}` when an existing venture is named.
2. Freeze audience, promise, positioning, constraints, assets, production media, & required deliverables.
3. Read `references/manual.md` for identity creation, evolution, audit, or application.
4. Read `references/brand-registry.md` before registry changes; use `visual-reference-libraries.md` only when visual research is needed.
5. Define one signature identity mechanism before colors, type, marks, voice, & applications.
6. Create materially divergent directions when exploration is requested.
7. Test contrast, scale, reproduction, accessibility, platform fit, & banned defaults.
8. Separate approved identity from exploration; never overwrite locked assets or rules.
9. Deliver decisions, tokens, assets, application examples, restrictions, provenance, & QA evidence.
