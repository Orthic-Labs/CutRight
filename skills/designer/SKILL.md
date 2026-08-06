---
name: designer
description: "Create, redesign, or polish websites, app UI, dashboards, components, static creative, print, motion systems, glass materials, illustration direction, and frontend craft. Route review-only work to cutright://skill/qa (visual_review mode) and identity systems to cutright://skill/brand-identity."
---

# Designer

MODE: OUTPUT_ONLY
PRIMARY_DELIVERABLE: Rendered design artifact.
DISCOVERY_PROFILE: D1_SCOPED_SOURCE
EFFECT_PROFILES: asset_read, asset_delivery_emit
SPECIALIST_REFS_MAX: 1
CHILD_AGENTS_MAX: 0
EXTERNAL_REQUESTS_MAX: 0
MAY_ADD_TASKS: NO
MAY_CALL_SKILLS: cutright://skill/brand, cutright://skill/brand-identity, cutright://skill/qa (visual_review)
TERMINAL: Rendered acceptance proven.

## Output contract (CutRight v2)

- This skill is OUTPUT_ONLY: it performs no direct workspace mutation. Every produced design
  artifact is emitted as an **AssetDelivery** record on the CutRight job plane; rendering requests
  arrive as **AssetRequest** / **RenderSampleRequest** and are answered with AssetDelivery only.
- Visual review is delegated to `cutright://skill/qa {"mode":"visual_review"}` (typed result:
  **VisualReviewResult**). Identity work is delegated to `cutright://skill/brand-identity`.
- Engine scripts under `engine/scripts/` and `engine/huashu/scripts/` are inert provenance; they
  run only as typed capabilities `cutright://capability/designer.<name>` inside the signed runtime
  pack, never as bare shell invocations.

Load `cutright://skill/brand {"brand_code":"<code>"}` when branded. Choose draft for exploration or ship for production.

- Web or app: `specialists/surface-design/GUIDE.md`, then `references/website.md`, `app.md`, or `native-app.md`.
- Static or print: `specialists/static-creative/GUIDE.md`.
- Craft command: `engine/GUIDE.md`, then one matching command reference.
- Slides or motion render: `engine/huashu/GUIDE.md`.
- Motion: `specialists/motion/GUIDE.md`; glass: `specialists/glass/GUIDE.md`; illustration direction: `skills/_shared/illustrate/GUIDE.md`.

Read one branch only. Read `references/manual.md` for mixed, production, or unfamiliar work. Freeze content, truth, platform, states, dimensions, accessibility, & acceptance. Build one exemplar before scaling. Reuse existing tokens & components. Inspect rendered states at target sizes. Route review-only work to `cutright://skill/qa {"mode":"visual_review"}` & identity creation to `cutright://skill/brand-identity`.
