---
name: writing
description: "Route editorial prose, essays, scripts, captions, threads, research articles, conversion copy, hooks, and content repurposing. Use when words are the deliverable. Invoked as cutright://skill/writing."
---

# Writing

MODE: OUTPUT_ONLY
PRIMARY_DELIVERABLE: Prose artifact (delivered as AssetDelivery).
DISCOVERY_PROFILE: D1_SCOPED_SOURCE
EFFECT_PROFILES: source_read, asset_delivery_emit
SPECIALIST_REFS_MAX: 1
CHILD_AGENTS_MAX: 0
EXTERNAL_REQUESTS_MAX: 0
MAY_ADD_TASKS: NO
MAY_CALL_SKILLS: cutright://skill/brand
TERMINAL: Artifact meets brief & evidence limits.

## Typed handoff artefacts (CutRight v2)

- **ScriptPlan** — timed script output from `references/script.md` (hook, body beats, closer,
  duration/word budget). Consumed by `cutright://skill/content` and `cutright://skill/social`.
- **PackageCopy** — gated conversion copy from `specialists/copywriting/GUIDE.md` (promise, proof,
  message architecture, variants). Consumed by `cutright://skill/designer` for rendered surfaces.
- The writing skill performs no direct workspace mutation: prose is delivered as AssetDelivery
  records; it never posts, schedules, or publishes anything itself.

Load `cutright://skill/brand {"brand_code":"<code>"}` when branded (typed result: BrandCard).
Route one deliverable:

- Editorial: `specialists/editorial/GUIDE.md`; research article: `references/research-article.md`;
  script: `references/script.md`; repurpose: `references/content-repurposer/reference.md` or
  `references/repurpose-content/reference.md`.
- Persuasive copy, hooks, ad copy: `specialists/copywriting/GUIDE.md`.

## Excluded lanes (see imports/v2/exclusions/writing.json)

- `specialists/blogs` — blogs lane not in the selected CutRight v2 writing closure.
- `specialists/email` — email lane not in the selected CutRight v2 writing closure.
- `specialists/profile-copy` — profile lane not in the selected CutRight v2 writing closure.
- `specialists/changelog` — changelog lane not in the selected CutRight v2 writing closure.

Read one branch. Use `references/manual.md` for mixed work. Freeze audience, goal, channel, length,
facts, CTA, voice, & restrictions. Never invent quotes, statistics, testimonials, stories, or
product facts. Remove generic openings, repetition, unsupported claims, rhythm monotony, &
wrong-channel structure. Media → cutright://skill/content; design → cutright://skill/designer;
distribution → cutright://skill/social (upstream marketing/ads strategy lanes are not vendored).
