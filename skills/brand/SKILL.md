---
name: brand
description: "Load brand voice, visuals, tone, and restrictions before creating branded content or design. Invoked as cutright://skill/brand {\"brand_code\":\"DD\"} (DD, RH, HR, TS, SS, VR, SR, CR, or MR), or when work names one of those ventures."
---

# Brand

MODE: OUTPUT_ONLY
PRIMARY_DELIVERABLE: Source-bound Brand Card.
DISCOVERY_PROFILE: D1_SCOPED_SOURCE
EFFECT_PROFILES: source_read
SPECIALIST_REFS_MAX: 1
CHILD_AGENTS_MAX: 0
EXTERNAL_REQUESTS_MAX: 0
MAY_ADD_TASKS: NO
MAY_CALL_SKILLS: cutright://skill/brand-identity
TERMINAL: One source-bound Brand Card is ready for downstream use.

## Typed artefacts (CutRight v2)

```rust
pub struct BrandCard {
    pub brand_id: String,
    pub voice: VoiceRules,
    pub visual: VisualTokens,
    pub restrictions: Vec<Restriction>,
    pub provenance: Vec<SourceRef>,
}
```

- The only output of this skill is a `BrandCard` assembled from the data pack sources below.
- **Non-mutation guarantee:** brand rules are advisory constraints on downstream output.
  They never mutate source media, timeline cuts, rendered assets, or code; downstream
  skills re-derive their work under the card's restrictions.
- Venture data arrives via the optional signed creative data pack (`brand-pack/`); when a
  pack is absent, say so — never invent brand facts.

## Flow

1. Resolve one brand code: DD, RH, HR, TS, SS, VR, SR, CR, or MR.
2. Read only that brand section in `brand-pack/manual.md` plus its cited canonical source:
   - DD → `brand-pack/damned-designs.md`
   - RH → `brand-pack/rotten-hand.md`
   - SS → `brand-pack/stunning-strangers.md`
   - HR → `brand-pack/heard-right.md` (+ `brand-pack/right-suite.md` for the locked Right Suite visual identity)
   - TS → `brand-pack/toxic-sundae.md`
   - VR, SR, CR, MR → `brand-pack/right-suite.md` (ViewRight, ScrapeRight, CodeRight, MailRight identities)
   - Cross-brand rules & motion principles → `references/cross-brand.md`
3. Treat locked identity, voice, logo, color, typography, motion, & restriction rules as invariants.
4. Ask for a brand only when no named venture or unambiguous repository establishes it.
5. Return a compact Brand Card: voice, visual system, restrictions, required assets, & source paths.
6. Apply the card to downstream work via `cutright://skill/writing`, `cutright://skill/content`,
   `cutright://skill/designer`, or `cutright://skill/social` (upstream `ads`/`marketing` routing
   is not vendored into the CutRight corpus).
7. Never infer missing brand facts or replace locked assets.

Use `cutright://skill/brand-identity` to create or evolve an identity system.
