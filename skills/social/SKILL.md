---
name: social
description: "Route Instagram, Pinterest, YouTube, Twitter or X, LinkedIn, Reels, Shorts, pins, threads, calendars, and social growth strategy. Use cutright://skill/social when social strategy or platform-native content is the deliverable. Plan and copy only — no posting, scheduling, or account mutation."
---

# Social

MODE: OUTPUT_ONLY
PRIMARY_DELIVERABLE: Platform-native artifact or strategy (delivered as AssetDelivery).
DISCOVERY_PROFILE: D1_SCOPED_SOURCE
EFFECT_PROFILES: source_read, asset_delivery_emit
SPECIALIST_REFS_MAX: 1
CHILD_AGENTS_MAX: 0
EXTERNAL_REQUESTS_MAX: 0
MAY_ADD_TASKS: NO
MAY_CALL_SKILLS: cutright://skill/brand, cutright://skill/designer, cutright://skill/writing
TERMINAL: Platform-native artifact or strategy meets frozen scope.

## Typed artefacts + boundary (CutRight v2)

- **PlatformConstraintSet** — the versioned platform rules (formats, limits, cadence, hook shape)
  carried under `references/`; consumed when shaping any platform-native artifact. Platform rules
  are versioned data, not executable web-format logic.
- **Hard boundary:** this skill cannot post, schedule, spend, or mutate an account. Posting,
  scheduling, and account connectors are excluded (`imports/v2/exclusions/social.json`); publishing
  steps are emitted as plan data for the operator, never executed.
- Live analytics are operator-provided (screenshots/exports); no logged-in session automation and
  no API tokens.

## Flow

1. Freeze brand, platform, audience, objective, period, source material, constraints, & metrics.
2. Load `cutright://skill/brand {"brand_code":"<code>"}` before branded output (typed result: BrandCard).
3. Route platform craft to `references/<platform>/reference.md`; route cross-platform content to
   `references/content/reference.md`.
4. Read `references/manual.md` for strategy, calendar, audit, analytics, or multi-platform work.
5. Media production → `cutright://skill/content`; prose → `cutright://skill/writing`; static assets
   → `cutright://skill/designer` (upstream paid-distribution and positioning lanes are not vendored).
6. Platform formats/limits are versioned data in this closure; verifying formats beyond the
   vendored snapshot is a host research capability, never a required path.
7. Never invent engagement, reach, testimonials, audience evidence, or performance.
8. Return platform-native outputs plus assumptions, source links, publishing order, & measurement plan.
9. Publishing order is plan data only — execution happens with the operator, outside this skill.
